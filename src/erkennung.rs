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
    ("anleiter", &["FW_ANLEITER"]),
    ("feuerleiter", &["F003"]),
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
        // Feuerwehrzeichen (DIN 14034) werden sehr unterschiedlich gezeichnet; steht nur eines zur
        // Wahl, belegt der Legendentext die Zuordnung (z. B. Anleiterstelle als Leiter statt Balken)
        let feuerwehr = ids.len() == 1 && symbols::find(id).is_some_and(|s| s.cat == symbols::Category::Feuerwehr);
        if d > if feuerwehr { 0.45 } else { 0.3 } {
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
            let legende: Vec<String> = doc.objects.iter().filter_map(|o| match &o.kind {
                ObjKind::Legend { a, b, cols, size } => Some(format!("({:.0},{:.0})-({:.0},{:.0}) {cols} Sp. {size:.1} mm", a.x, a.y, b.x, b.y)),
                _ => None,
            }).collect();
            eprintln!("  LEGENDE {:?} Einträge {:?}", legende, doc.legend.entries.iter().map(|e| e.label.as_str()).collect::<Vec<_>>());
            let gruppen: Vec<String> = doc.objects.iter().filter(|o| matches!(o.kind, ObjKind::Group { .. })).filter_map(|o| doc.layers.iter().find(|l| l.id == o.layer).filter(|l| BEREICH_EBENEN.contains(&l.name.as_str())).map(|l| { let b = ops::bounds_doc(&doc, &o.kind); format!("{} ({:.0},{:.0} {:.0}×{:.0})", l.name, b.min.x, b.min.y, b.w(), b.h()) })).collect();
            let i = &doc.info;
            eprintln!("  BEREICHE {gruppen:?}\n  INFO objekt={:?} gebaeude={:?} geschoss={:?} stand={:?} plannr={:?}", i.objekt, i.gebaeude, i.geschoss, i.stand, i.plannummer);
            if std::env::var("FLUCHTPLAN_DPI").is_ok() {
                let mut g = doc.clone();
                let n = nur_grundriss(&mut g);
                eprintln!("  NUR_GRUNDRISS {n} entfernt");
                std::fs::write(format!("{dir}/grundriss_{k}.png"), crate::export::render_png(&g, 50.0).unwrap()).unwrap();
            }
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
            if std::env::var("FLUCHTPLAN_DPI").is_ok() {
                std::fs::write(format!("{dir}/plan_{k}.png"), crate::export::render_png(&doc, std::env::var("FLUCHTPLAN_DPI").ok().and_then(|d| d.parse().ok()).unwrap_or(60.0)).unwrap()).unwrap();
            }
        }
    }
}

