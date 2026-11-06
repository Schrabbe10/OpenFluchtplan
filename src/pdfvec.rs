//! Vektor-Import von PDF-Seiten (z. B. CAD-Pläne von Planungsbüros): Linien, Flächen, Texte und
//! eingebettete Bilder werden zu einzelnen, bearbeitbaren Planobjekten. Koordinaten in mm,
//! Ursprung oben links auf der PDF-Seite.

use std::path::Path;

use hayro::hayro_interpret::font::{Glyph, GlyphRun};
use hayro::hayro_interpret::kurbo::{self, Affine, BezPath, PathEl, Point, Rect};
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_interpret::{
    BlendMode, ClipPath, Context, Device, DrawMode, DrawProps, Image, ImageData as HImage, ImageDrawProps, InterpreterCache, InterpreterSettings,
    Paint, SoftMask, interpret_page,
};

use crate::geom::BBox;
use crate::model::*;

const MM: f64 = 25.4 / 72.0;
/// Glyphen-Einheiten je Geviert (hayro)
const EM: f64 = 1000.0;

pub struct PdfItem {
    pub kind: ObjKind,
    pub style: Style,
    /// Farbe, nach der die Ebene gewählt wird
    pub color: Color,
}

pub struct PdfPage {
    pub w: f32,
    pub h: f32,
    pub items: Vec<PdfItem>,
}

struct TextRun {
    item: usize,
    end: Point,
    size: f64,
    angle: f64,
    color: Color,
    bold: bool,
    italic: bool,
}

type Contour = Vec<[f64; 2]>;

/// Wirksamer Clip-Bereich (Schnittmenge aller aktiven Clips), normalisiert als Konturen.
#[derive(Clone)]
struct Clip {
    contours: Vec<Contour>,
    bbox: BBox,
    /// einfaches achsenparalleles Rechteck (schneller Sonderfall)
    rect: bool,
}

impl Clip {
    fn contains_bbox(&self, b: &BBox) -> bool {
        self.rect && self.bbox.contains(b.min) && self.bbox.contains(b.max)
    }
    fn contains_point(&self, p: Pos) -> bool {
        if !self.bbox.contains(p) {
            return false;
        }
        let poly = |c: &Contour| c.iter().map(|q| Pos::new(q[0] as f32, q[1] as f32)).collect::<Vec<_>>();
        self.contours.iter().filter(|c| crate::geom::point_in_polygon(p, &poly(c))).count() % 2 == 1
    }
}

fn to_contours(rings: &[Vec<Pos>]) -> Vec<Contour> {
    rings.iter().map(|r| r.iter().map(|p| [p.x as f64, p.y as f64]).collect()).collect()
}

fn from_contour(c: &Contour) -> Vec<Pos> {
    c.iter().map(|q| Pos::new(q[0] as f32, q[1] as f32)).collect()
}

fn is_rect(c: &[Contour]) -> bool {
    c.len() == 1 && c[0].len() == 4 && {
        let p = &c[0];
        (0..4).all(|i| {
            let (a, b) = (p[i], p[(i + 1) % 4]);
            (a[0] - b[0]).abs() < 1e-3 || (a[1] - b[1]).abs() < 1e-3
        })
    }
}

fn fill_rule(r: hayro::hayro_interpret::FillRule) -> i_overlay::core::fill_rule::FillRule {
    match r {
        hayro::hayro_interpret::FillRule::EvenOdd => i_overlay::core::fill_rule::FillRule::EvenOdd,
        _ => i_overlay::core::fill_rule::FillRule::NonZero,
    }
}

struct Collector {
    items: Vec<PdfItem>,
    page: BBox,
    last_text: Option<TextRun>,
    clips: Vec<Option<Clip>>,
}

fn pos(p: Point) -> Pos {
    Pos::new(p.x as f32, p.y as f32)
}

fn paint_color(p: &Paint) -> Color {
    match p {
        Paint::Color(c) => {
            let [r, g, b, a] = c.to_rgba().to_rgba8();
            Color(r, g, b, a)
        }
        Paint::Pattern(_) => Color::rgb(0x80, 0x80, 0x80),
    }
}

