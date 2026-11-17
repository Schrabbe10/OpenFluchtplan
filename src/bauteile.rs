//! Bauteile in importierten Plänen erkennen: Aus den Einzelteilen einer CAD-PDF werden Objekte, die man
//! auch selbst zeichnen kann – Wandstreifen werden Wände, Türbögen mit Türblatt werden Türen, Farbflächen
//! werden Flächen nach Vorlage (Bedeutung der Farben aus der Legende des Plans).
//! Was nicht sicher erkannt wird, bleibt unverändert.

use crate::geom::*;
use crate::model::*;
use crate::{ops, presets};

/// Ergebnis der Erkennung (Anzahl je Bauteil)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Erkannt {
    pub waende: usize,
    pub tueren: usize,
    pub flaechen: usize,
}

/// Oberste Objekte aus `ids`, die frei sind (nicht gesperrt, keine Gruppe).
fn frei(doc: &Doc, ids: &[u64]) -> Vec<usize> {
    (0..doc.objects.len()).filter(|&i| ids.contains(&doc.objects[i].id) && !doc.objects[i].locked && !matches!(doc.objects[i].kind, ObjKind::Group { .. })).collect()
}

fn dunkel(c: Color) -> bool {
    (c.0 as u32 + c.1 as u32 + c.2 as u32) < 3 * 90 && c.3 > 200
}

fn fuellfarbe(o: &Obj) -> Option<Color> {
    match o.style.fill {
        Fill::Farbe(c) => Some(c),
        _ => None,
    }
}

fn linienfarbe(o: &Obj) -> Option<Color> {
    o.style.stroke.map(|s| s.color)
}

/// Umriss eines geschlossenen Objekts (Polygon oder Pfad mit einem Ring)
fn ring(o: &Obj) -> Option<Vec<Pos>> {
    let mut pts = match &o.kind {
        ObjKind::Polyline { pts, closed: true } => pts.clone(),
        ObjKind::Path { subpaths, closed: true } if subpaths.len() == 1 => subpaths[0].clone(),
        _ => return None,
    };
    if pts.len() >= 2 && pts[0].dist(*pts.last().unwrap()) < 0.05 {
        pts.pop();
    }
    Some(pts)
}

// ------------------------------------------------------------------ Wände

/// Gefülltes Rechteck als (Mittellinie, Dicke), wenn es ein Wandstreifen ist.
fn wandstreifen(o: &Obj) -> Option<(Pos, Pos, f32)> {
    if !fuellfarbe(o).is_some_and(dunkel) {
        return None;
    }
    let p = ring(o)?;
    if p.len() != 4 {
        return None;
    }
    // rechte Winkel
    for i in 0..4 {
        let a = (p[(i + 1) % 4] - p[i]).norm();
        let b = (p[(i + 2) % 4] - p[(i + 1) % 4]).norm();
        if a.dot(b).abs() > 0.05 {
            return None;
        }
    }
    let (l0, l1) = (p[0].dist(p[1]), p[1].dist(p[2]));
    let (lang, kurz) = (l0.max(l1), l0.min(l1));
    if !(0.12..=4.0).contains(&kurz) || lang < kurz * 2.5 {
        return None;
    }
    // Mittellinie zwischen den Mitten der kurzen Seiten
    let (a, b) = if l0 < l1 { (p[0].lerp(p[1], 0.5), p[2].lerp(p[3], 0.5)) } else { (p[1].lerp(p[2], 0.5), p[3].lerp(p[0], 0.5)) };
    Some((a, b, kurz))
}

