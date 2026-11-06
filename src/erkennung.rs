//! Symbolerkennung für importierte Pläne: Gruppen aus Einzelteilen (z. B. aus einer PDF) werden
//! mit der Symbolbibliothek verglichen und bei ausreichender Ähnlichkeit durch das Normzeichen
//! ersetzt. Danach verhalten sie sich wie selbst gesetzte Zeichen (Legende, Prüfung, Darstellung).

use std::sync::OnceLock;

use resvg::{tiny_skia, usvg};

use crate::geom::*;
use crate::model::*;
use crate::{export, ops, symbols};

/// Auflösung beim Rastern
const R: usize = 72;
/// Vergleichsraster (nach Zuschnitt auf den Inhalt)
const N: usize = 20;
/// Höchste mittlere Farbabweichung (0..1), bis zu der ein Zeichen als erkannt gilt
const SCHWELLE: f32 = 0.085;
/// Schwelle für Zeichen, deren Text übereinstimmt (Schriftbild weicht nach dem Import oft ab)
const SCHWELLE_TEXT: f32 = 0.2;
/// Schwelle für einteilige Formen (strenger, damit Pfeilspitzen o. Ä. nicht als Zeichen gelten)
const SCHWELLE_EINZELN: f32 = 0.03;
/// Größte Kantenlänge (mm), bis zu der eine Gruppe als Zeichen in Frage kommt
const MAX_MM: f32 = 40.0;

type Bild = Vec<[f32; 3]>;

struct Vorlage {
    id: &'static str,
    cat: symbols::Category,
    rot: f32,
    bild: Bild,
    /// enger Inhaltsrahmen im Symbolfeld 0..100 (x0, y0, x1, y1)
    bb: [f32; 4],
    /// Textinhalt (normiert, siehe `text_norm`)
    text: String,
}

/// Text vergleichbar machen: ohne Leerraum, Großbuchstaben, Zeichen sortiert
/// (Reihenfolge und Aufteilung der Textstücke ändern sich beim Import).
fn text_norm(s: &str) -> String {
    let mut v: Vec<char> = s.chars().filter(|c| !c.is_whitespace()).flat_map(|c| c.to_uppercase()).collect();
    v.sort_unstable();
    v.into_iter().collect()
}

/// Inhalt aller <text>-Elemente eines Symbol-SVG
fn svg_text(svg: &str) -> String {
    let mut out = String::new();
    let mut rest = svg;
    while let Some(i) = rest.find("<text") {
        rest = &rest[i..];
        let Some(a) = rest.find('>') else { break };
        let Some(e) = rest.find("</text>") else { break };
        if a < e {
            out.push_str(&rest[a + 1..e].replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">"));
        }
        rest = &rest[e + 7..];
    }
    out
}

fn obj_text(o: &Obj, out: &mut String) {
    match &o.kind {
        ObjKind::Text { text, .. } => out.push_str(text),
        ObjKind::Group { children } => children.iter().for_each(|c| obj_text(c, out)),
        _ => {}
    }
}

fn rastern(svg: &str) -> Option<Vec<[u8; 3]>> {
    let tree = usvg::Tree::from_str(svg, export::usvg_options()).ok()?;
    let mut pm = tiny_skia::Pixmap::new(R as u32, R as u32)?;
    pm.fill(tiny_skia::Color::WHITE);
    let s = tree.size();
    resvg::render(&tree, tiny_skia::Transform::from_scale(R as f32 / s.width(), R as f32 / s.height()), &mut pm.as_mut());
    Some(pm.pixels().iter().map(|p| [p.red(), p.green(), p.blue()]).collect())
}