/// Gezeichnete Legende eines importierten Plans durch eine echte Legende ersetzen, die sich mit dem
/// Plan aktualisiert. Erkannt wird sie an Normzeichen mit kurzer Beschriftung rechts daneben, die dicht
/// beieinander stehen; ein umgebender Rahmen samt Überschrift wird mit ersetzt. Liefert den Bereich.
pub fn legende_ersetzen(doc: &mut Doc, ids: &[u64]) -> Option<BBox> {
    fn texte_sammeln(doc: &Doc, o: &Obj, out: &mut Vec<(BBox, String)>) {
        match &o.kind {
            ObjKind::Text { text, .. } if !text.trim().is_empty() => out.push((ops::bounds_doc(doc, &o.kind), text.clone())),
            ObjKind::Group { children } => children.iter().for_each(|c| texte_sammeln(doc, c, out)),
            _ => {}
        }
    }
    let neu = |o: &Obj| ids.contains(&o.id) && !o.locked;
    let mut texte = Vec::new();
    for o in doc.objects.iter().filter(|o| neu(o)) {
        texte_sammeln(doc, o, &mut texte);
    }
    // Einträge: eingesetztes Normzeichen + passender Text rechts daneben (Texte merken)
    let mut eintraege: Vec<(BBox, Vec<usize>)> = Vec::new();
    for o in doc.objects.iter().filter(|o| neu(o)) {
        let ObjKind::Symbol { id: sym_id, .. } = &o.kind else { continue };
        let b = ops::bounds_doc(doc, &o.kind);
        let h = b.w().max(b.h());
        let neben: Vec<usize> = (0..texte.len())
            .filter(|&i| {
                let t = texte[i].0;
                t.min.x >= b.max.x - 0.5 && t.min.x <= b.max.x + 2.5 * h && t.center().y >= b.min.y - 0.25 * h && t.center().y <= b.max.y + 0.25 * h
            })
            .collect();
        let text: String = neben.iter().map(|&i| texte[i].1.as_str()).collect::<Vec<_>>().join(" ");
        // der Text muss das Zeichen beschreiben (nicht nur irgendein Begriff daneben)
        if neben.is_empty() || begriff_norm(&text).chars().count() > 50 || !ids_fuer_text(&text).contains(&sym_id.as_str()) {
            continue;
        }
        eintraege.push((neben.iter().fold(b, |acc, &i| acc.union(texte[i].0)), neben));
    }
    if eintraege.len() < 3 {
        return None;
    }
    let ist_titel = |s: &str| begriff_norm(s).starts_with("legende");
    // Bewertung eines Bereichs: enthaltene Einträge und fremde Texte (Verhaltensregeln o. Ä.)
    let bewerten = |r: &BBox| {
        let innen: Vec<usize> = (0..eintraege.len()).filter(|&i| eintraege[i].0.inside(&r.expand(0.5))).collect();
        let eigen: Vec<usize> = innen.iter().flat_map(|&i| eintraege[i].1.iter().copied()).collect();
        let fremd = (0..texte.len()).filter(|&i| r.contains(texte[i].0.center()) && !eigen.contains(&i) && !ist_titel(&texte[i].1)).count();
        (innen.len(), fremd)
    };
    let legende_artig = |(n, fremd): (usize, usize)| n >= 3 && fremd <= n.max(3);
    // 1. gerahmte Legende: größter Rahmen, der fast nur Legendeneinträge enthält
    let flaeche = |b: &BBox| b.w() * b.h();
    let rechteckig = |k: &ObjKind| match k {
        ObjKind::Rect { .. } => true,
        ObjKind::Polyline { pts, closed } => (*closed && (4..=5).contains(&pts.len())) || (pts.len() == 5 && pts[0].dist(pts[4]) < 0.5),
        ObjKind::Path { subpaths, .. } => subpaths.len() == 1 && (4..=5).contains(&subpaths[0].len()),
        _ => false,
    };
    let rahmen = doc
        .objects
        .iter()
        .filter(|o| neu(o) && rechteckig(&o.kind))
        .map(|o| ops::bounds_doc(doc, &o.kind))
        .filter(|r| r.w() >= 20.0 && r.h() >= 15.0)
        .filter(|r| legende_artig(bewerten(r)))
        .max_by(|a, b| flaeche(a).total_cmp(&flaeche(b)));
    let bereich = match rahmen {
        Some(r) => r,
        None => {
            // 2. ohne Rahmen: Einträge, die in einer Spalte oder Zeile dicht untereinander stehen
            let n = eintraege.len();
            let mut gruppe: Vec<usize> = (0..n).collect();
            fn wurzel(g: &mut [usize], i: usize) -> usize {
                let mut i = i;
                while g[i] != i {
                    g[i] = g[g[i]];
                    i = g[i];
                }
                i
            }
            for i in 0..n {
                for j in i + 1..n {
                    let (a, b) = (eintraege[i].0, eintraege[j].0);
                    let h = (a.h() + b.h()) / 2.0;
                    let spalte = (a.min.x - b.min.x).abs() < 1.5 && (a.min.y.max(b.min.y) - a.max.y.min(b.max.y)) < 1.5 * h;
                    let zeile = (a.center().y - b.center().y).abs() < 0.5 * h && (a.min.x.max(b.min.x) - a.max.x.min(b.max.x)) < 8.0 * h;
                    if spalte || zeile {
                        let (x, y) = (wurzel(&mut gruppe, i), wurzel(&mut gruppe, j));
                        gruppe[x] = y;
                    }
                }
            }
            let mut kandidaten: std::collections::HashMap<usize, BBox> = std::collections::HashMap::new();
            for i in 0..n {
                let w = wurzel(&mut gruppe, i);
                let e = eintraege[i].0;
                kandidaten.entry(w).and_modify(|b| *b = b.union(e)).or_insert(e);
            }
            let mut b = kandidaten.into_values().filter(|b| legende_artig(bewerten(b))).max_by_key(|b| bewerten(b).0)?;
            // Überschrift „Legende“ direkt darüber mitnehmen
            let h = 8.0f32.min(b.h());
            if let Some((t, _)) = texte.iter().find(|(t, s)| ist_titel(s) && t.max.y <= b.min.y + 0.5 && t.max.y >= b.min.y - 2.0 * h && t.max.x >= b.min.x && t.min.x <= b.max.x) {
                b = b.union(*t);
            }
            b.expand(1.0)
        }
    };
    // Rahmen aus einzelnen Linien (oder ein innerer Kasten wurde gewählt): senkrechte Kanten links
    // und rechts, die über den Bereich hinausreichen, bestimmen die volle Höhe
    let senkrecht = |o: &Obj| -> Option<(f32, f32, f32)> {
        let b = ops::bounds_doc(doc, &o.kind);
        (neu(o) && b.w() < 0.8 && b.h() > 5.0).then_some((b.center().x, b.min.y, b.max.y))
    };
    let mut kanten: Vec<(f32, f32, f32)> = doc.objects.iter().filter_map(senkrecht).collect();
    // auch die Seiten von Rechtecken, die den Bereich umschließen (nicht von Nachbarkästen)
    for o in doc.objects.iter().filter(|o| neu(o) && rechteckig(&o.kind)) {
        let b = ops::bounds_doc(doc, &o.kind);
        if !bereich.inside(&b.expand(0.5)) {
            continue;
        }
        kanten.push((b.min.x, b.min.y, b.max.y));
        kanten.push((b.max.x, b.min.y, b.max.y));
    }
    let mut bereich = bereich;
    // Kanten bündig oder bis 4 mm außerhalb (äußerer Rahmen um einen inneren Kasten)
    // (keine Kanten, die viel länger sind, z. B. der Blattrand)
    let ueberdeckt = |y0: f32, y1: f32| y0 <= bereich.max.y - 1.0 && y1 >= bereich.min.y + 1.0 && y1 - y0 <= bereich.h() * 2.5;
    let links: Vec<&(f32, f32, f32)> = kanten.iter().filter(|(x, y0, y1)| *x <= bereich.min.x + 1.5 && *x >= bereich.min.x - 4.0 && ueberdeckt(*y0, *y1)).collect();
    let rechts: Vec<&(f32, f32, f32)> = kanten.iter().filter(|(x, y0, y1)| *x >= bereich.max.x - 1.5 && *x <= bereich.max.x + 4.0 && ueberdeckt(*y0, *y1)).collect();
    if !links.is_empty() && !rechts.is_empty() {
        let x0 = links.iter().map(|k| k.0).fold(f32::MAX, f32::min).min(bereich.min.x);
        let x1 = rechts.iter().map(|k| k.0).fold(f32::MIN, f32::max).max(bereich.max.x);
        let oben = links.iter().map(|k| k.1).fold(f32::MAX, f32::min).max(rechts.iter().map(|k| k.1).fold(f32::MAX, f32::min));
        let unten = links.iter().map(|k| k.2).fold(f32::MIN, f32::max).min(rechts.iter().map(|k| k.2).fold(f32::MIN, f32::max));
        // höchstens gut doppelt so hoch, sonst ist es ein anderer Kasten (z. B. Übersichtsplan)
        let neu_b = BBox::from_two(Pos::new(x0, oben.min(bereich.min.y)), Pos::new(x1, unten.max(bereich.max.y)));
        if neu_b.h() <= bereich.h() * 2.2 && legende_artig(bewerten(&neu_b)) {
            bereich = neu_b;
        }
    }
    // Überschriftsbalken direkt darüber (gleich breit, mit „Legende“ oder leer)
    let balken = doc
        .objects
        .iter()
        .filter(|o| neu(o) && rechteckig(&o.kind))
        .map(|o| ops::bounds_doc(doc, &o.kind))
        .find(|r| (r.min.x - bereich.min.x).abs() < 1.5 && (r.max.x - bereich.max.x).abs() < 1.5 && (r.max.y - bereich.min.y).abs() < 1.5 && r.h() <= 15.0 && {
            let t: Vec<&(BBox, String)> = texte.iter().filter(|(t, _)| r.contains(t.center())).collect();
            t.iter().all(|(_, s)| ist_titel(s))
        });
    if let Some(r) = balken {
        bereich = bereich.union(r);
    }
    // alte Legende entfernen und echte Legende einsetzen
    let innen = bereich.expand(1.5);
    // Inhalt benachbarter Kästen, die in den Bereich ragen (Schriftfeld, Verhaltensregeln …), bleibt
    let nachbarn: Vec<BBox> = doc
        .objects
        .iter()
        .filter(|o| neu(o) && rechteckig(&o.kind))
        .map(|o| ops::bounds_doc(doc, &o.kind))
        .filter(|r| r.intersects(&bereich) && !r.inside(&innen) && !bereich.inside(&r.expand(0.5)))
        .collect();
    let weg: Vec<u64> = doc
        .objects
        .iter()
        .filter(|o| neu(o))
        .filter(|o| {
            let b = ops::bounds_doc(doc, &o.kind);
            b.inside(&innen) && !nachbarn.iter().any(|n| b.inside(&n.expand(0.3)))
        })
        .map(|o| o.id)
        .collect();
    // neue Legende an die Stelle der alten in der Zeichenreihenfolge (was darüber lag, bleibt darüber)
    let stelle = doc.objects.iter().position(|o| weg.contains(&o.id)).unwrap_or(doc.objects.len());
    doc.objects.retain(|o| !weg.contains(&o.id));
    // Einträge, die nur in der alten Legende vorkamen, fallen weg
    let benutzt: Vec<String> = ops::used_legend_keys(doc).into_iter().map(|(k, _)| k).collect();
    doc.legend.entries.retain(|e| benutzt.contains(&e.key));
    ops::sync_legend(doc);
    // die neue Legende darf benachbarte Kästen nicht überdecken: dort abschneiden, wo am meisten bleibt
    for n in &nachbarn {
        if !n.intersects(&bereich) {
            continue;
        }
        let mut wahl = Vec::new();
        for (a, b) in [
            (bereich.min, Pos::new(bereich.max.x, n.min.y)),
            (Pos::new(bereich.min.x, n.max.y), bereich.max),
            (bereich.min, Pos::new(n.min.x, bereich.max.y)),
            (Pos::new(n.max.x, bereich.min.y), bereich.max),
        ] {
            if b.x - a.x > 10.0 && b.y - a.y > 10.0 {
                wahl.push(BBox::from_two(a, b));
            }
        }
        if let Some(b) = wahl.into_iter().max_by(|a, b| (a.w() * a.h()).total_cmp(&(b.w() * b.h()))) {
            bereich = b;
        }
    }
    let (cols, size) = crate::render::legend_fit(doc, bereich, (bereich.w() / 60.0).clamp(0.5, 1.2));
    let layer = doc.find_layer_by_name("Legende");
    let id = doc.alloc_id();
    let stelle = stelle.min(doc.objects.len());
    doc.objects.insert(stelle, Obj {
        id,
        layer,
        kind: ObjKind::Legend { a: bereich.min, b: bereich.max, cols, size },
        style: Style { stroke: Some(StrokeStyle::new(farbe::SCHWARZ, 0.3)), fill: Fill::Farbe(farbe::WEISS) },
        print: true,
        locked: false,
    });
    Some(bereich)
}