/// Gerade Wandstücke in einer Flucht mit gleicher Dicke, die sich berühren, zusammenführen.
fn zusammenfuehren(mut w: Vec<(Pos, Pos, f32, Color)>) -> Vec<(Pos, Pos, f32, Color)> {
    loop {
        let mut neu = false;
        'suche: for i in 0..w.len() {
            for j in i + 1..w.len() {
                let (a, b, d, c) = w[i];
                let (p, q, e, f) = w[j];
                if (d - e).abs() > 0.05 || c != f {
                    continue;
                }
                let dir = (b - a).norm();
                if dir.dot((q - p).norm()).abs() < 0.999 || dist_to_line(p, a, dir) > 0.05 || dist_to_line(q, a, dir) > 0.05 {
                    continue;
                }
                // Projektionen auf die gemeinsame Gerade
                let t = |x: Pos| (x - a).dot(dir);
                let (s0, s1) = (0.0f32.min(t(b)), 0.0f32.max(t(b)));
                let (u0, u1) = (t(p).min(t(q)), t(p).max(t(q)));
                if u0 > s1 + d * 0.6 || u1 < s0 - d * 0.6 {
                    continue;
                }
                let (lo, hi) = (s0.min(u0), s1.max(u1));
                w[i] = (a + dir * lo, a + dir * hi, d, c);
                w.remove(j);
                neu = true;
                break 'suche;
            }
        }
        if !neu {
            return w;
        }
    }
}

fn dist_to_line(p: Pos, a: Pos, dir: Pos) -> f32 {
    (p - a).dot(dir.perp()).abs()
}

fn waende(doc: &mut Doc, ids: &[u64]) -> usize {
    let idx = frei(doc, ids);
    let mut streifen = Vec::new();
    let mut weg: Vec<u64> = Vec::new();
    let mut rahmen: Vec<(Vec<Pos>, BBox)> = Vec::new();
    for &i in &idx {
        let o = &doc.objects[i];
        if let Some((a, b, d)) = wandstreifen(o) {
            streifen.push((a, b, d, fuellfarbe(o).unwrap_or(farbe::SCHWARZ)));
            weg.push(o.id);
            let p = ring(o).unwrap_or_default();
            rahmen.push((p.clone(), BBox::from_pts(&p)));
        }
    }
    if streifen.is_empty() {
        return 0;
    }
    // Umrisslinien der Wandstreifen (CAD zeichnet oft Fläche und Kontur) mit entfernen
    for &i in &idx {
        let o = &doc.objects[i];
        let pts: Vec<Pos> = match &o.kind {
            ObjKind::Line { a, b } => vec![*a, *b],
            ObjKind::Polyline { pts, .. } => pts.clone(),
            _ => continue,
        };
        if weg.contains(&o.id) || pts.len() < 2 || !linienfarbe(o).is_some_and(dunkel) {
            continue;
        }
        let bb = BBox::from_pts(&pts);
        if rahmen.iter().any(|(r, rb)| bb.inside(&rb.expand(0.08)) && pts.iter().all(|p| dist_to_polyline(*p, r, true) < 0.08)) {
            weg.push(o.id);
        }
    }
    let waende = zusammenfuehren(streifen);
    let n = waende.len();
    // Wände an die Stelle des ersten Streifens in der Zeichenreihenfolge (unter Türen und Zeichen)
    let stelle = doc.objects.iter().position(|o| weg.contains(&o.id)).unwrap_or(0);
    doc.objects.retain(|o| !weg.contains(&o.id));
    let layer = doc.find_layer_by_name("Wände");
    let neu: Vec<Obj> = waende
        .into_iter()
        .map(|(a, b, d, c)| Obj { id: doc.alloc_id(), layer, kind: ObjKind::Wall { pts: vec![a, b], thickness: d, closed: false }, style: Style::line(c, d), print: true, locked: false })
        .collect();
    let stelle = stelle.min(doc.objects.len());
    doc.objects.splice(stelle..stelle, neu);
    n
}

// ------------------------------------------------------------------ Türen

