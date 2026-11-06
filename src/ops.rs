//! Operationen auf Objekten: Begrenzung, Treffer, Transformationen, Griffpunkte.

use crate::geom::*;
use crate::model::*;
use crate::{presets, symbols};

/// Für Hit-Test und Auswahlrahmen (ungefähre Textbreite).
pub fn text_width(text: &str, size: f32) -> f32 {
    text.lines().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f32 * size * 0.55
}

pub fn table_size(cols: &[f32], rows: &[f32]) -> (f32, f32) {
    (cols.iter().sum(), rows.iter().sum())
}

pub fn door_dir(rot: f32) -> Pos {
    Pos::new(1.0, 0.0).rotate(rot)
}

pub fn bounds(o: &ObjKind) -> BBox {
    match o {
        ObjKind::Line { a, b }
        | ObjKind::Rect { a, b, .. }
        | ObjKind::Image { a, b, .. }
        | ObjKind::Window { a, b, .. }
        | ObjKind::Legend { a, b, .. }
        | ObjKind::Block { a, b, .. }
        | ObjKind::Grid { a, b, .. }
        | ObjKind::FloorSection { a, b, .. } => BBox::from_two(*a, *b),
        ObjKind::Polyline { pts, .. }
        | ObjKind::Spline { pts }
        | ObjKind::Room { pts, .. }
        | ObjKind::Area { pts, .. }
        | ObjKind::RevisionCloud { pts }
        | ObjKind::Fence { pts }
        | ObjKind::Rail { pts } => BBox::from_pts(pts),
        ObjKind::Path { subpaths, .. } => subpaths.iter().fold(BBox::EMPTY, |b, sp| b.union(BBox::from_pts(sp))),
        ObjKind::Hose { pts, width } | ObjKind::Route { pts, width, .. } => BBox::from_pts(pts).expand(width / 2.0),
        ObjKind::Wall { pts, thickness, .. } => BBox::from_pts(pts).expand(thickness / 2.0),
        ObjKind::Circle { c, r } | ObjKind::NGon { c, r, .. } | ObjKind::Arc { c, r, .. } => {
            BBox::from_two(*c - Pos::new(*r, *r), *c + Pos::new(*r, *r))
        }
        ObjKind::Ellipse { c, rx, ry } => BBox::from_two(*c - Pos::new(*rx, *ry), *c + Pos::new(*rx, *ry)),
        ObjKind::FireCurve { c, r_out, .. } => BBox::from_two(*c - Pos::new(*r_out, *r_out), *c + Pos::new(*r_out, *r_out)),
        ObjKind::Text { pos, text, size, align, rot, .. } => {
            let w = text_width(text, *size);
            let h = text.lines().count().max(1) as f32 * size * 1.25;
            let x0 = match align {
                TAlign::Links => 0.0,
                TAlign::Mitte => -w / 2.0,
                TAlign::Rechts => -w,
            };
            let corners = [Pos::new(x0, 0.0), Pos::new(x0 + w, 0.0), Pos::new(x0 + w, h), Pos::new(x0, h)];
            BBox::from_pts(&corners.map(|c| *pos + c.rotate(*rot)))
        }
        ObjKind::Dimension { a, b, offset } => {
            let n = (*b - *a).norm().perp() * *offset;
            BBox::from_pts(&[*a, *b, *a + n, *b + n]).expand(2.0)
        }
        ObjKind::Table { pos, cols, rows, .. } => {
            let (w, h) = table_size(cols, rows);
            BBox::from_two(*pos, *pos + Pos::new(w, h))
        }
        ObjKind::Door { pos, width, rot, .. } => {
            let d = door_dir(*rot) * *width;
            let n = d.perp();
            BBox::from_pts(&[*pos, *pos + d, *pos + n, *pos + d + n, *pos - n])
        }
        ObjKind::Stairs { a, b, width, .. } => {
            let n = (*b - *a).norm().perp() * (width / 2.0);
            BBox::from_pts(&[*a + n, *a - n, *b + n, *b - n])
        }
        ObjKind::Note { pos, text } => BBox::from_two(*pos, *pos + Pos::new(text_width(text, 3.0).max(30.0) + 4.0, text.lines().count().max(1) as f32 * 3.75 + 4.0)),
        ObjKind::Symbol { pos, size, .. } => {
            let h = size * 0.5;
            BBox::from_two(*pos - Pos::new(h, h), *pos + Pos::new(h, h))
        }
        ObjKind::ScaleBar { pos, .. } => BBox::from_two(*pos - Pos::new(0.0, 4.0), *pos + Pos::new(60.0, 6.0)),
        ObjKind::QrCode { pos, size, .. } => BBox::from_two(*pos, *pos + Pos::new(*size, *size)),
        ObjKind::Group { children } => children.iter().fold(BBox::EMPTY, |b, c| b.union(bounds(&c.kind))),
    }
}

