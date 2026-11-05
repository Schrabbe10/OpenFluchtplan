//! Zeichnet Primitive mit egui und verwaltet Texturen (Symbole, Bilder).

use std::collections::HashMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use eframe::egui::{self, Color32, FontFamily, FontId, Mesh, Pos2, Rect, Shape, Stroke, TextureHandle, Vec2, epaint};

use crate::geom::*;
use crate::model::*;
use crate::render::{self, Prim};

pub fn c32(c: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, c.3)
}

#[derive(Clone, Copy)]
pub struct View {
    pub origin: Pos2,
    pub zoom: f32,
}

impl View {
    pub fn to_screen(&self, p: Pos) -> Pos2 {
        Pos2::new(self.origin.x + p.x * self.zoom, self.origin.y + p.y * self.zoom)
    }
    pub fn to_mm(&self, s: Pos2) -> Pos {
        Pos::new((s.x - self.origin.x) / self.zoom, (s.y - self.origin.y) / self.zoom)
    }
}

#[derive(Default)]
pub struct Textures {
    symbols: HashMap<(String, u32), TextureHandle>,
    images: HashMap<u64, (usize, TextureHandle)>,
    pub quality_high: bool,
}

impl Textures {
    pub fn symbol(&mut self, ctx: &egui::Context, doc: &Doc, id: &str, px: f32) -> Option<TextureHandle> {
        let max = if self.quality_high { 1024 } else { 256 };
        let bucket = (px.max(16.0).min(max as f32) as u32).next_power_of_two().min(max);
        let key = (id.to_string(), bucket);
        if let Some(t) = self.symbols.get(&key) {
            return Some(t.clone());
        }
        let svg = render::symbol_svg(doc, id)?;
        let src = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100' width='{bucket}' height='{bucket}'>{svg}</svg>");
        let tree = resvg::usvg::Tree::from_str(&src, crate::export::usvg_options()).ok()?;
        let mut pm = resvg::tiny_skia::Pixmap::new(bucket, bucket)?;
        let s = bucket as f32 / tree.size().width();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(s, s), &mut pm.as_mut());
        let img = egui::ColorImage::from_rgba_premultiplied([bucket as usize, bucket as usize], pm.data());
        let tex = ctx.load_texture(format!("sym-{id}-{bucket}"), img, egui::TextureOptions::LINEAR);
        self.symbols.insert(key, tex.clone());
        Some(tex)
    }

    pub fn image(&mut self, ctx: &egui::Context, key: u64, data: &ImageData) -> Option<TextureHandle> {
        if let Some((len, t)) = self.images.get(&key) {
            if *len == data.b64.len() {
                return Some(t.clone());
            }
        }
        let bytes = B64.decode(&data.b64).ok()?;
        let mut img = image::load_from_memory(&bytes).ok()?;
        if img.width() > 4096 || img.height() > 4096 {
            img = img.thumbnail(4096, 4096);
        }
        let rgba = img.to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw());
        let tex = ctx.load_texture(format!("img-{key}"), ci, egui::TextureOptions::LINEAR);
        self.images.insert(key, (data.b64.len(), tex.clone()));
        Some(tex)
    }

    pub fn clear_symbols(&mut self) {
        self.symbols.clear();
    }
}

/// System-Schriften (Arial) laden, damit Bildschirm und PDF gleich aussehen.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into());
    let load = |name: &str| std::fs::read(std::path::Path::new(&windir).join("Fonts").join(name)).ok();
    let mut add = |key: &str, file: &str, family: FontFamily| {
        if let Some(bytes) = load(file) {
            fonts.font_data.insert(key.into(), egui::FontData::from_owned(bytes).into());
            fonts.families.entry(family).or_default().insert(0, key.into());
        }
    };
    add("arial", "arial.ttf", FontFamily::Name("plan".into()));
    add("arialbd", "arialbd.ttf", FontFamily::Name("plan_bold".into()));
    add("ariali", "ariali.ttf", FontFamily::Name("plan_italic".into()));
    add("arialbi", "arialbi.ttf", FontFamily::Name("plan_bold_italic".into()));
    // Rückfall auf Standardschrift, falls Arial fehlt
    for fam in ["plan", "plan_bold", "plan_italic", "plan_bold_italic"] {
        let e = fonts.families.entry(FontFamily::Name(fam.into())).or_default();
        e.push("Ubuntu-Light".into());
    }
    ctx.set_fonts(fonts);
}

