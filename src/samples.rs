//! Musterpläne (eigene Beispiele) für Einstieg und Tests.

use crate::geom::*;
use crate::model::*;
use crate::{ops, presets};

struct B {
    doc: Doc,
}

impl B {
    fn new(frame: FrameKind, format: PaperFormat, landscape: bool) -> Self {
        let mut doc = Doc { frame, page: Page { format, landscape }, ..Doc::default() };
        doc.info.objekt = "Musterfirma GmbH – Verwaltung".into();
        doc.info.anschrift = "Musterstraße 1\n12345 Musterstadt".into();
        doc.info.ersteller = "Haustechnik".into();
        doc.info.stand = "11/2026".into();
        doc.info.plannummer = "001/1".into();
        doc.info.sammelstelle = "Parkplatz vor dem Haupteingang".into();
        Self { doc }
    }
    fn add(&mut self, kind: ObjKind, style: Style, layer: u32) {
        self.doc.add(kind, style, layer);
    }
    fn wall(&mut self, pts: &[(f32, f32)], closed: bool) {
        self.add(ObjKind::Wall { pts: p(pts), thickness: 1.4, closed }, Style::line(farbe::SCHWARZ, 1.4), 1);
    }
    fn inner_wall(&mut self, a: (f32, f32), b: (f32, f32)) {
        self.add(ObjKind::Wall { pts: p(&[a, b]), thickness: 0.7, closed: false }, Style::line(farbe::SCHWARZ, 0.7), 1);
    }
    fn room(&mut self, pts: &[(f32, f32)], name: &str) {
        self.add(ObjKind::Room { pts: p(pts), name: name.into(), show_area: false, size: 3.0 }, Style::NONE, 5);
    }
    fn area(&mut self, preset: &str, pts: &[(f32, f32)]) {
        let st = presets::find(preset).map(|p| p.style).unwrap_or_default();
        self.add(ObjKind::Area { pts: p(pts), preset: preset.into() }, st, 2);
    }
    fn route(&mut self, pts: &[(f32, f32)]) {
        self.add(ObjKind::Route { pts: p(pts), width: 7.0, curved: false, arrows: true }, Style::line(farbe::FLUCHT_HELL, 7.0), 3);
    }
    fn sym(&mut self, id: &str, x: f32, y: f32, size: f32, rot: f32) {
        self.add(ObjKind::Symbol { id: id.into(), pos: Pos::new(x, y), size, rot }, Style::NONE, 4);
    }
    fn door(&mut self, x: f32, y: f32, w: f32, rot: f32, mirror: bool) {
        self.add(ObjKind::Door { pos: Pos::new(x, y), width: w, rot, mirror, double: false }, Style::line(farbe::SCHWARZ, 0.35), 1);
    }
    fn text(&mut self, x: f32, y: f32, t: &str, size: f32) {
        self.add(ObjKind::Text { pos: Pos::new(x, y), text: t.into(), size, bold: false, italic: false, align: TAlign::Links, rot: 0.0 }, Style::line(farbe::SCHWARZ, 0.2), 5);
    }
    fn done(mut self) -> Doc {
        ops::sync_legend(&mut self.doc);
        self.doc
    }
}

fn p(v: &[(f32, f32)]) -> Vec<Pos> {
    v.iter().map(|&(x, y)| Pos::new(x, y)).collect()
}