/// Maßstabsbalken braucht die Blattlänge aus dem Dokument.
pub fn bounds_doc(doc: &Doc, o: &ObjKind) -> BBox {
    if let ObjKind::ScaleBar { pos, meters } = o {
        let w = doc.sheet_mm(*meters);
        return BBox::from_two(*pos - Pos::new(0.0, 4.0), *pos + Pos::new(w, 6.0));
    }
    bounds(o)
}

pub fn hit(o: &ObjKind, p: Pos, tol: f32) -> bool {
    match o {
        ObjKind::Line { a, b } => dist_to_segment(p, *a, *b) <= tol,
        ObjKind::Polyline { pts, closed } => dist_to_polyline(p, pts, *closed) <= tol || (*closed && point_in_polygon(p, pts)),
        ObjKind::Spline { pts } => dist_to_polyline(p, &spline_pts(pts), false) <= tol,
        ObjKind::Hose { pts, width } | ObjKind::Route { pts, width, .. } => dist_to_polyline(p, pts, false) <= width / 2.0 + tol,
        ObjKind::Wall { pts, thickness, closed } => dist_to_polyline(p, pts, *closed) <= thickness / 2.0 + tol,
        ObjKind::Fence { pts } | ObjKind::Rail { pts } => dist_to_polyline(p, pts, false) <= tol + 1.5,
        ObjKind::Room { pts, .. } | ObjKind::Area { pts, .. } | ObjKind::RevisionCloud { pts } => {
            point_in_polygon(p, pts) || dist_to_polyline(p, pts, true) <= tol
        }
        ObjKind::Circle { c, r } => p.dist(*c) <= r + tol,
        ObjKind::Arc { c, r, .. } => (p.dist(*c) - r).abs() <= tol,
        ObjKind::Window { a, b, thickness } => dist_to_segment(p, *a, *b) <= thickness / 2.0 + tol,
        ObjKind::Group { children } => children.iter().any(|c| hit(&c.kind, p, tol)),
        // Planraster liegt meist über dem Grundriss: nur Linien und Beschriftungsrand greifen
        ObjKind::Grid { a, b, nx, ny } => {
            let bb = BBox::from_two(*a, *b);
            let (nx, ny) = ((*nx).max(1), (*ny).max(1));
            let (cw, ch) = (bb.w() / nx as f32, bb.h() / ny as f32);
            let rand = (cw.min(ch) * 0.25).clamp(2.0, 5.0) * 1.5;
            if !bb.expand(rand + tol).contains(p) {
                return false;
            }
            let innen = bb.expand(-tol);
            if !innen.contains(p) {
                return true; // Rahmen bzw. Beschriftungsrand
            }
            (1..nx).any(|i| (p.x - (bb.min.x + cw * i as f32)).abs() <= tol) || (1..ny).any(|j| (p.y - (bb.min.y + ch * j as f32)).abs() <= tol)
        }
        ObjKind::Path { subpaths, closed } => {
            subpaths.iter().any(|sp| dist_to_polyline(p, sp, *closed) <= tol)
                || (*closed && subpaths.iter().filter(|sp| point_in_polygon(p, sp)).count() % 2 == 1)
        }
        _ => bounds(o).expand(tol).contains(p),
    }
}

/// Treffer mit Rücksicht auf die Füllung: ungefüllte geschlossene Linienzüge und Pfade
/// (z. B. importierte Blattrahmen) greifen nur an der Linie, nicht in der Fläche.
pub fn hit_obj(o: &Obj, p: Pos, tol: f32) -> bool {
    let hohl = matches!(o.style.fill, Fill::Keine);
    match &o.kind {
        ObjKind::Polyline { pts, closed } if hohl => dist_to_polyline(p, pts, *closed) <= tol,
        ObjKind::Path { subpaths, closed } if hohl => subpaths.iter().any(|sp| dist_to_polyline(p, sp, *closed) <= tol),
        ObjKind::Group { children } => children.iter().any(|c| hit_obj(c, p, tol)),
        k => hit(k, p, tol),
    }
}

