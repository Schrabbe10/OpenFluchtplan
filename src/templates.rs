//! Planrahmen (Vorlagen). Layout passt sich Blattgröße und Ausrichtung an.

use crate::geom::*;
use crate::model::*;
use crate::render::{Prim, Prims, fill_rect, frame_rect, legend_prims, line, text};

pub const REGELN_BRAND: [&str; 8] = [
    "Ruhe bewahren",
    "Brand melden (Notruf / Brandmelder)",
    "Gefährdete Personen warnen",
    "Türen schließen, nicht abschließen",
    "Gekennzeichneten Fluchtwegen folgen",
    "Keine Aufzüge benutzen",
    "Sammelstelle aufsuchen",
    "Löschversuch nur ohne Eigengefährdung",
];

pub const REGELN_UNFALL: [&str; 4] = ["Ruhe bewahren", "Unfall melden", "Erste Hilfe leisten", "Anweisungen beachten"];

/// Bereiche eines Rahmens.
pub struct Layout {
    pub plan: BBox,
    pub legend: Option<BBox>,
    pub rules: Option<BBox>,
    pub notruf: Option<BBox>,
    pub sammel: Option<BBox>,
    pub kopf: Option<BBox>,
    pub header: Option<BBox>,
    pub nummer: Option<BBox>,
    pub laufkarte: Option<BBox>,
}

fn b(x0: f32, y0: f32, x1: f32, y1: f32) -> BBox {
    BBox::from_two(Pos::new(x0, y0), Pos::new(x1, y1))
}

pub fn layout(doc: &Doc) -> Layout {
    let (w, h) = doc.page.size();
    let u = w.min(h) / 297.0; // Grundeinheit relativ zu A3
    let m = 10.0 * u;
    let gap = 5.0 * u;
    let mut l = Layout { plan: b(m, m, w - m, h - m), legend: None, rules: None, notruf: None, sammel: None, kopf: None, header: None, nummer: None, laufkarte: None };
    let cfg = &doc.frame_cfg;
    match doc.frame {
        FrameKind::Ohne => l.plan = b(0.0, 0.0, w, h),
        FrameKind::FrVollstaendig | FrameKind::FrZweizeilig | FrameKind::BrandschutzVollstaendig => {
            let zwei = doc.frame == FrameKind::FrZweizeilig;
            let hh = if zwei { 30.0 * u } else { 22.0 * u };
            l.header = Some(b(m, m, w - m, m + hh));
            let top = m + hh + gap + 8.0 * u;
            let brand = doc.frame == FrameKind::BrandschutzVollstaendig;
            if w >= h {
                let lw = (w * 0.23).max(80.0 * u);
                let x1 = m + lw;
                let mut y = m + hh + gap;
                if cfg.show_rules && !brand {
                    l.rules = Some(b(m, y, x1, y + 87.0 * u));
                    y += 87.0 * u + gap * 0.6;
                }
                if cfg.show_notruf && !brand {
                    l.notruf = Some(b(m, y, x1, y + 17.0 * u));
                    y += 17.0 * u + gap;
                }
                l.legend = Some(b(m, y, x1, h - m));
                let px = x1 + gap * 1.4;
                let bottom = h - m - 32.0 * u;
                l.plan = b(px, top, w - m, bottom - gap);
                if cfg.show_sammelstelle && !brand {
                    l.sammel = Some(b(px, bottom, px + (w - m - px) * 0.5, h - m));
                }
                l.kopf = Some(b(w - m - (w - m - px) * 0.45, bottom, w - m, h - m));
            } else {
                // Hochformat: Band unten etwas höher, damit die Verhaltensregeln Platz haben
                let band = if cfg.show_rules && !brand { 88.0 } else { 75.0 } * u;
                let by = h - m - band;
                l.plan = b(m, top, w - m, by - gap);
                let cw = (w - 2.0 * m - 2.0 * gap) / 3.0;
                let x0 = m;
                if cfg.show_rules && !brand {
                    l.rules = Some(b(x0, by, x0 + cw, h - m - 20.0 * u));
                    if cfg.show_notruf {
                        l.notruf = Some(b(x0, h - m - 17.0 * u, x0 + cw, h - m));
                    }
                }
                l.legend = Some(b(x0 + cw + gap, by, x0 + 2.0 * cw + gap, h - m));
                let kx = x0 + 2.0 * (cw + gap);
                if cfg.show_sammelstelle && !brand {
                    l.sammel = Some(b(kx, by, w - m, by + band * 0.45));
                    l.kopf = Some(b(kx, by + band * 0.45 + gap * 0.5, w - m, h - m));
                } else {
                    l.kopf = Some(b(kx, by, w - m, h - m));
                }
            }
        }
        FrameKind::FrNurOben | FrameKind::BrandschutzNurOben => {
            let hh = 22.0 * u;
            l.header = Some(b(m, m, w - m, m + hh));
            l.plan = b(m, m + hh + gap + 8.0 * u, w - m, h - m);
        }
        FrameKind::Feuerwehrplan => {
            let band = 52.0 * u;
            let kw = 95.0 * u;
            l.nummer = Some(b(w - m - 45.0 * u, m, w - m, m + 14.0 * u));
            l.plan = b(m, m, w - m, h - m - band);
            l.legend = Some(b(m, h - m - band, w - m - kw, h - m));
            l.kopf = Some(b(w - m - kw, h - m - band, w - m, h - m));
        }
        FrameKind::Laufkarte => {
            let hh = 18.0 * u;
            l.header = Some(b(m, m, w - m, m + hh));
            l.laufkarte = Some(b(m, m + hh, w - m, m + hh + 22.0 * u));
            let foot = 26.0 * u;
            l.plan = b(m, m + hh + 22.0 * u + gap, w - m, h - m - foot - gap);
            l.legend = Some(b(m, h - m - foot, w - m - 90.0 * u, h - m));
            l.kopf = Some(b(w - m - 90.0 * u, h - m - foot, w - m, h - m));
        }
    }
    l
}