/// Kreis durch drei Punkte
fn kreis(a: Pos, b: Pos, c: Pos) -> Option<(Pos, f32)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-6 {
        return None;
    }
    let (a2, b2, c2) = (a.dot(a), b.dot(b), c.dot(c));
    let m = Pos::new((a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d, (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d);
    Some((m, m.dist(a)))
}

/// Viertelkreisbogen (Türaufschlag): Mittelpunkt, Radius, Endpunkte.
fn tuerbogen(o: &Obj) -> Option<(Pos, f32, Pos, Pos)> {
    let ObjKind::Polyline { pts, closed: false } = &o.kind else { return None };
    if pts.len() < 4 || !linienfarbe(o).is_some_and(dunkel) || !matches!(o.style.fill, Fill::Keine) {
        return None;
    }
    let (a, e) = (pts[0], *pts.last().unwrap());
    let (m, r) = kreis(a, pts[pts.len() / 2], e)?;
    if !(3.0..=30.0).contains(&r) || pts.iter().any(|p| (p.dist(m) - r).abs() > r * 0.04 + 0.05) {
        return None;
    }
    let winkel = (a - m).norm().dot((e - m).norm()).clamp(-1.0, 1.0).acos().to_degrees();
    (70.0..=110.0).contains(&winkel).then_some((m, r, a, e))
}

fn tueren(doc: &mut Doc, ids: &[u64]) -> usize {
    let idx = frei(doc, ids);
    let linien: Vec<(usize, Pos, Pos)> = idx
        .iter()
        .filter_map(|&i| {
            let o = &doc.objects[i];
            match &o.kind {
                ObjKind::Line { a, b } => Some((i, *a, *b)),
                ObjKind::Polyline { pts, closed: false } if pts.len() == 2 => Some((i, pts[0], pts[1])),
                _ => None,
            }
            .filter(|_| linienfarbe(o).is_some_and(dunkel))
        })
        .collect();
    let mut weg: Vec<u64> = Vec::new();
    let mut neu: Vec<(usize, Obj)> = Vec::new();
    let layer = doc.find_layer_by_name("Wände");
    for &i in &idx {
        let o = &doc.objects[i];
        let Some((m, r, a, e)) = tuerbogen(o) else { continue };
        let tol = r * 0.06 + 0.15;
        // Türblatt: Linie vom Drehpunkt zu einem Bogenende
        let blatt = linien.iter().find(|(j, p, q)| {
            !weg.contains(&doc.objects[*j].id) && {
                let (p, q) = if p.dist(m) < q.dist(m) { (*p, *q) } else { (*q, *p) };
                p.dist(m) < tol && (q.dist(a) < tol || q.dist(e) < tol)
            }
        });
        let Some(&(j, p, q)) = blatt else { continue };
        let ende = if p.dist(m) < q.dist(m) { q } else { p };
        let (offen, zu) = if ende.dist(a) < ende.dist(e) { (a, e) } else { (e, a) };
        let dir = (zu - m).norm();
        let seite = dir.perp().dot(offen - m);
        let farbe_t = linienfarbe(o).unwrap_or(farbe::SCHWARZ);
        weg.push(o.id);
        weg.push(doc.objects[j].id);
        neu.push((
            i,
            Obj {
                id: 0,
                layer,
                kind: ObjKind::Door { pos: m, width: r, rot: dir.angle_deg(), mirror: seite < 0.0, double: false },
                style: Style::line(farbe_t, 0.35),
                print: true,
                locked: false,
            },
        ));
    }
    let n = neu.len();
    // Türen an die Stelle ihres Bogens; von hinten einsetzen, damit die Indizes stimmen
    neu.sort_by_key(|(i, _)| std::cmp::Reverse(*i));
    for (i, mut t) in neu {
        t.id = doc.alloc_id();
        doc.objects[i] = t;
    }
    let tuer_ids: Vec<u64> = doc.objects.iter().filter(|o| matches!(o.kind, ObjKind::Door { .. })).map(|o| o.id).collect();
    doc.objects.retain(|o| tuer_ids.contains(&o.id) || !weg.contains(&o.id));
    n
}

// ------------------------------------------------------------------ Flächen

/// Farbfläche eines Objekts: gefüllter Umriss oder kleine Gruppe aus gefülltem Umriss und Kontur
/// (so kommen Farbfelder aus CAD-Legenden an). Liefert Umriss, Stil (Füllung + ggf. Kontur) und Farbe.
fn flaeche_von(o: &Obj) -> Option<(Vec<Pos>, Style, Color)> {
    if let (Some(c), Some(p)) = (fuellfarbe(o), ring(o)) {
        return Some((p, o.style, c));
    }
    let ObjKind::Group { children } = &o.kind else { return None };
    if children.len() > 4 {
        return None;
    }
    let b = ops::bounds(&o.kind);
    let flaeche = children.iter().find_map(|c| {
        let cb = ops::bounds(&c.kind);
        (fuellfarbe(c).is_some() && ring(c).is_some() && cb.w() * cb.h() >= 0.8 * b.w() * b.h()).then_some(c)
    })?;
    // übrige Teile nur Konturen
    if children.iter().any(|c| !std::ptr::eq(c, flaeche) && (!matches!(c.style.fill, Fill::Keine) || matches!(c.kind, ObjKind::Text { .. } | ObjKind::Group { .. } | ObjKind::Symbol { .. }))) {
        return None;
    }
    let mut st = flaeche.style;
    if st.stroke.is_none() {
        st.stroke = children.iter().find_map(|c| c.style.stroke);
    }
    Some((ring(flaeche)?, st, fuellfarbe(flaeche)?))
}

/// Oberste Objekte aus `ids` einschließlich kleiner Gruppen (für Farbflächen)
fn frei_mit_gruppen(doc: &Doc, ids: &[u64]) -> Vec<usize> {
    (0..doc.objects.len()).filter(|&i| ids.contains(&doc.objects[i].id) && !doc.objects[i].locked).collect()
}

fn begriff(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// Flächenvorlage zu einem Legendentext
pub fn flaeche_fuer_text(text: &str) -> Option<&'static str> {
    let t = begriff(text);
    let weg = t.contains("rettungsweg") || t.contains("fluchtweg");
    if weg && (t.contains("vertikal") || t.contains("treppe")) {
        Some("fluchttreppe")
    } else if weg {
        Some("fluchtbereich")
    } else if t.contains("nichtbefahrbar") {
        Some("nicht_befahrbar")
    } else if t.contains("befahrbar") {
        Some("befahrbar")
    } else if t.contains("löschwasser") || t.contains("loeschwasser") {
        Some("loeschwasser")
    } else if t.contains("besonderegefahr") || t.contains("gefahrenbereich") {
        Some("gefahr")
    } else if t.contains("baulicheanlage") || t == "gebäude" || t == "gebaeude" {
        Some("bezugsgebaeude")
    } else {
        None
    }
}

fn farbabstand(a: Color, b: Color) -> f32 {
    let d = |x: u8, y: u8| (x as f32 - y as f32).powi(2);
    (d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)).sqrt()
}

