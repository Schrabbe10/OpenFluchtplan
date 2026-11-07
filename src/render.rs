//! Zerlegt Objekte, Legende und Rahmen in Grafik-Primitive (mm).
//! Dieselben Primitive zeichnen Bildschirm (egui) und Export (SVG/PDF).

use crate::geom::*;
use crate::model::*;
use crate::{i18n, ops, presets, symbols, templates};

#[derive(Clone, Debug, PartialEq)]
pub struct PStroke {
    pub color: Color,
    pub width: f32,
    pub dash: Option<Vec<f32>>,
    /// runde Enden/Ecken (sonst eckig)
    pub round: bool,
}

impl PStroke {
    pub fn solid(color: Color, width: f32) -> Self {
        Self { color, width, dash: None, round: true }
    }
    fn from_style(s: &StrokeStyle) -> Self {
        Self { color: s.color, width: s.width, dash: s.dash.pattern(s.width), round: true }
    }
}

#[derive(Clone, Debug)]
pub enum Prim<'a> {
    /// gefülltes Polygon (auch konkav)
    Fill { pts: Vec<Pos>, color: Color },
    /// gefüllte Fläche aus mehreren Ringen (Gerade-Ungerade-Regel, Löcher)
    FillRings { rings: Vec<Vec<Pos>>, color: Color },
    Stroke { pts: Vec<Pos>, stroke: PStroke, closed: bool },
    /// `pos` = Oberkante der Zeile am Ankerpunkt
    Text { pos: Pos, text: String, size: f32, color: Color, align: TAlign, bold: bool, italic: bool, rot: f32 },
    Symbol { id: String, c: Pos, size: f32, rot: f32 },
    Image { key: u64, a: Pos, b: Pos, data: &'a ImageData, opacity: f32 },
}

pub type Prims<'a> = Vec<Prim<'a>>;

pub fn text(out: &mut Prims, pos: Pos, s: impl Into<String>, size: f32, color: Color, align: TAlign, bold: bool) {
    let s = s.into();
    if s.is_empty() {
        return;
    }
    out.push(Prim::Text { pos, text: s, size, color, align, bold, italic: false, rot: 0.0 });
}

pub fn rect_pts(a: Pos, b: Pos) -> Vec<Pos> {
    vec![a, Pos::new(b.x, a.y), b, Pos::new(a.x, b.y)]
}

pub fn fill_rect(out: &mut Prims, a: Pos, b: Pos, color: Color) {
    out.push(Prim::Fill { pts: rect_pts(a, b), color });
}

pub fn frame_rect(out: &mut Prims, a: Pos, b: Pos, width: f32, color: Color) {
    out.push(Prim::Stroke { pts: rect_pts(a, b), stroke: PStroke { round: false, ..PStroke::solid(color, width) }, closed: true });
}

pub fn line(out: &mut Prims, a: Pos, b: Pos, width: f32, color: Color) {
    out.push(Prim::Stroke { pts: vec![a, b], stroke: PStroke::solid(color, width), closed: false });
}

/// Fläche (Füllung/Schraffur) + Umriss nach Stil.
pub fn style_prims(out: &mut Prims, pts: &[Pos], closed: bool, style: &Style) {
    if closed && pts.len() >= 3 {
        match style.fill {
            Fill::Keine => {}
            Fill::Farbe(c) => out.push(Prim::Fill { pts: pts.to_vec(), color: c }),
            Fill::Schraffur(h) => {
                if let Some(bg) = h.bg {
                    out.push(Prim::Fill { pts: pts.to_vec(), color: bg });
                }
                let mut angles = vec![h.angle];
                if h.cross {
                    angles.push(h.angle + 90.0);
                }
                for ang in angles {
                    for (a, b) in hatch_segments(pts, ang, h.spacing) {
                        out.push(Prim::Stroke { pts: vec![a, b], stroke: PStroke { round: false, ..PStroke::solid(h.color, h.width) }, closed: false });
                    }
                }
            }
        }
    }
    if let Some(s) = &style.stroke {
        if pts.len() >= 2 {
            out.push(Prim::Stroke { pts: pts.to_vec(), stroke: PStroke::from_style(s), closed });
        }
    }
}

fn chevrons(out: &mut Prims, pts: &[Pos], width: f32, color: Color) {
    let h = (width * 0.32).max(0.8);
    for (p, dir) in points_along(pts, width * 1.2, (width * 2.4).max(6.0)) {
        let n = dir.perp();
        out.push(Prim::Fill { pts: vec![p + dir * h, p - dir * h + n * h, p - dir * (h * 0.4), p - dir * h - n * h], color });
    }
}

