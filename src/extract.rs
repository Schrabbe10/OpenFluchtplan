//! Raster-Hilfen: umschlossene Fläche füllen, Wände aus Bild erkennen, Linienwerkzeuge.

use crate::geom::*;
use crate::model::*;

// ---------- Kontur / Vereinfachung ----------

fn douglas_peucker(pts: &[Pos], eps: f32) -> Vec<Pos> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (mut idx, mut dmax) = (0, 0.0);
    for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = dist_to_segment(*p, a, b);
        if d > dmax {
            dmax = d;
            idx = i;
        }
    }
    if dmax > eps {
        let mut l = douglas_peucker(&pts[..=idx], eps);
        let r = douglas_peucker(&pts[idx..], eps);
        l.pop();
        l.extend(r);
        l
    } else {
        vec![a, b]
    }
}

/// Außenkontur einer Pixelmaske (Moore-Nachbarschaft).
fn trace_contour(mask: &[bool], w: usize, h: usize) -> Vec<(i32, i32)> {
    let inside = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && mask[y as usize * w + x as usize];
    let Some(start_i) = mask.iter().position(|m| *m) else { return Vec::new() };
    let start = ((start_i % w) as i32, (start_i / w) as i32);
    const D: [(i32, i32); 8] = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)];
    let mut contour = vec![start];
    let mut cur = start;
    let mut dir = 6usize; // zuletzt "von oben" gekommen
    let limit = 4 * (w + h) * 50;
    for _ in 0..limit {
        let mut next = None;
        for k in 0..8 {
            let d = (dir + 6 + k) % 8;
            let n = (cur.0 + D[d].0, cur.1 + D[d].1);
            if inside(n.0, n.1) {
                next = Some((n, d));
                break;
            }
        }
        let Some((n, d)) = next else { break };
        if n == start && contour.len() > 2 {
            break;
        }
        contour.push(n);
        cur = n;
        dir = d;
    }
    contour
}

/// Umschlossene Fläche um `p` aus Wänden/Linien bestimmen (wie ein Füllwerkzeug).
pub fn fill_region(doc: &Doc, p: Pos) -> Option<Vec<Pos>> {
    use resvg::tiny_skia::{LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};
    let (pw, ph) = doc.page.size();
    let r = (2500.0 / pw.max(ph)).min(4.0);
    let (w, h) = ((pw * r).ceil() as u32, (ph * r).ceil() as u32);
    let mut pm = Pixmap::new(w, h)?;
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = false;
    let mut draw = |pts: &[Pos], closed: bool, width: f32| {
        if pts.len() < 2 {
            return;
        }
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].x * r, pts[0].y * r);
        for q in &pts[1..] {
            pb.line_to(q.x * r, q.y * r);
        }
        if closed {
            pb.close();
        }
        if let Some(path) = pb.finish() {
            let st = Stroke { width: (width * r).max(1.5), line_cap: LineCap::Square, ..Stroke::default() };
            pm.stroke_path(&path, &paint, &st, Transform::identity(), None);
        }
    };
    fn walk(objs: &[Obj], doc: &Doc, draw: &mut dyn FnMut(&[Pos], bool, f32)) {
        for o in objs {
            if !doc.layer_visible(o.layer) {
                continue;
            }
            match &o.kind {
                ObjKind::Wall { pts, thickness, closed } => draw(pts, *closed, *thickness),
                ObjKind::Line { a, b } => draw(&[*a, *b], false, 0.3),
                ObjKind::Polyline { pts, closed } => draw(pts, *closed, 0.3),
                ObjKind::Rect { a, b, .. } => draw(&[*a, Pos::new(b.x, a.y), *b, Pos::new(a.x, b.y)], true, 0.3),
                ObjKind::Window { a, b, .. } => draw(&[*a, *b], false, 0.5),
                ObjKind::Group { children } => walk(children, doc, draw),
                _ => {}
            }
        }
    }
    walk(&doc.objects, doc, &mut draw);
    let (sx, sy) = ((p.x * r) as i32, (p.y * r) as i32);
    if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
        return None;
    }
    let (wu, hu) = (w as usize, h as usize);
    let wall: Vec<bool> = pm.pixels().iter().map(|px| px.alpha() > 0).collect();
    if wall[sy as usize * wu + sx as usize] {
        return None;
    }
    let mut mask = vec![false; wu * hu];
    let mut stack = vec![(sx as usize, sy as usize)];
    let mut count = 0usize;
    while let Some((x, y)) = stack.pop() {
        let i = y * wu + x;
        if mask[i] || wall[i] {
            continue;
        }
        mask[i] = true;
        count += 1;
        if x == 0 || y == 0 || x == wu - 1 || y == hu - 1 {
            return None; // ausgelaufen: Fläche nicht geschlossen
        }
        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }
    if count < 4 {
        return None;
    }
    let contour = trace_contour(&mask, wu, hu);
    let pts: Vec<Pos> = contour.iter().map(|(x, y)| Pos::new((*x as f32 + 0.5) / r, (*y as f32 + 0.5) / r)).collect();
    let mut simple = douglas_peucker(&pts, 0.8 / r);
    if simple.len() > 3 && simple.first().map(|a| a.dist(*simple.last().unwrap()) < 1.0 / r).unwrap_or(false) {
        simple.pop();
    }
    if simple.len() < 3 { None } else { Some(simple) }
}