/// Planinhalt nach einem Wechsel von Format oder Rahmen in den neuen Planbereich übertragen
/// (gleichmäßig skaliert, mittig). Der Maßstab wird entsprechend angepasst. Liefert den Faktor.
pub fn inhalt_umsetzen(doc: &mut Doc, alt: BBox) -> Option<f32> {
    let neu = layout(doc).plan;
    if doc.objects.is_empty() || alt.w() <= 0.0 || alt.h() <= 0.0 {
        return None;
    }
    let f = (neu.w() / alt.w()).min(neu.h() / alt.h());
    let ziel = neu.min + Pos::new((neu.w() - alt.w() * f) / 2.0, (neu.h() - alt.h() * f) / 2.0);
    if (f - 1.0).abs() < 1e-3 && (ziel - alt.min).len() < 0.05 {
        return None;
    }
    for o in &mut doc.objects {
        crate::ops::scale(&mut o.kind, alt.min, f);
        crate::ops::translate(&mut o.kind, ziel - alt.min);
    }
    if doc.scale_n > 0.0 {
        doc.scale_n = (doc.scale_n / f).round().max(1.0);
    }
    Some(f)
}

fn white_box(out: &mut Prims, bb: BBox, line_w: f32) {
    fill_rect(out, bb.min, bb.max, farbe::WEISS);
    frame_rect(out, bb.min, bb.max, line_w, farbe::SCHWARZ);
}