#[cfg(test)]
mod normvergleich {
    use super::*;

    fn xml(s: &str) -> String {
        s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    /// Fremdes SVG als verschachteltes <svg> an Position x/y einsetzen (IDs eindeutig gemacht).
    fn einbetten(src: &str, praefix: &str, x: f32, y: f32, groesse: f32) -> String {
        let Some(start) = src.find("<svg") else { return String::new() };
        let ende = src[start..].find('>').map(|e| start + e).unwrap_or(start);
        let kopf = &src[start..=ende];
        // Attribute des Wurzelelements: name="wert" bzw. name='wert'
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut rest = kopf.trim_start_matches("<svg").trim_end_matches('>').trim_end_matches('/');
        while let Some(gl) = rest.find('=') {
            let name = rest[..gl].trim().to_string();
            let r = rest[gl + 1..].trim_start();
            let Some(q) = r.chars().next().filter(|c| *c == '"' || *c == '\'') else { break };
            let Some(e) = r[1..].find(q) else { break };
            attrs.push((name, r[1..1 + e].to_string()));
            rest = &r[1 + e + 1..];
        }
        let attr = |name: &str| attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        // ohne viewBox: Breite/Höhe (ohne Einheit) als Ausschnitt
        let zahl = |v: Option<String>| v.and_then(|v| v.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse::<f32>().ok());
        let viewbox = attr("viewBox").unwrap_or_else(|| format!("0 0 {} {}", zahl(attr("width")).unwrap_or(100.0), zahl(attr("height")).unwrap_or(100.0)));
        let rumpf = src[ende + 1..].rsplit_once("</svg>").map(|r| r.0).unwrap_or("");
        let rumpf = rumpf
            .replace("id=\"", &format!("id=\"{praefix}"))
            .replace("id='", &format!("id='{praefix}"))
            .replace("url(#", &format!("url(#{praefix}"))
            .replace("href=\"#", &format!("href=\"#{praefix}"))
            .replace("href='#", &format!("href='#{praefix}"));
        // Namensräume behalten, Darstellungsattribute (fill-rule, style …) auf eine Gruppe übertragen
        let mut ns = String::new();
        let mut darst = String::new();
        for (n, v) in &attrs {
            let v = xml(v).replace('"', "&quot;");
            if n.starts_with("xmlns:") {
                ns += &format!(" {n}=\"{v}\"");
            } else if !matches!(n.as_str(), "xmlns" | "width" | "height" | "x" | "y" | "viewBox" | "version" | "id" | "preserveAspectRatio") && !n.contains(':') {
                darst += &format!(" {n}=\"{v}\"");
            }
        }
        format!("<svg{ns} x='{x}' y='{y}' width='{groesse}' height='{groesse}' viewBox='{viewbox}' preserveAspectRatio='xMidYMid meet'><g{darst}>{rumpf}</g></svg>")
    }

    /// Gegenüberstellung unserer Zeichen mit Referenz-SVGs (z. B. ISO 7010 von Wikimedia Commons) als PDF.
    /// FLUCHTPLAN_NORMORDNER=Ordner mit E001.svg … FLUCHTPLAN_NORMPDF=ziel.pdf cargo test normvergleich_pdf -- --ignored
    #[test]
    #[ignore]
    fn normvergleich_pdf() {
        use pdf_writer::{Content, Finish, Name, Pdf, Rect, Ref};
        let (Ok(ordner), Ok(ziel)) = (std::env::var("FLUCHTPLAN_NORMORDNER"), std::env::var("FLUCHTPLAN_NORMPDF")) else { return };
        let ist_iso = |id: &str| id.len() == 4 && "EFWMP".contains(&id[..1]) && id[1..].chars().all(|c| c.is_ascii_digit());
        let mut zeichen: Vec<_> = crate::symbols::library().iter().filter(|s| ist_iso(s.id)).collect();
        let reihe = |id: &str| "EFWMP".find(&id[..1]).unwrap_or(9);
        zeichen.sort_by_key(|s| (reihe(s.id), s.id));

        let eigen_svg = |svg: &str| format!("<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' viewBox='0 0 100 100'>{svg}</svg>");
        // Abweichung wie bei der Symbolerkennung (mittlere Farbdifferenz, 0 = gleich)
        let bild = |svg: &str| rastern(svg).and_then(|p| normieren(&p)).map(|b| b.0);

        let (w, h) = (210.0f32, 297.0f32);
        let (zeile_h, kopf, pro_seite) = (31.0f32, 30.0f32, 16usize);
        let mut seiten = Vec::new();
        let mut abweichungen = Vec::new();
        let anzahl_seiten = zeichen.len().div_ceil(pro_seite);
        for (si, teil) in zeichen.chunks(pro_seite).enumerate() {
            let mut s = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' xmlns:xlink='http://www.w3.org/1999/xlink' width='{w}mm' height='{h}mm' viewBox='0 0 {w} {h}' font-family='Arial'>\
                 <rect width='{w}' height='{h}' fill='white'/>\
                 <text x='12' y='15' font-size='6' font-weight='bold'>OpenFluchtplan – Vergleich mit ISO 7010</text>\
                 <text x='12' y='21' font-size='3'>Links: unser Zeichen · Rechts: Referenz ISO 7010 (Wikimedia Commons) · Abweichung 0 = identisch, ab 0,10 rot markiert</text>\
                 <text x='{}' y='{}' font-size='3' text-anchor='end'>Seite {} / {anzahl_seiten}</text>",
                w - 12.0,
                h - 8.0,
                si + 1
            );
            for (i, sym) in teil.iter().enumerate() {
                let (sp, zl) = (i % 2, i / 2);
                let (x, y) = (12.0 + sp as f32 * 95.0, kopf + zl as f32 * zeile_h);
                let eigen = eigen_svg(&sym.svg);
                let referenz = std::fs::read_to_string(format!("{ordner}/{}.svg", sym.id)).ok();
                let d = referenz.as_deref().and_then(|r| Some(abstand(&bild(&eigen)?, &bild(r)?)));
                let farbe = match d {
                    Some(v) if v >= 0.10 => "#b01e23",
                    Some(_) => "#237f52",
                    None => "#666666",
                };
                s += &format!("<rect x='{x}' y='{y}' width='91' height='{}' fill='none' stroke='#cccccc' stroke-width='0.2'/>", zeile_h - 2.0);
                s += &format!("<text x='{}' y='{}' font-size='4' font-weight='bold'>{}</text>", x + 2.0, y + 6.0, sym.id);
                // Name auf bis zu drei Zeilen umbrechen
                let mut zeilen = vec![String::new()];
                for wort in sym.name.split_whitespace() {
                    if zeilen.last().unwrap().len() + wort.len() > 18 && !zeilen.last().unwrap().is_empty() {
                        zeilen.push(String::new());
                    }
                    let z = zeilen.last_mut().unwrap();
                    if !z.is_empty() {
                        z.push(' ');
                    }
                    z.push_str(wort);
                }
                for (k, z) in zeilen.iter().take(3).enumerate() {
                    s += &format!("<text x='{}' y='{}' font-size='2.8'>{}</text>", x + 2.0, y + 11.0 + k as f32 * 3.4, xml(z));
                }
                let text_d = d.map(|v| format!("Abweichung {:.2}", v).replace('.', ",")).unwrap_or_else(|| "keine Referenz".into());
                s += &format!("<text x='{}' y='{}' font-size='2.8' fill='{farbe}' font-weight='bold'>{text_d}</text>", x + 2.0, y + zeile_h - 5.0);
                s += &einbetten(&eigen, &format!("u{}_", sym.id), x + 36.0, y + 2.0, 24.0);
                if let Some(r) = &referenz {
                    s += &einbetten(r, &format!("r{}_", sym.id), x + 64.0, y + 2.0, 24.0);
                }
                s += &format!("<text x='{}' y='{}' font-size='2.2' fill='#666' text-anchor='middle'>unser</text>", x + 48.0, y + zeile_h - 2.6);
                s += &format!("<text x='{}' y='{}' font-size='2.2' fill='#666' text-anchor='middle'>ISO 7010</text>", x + 76.0, y + zeile_h - 2.6);
                abweichungen.push((sym.id, d));
            }
            s += "</svg>";
            seiten.push(s);
        }

        let mut alloc = Ref::new(1);
        let (catalog, tree_id) = (alloc.bump(), alloc.bump());
        let mut pdf = Pdf::new();
        let mut ids = Vec::new();
        let mut chunks = Vec::new();
        let (wp, hp) = (w / 25.4 * 72.0, h / 25.4 * 72.0);
        for s in &seiten {
            let t = svg2pdf::usvg::Tree::from_str(s, crate::export::usvg_options()).unwrap();
            let (chunk, svg_ref) = svg2pdf::to_chunk(&t, svg2pdf::ConversionOptions::default()).unwrap();
            let mut map = std::collections::HashMap::new();
            let chunk = chunk.renumber(|old| *map.entry(old).or_insert_with(|| alloc.bump()));
            ids.push((alloc.bump(), alloc.bump(), map[&svg_ref]));
            chunks.push(chunk);
        }
        pdf.catalog(catalog).pages(tree_id);
        pdf.pages(tree_id).kids(ids.iter().map(|p| p.0)).count(ids.len() as i32);
        for (page_id, content_id, svg_id) in &ids {
            let mut page = pdf.page(*page_id);
            page.media_box(Rect::new(0.0, 0.0, wp, hp));
            page.parent(tree_id);
            page.contents(*content_id);
            page.resources().x_objects().pair(Name(b"P1"), *svg_id);
            page.finish();
            let mut c = Content::new();
            c.transform([wp, 0.0, 0.0, hp, 0.0, 0.0]).x_object(Name(b"P1"));
            pdf.stream(*content_id, &c.finish());
        }
        for c in &chunks {
            pdf.extend(c);
        }
        std::fs::write(&ziel, pdf.finish()).unwrap();
        if std::env::var("FLUCHTPLAN_NORMSVG").is_ok() {
            for (i, s) in seiten.iter().enumerate() {
                std::fs::write(format!("{ziel}.seite{}.svg", i + 1), s).unwrap();
            }
        }
        for (id, d) in abweichungen {
            eprintln!("VERGLEICH {id} {}", d.map(|v| format!("{v:.3}")).unwrap_or("-".into()));
        }
    }
}