fn arrow_head(out: &mut Prims, tip: Pos, dir: Pos, size: f32, color: Color) {
    let n = dir.perp();
    out.push(Prim::Fill { pts: vec![tip, tip - dir * size + n * (size * 0.45), tip - dir * size - n * (size * 0.45)], color });
}

fn cloud_pts(poly: &[Pos]) -> Vec<Pos> {
    let mut out = Vec::new();
    let n = poly.len();
    if n < 2 {
        return poly.to_vec();
    }
    let ccw = {
        let mut a = 0.0;
        for i in 0..n {
            let (p, q) = (poly[i], poly[(i + 1) % n]);
            a += p.x * q.y - q.x * p.y;
        }
        a > 0.0
    };
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let len = a.dist(b);
        let k = (len / 5.0).ceil().max(1.0) as usize;
        for j in 0..k {
            let p0 = a.lerp(b, j as f32 / k as f32);
            let p1 = a.lerp(b, (j + 1) as f32 / k as f32);
            let c = p0.lerp(p1, 0.5);
            let r = p0.dist(p1) / 2.0;
            let start = (p0 - c).angle_deg();
            let sweep = if ccw { -180.0 } else { 180.0 };
            out.extend(arc_pts(c, r, r, start, start + sweep));
        }
    }
    out
}

pub fn qr_modules(text: &str) -> Option<(usize, Vec<bool>)> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let w = code.width();
    let colors = code.to_colors();
    Some((w, colors.iter().map(|c| *c == qrcode::Color::Dark).collect()))
}

fn door_prims(out: &mut Prims, pos: Pos, width: f32, rot: f32, mirror: bool, double: bool, stroke: Color) {
    let dir = ops::door_dir(rot);
    let side = if mirror { -1.0 } else { 1.0 };
    let n = dir.perp() * side;
    // Wandöffnung freistellen
    let wall = 1.2;
    out.push(Prim::Fill { pts: vec![pos - n * wall, pos + dir * width - n * wall, pos + dir * width + n * wall * 0.2, pos + n * wall * 0.2], color: farbe::WEISS });
    let leaves: Vec<(Pos, Pos, f32)> = if double {
        let h = width / 2.0;
        vec![(pos, dir, h), (pos + dir * width, -dir, h)]
    } else {
        vec![(pos, dir, width)]
    };
    for (hinge, d, w) in leaves {
        let open_end = hinge + d.perp() * side * w;
        // Türblatt
        out.push(Prim::Stroke { pts: vec![hinge, open_end], stroke: PStroke::solid(stroke, 0.35), closed: false });
        // Schwenkbereich
        let a0 = d.angle_deg();
        let a1 = (d.perp() * side).angle_deg();
        let mut sweep = a1 - a0;
        if sweep > 180.0 {
            sweep -= 360.0;
        }
        if sweep < -180.0 {
            sweep += 360.0;
        }
        out.push(Prim::Stroke { pts: arc_pts(hinge, w, w, a0, a0 + sweep), stroke: PStroke::solid(stroke, 0.18), closed: false });
    }
}

fn stairs_prims(out: &mut Prims, a: Pos, b: Pos, width: f32, steps: u32, color: Color) {
    let dir = (b - a).norm();
    let n = dir.perp() * (width / 2.0);
    let outline = vec![a + n, b + n, b - n, a - n];
    out.push(Prim::Fill { pts: outline.clone(), color: farbe::WEISS });
    out.push(Prim::Stroke { pts: outline, stroke: PStroke { round: false, ..PStroke::solid(color, 0.35) }, closed: true });
    let steps = steps.max(2);
    for i in 1..steps {
        let p = a.lerp(b, i as f32 / steps as f32);
        out.push(Prim::Stroke { pts: vec![p + n, p - n], stroke: PStroke::solid(color, 0.2), closed: false });
    }
    let len = a.dist(b);
    out.push(Prim::Stroke { pts: vec![a + dir * (len * 0.08), b - dir * (len * 0.12)], stroke: PStroke::solid(color, 0.25), closed: false });
    arrow_head(out, b - dir * (len * 0.04), dir, (width * 0.3).min(len * 0.15).max(1.0), color);
}