/// Verhaltensregeln (Brandfall, Unfälle) in ein Feld setzen; `u` ist die Grundeinheit (1 = A3).
pub fn regeln_prims(out: &mut Prims, rb: BBox, u: f32) {
    // Schrift verkleinern, wenn das Feld (z. B. bei A4 hoch) nicht für alle Regeln reicht
    let zeilen = (REGELN_BRAND.len() + REGELN_UNFALL.len()) as f32;
    let bedarf = (3.0 + 7.0 + 3.0 + 7.0 + 2.0) * u + zeilen * 3.2 * u * 1.5;
    let k = (rb.h() / bedarf).min(1.0);
    let u = u * k;
    let s = 3.2 * u;
    let mut y = rb.min.y + 3.0 * u;
    let x = rb.min.x + 3.0 * u;
    text(out, Pos::new(x, y), "Verhalten im Brandfall", 4.5 * u, farbe::SCHWARZ, TAlign::Links, true);
    y += 7.0 * u;
    for r in REGELN_BRAND {
        text(out, Pos::new(x + u, y), format!("• {r}"), s, farbe::SCHWARZ, TAlign::Links, false);
        y += s * 1.5;
    }
    y += 3.0 * u;
    text(out, Pos::new(x, y), "Verhalten bei Unfällen", 4.5 * u, farbe::SCHWARZ, TAlign::Links, true);
    y += 7.0 * u;
    for r in REGELN_UNFALL {
        text(out, Pos::new(x + u, y), format!("• {r}"), s, farbe::SCHWARZ, TAlign::Links, false);
        y += s * 1.5;
    }
}

/// Notruf-Balken.
pub fn notruf_prims<'a>(out: &mut Prims<'a>, doc: &'a Doc, nb: BBox) {
    let info = &doc.info;
    fill_rect(out, nb.min, nb.max, farbe::ROT);
    let ts = nb.h() * 0.42;
    text(out, Pos::new(nb.center().x, nb.min.y + (nb.h() - ts) / 2.0), format!("Notruf: {}", info.notruf), ts, farbe::WEISS, TAlign::Mitte, true);
}

/// Sammelstelle mit Zeichen und Beschreibung.
pub fn sammel_prims<'a>(out: &mut Prims<'a>, doc: &'a Doc, sb: BBox, u: f32) {
    let info = &doc.info;
    let s = (sb.h() * 0.6).min(16.0 * u);
    out.push(Prim::Symbol { id: "E007".into(), c: Pos::new(sb.min.x + s * 0.5 + 3.0 * u, sb.center().y), size: s, rot: 0.0 });
    let tx = sb.min.x + s + 7.0 * u;
    text(out, Pos::new(tx, sb.min.y + 4.0 * u), "Sammelstelle", 4.5 * u, farbe::SCHWARZ, TAlign::Links, true);
    for (i, l) in info.sammelstelle.lines().enumerate() {
        text(out, Pos::new(tx, sb.min.y + 11.0 * u + i as f32 * 4.5 * u), l, 3.5 * u, farbe::SCHWARZ, TAlign::Links, false);
    }
}