/// Treffer ohne Innenflächen (nur Umrisse von Räumen, Flächen, Polygonen und Pfaden).
/// Wird zuerst geprüft, damit Symbole und Texte vor darüberliegenden Flächen gewinnen.
pub fn hit_obj_umriss(o: &Obj, p: Pos, tol: f32) -> bool {
    match &o.kind {
        ObjKind::Polyline { pts, closed } => dist_to_polyline(p, pts, *closed) <= tol,
        ObjKind::Path { subpaths, closed } => subpaths.iter().any(|sp| dist_to_polyline(p, sp, *closed) <= tol),
        ObjKind::Room { pts, .. } | ObjKind::Area { pts, .. } | ObjKind::RevisionCloud { pts } => dist_to_polyline(p, pts, true) <= tol,
        ObjKind::Group { children } => children.iter().any(|c| hit_obj_umriss(c, p, tol)),
        _ => hit_obj(o, p, tol),
    }
}

/// Fasst kleine, übereinanderliegende Objekte zu Gruppen zusammen – z. B. die Einzelteile
/// (Hintergrund, Rahmen, Bildzeichen, Text) eines Symbols aus einem PDF-Import.
/// Berücksichtigt nur Objekte aus `ids`, die höchstens `max_mm` groß sind; zwei Objekte gehören
/// zusammen, wenn die Mitte des einen im anderen liegt. Liefert die Anzahl neuer Gruppen.
pub fn symbole_zusammenfassen(doc: &mut Doc, ids: &[u64], max_mm: f32) -> usize {
    teile_gruppieren(doc, ids, max_mm, true)
}

/// Wie [`symbole_zusammenfassen`]; mit `reihenfolge = false` werden Teile auch dann gruppiert, wenn
/// ein fremdes Objekt dazwischen liegt (z. B. ein Schild auf einer Wand, das danach ersetzt wird).
pub fn teile_gruppieren(doc: &mut Doc, ids: &[u64], max_mm: f32, reihenfolge: bool) -> usize {
    let cand: Vec<(usize, BBox)> = doc
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| ids.contains(&o.id) && !matches!(o.kind, ObjKind::Group { .. }) && !o.locked)
        .map(|(i, o)| (i, bounds_doc(doc, &o.kind)))
        .filter(|(_, b)| b.w() <= max_mm && b.h() <= max_mm && b.w() + b.h() > 0.5)
        .collect();
    let n = cand.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut cbox: Vec<BBox> = cand.iter().map(|c| c.1).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let center = |b: &BBox| Pos::new((b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0);
    for a in 0..n {
        for b in a + 1..n {
            let (ba, bb) = (cand[a].1, cand[b].1);
            if !(bb.expand(0.2).contains(center(&ba)) || ba.expand(0.2).contains(center(&bb))) {
                continue;
            }
            let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
            if ra == rb {
                continue;
            }
            let u = cbox[ra].union(cbox[rb]);
            // keine Ketten über größere Flächen (Schraffuren, Parkplätze …)
            if u.w() > max_mm * 1.3 || u.h() > max_mm * 1.3 {
                continue;
            }
            parent[rb] = ra;
            cbox[ra] = u;
        }
    }
    // Index im Plan -> Gruppe; die Gruppe ersetzt das oberste Teil (Zeichenreihenfolge bleibt sichtbar)
    let mut member: std::collections::HashMap<usize, usize> = Default::default();
    let mut count: std::collections::HashMap<usize, usize> = Default::default();
    let mut first: std::collections::HashMap<usize, usize> = Default::default();
    let mut last: std::collections::HashMap<usize, usize> = Default::default();
    for (k, (i, _)) in cand.iter().enumerate() {
        let r = root(&mut parent, k);
        member.insert(*i, r);
        *count.entry(r).or_default() += 1;
        first.entry(r).or_insert(*i);
        last.insert(r, *i);
    }
    // Liegt zwischen den Teilen ein fremdes Objekt, das sie überdeckt, würde die Gruppe die
    // Zeichenreihenfolge ändern – solche Teile bleiben einzeln.
    let boxes: Vec<BBox> = doc.objects.iter().map(|o| bounds_doc(doc, &o.kind)).collect();
    for (&r, c) in count.iter_mut() {
        if *c < 2 || !reihenfolge {
            continue;
        }
        let b = cbox[r];
        if (first[&r] + 1..last[&r]).any(|j| member.get(&j) != Some(&r) && boxes[j].intersects(&b)) {
            *c = 0;
        }
    }
    let mut pending: std::collections::HashMap<usize, Vec<Obj>> = Default::default();
    let mut out = Vec::with_capacity(doc.objects.len());
    let mut groups = 0;
    for (i, o) in std::mem::take(&mut doc.objects).into_iter().enumerate() {
        match member.get(&i) {
            Some(r) if count[r] > 1 => {
                let v = pending.entry(*r).or_default();
                v.push(o);
                if last[r] == i {
                    let children = pending.remove(r).unwrap_or_default();
                    let layer = children.last().map(|c| c.layer).unwrap_or(0);
                    let id = doc.alloc_id();
                    out.push(Obj { id, layer, kind: ObjKind::Group { children }, style: Style::default(), print: true, locked: false });
                    groups += 1;
                }
            }
            _ => out.push(o),
        }
    }
    doc.objects = out;
    groups
}

