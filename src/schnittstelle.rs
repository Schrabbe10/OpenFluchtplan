//! Schnittstelle für Automatisierung und KI-Assistenten:
//! - `OpenFluchtplan.exe --mcp`: MCP-Server (Model Context Protocol, JSON-RPC über stdin/stdout)
//! - `OpenFluchtplan.exe --cli <werkzeug> schluessel=wert …`: dieselben Werkzeuge auf der Kommandozeile
//!
//! Alle Werkzeuge arbeiten auf `.fplan`-Dateien. Ein geöffnetes Programmfenster lädt geänderte
//! Dateien automatisch neu. Koordinaten: Millimeter auf dem Blatt, Ursprung oben links, y nach unten.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde_json::{Value, json};

use crate::model::*;
use crate::{export, ops, pdfvec, pruefung, samples, symbols, templates};

pub enum Inhalt {
    Text(String),
    Bild(Vec<u8>),
}

type Erg = Result<Vec<Inhalt>, String>;

fn text(s: impl Into<String>) -> Erg {
    Ok(vec![Inhalt::Text(s.into())])
}

fn json_text(v: &Value) -> Erg {
    text(serde_json::to_string_pretty(v).unwrap_or_default())
}

// ---------- Argumente ----------

fn s(a: &Value, k: &str) -> Result<String, String> {
    a.get(k).and_then(|v| v.as_str()).map(String::from).ok_or_else(|| format!("Parameter „{k}“ fehlt"))
}
fn s_opt(a: &Value, k: &str) -> Option<String> {
    a.get(k).and_then(|v| v.as_str()).map(String::from)
}
fn f(a: &Value, k: &str) -> Result<f32, String> {
    a.get(k).and_then(|v| v.as_f64()).map(|v| v as f32).ok_or_else(|| format!("Zahl „{k}“ fehlt"))
}
fn f_opt(a: &Value, k: &str) -> Option<f32> {
    a.get(k).and_then(|v| v.as_f64()).map(|v| v as f32)
}
fn b_opt(a: &Value, k: &str) -> Option<bool> {
    a.get(k).and_then(|v| v.as_bool())
}
fn ids(a: &Value) -> Result<Vec<u64>, String> {
    a.get("ids").and_then(|v| v.as_array()).map(|v| v.iter().filter_map(|x| x.as_u64()).collect()).ok_or_else(|| "Liste „ids“ fehlt".into())
}
fn punkte(a: &Value, k: &str) -> Result<Vec<Pos>, String> {
    let arr = a.get(k).and_then(|v| v.as_array()).ok_or_else(|| format!("Punktliste „{k}“ fehlt"))?;
    arr.iter()
        .map(|p| {
            let p = p.as_array().ok_or("Punkt muss [x, y] sein")?;
            Ok(Pos::new(p.first().and_then(|v| v.as_f64()).ok_or("x fehlt")? as f32, p.get(1).and_then(|v| v.as_f64()).ok_or("y fehlt")? as f32))
        })
        .collect()
}

// ---------- Dateien ----------

pub fn laden(p: &str) -> Result<Doc, String> {
    let s = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
    serde_json::from_str(&s).map_err(|e| format!("{p}: keine gültige Plandatei ({e})"))
}

/// Atomar speichern (temporäre Datei + Umbenennen), damit ein geöffnetes Programm nie eine halbe Datei liest.
pub fn speichern(p: &str, doc: &Doc) -> Result<(), String> {
    let json = serde_json::to_string(doc).map_err(|e| e.to_string())?;
    let tmp = PathBuf::from(format!("{p}.tmp"));
    std::fs::write(&tmp, json).map_err(|e| format!("{p}: {e}"))?;
    std::fs::rename(&tmp, p).map_err(|e| format!("{p}: {e}"))
}

fn aendern(a: &Value, f: impl FnOnce(&mut Doc) -> Result<String, String>) -> Erg {
    let p = s(a, "pfad")?;
    let mut doc = laden(&p)?;
    let msg = f(&mut doc)?;
    ops::sync_legend(&mut doc);
    speichern(&p, &doc)?;
    text(msg)
}

fn r1(v: f32) -> f64 {
    (v as f64 * 10.0).round() / 10.0
}

fn layer_name(doc: &Doc, id: u32) -> String {
    doc.layers.iter().find(|l| l.id == id).map(|l| l.name.clone()).unwrap_or_default()
}