/// Schriftfeld mit den Plandaten.
pub fn schriftfeld_prims<'a>(out: &mut Prims<'a>, doc: &'a Doc, kb: BBox, u: f32) {
    let info = &doc.info;
    let s = (3.0 * u).min(kb.h() / 11.0);
    let x = kb.min.x + 2.5 * u;
    let mut y = kb.min.y + 2.0 * u;
    text(out, Pos::new(x, y), info.objekt.clone(), s * 1.45, farbe::SCHWARZ, TAlign::Links, true);
    y += s * 1.9;
    for line_ in info.anschrift.lines().take(2) {
        text(out, Pos::new(x, y), line_, s, farbe::SCHWARZ, TAlign::Links, false);
        y += s * 1.3;
    }
    let mut rows: Vec<(&str, String)> = Vec::new();
    if doc.frame == FrameKind::Feuerwehrplan {
        rows.push(("Planart:", info.planart.clone()));
    }
    if doc.frame == FrameKind::Laufkarte {
        rows.push(("BMZ:", info.bmz_standort.clone()));
    }
    if !info.gebaeude.is_empty() {
        rows.push(("Gebäude:", info.gebaeude.clone()));
    }
    rows.push(("Geschoss:", info.geschoss.clone()));
    if !info.auftraggeber.is_empty() {
        rows.push(("Auftraggeber:", info.auftraggeber.clone()));
    }
    rows.push(("Ersteller:", info.ersteller.clone()));
    rows.push(("Stand:", info.stand.clone()));
    if doc.frame != FrameKind::Feuerwehrplan {
        rows.push(("Plan-Nr.:", info.plannummer.clone()));
    }
    if !info.revision.is_empty() {
        rows.push(("Revision:", info.revision.clone()));
    }
    if !info.sonstiges.is_empty() {
        rows.push(("", info.sonstiges.clone()));
    }
    rows.push(("Maßstab:", format!("1:{}", doc.scale_n)));
    let avail = kb.max.y - y - 1.0 * u;
    let lh = (avail / rows.len() as f32).min(s * 1.45);
    for (k, v) in rows {
        text(out, Pos::new(x, y), k, s, farbe::SCHWARZ, TAlign::Links, false);
        text(out, Pos::new(x + 22.0 * u, y), v, s, farbe::SCHWARZ, TAlign::Links, false);
        y += lh;
    }
}