#[cfg(test)]
mod legende_tests {
    use super::*;

    fn obj(doc: &mut Doc, kind: ObjKind, style: Style) -> u64 {
        let id = doc.alloc_id();
        doc.objects.push(Obj { id, layer: 0, kind, style, print: true, locked: false });
        id
    }
    fn text(doc: &mut Doc, x: f32, y: f32, t: &str) -> u64 {
        obj(doc, ObjKind::Text { pos: Pos::new(x, y), text: t.into(), size: 2.5, bold: false, italic: false, align: TAlign::Links, rot: 0.0 }, Style::default())
    }
    fn rahmen(doc: &mut Doc, x0: f32, y0: f32, x1: f32, y1: f32) -> u64 {
        obj(doc, ObjKind::Rect { a: Pos::new(x0, y0), b: Pos::new(x1, y1), radius: 0.0 }, Style::line(farbe::SCHWARZ, 0.3))
    }

    /// Gerahmte Legende wird ersetzt, der Kasten mit Verhaltensregeln daneben bleibt unberührt.
    #[test]
    fn gezeichnete_legende_ersetzen() {
        let mut doc = Doc::default();
        // Zeichen im Plan
        obj(&mut doc, ObjKind::Symbol { id: "F001".into(), pos: Pos::new(100.0, 100.0), size: 8.0, rot: 0.0 }, Style::NONE);
        obj(&mut doc, ObjKind::Symbol { id: "STANDORT".into(), pos: Pos::new(120.0, 100.0), size: 8.0, rot: 0.0 }, Style::NONE);
        // Legende: Rahmen, Überschrift, Zeichen mit Text
        rahmen(&mut doc, 300.0, 140.0, 380.0, 200.0);
        text(&mut doc, 330.0, 143.0, "Legende");
        for (k, (id, t)) in [("F001", "Feuerlöscher"), ("STANDORT", "Standort"), ("E003", "Erste Hilfe"), ("E007", "Sammelstelle")].iter().enumerate() {
            let y = 155.0 + k as f32 * 10.0;
            obj(&mut doc, ObjKind::Symbol { id: (*id).into(), pos: Pos::new(306.0, y), size: 8.0, rot: 0.0 }, Style::NONE);
            text(&mut doc, 312.0, y - 1.0, t);
        }
        // Verhaltensregeln: Zeichen mit Text, aber viele weitere Texte
        let regeln = rahmen(&mut doc, 200.0, 140.0, 290.0, 200.0);
        obj(&mut doc, ObjKind::Symbol { id: "F001".into(), pos: Pos::new(206.0, 150.0), size: 6.0, rot: 0.0 }, Style::NONE);
        text(&mut doc, 211.0, 149.0, "Feuerlöscher benutzen");
        obj(&mut doc, ObjKind::Symbol { id: "E003".into(), pos: Pos::new(206.0, 160.0), size: 6.0, rot: 0.0 }, Style::NONE);
        text(&mut doc, 211.0, 159.0, "Erste Hilfe leisten");
        obj(&mut doc, ObjKind::Symbol { id: "E004".into(), pos: Pos::new(206.0, 170.0), size: 6.0, rot: 0.0 }, Style::NONE);
        text(&mut doc, 211.0, 169.0, "Telefon 112");
        for (k, t) in ["Ruhe bewahren", "Wer meldet?", "Was ist passiert?", "Wie viele Verletzte?", "Warten auf Rückfragen!"].iter().enumerate() {
            text(&mut doc, 230.0, 175.0 + k as f32 * 4.0, t);
        }
        let vorher_regeln = doc.objects.iter().filter(|o| ops::bounds_doc(&doc, &o.kind).inside(&BBox::from_two(Pos::new(199.0, 139.0), Pos::new(291.0, 201.0)))).count();
        let ids: Vec<u64> = doc.objects.iter().map(|o| o.id).collect();
        ops::sync_legend(&mut doc);

        let b = legende_ersetzen(&mut doc, &ids).expect("Legende nicht erkannt");
        assert!((b.min.x - 300.0).abs() < 0.5 && (b.max.y - 200.0).abs() < 0.5, "falscher Bereich {b:?}");
        let legenden: Vec<&Obj> = doc.objects.iter().filter(|o| matches!(o.kind, ObjKind::Legend { .. })).collect();
        assert_eq!(legenden.len(), 1);
        // alte Legendentexte sind weg, Regeln unverändert
        assert!(!doc.objects.iter().any(|o| matches!(&o.kind, ObjKind::Text { text, .. } if text == "Sammelstelle")));
        assert!(doc.objects.iter().any(|o| o.id == regeln));
        let nachher_regeln = doc.objects.iter().filter(|o| ops::bounds_doc(&doc, &o.kind).inside(&BBox::from_two(Pos::new(199.0, 139.0), Pos::new(291.0, 201.0)))).count();
        assert_eq!(vorher_regeln, nachher_regeln);
        // Sammelstelle kam nur in der alten Legende vor
        let keys: Vec<&str> = doc.legend.entries.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"sym:F001") && keys.contains(&"sym:STANDORT") && !keys.contains(&"sym:E007"), "{keys:?}");
    }
}