fn map_pts(pts: &mut [Pos], f: &impl Fn(Pos) -> Pos) {
    for p in pts {
        *p = f(*p);
    }
}

/// Wendet eine Punktabbildung an (für Verschieben / Drehen / Skalieren / Spiegeln).
fn map_points(o: &mut ObjKind, f: &impl Fn(Pos) -> Pos) {
    match o {
        ObjKind::Line { a, b }
        | ObjKind::Rect { a, b, .. }
        | ObjKind::Image { a, b, .. }
        | ObjKind::Window { a, b, .. }
        | ObjKind::Legend { a, b, .. }
        | ObjKind::Block { a, b, .. }
        | ObjKind::Grid { a, b, .. }
        | ObjKind::FloorSection { a, b, .. }
        | ObjKind::Dimension { a, b, .. }
        | ObjKind::Stairs { a, b, .. } => {
            *a = f(*a);
            *b = f(*b);
        }
        ObjKind::Polyline { pts, .. }
        | ObjKind::Spline { pts }
        | ObjKind::Hose { pts, .. }
        | ObjKind::Wall { pts, .. }
        | ObjKind::Room { pts, .. }
        | ObjKind::Area { pts, .. }
        | ObjKind::Route { pts, .. }
        | ObjKind::RevisionCloud { pts }
        | ObjKind::Fence { pts }
        | ObjKind::Rail { pts } => map_pts(pts, f),
        ObjKind::Path { subpaths, .. } => {
            for sp in subpaths {
                map_pts(sp, f);
            }
        }
        ObjKind::Circle { c, .. }
        | ObjKind::Ellipse { c, .. }
        | ObjKind::Arc { c, .. }
        | ObjKind::NGon { c, .. }
        | ObjKind::FireCurve { c, .. } => *c = f(*c),
        ObjKind::Text { pos, .. }
        | ObjKind::Table { pos, .. }
        | ObjKind::Door { pos, .. }
        | ObjKind::Note { pos, .. }
        | ObjKind::Symbol { pos, .. }
        | ObjKind::ScaleBar { pos, .. }
        | ObjKind::QrCode { pos, .. } => *pos = f(*pos),
        ObjKind::Group { children } => {
            for c in children {
                map_points(&mut c.kind, f);
            }
        }
    }
}

pub fn translate(o: &mut ObjKind, d: Pos) {
    map_points(o, &|p| p + d);
}

pub fn rotate(o: &mut ObjKind, c: Pos, deg: f32) {
    // Rechtecke werden bei schiefen Winkeln zu Polygonen
    if let ObjKind::Rect { a, b, .. } = o {
        if (deg.rem_euclid(90.0)).abs() > 0.01 {
            let pts = vec![*a, Pos::new(b.x, a.y), *b, Pos::new(a.x, b.y)];
            *o = ObjKind::Polyline { pts, closed: true };
        }
    }
    map_points(o, &|p| p.rotate_around(c, deg));
    match o {
        ObjKind::Arc { start, end, .. } | ObjKind::FireCurve { start, end, .. } => {
            *start += deg;
            *end += deg;
        }
        ObjKind::NGon { rot, .. } | ObjKind::Text { rot, .. } | ObjKind::Door { rot, .. } | ObjKind::Symbol { rot, .. } => {
            *rot = (*rot + deg).rem_euclid(360.0);
        }
        ObjKind::Ellipse { rx, ry, .. } => {
            if (deg.rem_euclid(180.0) - 90.0).abs() < 0.01 {
                std::mem::swap(rx, ry);
            }
        }
        ObjKind::Group { children } => {
            for ch in children {
                if let ObjKind::Symbol { rot, .. } | ObjKind::Text { rot, .. } | ObjKind::Door { rot, .. } = &mut ch.kind {
                    *rot = (*rot + deg).rem_euclid(360.0);
                }
            }
        }
        _ => {}
    }
}

