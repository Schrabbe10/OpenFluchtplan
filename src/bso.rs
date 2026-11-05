//! Brandschutzordnung Teil A (Aushang) in Anlehnung an DIN 14096 – als bearbeitbarer Plan (A4 hoch).

use crate::model::*;

pub struct BsoInput {
    pub objekt: String,
    pub notruf: String,
    pub intern: String,
    pub sammelstelle: String,
    pub stand: String,
}

fn text(doc: &mut Doc, x: f32, y: f32, t: &str, size: f32, bold: bool, color: Color) {
    doc.add(
        ObjKind::Text { pos: Pos::new(x, y), text: t.into(), size, bold, italic: false, align: TAlign::Links, rot: 0.0 },
        Style::line(color, 0.2),
        5,
    );
}

fn band(doc: &mut Doc, y: f32, h: f32, title: &str, color: Color) {
    doc.add(ObjKind::Rect { a: Pos::new(12.0, y), b: Pos::new(198.0, y + h), radius: 0.0 }, Style::filled(color), 2);
    text(doc, 16.0, y + (h - 6.0) / 2.0, title, 6.0, true, farbe::WEISS);
}

fn sym(doc: &mut Doc, id: &str, x: f32, y: f32, s: f32) {
    doc.add(ObjKind::Symbol { id: id.into(), pos: Pos::new(x, y), size: s, rot: 0.0 }, Style::NONE, 4);
}

pub fn teil_a(inp: &BsoInput) -> Doc {
    let mut d = Doc { frame: FrameKind::Ohne, page: Page { format: PaperFormat::A4, landscape: false }, ..Doc::default() };
    d.info.objekt = inp.objekt.clone();
    d.info.notruf = inp.notruf.clone();
    d.info.stand = inp.stand.clone();
    d.add(ObjKind::Rect { a: Pos::new(8.0, 8.0), b: Pos::new(202.0, 289.0), radius: 0.0 }, Style::line(farbe::SCHWARZ, 0.5), 5);
    d.add(ObjKind::Rect { a: Pos::new(12.0, 12.0), b: Pos::new(198.0, 38.0), radius: 0.0 }, Style::filled(farbe::ROT), 2);
    text(&mut d, 18.0, 15.0, "Brandschutzordnung", 11.0, true, farbe::WEISS);
    text(&mut d, 18.0, 29.0, "nach DIN 14096 – Teil A", 5.0, false, farbe::WEISS);
    text(&mut d, 12.0, 42.0, &inp.objekt, 5.0, true, farbe::SCHWARZ);

    let mut y = 52.0;
    band(&mut d, y, 10.0, "Brände verhüten", farbe::ROT);
    y += 12.0;
    sym(&mut d, "P003", 26.0, y + 9.0, 16.0);
    sym(&mut d, "P002", 46.0, y + 9.0, 16.0);
    text(&mut d, 62.0, y + 6.0, "Feuer, offenes Licht und Rauchen verboten", 4.5, true, farbe::SCHWARZ);
    y += 22.0;

    band(&mut d, y, 10.0, "Verhalten im Brandfall", farbe::ROT);
    y += 12.0;
    text(&mut d, 62.0, y + 3.0, "Ruhe bewahren", 7.0, true, farbe::SCHWARZ);
    y += 14.0;

    band(&mut d, y, 10.0, "Brand melden", farbe::ROT);
    y += 12.0;
    sym(&mut d, "F005", 26.0, y + 9.0, 16.0);
    sym(&mut d, "F006", 46.0, y + 9.0, 16.0);
    text(&mut d, 62.0, y, "Brandmelder betätigen", 4.5, true, farbe::SCHWARZ);
    let notruf = if inp.intern.is_empty() { format!("Notruf {}", inp.notruf) } else { format!("Notruf {}  –  intern {}", inp.notruf, inp.intern) };
    text(&mut d, 62.0, y + 6.5, &notruf, 4.5, true, farbe::ROT);
    text(&mut d, 62.0, y + 13.0, "Wer meldet? Was ist passiert? Wie viele sind betroffen?\nWo ist es passiert? Warten auf Rückfragen!", 3.4, false, farbe::SCHWARZ);
    y += 26.0;

    band(&mut d, y, 10.0, "Alarmsignale und Anweisungen beachten", farbe::ROT);
    y += 14.0;

    band(&mut d, y, 10.0, "In Sicherheit bringen", farbe::GRUEN);
    y += 12.0;
    sym(&mut d, "E002", 26.0, y + 9.0, 16.0);
    sym(&mut d, "E007", 46.0, y + 9.0, 16.0);
    sym(&mut d, "P020", 36.0, y + 30.0, 14.0);
    let mut lines = vec![
        "Gefährdete Personen warnen",
        "Hilflose Personen mitnehmen",
        "Türen schließen",
        "Gekennzeichneten Fluchtwegen folgen",
        "Aufzug nicht benutzen",
    ];
    let sammel = if inp.sammelstelle.is_empty() { "Sammelstelle aufsuchen".to_string() } else { format!("Sammelstelle aufsuchen: {}", inp.sammelstelle) };
    lines.push("Auf Anweisungen achten");
    for (i, l) in lines.iter().enumerate() {
        text(&mut d, 62.0, y + i as f32 * 5.6, &format!("• {l}"), 4.0, false, farbe::SCHWARZ);
    }
    text(&mut d, 62.0, y + lines.len() as f32 * 5.6, &format!("• {sammel}"), 4.0, true, farbe::SCHWARZ);
    y += 44.0;

    band(&mut d, y, 10.0, "Löschversuch unternehmen", farbe::ROT);
    y += 12.0;
    sym(&mut d, "F001", 26.0, y + 9.0, 16.0);
    sym(&mut d, "F002", 46.0, y + 9.0, 16.0);
    text(&mut d, 62.0, y, "• Feuerlöscher benutzen", 4.0, false, farbe::SCHWARZ);
    text(&mut d, 62.0, y + 5.6, "• Wandhydrant benutzen", 4.0, false, farbe::SCHWARZ);
    text(&mut d, 62.0, y + 11.2, "• Nur ohne Eigengefährdung!", 4.0, true, farbe::SCHWARZ);

    text(&mut d, 12.0, 281.0, &format!("Stand: {}", inp.stand), 3.0, false, farbe::SCHWARZ);
    d.renumber();
    d
}