/// Kurzbeschreibung eines Objekts für Listen.
fn kurz(doc: &Doc, o: &Obj) -> Value {
    let b = ops::bounds_doc(doc, &o.kind);
    let mut v = json!({
        "id": o.id,
        "art": ops::kind_name(&o.kind),
        "ebene": layer_name(doc, o.layer),
        "bereich": [r1(b.min.x), r1(b.min.y), r1(b.max.x), r1(b.max.y)],
    });
    match &o.kind {
        ObjKind::Text { text, size, .. } => {
            v["text"] = json!(text);
            v["groesse"] = json!(r1(*size));
        }
        ObjKind::Symbol { id, .. } => {
            v["symbol"] = json!(id);
            v["name"] = json!(symbols::find(id).map(|s| s.name).unwrap_or("eigenes Symbol"));
        }
        ObjKind::Room { name, .. } => v["text"] = json!(name),
        ObjKind::Note { text, .. } => v["text"] = json!(text),
        ObjKind::Area { preset, .. } => v["vorlage"] = json!(preset),
        ObjKind::Group { children } => v["anzahl"] = json!(children.len()),
        _ => {}
    }
    if let Some(st) = &o.style.stroke {
        v["farbe"] = json!(st.color.hex());
    } else if let Fill::Farbe(c) = o.style.fill {
        v["farbe"] = json!(c.hex());
    }
    v
}

fn neu_doc(vorlage: &str) -> Result<Doc, String> {
    let (frame, format, landscape) = match vorlage {
        "fr_a3" | "" => (FrameKind::FrVollstaendig, PaperFormat::A3, true),
        "fr_a3_hoch" => (FrameKind::FrVollstaendig, PaperFormat::A3, false),
        "fr_a4" => (FrameKind::FrVollstaendig, PaperFormat::A4, true),
        "fr_zweisprachig" => (FrameKind::FrZweizeilig, PaperFormat::A3, true),
        "feuerwehrplan" => (FrameKind::Feuerwehrplan, PaperFormat::A3, true),
        "brandschutzplan" => (FrameKind::BrandschutzVollstaendig, PaperFormat::A3, true),
        "laufkarte" => (FrameKind::Laufkarte, PaperFormat::A4, true),
        "leer" => (FrameKind::Ohne, PaperFormat::A3, true),
        v => {
            // Musterplan nach Name
            return samples::all().into_iter().find(|(n, _)| n.to_lowercase().contains(&v.to_lowercase())).map(|(_, d)| d).ok_or_else(|| format!("Unbekannte Vorlage „{v}“"));
        }
    };
    let mut doc = Doc { frame, page: Page { format, landscape }, ..Doc::default() };
    match frame {
        FrameKind::BrandschutzVollstaendig => {
            doc.frame_cfg.title = "BRANDSCHUTZPLAN".into();
            doc.frame_cfg.color = farbe::ROT;
        }
        FrameKind::Laufkarte => {
            doc.frame_cfg.title = "FEUERWEHR-LAUFKARTE".into();
            doc.frame_cfg.color = farbe::ROT;
        }
        FrameKind::Feuerwehrplan => doc.info.planart = "Geschossplan".into(),
        _ => {}
    }
    Ok(doc)
}

fn auto_layer(doc: &mut Doc, kind: &ObjKind) -> u32 {
    let name = match kind {
        ObjKind::Wall { .. } | ObjKind::Door { .. } | ObjKind::Window { .. } | ObjKind::Stairs { .. } => "Wände",
        ObjKind::Area { .. } => "Flächen",
        ObjKind::Route { .. } => "Fluchtwege",
        ObjKind::Symbol { .. } => "Symbole",
        _ => "Beschriftung",
    };
    doc.find_layer_by_name(name)
}

fn stamp(name: &str) -> export::Stamp {
    match name.to_lowercase().as_str() {
        "entwurf" => export::Stamp::Entwurf,
        "muster" => export::Stamp::Muster,
        "vorabzug" => export::Stamp::Vorabzug,
        _ => export::Stamp::Keiner,
    }
}

// ---------- Werkzeuge ----------

