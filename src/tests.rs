//! Funktionstests ohne Oberfläche.

use crate::geom::*;
use crate::model::*;
use crate::{bso, export, extract, import, ops, samples, templates};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join("fluchtplan_test");
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

#[test]
fn speichern_und_laden() {
    for (_, doc) in samples::all() {
        let json = serde_json::to_string(&doc).unwrap();
        let back: Doc = serde_json::from_str(&json).unwrap();
        assert_eq!(doc, back);
    }
}

#[test]
fn dxf_hin_und_zurueck() {
    let doc = samples::flucht_rettungsplan();
    let p = tmp("rundlauf.dxf");
    export::export_dxf(&doc, &p).unwrap();
    let area = templates::layout(&doc).plan;
    let objs = import::load_dxf(&std::fs::read(&p).unwrap(), area, doc.scale_n, false).unwrap();
    assert!(objs.len() > 100, "nur {} Objekte", objs.len());
    // Linien (Texte ragen je nach Schriftbreite über den Ankerpunkt hinaus)
    let bb = objs.iter().filter(|o| matches!(o.kind, ObjKind::Polyline { .. })).fold(BBox::EMPTY, |b, o| b.union(ops::bounds(&o.kind)));
    assert!(bb.inside(&area.expand(1.0)), "Import liegt außerhalb des Planbereichs: {bb:?} / {area:?}");
}

#[test]
fn brandschutzordnung() {
    let d = bso::teil_a(&bso::BsoInput { objekt: "Test".into(), notruf: "112".into(), intern: "55".into(), sammelstelle: "Hof".into(), stand: "11/2026".into() });
    assert!(d.objects.len() > 20);
    export::export_pdf(&[&d], export::Stamp::Keiner, &tmp("bso.pdf")).unwrap();
    export::export_png(&d, 60.0, &tmp("bso.png")).unwrap();
}

#[test]
fn transformationen() {
    let mut k = ObjKind::Wall { pts: vec![Pos::new(0.0, 0.0), Pos::new(10.0, 0.0)], thickness: 1.0, closed: false };
    ops::rotate(&mut k, Pos::new(0.0, 0.0), 90.0);
    let h = ops::handles(&k);
    assert!((h[1].x).abs() < 1e-4 && (h[1].y - 10.0).abs() < 1e-4);
    ops::mirror(&mut k, Pos::new(5.0, 0.0), true);
    assert!((ops::handles(&k)[1].x - 10.0).abs() < 1e-4);
    ops::scale(&mut k, Pos::new(10.0, 0.0), 2.0);
    assert!((ops::handles(&k)[1].y - 20.0).abs() < 1e-4);
    assert!(ops::insert_point(&mut k, Pos::new(10.0, 5.0)));
    assert_eq!(ops::handles(&k).len(), 3);
}

#[test]
fn wandwerkzeuge() {
    let w = ObjKind::Wall { pts: vec![Pos::new(0.0, 0.0), Pos::new(10.0, 0.0), Pos::new(10.0, 10.0), Pos::new(0.0, 10.0)], thickness: 1.0, closed: false };
    let parts = extract::cut_segment(&w, Pos::new(10.0, 5.0)).unwrap();
    assert_eq!(parts.len(), 2);
    let mut w2 = w.clone();
    assert!(extract::fillet(&mut w2, Pos::new(10.0, 0.0), 3.0, 1.0));
    assert!(ops::handles(&w2).len() > 4);
}

#[test]
fn suchen_ersetzen() {
    let mut doc = samples::flucht_rettungsplan();
    let n = ops::replace_text(&mut doc.objects, "büro", "Office", false, true);
    assert_eq!(n, 3);
    assert_eq!(ops::replace_text(&mut doc.objects, "Office", "", true, false), 3);
}