fn family(bold: bool, italic: bool) -> FontFamily {
    FontFamily::Name(
        match (bold, italic) {
            (false, false) => "plan",
            (true, false) => "plan_bold",
            (false, true) => "plan_italic",
            (true, true) => "plan_bold_italic",
        }
        .into(),
    )
}

fn textured_quad(painter: &egui::Painter, tex: &TextureHandle, corners: [Pos2; 4], tint: Color32) {
    let mut mesh = Mesh::with_texture(tex.id());
    let uvs = [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)];
    for (p, uv) in corners.iter().zip(uvs) {
        mesh.vertices.push(epaint::Vertex { pos: *p, uv, color: tint });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(Shape::mesh(mesh));
}

pub fn paint_prims(painter: &egui::Painter, ctx: &egui::Context, tex: &mut Textures, doc: &Doc, prims: &[Prim], view: View) {
    let clip = painter.clip_rect();
    let z = view.zoom;
    for prim in prims {
        match prim {
            Prim::Fill { pts, color } => {
                if pts.len() < 3 {
                    continue;
                }
                let bb = BBox::from_pts(pts);
                let r = Rect::from_min_max(view.to_screen(bb.min), view.to_screen(bb.max));
                if !r.intersects(clip) {
                    continue;
                }
                let col = c32(*color);
                if pts.len() == 4 || pts.len() == 3 {
                    painter.add(Shape::convex_polygon(pts.iter().map(|p| view.to_screen(*p)).collect(), col, Stroke::NONE));
                    continue;
                }
                let idx = triangulate(pts);
                let mut mesh = Mesh::default();
                for p in pts {
                    mesh.colored_vertex(view.to_screen(*p), col);
                }
                mesh.indices.extend(idx.iter().map(|i| *i as u32));
                painter.add(Shape::mesh(mesh));
            }
            Prim::FillRings { rings, color } => {
                let bb = rings.iter().fold(BBox::EMPTY, |b, r| b.union(BBox::from_pts(r)));
                let r = Rect::from_min_max(view.to_screen(bb.min), view.to_screen(bb.max));
                if !r.intersects(clip) {
                    continue;
                }
                let col = c32(*color);
                let (pts, idx) = crate::geom::triangulate_rings(rings);
                let mut mesh = Mesh::default();
                for p in &pts {
                    mesh.colored_vertex(view.to_screen(*p), col);
                }
                mesh.indices.extend(idx.iter().map(|i| *i as u32));
                painter.add(Shape::mesh(mesh));
            }
            Prim::Stroke { pts, stroke, closed } => {
                if pts.len() < 2 {
                    continue;
                }
                let bb = BBox::from_pts(pts).expand(stroke.width);
                let r = Rect::from_min_max(view.to_screen(bb.min), view.to_screen(bb.max));
                if !r.intersects(clip) {
                    continue;
                }
                let w = (stroke.width * z).max(0.6);
                let col = c32(stroke.color);
                let mut p = pts.clone();
                if *closed {
                    p.push(pts[0]);
                }
                let parts = match &stroke.dash {
                    Some(d) => dash_polyline(&p, d),
                    None => vec![p],
                };
                for part in parts {
                    let sp: Vec<Pos2> = part.iter().map(|q| view.to_screen(*q)).collect();
                    if stroke.round && w > 3.0 {
                        for q in &sp {
                            painter.circle_filled(*q, w / 2.0, col);
                        }
                    }
                    painter.add(Shape::line(sp, Stroke::new(w, col)));
                }
            }
            Prim::Text { pos, text, size, color, align, bold, italic, rot } => {
                let px = size * z;
                if px < 1.5 || text.is_empty() {
                    continue;
                }
                let sp = view.to_screen(*pos);
                if !clip.expand(px * text.len() as f32).contains(sp) {
                    continue;
                }
                let galley = painter.layout_no_wrap(text.clone(), FontId::new(px, family(*bold, *italic)), c32(*color));
                let w = galley.size().x;
                let dx = match align {
                    TAlign::Links => 0.0,
                    TAlign::Mitte => -w / 2.0,
                    TAlign::Rechts => -w,
                };
                // Galley-Oberkante liegt etwas über der Versalhöhe -> leicht nach oben ziehen
                let off = Vec2::new(dx, -px * 0.08);
                let rad = rot.to_radians();
                let (s, c) = rad.sin_cos();
                let off_r = Vec2::new(off.x * c - off.y * s, off.x * s + off.y * c);
                let shape = epaint::TextShape::new(sp + off_r, galley, c32(*color)).with_angle(rad);
                painter.add(shape);
            }
            Prim::Symbol { id, c, size, rot } => {
                let px = size * z;
                let center = view.to_screen(*c);
                if px < 1.0 || !clip.expand(px).contains(center) {
                    continue;
                }
                if let Some(t) = tex.symbol(ctx, doc, id, px) {
                    let h = px / 2.0;
                    let r2 = egui::emath::Rot2::from_angle(rot.to_radians());
                    let corners = [Vec2::new(-h, -h), Vec2::new(h, -h), Vec2::new(h, h), Vec2::new(-h, h)].map(|v| center + r2 * v);
                    textured_quad(painter, &t, corners, Color32::WHITE);
                } else {
                    painter.rect_stroke(Rect::from_center_size(center, Vec2::splat(px)), 0.0, Stroke::new(1.0, Color32::RED), egui::StrokeKind::Inside);
                }
            }
            Prim::Image { key, a, b, data, opacity } => {
                if let Some(t) = tex.image(ctx, *key, data) {
                    let bb = BBox::from_two(*a, *b);
                    let corners = [bb.min, Pos::new(bb.max.x, bb.min.y), bb.max, Pos::new(bb.min.x, bb.max.y)].map(|p| view.to_screen(p));
                    textured_quad(painter, &t, corners, Color32::from_white_alpha((opacity * 255.0) as u8));
                }
            }
        }
    }
}