/// Beschreibung aller Werkzeuge (Name, Beschreibung, JSON-Schema der Parameter).
pub fn werkzeuge() -> Vec<Value> {
    let pfad = json!({"type": "string", "description": "Pfad zur .fplan-Datei"});
    let num = |d: &str| json!({"type": "number", "description": d});
    let st = |d: &str| json!({"type": "string", "description": d});
    let tool = |name: &str, desc: &str, props: Value, req: &[&str]| json!({"name": name, "description": desc, "inputSchema": {"type": "object", "properties": props, "required": req}});
    vec![
        tool("plan_neu", "Neuen Plan anlegen. vorlage: fr_a3 (Flucht- und Rettungsplan A3 quer), fr_a3_hoch, fr_a4, fr_zweisprachig, feuerwehrplan, brandschutzplan, laufkarte, leer – oder Teil eines Musterplan-Namens.", json!({"ziel": st("Pfad der neuen .fplan-Datei"), "vorlage": st("Vorlage")}), &["ziel"]),
        tool("plan_info", "Überblick über einen Plan: Planangaben, Format, Rahmen, Maßstab, Planfläche (mm), Ebenen, Anzahl Objekte je Art.", json!({"pfad": pfad}), &["pfad"]),
        tool(
            "objekte_auflisten",
            "Objekte eines Plans auflisten (ID, Art, Ebene, Bereich [x0,y0,x1,y1] in Blatt-mm, Text/Symbol). Filter optional. Koordinaten: mm auf dem Blatt, Ursprung oben links, y nach unten.",
            json!({"pfad": pfad, "art": st("Objektart, z. B. Text, Symbol, Linie, Polygon, Wand, Fluchtweg, Gruppe"), "text": st("Text enthält (Groß/Klein egal)"), "ebene": st("Ebenenname"), "bereich": {"type": "array", "items": {"type": "number"}, "description": "[x0, y0, x1, y1] – nur Objekte, die diesen Bereich berühren"}, "max": num("Höchstzahl (Standard 100)"), "ab": num("Überspringen (Blättern)")}),
            &["pfad"],
        ),
        tool("objekt_lesen", "Ein Objekt vollständig als JSON lesen (Grundlage für objekt_schreiben).", json!({"pfad": pfad, "id": num("Objekt-ID")}), &["pfad", "id"]),
        tool(
            "objekt_schreiben",
            "Objekt als JSON schreiben (Format wie objekt_lesen). Mit vorhandener id wird das Objekt ersetzt, mit id 0 oder ohne id neu angelegt. Erlaubt jede Änderung an Geometrie, Stil, Ebene.",
            json!({"pfad": pfad, "objekt": {"type": "object", "description": "Objekt-JSON"}}),
            &["pfad", "objekt"],
        ),
        tool("objekte_loeschen", "Objekte löschen.", json!({"pfad": pfad, "ids": {"type": "array", "items": {"type": "number"}}}), &["pfad", "ids"]),
        tool("objekte_verschieben", "Objekte um dx/dy (mm) verschieben.", json!({"pfad": pfad, "ids": {"type": "array", "items": {"type": "number"}}, "dx": num("mm"), "dy": num("mm")}), &["pfad", "ids", "dx", "dy"]),
        tool("objekte_gruppierung_aufheben", "Gruppen auflösen (z. B. nach PDF-Import mit Gruppierung). Ohne ids werden alle Gruppen aufgelöst.", json!({"pfad": pfad, "ids": {"type": "array", "items": {"type": "number"}}}), &["pfad"]),
        tool(
            "symbole_zusammenfassen",
            "Kleine übereinanderliegende Objekte (Einzelteile importierter Symbole: Hintergrund, Rahmen, Zeichen, Text) zu je einer Gruppe zusammenfassen, damit sie sich als Ganzes verschieben lassen. Ohne ids: ganzer Plan.",
            json!({"pfad": pfad, "ids": {"type": "array", "items": {"type": "number"}}, "max_mm": num("größte Symbolgröße in mm (Standard 12)")}),
            &["pfad"],
        ),
        tool(
            "planangaben_setzen",
            "Planangaben (Plankopf) setzen. Felder: objekt, anschrift, geschoss, auftraggeber, ersteller, plannummer, planart, stand, revision, sonstiges, notruf, sammelstelle, meldergruppe, melder, meldertyp, raum, bmz_standort. Zusätzlich massstab (Zahl N für 1:N).",
            json!({"pfad": pfad, "felder": {"type": "object"}}),
            &["pfad", "felder"],
        ),
        tool("text_ersetzen", "Text in allen Textobjekten, Raumnamen, Notizen und Tabellen ersetzen.", json!({"pfad": pfad, "suchen": st("Suchtext"), "ersetzen": st("Ersatz"), "gross_klein": {"type": "boolean"}}), &["pfad", "suchen", "ersetzen"]),
        tool("text_einfuegen", "Text einfügen. Position = linke Oberkante in Blatt-mm.", json!({"pfad": pfad, "x": num("mm"), "y": num("mm"), "text": st("Text (Zeilenumbruch mit \\n)"), "groesse": num("Schrifthöhe mm (Standard 3.5)"), "fett": {"type": "boolean"}, "farbe": st("#rrggbb"), "drehung": num("Grad")}), &["pfad", "x", "y", "text"]),
        tool("symbol_suchen", "Symbole der Bibliothek suchen (Name oder ID, z. B. Feuerlöscher, Notausgang, F001, E002). Ohne Suchbegriff: Kategorien.", json!({"suchbegriff": st("Suchbegriff")}), &[]),
        tool("symbol_einfuegen", "Sicherheitszeichen/Symbol einfügen (Mittelpunkt in Blatt-mm). Die Legende wird automatisch ergänzt.", json!({"pfad": pfad, "symbol": st("Symbol-ID, z. B. F001"), "x": num("mm"), "y": num("mm"), "groesse": num("Kantenlänge mm (Standard 8)"), "drehung": num("Grad")}), &["pfad", "symbol", "x", "y"]),
        tool("fluchtweg_einfuegen", "Fluchtweg (grün mit Pfeilen) oder Laufweg der Feuerwehr (rot) als Linienzug einfügen.", json!({"pfad": pfad, "punkte": {"type": "array", "items": {"type": "array", "items": {"type": "number"}}, "description": "[[x,y], …] in Blatt-mm"}, "breite": num("mm (Standard 7)"), "art": st("fluchtweg | fluchtweg_bogen | laufweg")}), &["pfad", "punkte"]),
        tool("plan_pruefen", "Pflichtinhalte nach DIN ISO 23601 / DIN 14095 prüfen (Standort, Fluchtwege, Legende, Stand …).", json!({"pfad": pfad}), &["pfad"]),
        tool("vorschau", "Plan als Bild ansehen (PNG). Optional nur ein Bereich [x0,y0,x1,y1] in Blatt-mm, dann in höherer Auflösung.", json!({"pfad": pfad, "dpi": num("Auflösung (Standard 50, mit Bereich 150)"), "bereich": {"type": "array", "items": {"type": "number"}}, "datei": st("optional: PNG zusätzlich hier speichern")}), &["pfad"]),
        tool("exportieren", "Plan ausgeben. Format nach Dateiendung: .pdf, .png, .svg, .dxf.", json!({"pfad": pfad, "ziel": st("Zieldatei"), "stempel": st("entwurf | muster | vorabzug"), "dpi": num("für PNG (Standard 200)")}), &["pfad", "ziel"]),
        tool(
            "pdf_importieren",
            "PDF-Seite (z. B. vorhandener Flucht- und Rettungsplan) als bearbeitbare Vektorgrafik in einen Plan übernehmen. modus: formatfuellend (Blattformat übernehmen, Rahmen aus – zum Weiterbearbeiten fertiger Pläne), einpassen, original. Ist ziel nicht vorhanden, wird ein neuer Plan angelegt.",
            json!({"pdf": st("PDF-Datei"), "ziel": st(".fplan-Datei"), "seite": num("Seite ab 1 (Standard 1)"), "modus": st("formatfuellend | einpassen | original"), "gruppieren": {"type": "boolean", "description": "alles in eine Gruppe, Standard false"}, "symbole_erkennen": {"type": "boolean", "description": "Einzelteile von Symbolen zu Gruppen zusammenfassen, Standard true"}, "texte_prozent": num("Textgröße in % (Standard 100)"), "als_bild": {"type": "boolean"}}),
            &["pdf", "ziel"],
        ),
    ]
}