/// Pfad in Teilzüge zerlegen (Kurven werden fein angenähert).
fn flatten(path: &BezPath, t: Affine) -> Vec<(Vec<Pos>, bool)> {
    let p = t * path.clone();
    let mut out: Vec<(Vec<Pos>, bool)> = Vec::new();
    let mut cur: Vec<Pos> = Vec::new();
    kurbo::flatten(p.iter(), 0.02, |el| match el {
        PathEl::MoveTo(q) => {
            if cur.len() > 1 {
                out.push((std::mem::take(&mut cur), false));
            }
            cur = vec![pos(q)];
        }
        PathEl::LineTo(q) => {
            let q = pos(q);
            if cur.last().is_none_or(|l| l.dist(q) > 1e-4) {
                cur.push(q);
            }
        }
        PathEl::ClosePath => {
            if cur.len() > 1 {
                if cur.len() > 2 && cur[0].dist(*cur.last().unwrap()) < 1e-3 {
                    cur.pop();
                }
                let start = cur[0];
                out.push((std::mem::take(&mut cur), true));
                cur.push(start);
            }
        }
        _ => {}
    });
    if cur.len() > 1 {
        out.push((cur, false));
    }
    out
}

fn line_width(t: Affine, w: f32) -> f32 {
    let det = t.determinant().abs().sqrt() as f32;
    (w * det).max(0.1)
}

impl Collector {
    fn push(&mut self, kind: ObjKind, style: Style, color: Color) {
        self.items.push(PdfItem { kind, style, color });
    }

    fn clip(&self) -> Option<&Clip> {
        self.clips.last().and_then(|c| c.as_ref())
    }

    /// Flächen mit dem Clip-Bereich schneiden; liefert je Teilfläche die Ringe (Außenkontur + Löcher).
    fn clip_fill(&self, rings: Vec<Vec<Pos>>, rule: hayro::hayro_interpret::FillRule) -> Vec<Vec<Vec<Pos>>> {
        use i_overlay::core::overlay_rule::OverlayRule;
        use i_overlay::float::single::SingleFloatOverlay;
        let bb = rings.iter().fold(BBox::EMPTY, |b, r| b.union(BBox::from_pts(r)));
        let Some(clip) = self.clip() else { return vec![rings] };
        if clip.contains_bbox(&bb) {
            return vec![rings];
        }
        if !clip.bbox.intersects(&bb) {
            return Vec::new();
        }
        // Subjekt zuerst normalisieren (Füllregel), dann mit dem normalisierten Clip schneiden
        let subj = to_contours(&rings).overlay(&Vec::<Contour>::new(), OverlayRule::Subject, fill_rule(rule));
        let subj: Vec<Contour> = subj.into_iter().flatten().collect();
        let res = subj.overlay(&clip.contours, OverlayRule::Intersect, i_overlay::core::fill_rule::FillRule::EvenOdd);
        // Fläche einer Form: Außenkontur minus Löcher
        let shape_area = |s: &[Contour]| {
            let a: Vec<f64> = s.iter().map(|c| (crate::geom::polygon_area(&from_contour(c)) as f64).abs()).collect();
            a.first().copied().unwrap_or(0.0) - a.iter().skip(1).sum::<f64>()
        };
        let subj_shapes = to_contours(&rings).overlay(&Vec::<Contour>::new(), OverlayRule::Subject, fill_rule(rule));
        let a_subj: f64 = subj_shapes.iter().map(|s| shape_area(s)).sum();
        // Splitter ohne Fläche verwerfen
        let res: Vec<_> = res.into_iter().filter(|s| shape_area(s) > 0.005).collect();
        let a_res: f64 = res.iter().map(|s| shape_area(s)).sum();
        // Fläche unverändert (Clip schneidet nichts ab) -> Original behalten, nicht zerstückeln
        if a_subj > 0.0 && (a_res - a_subj).abs() < a_subj * 0.002 {
            return vec![rings];
        }
        res.into_iter().map(|shape| shape.iter().map(from_contour).filter(|r| r.len() >= 3).collect::<Vec<_>>()).filter(|s: &Vec<Vec<Pos>>| !s.is_empty()).collect()
    }

    /// Linienzug mit dem Clip-Bereich zuschneiden.
    fn clip_stroke(&self, mut pts: Vec<Pos>, closed: bool) -> Vec<(Vec<Pos>, bool)> {
        use i_overlay::float::clip::FloatClip;
        use i_overlay::string::clip::ClipRule;
        let Some(clip) = self.clip() else { return vec![(pts, closed)] };
        let bb = BBox::from_pts(&pts);
        if clip.contains_bbox(&bb) {
            return vec![(pts, closed)];
        }
        if !clip.bbox.intersects(&bb) {
            return Vec::new();
        }
        if closed {
            pts.push(pts[0]);
        }
        let line: Contour = pts.iter().map(|p| [p.x as f64, p.y as f64]).collect();
        let res = line.clip_by(&clip.contours, i_overlay::core::fill_rule::FillRule::EvenOdd, ClipRule { invert: false, boundary_included: true });
        res.iter().map(|c| (from_contour(c), false)).filter(|(p, _)| p.len() >= 2).collect()
    }