// ---------- Wände aus Bild ----------

/// Erkennt lange waagerechte/senkrechte dunkle Linien als Wände.
pub fn extract_walls(img: &image::DynamicImage, a: Pos, b: Pos, threshold: u8, min_len_mm: f32) -> Vec<ObjKind> {
    let mut img = img.clone();
    if img.width() > 2400 || img.height() > 2400 {
        img = img.thumbnail(2400, 2400);
    }
    let g = img.to_luma8();
    let (w, h) = (g.width() as usize, g.height() as usize);
    let bb = BBox::from_two(a, b);
    let (sx, sy) = (bb.w() / w as f32, bb.h() / h as f32);
    let dark: Vec<bool> = g.pixels().map(|p| p.0[0] < threshold).collect();
    let min_px_h = (min_len_mm / sx).max(5.0) as usize;
    let min_px_v = (min_len_mm / sy).max(5.0) as usize;
    let mut out = Vec::new();
    for horizontal in [true, false] {
        let mut run_mask = vec![false; w * h];
        let (outer, inner, min_px) = if horizontal { (h, w, min_px_h) } else { (w, h, min_px_v) };
        let idx = |o: usize, i: usize| if horizontal { o * w + i } else { i * w + o };
        for o in 0..outer {
            let mut i = 0;
            while i < inner {
                if dark[idx(o, i)] {
                    let s = i;
                    while i < inner && dark[idx(o, i)] {
                        i += 1;
                    }
                    if i - s >= min_px {
                        for k in s..i {
                            run_mask[idx(o, k)] = true;
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }
        // zusammenhängende Bänder
        let mut seen = vec![false; w * h];
        for start in 0..w * h {
            if !run_mask[start] || seen[start] {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
            let mut stack = vec![start];
            seen[start] = true;
            while let Some(i) = stack.pop() {
                let (x, y) = (i % w, i / w);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
                let mut push = |j: usize| {
                    if run_mask[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                };
                if x + 1 < w {
                    push(i + 1);
                }
                if x > 0 {
                    push(i - 1);
                }
                if y + 1 < h {
                    push(i + w);
                }
                if y > 0 {
                    push(i - w);
                }
            }
            let (bw, bh) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
            let to_mm = |x: f32, y: f32| Pos::new(bb.min.x + x * sx, bb.min.y + y * sy);
            if horizontal && bw >= min_px as f32 && bh < bw / 3.0 {
                let yc = (y0 + y1) as f32 / 2.0 + 0.5;
                out.push(ObjKind::Wall { pts: vec![to_mm(x0 as f32, yc), to_mm(x1 as f32 + 1.0, yc)], thickness: (bh * sy).max(0.4), closed: false });
            } else if !horizontal && bh >= min_px as f32 && bw < bh / 3.0 {
                let xc = (x0 + x1) as f32 / 2.0 + 0.5;
                out.push(ObjKind::Wall { pts: vec![to_mm(xc, y0 as f32), to_mm(xc, y1 as f32 + 1.0)], thickness: (bw * sx).max(0.4), closed: false });
            }
        }
    }
    out
}

// ---------- Linienwerkzeuge ----------

fn nearest_segment(pts: &[Pos], closed: bool, p: Pos) -> Option<usize> {
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let segs = if closed { n } else { n - 1 };
    (0..segs).min_by(|a, b| {
        let da = dist_to_segment(p, pts[*a], pts[(*a + 1) % n]);
        let db = dist_to_segment(p, pts[*b], pts[(*b + 1) % n]);
        da.partial_cmp(&db).unwrap()
    })
}

/// Segment entfernen: liefert die verbleibenden Teile.
pub fn cut_segment(k: &ObjKind, p: Pos) -> Option<Vec<ObjKind>> {
    let (pts, closed) = match k {
        ObjKind::Wall { pts, closed, .. } | ObjKind::Polyline { pts, closed } => (pts.clone(), *closed),
        _ => return None,
    };
    let i = nearest_segment(&pts, closed, p)?;
    let n = pts.len();
    let parts: Vec<Vec<Pos>> = if closed {
        let mut v: Vec<Pos> = pts[(i + 1) % n..].to_vec();
        v.extend_from_slice(&pts[..=i]);
        vec![v]
    } else {
        vec![pts[..=i].to_vec(), pts[i + 1..].to_vec()]
    };
    let mk = |v: Vec<Pos>| match k {
        ObjKind::Wall { thickness, .. } => ObjKind::Wall { pts: v, thickness: *thickness, closed: false },
        _ => ObjKind::Polyline { pts: v, closed: false },
    };
    Some(parts.into_iter().filter(|v| v.len() >= 2).map(mk).collect())
}

/// Ecke abrunden.
pub fn fillet(k: &mut ObjKind, p: Pos, r: f32, tol: f32) -> bool {
    let (pts, closed) = match k {
        ObjKind::Wall { pts, closed, .. } | ObjKind::Polyline { pts, closed } => (pts, *closed),
        ObjKind::Room { pts, .. } | ObjKind::Area { pts, .. } => (pts, true),
        ObjKind::Route { pts, .. } | ObjKind::Fence { pts } | ObjKind::Rail { pts } | ObjKind::Hose { pts, .. } => (pts, false),
        _ => return false,
    };
    let n = pts.len();
    let Some(i) = (0..n).filter(|i| closed || (*i > 0 && *i < n - 1)).min_by(|a, b| pts[*a].dist(p).partial_cmp(&pts[*b].dist(p)).unwrap()) else { return false };
    if pts[i].dist(p) > tol.max(r) {
        return false;
    }
    let v = pts[i];
    let prev = pts[(i + n - 1) % n];
    let next = pts[(i + 1) % n];
    let ta = r.min(v.dist(prev) * 0.45);
    let tb = r.min(v.dist(next) * 0.45);
    let a = v + (prev - v).norm() * ta;
    let b = v + (next - v).norm() * tb;
    let curve: Vec<Pos> = (0..=8)
        .map(|j| {
            let t = j as f32 / 8.0;
            a * ((1.0 - t) * (1.0 - t)) + v * (2.0 * (1.0 - t) * t) + b * (t * t)
        })
        .collect();
    pts.splice(i..=i, curve);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raum_fuellen() {
        let mut doc = Doc::default();
        doc.add(
            ObjKind::Wall { pts: vec![Pos::new(100.0, 100.0), Pos::new(200.0, 100.0), Pos::new(200.0, 180.0), Pos::new(100.0, 180.0)], thickness: 1.0, closed: true },
            Style::default(),
            1,
        );
        let poly = fill_region(&doc, Pos::new(150.0, 140.0)).expect("Fläche");
        let a = polygon_area(&poly);
        assert!(a > 7000.0 && a < 8000.0, "Fläche {a}");
        assert!(fill_region(&doc, Pos::new(20.0, 20.0)).is_none());
    }
}