pub fn ausfuehren(name: &str, a: &Value) -> Erg {
    match name {
        "plan_neu" => {
            let ziel = s(a, "ziel")?;
            let doc = neu_doc(&s_opt(a, "vorlage").unwrap_or_default())?;
            speichern(&ziel, &doc)?;
            text(format!("Plan angelegt: {ziel}"))
        }
        "plan_info" => {
            let doc = laden(&s(a, "pfad")?)?;
            let mut arten: std::collections::BTreeMap<&str, usize> = Default::default();
            for o in &doc.objects {
                *arten.entry(ops::kind_name(&o.kind)).or_default() += 1;
            }
            let (w, h) = doc.page.size();
            let plan = templates::layout(&doc).plan;
            json_text(&json!({
                "planangaben": doc.info,
                "format": doc.page.format.name(), "breite_mm": w, "hoehe_mm": h,
                "rahmen": format!("{:?}", doc.frame),
                "massstab": format!("1:{}", doc.scale_n),
                "planflaeche_mm": [r1(plan.min.x), r1(plan.min.y), r1(plan.max.x), r1(plan.max.y)],
                "ebenen": doc.layers.iter().map(|l| json!({"name": l.name, "sichtbar": l.visible, "gesperrt": l.locked, "objekte": doc.objects.iter().filter(|o| o.layer == l.id).count()})).collect::<Vec<_>>(),
                "objekte": doc.objects.len(),
                "objektarten": arten,
            }))
        }
        "objekte_auflisten" => {
            let doc = laden(&s(a, "pfad")?)?;
            let art = s_opt(a, "art").map(|x| x.to_lowercase());
            let txt = s_opt(a, "text").map(|x| x.to_lowercase());
            let ebene = s_opt(a, "ebene");
            let ber = a.get("bereich").and_then(|v| v.as_array()).filter(|v| v.len() == 4).map(|v| {
                let n: Vec<f32> = v.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect();
                crate::geom::BBox::from_two(Pos::new(n[0], n[1]), Pos::new(n[2], n[3]))
            });
            let max = f_opt(a, "max").unwrap_or(100.0) as usize;
            let ab = f_opt(a, "ab").unwrap_or(0.0) as usize;
            let treffer: Vec<Value> = doc
                .objects
                .iter()
                .filter(|o| art.as_ref().is_none_or(|x| ops::kind_name(&o.kind).to_lowercase().contains(x.as_str())))
                .filter(|o| ebene.as_ref().is_none_or(|e| &layer_name(&doc, o.layer) == e))
                .filter(|o| ber.is_none_or(|b| ops::bounds_doc(&doc, &o.kind).intersects(&b)))
                .map(|o| kurz(&doc, o))
                .filter(|v| txt.as_ref().is_none_or(|t| v.get("text").and_then(|x| x.as_str()).is_some_and(|x| x.to_lowercase().contains(t.as_str()))))
                .collect();
            let gesamt = treffer.len();
            json_text(&json!({"gesamt": gesamt, "ab": ab, "objekte": treffer.into_iter().skip(ab).take(max).collect::<Vec<_>>()}))
        }
        "objekt_lesen" => {
            let doc = laden(&s(a, "pfad")?)?;
            let id = f(a, "id")? as u64;
            let o = doc.objects.iter().find(|o| o.id == id).ok_or(format!("Objekt {id} nicht gefunden"))?;
            json_text(&serde_json::to_value(o).map_err(|e| e.to_string())?)
        }
        "objekt_schreiben" => aendern(a, |doc| {
            let mut obj: Obj = serde_json::from_value(a.get("objekt").cloned().ok_or("objekt fehlt")?).map_err(|e| format!("Objekt-JSON ungültig: {e}"))?;
            if let Some(i) = doc.objects.iter().position(|o| o.id == obj.id && obj.id != 0) {
                doc.objects[i] = obj;
                Ok(format!("Objekt {} ersetzt.", doc.objects[i].id))
            } else {
                obj.id = doc.alloc_id();
                if !doc.layers.iter().any(|l| l.id == obj.layer) {
                    obj.layer = auto_layer(doc, &obj.kind);
                }
                let id = obj.id;
                doc.objects.push(obj);
                Ok(format!("Objekt {id} angelegt."))
            }
        }),
        "objekte_loeschen" => {
            let ids = ids(a)?;
            aendern(a, |doc| {
                let n = doc.objects.len();
                doc.objects.retain(|o| !ids.contains(&o.id));
                Ok(format!("{} Objekt(e) gelöscht.", n - doc.objects.len()))
            })
        }
        "objekte_verschieben" => {
            let ids = ids(a)?;
            let d = Pos::new(f(a, "dx")?, f(a, "dy")?);
            aendern(a, |doc| {
                let mut n = 0;
                for o in doc.objects.iter_mut().filter(|o| ids.contains(&o.id)) {
                    ops::translate(&mut o.kind, d);
                    n += 1;
                }
                Ok(format!("{n} Objekt(e) verschoben."))
            })
        }
        "objekte_gruppierung_aufheben" => {
            let only = ids(a).ok();
            aendern(a, |doc| {
                let mut out = Vec::with_capacity(doc.objects.len());
                let mut n = 0;
                for o in std::mem::take(&mut doc.objects) {
                    match o.kind {
                        ObjKind::Group { children } if only.as_ref().is_none_or(|v| v.contains(&o.id)) => {
                            n += 1;
                            out.extend(children);
                        }
                        kind => out.push(Obj { kind, ..o }),
                    }
                }
                doc.objects = out;
                Ok(format!("{n} Gruppe(n) aufgelöst, jetzt {} Objekte.", doc.objects.len()))
            })
        }
        "planangaben_setzen" => aendern(a, |doc| {
            // Felder als Objekt „felder“ oder direkt als Parameter (bequemer auf der Kommandozeile)
            let direkt: serde_json::Map<String, Value> = a.as_object().map(|m| m.iter().filter(|(k, _)| *k != "pfad" && *k != "felder").map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
            let felder = a.get("felder").and_then(|v| v.as_object()).unwrap_or(&direkt);
            if felder.is_empty() {
                return Err("Keine Felder angegeben (z. B. stand=11/2026).".into());
            }
            let mut info = serde_json::to_value(&doc.info).map_err(|e| e.to_string())?;
            let mut gesetzt = Vec::new();
            for (k, v) in felder {
                if k == "massstab" {
                    doc.scale_n = v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim_start_matches("1:").parse().ok())).ok_or("massstab muss eine Zahl sein")? as f32;
                    gesetzt.push(k.clone());
                    continue;
                }
                if info.get(k).is_none() {
                    return Err(format!("Unbekanntes Feld „{k}“"));
                }
                info[k] = json!(v.as_str().map(String::from).unwrap_or_else(|| v.to_string()));
                gesetzt.push(k.clone());
            }
            doc.info = serde_json::from_value(info).map_err(|e| e.to_string())?;
            Ok(format!("Gesetzt: {}", gesetzt.join(", ")))
        }),
        "text_ersetzen" => {
            let (such, ers) = (s(a, "suchen")?, s(a, "ersetzen")?);
            let case = b_opt(a, "gross_klein").unwrap_or(false);
            aendern(a, |doc| {
                let n = ops::replace_text(&mut doc.objects, &such, &ers, case, true);
                Ok(format!("{n} Stelle(n) ersetzt."))
            })
        }
        "text_einfuegen" => aendern(a, |doc| {
            let color = s_opt(a, "farbe").and_then(|c| hex(&c)).unwrap_or(farbe::SCHWARZ);
            let kind = ObjKind::Text {
                pos: Pos::new(f(a, "x")?, f(a, "y")?),
                text: s(a, "text")?,
                size: f_opt(a, "groesse").unwrap_or(3.5),
                bold: b_opt(a, "fett").unwrap_or(false),
                italic: false,
                align: TAlign::Links,
                rot: f_opt(a, "drehung").unwrap_or(0.0),
            };
            let layer = auto_layer(doc, &kind);
            let id = doc.add(kind, Style::line(color, 0.2), layer);
            Ok(format!("Text angelegt, ID {id}."))
        }),
        "symbol_suchen" => {
            let q = s_opt(a, "suchbegriff").unwrap_or_default().to_lowercase();
            if q.is_empty() {
                return json_text(&json!(symbols::Category::ALL.iter().map(|c| json!({"kategorie": c.name(), "symbole": symbols::library().iter().filter(|s| s.cat == *c).count()})).collect::<Vec<_>>()));
            }
            let hits: Vec<Value> = symbols::library()
                .iter()
                .filter(|s| s.name.to_lowercase().contains(&q) || s.id.to_lowercase().contains(&q) || s.cat.name().to_lowercase().contains(&q))
                .take(60)
                .map(|s| json!({"id": s.id, "name": s.name, "kategorie": s.cat.name()}))
                .collect();
            json_text(&json!(hits))
        }
        "symbol_einfuegen" => aendern(a, |doc| {
            let id = s(a, "symbol")?;
            if symbols::find(&id).is_none() && !doc.custom_symbols.iter().any(|c| c.id == id) {
                return Err(format!("Symbol „{id}“ unbekannt – mit symbol_suchen nachsehen."));
            }
            let kind = ObjKind::Symbol { id: id.clone(), pos: Pos::new(f(a, "x")?, f(a, "y")?), size: f_opt(a, "groesse").unwrap_or(8.0), rot: f_opt(a, "drehung").unwrap_or(0.0) };
            let layer = auto_layer(doc, &kind);
            let oid = doc.add(kind, Style::NONE, layer);
            Ok(format!("Symbol {id} eingefügt, ID {oid}."))
        }),
        "fluchtweg_einfuegen" => aendern(a, |doc| {
            let pts = punkte(a, "punkte")?;
            if pts.len() < 2 {
                return Err("Mindestens zwei Punkte nötig.".into());
            }
            let art = s_opt(a, "art").unwrap_or_else(|| "fluchtweg".into());
            let (kind, style) = if art == "laufweg" {
                (ObjKind::Route { pts, width: 2.5, curved: false, arrows: true }, Style::line(farbe::ROT, 2.5))
            } else {
                let w = f_opt(a, "breite").unwrap_or(7.0);
                (ObjKind::Route { pts, width: w, curved: art == "fluchtweg_bogen", arrows: true }, Style::line(farbe::FLUCHT_HELL, w))
            };
            let layer = auto_layer(doc, &kind);
            let id = doc.add(kind, style, layer);
            Ok(format!("{} eingefügt, ID {id}.", if art == "laufweg" { "Laufweg" } else { "Fluchtweg" }))
        }),
        "plan_pruefen" => {
            let doc = laden(&s(a, "pfad")?)?;
            let b = pruefung::pruefen(&doc);
            let zeilen: Vec<String> = b
                .iter()
                .map(|(st, t)| {
                    let m = match st {
                        pruefung::Stufe::Ok => "OK     ",
                        pruefung::Stufe::Hinweis => "Hinweis",
                        pruefung::Stufe::Fehler => "FEHLT  ",
                    };
                    format!("{m}  {t}")
                })
                .collect();
            let fehler = b.iter().filter(|x| x.0 == pruefung::Stufe::Fehler).count();
            text(format!("{}\n\n{}", if fehler == 0 { "Keine Pflichtangaben fehlen.".to_string() } else { format!("{fehler} Pflichtangabe(n) fehlen.") }, zeilen.join("\n")))
        }
        "vorschau" => {
            let doc = laden(&s(a, "pfad")?)?;
            let ber = a.get("bereich").and_then(|v| v.as_array()).filter(|v| v.len() == 4).map(|v| v.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect::<Vec<_>>());
            let dpi = f_opt(a, "dpi").unwrap_or(if ber.is_some() { 150.0 } else { 50.0 }).clamp(10.0, 400.0);
            let mut png = export::render_png(&doc, dpi)?;
            if let Some(b) = ber {
                let img = image::load_from_memory(&png).map_err(|e| e.to_string())?;
                let k = dpi / 25.4;
                let (x0, y0) = ((b[0].min(b[2]).max(0.0) * k) as u32, (b[1].min(b[3]).max(0.0) * k) as u32);
                let (x1, y1) = (((b[0].max(b[2]) * k) as u32).min(img.width()), ((b[1].max(b[3]) * k) as u32).min(img.height()));
                if x1 <= x0 || y1 <= y0 {
                    return Err("Bereich liegt außerhalb des Blatts.".into());
                }
                let crop = img.crop_imm(x0, y0, x1 - x0, y1 - y0);
                png.clear();
                crop.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).map_err(|e| e.to_string())?;
            }
            if let Some(d) = s_opt(a, "datei") {
                std::fs::write(&d, &png).map_err(|e| format!("{d}: {e}"))?;
            }
            Ok(vec![Inhalt::Bild(png)])
        }
        "exportieren" => {
            let doc = laden(&s(a, "pfad")?)?;
            let ziel = s(a, "ziel")?;
            let p = Path::new(&ziel);
            match p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
                Some("pdf") => export::export_pdf(&[&doc], stamp(&s_opt(a, "stempel").unwrap_or_default()), p)?,
                Some("png") => export::export_png(&doc, f_opt(a, "dpi").unwrap_or(200.0), p)?,
                Some("svg") => export::export_svg(&doc, p)?,
                Some("dxf") => export::export_dxf(&doc, p)?,
                _ => return Err("Zielformat nicht unterstützt (pdf, png, svg, dxf).".into()),
            }
            text(format!("Ausgegeben: {ziel}"))
        }
        "pdf_importieren" => {
            let pdf = s(a, "pdf")?;
            let ziel = s(a, "ziel")?;
            let seite = f_opt(a, "seite").unwrap_or(1.0).max(1.0) as usize - 1;
            let mut doc = if Path::new(&ziel).exists() { laden(&ziel)? } else { neu_doc("fr_a3")? };
            if b_opt(a, "als_bild").unwrap_or(false) {
                let data = crate::import::pdf_page_image(Path::new(&pdf), seite, 200.0)?;
                let area = templates::layout(&doc).plan;
                let (p0, p1) = crate::import::fit_rect(area, data.w, data.h);
                let layer = doc.find_layer_by_name("Grundriss");
                let id = doc.alloc_id();
                doc.objects.insert(0, Obj { id, layer, kind: ObjKind::Image { a: p0, b: p1, data, opacity: 0.6 }, style: Style::NONE, print: true, locked: false });
                speichern(&ziel, &doc)?;
                return text(format!("PDF-Seite als Bild eingefügt: {ziel}"));
            }
            let pg = pdfvec::read_page(Path::new(&pdf), seite)?;
            if pg.items.is_empty() {
                return Err("Die PDF-Seite enthält keine Vektordaten (vermutlich ein Scan) – als_bild verwenden.".into());
            }
            let n = pg.items.len();
            let fit = match s_opt(a, "modus").as_deref().unwrap_or("formatfuellend") {
                "einpassen" => pdfvec::PdfFit::Einpassen,
                "original" => pdfvec::PdfFit::Massstab,
                _ => pdfvec::PdfFit::Formatfuellend,
            };
            let opt = pdfvec::PdfImportOpt {
                vector: true,
                group: b_opt(a, "gruppieren").unwrap_or(false),
                symbols: b_opt(a, "symbole_erkennen").unwrap_or(true),
                text_pct: f_opt(a, "texte_prozent").unwrap_or(100.0),
                fit,
            };
            let neu = pdfvec::place(&mut doc, pg, opt);
            speichern(&ziel, &doc)?;
            text(format!("{n} Teile aus {pdf} (Seite {}) übernommen, {} Objekte im Plan: {ziel}", seite + 1, neu.len()))
        }
        "symbole_zusammenfassen" => {
            let only = ids(a).ok();
            let max = f_opt(a, "max_mm").unwrap_or(pdfvec::SYMBOL_MAX_MM);
            aendern(a, |doc| {
                let alle: Vec<u64> = doc.objects.iter().map(|o| o.id).filter(|id| only.as_ref().is_none_or(|v| v.contains(id))).collect();
                let n = ops::symbole_zusammenfassen(doc, &alle, max);
                Ok(format!("{n} Symbole zu Gruppen zusammengefasst, jetzt {} Objekte.", doc.objects.len()))
            })
        }
        _ => Err(format!("Unbekanntes Werkzeug „{name}“")),
    }
}