    fn add_path(&mut self, path: &BezPath, props: &DrawProps, mode: &DrawMode) {
        let (fill, stroke) = match mode {
            DrawMode::Fill(r) => (Some(*r), None),
            DrawMode::Stroke(s) => (None, Some(s)),
            DrawMode::FillAndStroke(r, s) => (Some(*r), Some(s)),
            DrawMode::Invisible => return,
        };
        let color = paint_color(&props.paint);
        if color.3 == 0 {
            return;
        }
        let parts = flatten(path, props.transform);
        if parts.is_empty() {
            return;
        }
        let width = stroke.map(|s| line_width(props.transform, s.line_width));
        if let Some(rule) = fill {
            let rings: Vec<Vec<Pos>> = parts.iter().filter(|(p, _)| p.len() >= 3).map(|(p, _)| p.clone()).collect();
            if rings.is_empty() {
                return;
            }
            let bb = rings.iter().fold(BBox::EMPTY, |b, r| b.union(BBox::from_pts(r)));
            // weißer Seitenhintergrund ist kein Planinhalt
            if color.0 > 250 && color.1 > 250 && color.2 > 250 && bb.w() * bb.h() > self.page.w() * self.page.h() * 0.8 {
                return;
            }
            let mut style = Style { stroke: None, fill: Fill::Farbe(color) };
            if let Some(w) = width {
                style.stroke = Some(StrokeStyle::new(color, w));
            }
            for shape in self.clip_fill(rings, rule) {
                let kind = if shape.len() == 1 {
                    ObjKind::Polyline { pts: shape.into_iter().next().unwrap(), closed: true }
                } else {
                    ObjKind::Path { subpaths: shape, closed: true }
                };
                self.push(kind, style, color);
            }
        } else if let (Some(s), Some(w)) = (stroke, width) {
            let mut st = StrokeStyle::new(color, w);
            if !s.dash_array.is_empty() {
                st.dash = if s.dash_array.iter().all(|d| *d < 1.5) { Dash::Gepunktet } else { Dash::Gestrichelt };
            }
            let style = Style { stroke: Some(st), fill: Fill::Keine };
            // jeder Teilzug wird ein eigenes Objekt (einzeln bearbeitbar)
            for (pts, closed) in parts {
                for (pts, closed) in self.clip_stroke(pts, closed) {
                    let kind = if pts.len() == 2 && !closed { ObjKind::Line { a: pts[0], b: pts[1] } } else { ObjKind::Polyline { pts, closed } };
                    self.push(kind, style, color);
                }
            }
        }
        self.last_text = None;
    }
}