/// Ebenen, auf die der Import erkannte Planbereiche legt (je Bereich eine Gruppe).
pub const BEREICH_EBENEN: [&str; 5] = ["Kopfzeile", "Verhaltensregeln", "Übersichtsplan", "Schriftfeld", "Prüfplakette"];

fn ist_rechteckig(k: &ObjKind) -> bool {
    match k {
        ObjKind::Rect { .. } => true,
        ObjKind::Polyline { pts, closed } => (*closed && (4..=5).contains(&pts.len())) || (pts.len() == 5 && pts[0].dist(pts[4]) < 0.5),
        ObjKind::Path { subpaths, .. } => subpaths.len() == 1 && (4..=5).contains(&subpaths[0].len()),
        _ => false,
    }
}

/// Alle Objekte (aus `ids`, oberste Ebene) innerhalb `bb` zu einer Gruppe auf der Ebene `ebene` zusammenfassen.
fn gruppieren(doc: &mut Doc, ids: &[u64], bb: BBox, ebene: &str) -> Option<u64> {
    let innen = bb.expand(0.5);
    let teil: Vec<u64> = doc
        .objects
        .iter()
        .filter(|o| {
            let b = ops::bounds_doc(doc, &o.kind);
            // Textbreiten sind geschätzt und ragen leicht über den Kasten: bei Texten zählt die Mitte
            let drin = b.inside(&innen) || (matches!(o.kind, ObjKind::Text { .. }) && b.center().x > bb.min.x && b.center().x < bb.max.x && b.center().y > bb.min.y && b.center().y < bb.max.y);
            ids.contains(&o.id) && !o.locked && !matches!(o.kind, ObjKind::Legend { .. }) && drin
        })
        .map(|o| o.id)
        .collect();
    if teil.len() < 2 {
        return None;
    }
    let stelle = doc.objects.iter().position(|o| teil.contains(&o.id))?;
    let mut kinder = Vec::new();
    doc.objects.retain(|o| {
        if teil.contains(&o.id) {
            kinder.push(o.clone());
            false
        } else {
            true
        }
    });
    let layer = doc.find_layer_by_name(ebene);
    let id = doc.alloc_id();
    doc.objects.insert(stelle.min(doc.objects.len()), Obj { id, layer, kind: ObjKind::Group { children: kinder }, style: Style::default(), print: true, locked: false });
    Some(id)
}