fn fire_curve_prims(out: &mut Prims, doc: &Doc, c: Pos, r_out: f32, width: f32, start: f32, end: f32) {
    let r_in = (r_out - width).max(0.1);
    let mut outer = arc_pts(c, r_out, r_out, start, end);
    let mut inner = arc_pts(c, r_in, r_in, start, end);
    inner.reverse();
    let mut band = outer.clone();
    band.extend(inner.iter().copied());
    out.push(Prim::Fill { pts: band, color: Color::rgb(0xa0, 0xa0, 0xa0) });
    out.push(Prim::Stroke { pts: std::mem::take(&mut outer), stroke: PStroke::solid(farbe::SCHWARZ, 0.3), closed: false });
    inner.reverse();
    out.push(Prim::Stroke { pts: inner, stroke: PStroke::solid(farbe::SCHWARZ, 0.3), closed: false });
    let mid = c + Pos::new(r_out - width / 2.0, 0.0).rotate((start + end) / 2.0);
    let label = format!("R {:.1} m / B {:.1} m", doc.real_m(r_out), doc.real_m(width));
    out.push(Prim::Text { pos: mid, text: label, size: 2.5, color: farbe::SCHWARZ, align: TAlign::Mitte, bold: false, italic: false, rot: 0.0 });
}

/// Geschoss-Schnitt nach DIN 14095: Gebäude als Stapel von Geschossen, Erdgeleiche als dicke Linie,
/// das dargestellte Geschoss rot hinterlegt.
fn floor_section_prims(out: &mut Prims, a: Pos, b: Pos, floors: &[String], current: usize) {
    let bb = BBox::from_two(a, b);
    let n = floors.len().max(1);
    let fh = bb.h() / n as f32;
    let x0 = bb.min.x + bb.w() * 0.08;
    let x1 = bb.max.x - bb.w() * 0.08;
    let ts = (fh * 0.55).clamp(1.8, 5.0);
    for (i, name) in floors.iter().enumerate() {
        let y0 = bb.min.y + fh * i as f32;
        let (p0, p1) = (Pos::new(x0, y0), Pos::new(x1, y0 + fh));
        let cur = i == current;
        fill_rect(out, p0, p1, if cur { farbe::ROT } else { farbe::WEISS });
        frame_rect(out, p0, p1, 0.3, farbe::SCHWARZ);
        let c = if cur { farbe::WEISS } else { farbe::SCHWARZ };
        text(out, Pos::new((x0 + x1) / 2.0, y0 + (fh - ts) / 2.0), name.clone(), ts, c, TAlign::Mitte, cur);
    }
    // Geländeoberfläche unter dem Erdgeschoss (sonst unter dem untersten Geschoss)
    let eg = floors
        .iter()
        .position(|f| {
            let f = f.trim().to_lowercase();
            f == "eg" || f.starts_with("erdgeschoss") || f.starts_with("eg ")
        })
        .unwrap_or(n - 1);
    let gy = bb.min.y + fh * (eg + 1) as f32;
    out.push(Prim::Stroke { pts: vec![Pos::new(bb.min.x, gy), Pos::new(bb.max.x, gy)], stroke: PStroke { round: false, ..PStroke::solid(farbe::SCHWARZ, 0.8) }, closed: false });
}

fn grid_prims(out: &mut Prims, a: Pos, b: Pos, nx: u32, ny: u32, color: Color) {
    let bb = BBox::from_two(a, b);
    frame_rect(out, bb.min, bb.max, 0.35, color);
    let (nx, ny) = (nx.max(1), ny.max(1));
    let dash = Some(vec![2.0, 1.5]);
    let cw = bb.w() / nx as f32;
    let ch = bb.h() / ny as f32;
    for i in 1..nx {
        let x = bb.min.x + cw * i as f32;
        out.push(Prim::Stroke { pts: vec![Pos::new(x, bb.min.y), Pos::new(x, bb.max.y)], stroke: PStroke { dash: dash.clone(), ..PStroke::solid(color, 0.2) }, closed: false });
    }
    for j in 1..ny {
        let y = bb.min.y + ch * j as f32;
        out.push(Prim::Stroke { pts: vec![Pos::new(bb.min.x, y), Pos::new(bb.max.x, y)], stroke: PStroke { dash: dash.clone(), ..PStroke::solid(color, 0.2) }, closed: false });
    }
    let s = (cw.min(ch) * 0.25).clamp(2.0, 5.0);
    for i in 0..nx {
        let x = bb.min.x + cw * (i as f32 + 0.5);
        let l = char::from(b'A' + (i % 26) as u8).to_string();
        text(out, Pos::new(x, bb.min.y - s * 1.3), l.clone(), s, color, TAlign::Mitte, true);
        text(out, Pos::new(x, bb.max.y + s * 0.3), l, s, color, TAlign::Mitte, true);
    }
    for j in 0..ny {
        let y = bb.min.y + ch * (j as f32 + 0.5) - s * 0.5;
        text(out, Pos::new(bb.min.x - s * 0.4, y), (j + 1).to_string(), s, color, TAlign::Rechts, true);
        text(out, Pos::new(bb.max.x + s * 0.4, y), (j + 1).to_string(), s, color, TAlign::Links, true);
    }
}