impl<'a> Device<'a> for Collector {
    fn draw_path(&mut self, path: &BezPath, props: DrawProps<'a>, draw_mode: &DrawMode) {
        self.add_path(path, &props, draw_mode);
    }
    fn push_clip_path(&mut self, c: &ClipPath) {
        use i_overlay::core::overlay_rule::OverlayRule;
        use i_overlay::float::single::SingleFloatOverlay;
        // Clip-Pfade liefert hayro bereits in Gerätekoordinaten (mm)
        let rings: Vec<Vec<Pos>> = flatten(&c.path, Affine::IDENTITY).into_iter().filter(|(p, _)| p.len() >= 3).map(|(p, _)| p).collect();
        let norm: Vec<Contour> = to_contours(&rings).overlay(&Vec::<Contour>::new(), OverlayRule::Subject, fill_rule(c.fill)).into_iter().flatten().collect();
        let contours = match self.clip() {
            Some(prev) => prev.contours.overlay(&norm, OverlayRule::Intersect, i_overlay::core::fill_rule::FillRule::EvenOdd).into_iter().flatten().collect(),
            None => norm,
        };
        let bbox = contours.iter().fold(BBox::EMPTY, |b, c| b.union(BBox::from_pts(&from_contour(c))));
        // Clip, der die ganze Seite umfasst, wirkt nicht
        let rect = is_rect(&contours);
        let clip = if rect && bbox.contains(self.page.min) && bbox.contains(self.page.max) { None } else { Some(Clip { rect, bbox, contours }) };
        self.clips.push(clip);
    }
    fn push_transparency_group(&mut self, _: f32, _: Option<SoftMask<'a>>, _: BlendMode) {}
    fn pop_clip(&mut self) {
        self.clips.pop();
    }
    fn pop_transparency_group(&mut self) {}

    fn draw_glyph_run(&mut self, run: &GlyphRun<'_, 'a>, props: DrawProps<'a>, draw_mode: &DrawMode) {
        if matches!(draw_mode, DrawMode::Invisible) {
            return;
        }
        let color = paint_color(&props.paint);
        // Schriftstil aus den eingebetteten Schriftdaten (Gewicht, Kursiv, PostScript-Name)
        let (bold, italic) = run
            .glyphs()
            .iter()
            .find_map(|g| match &**g {
                Glyph::Outline(o) => o.font_data(),
                _ => None,
            })
            .map(|fd| {
                let name = fd.postscript_name.unwrap_or_default().to_lowercase();
                (fd.weight.is_some_and(|w| w >= 600) || name.contains("bold") || name.contains("black"), fd.is_italic || name.contains("italic") || name.contains("oblique"))
            })
            .unwrap_or((false, false));
        for g in run.glyphs() {
            // Vorschub des Zeichens in Geviert (aus der Schrift, sonst geschätzt)
            let adv_em = |ch: &str| -> f64 {
                let a = match &**g {
                    Glyph::Outline(o) => o.advance_width(),
                    Glyph::Type3(t3) => t3.advance_width(),
                };
                a.filter(|a| *a > 0.0).map(|a| a as f64 / EM).unwrap_or_else(|| glyph_advance(ch))
            };
            let t = props.transform * g.transform();
            let origin = t * Point::ZERO;
            let xdir = t * Point::new(EM, 0.0) - origin;
            let ydir = t * Point::new(0.0, EM) - origin;
            let size = ydir.hypot();
            if size < 0.2 || self.clip().is_some_and(|c| !c.contains_point(pos(origin))) {
                continue;
            }
            let angle = xdir.y.atan2(xdir.x).to_degrees();
            let ch: String = match g.as_unicode() {
                Some(hayro::hayro_interpret::hayro_cmap::BfString::Char(c)) => c.to_string(),
                Some(hayro::hayro_interpret::hayro_cmap::BfString::String(s)) => s,
                None => continue,
            };
            if ch.chars().all(|c| c.is_control()) {
                continue;
            }
            // an vorhandenen Text auf derselben Grundlinie anhängen?
            if let Some(last) = &mut self.last_text {
                let d = origin - last.end;
                let u = kurbo::Vec2::new(angle.to_radians().cos(), angle.to_radians().sin());
                let along = d.dot(u);
                let across = d.cross(u).abs();
                // gleicher Stil, gleiche Grundlinie, Lücke höchstens ein Wortabstand (Spalten bleiben getrennt)
                if (last.size - size).abs() < size * 0.05
                    && (last.angle - angle).abs() < 1.0
                    && last.color == color
                    && last.bold == bold
                    && last.italic == italic
                    && across < size * 0.2
                    && along > -size * 0.35
                    && along < size * 0.6
                {
                    if let ObjKind::Text { text, .. } = &mut self.items[last.item].kind {
                        if along > size * 0.15 && !text.ends_with(' ') && ch != " " {
                            text.push(' ');
                        }
                        text.push_str(&ch);
                    }
                    last.end = origin + u * (size * adv_em(&ch));
                    continue;
                }
            }
            if ch.trim().is_empty() {
                continue;
            }
            let base = pos(origin);
            let top = base - Pos::new(0.0, size as f32 * 0.8).rotate(angle as f32);
            self.push(
                ObjKind::Text { pos: top, text: ch.clone(), size: size as f32, bold, italic, align: TAlign::Links, rot: angle as f32 },
                Style { stroke: Some(StrokeStyle::new(color, 0.2)), fill: Fill::Keine },
                color,
            );
            let u = kurbo::Vec2::new(angle.to_radians().cos(), angle.to_radians().sin());
            self.last_text = Some(TextRun { item: self.items.len() - 1, end: origin + u * (size * adv_em(&ch)), size, angle, color, bold, italic });
        }
    }

    fn draw_image(&mut self, image: Image<'a, '_>, props: ImageDrawProps<'a>) {
        let t = props.transform;
        let p00 = t * Point::new(0.0, 0.0);
        let p10 = t * Point::new(1.0, 0.0);
        let p01 = t * Point::new(0.0, 1.0);
        let p11 = t * Point::new(1.0, 1.0);
        let bb = BBox::from_pts(&[pos(p00), pos(p10), pos(p01), pos(p11)]);
        if bb.w() < 0.5 || bb.h() < 0.5 || self.clip().is_some_and(|c| !c.bbox.intersects(&bb)) {
            return;
        }
        let mut rgba: Option<image::RgbaImage> = None;
        match image {
            Image::Raster(r) => r.with_rgba(
                |data, alpha| {
                    let (w, h) = (data.width(), data.height());
                    let mut img = image::RgbaImage::new(w, h);
                    for (i, px) in img.pixels_mut().enumerate() {
                        let rgb = match &data {
                            HImage::Rgb(d) => [d.data[i * 3], d.data[i * 3 + 1], d.data[i * 3 + 2]],
                            HImage::Luma(d) => [d.data[i]; 3],
                        };
                        let a = alpha.as_ref().filter(|m| m.width == w && m.height == h).map(|m| m.data[i]).unwrap_or(255);
                        *px = image::Rgba([rgb[0], rgb[1], rgb[2], a]);
                    }
                    rgba = Some(img);
                },
                None,
            ),
            Image::Stencil(s) => s.with_stencil(
                |luma, paint| {
                    let c = paint_color(paint);
                    let mut img = image::RgbaImage::new(luma.width, luma.height);
                    for (i, px) in img.pixels_mut().enumerate() {
                        *px = image::Rgba([c.0, c.1, c.2, luma.data[i]]);
                    }
                    rgba = Some(img);
                },
                None,
            ),
        }
        let Some(mut img) = rgba else { return };
        // Bildzeile 0 liegt im PDF bei y = 1 des Einheitsquadrats
        if p01.y > p00.y {
            image::imageops::flip_vertical_in_place(&mut img);
        }
        if p10.x < p00.x {
            image::imageops::flip_horizontal_in_place(&mut img);
        }
        let mut png = Vec::new();
        if image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).is_err() {
            return;
        }
        if let Ok(data) = crate::import::image_data(&png, "image/png") {
            self.push(ObjKind::Image { a: bb.min, b: bb.max, data, opacity: 1.0 }, Style::NONE, Color::rgb(0x80, 0x80, 0x80));
        }
        self.last_text = None;
    }
}