/// Planbereiche eines importierten Plans aufbereiten:
/// - Schriftfeld: Angaben (Objekt, Gebäude, Etage, Stand, Plan-Nr.) in die Plandaten übernehmen und die
///   Werte durch Platzhalter ersetzen, damit sie sich mit den Plandaten ändern
/// - Kopfzeile, Verhaltensregeln, Übersichtsplan, Schriftfeld, Prüfplakette je als eine Gruppe auf eigene
///   Ebene legen (ein Klick wählt den ganzen Bereich, zum Verschieben oder Löschen)
///
/// Liefert die Namen der aufbereiteten Bereiche.
pub fn bereiche_aufbereiten(doc: &mut Doc, ids: &[u64]) -> Vec<&'static str> {
    let mut erledigt = Vec::new();
    let texte = |doc: &Doc| -> Vec<(u64, BBox, String)> {
        doc.objects
            .iter()
            .filter(|o| ids.contains(&o.id))
            .filter_map(|o| match &o.kind {
                ObjKind::Text { text, .. } if !text.trim().is_empty() => Some((o.id, ops::bounds_doc(doc, &o.kind), text.clone())),
                _ => None,
            })
            .collect()
    };
    let kaesten = |doc: &Doc| -> Vec<BBox> { doc.objects.iter().filter(|o| ids.contains(&o.id) && ist_rechteckig(&o.kind)).map(|o| ops::bounds_doc(doc, &o.kind)).collect() };
    let (pw, ph) = doc.page.size();
    let blatt = pw * ph;

    // --- Schriftfeld ---
    let feld = |n: &str| -> Option<&'static str> {
        Some(match n {
            "objekt" => "objekt",
            "gebäude" | "gebaeude" | "bauteil" => "gebaeude",
            "etage" | "geschoss" | "ebene" => "geschoss",
            "stand" | "datum" => "stand",
            "plannr" | "plannummer" | "zeichnungsnr" => "plannummer",
            _ => return None,
        })
    };
    let alle = texte(doc);
    let labels: Vec<(u64, BBox, &'static str)> = alle.iter().filter_map(|(id, b, t)| feld(&begriff_norm(t)).map(|f| (*id, *b, f))).collect();
    let mut werte: Vec<(&'static str, Vec<u64>, String)> = Vec::new();
    for (lid, lb, f) in &labels {
        let h = lb.h().max(1.0);
        let mut zeile: Vec<&(u64, BBox, String)> = alle
            .iter()
            .filter(|(id, b, _)| id != lid && b.min.x >= lb.max.x - 0.5 && b.min.x <= lb.max.x + 120.0 && b.center().y >= lb.min.y - 0.5 * h && b.center().y <= lb.max.y + 0.5 * h)
            .collect();
        zeile.sort_by(|a, b| a.1.min.x.total_cmp(&b.1.min.x));
        // bis zur nächsten Beschriftung derselben Zeile
        let teil: Vec<&(u64, BBox, String)> = zeile.into_iter().take_while(|(id, _, _)| !labels.iter().any(|(l, _, _)| l == id)).collect();
        let wert = teil.iter().map(|(_, _, t)| t.trim()).collect::<Vec<_>>().join(" ");
        if !wert.is_empty() && !werte.iter().any(|(g, _, _)| g == f) {
            werte.push((f, teil.iter().map(|(id, _, _)| *id).collect(), wert));
        }
    }
    // Werte nur übernehmen, wenn sie als Text vorliegen (oft sind sie als Umrisse gezeichnet)
    if werte.len() >= 3 {
        for (f, tids, wert) in &werte {
            let i = &mut doc.info;
            match *f {
                "objekt" => i.objekt = wert.clone(),
                "gebaeude" => i.gebaeude = wert.clone(),
                "geschoss" => i.geschoss = wert.clone(),
                "stand" => i.stand = wert.clone(),
                _ => i.plannummer = wert.clone(),
            }
            // erster Werttext wird zum Platzhalter, weitere Teile derselben Zeile entfallen
            if let Some(o) = doc.objects.iter_mut().find(|o| o.id == tids[0]) {
                if let ObjKind::Text { text, .. } = &mut o.kind {
                    *text = format!("{{{f}}}");
                }
            }
            doc.objects.retain(|o| !tids[1..].contains(&o.id));
        }
    }
    let mut feldarten: Vec<&str> = labels.iter().map(|l| l.2).collect();
    feldarten.sort();
    feldarten.dedup();
    if feldarten.len() >= 3 {
        let lb = labels.iter().map(|l| l.1).reduce(|a, b| a.union(b)).unwrap();
        if let Some(k) = kaesten(doc).into_iter().filter(|k| lb.inside(&k.expand(0.5)) && k.w() * k.h() < blatt * 0.25).min_by(|a, b| (a.w() * a.h()).total_cmp(&(b.w() * b.h()))) {
            gruppieren(doc, ids, k, "Schriftfeld");
        }
        erledigt.push("Schriftfeld");
    }

    // --- Bereiche mit Überschrift ---
    // (Überschrift, Ebene, größter Kasten um die Überschrift, der keine andere Überschrift enthält)
    let titel = |n: &str| -> Option<&'static str> {
        if n.starts_with("verhaltenimbrandfall") || n.starts_with("verhaltenbeiunfällen") || n.starts_with("verhaltenbeiunfaellen") {
            Some("Verhaltensregeln")
        } else if n.starts_with("übersichtsplan") || n.starts_with("uebersichtsplan") || n.starts_with("lageplan") {
            Some("Übersichtsplan")
        } else if n.contains("fluchtundrettungsplan") || n.contains("brandschutzplan") {
            Some("Kopfzeile")
        } else {
            None
        }
    };
    let alle = texte(doc);
    let ueberschriften: Vec<(BBox, &'static str)> = alle.iter().filter_map(|(_, b, t)| titel(&begriff_norm(t)).map(|e| (*b, e))).collect();
    let enthaelt = |k: &BBox, p: Pos| p.x >= k.min.x - 0.5 && p.x <= k.max.x + 0.5 && p.y >= k.min.y - 0.5 && p.y <= k.max.y + 0.5;
    for (tb, ebene) in &ueberschriften {
        let andere: Vec<Pos> = ueberschriften.iter().filter(|(b, _)| b != tb).map(|(b, _)| b.center()).collect();
        let alle_k = kaesten(doc);
        let k = alle_k
            .iter()
            .filter(|k| enthaelt(k, tb.center()) && tb.min.x >= k.min.x - 1.0 && tb.max.x <= k.max.x + 1.0 && k.w() * k.h() < blatt * 0.3 && !andere.iter().any(|a| enthaelt(k, *a)))
            .max_by(|a, b| (a.w() * a.h()).total_cmp(&(b.w() * b.h())))
            .copied();
        // Überschrift im eigenen Balken: direkt anschließenden Kasten gleicher Breite dazunehmen
        let k = k.map(|k| {
            let mut k = k;
            for n in &alle_k {
                let gleich_breit = (n.min.x - k.min.x).abs() < 1.5 && (n.max.x - k.max.x).abs() < 1.5;
                let anschliessend = (n.min.y - k.max.y).abs() < 2.0 || (k.min.y - n.max.y).abs() < 2.0;
                if gleich_breit && anschliessend && n.w() * n.h() < blatt * 0.3 && !andere.iter().any(|a| enthaelt(n, *a)) {
                    k = k.union(*n);
                }
            }
            k
        });
        if let Some(k) = k {
            if gruppieren(doc, ids, k, ebene).is_some() && !erledigt.contains(ebene) {
                erledigt.push(ebene);
            }
        }
    }

    // --- Prüfplakette („Nächste Prüfung“ mit Plakette darunter) ---
    let alle = texte(doc);
    let pruef: Vec<BBox> = alle.iter().filter(|(_, _, t)| matches!(begriff_norm(t).as_str(), "nächste" | "prüfung" | "nächsteprüfung")).map(|(_, b, _)| *b).collect();
    if let Some(tb) = pruef.into_iter().reduce(|a, b| a.union(b)) {
        // Hinweisschild um den Text und die Plakette (etwa so breit wie das Schild) darunter
        let w = tb.w().max(10.0) * 1.6;
        let bb = BBox::from_two(Pos::new(tb.center().x - w / 2.0, tb.min.y - 3.0), Pos::new(tb.center().x + w / 2.0, tb.max.y + w * 1.3));
        if gruppieren(doc, ids, bb, "Prüfplakette").is_some() {
            erledigt.push("Prüfplakette");
        }
    }
    erledigt
}

/// Nur den Grundriss behalten: Zeichen, Legende, Bausteine, erkannte Planbereiche (Kopfzeile, Regeln,
/// Übersichtsplan, Schriftfeld, Prüfplakette), Rettungswegflächen, Standort-Hinweis und Blattrahmen entfernen.
/// Liefert die Zahl der entfernten Objekte.
pub fn nur_grundriss(doc: &mut Doc) -> usize {
    let bereich_ebenen: Vec<u32> = doc.layers.iter().filter(|l| BEREICH_EBENEN.contains(&l.name.as_str()) || l.name == "Legende").map(|l| l.id).collect();
    let (pw, ph) = doc.page.size();
    // Standort-Hinweis: Text „Standort“ samt gefülltem Kästchen darum
    let standort_texte: Vec<BBox> = doc
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjKind::Text { text, .. } if begriff_norm(text) == "standort" => Some(ops::bounds_doc(doc, &o.kind)),
            _ => None,
        })
        .collect();
    // gezeichnete Richtungspfeile: kleine Formen in Sicherheitsgrün
    fn gruen(c: Color) -> bool {
        c.1 >= 90 && c.0 < 80 && (c.1 as i32 - c.2 as i32) > 20
    }
    fn gruen_gefuellt(o: &Obj) -> bool {
        match &o.kind {
            ObjKind::Group { children } => children.iter().any(gruen_gefuellt),
            _ => matches!(o.style.fill, Fill::Farbe(c) if gruen(c)),
        }
    }
    let weg = |o: &Obj, doc: &Doc| -> bool {
        if o.locked {
            return false;
        }
        let b = ops::bounds_doc(doc, &o.kind);
        let gefuellt = matches!(o.style.fill, Fill::Farbe(_));
        match &o.kind {
            ObjKind::Symbol { .. } | ObjKind::Legend { .. } | ObjKind::Block { .. } | ObjKind::Route { .. } | ObjKind::Area { .. } => true,
            _ if bereich_ebenen.contains(&o.layer) => true,
            ObjKind::Text { text, .. } if begriff_norm(text) == "standort" => true,
            // Blattrahmen
            _ if ist_rechteckig(&o.kind) && b.w() > pw * 0.9 && b.h() > ph * 0.85 => true,
            // Rettungswegflächen und Standort-Kästchen
            _ if gefuellt && ops::legend_key(doc, &o.kind, &o.style).is_some_and(|(k, _)| k == "area:fluchtbereich") => true,
            _ if b.w() < 12.0 && b.h() < 12.0 && gruen_gefuellt(o) => true,
            _ if gefuellt && b.w() < 40.0 && standort_texte.iter().any(|t| t.inside(&b.expand(0.5))) => true,
            _ => false,
        }
    };
    let vorher = doc.objects.len();
    let behalten: Vec<bool> = doc.objects.iter().map(|o| !weg(o, doc)).collect();
    let mut i = 0;
    doc.objects.retain(|_| {
        i += 1;
        behalten[i - 1]
    });
    let benutzt: Vec<String> = ops::used_legend_keys(doc).into_iter().map(|(k, _)| k).collect();
    doc.legend.entries.retain(|e| benutzt.contains(&e.key));
    vorher - doc.objects.len()
}
#[cfg(test)]
mod bereiche_tests {
    use super::*;