/// Grundriss eines Bürogebäudes im Planbereich eines A3-quer-Rahmens.
fn buero_grundriss(b: &mut B, ox: f32, oy: f32) {
    let q = |x: f32, y: f32| (ox + x, oy + y);
    b.area("fluchtbereich", &[q(10.0, 70.0), q(260.0, 70.0), q(260.0, 90.0), q(10.0, 90.0)]);
    b.area("fluchtbereich", &[q(120.0, 90.0), q(140.0, 90.0), q(140.0, 170.0), q(120.0, 170.0)]);
    b.wall(&[q(0.0, 0.0), q(270.0, 0.0), q(270.0, 170.0), q(0.0, 170.0)], true);
    for x in [55.0, 110.0, 165.0, 220.0] {
        b.inner_wall(q(x, 0.0), q(x, 70.0));
    }
    b.inner_wall(q(0.0, 70.0), q(270.0, 70.0));
    b.inner_wall(q(0.0, 90.0), q(120.0, 90.0));
    b.inner_wall(q(140.0, 90.0), q(270.0, 90.0));
    b.inner_wall(q(120.0, 90.0), q(120.0, 170.0));
    b.inner_wall(q(140.0, 90.0), q(140.0, 170.0));
    b.inner_wall(q(60.0, 90.0), q(60.0, 170.0));
    b.inner_wall(q(200.0, 90.0), q(200.0, 170.0));
    let names = [("Büro 1", 27.0), ("Büro 2", 82.0), ("Besprechung", 137.0), ("Büro 3", 192.0), ("Archiv", 245.0)];
    for (n, x) in names {
        b.room(&[q(x - 25.0, 5.0), q(x + 25.0, 5.0), q(x + 25.0, 65.0), q(x - 25.0, 65.0)], n);
    }
    b.room(&[q(5.0, 95.0), q(55.0, 95.0), q(55.0, 165.0), q(5.0, 165.0)], "Küche");
    b.room(&[q(65.0, 95.0), q(115.0, 95.0), q(115.0, 165.0), q(65.0, 165.0)], "WC");
    b.room(&[q(145.0, 95.0), q(195.0, 95.0), q(195.0, 165.0), q(145.0, 165.0)], "Empfang");
    b.room(&[q(205.0, 95.0), q(265.0, 95.0), q(265.0, 165.0), q(205.0, 165.0)], "Lager");
    for x in [20.0, 75.0, 130.0, 185.0, 238.0] {
        b.door(ox + x, oy + 70.0, 9.0, 0.0, true);
    }
    b.door(ox + 125.0, oy + 170.0, 10.0, 0.0, false);
    b.door(ox + 262.0, oy + 70.0, 8.0, 90.0, false);
    b.add(ObjKind::Stairs { a: Pos::new(ox + 8.0, oy + 80.0), b: Pos::new(ox + 40.0, oy + 80.0), width: 14.0, steps: 10 }, Style::line(farbe::SCHWARZ, 0.3), 1);
    b.route(&[q(250.0, 80.0), q(130.0, 80.0), q(130.0, 178.0)]);
    b.route(&[q(50.0, 80.0), q(118.0, 80.0)]);
}

pub fn flucht_rettungsplan() -> Doc {
    let mut b = B::new(FrameKind::FrVollstaendig, PaperFormat::A3, true);
    let l = crate::templates::layout(&b.doc);
    let (ox, oy) = (l.plan.min.x + 12.0, l.plan.min.y + 12.0);
    buero_grundriss(&mut b, ox, oy);
    b.sym("STANDORT", ox + 160.0, oy + 80.0, 7.0, 0.0);
    b.sym("E002", ox + 140.0, oy + 184.0, 9.0, 0.0);
    b.sym("PFEIL_GRUEN", ox + 151.0, oy + 184.0, 9.0, 90.0);
    b.sym("F001", ox + 115.0, oy + 76.0, 8.0, 0.0);
    b.sym("F001", ox + 230.0, oy + 76.0, 8.0, 0.0);
    b.sym("F005", ox + 146.0, oy + 100.0, 8.0, 0.0);
    b.sym("E003", ox + 170.0, oy + 105.0, 8.0, 0.0);
    b.sym("E004", ox + 180.0, oy + 105.0, 8.0, 0.0);
    b.sym("F002", ox + 60.0, oy + 76.0, 8.0, 0.0);
    b.doc.info.geschoss = "Erdgeschoss".into();
    b.done()
}

pub fn flucht_rettungsplan_zweisprachig() -> Doc {
    let mut d = flucht_rettungsplan();
    d.frame = FrameKind::FrZweizeilig;
    d.legend.langs = vec![Lang::En];
    d
}