pub fn scale(o: &mut ObjKind, c: Pos, f: f32) {
    map_points(o, &|p| c + (p - c) * f);
    match o {
        ObjKind::Circle { r, .. } | ObjKind::Arc { r, .. } | ObjKind::NGon { r, .. } => *r *= f,
        ObjKind::Ellipse { rx, ry, .. } => {
            *rx *= f;
            *ry *= f;
        }
        ObjKind::FireCurve { r_out, width, .. } => {
            *r_out *= f;
            *width *= f;
        }
        ObjKind::Text { size, .. } | ObjKind::Symbol { size, .. } | ObjKind::QrCode { size, .. } | ObjKind::Legend { size, .. } => *size *= f,
        ObjKind::Door { width, .. } | ObjKind::Stairs { width, .. } | ObjKind::Route { width, .. } | ObjKind::Hose { width, .. } => *width *= f,
        ObjKind::Table { cols, rows, size, .. } => {
            cols.iter_mut().for_each(|c| *c *= f);
            rows.iter_mut().for_each(|r| *r *= f);
            *size *= f;
        }
        ObjKind::Group { children } => {
            for ch in children {
                if let ObjKind::Symbol { size, .. } | ObjKind::Text { size, .. } = &mut ch.kind {
                    *size *= f;
                }
            }
        }
        _ => {}
    }
}

pub fn mirror(o: &mut ObjKind, c: Pos, horizontal: bool) {
    let f = |p: Pos| if horizontal { Pos::new(2.0 * c.x - p.x, p.y) } else { Pos::new(p.x, 2.0 * c.y - p.y) };
    map_points(o, &f);
    match o {
        ObjKind::Symbol { rot, .. } | ObjKind::Text { rot, .. } => *rot = (-*rot).rem_euclid(360.0),
        ObjKind::Door { rot, mirror, .. } => {
            *rot = if horizontal { (180.0 - *rot).rem_euclid(360.0) } else { (-*rot).rem_euclid(360.0) };
            *mirror = !*mirror;
        }
        ObjKind::Arc { start, end, .. } => {
            let (s, e) = if horizontal { (180.0 - *end, 180.0 - *start) } else { (-*end, -*start) };
            *start = s;
            *end = e;
        }
        _ => {}
    }
}

/// Bearbeitbare Punkte eines Objekts.
pub fn handles(o: &ObjKind) -> Vec<Pos> {
    match o {
        ObjKind::Line { a, b }
        | ObjKind::Rect { a, b, .. }
        | ObjKind::Image { a, b, .. }
        | ObjKind::Window { a, b, .. }
        | ObjKind::Legend { a, b, .. }
        | ObjKind::Block { a, b, .. }
        | ObjKind::Grid { a, b, .. }
        | ObjKind::FloorSection { a, b, .. }
        | ObjKind::Stairs { a, b, .. } => vec![*a, *b],
        ObjKind::Dimension { a, b, offset } => vec![*a, *b, a.lerp(*b, 0.5) + (*b - *a).norm().perp() * *offset],
        ObjKind::Polyline { pts, .. }
        | ObjKind::Spline { pts }
        | ObjKind::Hose { pts, .. }
        | ObjKind::Wall { pts, .. }
        | ObjKind::Room { pts, .. }
        | ObjKind::Area { pts, .. }
        | ObjKind::Route { pts, .. }
        | ObjKind::RevisionCloud { pts }
        | ObjKind::Fence { pts }
        | ObjKind::Rail { pts } => pts.clone(),
        ObjKind::Path { subpaths, .. } => subpaths.iter().flatten().copied().collect(),
        ObjKind::Circle { c, r } => vec![*c, *c + Pos::new(*r, 0.0)],
        ObjKind::Ellipse { c, rx, ry } => vec![*c, *c + Pos::new(*rx, 0.0), *c + Pos::new(0.0, *ry)],
        ObjKind::Arc { c, r, start, end } => vec![*c, *c + Pos::new(*r, 0.0).rotate(*start), *c + Pos::new(*r, 0.0).rotate(*end)],
        ObjKind::NGon { c, r, rot, .. } => vec![*c, *c + Pos::new(0.0, -*r).rotate(*rot)],
        ObjKind::Door { pos, width, rot, .. } => vec![*pos, *pos + door_dir(*rot) * *width],
        ObjKind::Symbol { pos, size, .. } => vec![*pos, *pos + Pos::new(size / 2.0, size / 2.0)],
        ObjKind::Text { pos, .. }
        | ObjKind::Table { pos, .. }
        | ObjKind::Note { pos, .. }
        | ObjKind::ScaleBar { pos, .. }
        | ObjKind::QrCode { pos, .. } => vec![*pos],
        ObjKind::FireCurve { c, .. } => vec![*c],
        ObjKind::Group { .. } => vec![],
    }
}