#[test]
fn legende_und_rahmen() {
    for (_, mut doc) in samples::all() {
        ops::sync_legend(&mut doc);
        let used = ops::used_legend_keys(&doc);
        assert!(used.iter().all(|(k, _)| doc.legend.entries.iter().any(|e| &e.key == k)));
        for f in FrameKind::ALL {
            for land in [true, false] {
                doc.frame = f;
                doc.page.landscape = land;
                let l = templates::layout(&doc);
                assert!(l.plan.w() > 50.0 && l.plan.h() > 50.0, "{:?} quer={land}", f);
                let _ = templates::frame_prims(&doc);
            }
        }
    }
}

#[test]
fn bild_import_und_waende_erkennen() {
    // Testbild: Rechteck aus dunklen Linien
    let mut img = image::RgbImage::from_pixel(400, 300, image::Rgb([255, 255, 255]));
    for x in 20..380 {
        for t in 0..6 {
            img.put_pixel(x, 20 + t, image::Rgb([0, 0, 0]));
            img.put_pixel(x, 274 + t, image::Rgb([0, 0, 0]));
        }
    }
    for y in 20..280 {
        for t in 0..6 {
            img.put_pixel(20 + t, y, image::Rgb([0, 0, 0]));
            img.put_pixel(374 + t, y, image::Rgb([0, 0, 0]));
        }
    }
    let p = tmp("grundriss.png");
    img.save(&p).unwrap();
    let data = import::load_image_file(&p).unwrap();
    assert_eq!((data.w, data.h), (400, 300));
    let walls = extract::extract_walls(&image::DynamicImage::ImageRgb8(img), Pos::new(0.0, 0.0), Pos::new(200.0, 150.0), 110, 15.0);
    assert_eq!(walls.len(), 4, "{walls:?}");
}

#[test]
fn eigenes_svg_symbol() {
    let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='20' height='10'><rect width='20' height='10' fill='red'/></svg>";
    let inner = crate::app::custom_symbol_from_svg(svg).unwrap();
    let full = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>{inner}</svg>");
    resvg::usvg::Tree::from_str(&full, &resvg::usvg::Options::default()).unwrap();
}

#[test]
fn feuerwehrkurve_din14090() {
    assert_eq!(crate::tools::din14090_width(10.5), 5.0);
    assert_eq!(crate::tools::din14090_width(14.0), 4.5);
    assert_eq!(crate::tools::din14090_width(18.0), 4.0);
    assert_eq!(crate::tools::din14090_width(30.0), 3.5);
    assert_eq!(crate::tools::din14090_width(50.0), 3.2);
    assert_eq!(crate::tools::din14090_width(80.0), 3.0);
}

#[test]
#[ignore = "braucht Internet (Nominatim/OSM)"]
fn osm_online() {
    let (lat, lon) = crate::osm::geocode("Hamburg").expect("Geocoding");
    assert!((lat - 54.09).abs() < 0.2 && (lon - 9.07).abs() < 0.2, "{lat} {lon}");
    crate::osm::map_image(lat, lon, 16, 2, 2).expect("Kacheln");
}