fn hex(s: &str) -> Option<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(s, 16).ok()?;
    Some(Color::rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

// ---------- MCP-Server ----------

pub fn mcp() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        // manche Aufrufer (.NET, PowerShell 5.1) setzen ein BOM vor die erste Zeile
        let line = line.trim_start_matches('\u{feff}').trim();
        if line.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(line) else {
            let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "Ungültiges JSON"}}));
            let _ = out.flush();
            continue;
        };
        // Benachrichtigungen (ohne id) brauchen keine Antwort
        let Some(id) = req.get("id").cloned() else { continue };
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));
        let antwort = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "fluchtplan", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Werkzeuge zum Lesen und Bearbeiten von Flucht- und Rettungsplänen, Feuerwehrplänen und Laufkarten (.fplan). Koordinaten in mm auf dem Blatt, Ursprung oben links, y nach unten. Erst plan_info und vorschau nutzen, dann ändern; ein geöffnetes Programmfenster lädt Änderungen automatisch neu.",
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": werkzeuge()})),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let (content, fehler) = match std::panic::catch_unwind(|| ausfuehren(name, &args)) {
                    Ok(Ok(v)) => (v, false),
                    Ok(Err(e)) => (vec![Inhalt::Text(e)], true),
                    Err(_) => (vec![Inhalt::Text("Interner Fehler im Werkzeug".into())], true),
                };
                let content: Vec<Value> = content
                    .into_iter()
                    .map(|c| match c {
                        Inhalt::Text(t) => json!({"type": "text", "text": t}),
                        Inhalt::Bild(png) => json!({"type": "image", "data": base64::engine::general_purpose::STANDARD.encode(png), "mimeType": "image/png"}),
                    })
                    .collect();
                Ok(json!({"content": content, "isError": fehler}))
            }
            m => Err(json!({"code": -32601, "message": format!("Methode {m} nicht unterstützt")})),
        };
        let resp = match antwort {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        };
        let _ = writeln!(out, "{resp}");
        let _ = out.flush();
    }
}