pub fn set_handle(o: &mut ObjKind, i: usize, p: Pos) {
    match o {
        ObjKind::Line { a, b }
        | ObjKind::Rect { a, b, .. }
        | ObjKind::Image { a, b, .. }
        | ObjKind::Window { a, b, .. }
        | ObjKind::Legend { a, b, .. }
        | ObjKind::Block { a, b, .. }
        | ObjKind::Grid { a, b, .. }
        | ObjKind::FloorSection { a, b, .. }
        | ObjKind::Stairs { a, b, .. } => {
            if i == 0 { *a = p } else { *b = p }
        }
        ObjKind::Dimension { a, b, offset } => match i {
            0 => *a = p,
            1 => *b = p,
            _ => *offset = (p - a.lerp(*b, 0.5)).dot((*b - *a).norm().perp()),
        },
        ObjKind::Polyline { pts, .. }
        | ObjKind::Spline { pts }
        | ObjKind::Hose { pts, .. }
        | ObjKind::Wall { pts, .. }
        | ObjKind::Room { pts, .. }
        | ObjKind::Area { pts, .. }
        | ObjKind::Route { pts, .. }
        | ObjKind::RevisionCloud { pts }
        | ObjKind::Fence { pts }
        | ObjKind::Rail { pts } => {
            if let Some(q) = pts.get_mut(i) {
                *q = p;
            }
        }
        ObjKind::Path { subpaths, .. } => {
            if let Some(q) = subpaths.iter_mut().flatten().nth(i) {
                *q = p;
            }
        }
        ObjKind::Circle { c, r } => {
            if i == 0 { *c = p } else { *r = p.dist(*c).max(0.5) }
        }
        ObjKind::Ellipse { c, rx, ry } => match i {
            0 => *c = p,
            1 => *rx = (p.x - c.x).abs().max(0.5),
            _ => *ry = (p.y - c.y).abs().max(0.5),
        },
        ObjKind::Arc { c, r, start, end } => match i {
            0 => *c = p,
            1 => {
                *start = (p - *c).angle_deg();
                *r = p.dist(*c).max(0.5);
            }
            _ => {
                *end = (p - *c).angle_deg();
                if *end < *start {
                    *end += 360.0;
                }
            }
        },
        ObjKind::NGon { c, r, rot, .. } => {
            if i == 0 {
                *c = p
            } else {
                *r = p.dist(*c).max(0.5);
                *rot = (p - *c).angle_deg() + 90.0;
            }
        }
        ObjKind::Door { pos, width, rot, .. } => {
            if i == 0 {
                *pos = p
            } else {
                *width = p.dist(*pos).max(2.0);
                *rot = (p - *pos).angle_deg();
            }
        }
        ObjKind::Symbol { pos, size, .. } => {
            if i == 0 {
                *pos = p
            } else {
                *size = ((p.x - pos.x).abs().max((p.y - pos.y).abs()) * 2.0).max(1.0)
            }
        }
        ObjKind::Text { pos, .. }
        | ObjKind::Table { pos, .. }
        | ObjKind::Note { pos, .. }
        | ObjKind::ScaleBar { pos, .. }
        | ObjKind::QrCode { pos, .. } => *pos = p,
        ObjKind::FireCurve { c, .. } => *c = p,
        ObjKind::Group { .. } => {}
    }
}

/// Punkt einfügen (zwischen den beiden nächsten Punkten eines Linienzugs).
pub fn insert_point(o: &mut ObjKind, p: Pos) -> bool {
    let (pts, closed) = match o {
        ObjKind::Polyline { pts, closed } | ObjKind::Wall { pts, closed, .. } => (pts, *closed),
        ObjKind::Room { pts, .. } | ObjKind::Area { pts, .. } | ObjKind::RevisionCloud { pts } => (pts, true),
        ObjKind::Spline { pts } | ObjKind::Hose { pts, .. } | ObjKind::Route { pts, .. } | ObjKind::Fence { pts } | ObjKind::Rail { pts } => (pts, false),
        _ => return false,
    };
    let n = pts.len();
    if n < 2 {
        return false;
    }
    let segs = if closed { n } else { n - 1 };
    let (mut best, mut bi) = (f32::MAX, 0);
    for i in 0..segs {
        let d = dist_to_segment(p, pts[i], pts[(i + 1) % n]);
        if d < best {
            best = d;
            bi = i;
        }
    }
    pts.insert(bi + 1, p);
    true
}