#[test]
fn wms_dienstbeschreibung() {
    let xml = r#"<?xml version="1.0"?>
<WMS_Capabilities version="1.3.0" xmlns="http://www.opengis.net/wms" xmlns:xlink="http://www.w3.org/1999/xlink">
 <Service><Name>WMS</Name><Title>ALKIS Test</Title><Fees>Datenlizenz Deutschland – Zero</Fees><AccessConstraints>NONE</AccessConstraints></Service>
 <Capability>
  <Request><GetMap><Format>image/png</Format><DCPType><HTTP><Get><OnlineResource xlink:href="https://example.org/wms?"/></Get></HTTP></DCPType></GetMap></Request>
  <Layer><Title>Wurzel</Title><CRS>EPSG:25832</CRS><CRS>EPSG:3857</CRS>
   <Layer><Name>adv_alkis_flurstuecke</Name><Title>Flurstücke</Title></Layer>
   <Layer><Name>adv_alkis_gebaeude</Name><Title>Gebäude</Title><Attribution><Title>© Testamt</Title></Attribution></Layer>
  </Layer>
 </Capability>
</WMS_Capabilities>"#;
    let c = crate::wms::parse_capabilities(xml, "https://example.org/caps").unwrap();
    assert_eq!(c.title, "ALKIS Test");
    assert_eq!(c.layers.len(), 2);
    assert_eq!(c.layers[0].title, "Flurstücke");
    assert!(c.supports_3857);
    assert_eq!(c.map_url, "https://example.org/wms");
    assert_eq!(c.terms, "Datenlizenz Deutschland – Zero");
    assert_eq!(c.attribution, "© Testamt");
    // Web-Mercator-Ursprung und Äquator
    let (x, y) = crate::wms::to_3857(0.0, 0.0);
    assert!(x.abs() < 1e-6 && y.abs() < 1e-6);
    // 45° N / 45° O: x = R·π/4, y = R·ln(tan(67,5°)) ≈ 5 621 521 m
    let (x, y) = crate::wms::to_3857(45.0, 45.0);
    assert!((x - 5_009_377.1).abs() < 1.0 && (y - 5_621_521.5).abs() < 1.0, "{x} {y}");
}

#[test]
#[ignore = "braucht Internet (WMS NRW, dl-de/zero)"]
fn wms_online_nrw() {
    let c = crate::wms::capabilities(crate::wms::PRESETS[1].1).expect("GetCapabilities");
    assert!(c.supports_3857 && c.layers.iter().any(|l| l.name == "adv_alkis_flurstuecke"));
    let img = crate::wms::get_map(&c.map_url, &["adv_alkis_flurstuecke".into(), "adv_alkis_gebaeude".into()], 51.2254, 6.7766, 200.0, 140.0, 400, 280).expect("GetMap");
    assert_eq!((img.w, img.h), (400, 280));
    // Das Bild darf nicht leer (einfarbig weiß) sein
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD.decode(&img.b64).unwrap();
    let rgba = image::load_from_memory(&png).unwrap().to_rgba8();
    let dunkel = rgba.pixels().filter(|p| p.0[0] < 200).count();
    assert!(dunkel > 500, "Kartenbild ist leer ({dunkel} dunkle Pixel)");
    if let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") {
        std::fs::write(std::path::Path::new(&dir).join("wms_nrw.png"), &png).unwrap();
    }
}

#[test]
fn normpruefung_musterplaene() {
    use crate::pruefung::{pruefen, Stufe};
    // Leerer Plan: Pflichtangaben fehlen
    let leer = pruefen(&Doc::default());
    assert!(leer.iter().any(|b| b.0 == Stufe::Fehler));
    // Eigene Musterpläne erfüllen alle Pflichtangaben
    for (name, doc) in crate::samples::all() {
        let fehler: Vec<_> = pruefen(&doc).into_iter().filter(|b| b.0 == Stufe::Fehler).map(|b| b.1).collect();
        assert!(fehler.is_empty(), "Musterplan „{name}“: {fehler:?}");
    }
}

#[test]
fn ebenenfarbe_ueberschreibt_linien() {
    let mut d = Doc::default();
    let l = d.layers[1].id;
    d.add(ObjKind::Line { a: Pos::new(0.0, 0.0), b: Pos::new(10.0, 0.0) }, Style::line(farbe::SCHWARZ, 0.5), l);
    d.layers[1].color = Some(farbe::ROT);
    let prims = crate::render::page_prims(&d, true);
    assert!(prims.iter().any(|p| matches!(p, crate::render::Prim::Stroke { stroke, .. } if stroke.color == farbe::ROT)));
}