    fn obj(doc: &mut Doc, kind: ObjKind, style: Style) -> u64 {
        let id = doc.alloc_id();
        doc.objects.push(Obj { id, layer: 0, kind, style, print: true, locked: false });
        id
    }
    fn text(doc: &mut Doc, x: f32, y: f32, t: &str) -> u64 {
        obj(doc, ObjKind::Text { pos: Pos::new(x, y), text: t.into(), size: 2.5, bold: false, italic: false, align: TAlign::Links, rot: 0.0 }, Style::default())
    }
    fn rahmen(doc: &mut Doc, x0: f32, y0: f32, x1: f32, y1: f32) -> u64 {
        obj(doc, ObjKind::Rect { a: Pos::new(x0, y0), b: Pos::new(x1, y1), radius: 0.0 }, Style::line(farbe::SCHWARZ, 0.3))
    }
    fn ebene(doc: &Doc, o: &Obj) -> String {
        doc.layers.iter().find(|l| l.id == o.layer).map(|l| l.name.clone()).unwrap_or_default()
    }

    /// Schriftfeld-Werte werden zu Platzhaltern, Kästen zu Gruppen auf eigenen Ebenen; „Nur Grundriss“ räumt alles ab.
    #[test]
    fn bereiche_und_nur_grundriss() {
        let mut doc = Doc::default();
        // Grundriss
        let wand = obj(&mut doc, ObjKind::Polyline { pts: vec![Pos::new(100.0, 60.0), Pos::new(250.0, 60.0), Pos::new(250.0, 200.0)], closed: false }, Style::line(farbe::SCHWARZ, 1.0));
        let raum = text(&mut doc, 150.0, 120.0, "Lager");
        obj(&mut doc, ObjKind::Symbol { id: "F001".into(), pos: Pos::new(120.0, 100.0), size: 8.0, rot: 0.0 }, Style::NONE);
        // Kopfzeile
        rahmen(&mut doc, 5.0, 5.0, 415.0, 35.0);
        text(&mut doc, 50.0, 12.0, "FLUCHT- UND RETTUNGSPLAN");
        // Verhaltensregeln
        rahmen(&mut doc, 8.0, 40.0, 85.0, 125.0);
        text(&mut doc, 20.0, 44.0, "Verhalten im Brandfall");
        text(&mut doc, 20.0, 60.0, "Ruhe bewahren");
        // Übersichtsplan: Überschrift in eigenem Balken, ragt etwas heraus
        rahmen(&mut doc, 320.0, 38.0, 412.0, 49.0);
        text(&mut doc, 336.0, 39.0, "Übersichtsplan");
        rahmen(&mut doc, 320.0, 49.0, 412.0, 147.0);
        rahmen(&mut doc, 330.0, 60.0, 400.0, 130.0);
        // Schriftfeld
        rahmen(&mut doc, 325.0, 254.0, 415.0, 292.0);
        for (x, y, l, w) in [(326.0, 257.0, "Objekt", "Evers & Co"), (326.0, 264.0, "Gebäude:", "Kantine"), (371.0, 264.0, "Etage:", "Erdgeschoss"), (326.0, 270.0, "Stand:", "09 / 2024"), (371.0, 270.0, "Plan-Nr.:", "23-034/920")] {
            text(&mut doc, x, y, l);
            text(&mut doc, x + 16.0, y, w);
        }
        let ids: Vec<u64> = doc.objects.iter().map(|o| o.id).collect();
        let fertig = bereiche_aufbereiten(&mut doc, &ids);
        for b in ["Schriftfeld", "Verhaltensregeln", "Übersichtsplan", "Kopfzeile"] {
            assert!(fertig.contains(&b), "{b} fehlt: {fertig:?}");
        }
        assert_eq!(doc.info.objekt, "Evers & Co");
        assert_eq!(doc.info.gebaeude, "Kantine");
        assert_eq!(doc.info.geschoss, "Erdgeschoss");
        assert_eq!(doc.info.stand, "09 / 2024");
        assert_eq!(doc.info.plannummer, "23-034/920");
        // Werte als Platzhalter im Schriftfeld, die Darstellung zeigt den Wert aus den Plandaten
        let mut platzhalter = Vec::new();
        fn sammeln(o: &Obj, out: &mut Vec<String>) {
            match &o.kind {
                ObjKind::Text { text, .. } if text.starts_with('{') => out.push(text.clone()),
                ObjKind::Group { children } => children.iter().for_each(|c| sammeln(c, out)),
                _ => {}
            }
        }
        doc.objects.iter().for_each(|o| sammeln(o, &mut platzhalter));
        assert_eq!(platzhalter.len(), 5, "{platzhalter:?}");
        assert_eq!(ops::platzhalter(&doc, "{gebaeude}"), "Kantine");
        // Übersichtsplan samt Balken und Inhalt in einer Gruppe
        let ue = doc.objects.iter().find(|o| ebene(&doc, o) == "Übersichtsplan").expect("Gruppe Übersichtsplan");
        assert!(matches!(&ue.kind, ObjKind::Group { children } if children.len() == 4));
        // Grundriss bleibt außerhalb der Gruppen
        assert!(doc.objects.iter().any(|o| o.id == wand) && doc.objects.iter().any(|o| o.id == raum));

        let n = nur_grundriss(&mut doc);
        assert!(n >= 5, "nur {n} entfernt");
        let rest: Vec<u64> = doc.objects.iter().map(|o| o.id).collect();
        assert_eq!(rest, vec![wand, raum]);
    }
}