// ---------- Kommandozeile ----------

#[cfg(windows)]
fn konsole_anhaengen() {
    unsafe extern "system" {
        fn AttachConsole(pid: u32) -> i32;
        fn GetStdHandle(n: u32) -> isize;
    }
    // Ausgabe nur dann an die aufrufende Konsole hängen, wenn sie nicht umgeleitet ist
    unsafe {
        let h = GetStdHandle(-11i32 as u32);
        if h == 0 || h == -1 {
            AttachConsole(u32::MAX);
        }
    }
}
#[cfg(not(windows))]
fn konsole_anhaengen() {}

/// `--cli <werkzeug> schluessel=wert …` (Werte als JSON, sonst als Text). Rückgabe: Exit-Code.
pub fn cli(args: &[String]) -> i32 {
    konsole_anhaengen();
    let Some(name) = args.first() else {
        println!("OpenFluchtplan – Kommandozeile\n\nAufruf:  OpenFluchtplan.exe --cli <werkzeug> schluessel=wert …\n         OpenFluchtplan.exe --mcp   (MCP-Server für KI-Assistenten)\n\nWerkzeuge:");
        for t in werkzeuge() {
            println!("  {:<30} {}", t["name"].as_str().unwrap_or(""), t["description"].as_str().unwrap_or(""));
            if let Some(p) = t["inputSchema"]["properties"].as_object() {
                println!("  {:<30} Parameter: {}", "", p.keys().cloned().collect::<Vec<_>>().join(", "));
            }
        }
        println!("\nBeispiel:\n  OpenFluchtplan.exe --cli pdf_importieren pdf=\"Pläne\\EG.pdf\" ziel=EG.fplan\n  OpenFluchtplan.exe --cli planangaben_setzen pfad=EG.fplan stand=11/2026 \"ersteller=Haustechnik\"\n  OpenFluchtplan.exe --cli plan_pruefen pfad=EG.fplan\n  OpenFluchtplan.exe --cli exportieren pfad=EG.fplan ziel=EG.pdf");
        return 0;
    };
    let mut a = serde_json::Map::new();
    for kv in &args[1..] {
        let Some((k, v)) = kv.split_once('=') else {
            eprintln!("Parameter „{kv}“ muss die Form schluessel=wert haben.");
            return 2;
        };
        // Zahlen, true/false, Listen und Objekte als JSON, alles andere (auch Pfade) als Text
        let val = match serde_json::from_str::<Value>(v) {
            Ok(j) if !j.is_string() => j,
            _ => Value::String(v.to_string()),
        };
        // „felder.stand=…“ -> {"felder": {"stand": …}} (ohne JSON-Anführungszeichen, die PowerShell verschluckt)
        if let Some((obj, sub)) = k.split_once('.') {
            let e = a.entry(obj.to_string()).or_insert_with(|| json!({}));
            if let Some(m) = e.as_object_mut() {
                // Feldwerte als Text, damit z. B. Plannummern mit führender Null erhalten bleiben
                m.insert(sub.to_string(), Value::String(v.to_string()));
            }
            continue;
        }
        a.insert(k.to_string(), val);
    }
    match ausfuehren(name, &Value::Object(a)) {
        Ok(inhalt) => {
            for i in inhalt {
                match i {
                    Inhalt::Text(t) => println!("{t}"),
                    Inhalt::Bild(png) => println!("(Bild, {} Bytes – mit datei=… speichern)", png.len()),
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Fehler: {e}");
            1
        }
    }
}