#[test]
fn geschoss_schnitt_rendert_markierung() {
    let mut d = Doc::default();
    d.add(ObjKind::FloorSection { a: Pos::new(0.0, 0.0), b: Pos::new(20.0, 30.0), floors: vec!["OG".into(), "EG".into(), "KG".into()], current: 1 }, Style::default(), 0);
    let prims = crate::render::page_prims(&d, true);
    let rot = prims.iter().filter(|p| matches!(p, crate::render::Prim::Fill { color, .. } if *color == farbe::ROT)).count();
    assert!(rot >= 1);
    assert!(prims.iter().any(|p| matches!(p, crate::render::Prim::Text { text, .. } if text == "EG")));
}

#[test]
fn symbolanzahl() {
    eprintln!("SYMBOLE={}", crate::symbols::library().len());
    assert!(crate::symbols::library().len() > 200);
}


/// PDF-Dateien aus dem Ordner in `FLUCHTPLAN_PDF_ORDNER` (eigene Pläne zum Testen des Imports).
pub fn test_pdfs() -> Vec<std::path::PathBuf> {
    let Ok(dir) = std::env::var("FLUCHTPLAN_PDF_ORDNER") else { return Vec::new() };
    let mut v: Vec<_> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf"))).collect()).unwrap_or_default();
    v.sort();
    v
}

#[test]
#[ignore = "braucht eigene PDF-Pläne im Ordner FLUCHTPLAN_PDF_ORDNER"]
fn pdf_vektorimport_alle_plaene() {
    for f in &test_pdfs() {
        let n = crate::import::pdf_page_count(f).unwrap();
        for page in 0..n {
            let t = std::time::Instant::now();
            let pg = crate::pdfvec::read_page(f, page).expect("Import");
            let texte = pg.items.iter().filter(|i| matches!(i.kind, ObjKind::Text { .. })).count();
            eprintln!("PLAN {} S.{} {:.0}x{:.0} mm: {} Objekte, {} Texte, {} ms", f.file_name().unwrap().to_string_lossy(), page + 1, pg.w, pg.h, pg.items.len(), texte, t.elapsed().as_millis());
            assert!(pg.items.len() > 5);
            if let (Ok(out), true) = (std::env::var("FLUCHTPLAN_TESTBILD"), page == 0) {
                let name = f.file_stem().unwrap().to_string_lossy().into_owned();
                let mut doc = Doc::default();
                doc.frame = FrameKind::Ohne;
                doc.page = Page { format: PaperFormat::A4, landscape: pg.w > pg.h };
                for it in pg.items { doc.add(it.kind, it.style, 0); }
                crate::export::export_png(&doc, 150.0, &std::path::Path::new(&out).join(format!("{name}_import.png"))).unwrap();
                let orig = crate::import::pdf_page_image(f, page, 150.0).unwrap();
                use base64::Engine;
                std::fs::write(std::path::Path::new(&out).join(format!("{name}_orig.png")), base64::engine::general_purpose::STANDARD.decode(&orig.b64).unwrap()).unwrap();
            }
        }
    }
}