pub fn feuerwehr_uebersicht() -> Doc {
    let mut b = B::new(FrameKind::Feuerwehrplan, PaperFormat::A3, true);
    b.doc.info.planart = "Übersichtsplan".into();
    b.doc.info.plannummer = "1/4".into();
    b.doc.info.geschoss = "–".into();
    b.doc.scale_n = 500.0;
    let l = crate::templates::layout(&b.doc);
    let o = l.plan.min;
    let q = |x: f32, y: f32| (o.x + x, o.y + y);
    b.area("befahrbar", &[q(0.0, 20.0), q(400.0, 20.0), q(400.0, 45.0), q(0.0, 45.0)]);
    b.area("befahrbar", &[q(320.0, 45.0), q(345.0, 45.0), q(345.0, 220.0), q(320.0, 220.0)]);
    b.area("befahrbar", &[q(60.0, 45.0), q(320.0, 45.0), q(320.0, 70.0), q(60.0, 70.0)]);
    b.area("nicht_befahrbar", &[q(200.0, 150.0), q(310.0, 150.0), q(310.0, 200.0), q(200.0, 200.0)]);
    b.area("bezugsgebaeude", &[q(60.0, 80.0), q(190.0, 80.0), q(190.0, 200.0), q(60.0, 200.0)]);
    b.area("bezugsgebaeude", &[q(200.0, 80.0), q(310.0, 80.0), q(310.0, 140.0), q(200.0, 140.0)]);
    b.area("gefahr", &[q(205.0, 160.0), q(240.0, 160.0), q(240.0, 190.0), q(205.0, 190.0)]);
    b.area("loeschwasser", &[q(10.0, 120.0), q(45.0, 120.0), q(45.0, 160.0), q(10.0, 160.0)]);
    b.text(o.x + 100.0, o.y + 132.0, "Verwaltung\nE+2", 5.0);
    b.text(o.x + 235.0, o.y + 102.0, "Produktion\n-1+E+1", 5.0);
    b.text(o.x + 150.0, o.y + 27.0, "Musterstraße", 5.0);
    b.sym("FW_HAUPTZUGANG", o.x + 125.0, o.y + 76.0, 9.0, 0.0);
    b.sym("FW_ZUGANG", o.x + 255.0, o.y + 144.0, 6.0, 180.0);
    b.sym("FW_ZUFAHRT", o.x + 332.0, o.y + 10.0, 12.0, 90.0);
    b.sym("FW_UNTERFLUR", o.x + 50.0, o.y + 50.0, 8.0, 0.0);
    b.sym("FW_UEBERFLUR", o.x + 300.0, o.y + 52.0, 8.0, 0.0);
    b.sym("FW_FSD", o.x + 140.0, o.y + 88.0, 8.0, 0.0);
    b.sym("FW_BMZ", o.x + 150.0, o.y + 88.0, 8.0, 0.0);
    b.sym("FW_FAT", o.x + 160.0, o.y + 88.0, 8.0, 0.0);
    b.sym("FW_GEFAHRSTOFF", o.x + 222.0, o.y + 174.0, 9.0, 0.0);
    b.sym("E007", o.x + 25.0, o.y + 180.0, 10.0, 0.0);
    b.sym("N1", o.x + 375.0, o.y + 80.0, 18.0, 20.0);
    b.add(ObjKind::ScaleBar { pos: Pos::new(o.x + 350.0, o.y + 200.0), meters: 20.0 }, Style::default(), 5);
    b.done()
}

pub fn feuerwehr_geschoss() -> Doc {
    let mut b = B::new(FrameKind::Feuerwehrplan, PaperFormat::A3, true);
    b.doc.info.planart = "Geschossplan".into();
    b.doc.info.plannummer = "2/4".into();
    let l = crate::templates::layout(&b.doc);
    let (ox, oy) = (l.plan.min.x + 40.0, l.plan.min.y + 20.0);
    buero_grundriss(&mut b, ox, oy);
    b.area("gefahr", &[(ox + 222.0, oy + 92.0), (ox + 268.0, oy + 92.0), (ox + 268.0, oy + 168.0), (ox + 222.0, oy + 168.0)]);
    b.sym("FW_TREPPENRAUM", ox + 24.0, oy + 100.0, 8.0, 0.0);
    b.sym("FW_WANDHYDRANT", ox + 60.0, oy + 78.0, 7.0, 0.0);
    b.sym("FW_HAUPTSCHALTER", ox + 210.0, oy + 100.0, 7.0, 0.0);
    b.sym("FW_GEFAHRSTOFF", ox + 245.0, oy + 130.0, 8.0, 0.0);
    b.sym("FW_HAUPTZUGANG", ox + 130.0, oy + 182.0, 9.0, 0.0);
    b.sym("FW_INFO", ox + 160.0, oy + 110.0, 7.0, 0.0);
    b.sym("FW_RWA", ox + 100.0, oy + 78.0, 7.0, 0.0);
    b.sym("N1", l.plan.max.x - 20.0, l.plan.min.y + 25.0, 16.0, 20.0);
    b.add(ObjKind::Grid { a: Pos::new(ox - 10.0, oy - 8.0), b: Pos::new(ox + 280.0, oy + 190.0), nx: 6, ny: 4 }, Style::line(Color::rgb(0x70, 0x70, 0x70), 0.2), 0);
    let (sx, sy) = (l.plan.max.x - 32.0, l.plan.max.y - 42.0);
    b.add(
        ObjKind::FloorSection { a: Pos::new(sx, sy), b: Pos::new(sx + 26.0, sy + 32.0), floors: ["1. OG", "EG", "KG"].map(String::from).to_vec(), current: 1 },
        Style::default(),
        5,
    );
    b.done()
}

