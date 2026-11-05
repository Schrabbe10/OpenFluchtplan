//! Plan prüfen: Pflichtinhalte nach DIN ISO 23601 / ASR A2.3 (Flucht- und Rettungspläne) und
//! DIN 14095 (Feuerwehrpläne). Ersetzt keine fachliche Abnahme.

use crate::model::*;
use crate::templates;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stufe {
    Ok,
    Hinweis,
    Fehler,
}

pub type Befund = (Stufe, String);

fn walk<'a>(objs: &'a [Obj], out: &mut Vec<&'a ObjKind>) {
    for o in objs {
        if let ObjKind::Group { children } = &o.kind {
            walk(children, out);
        } else {
            out.push(&o.kind);
        }
    }
}

fn check(out: &mut Vec<Befund>, ok: bool, stufe_fehlt: Stufe, gut: &str, fehlt: &str) {
    out.push(if ok { (Stufe::Ok, gut.to_string()) } else { (stufe_fehlt, fehlt.to_string()) });
}

pub fn pruefen(doc: &Doc) -> Vec<Befund> {
    let mut kinds = Vec::new();
    walk(&doc.objects, &mut kinds);
    let symbols: Vec<(&str, f32)> = kinds
        .iter()
        .filter_map(|k| if let ObjKind::Symbol { id, size, .. } = k { Some((id.as_str(), *size)) } else { None })
        .collect();
    let has = |ids: &[&str]| symbols.iter().any(|(id, _)| ids.contains(id));
    let layout = templates::layout(doc);
    let legend = layout.legend.is_some() || kinds.iter().any(|k| matches!(k, ObjKind::Legend { .. }));
    let (w, h) = doc.page.size();
    let mut out = Vec::new();

    let fr = matches!(doc.frame, FrameKind::FrVollstaendig | FrameKind::FrZweizeilig | FrameKind::FrNurOben);
    let fw = doc.frame == FrameKind::Feuerwehrplan;
    let art = match doc.frame {
        FrameKind::FrVollstaendig | FrameKind::FrZweizeilig | FrameKind::FrNurOben => "Flucht- und Rettungsplan (DIN ISO 23601)",
        FrameKind::Feuerwehrplan => "Feuerwehrplan (DIN 14095)",
        FrameKind::Laufkarte => "Feuerwehr-Laufkarte (DIN 14675)",
        FrameKind::BrandschutzVollstaendig | FrameKind::BrandschutzNurOben => "Brandschutzplan",
        FrameKind::Ohne => "Plan ohne Rahmen",
    };
    out.push((Stufe::Hinweis, format!("Geprüft als: {art}")));

    // Allgemein
    check(&mut out, !doc.objects.is_empty(), Stufe::Fehler, "Plan enthält Elemente.", "Plan ist leer.");
    check(&mut out, !doc.info.stand.trim().is_empty(), Stufe::Fehler, "Stand (Datum) angegeben.", "Stand (Monat/Jahr) fehlt in den Planangaben.");
    check(&mut out, !doc.info.objekt.trim().is_empty() && doc.info.objekt != "Name des Gebäudes", Stufe::Fehler, "Objekt/Gebäude benannt.", "Objektname fehlt (Planangaben → Objekt).");
    check(&mut out, !doc.info.ersteller.trim().is_empty(), Stufe::Hinweis, "Ersteller angegeben.", "Ersteller fehlt in den Planangaben.");
    check(&mut out, doc.scale_n > 0.0, Stufe::Fehler, "Maßstab gesetzt.", "Maßstab fehlt.");
    let klein: Vec<_> = symbols.iter().filter(|(_, s)| *s < 7.0).collect();
    check(
        &mut out,
        klein.is_empty(),
        Stufe::Hinweis,
        "Alle Sicherheitszeichen mindestens 7 mm groß.",
        &format!("{} Sicherheitszeichen kleiner als 7 mm – auf Lesbarkeit achten.", klein.len()),
    );

    if fr {
        check(&mut out, w.min(h) >= 296.0, Stufe::Hinweis, "Format mindestens DIN A3.", "Format kleiner als DIN A3 – nur für einzelne Räume zulässig.");
        check(&mut out, has(&["STANDORT", "STANDORT_PFEIL"]), Stufe::Fehler, "Standort („Sie sind hier“) eingezeichnet.", "Standort-Symbol fehlt.");
        check(&mut out, kinds.iter().any(|k| matches!(k, ObjKind::Route { .. })), Stufe::Fehler, "Fluchtwege eingezeichnet.", "Keine Fluchtwege eingezeichnet.");
        check(&mut out, has(&["E001", "E002"]), Stufe::Fehler, "Notausgang gekennzeichnet.", "Kein Notausgang-Zeichen im Plan.");
        let sammel = has(&["E007", "E_SAMMEL_TEXT"]) || (layout.sammel.is_some() && !doc.info.sammelstelle.trim().is_empty());
        check(&mut out, sammel, Stufe::Fehler, "Sammelstelle angegeben.", "Sammelstelle fehlt (Symbol oder Feld „Sammelstelle“ ausfüllen).");
        check(&mut out, has(&["F001"]), Stufe::Hinweis, "Feuerlöscher eingezeichnet.", "Keine Feuerlöscher eingezeichnet.");
        check(&mut out, has(&["E003"]), Stufe::Hinweis, "Erste Hilfe eingezeichnet.", "Keine Erste-Hilfe-Einrichtung eingezeichnet.");
        check(&mut out, legend, Stufe::Fehler, "Legende vorhanden.", "Legende fehlt.");
        check(&mut out, layout.rules.is_some(), Stufe::Hinweis, "Verhaltensregeln vorhanden.", "Verhaltensregeln im Brandfall/bei Unfällen fehlen (Rahmen „vollständig“ wählen).");
        out.push((Stufe::Hinweis, "Nicht automatisch prüfbar: lagerichtige Ausrichtung zum Aushangort, Aushanghöhe, Aktualität.".into()));
    }
    if fw {
        check(&mut out, has(&["N1", "N2", "N3", "N4"]), Stufe::Fehler, "Nordpfeil vorhanden.", "Nordpfeil fehlt.");
        check(&mut out, legend, Stufe::Fehler, "Legende vorhanden.", "Legende fehlt.");
        check(&mut out, !doc.info.plannummer.trim().is_empty(), Stufe::Fehler, "Plannummer angegeben.", "Plannummer fehlt.");
        let uebersicht = doc.info.planart.to_lowercase().contains("übersicht");
        if uebersicht {
            check(&mut out, kinds.iter().any(|k| matches!(k, ObjKind::ScaleBar { .. })), Stufe::Hinweis, "Maßstabsleiste vorhanden.", "Maßstabsleiste fehlt.");
            check(&mut out, has(&["FW_SAMMELSTELLE", "E007"]), Stufe::Hinweis, "Sammelstelle eingezeichnet.", "Sammelstelle im Übersichtsplan fehlt.");
            check(&mut out, has(&["FW_HAUPTZUGANG", "FW_ZUGANG"]), Stufe::Fehler, "Feuerwehrzugang eingezeichnet.", "Hauptzugang für die Feuerwehr fehlt.");
            check(&mut out, has(&["FW_UEBERFLUR", "FW_UNTERFLUR", "FW_BRUNNEN", "FW_BEHAELTER", "FW_EINSPEISUNG"]), Stufe::Hinweis, "Löschwasserentnahme eingezeichnet.", "Keine Löschwasserentnahmestelle eingezeichnet.");
        } else {
            check(&mut out, kinds.iter().any(|k| matches!(k, ObjKind::FloorSection { .. })), Stufe::Fehler, "Geschoss-Schnitt vorhanden.", "Geschoss-Schnitt fehlt (Einfügen → Geschoss-Schnitt).");
            check(&mut out, !doc.info.geschoss.trim().is_empty(), Stufe::Fehler, "Geschoss angegeben.", "Geschoss fehlt in den Planangaben.");
            check(&mut out, has(&["FW_TREPPENRAUM"]), Stufe::Hinweis, "Treppenräume gekennzeichnet.", "Keine Treppenräume gekennzeichnet.");
        }
        check(&mut out, w.min(h) >= 296.0, Stufe::Hinweis, "Format mindestens DIN A3.", "Format kleiner als DIN A3 (DIN 14095 sieht A3 vor).");
        out.push((Stufe::Hinweis, "Örtliche Vorgaben der Brandschutzdienststelle zusätzlich beachten.".into()));
    }
    if doc.frame == FrameKind::Laufkarte {
        check(&mut out, !doc.info.meldergruppe.trim().is_empty(), Stufe::Fehler, "Meldergruppe angegeben.", "Meldergruppe fehlt.");
        check(&mut out, !doc.info.bmz_standort.trim().is_empty(), Stufe::Hinweis, "Standort BMZ angegeben.", "Standort der Brandmelderzentrale fehlt.");
        check(&mut out, kinds.iter().any(|k| matches!(k, ObjKind::Route { .. })), Stufe::Fehler, "Laufweg eingezeichnet.", "Laufweg fehlt.");
    }
    out
}