#[test]
fn pdf_vektorimport_rundlauf() {
    // Musterplan als PDF ausgeben und als Vektorgrafik wieder einlesen
    let doc = crate::samples::flucht_rettungsplan();
    let dir = std::env::temp_dir().join(format!("fluchtplan_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = dir.join("rundlauf.pdf");
    crate::export::export_pdf(&[&doc], crate::export::Stamp::Keiner, &pdf).unwrap();
    let pg = crate::pdfvec::read_page(&pdf, 0).unwrap();
    let (w, h) = doc.page.size();
    assert!((pg.w - w).abs() < 1.0 && (pg.h - h).abs() < 1.0, "Seitengröße {}x{}", pg.w, pg.h);
    let texte: Vec<&str> = pg.items.iter().filter_map(|i| if let ObjKind::Text { text, .. } = &i.kind { Some(text.as_str()) } else { None }).collect();
    assert!(texte.iter().any(|t| t.contains("Notruf")), "Texte: {texte:?}");
    assert!(pg.items.iter().any(|i| matches!(i.kind, ObjKind::Polyline { closed: true, .. } | ObjKind::Path { .. })));
    // Fluchtwegfarbe kommt als eigene Farbe (Ebene) an
    assert!(pg.items.iter().any(|i| i.color == farbe::GRUEN));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn musterplaene_symbole_anklickbar() {
    // Ein Klick auf ein Symbol muss das Symbol treffen – nicht ein darüberliegendes großes Objekt
    for (name, doc) in crate::samples::all() {
        for o in &doc.objects {
            let ObjKind::Symbol { pos, .. } = &o.kind else { continue };
            let sichtbar = || doc.objects.iter().rev().filter(|x| doc.layer_visible(x.layer) && !doc.layer_locked(x.layer));
            let hit = sichtbar().find(|x| crate::ops::hit_obj_umriss(x, *pos, 0.5)).or_else(|| sichtbar().find(|x| crate::ops::hit_obj(x, *pos, 0.5)));
            let hit = hit.map(|x| crate::ops::kind_name(&x.kind)).unwrap_or("nichts");
            assert_eq!(hit, "Symbol", "{name}: Klick auf Symbol bei {pos:?} trifft {hit}");
        }
    }
}

#[test]
fn importierte_symbole_und_blattrahmen() {
    let mut doc = crate::samples::flucht_rettungsplan();
    doc.objects.clear();
    let rect = |x: f32, y: f32, w: f32, fill: Fill| Obj {
        id: 0,
        layer: 0,
        kind: ObjKind::Polyline { pts: vec![Pos::new(x, y), Pos::new(x + w, y), Pos::new(x + w, y + w), Pos::new(x, y + w)], closed: true },
        style: Style { stroke: Some(StrokeStyle::new(farbe::SCHWARZ, 0.25)), fill },
        print: true,
        locked: false,
    };
    // Symbol aus drei Teilen (Hintergrund, Rahmen, Text), daneben ein zweites, darüber ein ungefüllter Blattrahmen
    let mut teile = vec![
        rect(100.0, 100.0, 7.0, Fill::Farbe(farbe::WEISS)),
        rect(100.0, 100.0, 7.0, Fill::Keine),
        Obj { kind: ObjKind::Text { pos: Pos::new(101.0, 102.0), text: "FIBS".into(), size: 2.0, bold: true, italic: false, align: TAlign::Links, rot: 0.0 }, ..rect(0.0, 0.0, 1.0, Fill::Keine) },
        rect(150.0, 100.0, 7.0, Fill::Farbe(farbe::ROT)),
        rect(151.0, 101.0, 5.0, Fill::Farbe(farbe::WEISS)),
        rect(0.0, 0.0, 420.0, Fill::Keine),
    ];
    for t in &mut teile {
        t.id = doc.alloc_id();
    }
    let ids: Vec<u64> = teile.iter().map(|o| o.id).collect();
    doc.objects = teile;
    let rahmen = *ids.last().unwrap();
    // ungefüllter Rahmen greift nur an der Linie
    let rahmen_obj = doc.objects.last().unwrap().clone();
    assert!(!crate::ops::hit_obj(&rahmen_obj, Pos::new(103.0, 103.0), 0.5));
    assert!(crate::ops::hit_obj(&rahmen_obj, Pos::new(0.2, 50.0), 0.5));
    let n = crate::ops::symbole_zusammenfassen(&mut doc, &ids, 12.0);
    assert_eq!(n, 2, "zwei Symbole erwartet");
    assert_eq!(doc.objects.len(), 3);
    assert_eq!(doc.objects.last().unwrap().id, rahmen, "Blattrahmen bleibt einzeln und oben");
    assert!(doc.objects.iter().filter(|o| matches!(&o.kind, ObjKind::Group { children } if children.len() >= 2)).count() == 2);
}

#[test]
fn schnittstelle_werkzeuge() {
    use crate::schnittstelle::{Inhalt, ausfuehren};
    use serde_json::json;
    let dir = std::env::temp_dir().join(format!("fluchtplan_mcp_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("test.fplan").to_string_lossy().into_owned();
    let txt = |r: Result<Vec<Inhalt>, String>| match r.expect("Werkzeug").into_iter().next() {
        Some(Inhalt::Text(t)) => t,
        _ => String::new(),
    };
    assert!(txt(ausfuehren("plan_neu", &json!({"ziel": p, "vorlage": "fr_a3"}))).contains("angelegt"));
    txt(ausfuehren("planangaben_setzen", &json!({"pfad": p, "objekt": "Halle 1", "stand": "11/2026", "massstab": 200})));
    txt(ausfuehren("symbol_einfuegen", &json!({"pfad": p, "symbol": "STANDORT", "x": 200, "y": 150})));
    txt(ausfuehren("symbol_einfuegen", &json!({"pfad": p, "symbol": "E002", "x": 250, "y": 150})));
    txt(ausfuehren("fluchtweg_einfuegen", &json!({"pfad": p, "punkte": [[150, 160], [260, 160]]})));
    let id: u64 = txt(ausfuehren("text_einfuegen", &json!({"pfad": p, "x": 120, "y": 120, "text": "Büro 1"}))).trim_end_matches('.').rsplit(' ').next().unwrap().parse().unwrap();
    assert_eq!(txt(ausfuehren("text_ersetzen", &json!({"pfad": p, "suchen": "Büro 1", "ersetzen": "Lager"}))), "1 Stelle(n) ersetzt.");
    let liste = txt(ausfuehren("objekte_auflisten", &json!({"pfad": p, "text": "lager"})));
    assert!(liste.contains(&format!("\"id\": {id}")), "{liste}");
    // Objekt per JSON ändern (Schriftgröße)
    let mut o: serde_json::Value = serde_json::from_str(&txt(ausfuehren("objekt_lesen", &json!({"pfad": p, "id": id})))).unwrap();
    o["kind"]["Text"]["size"] = json!(6.0);
    txt(ausfuehren("objekt_schreiben", &json!({"pfad": p, "objekt": o})));
    let doc = crate::schnittstelle::laden(&p).unwrap();
    assert!(doc.objects.iter().any(|o| matches!(&o.kind, ObjKind::Text { text, size, .. } if text == "Lager" && (*size - 6.0).abs() < 1e-3)));
    assert_eq!(doc.scale_n, 200.0);
    assert!(doc.legend.entries.iter().any(|e| e.key == "sym:E002"), "Legende ergänzt");
    // Vorschau liefert ein PNG
    assert!(matches!(ausfuehren("vorschau", &json!({"pfad": p})).unwrap().first(), Some(Inhalt::Bild(b)) if b.starts_with(&[0x89, b'P', b'N', b'G'])));
    // Fehler werden gemeldet, nicht verschluckt
    assert!(ausfuehren("symbol_einfuegen", &json!({"pfad": p, "symbol": "GIBTSNICHT", "x": 1, "y": 1})).is_err());
    let pdf = dir.join("t.pdf").to_string_lossy().into_owned();
    txt(ausfuehren("exportieren", &json!({"pfad": p, "ziel": pdf})));
    assert!(std::fs::metadata(&pdf).unwrap().len() > 1000);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore = "Sichtprüfung"]
fn a4_rahmen_als_bild() {
    let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") else { return };
    for (name, ls) in [("a4_quer", true), ("a4_hoch", false), ("a3_quer", true), ("a3_hoch", false)] {
        let mut d = crate::samples::flucht_rettungsplan();
        if name != "a3_quer" {
            let alt = crate::templates::layout(&d).plan;
            d.page = Page { format: if name.starts_with("a4") { PaperFormat::A4 } else { PaperFormat::A3 }, landscape: ls };
            crate::templates::inhalt_umsetzen(&mut d, alt);
        }
        let png = crate::export::render_png(&d, 100.0).unwrap();
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.png")), png).unwrap();
        for (s, t) in crate::pruefung::pruefen(&d) {
            println!("{name} {s:?} {t}");
        }
    }
}

/// Ältere Datei (Version 2) mit Inhalt im früheren, größeren Planbereich: Inhalt wird beim Laden eingepasst.
#[test]
fn alte_datei_wird_eingepasst() {
    let mut d = samples::flucht_rettungsplan();
    let plan = templates::layout(&d).plan;
    // Inhalt nach unten in den Bereich der Verhaltensregeln schieben (so lag er bei Version 2)
    for o in &mut d.objects {
        ops::translate(&mut o.kind, Pos::new(0.0, 40.0));
    }
    d.version = 2;
    templates::nach_laden(&mut d);
    assert_eq!(d.version, 3);
    let bb = d.objects.iter().fold(BBox::EMPTY, |bb, o| bb.union(ops::bounds_doc(&d, &o.kind)));
    assert!(bb.inside(&plan.expand(0.5)), "{bb:?} nicht in {plan:?}");
    // aktuelle Dateien bleiben unverändert
    let vorher = d.objects.clone();
    templates::nach_laden(&mut d);
    assert_eq!(vorher, d.objects);
}

/// Verhaltensregeln: breite Felder nebeneinander, schmale untereinander – immer innerhalb des Felds.
#[test]
fn verhaltensregeln_im_feld() {
    let d = samples::flucht_rettungsplan();
    for rb in [BBox::from_two(Pos::new(0.0, 0.0), Pos::new(200.0, 100.0)), BBox::from_two(Pos::new(10.0, 10.0), Pos::new(90.0, 210.0))] {
        let mut out = Vec::new();
        templates::regeln_prims(&mut out, &d, rb);
        assert!(out.len() > 40);
        for p in &out {
            if let crate::render::Prim::Text { pos, text, size, .. } = p {
                assert!(pos.y >= rb.min.y && pos.y + size <= rb.max.y + 0.01, "{text} ragt heraus");
                assert!(pos.x >= rb.min.x - ops::text_width(text, *size) && pos.x <= rb.max.x, "{text} ragt heraus");
            }
        }
    }
}

/// Wechsel A3 → A4: Inhalt liegt danach im neuen Planbereich, Maßstab angepasst, Legende vollständig.
#[test]
fn formatwechsel_auf_a4() {
    for ls in [true, false] {
        let mut d = crate::samples::flucht_rettungsplan();
        let alt = crate::templates::layout(&d).plan;
        let massstab = d.scale_n;
        d.page = Page { format: PaperFormat::A4, landscape: ls };
        let f = crate::templates::inhalt_umsetzen(&mut d, alt).unwrap();
        assert!(f < 1.0);
        assert!((d.scale_n - (massstab / f).round()).abs() < 0.5);
        let neu = crate::templates::layout(&d).plan.expand(0.5);
        for o in &d.objects {
            let b = crate::ops::bounds_doc(&d, &o.kind);
            assert!(neu.contains(b.min) && neu.contains(b.max), "{} außerhalb des Planbereichs", crate::ops::kind_name(&o.kind));
        }
        // alle Legendeneinträge passen ins Feld
        let l = crate::templates::layout(&d);
        let lb = l.legend.unwrap();
        let u = 210.0 / 297.0;
        let (cols, size) = crate::render::legend_fit(&d, lb, u);
        let n = crate::ops::used_legend_keys(&d).len();
        let row_h = size * 2.2;
        let rows = ((lb.max.y - (lb.min.y + size * 0.48 + size * 2.4)) / row_h).floor() as usize;
        assert!(rows * cols as usize >= n, "Legende passt nicht: {rows} Zeilen × {cols} Spalten für {n} Einträge");
    }
}