fn dimension_prims(out: &mut Prims, doc: &Doc, a: Pos, b: Pos, offset: f32, color: Color) {
    let dir = (b - a).norm();
    let n = dir.perp() * offset;
    let (pa, pb) = (a + n, b + n);
    let ext = dir.perp() * offset.signum() * 1.5;
    out.push(Prim::Stroke { pts: vec![a, pa + ext], stroke: PStroke::solid(color, 0.18), closed: false });
    out.push(Prim::Stroke { pts: vec![b, pb + ext], stroke: PStroke::solid(color, 0.18), closed: false });
    out.push(Prim::Stroke { pts: vec![pa, pb], stroke: PStroke::solid(color, 0.25), closed: false });
    let tick = (dir + dir.perp()).norm() * 1.2;
    for p in [pa, pb] {
        out.push(Prim::Stroke { pts: vec![p - tick, p + tick], stroke: PStroke::solid(color, 0.35), closed: false });
    }
    let mut ang = dir.angle_deg();
    if ang > 90.0 || ang < -90.0 {
        ang += 180.0;
    }
    let mid = pa.lerp(pb, 0.5) + Pos::new(0.0, -3.4).rotate(ang);
    let label = format!("{:.2} m", doc.real_m(a.dist(b)));
    out.push(Prim::Text { pos: mid, text: label, size: 2.8, color, align: TAlign::Mitte, bold: false, italic: false, rot: ang });
}

/// Spaltenzahl und Schriftgröße, mit denen alle Legendeneinträge in das Feld passen: erst mehr
/// Spalten, dann kleinere Schrift (bis etwa 60 %, darunter wird es unleserlich).
pub fn legend_fit(doc: &Doc, bb: BBox, u: f32) -> (u32, f32) {
    let used: Vec<String> = ops::used_legend_keys(doc).into_iter().map(|(k, _)| k).collect();
    let n = doc.legend.entries.iter().filter(|e| e.visible && used.contains(&e.key)).count().max(1);
    let langs = doc.legend.langs.len() as f32;
    let breit = bb.w() > bb.h() * 2.0;
    let grund = if breit { ((bb.w() / (70.0 * u)).floor() as u32).max(1) } else { 1 };
    let mut size = 3.4 * u;
    let min = 3.4 * u * 0.6;
    loop {
        let row_h = (size * 2.2).max(size * (1.3 + 1.15 * langs));
        let y0 = bb.min.y + size * 0.8 * 0.6 + size * 2.4;
        let rows = (((bb.max.y - y0) / row_h).floor() as usize).max(1);
        // Spaltenbreite: Zeichen plus etwa 22 Zeichen Text
        let max_cols = (((bb.w() - size * 0.8) / (size * 16.0)).floor() as u32).max(1);
        let noetig = n.div_ceil(rows) as u32;
        if noetig <= max_cols || size <= min {
            return (noetig.max(grund).min(max_cols.max(grund)), size);
        }
        size = (size * 0.92).max(min);
    }
}

/// Legende in ein Rechteck setzen (mehrspaltig, optional mehrsprachig).
pub fn legend_prims(out: &mut Prims, doc: &Doc, a: Pos, b: Pos, cols: u32, size: f32, with_title: bool) {
    let bb = BBox::from_two(a, b);
    let used: Vec<String> = ops::used_legend_keys(doc).into_iter().map(|(k, _)| k).collect();
    let entries: Vec<&LegendEntry> = doc.legend.entries.iter().filter(|e| e.visible && used.contains(&e.key)).collect();
    let pad = size * 0.8;
    let mut y0 = bb.min.y + pad * 0.6;
    if with_title {
        let mut title = doc.legend.title.clone();
        for l in &doc.legend.langs {
            if let Some(t) = i18n::translate("title", *l) {
                title.push_str(" / ");
                title.push_str(t);
            }
        }
        text(out, Pos::new(bb.min.x + pad, y0), title, size * 1.3, farbe::SCHWARZ, TAlign::Links, true);
        y0 += size * 2.4;
    }
    let cols = cols.max(1) as usize;
    let langs = doc.legend.langs.len();
    let row_h = (size * 2.2).max(size * (1.3 + 1.15 * langs as f32));
    let icon = size * 1.9;
    let col_w = (bb.w() - pad) / cols as f32;
    let rows_avail = (((bb.max.y - y0) / row_h).floor() as usize).max(1);
    for (i, e) in entries.iter().enumerate() {
        let col = i / rows_avail;
        let row = i % rows_avail;
        if col >= cols {
            break;
        }
        let x = bb.min.x + pad + col as f32 * col_w;
        let y = y0 + row as f32 * row_h;
        let c = Pos::new(x + icon / 2.0, y + row_h / 2.0 - size * 0.2);
        legend_sample(out, doc, &e.key, c, icon);
        let tx = x + icon + size * 0.8;
        let lines = 1 + langs;
        let ty = c.y - (lines as f32 * size * 1.15) / 2.0;
        text(out, Pos::new(tx, ty), e.label.clone(), size, farbe::SCHWARZ, TAlign::Links, false);
        for (k, l) in doc.legend.langs.iter().enumerate() {
            if let Some(t) = i18n::translate(&e.key, *l) {
                out.push(Prim::Text { pos: Pos::new(tx, ty + (k + 1) as f32 * size * 1.15), text: t.to_string(), size: size * 0.9, color: Color::rgb(0x44, 0x44, 0x44), align: TAlign::Links, bold: false, italic: true, rot: 0.0 });
            }
        }
    }
}