/// Auf den sichtbaren Inhalt zuschneiden und auf N × N mitteln. Liefert auch den Inhaltsrahmen (Pixel).
fn normieren(px: &[[u8; 3]]) -> Option<(Bild, [f32; 4])> {
    let (mut x0, mut y0, mut x1, mut y1) = (R, R, 0, 0);
    for y in 0..R {
        for x in 0..R {
            if px[y * R + x].iter().any(|&c| c < 235) {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    if x1 <= x0 + 2 || y1 <= y0 + 2 {
        return None;
    }
    // dünn besetzte Randzeilen/-spalten (Streupunkte, angrenzende Linienenden) abschneiden,
    // höchstens 15 % je Seite; Vorlagen und Planzeichen werden gleich behandelt
    let voll = |x: usize, y: usize| px[y * R + x].iter().any(|&c| c < 235);
    let (ox0, oy0, ox1, oy1) = (x0, y0, x1, y1);
    let (mx, my) = ((x1 - x0) * 15 / 100, (y1 - y0) * 15 / 100);
    for _ in 0..2 {
        let zeile = |y: usize, x0: usize, x1: usize| (x0..x1).filter(|&x| voll(x, y)).count();
        let spalte = |x: usize, y0: usize, y1: usize| (y0..y1).filter(|&y| voll(x, y)).count();
        while y0 - oy0 < my && zeile(y0, x0, x1) * 10 < x1 - x0 {
            y0 += 1;
        }
        while oy1 - y1 < my && zeile(y1 - 1, x0, x1) * 10 < x1 - x0 {
            y1 -= 1;
        }
        while x0 - ox0 < mx && spalte(x0, y0, y1) * 10 < y1 - y0 {
            x0 += 1;
        }
        while ox1 - x1 < mx && spalte(x1 - 1, y0, y1) * 10 < y1 - y0 {
            x1 -= 1;
        }
    }
    let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let mut out = vec![[0.0; 3]; N * N];
    for j in 0..N {
        let ya = (y0 as f32 + h * j as f32 / N as f32).floor() as usize;
        let yb = ((y0 as f32 + h * (j + 1) as f32 / N as f32).ceil() as usize).clamp(ya + 1, R);
        for i in 0..N {
            let xa = (x0 as f32 + w * i as f32 / N as f32).floor() as usize;
            let xb = ((x0 as f32 + w * (i + 1) as f32 / N as f32).ceil() as usize).clamp(xa + 1, R);
            let (mut acc, mut n) = ([0.0f32; 3], 0.0);
            for y in ya..yb {
                for x in xa..xb {
                    for c in 0..3 {
                        acc[c] += px[y * R + x][c] as f32;
                    }
                    n += 1.0;
                }
            }
            out[j * N + i] = [acc[0] / n / 255.0, acc[1] / n / 255.0, acc[2] / n / 255.0];
        }
    }
    Some((out, [x0 as f32, y0 as f32, x1 as f32, y1 as f32]))
}

/// Um 90° im Uhrzeigersinn drehen (wie `rot` beim Zeichnen)
fn drehen(b: &Bild) -> Bild {
    let mut out = vec![[0.0; 3]; N * N];
    for y in 0..N {
        for x in 0..N {
            out[y * N + x] = b[(N - 1 - x) * N + y];
        }
    }
    out
}

fn vorlagen() -> &'static [Vorlage] {
    static V: OnceLock<Vec<Vorlage>> = OnceLock::new();
    V.get_or_init(|| {
        let mut out = Vec::new();
        for s in symbols::library().iter().filter(|s| s.cat != symbols::Category::Einrichtung) {
            let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'>{}</svg>", s.svg);
            let Some((bild, px)) = rastern(&svg).and_then(|p| normieren(&p)) else { continue };
            let f = 100.0 / R as f32;
            let mut bb = [px[0] * f, px[1] * f, px[2] * f, px[3] * f];
            let mut b = bild;
            let text = text_norm(&svg_text(&s.svg));
            for k in 0..4 {
                out.push(Vorlage { id: s.id, cat: s.cat, rot: k as f32 * 90.0, bild: b.clone(), bb, text: text.clone() });
                b = drehen(&b);
                bb = [100.0 - bb[3], bb[0], 100.0 - bb[1], bb[2]];
            }
        }
        out
    })
}

fn abstand(a: &Bild, b: &Bild) -> f32 {
    a.iter().zip(b).map(|(p, q)| (p[0] - q[0]).abs() + (p[1] - q[1]).abs() + (p[2] - q[2]).abs()).sum::<f32>() / (3 * N * N) as f32
}

/// Ein Objekt allein rastern: quadratischer Ausschnitt um seinen Rahmen, mit Rand für Strichstärken.
/// Liefert die Pixel und den Ausschnitt (Ursprung, Kantenlänge in mm).
fn gruppe_rastern(doc: &Doc, o: &Obj, b: BBox) -> Option<(Vec<[u8; 3]>, Pos, f32)> {
    let s = b.w().max(b.h()).max(0.1) * 1.25;
    let origin = Pos::new(b.center().x - s / 2.0, b.center().y - s / 2.0);
    let mut k = o.clone();
    ops::translate(&mut k.kind, Pos::new(-origin.x, -origin.y));
    let mut tmp = Doc { frame: FrameKind::Ohne, page: Page { format: PaperFormat::Eigenes { w: s, h: s }, landscape: false }, ..Doc::default() };
    tmp.layers = doc.layers.clone();
    tmp.custom_symbols = doc.custom_symbols.clone();
    tmp.objects = vec![k];
    Some((rastern(&export::plan_svg(&tmp, export::Stamp::Keiner))?, origin, s))
}

struct Fund {
    id: &'static str,
    pos: Pos,
    size: f32,
    rot: f32,
    /// Teil der Gruppe, das nicht zum Zeichen gehört (z. B. eine angrenzende Pfeilspitze)
    rest: Option<usize>,
}

/// Sichtbares Bild (normiert) und Inhaltsrahmen in mm eines möglichen Zeichens
fn bild_von(doc: &Doc, o: &Obj) -> Option<(Bild, BBox)> {
    let b = ops::bounds_doc(doc, &o.kind);
    let einzeln = !matches!(o.kind, ObjKind::Group { .. });
    if b.w().max(b.h()) > MAX_MM || b.w().min(b.h()) < if einzeln { 4.0 } else { 1.5 } {
        return None;
    }
    let (px, origin, kante) = gruppe_rastern(doc, o, b)?;
    let (bild, pb) = normieren(&px)?;
    let f = kante / R as f32;
    Some((bild, BBox::from_two(origin + Pos::new(pb[0] * f, pb[1] * f), origin + Pos::new(pb[2] * f, pb[3] * f))))
}

fn vorlage(id: &str, rot: f32) -> Option<&'static Vorlage> {
    vorlagen().iter().find(|v| v.id == id && v.rot == rot)
}

/// Erkennungsmuster aus der Legende des importierten Plans (Zeichen in der Darstellung des Erstellers)
struct Muster {
    id: &'static str,
    rot: f32,
    bild: Bild,
    /// Aufschlag für unwahrscheinliche Fassungen (gedreht), damit die ungedrehte bei Gleichstand gewinnt
    strafe: f32,
    /// Seitenverhältnis (Breite/Höhe) des Zeichens im Plan
    seiten: f32,
}

/// Schwelle für Zeichen, die einem Muster aus der Legende des Plans gleichen (gleicher Ersteller)
const SCHWELLE_GELERNT: f32 = 0.07;

/// Bestes Normzeichen für ein Objekt: (Symbol-ID, Drehung, Abstand/Schwelle, Inhaltsrahmen).
/// Zuerst gegen die aus der Planlegende gelernten Muster, dann gegen die Bibliothek.
fn vergleichen(doc: &Doc, o: &Obj, muster: &[Muster]) -> Option<(&'static str, f32, f32, BBox)> {
    let (bild, b) = bild_von(doc, o)?;
    let mut best: Option<(&'static str, f32, f32)> = None;
    let seiten = b.w() / b.h().max(0.01);
    for m in muster.iter().filter(|m| {
        let q = seiten / m.seiten;
        (0.75..=1.33).contains(&q)
    }) {
        let r = abstand(&bild, &m.bild) / SCHWELLE_GELERNT + m.strafe;
        if best.is_none_or(|t| r < t.2) {
            best = Some((m.id, m.rot, r));
        }
    }
    if best.is_some_and(|t| t.2 <= 1.0) {
        return best.map(|(id, rot, r)| (id, rot, r, b));
    }
    let mut t = String::new();
    obj_text(o, &mut t);
    let t = text_norm(&t);
    // ein einzelner Buchstabe sagt zu wenig (z. B. „I“ in einem Kästchen)
    let mit_text = t.chars().count() >= 2;
    let schwelle = if mit_text {
        SCHWELLE_TEXT
    } else if !matches!(o.kind, ObjKind::Group { .. }) {
        SCHWELLE_EINZELN
    } else {
        SCHWELLE
    };
    // Brandmeldetechnik-Zeichen gibt es nur auf Feuerwehr-Laufkarten. Hat der Plan eine eigene
    // Legende, kommen außer den dort gefundenen Zeichen nur Sicherheitszeichen nach ISO 7010 in Frage.
    let laufkarte = doc.frame == FrameKind::Laufkarte;
    let iso = |c: symbols::Category| matches!(c, symbols::Category::Rettung | symbols::Category::Brandschutz | symbols::Category::Warnung | symbols::Category::Gebot | symbols::Category::Verbot);
    let zulaessig = |v: &Vorlage| (laufkarte || v.cat != symbols::Category::Brandmeldetechnik) && (muster.is_empty() || iso(v.cat) || muster.iter().any(|m| m.id == v.id));
    // Seitenverhältnis muss passen (der Bildvergleich selbst streckt alles auf ein Quadrat)
    let seiten = b.w() / b.h().max(0.01);
    let passt = |v: &Vorlage| {
        let q = seiten / ((v.bb[2] - v.bb[0]) / (v.bb[3] - v.bb[1]).max(0.01));
        (0.75..=1.33).contains(&q)
    };
    for v in vorlagen().iter().filter(|v| (!mit_text || v.text == t) && zulaessig(v) && passt(v)) {
        // gedrehte Fassungen nur bei deutlich besserer Übereinstimmung; gleich aussehende Zeichen
        // (z. B. Rauchmelder / Rauchabzug) nach Planart unterscheiden
        let fremd = match v.cat {
            symbols::Category::Brandmeldetechnik => doc.frame != FrameKind::Laufkarte,
            symbols::Category::Feuerwehr => doc.frame == FrameKind::Laufkarte,
            _ => false,
        };
        let d = abstand(&bild, &v.bild) + if v.rot == 0.0 { 0.0 } else { 0.01 } + if fremd { 0.002 } else { 0.0 };
        let r = d / schwelle;
        if best.is_none_or(|t| r < t.2) {
            best = Some((v.id, v.rot, r));
        }
    }
    best.map(|(id, rot, r)| (id, rot, r, b))
}

/// Begriffe aus Planlegenden und die Zeichen, die dazu passen (Text ohne Leerzeichen und Satzzeichen, klein)
const BEGRIFFE: &[(&str, &[&str])] = &[
    ("standort", &["STANDORT"]),
    ("befindensichhier", &["STANDORT"]),
    ("siesindhier", &["STANDORT"]),
    ("notausgang", &["E001", "E002"]),
    ("rettungsweg", &["PFEIL_GRUEN", "PFEIL_GRUEN_DIAG", "E001", "E002"]),
    ("fluchtweg", &["PFEIL_GRUEN", "PFEIL_GRUEN_DIAG", "E001", "E002"]),
    ("richtungsangabe", &["PFEIL_GRUEN", "PFEIL_GRUEN_DIAG", "PFEIL_ROT", "PFEIL_ROT_DIAG"]),
    ("erstehilfe", &["E003"]),
    ("ersthilfe", &["E003"]),
    ("sammelstelle", &["E007"]),
    ("sammelplatz", &["E007"]),
    ("notruftelefon", &["E004"]),
    ("notruf", &["E004", "F006"]),
    ("telefon", &["E004", "F006"]),
    ("arzt", &["E009"]),
    ("defibrillator", &["E010"]),
    ("augenspül", &["E011"]),
    ("augendusche", &["E011"]),
    ("notdusche", &["E012"]),
    ("körperdusche", &["E012"]),
    ("krankentrage", &["E013"]),
    ("notausstieg", &["E016", "E017"]),
    ("rettungsausstieg", &["E017"]),
    ("feuerlöscher", &["F001"]),
    ("wandhydrant", &["F002", "FW_WANDHYDRANT"]),
    ("löschschlauch", &["F002"]),
    ("feuerleiter", &["F003"]),
    ("brandbekämpfung", &["F004"]),
    ("brandmelder", &["F005"]),
    ("feuermelder", &["F005"]),
    ("druckknopfmelder", &["F005"]),
    ("brandmeldetelefon", &["F006"]),
    ("löschdecke", &["F_LOESCHDECKE"]),
    ("brandwand", &["FW_BRANDWAND"]),
];

fn begriff_norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

/// Zeichen, die zu einem Legendentext passen
fn ids_fuer_text(text: &str) -> Vec<&'static str> {
    let t = begriff_norm(text);
    let mut out: Vec<&'static str> = Vec::new();
    for (k, ids) in BEGRIFFE {
        if t.contains(k) {
            out.extend(ids.iter().copied());
        }
    }
    if out.is_empty() && t.chars().count() >= 6 {
        for s in symbols::library() {
            let n = begriff_norm(s.name);
            if n.chars().count() >= 6 && (t.contains(&n) || n.contains(&t)) {
                out.push(s.id);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn spiegeln(b: &Bild) -> Bild {
    let mut out = vec![[0.0; 3]; N * N];
    for y in 0..N {
        for x in 0..N {
            out[y * N + x] = b[y * N + N - 1 - x];
        }
    }
    out
}

/// Planlegende auswerten: Zeichen mit einem Text rechts daneben („Feuerlöscher“, „Standort“ …)
/// werden als Muster gemerkt. So werden auch Zeichen erkannt, die der Ersteller anders zeichnet als
/// unsere Bibliothek.
fn aus_legende_lernen(doc: &Doc) -> Vec<Muster> {
    // alle Texte einzeln (auch aus Gruppen, in die der Import sie gesteckt hat)
    fn texte_sammeln(doc: &Doc, o: &Obj, out: &mut Vec<(BBox, String)>) {
        match &o.kind {
            ObjKind::Text { text, .. } if !text.trim().is_empty() => out.push((ops::bounds_doc(doc, &o.kind), text.clone())),
            ObjKind::Group { children } => children.iter().for_each(|c| texte_sammeln(doc, c, out)),
            _ => {}
        }
    }
    let mut texte: Vec<(BBox, String)> = Vec::new();
    for o in &doc.objects {
        texte_sammeln(doc, o, &mut texte);
    }
    let mut out = Vec::new();
    for o in &doc.objects {
        if !matches!(o.kind, ObjKind::Group { .. } | ObjKind::Polyline { closed: true, .. } | ObjKind::Path { .. }) {
            continue;
        }
        let b = ops::bounds_doc(doc, &o.kind);
        let h = b.w().max(b.h());
        // Zeichen sind etwa quadratisch; beschriftete Kästen o. Ä. scheiden aus
        if !(3.0..=MAX_MM).contains(&h) || b.w().min(b.h()) < h * 0.6 {
            continue;
        }
        // Texte rechts daneben auf gleicher Höhe (auch hinter einem zweiten Zeichen, z. B. Notausgang + Pfeil)
        let mut neben: Vec<&(BBox, String)> = texte
            .iter()
            .filter(|(t, _)| t.min.x >= b.max.x - 0.5 && t.min.x <= b.max.x + 4.5 * h && t.center().y >= b.min.y - 0.25 * h && t.center().y <= b.max.y + 0.25 * h)
            .collect();
        let Some(x0) = neben.iter().map(|(t, _)| t.min.x).reduce(f32::min) else { continue };
        neben.retain(|(t, _)| t.min.x <= x0 + 2.0);
        neben.sort_by(|a, b| a.0.min.y.total_cmp(&b.0.min.y));
        let text: String = neben.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join(" ");
        // Legendentexte sind kurz (Adresszeilen mit „Telefon …“ o. Ä. nicht)
        if begriff_norm(&text).chars().count() > 50 {
            continue;
        }
        let ids = ids_fuer_text(&text);
        if ids.is_empty() {
            continue;
        }
        let Some((bild, inhalt)) = bild_von(doc, o) else { continue };
        let seiten = inhalt.w() / inhalt.h().max(0.01);
        // unter den passenden Zeichen das ähnlichste wählen; ein Richtungspfeil nur bei klarer
        // Ähnlichkeit (zu „Rettungsweg / Notausgang“ gehören Pfeil und Notausgangszeichen)
        let mut wahl: Vec<(&'static str, f32)> = ids.iter().filter_map(|id| vorlage(id, 0.0).map(|v| (*id, abstand(&bild, &v.bild)))).collect();
        if wahl.iter().any(|(id, _)| !id.starts_with("PFEIL_")) {
            wahl.retain(|(id, d)| !id.starts_with("PFEIL_") || *d < 0.18);
        }
        let Some((id, d)) = wahl.into_iter().min_by(|a, b| a.1.total_cmp(&b.1)) else { continue };
        if d > 0.3 {
            continue;
        }
        // Zeichen kommen gedreht vor (Richtungspfeile, Brandwand an waagerechter Wand …),
        // Notausgänge auch gespiegelt
        let strafe = if id.starts_with("PFEIL_") { 0.0 } else { 0.05 };
        let mut bb = bild.clone();
        for k in 0..4 {
            let s = if k % 2 == 0 { seiten } else { 1.0 / seiten };
            out.push(Muster { id, rot: k as f32 * 90.0, bild: bb.clone(), strafe: if k == 0 { 0.0 } else { strafe }, seiten: s });
            bb = drehen(&bb);
        }
        let gegen = match id {
            "E001" => Some("E002"),
            "E002" => Some("E001"),
            _ => None,
        };
        if let Some(g) = gegen {
            out.push(Muster { id: g, rot: 0.0, bild: spiegeln(&bild), strafe: 0.0, seiten });
        }
    }
    out
}

/// Schräglage eines Schildes (z. B. an einer schrägen Wand): Winkel der längsten Kante der größten
/// gefüllten Fläche, auf −45…45° gebracht, und deren Mitte. `None`, wenn es (fast) gerade steht.
fn schraeglage(doc: &Doc, o: &Obj) -> Option<(f32, Pos)> {
    let teile: Vec<&Obj> = match &o.kind {
        ObjKind::Group { children } => children.iter().collect(),
        _ => vec![o],
    };
    let flaeche = teile
        .into_iter()
        .filter(|c| matches!(c.style.fill, Fill::Farbe(_)))
        .filter_map(|c| match &c.kind {
            ObjKind::Polyline { pts, .. } => Some((c, pts.clone())),
            ObjKind::Path { subpaths, .. } => subpaths.first().map(|p| (c, p.clone())),
            _ => None,
        })
        .max_by(|a, b| {
            let fa = ops::bounds_doc(doc, &a.0.kind);
            let fb = ops::bounds_doc(doc, &b.0.kind);
            (fa.w() * fa.h()).total_cmp(&(fb.w() * fb.h()))
        })?;
    let pts = &flaeche.1;
    if pts.len() < 3 {
        return None;
    }
    let (mut lang, mut winkel) = (0.0, 0.0);
    for i in 0..pts.len() {
        let d = pts[(i + 1) % pts.len()] - pts[i];
        if d.len() > lang {
            lang = d.len();
            winkel = d.y.atan2(d.x).to_degrees();
        }
    }
    // nur quadratische Flächen (Fläche ≈ Kantenlänge²), nicht Kreise oder Dreiecke
    let inhalt = (0..pts.len()).map(|i| pts[i].x * pts[(i + 1) % pts.len()].y - pts[(i + 1) % pts.len()].x * pts[i].y).sum::<f32>().abs() / 2.0;
    let q = inhalt / (lang * lang).max(1e-6);
    if !(0.75..=1.15).contains(&q) {
        return None;
    }
    let a = (winkel + 45.0).rem_euclid(90.0) - 45.0;
    if a.abs() < 1.5 {
        return None;
    }
    Some((a, ops::bounds_doc(doc, &flaeche.0.kind).center()))
}

/// Passendes Normzeichen für eine Gruppe; schräg hängende Schilder werden zum Vergleich gerade gedreht.
fn zuordnen(doc: &Doc, o: &Obj, muster: &[Muster]) -> Option<Fund> {
    // liegt ein quadratischer Zeichengrund schräg, zuerst gerade gedreht vergleichen
    schraeg_zuordnen(doc, o, muster).or_else(|| zuordnen_gerade(doc, o, muster))
}

fn schraeg_zuordnen(doc: &Doc, o: &Obj, muster: &[Muster]) -> Option<Fund> {
    let (a, c) = schraeglage(doc, o)?;
    let mut gerade = o.clone();
    ops::rotate(&mut gerade.kind, c, -a);
    let mut f = zuordnen_gerade(doc, &gerade, muster)?;
    // schräg liegen nur rechteckige Rettungs-/Brandschutzzeichen oder Zeichen aus der Planlegende
    let sym = crate::symbols::find(f.id)?;
    let gelernt = muster.iter().any(|m| m.id == f.id);
    if f.id == "STANDORT" || !(gelernt || matches!(sym.cat, crate::symbols::Category::Rettung | crate::symbols::Category::Brandschutz)) {
        return None;
    }
    f.pos = f.pos.rotate_around(c, a);
    f.rot = (f.rot + a).rem_euclid(360.0);
    Some(f)
}

/// Passendes Normzeichen für eine Gruppe. Passt die ganze Gruppe nicht, wird sie ohne je eines
/// ihrer Teile verglichen (beim Import hängen oft angrenzende Linien oder Pfeilspitzen mit daran).
fn zuordnen_gerade(doc: &Doc, o: &Obj, muster: &[Muster]) -> Option<Fund> {
    let children: &[Obj] = match &o.kind {
        ObjKind::Group { children } => children,
        ObjKind::Polyline { closed: true, .. } | ObjKind::Path { .. } => &[],
        _ => return None,
    };
    if children.iter().any(|c| matches!(c.kind, ObjKind::Group { .. } | ObjKind::Symbol { .. })) {
        return None;
    }
    let mut best = vergleichen(doc, o, muster).map(|(id, rot, r, b)| (id, rot, r, b, None));
    if best.as_ref().is_none_or(|t| t.2 > 1.0) && (3..=40).contains(&children.len()) {
        for k in 0..children.len() {
            let mut teil = o.clone();
            if let ObjKind::Group { children } = &mut teil.kind {
                children.remove(k);
            }
            if let Some((id, rot, r, b)) = vergleichen(doc, &teil, muster) {
                if best.as_ref().is_none_or(|t| r < t.2) {
                    best = Some((id, rot, r, b, Some(k)));
                }
            }
        }
    }
    let (id, rot, r, b, rest) = best?;
    if r > 1.0 {
        return None;
    }
    // Größe und Lage des Symbolfelds aus dem Inhaltsrahmen der Vorlage zurückrechnen
    let v = vorlage(id, rot)?;
    let (bw, bh) = ((v.bb[2] - v.bb[0]) / 100.0, (v.bb[3] - v.bb[1]) / 100.0);
    let size = (b.w() / bw + b.h() / bh) / 2.0;
    let x0 = b.min.x - v.bb[0] / 100.0 * size;
    let y0 = b.min.y - v.bb[1] / 100.0 * size;
    Some(Fund { id, pos: Pos::new(x0 + size / 2.0, y0 + size / 2.0), size, rot, rest })
}

/// Gruppen (oberste Ebene, optional nur `ids`) durch erkannte Normzeichen ersetzen und die Legende
/// abgleichen. Liefert die Anzahl ersetzter Zeichen.
pub fn normzeichen_einsetzen(doc: &mut Doc, ids: Option<&[u64]>) -> usize {
    let muster = aus_legende_lernen(doc);
    let mut funde: Vec<(usize, Fund)> = doc
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| !o.locked && ids.is_none_or(|ids| ids.contains(&o.id)))
        .filter_map(|(i, o)| zuordnen(doc, o, &muster).map(|f| (i, f)))
        .collect();
    let n = funde.len();
    let layer = doc.find_layer_by_name("Symbole");
    // von hinten, damit eingefügte Restteile die Positionen davor nicht verschieben
    funde.reverse();
    for (i, f) in funde {
        let rest = match (&mut doc.objects[i].kind, f.rest) {
            (ObjKind::Group { children }, Some(k)) => Some(children.remove(k)),
            _ => None,
        };
        let o = &mut doc.objects[i];
        o.kind = ObjKind::Symbol { id: f.id.into(), pos: f.pos, size: (f.size * 10.0).round() / 10.0, rot: f.rot };
        o.style = Style::NONE;
        o.layer = layer;
        if let Some(mut r) = rest {
            r.id = doc.alloc_id();
            doc.objects.insert(i, r);
        }
    }
    let n = n + lose_teile_ersetzen(doc, ids, &muster);
    ops::sync_legend(doc);
    n
}

/// Zweiter Durchgang: kleine, nicht gruppierte Teile (z. B. Schilder, die auf einer Wand liegen und
/// deshalb beim Import nicht zusammengefasst wurden) probeweise bündeln und vergleichen.
fn lose_teile_ersetzen(doc: &mut Doc, ids: Option<&[u64]>, muster: &[Muster]) -> usize {
    let max = crate::pdfvec::SYMBOL_MAX_MM;
    let mut probe = doc.clone();
    // kleine, nicht erkannte Gruppen auflösen; Teil-ID -> ID des Objekts im Plan
    let mut herkunft: std::collections::HashMap<u64, u64> = Default::default();
    let mut flach = Vec::with_capacity(probe.objects.len());
    for o in std::mem::take(&mut probe.objects) {
        let klein = ops::bounds_doc(doc, &o.kind).w().max(ops::bounds_doc(doc, &o.kind).h()) <= max;
        let frei = !o.locked && ids.is_none_or(|ids| ids.contains(&o.id));
        match o.kind {
            ObjKind::Group { children } if klein && frei && children.iter().all(|c| !matches!(c.kind, ObjKind::Group { .. } | ObjKind::Symbol { .. })) => {
                for c in children {
                    herkunft.insert(c.id, o.id);
                    flach.push(c);
                }
            }
            kind => {
                if frei {
                    herkunft.insert(o.id, o.id);
                }
                flach.push(Obj { kind, ..o });
            }
        }
    }
    probe.objects = flach;
    // Ausgangspunkt: gefüllte, etwa quadratische Flächen in Schildgröße (Schildhintergrund);
    // dazu gehört alles, was vollständig darin liegt
    let lose: Vec<usize> = (0..probe.objects.len()).filter(|&i| herkunft.contains_key(&probe.objects[i].id) && !matches!(probe.objects[i].kind, ObjKind::Symbol { .. } | ObjKind::Group { .. })).collect();
    let rahmen: Vec<BBox> = lose.iter().map(|&i| ops::bounds_doc(&probe, &probe.objects[i].kind)).collect();
    let mut belegt = vec![false; lose.len()];
    let mut funde: Vec<(Vec<u64>, Fund)> = Vec::new();
    for (k, &i) in lose.iter().enumerate() {
        let o = &probe.objects[i];
        let b = rahmen[k];
        let gefuellt = matches!(o.style.fill, Fill::Farbe(c) if c.3 > 200 && !(c.0 > 230 && c.1 > 230 && c.2 > 230));
        if belegt[k] || !gefuellt || !(4.0..=max).contains(&b.w()) || (b.w() - b.h()).abs() > b.w() * 0.15 {
            continue;
        }
        let innen = b.expand(0.3);
        let teile: Vec<usize> = (0..lose.len()).filter(|&j| !belegt[j] && innen.contains(rahmen[j].min) && innen.contains(rahmen[j].max)).collect();
        if teile.len() < 2 {
            continue;
        }
        let children: Vec<Obj> = teile.iter().map(|&j| probe.objects[lose[j]].clone()).collect();
        let g = Obj { id: 0, layer: o.layer, kind: ObjKind::Group { children }, style: Style::default(), print: true, locked: false };
        #[cfg(test)]
        if std::env::var("FLUCHTPLAN_LOSE").is_ok() && zuordnen(&probe, &g, muster).is_none() {
            eprintln!("LOSE @({:.0},{:.0}) {:.1} best={:?}", b.min.x, b.min.y, b.w(), vergleichen(&probe, &g, muster).map(|t| (t.0, t.1, t.2)));
        }
        if let Some(f) = zuordnen(&probe, &g, muster) {
            let mut ids: Vec<u64> = teile.iter().enumerate().filter(|(n, _)| Some(*n) != f.rest).map(|(_, &j)| herkunft[&probe.objects[lose[j]].id]).collect();
            ids.sort_unstable();
            ids.dedup();
            for &j in &teile {
                belegt[j] = true;
            }
            funde.push((ids, f));
        }
    }
    let layer = doc.find_layer_by_name("Symbole");
    let mut n = 0;
    for (teile, f) in &funde {
        // Symbol an die Stelle des obersten Teils, die übrigen Teile entfernen
        let Some(i) = doc.objects.iter().rposition(|o| teile.contains(&o.id)) else { continue };
        let id = doc.objects[i].id;
        doc.objects[i] = Obj { id, layer, kind: ObjKind::Symbol { id: f.id.into(), pos: f.pos, size: (f.size * 10.0).round() / 10.0, rot: f.rot }, style: Style::NONE, print: true, locked: false };
        doc.objects.retain(|o| o.id == id || !teile.contains(&o.id));
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Musterplan als PDF ausgeben, als Vektorgrafik wieder einlesen: die Zeichen müssen erkannt werden.
    #[test]
    fn zeichen_aus_pdf_erkennen() {
        let orig = crate::samples::flucht_rettungsplan();
        let mut nur_plan = orig.clone();
        nur_plan.frame = FrameKind::Ohne;
        let pdf = export::pdf_bytes(&[&nur_plan], export::Stamp::Keiner).unwrap();
        let pg = crate::pdfvec::read_page_bytes(pdf, 0).unwrap();
        let mut doc = Doc { frame: FrameKind::Ohne, page: orig.page, ..Doc::default() };
        let opt = crate::pdfvec::PdfImportOpt { fit: crate::pdfvec::PdfFit::Massstab, ..Default::default() };
        crate::pdfvec::place(&mut doc, pg, opt);
        let mut soll: Vec<(String, Pos, f32)> = Vec::new();
        let mut ist: Vec<(String, Pos, f32)> = Vec::new();
        for (d, v) in [(&orig, &mut soll), (&doc, &mut ist)] {
            for o in &d.objects {
                if let ObjKind::Symbol { id, pos, rot, .. } = &o.kind {
                    v.push((id.clone(), *pos, *rot));
                }
            }
        }
        for (id, p, rot) in &soll {
            let ok = ist.iter().any(|(i, q, r)| i == id && (*q - *p).len() < 1.0 && (r - rot).abs() < 1.0);
            assert!(ok, "{id} bei {p:?} (Drehung {rot}) nicht erkannt; gefunden: {ist:?}");
        }
        assert_eq!(ist.len(), soll.len(), "falsch erkannte Zeichen: {ist:?}");
        // Legende enthält die erkannten Zeichen
        assert!(doc.legend.entries.iter().any(|e| e.key == "sym:F001"));
    }
}
#[cfg(test)]
mod muster {
    use super::*;

    fn zeichen(d: &Doc) -> Vec<(String, Pos)> {
        d.objects.iter().filter_map(|o| if let ObjKind::Symbol { id, pos, .. } = &o.kind { Some((id.clone(), *pos)) } else { None }).collect()
    }

    /// Alle Musterpläne über PDF zurückgewinnen: keine falschen Zeichen, fast alle erkannt.
    #[test]
    fn alle_musterplaene() {
        for (name, orig) in crate::samples::all() {
            let mut nur = orig.clone();
            let frame = orig.frame;
            nur.frame = FrameKind::Ohne;
            let pdf = export::pdf_bytes(&[&nur], export::Stamp::Keiner).unwrap();
            let pg = crate::pdfvec::read_page_bytes(pdf, 0).unwrap();
            // Planart vorab setzen (unterscheidet gleich aussehende Zeichen), Einfügen 1:1 an derselben Stelle
            let mut doc = Doc { frame, page: orig.page, ..Doc::default() };
            let opt = crate::pdfvec::PdfImportOpt { fit: crate::pdfvec::PdfFit::Massstab, symbols: false, ..Default::default() };
            let mut tmp = Doc { frame: FrameKind::Ohne, page: orig.page, ..Doc::default() };
            let ids = crate::pdfvec::place(&mut tmp, pg, opt);
            doc.objects = tmp.objects;
            doc.layers = tmp.layers;
            doc.next_id = tmp.next_id;
            ops::symbole_zusammenfassen(&mut doc, &ids, crate::pdfvec::SYMBOL_MAX_MM);
            normzeichen_einsetzen(&mut doc, None);
            let (soll, ist) = (zeichen(&orig), zeichen(&doc));
            let gleich = |a: &(String, Pos), b: &(String, Pos)| a.0 == b.0 && (a.1 - b.1).len() < 1.0;
            let falsch: Vec<_> = ist.iter().filter(|i| !soll.iter().any(|s| gleich(s, i))).collect();
            assert!(falsch.is_empty(), "{name}: falsch erkannt {falsch:?}");
            // bekannte Lücken: sehr kleine Zugangs-Dreiecke, mehrteilige Nordpfeile
            let fehlt: Vec<_> = soll.iter().filter(|s| !ist.iter().any(|i| gleich(s, i)) && !["FW_ZUGANG", "N1"].contains(&s.0.as_str())).collect();
            assert!(fehlt.is_empty(), "{name}: nicht erkannt {fehlt:?}");
        }
    }
}

#[cfg(test)]
mod diagnose {
    use super::*;
    /// FLUCHTPLAN_PDF=… FLUCHTPLAN_TESTBILD=Ordner cargo test diagnose_pdf -- --ignored --nocapture
    #[test]
    #[ignore = "Diagnose"]
    fn diagnose_pdf() {
        let (Ok(pdf), Ok(dir)) = (std::env::var("FLUCHTPLAN_PDF"), std::env::var("FLUCHTPLAN_TESTBILD")) else { return };
        let bytes = std::fs::read(&pdf).unwrap();
        let img = crate::import::pdf_page_image_bytes(bytes.clone(), 0, 60.0).unwrap();
        std::fs::write(format!("{dir}/seite.{}", if img.mime.contains("png") { "png" } else { "jpg" }), base64_decode(&img.b64)).unwrap();
        let pg = crate::pdfvec::read_page_bytes(bytes, 0).unwrap();
        eprintln!("Seite {}×{} mm, {} Teile", pg.w, pg.h, pg.items.len());
        let mut doc = Doc { frame: FrameKind::Ohne, ..Doc::default() };
        let opt = crate::pdfvec::PdfImportOpt { fit: crate::pdfvec::PdfFit::Formatfuellend, symbols: false, ..Default::default() };
        let ids = crate::pdfvec::place(&mut doc, pg, opt);
        let n = ops::symbole_zusammenfassen(&mut doc, &ids, crate::pdfvec::SYMBOL_MAX_MM);
        eprintln!("{n} Gruppen");
        if let Ok(f) = std::env::var("FLUCHTPLAN_FENSTER") {
            let v: Vec<f32> = f.split(',').map(|x| x.parse().unwrap()).collect();
            let fenster = BBox::from_two(Pos::new(v[0], v[1]), Pos::new(v[2], v[3]));
            for (i, o) in doc.objects.iter().enumerate() {
                let b = ops::bounds_doc(&doc, &o.kind);
                if b.intersects(&fenster) {
                    eprintln!("OBJ #{i} {} {:?} {:.1}×{:.1} @({:.1},{:.1}) fill={:?}", ops::kind_name(&o.kind), match &o.kind { ObjKind::Group { children } => children.len(), _ => 0 }, b.w(), b.h(), b.min.x, b.min.y, o.style.fill);
                }
            }
        }
        let mut kopie = doc.clone();
        eprintln!("ersetzt: {}", normzeichen_einsetzen(&mut kopie, None));
        for o in &kopie.objects {
            if let ObjKind::Symbol { id, pos, size, rot } = &o.kind {
                eprintln!("SYM {id} @({:.0},{:.0}) {size} {rot}", pos.x, pos.y);
            }
        }
        let muster = aus_legende_lernen(&doc);
        eprintln!("gelernt: {:?}", muster.iter().map(|m| (m.id, m.rot)).collect::<Vec<_>>());
        let mut k = 0;
        let mut sheet: Vec<(usize, Vec<[u8; 3]>, Vec<[u8; 3]>)> = Vec::new();
        for o in &doc.objects {
            if !matches!(o.kind, ObjKind::Group { .. }) { continue }
            let b = ops::bounds_doc(&doc, &o.kind);
            let z = zuordnen(&doc, o, &muster).map(|f| f.id);
            let v = vergleichen(&doc, o, &[]).map(|(id, _, r, _)| (id, (r * 1000.0).round() / 1000.0));
            eprintln!("G{k} @({:.0},{:.0}) {:.1}×{:.1} n={} erkannt={z:?} best={v:?}", b.min.x, b.min.y, b.w(), b.h(), match &o.kind { ObjKind::Group { children } => children.len(), _ => 0 });
            if let Some((px, _, _)) = gruppe_rastern(&doc, o, b) {
                let mut im = image::RgbImage::new(R as u32, R as u32);
                for y in 0..R { for x in 0..R { im.put_pixel(x as u32, y as u32, image::Rgb(px[y * R + x])); } }
                im.save(format!("{dir}/g{k}.png")).unwrap();
            }
            if let (Some((px, _, _)), Some((vid, vrot, _, _))) = (gruppe_rastern(&doc, o, b), zuordnen(&doc, o, &muster).map(|f| (f.id, f.rot, 0, 0)).or_else(|| vergleichen(&doc, o, &muster).map(|(i, r, _, _)| (i, r, 0, 0)))) {
                if b.w().max(b.h()) > 4.0 {
                    let sym = crate::symbols::find(vid).unwrap();
                    let tp = rastern(&format!("<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'><g transform='rotate({} 50 50)'>{}</g></svg>", vrot, sym.svg)).unwrap();
                    sheet.push((k, px, tp));
                }
            }
            k += 1;
        }
        // Bogen: je Zeile Gruppe | beste Vorlage, 4 Paare pro Zeile
        let cols = 4;
        let rows = sheet.len().div_ceil(cols);
        let mut im = image::RgbImage::from_pixel((cols * (2 * R + 12)) as u32, (rows * (R + 4)) as u32, image::Rgb([120, 120, 120]));
        for (n, (_, a, t)) in sheet.iter().enumerate() {
            let (ox, oy) = ((n % cols) * (2 * R + 12), (n / cols) * (R + 4));
            for y in 0..R { for x in 0..R {
                im.put_pixel((ox + x) as u32, (oy + y) as u32, image::Rgb(a[y * R + x]));
                im.put_pixel((ox + R + 2 + x) as u32, (oy + y) as u32, image::Rgb(t[y * R + x]));
            } }
        }
        im.save(format!("{dir}/bogen.png")).unwrap();
        eprintln!("Bogen: {:?}", sheet.iter().map(|s| s.0).collect::<Vec<_>>());
    }
    fn base64_decode(s: &str) -> Vec<u8> {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(s).unwrap()
    }
}

#[cfg(test)]
mod alle_plaene {
    use super::*;
    /// FLUCHTPLAN_PDFORDNER=… FLUCHTPLAN_TESTBILD=… cargo test alle_pdf_plaene -- --ignored --nocapture
    fn pg_w(d: &Doc) -> f32 {
        d.page.size().0
    }
    #[test]
    #[ignore = "Diagnose"]
    fn alle_pdf_plaene() {
        let (Ok(ordner), Ok(dir)) = (std::env::var("FLUCHTPLAN_PDFORDNER"), std::env::var("FLUCHTPLAN_TESTBILD")) else { return };
        let mut namen: Vec<_> = std::fs::read_dir(&ordner).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "pdf")).collect();
        namen.sort();
        for (k, p) in namen.iter().enumerate() {
            let t = std::time::Instant::now();
            let pg = crate::pdfvec::read_page_bytes(std::fs::read(p).unwrap(), 0).unwrap();
            let mut doc = Doc { frame: FrameKind::Ohne, ..Doc::default() };
            let opt = crate::pdfvec::PdfImportOpt { fit: crate::pdfvec::PdfFit::Formatfuellend, ..Default::default() };
            crate::pdfvec::place(&mut doc, pg, opt);
            let mut zahl: std::collections::BTreeMap<String, usize> = Default::default();
            for o in &doc.objects {
                if let ObjKind::Symbol { id, .. } = &o.kind {
                    *zahl.entry(id.clone()).or_default() += 1;
                }
            }
            eprintln!("PLAN {} ({:.1} s): {:?}", p.file_name().unwrap().to_string_lossy(), t.elapsed().as_secs_f32(), zahl);
            // Ausschnitte: verdächtige Treffer und übrig gebliebene farbige Schilder
            let seite = crate::import::pdf_page_image_bytes(std::fs::read(p).unwrap(), 0, 200.0).unwrap();
            let roh = { use base64::Engine; base64::engine::general_purpose::STANDARD.decode(&seite.b64).unwrap() };
            let bild = image::load_from_memory(&roh).unwrap().to_rgb8();
            let mm = bild.width() as f32 / pg_w(&doc);
            let mut stellen: Vec<(Pos, String)> = Vec::new();
            for o in &doc.objects {
                if let ObjKind::Symbol { id, pos, .. } = &o.kind {
                    if ["BMT_LINIE", "FW_NSHV", "E016", "E017", "F_LOESCHDECKE", "PFEIL_GRUEN_DIAG"].contains(&id.as_str()) {
                        stellen.push((*pos, id.clone()));
                    }
                } else {
                    let b = ops::bounds_doc(&doc, &o.kind);
                    let schild = |c: Color| (c.0 > 140 && c.1 < 80 && c.2 < 80) || (c.1 > 100 && c.0 < 60 && c.2 < 120);
                    let farbig = match (&o.kind, &o.style.fill) {
                        (ObjKind::Group { children }, _) => children.iter().any(|c| matches!(c.style.fill, Fill::Farbe(f) if schild(f)) && { let cb = ops::bounds_doc(&doc, &c.kind); cb.w() > 4.0 && (cb.w() - cb.h()).abs() < 1.0 }),
                        (_, Fill::Farbe(f)) => schild(*f),
                        _ => false,
                    };
                    if farbig && (4.0..=14.0).contains(&b.w()) && (b.w() - b.h()).abs() < 1.5 {
                        stellen.push((b.center(), "?".into()));
                    }
                }
            }
            let z = 110u32;
            let cols = 10usize;
            if !stellen.is_empty() {
                let rows = stellen.len().div_ceil(cols) as u32;
                let mut bogen = image::RgbImage::from_pixel(cols as u32 * z, rows * z, image::Rgb([90, 90, 90]));
                for (n, (pos, id)) in stellen.iter().enumerate() {
                    let (cx, cy) = ((pos.x * mm) as i64, (pos.y * mm) as i64);
                    for y in 0..z - 14 { for x in 0..z - 4 {
                        let (sx, sy) = (cx - 53 + x as i64, cy - 48 + y as i64);
                        if sx >= 0 && sy >= 0 && (sx as u32) < bild.width() && (sy as u32) < bild.height() {
                            bogen.put_pixel((n % cols) as u32 * z + x, (n / cols) as u32 * z + y, *bild.get_pixel(sx as u32, sy as u32));
                        }
                    } }
                    eprintln!("  #{n}: {id} @({:.0},{:.0})", pos.x, pos.y);
                }
                bogen.save(format!("{dir}/stellen_{k}.png")).unwrap();
            }
            if std::env::var("FLUCHTPLAN_DPI").is_ok() && k % 6 == 0 {
                std::fs::write(format!("{dir}/plan_{k}.png"), crate::export::render_png(&doc, std::env::var("FLUCHTPLAN_DPI").ok().and_then(|d| d.parse().ok()).unwrap_or(60.0)).unwrap()).unwrap();
            }
        }
    }
}
