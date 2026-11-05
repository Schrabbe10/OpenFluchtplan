//! Export: SVG, PDF (ein-/mehrseitig, Überdruck), PNG, DXF.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::OnceLock;

use crate::geom::*;
use crate::model::*;
use crate::render::{self, Prim};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stamp {
    Keiner,
    Entwurf,
    Muster,
    Vorabzug,
}

impl Stamp {
    pub fn text(self) -> &'static str {
        match self {
            Stamp::Keiner => "",
            Stamp::Entwurf => "ENTWURF",
            Stamp::Muster => "MUSTER",
            Stamp::Vorabzug => "VORABZUG",
        }
    }
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

fn pts_str(p: &[Pos]) -> String {
    p.iter().map(|p| format!("{:.3},{:.3}", p.x, p.y)).collect::<Vec<_>>().join(" ")
}

fn write_prim(s: &mut String, doc: &Doc, prim: &Prim) {
    match prim {
        Prim::Fill { pts, color } => {
            let _ = write!(s, "<polygon points=\"{}\" fill=\"{}\" fill-opacity=\"{:.3}\"/>", pts_str(pts), color.hex(), color.alpha());
        }
        Prim::FillRings { rings, color } => {
            let mut d = String::new();
            for r in rings.iter().filter(|r| r.len() >= 3) {
                let _ = write!(d, "M{:.3} {:.3}", r[0].x, r[0].y);
                for p in &r[1..] {
                    let _ = write!(d, "L{:.3} {:.3}", p.x, p.y);
                }
                d.push('Z');
            }
            let _ = write!(s, "<path d=\"{d}\" fill=\"{}\" fill-opacity=\"{:.3}\" fill-rule=\"evenodd\"/>", color.hex(), color.alpha());
        }
        Prim::Stroke { pts, stroke, closed } => {
            let tag = if *closed { "polygon" } else { "polyline" };
            let (cap, join) = if stroke.round { ("round", "round") } else { ("butt", "miter") };
            let dash = stroke
                .dash
                .as_ref()
                .map(|d| format!(" stroke-dasharray=\"{}\"", d.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            let _ = write!(
                s,
                "<{tag} points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{:.3}\" stroke-width=\"{:.3}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\" stroke-miterlimit=\"6\"{dash}/>",
                pts_str(pts),
                stroke.color.hex(),
                stroke.color.alpha(),
                stroke.width
            );
        }
        Prim::Text { pos, text, size, color, align, bold, italic, rot } => {
            if text.is_empty() {
                return;
            }
            let anchor = match align {
                TAlign::Links => "start",
                TAlign::Mitte => "middle",
                TAlign::Rechts => "end",
            };
            let base = *pos + Pos::new(0.0, size * 0.8).rotate(*rot);
            let tr = if rot.abs() > 0.01 { format!(" transform=\"rotate({:.2} {:.3} {:.3})\"", rot, base.x, base.y) } else { String::new() };
            let _ = write!(
                s,
                "<text x=\"{:.3}\" y=\"{:.3}\" font-family=\"Arial, Helvetica, sans-serif\" font-size=\"{:.3}\" font-weight=\"{}\" font-style=\"{}\" text-anchor=\"{anchor}\" fill=\"{}\"{tr} xml:space=\"preserve\">{}</text>",
                base.x,
                base.y,
                size,
                if *bold { "bold" } else { "normal" },
                if *italic { "italic" } else { "normal" },
                color.hex(),
                esc(text)
            );
        }
        Prim::Symbol { id, c, size, rot } => {
            if let Some(svg) = render::symbol_svg(doc, id) {
                let _ = write!(
                    s,
                    "<g transform=\"translate({:.3} {:.3}) rotate({:.2}) translate({:.3} {:.3}) scale({:.5})\">{}</g>",
                    c.x,
                    c.y,
                    rot,
                    -size / 2.0,
                    -size / 2.0,
                    size / 100.0,
                    svg
                );
            }
        }
        Prim::Image { a, b, data, opacity, .. } => {
            let bb = BBox::from_two(*a, *b);
            let _ = write!(
                s,
                "<image x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" opacity=\"{:.3}\" preserveAspectRatio=\"none\" xlink:href=\"data:{};base64,{}\"/>",
                bb.min.x,
                bb.min.y,
                bb.w(),
                bb.h(),
                opacity,
                data.mime,
                data.b64
            );
        }
    }
}

pub fn plan_svg(doc: &Doc, stamp: Stamp) -> String {
    let (w, h) = doc.page.size();
    let mut s = String::with_capacity(64 * 1024);
    let _ = write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">"
    );
    let _ = write!(s, "<rect x=\"0\" y=\"0\" width=\"{w}\" height=\"{h}\" fill=\"#ffffff\"/>");
    for p in render::page_prims(doc, true) {
        write_prim(&mut s, doc, &p);
    }
    if stamp != Stamp::Keiner {
        let fs = w.min(h) / 5.0;
        let _ = write!(
            s,
            "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Arial\" font-weight=\"bold\" font-size=\"{fs:.1}\" text-anchor=\"middle\" fill=\"#d00000\" fill-opacity=\"0.22\" transform=\"rotate(-28 {:.1} {:.1})\">{}</text>",
            w / 2.0,
            h / 2.0 + fs * 0.35,
            w / 2.0,
            h / 2.0,
            stamp.text()
        );
    }
    s.push_str("</svg>");
    s
}

pub fn usvg_options() -> &'static svg2pdf::usvg::Options<'static> {
    static OPT: OnceLock<svg2pdf::usvg::Options<'static>> = OnceLock::new();
    OPT.get_or_init(|| {
        let mut o = svg2pdf::usvg::Options::default();
        o.fontdb_mut().load_system_fonts();
        // im Browser gibt es keine Systemschriften: mitgelieferte, zu Arial maßgleiche Schrift
        for f in crate::plattform::SCHRIFTEN {
            o.fontdb_mut().load_font_data(f.to_vec());
        }
        if o.fontdb.faces().any(|f| f.families.iter().any(|(n, _)| n == "Arial")) {
            o.font_family = "Arial".into();
        } else {
            o.font_family = "Liberation Sans".into();
            o.fontdb_mut().set_sans_serif_family("Liberation Sans");
        }
        o
    })
}

fn tree(doc: &Doc, stamp: Stamp) -> Result<svg2pdf::usvg::Tree, String> {
    svg2pdf::usvg::Tree::from_str(&plan_svg(doc, stamp), usvg_options()).map_err(|e| format!("SVG-Fehler: {e}"))
}

/// Ein oder mehrere Pläne als PDF (eine Seite je Plan, jeweils in eigener Blattgröße).
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn export_pdf(docs: &[&Doc], stamp: Stamp, path: &Path) -> Result<(), String> {
    std::fs::write(path, pdf_bytes(docs, stamp)?).map_err(|e| format!("Speichern fehlgeschlagen: {e}"))
}

pub fn pdf_bytes(docs: &[&Doc], stamp: Stamp) -> Result<Vec<u8>, String> {
    use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
    let mut alloc = Ref::new(1);
    let catalog = alloc.bump();
    let tree_id = alloc.bump();
    let mut pdf = Pdf::new();
    let mut page_ids = Vec::new();
    let mut chunks = Vec::new();
    for doc in docs {
        let t = tree(doc, stamp)?;
        let (chunk, svg_ref) = svg2pdf::to_chunk(&t, svg2pdf::ConversionOptions::default()).map_err(|e| format!("PDF-Fehler: {e}"))?;
        let mut map = HashMap::new();
        let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_id = map[&svg_ref];
        let page_id = alloc.bump();
        let content_id = alloc.bump();
        let (w, h) = doc.page.size();
        let (wp, hp) = (w / 25.4 * 72.0, h / 25.4 * 72.0);
        page_ids.push((page_id, content_id, svg_id, wp, hp));
        chunks.push(chunk);
    }
    pdf.catalog(catalog).pages(tree_id);
    pdf.pages(tree_id).kids(page_ids.iter().map(|p| p.0)).count(page_ids.len() as i32);
    for (page_id, content_id, svg_id, wp, hp) in &page_ids {
        let mut page = pdf.page(*page_id);
        page.media_box(Rect::new(0.0, 0.0, *wp, *hp));
        page.parent(tree_id);
        page.contents(*content_id);
        page.resources().x_objects().pair(Name(b"P1"), *svg_id);
        page.finish();
        let mut content = Content::new();
        content.transform([*wp, 0.0, 0.0, *hp, 0.0, 0.0]).x_object(Name(b"P1"));
        pdf.stream(*content_id, &content.finish());
    }
    for c in &chunks {
        pdf.extend(c);
    }
    Ok(pdf.finish())
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn export_svg(doc: &Doc, path: &Path) -> Result<(), String> {
    std::fs::write(path, plan_svg(doc, Stamp::Keiner)).map_err(|e| e.to_string())
}

/// Rasterbild des Plans (z. B. für Präsentationen oder Webseiten).
pub fn render_png(doc: &Doc, dpi: f32) -> Result<Vec<u8>, String> {
    let t = tree(doc, Stamp::Keiner)?;
    let (w, h) = doc.page.size();
    let px_w = (w / 25.4 * dpi).round() as u32;
    let px_h = (h / 25.4 * dpi).round() as u32;
    let mut pm = resvg::tiny_skia::Pixmap::new(px_w.max(1), px_h.max(1)).ok_or("Bild zu groß")?;
    pm.fill(resvg::tiny_skia::Color::WHITE);
    let sx = px_w as f32 / t.size().width();
    let sy = px_h as f32 / t.size().height();
    resvg::render(&t, resvg::tiny_skia::Transform::from_scale(sx, sy), &mut pm.as_mut());
    pm.encode_png().map_err(|e| e.to_string())
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn export_png(doc: &Doc, dpi: f32, path: &Path) -> Result<(), String> {
    std::fs::write(path, render_png(doc, dpi)?).map_err(|e| e.to_string())
}

// ---------- DXF ----------

/// Vektorpfade eines Symbols (SVG) für den DXF-Export.
fn symbol_polylines(svg: &str) -> Vec<(Vec<Pos>, bool)> {
    let src = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>{svg}</svg>");
    let Ok(t) = svg2pdf::usvg::Tree::from_str(&src, usvg_options()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    fn walk(g: &svg2pdf::usvg::Group, out: &mut Vec<(Vec<Pos>, bool)>) {
        use svg2pdf::usvg::Node;
        use svg2pdf::usvg::tiny_skia_path::PathSegment;
        for n in g.children() {
            match n {
                Node::Group(g) => walk(g, out),
                Node::Path(p) => {
                    let tr = p.abs_transform();
                    let map = |x: f32, y: f32| {
                        let mut pt = [svg2pdf::usvg::tiny_skia_path::Point::from_xy(x, y)];
                        tr.map_points(&mut pt);
                        Pos::new(pt[0].x, pt[0].y)
                    };
                    let mut cur: Vec<Pos> = Vec::new();
                    let mut last = Pos::default();
                    for seg in p.data().segments() {
                        match seg {
                            PathSegment::MoveTo(a) => {
                                if cur.len() > 1 {
                                    out.push((std::mem::take(&mut cur), false));
                                }
                                cur.clear();
                                last = Pos::new(a.x, a.y);
                                cur.push(map(a.x, a.y));
                            }
                            PathSegment::LineTo(a) => {
                                last = Pos::new(a.x, a.y);
                                cur.push(map(a.x, a.y));
                            }
                            PathSegment::QuadTo(c, a) => {
                                for i in 1..=6 {
                                    let t = i as f32 / 6.0;
                                    let p0 = last;
                                    let q = p0 * ((1.0 - t) * (1.0 - t)) + Pos::new(c.x, c.y) * (2.0 * (1.0 - t) * t) + Pos::new(a.x, a.y) * (t * t);
                                    cur.push(map(q.x, q.y));
                                }
                                last = Pos::new(a.x, a.y);
                            }
                            PathSegment::CubicTo(c1, c2, a) => {
                                for i in 1..=8 {
                                    let t = i as f32 / 8.0;
                                    let u = 1.0 - t;
                                    let q = last * (u * u * u) + Pos::new(c1.x, c1.y) * (3.0 * u * u * t) + Pos::new(c2.x, c2.y) * (3.0 * u * t * t) + Pos::new(a.x, a.y) * (t * t * t);
                                    cur.push(map(q.x, q.y));
                                }
                                last = Pos::new(a.x, a.y);
                            }
                            PathSegment::Close => {
                                if cur.len() > 1 {
                                    out.push((std::mem::take(&mut cur), true));
                                }
                            }
                        }
                    }
                    if cur.len() > 1 {
                        out.push((cur, false));
                    }
                }
                Node::Text(t) => walk(t.flattened(), out),
                Node::Image(_) => {}
            }
        }
    }
    walk(t.root(), &mut out);
    out
}

fn aci(c: Color) -> u8 {
    // grobe Zuordnung zur AutoCAD-Farbpalette
    let (r, g, b) = (c.0 as i32, c.1 as i32, c.2 as i32);
    let cands: [(u8, (i32, i32, i32)); 8] = [
        (7, (0, 0, 0)),
        (1, (255, 0, 0)),
        (2, (255, 255, 0)),
        (3, (0, 255, 0)),
        (4, (0, 255, 255)),
        (5, (0, 0, 255)),
        (6, (255, 0, 255)),
        (8, (128, 128, 128)),
    ];
    cands
        .iter()
        .min_by_key(|(_, (cr, cg, cb))| (r - cr).pow(2) + (g - cg).pow(2) + (b - cb).pow(2))
        .map(|c| c.0)
        .unwrap_or(7)
}

pub fn export_dxf(doc: &Doc, path: &Path) -> Result<(), String> {
    std::fs::write(path, dxf_bytes(doc)?).map_err(|e| e.to_string())
}

/// DXF in realen Maßen (mm), y nach oben.
pub fn dxf_bytes(doc: &Doc) -> Result<Vec<u8>, String> {
    use dxf::entities::{Entity, EntityType, LwPolyline, Text};
    use dxf::{Drawing, LwPolylineVertex, Point};
    let (_, h) = doc.page.size();
    let k = doc.scale_n as f64;
    let tp = |p: Pos| ((p.x as f64) * k, ((h - p.y) as f64) * k);
    let mut d = Drawing::new();
    // LWPOLYLINE gibt es erst ab R14 – sonst würden Linienzüge beim Speichern entfallen
    d.header.version = dxf::enums::AcadVersion::R2010;
    let add_poly = |d: &mut Drawing, pts: &[Pos], closed: bool, color: Color, layer: &str| {
        if pts.len() < 2 {
            return;
        }
        let mut pl = LwPolyline::default();
        for p in pts {
            let (x, y) = tp(*p);
            pl.vertices.push(LwPolylineVertex { x, y, ..Default::default() });
        }
        pl.set_is_closed(closed);
        let mut e = Entity::new(EntityType::LwPolyline(pl));
        e.common.color = dxf::Color::from_index(aci(color));
        e.common.layer = layer.to_string();
        d.add_entity(e);
    };
    let mut sym_cache: HashMap<String, Vec<(Vec<Pos>, bool)>> = HashMap::new();
    for prim in render::page_prims(doc, true) {
        match &prim {
            Prim::Fill { pts, color } => add_poly(&mut d, pts, true, *color, "FLAECHEN"),
            Prim::FillRings { rings, color } => {
                for r in rings {
                    add_poly(&mut d, r, true, *color, "FLAECHEN");
                }
            }
            Prim::Stroke { pts, stroke, closed } => add_poly(&mut d, pts, *closed, stroke.color, "LINIEN"),
            Prim::Text { pos, text, size, color, rot, .. } => {
                let base = *pos + Pos::new(0.0, size * 0.8).rotate(*rot);
                let (x, y) = tp(base);
                let mut t = Text::default();
                t.location = Point::new(x, y, 0.0);
                t.text_height = (*size as f64) * k * 0.72;
                t.value = text.clone();
                t.rotation = -(*rot as f64);
                let mut e = Entity::new(EntityType::Text(t));
                e.common.color = dxf::Color::from_index(aci(*color));
                e.common.layer = "TEXT".into();
                d.add_entity(e);
            }
            Prim::Symbol { id, c, size, rot } => {
                let polys = sym_cache.entry(id.clone()).or_insert_with(|| render::symbol_svg(doc, id).map(|s| symbol_polylines(&s)).unwrap_or_default());
                for (pts, closed) in polys.iter() {
                    let mapped: Vec<Pos> = pts.iter().map(|p| *c + (Pos::new(p.x - 50.0, p.y - 50.0) * (size / 100.0)).rotate(*rot)).collect();
                    add_poly(&mut d, &mapped, *closed, farbe::SCHWARZ, "SYMBOLE");
                }
            }
            Prim::Image { a, b, .. } => {
                let bb = BBox::from_two(*a, *b);
                add_poly(&mut d, &render::rect_pts(bb.min, bb.max), true, farbe::GRAU, "BILDER");
            }
        }
    }
    let mut out = Vec::new();
    d.save(&mut out).map_err(|e| format!("DXF-Fehler: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples;

    #[test]
    fn musterplaene_exportieren() {
        let dir = std::env::temp_dir().join("fluchtplan_test");
        std::fs::create_dir_all(&dir).unwrap();
        let docs: Vec<Doc> = samples::all().into_iter().map(|(_, d)| d).collect();
        let refs: Vec<&Doc> = docs.iter().collect();
        export_pdf(&refs, Stamp::Muster, &dir.join("muster.pdf")).expect("PDF");
        let pdf = std::fs::read(dir.join("muster.pdf")).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        export_dxf(&docs[0], &dir.join("muster.dxf")).expect("DXF");
        export_png(&docs[0], 60.0, &dir.join("muster.png")).expect("PNG");
        for (i, d) in docs.iter().enumerate() {
            export_png(d, 50.0, &dir.join(format!("muster_{i}.png"))).unwrap();
        }
    }
}