fn legend_sample(out: &mut Prims, _doc: &Doc, key: &str, c: Pos, s: f32) {
    let h = s / 2.0;
    if let Some(id) = key.strip_prefix("sym:") {
        out.push(Prim::Symbol { id: id.to_string(), c, size: s, rot: 0.0 });
    } else if let Some(k) = key.strip_prefix("area:") {
        if let Some(p) = presets::find(k) {
            let pts = rect_pts(c - Pos::new(h, h * 0.7), c + Pos::new(h, h * 0.7));
            let mut st = p.style;
            if st.stroke.is_none() {
                st.stroke = Some(StrokeStyle::new(Color::rgb(0x66, 0x66, 0x66), 0.15));
            }
            style_prims(out, &pts, true, &st);
        }
    } else {
        match key {
            "route" => {
                let pts = vec![c - Pos::new(h, 0.0), c + Pos::new(h, 0.0)];
                out.push(Prim::Stroke { pts: pts.clone(), stroke: PStroke { round: false, ..PStroke::solid(farbe::FLUCHT_HELL, s * 0.7) }, closed: false });
                arrow_head(out, c + Pos::new(h * 0.45, 0.0), Pos::new(1.0, 0.0), s * 0.4, farbe::GRUEN);
            }
            "laufweg" => {
                let pts = vec![c - Pos::new(h, 0.0), c + Pos::new(h, 0.0)];
                out.push(Prim::Stroke { pts, stroke: PStroke { round: false, ..PStroke::solid(farbe::ROT, s * 0.35) }, closed: false });
                arrow_head(out, c + Pos::new(h * 0.3, 0.0), Pos::new(1.0, 0.0), s * 0.25, farbe::WEISS);
            }
            "stairs" => stairs_prims(out, c - Pos::new(h, 0.0), c + Pos::new(h, 0.0), s * 0.7, 5, farbe::SCHWARZ),
            "fence" => fence_prims(out, &[c - Pos::new(h, 0.0), c + Pos::new(h, 0.0)], farbe::SCHWARZ),
            "rail" => rail_prims(out, &[c - Pos::new(h, 0.0), c + Pos::new(h, 0.0)], farbe::SCHWARZ),
            "hose" => {
                let n = Pos::new(0.0, s * 0.15);
                line(out, c - Pos::new(h, 0.0) + n, c + Pos::new(h, 0.0) + n, 0.25, farbe::SCHWARZ);
                line(out, c - Pos::new(h, 0.0) - n, c + Pos::new(h, 0.0) - n, 0.25, farbe::SCHWARZ);
            }
            _ => {}
        }
    }
}

