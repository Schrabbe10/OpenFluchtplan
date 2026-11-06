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

/// Bestes Normzeichen für ein Objekt: (Vorlage, Abstand, Rahmen, zulässige Schwelle).
/// Enthält es Text, kommen nur Zeichen mit gleichem Text in Frage.
fn vergleichen(doc: &Doc, o: &Obj) -> Option<(&'static Vorlage, f32, BBox, f32)> {
    let b = ops::bounds_doc(doc, &o.kind);
    let einzeln = !matches!(o.kind, ObjKind::Group { .. });
    if b.w().max(b.h()) > MAX_MM || b.w().min(b.h()) < if einzeln { 4.0 } else { 1.5 } {
        return None;
    }
    let mut t = String::new();
    obj_text(o, &mut t);
    let t = text_norm(&t);
    let schwelle = if !t.is_empty() {
        SCHWELLE_TEXT
    } else if einzeln {
        SCHWELLE_EINZELN
    } else {
        SCHWELLE
    };
    let (px, origin, kante) = gruppe_rastern(doc, o, b)?;
    let (bild, pb) = normieren(&px)?;
    // sichtbarer Inhalt in mm (wie der Inhaltsrahmen der Vorlagen)
    let f = kante / R as f32;
    let b = BBox::from_two(origin + Pos::new(pb[0] * f, pb[1] * f), origin + Pos::new(pb[2] * f, pb[3] * f));
    let mut best: Option<(&Vorlage, f32)> = None;
    for v in vorlagen().iter().filter(|v| t.is_empty() || v.text == t) {
        // gedrehte Fassungen nur bei deutlich besserer Übereinstimmung; gleich aussehende Zeichen
        // (z. B. Rauchmelder / Rauchabzug) nach Planart unterscheiden
        let fremd = match v.cat {
            symbols::Category::Brandmeldetechnik => doc.frame != FrameKind::Laufkarte,
            symbols::Category::Feuerwehr => doc.frame == FrameKind::Laufkarte,
            _ => false,
        };
        let d = abstand(&bild, &v.bild) + if v.rot == 0.0 { 0.0 } else { 0.01 } + if fremd { 0.002 } else { 0.0 };
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((v, d));
        }
    }
    best.map(|(v, d)| (v, d, b, schwelle))
}

/// Passendes Normzeichen für eine Gruppe. Passt die ganze Gruppe nicht, wird sie ohne je eines
/// ihrer Teile verglichen (beim Import hängen oft angrenzende Linien oder Pfeilspitzen mit daran).
fn zuordnen(doc: &Doc, o: &Obj) -> Option<Fund> {
    let children: &[Obj] = match &o.kind {
        ObjKind::Group { children } => children,
        ObjKind::Polyline { closed: true, .. } | ObjKind::Path { .. } => &[],
        _ => return None,
    };
    if children.iter().any(|c| matches!(c.kind, ObjKind::Group { .. } | ObjKind::Symbol { .. })) {
        return None;
    }
    // Rang: Abstand im Verhältnis zur jeweiligen Schwelle
    let mut best = vergleichen(doc, o).map(|(v, d, b, s)| (v, d / s, b, None));
    if best.as_ref().is_none_or(|t| t.1 > 1.0) && (3..=12).contains(&children.len()) {
        for k in 0..children.len() {
            let mut teil = o.clone();
            if let ObjKind::Group { children } = &mut teil.kind {
                children.remove(k);
            }
            if let Some((v, d, b, s)) = vergleichen(doc, &teil) {
                if best.as_ref().is_none_or(|t| d / s < t.1) {
                    best = Some((v, d / s, b, Some(k)));
                }
            }
        }
    }
    let (v, d, b, rest) = best?;
    if d > 1.0 {
        return None;
    }
    // Größe und Lage des Symbolfelds aus dem Inhaltsrahmen der Vorlage zurückrechnen
    let (bw, bh) = ((v.bb[2] - v.bb[0]) / 100.0, (v.bb[3] - v.bb[1]) / 100.0);
    let size = (b.w() / bw + b.h() / bh) / 2.0;
    let x0 = b.min.x - v.bb[0] / 100.0 * size;
    let y0 = b.min.y - v.bb[1] / 100.0 * size;
    Some(Fund { id: v.id, pos: Pos::new(x0 + size / 2.0, y0 + size / 2.0), size, rot: v.rot, rest })
}

/// Gruppen (oberste Ebene, optional nur `ids`) durch erkannte Normzeichen ersetzen und die Legende
/// abgleichen. Liefert die Anzahl ersetzter Zeichen.
pub fn normzeichen_einsetzen(doc: &mut Doc, ids: Option<&[u64]>) -> usize {
    let mut funde: Vec<(usize, Fund)> = doc
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| !o.locked && ids.is_none_or(|ids| ids.contains(&o.id)))
        .filter_map(|(i, o)| zuordnen(doc, o).map(|f| (i, f)))
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
    ops::sync_legend(doc);
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