pub fn brandschutzplan() -> Doc {
    let mut b = B::new(FrameKind::BrandschutzVollstaendig, PaperFormat::A3, true);
    b.doc.frame_cfg.title = "BRANDSCHUTZPLAN".into();
    b.doc.frame_cfg.color = farbe::ROT;
    let l = crate::templates::layout(&b.doc);
    let (ox, oy) = (l.plan.min.x + 12.0, l.plan.min.y + 12.0);
    let q = |x: f32, y: f32| (ox + x, oy + y);
    b.area("notw_flur", &[q(0.0, 70.0), q(270.0, 70.0), q(270.0, 90.0), q(0.0, 90.0)]);
    b.area("notw_treppe", &[q(0.0, 90.0), q(40.0, 90.0), q(40.0, 130.0), q(0.0, 130.0)]);
    b.area("rei90m", &[q(108.0, 0.0), q(112.0, 0.0), q(112.0, 170.0), q(108.0, 170.0)]);
    b.area("rei30", &[q(0.0, 68.0), q(108.0, 68.0), q(108.0, 70.0), q(0.0, 70.0)]);
    b.area("rei30", &[q(112.0, 68.0), q(270.0, 68.0), q(270.0, 70.0), q(112.0, 70.0)]);
    b.area("rei90", &[q(40.0, 90.0), q(42.0, 90.0), q(42.0, 130.0), q(40.0, 130.0)]);
    b.area("rei60a", &[q(200.0, 92.0), q(270.0, 92.0), q(270.0, 170.0), q(200.0, 170.0)]);
    b.area("brandabschnitt", &[q(-4.0, -4.0), q(110.0, -4.0), q(110.0, 174.0), q(-4.0, 174.0)]);
    b.wall(&[q(0.0, 0.0), q(270.0, 0.0), q(270.0, 170.0), q(0.0, 170.0)], true);
    b.sym("FW_T30", ox + 60.0, oy + 80.0, 7.0, 0.0);
    b.sym("FW_RS", ox + 160.0, oy + 80.0, 7.0, 0.0);
    b.sym("FW_T90", ox + 110.0, oy + 80.0, 7.0, 0.0);
    b.done()
}

pub fn laufkarte() -> Doc {
    let mut b = B::new(FrameKind::Laufkarte, PaperFormat::A4, true);
    b.doc.frame_cfg.title = "FEUERWEHR-LAUFKARTE".into();
    b.doc.frame_cfg.color = farbe::ROT;
    b.doc.info.meldergruppe = "12".into();
    b.doc.info.melder = "3".into();
    b.doc.info.meldertyp = "Rauchmelder".into();
    b.doc.info.raum = "Archiv".into();
    b.doc.info.bmz_standort = "Haupteingang, Empfang".into();
    let l = crate::templates::layout(&b.doc);
    let s = ((l.plan.w() - 20.0) / 270.0).min((l.plan.h() - 10.0) / 170.0);
    let (ox, oy) = (l.plan.min.x + 10.0, l.plan.min.y + 5.0);
    let q = |x: f32, y: f32| (ox + x * s, oy + y * s);
    b.wall(&[q(0.0, 0.0), q(270.0, 0.0), q(270.0, 170.0), q(0.0, 170.0)], true);
    b.inner_wall(q(0.0, 70.0), q(270.0, 70.0));
    b.inner_wall(q(220.0, 0.0), q(220.0, 70.0));
    b.inner_wall(q(140.0, 90.0), q(140.0, 170.0));
    b.inner_wall(q(0.0, 90.0), q(120.0, 90.0));
    b.inner_wall(q(140.0, 90.0), q(270.0, 90.0));
    b.add(
        ObjKind::Route { pts: p(&[q(130.0, 165.0), q(130.0, 80.0), q(245.0, 80.0), q(245.0, 40.0)]), width: 2.5, curved: false, arrows: true },
        Style { stroke: Some(StrokeStyle { color: farbe::ROT, width: 2.5, dash: Dash::Durchgehend }), fill: Fill::Keine },
        3,
    );
    b.sym("BMT_START", ox + 130.0 * s, oy + 165.0 * s, 7.0, 0.0);
    b.sym("BMT_RAUCH", ox + 245.0 * s, oy + 30.0 * s, 7.0, 0.0);
    b.sym("FW_BMZ", ox + 150.0 * s, oy + 160.0 * s, 6.0, 0.0);
    b.room(&[q(220.0, 0.0), q(270.0, 0.0), q(270.0, 70.0), q(220.0, 70.0)], "Archiv");
    b.done()
}

pub fn all() -> Vec<(&'static str, Doc)> {
    vec![
        ("Flucht- und Rettungsplan (A3 quer)", flucht_rettungsplan()),
        ("Flucht- und Rettungsplan zweisprachig", flucht_rettungsplan_zweisprachig()),
        ("Feuerwehrplan – Übersichtsplan", feuerwehr_uebersicht()),
        ("Feuerwehrplan – Geschossplan", feuerwehr_geschoss()),
        ("Brandschutzplan", brandschutzplan()),
        ("Feuerwehr-Laufkarte", laufkarte()),
    ]
}