/// Primitive eines Objekts. `printing` blendet Notizen und nicht druckbare Objekte aus.
pub fn obj_prims<'a>(out: &mut Prims<'a>, doc: &'a Doc, o: &'a Obj, printing: bool) {
    if printing && !o.print {
        return;
    }
    let st = &o.style;
    let col = st.stroke.map(|s| s.color).unwrap_or(farbe::SCHWARZ);
    match &o.kind {
        ObjKind::Line { a, b } => style_prims(out, &[*a, *b], false, st),
        ObjKind::Polyline { pts, closed } => style_prims(out, pts, *closed, st),
        ObjKind::Rect { a, b, radius } => {
            let bb = BBox::from_two(*a, *b);
            let r = radius.min(bb.w() / 2.0).min(bb.h() / 2.0);
            let pts = if r > 0.05 {
                let mut v = Vec::new();
                v.extend(arc_pts(Pos::new(bb.max.x - r, bb.min.y + r), r, r, -90.0, 0.0));
                v.extend(arc_pts(Pos::new(bb.max.x - r, bb.max.y - r), r, r, 0.0, 90.0));
                v.extend(arc_pts(Pos::new(bb.min.x + r, bb.max.y - r), r, r, 90.0, 180.0));
                v.extend(arc_pts(Pos::new(bb.min.x + r, bb.min.y + r), r, r, 180.0, 270.0));
                v
            } else {
                rect_pts(bb.min, bb.max)
            };
            style_prims(out, &pts, true, st);
        }
        ObjKind::Circle { c, r } => style_prims(out, &ellipse_pts(*c, *r, *r), true, st),
        ObjKind::Ellipse { c, rx, ry } => style_prims(out, &ellipse_pts(*c, *rx, *ry), true, st),
        ObjKind::Arc { c, r, start, end } => style_prims(out, &arc_pts(*c, *r, *r, *start, *end), false, st),
        ObjKind::NGon { c, r, n, rot } => style_prims(out, &ngon_pts(*c, *r, *n, *rot), true, st),
        ObjKind::Spline { pts } => style_prims(out, &spline_pts(pts), false, st),
        ObjKind::Hose { pts, width } => {
            let s = st.stroke.unwrap_or(StrokeStyle::new(farbe::SCHWARZ, 0.25));
            let p = spline_pts(pts);
            for d in [width / 2.0, -width / 2.0] {
                out.push(Prim::Stroke { pts: offset_polyline(&p, d, false), stroke: PStroke::from_style(&s), closed: false });
            }
        }
        ObjKind::Text { pos, text: t, size, bold, italic, align, rot } => {
            let c = st.stroke.map(|s| s.color).unwrap_or(farbe::SCHWARZ);
            let t = ops::platzhalter(doc, t);
            for (i, l) in t.lines().enumerate() {
                let p = *pos + Pos::new(0.0, i as f32 * size * 1.25).rotate(*rot);
                out.push(Prim::Text { pos: p, text: l.to_string(), size: *size, color: c, align: *align, bold: *bold, italic: *italic, rot: *rot });
            }
        }
        ObjKind::Image { a, b, data, opacity } => out.push(Prim::Image { key: o.id, a: *a, b: *b, data, opacity: *opacity }),
        ObjKind::Dimension { a, b, offset } => dimension_prims(out, doc, *a, *b, *offset, col),
        ObjKind::Table { pos, cols, rows, cells, size } => {
            let (w, h) = ops::table_size(cols, rows);
            if let Fill::Farbe(f) = st.fill {
                fill_rect(out, *pos, *pos + Pos::new(w, h), f);
            }
            frame_rect(out, *pos, *pos + Pos::new(w, h), 0.35, col);
            let mut x = pos.x;
            for cw in cols.iter().take(cols.len().saturating_sub(1)) {
                x += cw;
                line(out, Pos::new(x, pos.y), Pos::new(x, pos.y + h), 0.2, col);
            }
            let mut y = pos.y;
            for rh in rows.iter().take(rows.len().saturating_sub(1)) {
                y += rh;
                line(out, Pos::new(pos.x, y), Pos::new(pos.x + w, y), 0.2, col);
            }
            let mut y = pos.y;
            for (r, rh) in rows.iter().enumerate() {
                let mut x = pos.x;
                for (c, cw) in cols.iter().enumerate() {
                    if let Some(t) = cells.get(r).and_then(|row| row.get(c)) {
                        text(out, Pos::new(x + 1.0, y + (rh - size) / 2.0), t.clone(), *size, col, TAlign::Links, r == 0);
                    }
                    x += cw;
                }
                y += rh;
            }
        }
        ObjKind::Wall { pts, thickness, closed } => {
            let c = st.stroke.map(|s| s.color).unwrap_or(farbe::SCHWARZ);
            out.push(Prim::Stroke { pts: pts.clone(), stroke: PStroke { round: false, ..PStroke::solid(c, *thickness) }, closed: *closed });
        }
        ObjKind::Room { pts, name, show_area, size } => {
            style_prims(out, pts, true, st);
            let c = polygon_centroid(pts);
            let mut y = c.y - size * 0.6;
            if *show_area {
                y -= size * 0.6;
            }
            text(out, Pos::new(c.x, y), name.clone(), *size, farbe::SCHWARZ, TAlign::Mitte, false);
            if *show_area {
                let m2 = polygon_area(pts) * (doc.scale_n / 1000.0).powi(2);
                text(out, Pos::new(c.x, y + size * 1.3), format!("{m2:.2} m²"), size * 0.85, farbe::SCHWARZ, TAlign::Mitte, false);
            }
        }
        ObjKind::Window { a, b, thickness } => {
            let n = (*b - *a).norm().perp() * (thickness / 2.0);
            let pts = vec![*a + n, *b + n, *b - n, *a - n];
            out.push(Prim::Fill { pts: pts.clone(), color: farbe::WEISS });
            out.push(Prim::Stroke { pts, stroke: PStroke { round: false, ..PStroke::solid(col, 0.25) }, closed: true });
            line(out, *a, *b, 0.18, col);
        }
        ObjKind::Door { pos, width, rot, mirror, double } => door_prims(out, *pos, *width, *rot, *mirror, *double, col),
        ObjKind::Stairs { a, b, width, steps } => stairs_prims(out, *a, *b, *width, *steps, col),
        ObjKind::Note { pos, text: t } => {
            if printing {
                return;
            }
            let bb = ops::bounds(&o.kind);
            fill_rect(out, bb.min, bb.max, Color::rgb(0xff, 0xf3, 0x9a));
            frame_rect(out, bb.min, bb.max, 0.2, Color::rgb(0xb8, 0x9b, 0x00));
            for (i, l) in t.lines().enumerate() {
                text(out, *pos + Pos::new(2.0, 2.0 + i as f32 * 3.75), l, 3.0, farbe::SCHWARZ, TAlign::Links, false);
            }
        }
        ObjKind::Area { pts, .. } => style_prims(out, pts, true, st),
        ObjKind::Route { pts, width, curved, arrows } => {
            let p = if *curved { spline_pts(pts) } else { pts.clone() };
            let c = st.stroke.map(|s| s.color).unwrap_or(farbe::FLUCHT_HELL);
            out.push(Prim::Stroke { pts: p.clone(), stroke: PStroke { round: false, ..PStroke::solid(c, *width) }, closed: false });
            if *arrows {
                let arrow_col = if c == farbe::FLUCHT_HELL { farbe::GRUEN } else { farbe::WEISS };
                chevrons(out, &p, *width, arrow_col);
            }
        }
        ObjKind::Symbol { id, pos, size, rot } => out.push(Prim::Symbol { id: id.clone(), c: *pos, size: *size, rot: *rot }),
        ObjKind::Legend { a, b, cols, size } => {
            if let Fill::Farbe(f) = st.fill {
                fill_rect(out, *a, *b, f);
            }
            if let Some(s) = st.stroke {
                frame_rect(out, *a, *b, s.width, s.color);
            }
            // passen nicht mehr alle Einträge hinein (Symbol ergänzt), Spalten/Schrift anpassen
            let (c, s) = legend_fit(doc, BBox::from_two(*a, *b), *size / 3.4);
            let (c, s) = if s < *size || c > *cols { (c, s) } else { (*cols, *size) };
            legend_prims(out, doc, *a, *b, c, s, true);
        }
        ObjKind::Block { a, b, art } => {
            let bb = BBox::from_two(*a, *b);
            if let Fill::Farbe(f) = st.fill {
                fill_rect(out, *a, *b, f);
            }
            if let Some(s) = st.stroke {
                frame_rect(out, *a, *b, s.width, s.color);
            }
            // Grundeinheit aus der Feldgröße (die Rahmenvorlage rechnet in A3-Einheiten)
            match art {
                BlockArt::Regeln => crate::templates::regeln_prims(out, doc, bb),
                BlockArt::Schriftfeld => crate::templates::schriftfeld_prims(out, doc, bb, (bb.w() / 85.0).clamp(0.3, 3.0)),
                BlockArt::Notruf => crate::templates::notruf_prims(out, doc, bb),
                BlockArt::Sammelstelle => crate::templates::sammel_prims(out, doc, bb, (bb.h() / 25.0).clamp(0.3, 3.0)),
            }
        }
        ObjKind::Grid { a, b, nx, ny } => grid_prims(out, *a, *b, *nx, *ny, col),
        ObjKind::FloorSection { a, b, floors, current } => floor_section_prims(out, *a, *b, floors, *current),
        ObjKind::Path { subpaths, closed } => {
            if let Fill::Farbe(f) = st.fill {
                if subpaths.len() == 1 {
                    out.push(Prim::Fill { pts: subpaths[0].clone(), color: f });
                } else {
                    out.push(Prim::FillRings { rings: subpaths.clone(), color: f });
                }
            }
            if let Some(s) = st.stroke {
                for sp in subpaths {
                    out.push(Prim::Stroke { pts: sp.clone(), stroke: PStroke::from_style(&s), closed: *closed });
                }
            }
        }
        ObjKind::RevisionCloud { pts } => {
            let s = st.stroke.unwrap_or(StrokeStyle::new(farbe::ROT, 0.35));
            out.push(Prim::Stroke { pts: cloud_pts(pts), stroke: PStroke::from_style(&s), closed: true });
        }
        ObjKind::Fence { pts } => fence_prims(out, pts, col),
        ObjKind::Rail { pts } => rail_prims(out, pts, col),
        ObjKind::ScaleBar { pos, meters } => {
            let w = doc.sheet_mm(*meters);
            let n = 4;
            for i in 0..n {
                let x0 = pos.x + w * i as f32 / n as f32;
                let x1 = pos.x + w * (i + 1) as f32 / n as f32;
                let c = if i % 2 == 0 { farbe::SCHWARZ } else { farbe::WEISS };
                fill_rect(out, Pos::new(x0, pos.y), Pos::new(x1, pos.y + 1.5), c);
            }
            frame_rect(out, *pos, *pos + Pos::new(w, 1.5), 0.2, farbe::SCHWARZ);
            text(out, *pos + Pos::new(0.0, 2.2), "0", 2.5, farbe::SCHWARZ, TAlign::Mitte, false);
            text(out, *pos + Pos::new(w / 2.0, 2.2), format!("{}", meters / 2.0), 2.5, farbe::SCHWARZ, TAlign::Mitte, false);
            text(out, *pos + Pos::new(w, 2.2), format!("{meters} m"), 2.5, farbe::SCHWARZ, TAlign::Mitte, false);
            text(out, *pos + Pos::new(w / 2.0, -3.6), format!("M 1:{}", doc.scale_n), 2.5, farbe::SCHWARZ, TAlign::Mitte, false);
        }
        ObjKind::QrCode { pos, size, text: t } => {
            if let Some((w, mods)) = qr_modules(t) {
                let quiet = 2.0;
                let m = size / (w as f32 + quiet * 2.0);
                fill_rect(out, *pos, *pos + Pos::new(*size, *size), farbe::WEISS);
                for y in 0..w {
                    let mut x = 0;
                    while x < w {
                        if mods[y * w + x] {
                            let start = x;
                            while x < w && mods[y * w + x] {
                                x += 1;
                            }
                            let a = *pos + Pos::new((start as f32 + quiet) * m, (y as f32 + quiet) * m);
                            let b = *pos + Pos::new((x as f32 + quiet) * m, (y as f32 + quiet + 1.0) * m);
                            fill_rect(out, a, b + Pos::new(0.0, 0.02), farbe::SCHWARZ);
                        } else {
                            x += 1;
                        }
                    }
                }
            }
        }
        ObjKind::FireCurve { c, r_out, width, start, end } => fire_curve_prims(out, doc, *c, *r_out, *width, *start, *end),
        ObjKind::Group { children } => {
            for ch in children {
                obj_prims(out, doc, ch, printing);
            }
        }
    }
}