/// Feste Rahmenbestandteile. Der Planbereich bleibt frei.
pub fn frame_prims(doc: &Doc) -> Prims<'_> {
    let mut out: Prims = Vec::new();
    if doc.frame == FrameKind::Ohne {
        return out;
    }
    let (w, h) = doc.page.size();
    let u = w.min(h) / 297.0;
    let l = layout(doc);
    let info = &doc.info;
    let cfg = &doc.frame_cfg;
    let brand = matches!(doc.frame, FrameKind::BrandschutzVollstaendig | FrameKind::BrandschutzNurOben);
    let color = if brand && cfg.color == farbe::GRUEN { farbe::ROT } else { cfg.color };

    // Abdeckung außerhalb des Planbereichs (Ränder)
    let p = l.plan;
    for (a, bb) in [
        (Pos::new(0.0, 0.0), Pos::new(w, p.min.y)),
        (Pos::new(0.0, p.max.y), Pos::new(w, h)),
        (Pos::new(0.0, p.min.y), Pos::new(p.min.x, p.max.y)),
        (Pos::new(p.max.x, p.min.y), Pos::new(w, p.max.y)),
    ] {
        if bb.x > a.x + 0.01 && bb.y > a.y + 0.01 {
            fill_rect(&mut out, a, bb, farbe::WEISS);
        }
    }
    frame_rect(&mut out, Pos::new(5.0 * u, 5.0 * u), Pos::new(w - 5.0 * u, h - 5.0 * u), 0.5, farbe::SCHWARZ);

    if let Some(hb) = l.header {
        fill_rect(&mut out, hb.min, hb.max, color);
        let title = if cfg.title.is_empty() {
            if brand { "BRANDSCHUTZPLAN".to_string() } else { "FLUCHT- UND RETTUNGSPLAN".to_string() }
        } else if brand && cfg.title == FrameCfg::default().title {
            "BRANDSCHUTZPLAN".to_string()
        } else if doc.frame == FrameKind::Laufkarte && cfg.title == FrameCfg::default().title {
            "FEUERWEHR-LAUFKARTE".to_string()
        } else {
            cfg.title.clone()
        };
        let zwei = doc.frame == FrameKind::FrZweizeilig;
        let ts = (hb.h() * if zwei { 0.42 } else { 0.62 }).min(hb.w() / (title.chars().count() as f32 * 0.66));
        let mut ty = hb.min.y + (hb.h() - ts) / 2.0;
        if zwei {
            ty = hb.min.y + hb.h() * 0.1;
        }
        text(&mut out, Pos::new(hb.center().x, ty), title, ts, farbe::WEISS, TAlign::Mitte, true);
        if zwei {
            let ts2 = ts * 0.62;
            text(&mut out, Pos::new(hb.center().x, ty + ts * 1.25), cfg.title2.clone(), ts2, farbe::WEISS, TAlign::Mitte, false);
        }
        if let Some(logo) = &cfg.logo {
            let lh = hb.h() * 0.8;
            let lw = lh * logo.w as f32 / logo.h.max(1) as f32;
            let a = Pos::new(hb.max.x - lw - hb.h() * 0.1, hb.min.y + hb.h() * 0.1);
            fill_rect(&mut out, a - Pos::new(1.0, 1.0), a + Pos::new(lw + 1.0, lh + 1.0), farbe::WEISS);
            out.push(Prim::Image { key: u64::MAX, a, b: a + Pos::new(lw, lh), data: logo, opacity: 1.0 });
        }
        if doc.frame != FrameKind::Laufkarte {
            // Geschoss-Titel über dem Planbereich
            let gs = 8.0 * u;
            text(&mut out, Pos::new(p.min.x, hb.max.y + 2.0 * u), info.geschoss.clone(), gs, color, TAlign::Links, true);
        }
    }

    if let Some(rb) = l.rules {
        white_box(&mut out, rb, 0.4);
        regeln_prims(&mut out, rb, u);
    }
    if let Some(nb) = l.notruf {
        notruf_prims(&mut out, doc, nb);
    }
    if let Some(lb) = l.legend {
        white_box(&mut out, lb, 0.4);
        let (cols, size) = crate::render::legend_fit(doc, lb, u);
        legend_prims(&mut out, doc, lb.min, lb.max, cols, size, true);
    }
    if let Some(sb) = l.sammel {
        white_box(&mut out, sb, 0.4);
        sammel_prims(&mut out, doc, sb, u);
    }
    if let Some(nb) = l.nummer {
        white_box(&mut out, nb, 0.5);
        text(&mut out, Pos::new(nb.center().x, nb.min.y + nb.h() * 0.2), info.plannummer.clone(), nb.h() * 0.6, farbe::SCHWARZ, TAlign::Mitte, true);
    }
    if let Some(kb) = l.laufkarte {
        white_box(&mut out, kb, 0.5);
        let fields = [("Meldergruppe", &info.meldergruppe), ("Melder", &info.melder), ("Meldertyp", &info.meldertyp), ("Geschoss / Raum", &info.raum)];
        let cw = kb.w() / fields.len() as f32;
        for (i, (k, v)) in fields.iter().enumerate() {
            let x0 = kb.min.x + i as f32 * cw;
            if i > 0 {
                line(&mut out, Pos::new(x0, kb.min.y), Pos::new(x0, kb.max.y), 0.3, farbe::SCHWARZ);
            }
            text(&mut out, Pos::new(x0 + 2.0 * u, kb.min.y + 1.5 * u), *k, 3.0 * u, farbe::SCHWARZ, TAlign::Links, false);
            let vs = if i < 2 { 11.0 * u } else { 5.5 * u };
            let v = if i == 3 && !info.geschoss.is_empty() { format!("{} / {}", info.geschoss, v) } else { (*v).clone() };
            text(&mut out, Pos::new(x0 + cw / 2.0, kb.min.y + 6.5 * u), v, vs.min(kb.h() * 0.55), farbe::ROT, TAlign::Mitte, true);
        }
    }
    if let Some(kb) = l.kopf {
        white_box(&mut out, kb, 0.4);
        schriftfeld_prims(&mut out, doc, kb, u);
    }
    if doc.frame == FrameKind::Feuerwehrplan || doc.frame == FrameKind::Laufkarte {
        frame_rect(&mut out, p.min, p.max, 0.5, farbe::SCHWARZ);
    } else {
        frame_rect(&mut out, p.min, p.max, 0.25, farbe::GRAU);
    }
    out
}