/// Schlüssel und Standardtext für die Legende.
/// Helles Grün wie bei Rettungswegflächen (nicht das dunkle Grün der Rettungszeichen).
fn hellgruen(c: Color) -> bool {
    let (r, g, b) = (c.0 as i32, c.1 as i32, c.2 as i32);
    g >= 170 && r >= 120 && g - r >= 15 && g - b >= 15
}

pub fn legend_key(doc: &Doc, o: &ObjKind, style: &Style) -> Option<(String, String)> {
    match o {
        ObjKind::Symbol { id, .. } => {
            let name = symbols::find(id)
                .map(|s| s.name.to_string())
                .or_else(|| doc.custom_symbols.iter().find(|c| &c.id == id).map(|c| c.name.clone()))?;
            Some((format!("sym:{id}"), name))
        }
        ObjKind::Area { preset, .. } => presets::find(preset).map(|p| (format!("area:{}", p.key), p.name.to_string())),
        // hellgrüne Flächen (z. B. aus importierten Plänen) sind Rettungswegflächen
        ObjKind::Polyline { closed: true, .. } | ObjKind::Path { closed: true, .. } if matches!(style.fill, Fill::Farbe(c) if hellgruen(c)) && {
            let b = bounds_doc(doc, o);
            b.w().max(b.h()) >= 20.0
        } =>
        {
            presets::find("fluchtbereich").map(|p| (format!("area:{}", p.key), p.name.to_string()))
        }
        ObjKind::Route { .. } => {
            let c = style.stroke.map(|s| s.color).unwrap_or(farbe::FLUCHT_HELL);
            if c == farbe::FLUCHT_HELL {
                Some(("route".into(), "Fluchtweg".into()))
            } else {
                Some(("laufweg".into(), "Laufweg der Feuerwehr".into()))
            }
        }
        ObjKind::Stairs { .. } => Some(("stairs".into(), "Treppe".into())),
        ObjKind::Fence { .. } => Some(("fence".into(), "Zaun / Gitter".into())),
        ObjKind::Rail { .. } => Some(("rail".into(), "Gleis".into())),
        ObjKind::Hose { .. } => Some(("hose".into(), "Schlauchleitung".into())),
        _ => None,
    }
}

/// Alle Legenden-Schlüssel im Plan (Reihenfolge des ersten Auftretens).
pub fn used_legend_keys(doc: &Doc) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    fn walk(doc: &Doc, objs: &[Obj], out: &mut Vec<(String, String)>) {
        for o in objs {
            if let ObjKind::Group { children } = &o.kind {
                walk(doc, children, out);
            } else if let Some(k) = legend_key(doc, &o.kind, &o.style) {
                if !out.iter().any(|(e, _)| *e == k.0) {
                    out.push(k);
                }
            }
        }
    }
    walk(doc, &doc.objects, &mut out);
    out
}

/// Legende abgleichen: neue Einträge anhängen, Reihenfolge und Texte des Nutzers behalten.
pub fn sync_legend(doc: &mut Doc) {
    let used = used_legend_keys(doc);
    for (key, label) in &used {
        if !doc.legend.entries.iter().any(|e| &e.key == key) {
            doc.legend.entries.push(LegendEntry { key: key.clone(), label: label.clone(), visible: true });
        }
    }
}

/// Platzhalter in Texten durch die Plandaten ersetzen: {objekt}, {anschrift}, {gebaeude}, {geschoss},
/// {stand}, {plannummer}, {ersteller}, {revision}, {notruf}, {sammelstelle}, {massstab}.
pub const PLATZHALTER: [&str; 11] = ["objekt", "anschrift", "gebaeude", "geschoss", "stand", "plannummer", "ersteller", "revision", "notruf", "sammelstelle", "massstab"];

pub fn platzhalter_wert(doc: &Doc, name: &str) -> Option<String> {
    let i = &doc.info;
    Some(match name {
        "objekt" => i.objekt.clone(),
        "anschrift" => i.anschrift.clone(),
        "gebaeude" => i.gebaeude.clone(),
        "geschoss" => i.geschoss.clone(),
        "stand" => i.stand.clone(),
        "plannummer" => i.plannummer.clone(),
        "ersteller" => i.ersteller.clone(),
        "revision" => i.revision.clone(),
        "notruf" => i.notruf.clone(),
        "sammelstelle" => i.sammelstelle.clone(),
        "massstab" => format!("1:{}", doc.scale_n),
        _ => return None,
    })
}