fn fence_prims(out: &mut Prims, pts: &[Pos], col: Color) {
    out.push(Prim::Stroke { pts: pts.to_vec(), stroke: PStroke::solid(col, 0.25), closed: false });
    for (p, d) in points_along(pts, 2.0, 4.0) {
        let a = (d + d.perp()).norm() * 0.9;
        let b = (d - d.perp()).norm() * 0.9;
        line(out, p - a, p + a, 0.2, col);
        line(out, p - b, p + b, 0.2, col);
    }
}

fn rail_prims(out: &mut Prims, pts: &[Pos], col: Color) {
    for d in [0.7, -0.7] {
        out.push(Prim::Stroke { pts: offset_polyline(pts, d, false), stroke: PStroke::solid(col, 0.25), closed: false });
    }
    for (p, d) in points_along(pts, 1.0, 2.5) {
        let n = d.perp() * 1.3;
        line(out, p - n, p + n, 0.3, col);
    }
}

/// Gesamter Plan: Rahmen, Objekte (sichtbare Ebenen).
pub fn page_prims<'a>(doc: &'a Doc, printing: bool) -> Prims<'a> {
    let mut out = Vec::new();
    for o in &doc.objects {
        if doc.layer_visible(o.layer) {
            let start = out.len();
            obj_prims(&mut out, doc, o, printing);
            // Ebenenfarbe überschreibt Linien und Texte (weiße Schrift auf Farbflächen bleibt)
            if let Some(c) = doc.layers.iter().find(|l| l.id == o.layer).and_then(|l| l.color) {
                for p in &mut out[start..] {
                    match p {
                        Prim::Stroke { stroke, .. } => stroke.color = c,
                        Prim::Text { color, .. } if *color != farbe::WEISS => *color = c,
                        _ => {}
                    }
                }
            }
        }
    }
    let mut frame = templates::frame_prims(doc);
    // Rahmen liegt über den Objekten (deckt Überstände ab), Planbereich bleibt frei
    out.append(&mut frame);
    out
}

pub fn symbol_svg(doc: &Doc, id: &str) -> Option<String> {
    symbols::find(id)
        .map(|s| s.svg.clone())
        .or_else(|| doc.custom_symbols.iter().find(|c| c.id == id).map(|c| c.svg.clone()))
}