/// Grobe Zeichenbreite in Geviert (für das Zusammenfügen einzeln gesetzter Zeichen).
fn glyph_advance(s: &str) -> f64 {
    s.chars()
        .map(|c| match c {
            'i' | 'j' | 'l' | 'I' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' => 0.28,
            'f' | 't' | 'r' | ' ' | '(' | ')' | '-' => 0.36,
            'm' | 'w' | 'M' | 'W' => 0.85,
            c if c.is_uppercase() || c.is_ascii_digit() => 0.62,
            _ => 0.52,
        })
        .sum()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PdfFit {
    /// in die Planfläche einpassen
    #[default]
    Einpassen,
    /// 1:1 in Blatt-Millimetern
    Massstab,
    /// Blattformat der PDF übernehmen, Rahmen ausblenden (vorhandenen Plan weiterbearbeiten)
    Formatfuellend,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PdfImportOpt {
    pub vector: bool,
    pub group: bool,
    /// Einzelteile von Symbolen (Hintergrund, Rahmen, Zeichen, Text) zu Gruppen zusammenfassen
    pub symbols: bool,
    pub text_pct: f32,
    pub fit: PdfFit,
}

impl Default for PdfImportOpt {
    fn default() -> Self {
        Self { vector: true, group: false, symbols: true, text_pct: 100.0, fit: PdfFit::Einpassen }
    }
}

/// Größte Kantenlänge (Blatt-mm), bis zu der Teile beim Import als ein Symbol gelten.
pub const SYMBOL_MAX_MM: f32 = 12.0;

/// Importierte Seite in den Plan einsetzen (Platzierung, Ebenen je Farbe, optional als Gruppe).
/// Liefert die IDs der neuen Objekte.
pub fn place(doc: &mut Doc, pg: PdfPage, opt: PdfImportOpt) -> Vec<u64> {
    use crate::{ops, templates};
    let (f, offset) = match opt.fit {
        PdfFit::Formatfuellend => {
            let (w, h) = (pg.w.max(pg.h), pg.w.min(pg.h));
            let format = PaperFormat::LISTE
                .into_iter()
                .find(|f| {
                    let (fw, fh) = f.size();
                    (fh - w).abs() < 3.0 && (fw - h).abs() < 3.0
                })
                .unwrap_or(PaperFormat::Eigenes { w, h });
            doc.page = Page { format, landscape: pg.w > pg.h };
            doc.frame = FrameKind::Ohne;
            (1.0, Pos::new(0.0, 0.0))
        }
        PdfFit::Massstab => (1.0, templates::layout(doc).plan.min),
        PdfFit::Einpassen => {
            let area = templates::layout(doc).plan;
            let f = (area.w() / pg.w).min(area.h() / pg.h);
            (f, area.min + Pos::new((area.w() - pg.w * f) / 2.0, (area.h() - pg.h * f) / 2.0))
        }
    };
    let mut objs = Vec::with_capacity(pg.items.len());
    for mut it in pg.items {
        ops::scale(&mut it.kind, Pos::new(0.0, 0.0), f);
        ops::translate(&mut it.kind, offset);
        if let ObjKind::Text { size, .. } = &mut it.kind {
            *size *= opt.text_pct / 100.0;
        }
        if let Some(s) = &mut it.style.stroke {
            s.width = (s.width * f).max(0.05);
        }
        let layer = doc.find_layer_by_name(&format!("PDF {}", it.color.hex()));
        let id = doc.alloc_id();
        objs.push(Obj { id, layer, kind: it.kind, style: it.style, print: true, locked: false });
    }
    if opt.group {
        let layer = doc.find_layer_by_name("PDF-Import");
        let id = doc.alloc_id();
        doc.objects.push(Obj { id, layer, kind: ObjKind::Group { children: objs }, style: Style::default(), print: true, locked: false });
        vec![id]
    } else {
        let ids: Vec<u64> = objs.iter().map(|o| o.id).collect();
        let start = doc.objects.len();
        doc.objects.extend(objs);
        if opt.symbols {
            ops::symbole_zusammenfassen(doc, &ids, SYMBOL_MAX_MM * f.max(0.05));
            // erkannte Zeichen durch die Normzeichen der Bibliothek ersetzen (Legende, Prüfung)
            let neu: Vec<u64> = doc.objects[start..].iter().map(|o| o.id).collect();
            crate::erkennung::normzeichen_einsetzen(doc, Some(&neu));
            return doc.objects[start..].iter().map(|o| o.id).collect();
        }
        ids
    }
}

/// Liest eine PDF-Seite als Vektorobjekte.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn read_page(path: &Path, page: usize) -> Result<PdfPage, String> {
    read_page_bytes(std::fs::read(path).map_err(|e| e.to_string())?, page)
}

pub fn read_page_bytes(bytes: Vec<u8>, page: usize) -> Result<PdfPage, String> {
    let pdf = hayro::hayro_syntax::Pdf::new(bytes).map_err(|e| format!("PDF nicht lesbar: {e:?}"))?;
    let pages = pdf.pages();
    let pg = pages.get(page).ok_or("Seite nicht vorhanden")?;
    let (w, h) = pg.render_dimensions();
    let (w, h) = (w as f64 * MM, h as f64 * MM);
    let transform = Affine::scale(MM) * pg.initial_transform(true).to_kurbo();
    let cache = InterpreterCache::new();
    let mut ctx = Context::new(transform, Rect::new(0.0, 0.0, w, h), &cache, pg.xref(), InterpreterSettings::default());
    let mut dev = Collector { items: Vec::new(), page: BBox::from_two(Pos::new(0.0, 0.0), Pos::new(w as f32, h as f32)), last_text: None, clips: Vec::new() };
    interpret_page(pg, &mut ctx, &mut dev);
    // Leerzeichen am Textende entfernen
    for it in &mut dev.items {
        if let ObjKind::Text { text, .. } = &mut it.kind {
            *text = text.trim_end().to_string();
        }
    }
    dev.items.retain(|it| !matches!(&it.kind, ObjKind::Text { text, .. } if text.is_empty()));
    Ok(PdfPage { w: w as f32, h: h as f32, items: dev.items })
}