/// Symbol-Vorschau in einem UI-Rechteck (Bibliothek, Legende).
pub fn paint_symbol_icon(painter: &egui::Painter, ctx: &egui::Context, tex: &mut Textures, doc: &Doc, id: &str, rect: Rect) {
    if let Some(t) = tex.symbol(ctx, doc, id, rect.width().max(rect.height())) {
        let s = rect.width().min(rect.height());
        let r = Rect::from_center_size(rect.center(), Vec2::splat(s));
        painter.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
}

/// Flächenmuster-Vorschau.
pub fn paint_style_icon(painter: &egui::Painter, ctx: &egui::Context, tex: &mut Textures, doc: &Doc, style: &Style, rect: Rect) {
    let view = View { origin: rect.min, zoom: rect.width() / 10.0 };
    let mut prims = Vec::new();
    let pts = render::rect_pts(Pos::new(0.5, 0.5), Pos::new(9.5, 10.0 * rect.height() / rect.width() - 0.5));
    let mut st = *style;
    if let Fill::Schraffur(mut h) = st.fill {
        h.spacing = h.spacing.max(1.5);
        st.fill = Fill::Schraffur(h);
    }
    if st.stroke.is_none() {
        st.stroke = Some(StrokeStyle::new(Color::rgb(0x80, 0x80, 0x80), 0.1));
    }
    render::style_prims(&mut prims, &pts, true, &st);
    paint_prims(painter, ctx, tex, doc, &prims, view);
}
