//! Geometrie-Hilfen. Koordinaten in mm auf dem Blatt, y nach unten.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn dist(self, o: Pos) -> f32 {
        (self - o).len()
    }
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn norm(self) -> Pos {
        let l = self.len();
        if l < 1e-9 { Pos::new(1.0, 0.0) } else { self * (1.0 / l) }
    }
    pub fn perp(self) -> Pos {
        Pos::new(-self.y, self.x)
    }
    pub fn dot(self, o: Pos) -> f32 {
        self.x * o.x + self.y * o.y
    }
    pub fn rotate(self, deg: f32) -> Pos {
        let (s, c) = deg.to_radians().sin_cos();
        Pos::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    pub fn rotate_around(self, c: Pos, deg: f32) -> Pos {
        c + (self - c).rotate(deg)
    }
    pub fn angle_deg(self) -> f32 {
        self.y.atan2(self.x).to_degrees()
    }
    pub fn lerp(self, o: Pos, t: f32) -> Pos {
        self + (o - self) * t
    }
}

impl std::ops::Add for Pos {
    type Output = Pos;
    fn add(self, o: Pos) -> Pos {
        Pos::new(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for Pos {
    type Output = Pos;
    fn sub(self, o: Pos) -> Pos {
        Pos::new(self.x - o.x, self.y - o.y)
    }
}
impl std::ops::Mul<f32> for Pos {
    type Output = Pos;
    fn mul(self, f: f32) -> Pos {
        Pos::new(self.x * f, self.y * f)
    }
}
impl std::ops::Neg for Pos {
    type Output = Pos;
    fn neg(self) -> Pos {
        Pos::new(-self.x, -self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BBox {
    pub min: Pos,
    pub max: Pos,
}

impl BBox {
    pub const EMPTY: BBox = BBox {
        min: Pos::new(f32::MAX, f32::MAX),
        max: Pos::new(f32::MIN, f32::MIN),
    };
    pub fn from_pts(pts: &[Pos]) -> BBox {
        let mut b = BBox::EMPTY;
        for p in pts {
            b.add(*p);
        }
        b
    }
    pub fn from_two(a: Pos, b: Pos) -> BBox {
        BBox::from_pts(&[a, b])
    }
    pub fn add(&mut self, p: Pos) {
        self.min = Pos::new(self.min.x.min(p.x), self.min.y.min(p.y));
        self.max = Pos::new(self.max.x.max(p.x), self.max.y.max(p.y));
    }
    pub fn union(mut self, o: BBox) -> BBox {
        if o.is_empty() {
            return self;
        }
        self.add(o.min);
        self.add(o.max);
        self
    }
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }
    pub fn expand(self, d: f32) -> BBox {
        BBox {
            min: self.min - Pos::new(d, d),
            max: self.max + Pos::new(d, d),
        }
    }
    pub fn contains(&self, p: Pos) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn intersects(&self, o: &BBox) -> bool {
        self.min.x <= o.max.x && o.min.x <= self.max.x && self.min.y <= o.max.y && o.min.y <= self.max.y
    }
    pub fn inside(&self, outer: &BBox) -> bool {
        outer.contains(self.min) && outer.contains(self.max)
    }
    pub fn center(&self) -> Pos {
        self.min.lerp(self.max, 0.5)
    }
    pub fn w(&self) -> f32 {
        self.max.x - self.min.x
    }
    pub fn h(&self) -> f32 {
        self.max.y - self.min.y
    }
}

pub fn dist_to_segment(p: Pos, a: Pos, b: Pos) -> f32 {
    let ab = b - a;
    let len2 = ab.dot(ab);
    if len2 < 1e-9 {
        return p.dist(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.dist(a + ab * t)
}

pub fn dist_to_polyline(p: Pos, pts: &[Pos], closed: bool) -> f32 {
    let mut d = f32::MAX;
    for s in pts.windows(2) {
        d = d.min(dist_to_segment(p, s[0], s[1]));
    }
    if closed && pts.len() > 2 {
        d = d.min(dist_to_segment(p, pts[pts.len() - 1], pts[0]));
    }
    if pts.len() == 1 {
        d = p.dist(pts[0]);
    }
    d
}

pub fn point_in_polygon(p: Pos, poly: &[Pos]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn polygon_area(poly: &[Pos]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut a = 0.0;
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        a += p.x * q.y - q.x * p.y;
    }
    (a / 2.0).abs()
}

pub fn polygon_centroid(poly: &[Pos]) -> Pos {
    let n = poly.len();
    if n == 0 {
        return Pos::default();
    }
    let mut a = 0.0;
    let mut c = Pos::default();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let cr = p.x * q.y - q.x * p.y;
        a += cr;
        c = c + (p + q) * cr;
    }
    if a.abs() < 1e-6 {
        let s = poly.iter().fold(Pos::default(), |s, p| s + *p);
        return s * (1.0 / n as f32);
    }
    c * (1.0 / (3.0 * a))
}

pub fn polyline_len(pts: &[Pos]) -> f32 {
    pts.windows(2).map(|s| s[0].dist(s[1])).sum()
}

/// Kreis/Bogen als Polylinie (Winkel in Grad, im Uhrzeigersinn bei y nach unten).
pub fn arc_pts(c: Pos, rx: f32, ry: f32, start: f32, end: f32) -> Vec<Pos> {
    let sweep = end - start;
    let n = ((sweep.abs() / 6.0).ceil() as usize).clamp(4, 180);
    (0..=n)
        .map(|i| {
            let a = (start + sweep * i as f32 / n as f32).to_radians();
            Pos::new(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}

pub fn ellipse_pts(c: Pos, rx: f32, ry: f32) -> Vec<Pos> {
    let mut v = arc_pts(c, rx, ry, 0.0, 360.0);
    v.pop();
    v
}

pub fn ngon_pts(c: Pos, r: f32, n: u32, rot: f32) -> Vec<Pos> {
    let n = n.max(3);
    (0..n)
        .map(|i| c + Pos::new(0.0, -r).rotate(rot + 360.0 * i as f32 / n as f32))
        .collect()
}

/// Catmull-Rom-Spline durch alle Punkte.
pub fn spline_pts(pts: &[Pos]) -> Vec<Pos> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut out = Vec::new();
    for i in 0..pts.len() - 1 {
        let p0 = if i == 0 { pts[0] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < pts.len() { pts[i + 2] } else { pts[i + 1] };
        for k in 0..12 {
            let t = k as f32 / 12.0;
            let t2 = t * t;
            let t3 = t2 * t;
            let p = (p1 * 2.0
                + (p2 - p0) * t
                + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
                + (p1 * 3.0 - p0 - p2 * 3.0 + p3) * t3)
                * 0.5;
            out.push(p);
        }
    }
    out.push(*pts.last().unwrap());
    out
}

/// Parallele Versetzung einer Polylinie (Gehrung an Ecken).
pub fn offset_polyline(pts: &[Pos], d: f32, closed: bool) -> Vec<Pos> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let seg_n = |i: usize| (pts[(i + 1) % n] - pts[i]).norm().perp();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = if i == 0 {
            if closed { Some(n - 1) } else { None }
        } else {
            Some(i - 1)
        };
        let next = if i == n - 1 && !closed { None } else { Some(i) };
        let normal = match (prev, next) {
            (Some(a), Some(b)) => {
                let (na, nb) = (seg_n(a), seg_n(b));
                let m = (na + nb).norm();
                let cos = m.dot(nb).max(0.25);
                m * (1.0 / cos)
            }
            (Some(a), None) => seg_n(a),
            (None, Some(b)) => seg_n(b),
            _ => Pos::new(0.0, 1.0),
        };
        out.push(pts[i] + normal * d);
    }
    out
}

/// Schraffurlinien eines Polygons (Scanline-Verfahren).
pub fn hatch_segments(poly: &[Pos], angle: f32, spacing: f32) -> Vec<(Pos, Pos)> {
    let mut out = Vec::new();
    if poly.len() < 3 || spacing <= 0.05 {
        return out;
    }
    // In ein gedrehtes Koordinatensystem, in dem die Schraffur horizontal liegt
    let rot: Vec<Pos> = poly.iter().map(|p| p.rotate(-angle)).collect();
    let bb = BBox::from_pts(&rot);
    let mut y = (bb.min.y / spacing).floor() * spacing + spacing * 0.5;
    let n = rot.len();
    while y < bb.max.y {
        let mut xs = Vec::new();
        for i in 0..n {
            let (a, b) = (rot[i], rot[(i + 1) % n]);
            if (a.y > y) != (b.y > y) {
                xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for pair in xs.chunks(2) {
            if pair.len() == 2 {
                out.push((
                    Pos::new(pair[0], y).rotate(angle),
                    Pos::new(pair[1], y).rotate(angle),
                ));
            }
        }
        y += spacing;
    }
    out
}

/// Punkte im gleichmäßigen Abstand entlang einer Polylinie: (Position, Richtung).
pub fn points_along(pts: &[Pos], start: f32, step: f32) -> Vec<(Pos, Pos)> {
    let mut out = Vec::new();
    if step <= 0.0 {
        return out;
    }
    let mut next = start;
    let mut walked = 0.0;
    for s in pts.windows(2) {
        let len = s[0].dist(s[1]);
        if len < 1e-6 {
            continue;
        }
        let dir = (s[1] - s[0]) * (1.0 / len);
        while next <= walked + len {
            out.push((s[0] + dir * (next - walked), dir));
            next += step;
        }
        walked += len;
    }
    out
}

/// Polylinie in Striche nach Muster (an, aus, an, aus …) zerlegen.
pub fn dash_polyline(pts: &[Pos], pattern: &[f32]) -> Vec<Vec<Pos>> {
    let mut out = Vec::new();
    if pattern.is_empty() || pattern.iter().all(|v| *v <= 0.0) {
        return vec![pts.to_vec()];
    }
    let mut idx = 0;
    let mut left = pattern[0];
    let mut on = true;
    let mut cur: Vec<Pos> = Vec::new();
    for s in pts.windows(2) {
        let (mut a, b) = (s[0], s[1]);
        let mut len = a.dist(b);
        if on && cur.is_empty() {
            cur.push(a);
        }
        while len > left {
            let p = a.lerp(b, left / len);
            if on {
                cur.push(p);
                out.push(std::mem::take(&mut cur));
            } else {
                cur = vec![p];
            }
            len -= left;
            a = p;
            on = !on;
            idx = (idx + 1) % pattern.len();
            left = pattern[idx].max(0.01);
        }
        left -= len;
        if on {
            cur.push(b);
        }
    }
    if on && cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// Auf 45°-Schritte einrasten (Umschalttaste beim Zeichnen).
pub fn snap_ortho(last: Pos, p: Pos) -> Pos {
    let d = p - last;
    let base = (d.angle_deg() / 45.0).round() * 45.0;
    last + Pos::new(d.len(), 0.0).rotate(base)
}

/// Triangulierung (auch konkave Polygone).
pub fn triangulate(poly: &[Pos]) -> Vec<usize> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let flat: Vec<f64> = poly.iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
    earcutr::earcut(&flat, &[], 2).unwrap_or_default()
}

/// Triangulierung mehrerer Ringe nach der Gerade-Ungerade-Regel (z. B. Buchstaben „O“, Ringe):
/// Ringe gerader Schachtelungstiefe sind Außenkonturen, ungerader Tiefe Löcher.
/// Liefert die Punktliste und die Dreiecksindizes darauf.
pub fn triangulate_rings(rings: &[Vec<Pos>]) -> (Vec<Pos>, Vec<usize>) {
    let rings: Vec<&Vec<Pos>> = rings.iter().filter(|r| r.len() >= 3).collect();
    let mut order: Vec<usize> = (0..rings.len()).collect();
    let areas: Vec<f32> = rings.iter().map(|r| polygon_area(r).abs()).collect();
    order.sort_by(|a, b| areas[*b].total_cmp(&areas[*a]));
    // Elternring = kleinster bereits einsortierter Ring, der den ersten Punkt enthält
    let mut parent: Vec<Option<usize>> = vec![None; rings.len()];
    let mut depth = vec![0usize; rings.len()];
    for (k, &i) in order.iter().enumerate() {
        let mut best: Option<usize> = None;
        for &j in &order[..k] {
            if point_in_polygon(rings[i][0], rings[j]) && best.is_none_or(|b| areas[j] < areas[b]) {
                best = Some(j);
            }
        }
        if let Some(j) = best {
            parent[i] = Some(j);
            depth[i] = depth[j] + 1;
        }
    }
    let mut pts = Vec::new();
    let mut idx = Vec::new();
    for &o in order.iter().filter(|&&i| depth[i] % 2 == 0) {
        let mut flat: Vec<f64> = rings[o].iter().flat_map(|p| [p.x as f64, p.y as f64]).collect();
        let mut local: Vec<Pos> = rings[o].clone();
        let mut holes = Vec::new();
        for (h, r) in rings.iter().enumerate() {
            if parent[h] == Some(o) && depth[h] % 2 == 1 {
                holes.push(local.len());
                flat.extend(r.iter().flat_map(|p| [p.x as f64, p.y as f64]));
                local.extend(r.iter().copied());
            }
        }
        let base = pts.len();
        if let Ok(tri) = earcutr::earcut(&flat, &holes, 2) {
            idx.extend(tri.into_iter().map(|i| i + base));
        }
        pts.extend(local);
    }
    (pts, idx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flaeche_rechteck() {
        let r = [Pos::new(0.0, 0.0), Pos::new(10.0, 0.0), Pos::new(10.0, 5.0), Pos::new(0.0, 5.0)];
        assert!((polygon_area(&r) - 50.0).abs() < 1e-4);
        assert!(point_in_polygon(Pos::new(5.0, 2.0), &r));
        assert!(!point_in_polygon(Pos::new(15.0, 2.0), &r));
        assert_eq!(triangulate(&r).len(), 6);
        assert!(!hatch_segments(&r, 45.0, 1.0).is_empty());
    }
}