pub fn platzhalter<'a>(doc: &Doc, t: &'a str) -> std::borrow::Cow<'a, str> {
    if !t.contains('{') {
        return std::borrow::Cow::Borrowed(t);
    }
    let mut s = t.to_string();
    for name in PLATZHALTER {
        let k = format!("{{{name}}}");
        if s.contains(&k) {
            s = s.replace(&k, &platzhalter_wert(doc, name).unwrap_or_default());
        }
    }
    std::borrow::Cow::Owned(s)
}

/// Enthält das Objekt (auch in Gruppen) einen Text mit diesem Inhalt (ohne Groß-/Kleinschreibung)?
pub fn texte_enthalten(o: &ObjKind, suche: &str) -> bool {
    match o {
        ObjKind::Text { text, .. } => text.to_lowercase().contains(suche),
        ObjKind::Group { children } => children.iter().any(|c| texte_enthalten(&c.kind, suche)),
        _ => false,
    }
}

pub fn kind_name(o: &ObjKind) -> &'static str {
    match o {
        ObjKind::Line { .. } => "Linie",
        ObjKind::Polyline { closed: true, .. } => "Polygon",
        ObjKind::Polyline { .. } => "Linienzug",
        ObjKind::Path { .. } => "Pfad",
        ObjKind::Rect { .. } => "Rechteck",
        ObjKind::Circle { .. } => "Kreis",
        ObjKind::Ellipse { .. } => "Ellipse",
        ObjKind::Arc { .. } => "Bogen",
        ObjKind::NGon { .. } => "N-Eck",
        ObjKind::Spline { .. } => "Spline",
        ObjKind::Hose { .. } => "Schlauch",
        ObjKind::Text { .. } => "Text",
        ObjKind::Image { .. } => "Bild",
        ObjKind::Dimension { .. } => "Maßlinie",
        ObjKind::Table { .. } => "Tabelle",
        ObjKind::Wall { .. } => "Wand",
        ObjKind::Room { .. } => "Raum",
        ObjKind::Window { .. } => "Fenster",
        ObjKind::Door { .. } => "Tür",
        ObjKind::Stairs { .. } => "Treppe",
        ObjKind::Note { .. } => "Notiz",
        ObjKind::Area { .. } => "Fläche",
        ObjKind::Route { .. } => "Fluchtweg",
        ObjKind::Symbol { .. } => "Symbol",
        ObjKind::Legend { .. } => "Legende",
        ObjKind::Block { art, .. } => art.name(),
        ObjKind::Grid { .. } => "Planraster",
        ObjKind::FloorSection { .. } => "Geschoss-Schnitt",
        ObjKind::RevisionCloud { .. } => "Revisionswolke",
        ObjKind::Fence { .. } => "Zaun",
        ObjKind::Rail { .. } => "Gleis",
        ObjKind::ScaleBar { .. } => "Maßstabsleiste",
        ObjKind::QrCode { .. } => "QR-Code",
        ObjKind::FireCurve { .. } => "Kurve Feuerwehrzufahrt",
        ObjKind::Group { .. } => "Gruppe",
    }
}

/// Texte durchsuchen/ersetzen; liefert Anzahl Treffer.
pub fn replace_text(objs: &mut [Obj], find: &str, repl: &str, case: bool, apply: bool) -> usize {
    let mut n = 0;
    let matches = |s: &str| if case { s.contains(find) } else { s.to_lowercase().contains(&find.to_lowercase()) };
    let do_repl = |s: &mut String| {
        if case {
            *s = s.replace(find, repl);
        } else {
            let lower = s.to_lowercase();
            let f = find.to_lowercase();
            let mut out = String::new();
            let mut i = 0;
            while let Some(j) = lower[i..].find(&f) {
                out.push_str(&s[i..i + j]);
                out.push_str(repl);
                i += j + f.len();
            }
            out.push_str(&s[i..]);
            *s = out;
        }
    };
    for o in objs {
        let texts: Vec<&mut String> = match &mut o.kind {
            ObjKind::Text { text, .. } | ObjKind::Note { text, .. } | ObjKind::QrCode { text, .. } => vec![text],
            ObjKind::Room { name, .. } => vec![name],
            ObjKind::Table { cells, .. } => cells.iter_mut().flatten().collect(),
            ObjKind::Group { children } => {
                n += replace_text(children, find, repl, case, apply);
                vec![]
            }
            _ => vec![],
        };
        for t in texts {
            if !find.is_empty() && matches(t) {
                n += 1;
                if apply {
                    do_repl(t);
                }
            }
        }
    }
    n
}