/// Farben der Legende: kleine Farbfelder mit Text rechts daneben
fn farben_aus_legende(doc: &Doc, idx: &[usize]) -> Vec<(Color, &'static str)> {
    let mut texte: Vec<(BBox, String)> = Vec::new();
    fn sammeln(doc: &Doc, o: &Obj, out: &mut Vec<(BBox, String)>) {
        match &o.kind {
            ObjKind::Text { text, .. } => out.push((ops::bounds_doc(doc, &o.kind), text.clone())),
            ObjKind::Group { children } => children.iter().for_each(|c| sammeln(doc, c, out)),
            _ => {}
        }
    }
    for o in &doc.objects {
        sammeln(doc, o, &mut texte);
    }
    let mut out = Vec::new();
    for &i in idx {
        let o = &doc.objects[i];
        let Some((_, _, c)) = flaeche_von(o) else { continue };
        let b = ops::bounds_doc(doc, &o.kind);
        if !(3.0..=20.0).contains(&b.w()) || !(1.5..=10.0).contains(&b.h()) {
            continue;
        }
        let h = b.h();
        let text: String = texte
            .iter()
            .filter(|(t, _)| t.min.x >= b.max.x - 0.5 && t.min.x <= b.max.x + 3.0 * h.max(4.0) && t.center().y >= b.min.y - h && t.center().y <= b.max.y + h)
            .map(|(_, s)| s.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        if let Some(k) = flaeche_fuer_text(&text) {
            out.push((c, k));
        }
    }
    out
}

fn flaechen(doc: &mut Doc, ids: &[u64]) -> usize {
    let idx = frei_mit_gruppen(doc, ids);
    let gelernt = farben_aus_legende(doc, &idx);
    // ohne Legende nur die eindeutigen Rettungsweg-Farben – und nur in Plänen mit Sicherheitszeichen
    // (auf Textseiten sind grüne Flächen z. B. Tabellenzellen)
    let plan = !gelernt.is_empty() || doc.objects.iter().any(|o| matches!(o.kind, ObjKind::Symbol { .. }));
    let standard: Vec<(Color, &'static str)> = ["fluchtbereich", "fluchttreppe"]
        .iter()
        .filter(|_| plan)
        .filter_map(|k| presets::find(k).and_then(|p| if let Fill::Farbe(c) = p.style.fill { Some((c, *k)) } else { None }))
        .collect();
    let mut n = 0;
    for &i in &idx {
        let o = &doc.objects[i];
        let Some((pts, style, c)) = flaeche_von(o) else { continue };
        if pts.len() < 3 || polygon_area(&pts).abs() < 4.0 {
            continue;
        }
        let key = gelernt
            .iter()
            .filter(|(g, _)| farbabstand(*g, c) < 12.0)
            .map(|(_, k)| *k)
            .next()
            .or_else(|| standard.iter().filter(|(g, _)| farbabstand(*g, c) < 40.0).map(|(_, k)| *k).next());
        let Some(key) = key else { continue };
        doc.objects[i].kind = ObjKind::Area { pts, preset: key.into() };
        // Farbe des Plans behalten (Darstellung bleibt gleich)
        doc.objects[i].style = style;
        n += 1;
    }
    n
}

// ------------------------------------------------------------------ Ablauf

/// Kleine Gruppen, die kein Schild sind (kein gefüllter Hintergrund), wieder in Einzelteile auflösen –
/// sonst hängen z. B. Wandenden und Türbögen in zufälligen Gruppen fest.
fn lose_gruppen_aufloesen(doc: &mut Doc, ids: &[u64], max_mm: f32) -> Vec<u64> {
    let mut neue_ids = Vec::new();
    let mut out = Vec::with_capacity(doc.objects.len());
    for o in std::mem::take(&mut doc.objects) {
        let b = ops::bounds_doc(doc, &o.kind);
        let aufloesen = ids.contains(&o.id)
            && b.w().max(b.h()) <= max_mm
            && match &o.kind {
                ObjKind::Group { children } => {
                    let flaeche = (b.w() * b.h()).max(0.01);
                    let schild = children.iter().any(|c| {
                        let cb = ops::bounds(&c.kind);
                        fuellfarbe(c).is_some() && ring(c).is_some() && cb.w() * cb.h() >= 0.6 * flaeche
                    });
                    let text = children.iter().any(|c| matches!(c.kind, ObjKind::Text { .. }));
                    !schild && !text && children.iter().all(|c| !matches!(c.kind, ObjKind::Group { .. } | ObjKind::Symbol { .. }))
                }
                _ => false,
            };
        if aufloesen {
            if let ObjKind::Group { children } = o.kind {
                for c in children {
                    neue_ids.push(c.id);
                    out.push(c);
                }
            }
        } else {
            out.push(o);
        }
    }
    doc.objects = out;
    neue_ids
}

/// Flächen erkennen (vor dem Ersetzen der Legende, damit die Farbfelder als Einträge zählen).
pub fn flaechen_erkennen(doc: &mut Doc, ids: &[u64]) -> usize {
    flaechen(doc, ids)
}

/// Wände und Türen erkennen. `ids`: Objekte des Imports.
pub fn waende_und_tueren(doc: &mut Doc, ids: &[u64], max_mm: f32) -> Erkannt {
    let mut alle: Vec<u64> = ids.to_vec();
    alle.extend(lose_gruppen_aufloesen(doc, ids, max_mm));
    let tueren = tueren(doc, &alle);
    // Wandstreifen nur in Grundrissen (mit Türen) – auf Text- und Tabellenseiten sind es Tabellenlinien
    let waende = if tueren > 0 { waende(doc, &alle) } else { 0 };
    Erkannt { waende, tueren, flaechen: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(kind: ObjKind, style: Style) -> Obj {
        Obj { id: 0, layer: 0, kind, style, print: true, locked: false }
    }

    #[test]
    fn wandstreifen_tuer_und_flaeche() {
        let mut doc = Doc::default();
        let mut add = |o: Obj| {
            let mut o = o;
            let id = doc.alloc_id();
            o.id = id;
            doc.objects.push(o);
            id
        };
        let schwarz = Style::filled(farbe::SCHWARZ);
        let rechteck = |x0: f32, y0: f32, x1: f32, y1: f32| ObjKind::Polyline { pts: vec![Pos::new(x0, y0), Pos::new(x1, y0), Pos::new(x1, y1), Pos::new(x0, y1)], closed: true };
        let mut ids = vec![
            add(obj(rechteck(10.0, 10.0, 40.0, 10.6), schwarz)),
            add(obj(rechteck(40.0, 10.0, 70.0, 10.6), schwarz)),
            // Kontur des ersten Streifens
            add(obj(ObjKind::Line { a: Pos::new(10.0, 10.0), b: Pos::new(40.0, 10.0) }, Style::line(farbe::SCHWARZ, 0.1))),
        ];
        // Tür: Drehpunkt (80,20), Breite 9, geschlossen nach +x, offen nach +y
        let bogen: Vec<Pos> = (0..=8).map(|k| Pos::new(80.0, 20.0) + Pos::new(9.0, 0.0).rotate(k as f32 * 90.0 / 8.0)).collect();
        let offen = *bogen.last().unwrap();
        ids.push(add(obj(ObjKind::Polyline { pts: bogen, closed: false }, Style::line(farbe::SCHWARZ, 0.1))));
        ids.push(add(obj(ObjKind::Line { a: Pos::new(80.0, 20.0), b: offen }, Style::line(farbe::SCHWARZ, 0.2))));
        // Rettungswegfläche in Planfarbe; ein Sicherheitszeichen kennzeichnet den Grundriss
        ids.push(add(obj(rechteck(10.0, 30.0, 60.0, 40.0), Style::filled(Color::rgb(0xb9, 0xce, 0xac)))));
        add(obj(ObjKind::Symbol { id: "F001".into(), pos: Pos::new(100.0, 100.0), size: 8.0, rot: 0.0 }, Style::NONE));

        assert_eq!(flaechen_erkennen(&mut doc, &ids), 1);
        let e = waende_und_tueren(&mut doc, &ids, 12.0);
        assert_eq!(e, Erkannt { waende: 1, tueren: 1, flaechen: 0 });
        let wand = doc.objects.iter().find_map(|o| if let ObjKind::Wall { pts, thickness, .. } = &o.kind { Some((pts.clone(), *thickness)) } else { None }).unwrap();
        // Richtung der Mittellinie ist gleichgültig
        let enden = |a: Pos, b: Pos| (wand.0[0].dist(a) < 0.01 && wand.0[1].dist(b) < 0.01) || (wand.0[0].dist(b) < 0.01 && wand.0[1].dist(a) < 0.01);
        assert!((wand.1 - 0.6).abs() < 0.01 && enden(Pos::new(10.0, 10.3), Pos::new(70.0, 10.3)), "{wand:?}");
        assert!(!doc.objects.iter().any(|o| matches!(o.kind, ObjKind::Line { .. })), "Kontur und Türblatt entfernt");
        let tuer = doc.objects.iter().find_map(|o| if let ObjKind::Door { pos, width, rot, .. } = &o.kind { Some((*pos, *width, *rot)) } else { None }).unwrap();
        assert!(tuer.0.dist(Pos::new(80.0, 20.0)) < 0.05 && (tuer.1 - 9.0).abs() < 0.05, "{tuer:?}");
        assert!(doc.objects.iter().any(|o| matches!(&o.kind, ObjKind::Area { preset, .. } if preset == "fluchtbereich")));
    }
}
