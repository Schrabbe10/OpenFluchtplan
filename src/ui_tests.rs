//! Oberflächentests ohne Bildschirm (egui_kittest): Menüband, Werkzeuge, Dialoge.

use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

use crate::app::{App, Tool};
use crate::model::*;

fn harness() -> Harness<'static, App> {
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_max_steps(20).with_accessibility_check(false).build_eframe(|cc| {
        let mut app = App::new(cc);
        app.dlg.welcome = false;
        app.settings.ui_scale = 1.0;
        app
    });
    h.run_steps(3);
    h
}

fn click_label(h: &mut Harness<'static, App>, label: &str) {
    h.get_all_by_label(label).next().unwrap_or_else(|| panic!("Schaltfläche „{label}“ nicht gefunden")).click();
    h.run_steps(2);
}

fn pointer(h: &mut Harness<'static, App>, p: Pos2, pressed: bool) {
    h.input_mut().events.push(Event::PointerMoved(p));
    h.input_mut().events.push(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE });
}

fn click_canvas(h: &mut Harness<'static, App>, p: Pos2) {
    pointer(h, p, true);
    h.step();
    pointer(h, p, false);
    h.step();
    h.run_steps(2);
}

/// Mitte der Zeichenfläche (zwischen den Seitenleisten).
fn canvas_point(dx: f32, dy: f32) -> Pos2 {
    Pos2::new(800.0 + dx, 560.0 + dy)
}

#[test]
fn menueband_alle_reiter() {
    let mut h = harness();
    for tab in ["Datei", "Plan", "Einfügen", "Bearbeiten", "Ansicht", "Extras"] {
        click_label(&mut h, tab);
    }
}

#[test]
fn wand_zeichnen_und_rueckgaengig() {
    let mut h = harness();
    click_label(&mut h, "Einfügen");
    click_label(&mut h, "Wand");
    assert_eq!(h.state().tool, Tool::Wall);
    click_canvas(&mut h, canvas_point(-100.0, 0.0));
    click_canvas(&mut h, canvas_point(100.0, 0.0));
    click_canvas(&mut h, canvas_point(100.0, 120.0));
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let walls: Vec<&Obj> = h.state().doc().objects.iter().filter(|o| matches!(o.kind, ObjKind::Wall { .. })).collect();
    assert_eq!(walls.len(), 1, "Wand wurde nicht angelegt");
    if let ObjKind::Wall { pts, .. } = &walls[0].kind {
        assert_eq!(pts.len(), 3);
    }
    // Rückgängig über das Menüband
    click_label(&mut h, "Bearbeiten");
    click_label(&mut h, "Rückgängig");
    assert!(h.state().doc().objects.is_empty());
    click_label(&mut h, "Wiederholen");
    assert_eq!(h.state().doc().objects.len(), 1);
}

#[test]
fn symbol_setzen_auswaehlen_loeschen() {
    let mut h = harness();
    h.state_mut().tool = Tool::Symbol("F001".into());
    click_canvas(&mut h, canvas_point(0.0, 0.0));
    assert_eq!(h.state().doc().objects.len(), 1);
    assert!(h.state().doc().legend.entries.iter().any(|e| e.key == "sym:F001"), "Legende nicht ergänzt");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert_eq!(h.state().tool, Tool::Select);
    click_canvas(&mut h, canvas_point(0.0, 0.0));
    assert_eq!(h.state().tab().sel.len(), 1, "Symbol nicht ausgewählt");
    h.key_press(egui::Key::Delete);
    h.run_steps(2);
    assert!(h.state().doc().objects.is_empty());
}

#[test]
fn symbol_nach_dem_setzen_sofort_verschieben() {
    let mut h = harness();
    h.state_mut().tool = Tool::Symbol("F001".into());
    click_canvas(&mut h, canvas_point(0.0, 0.0));
    assert_eq!(h.state().doc().objects.len(), 1);
    let pos = |h: &Harness<'static, App>| match &h.state().doc().objects[0].kind {
        ObjKind::Symbol { pos, .. } => *pos,
        _ => panic!("Symbol erwartet"),
    };
    let vorher = pos(&h);
    // Werkzeug bleibt aktiv; Ziehen auf dem Symbol verschiebt es, statt ein zweites zu setzen
    pointer(&mut h, canvas_point(0.0, 0.0), true);
    h.step();
    for i in 1..=6 {
        h.input_mut().events.push(Event::PointerMoved(canvas_point(i as f32 * 10.0, 0.0)));
        h.step();
    }
    pointer(&mut h, canvas_point(60.0, 0.0), false);
    h.run_steps(3);
    assert_eq!(h.state().doc().objects.len(), 1, "zweites Symbol gesetzt statt verschoben");
    assert!(pos(&h).x > vorher.x + 1.0, "Symbol nicht verschoben: {vorher:?} -> {:?}", pos(&h));
    // daneben klicken setzt weiterhin ein neues Symbol
    click_canvas(&mut h, canvas_point(-200.0, 100.0));
    assert_eq!(h.state().doc().objects.len(), 2);
}

#[test]
fn fluchtweg_mit_doppelklick() {
    let mut h = harness();
    h.state_mut().tool = Tool::Route;
    click_canvas(&mut h, canvas_point(-150.0, 50.0));
    click_canvas(&mut h, canvas_point(50.0, 50.0));
    click_canvas(&mut h, canvas_point(50.0, -80.0));
    // Rechtsklick beendet den Linienzug
    let p = canvas_point(50.0, -80.0);
    for pressed in [true, false] {
        h.input_mut().events.push(Event::PointerButton { pos: p, button: PointerButton::Secondary, pressed, modifiers: Modifiers::NONE });
        h.step();
    }
    h.run_steps(3);
    let routes: Vec<&Obj> = h.state().doc().objects.iter().filter(|o| matches!(o.kind, ObjKind::Route { .. })).collect();
    assert_eq!(routes.len(), 1);
    if let ObjKind::Route { pts, .. } = &routes[0].kind {
        assert_eq!(pts.len(), 3, "letzter Punkt beim Doppelklick verloren");
    }
}

#[test]
fn musterplan_alles_auswaehlen_gruppieren() {
    let mut h = harness();
    h.state_mut().open_sample(0);
    h.run_steps(3);
    let n = h.state().doc().objects.len();
    assert!(n > 20);
    click_label(&mut h, "Bearbeiten");
    click_label(&mut h, "Alles");
    assert_eq!(h.state().tab().sel.len(), n);
    click_label(&mut h, "Gruppieren");
    assert_eq!(h.state().doc().objects.len(), 1);
    click_label(&mut h, "Gruppe aufheben");
    assert_eq!(h.state().doc().objects.len(), n);
}

#[test]
fn dialoge_oeffnen() {
    let mut h = harness();
    h.state_mut().open_sample(2);
    h.run_steps(2);
    click_label(&mut h, "Plan");
    click_label(&mut h, "Legende …");
    assert!(h.state().dlg.legend);
    assert!(h.get_all_by_label("Symbollegende").next().is_some());
    h.state_mut().dlg.legend = false;
    click_label(&mut h, "Rahmen und Titel …");
    assert!(h.state().dlg.page);
    h.state_mut().dlg.page = false;
    click_label(&mut h, "Bearbeiten");
    click_label(&mut h, "Symbol-Manager …");
    assert!(h.state().dlg.symbol_manager);
    h.state_mut().dlg.symbol_manager = false;
    click_label(&mut h, "Suchen / Ersetzen …");
    assert!(h.state().dlg.search);
    h.state_mut().dlg.search = false;
    click_label(&mut h, "Extras");
    click_label(&mut h, "Brandschutzordnung (Teil A) …");
    click_label(&mut h, "Erstellen");
    assert_eq!(h.state().doc().page.format, PaperFormat::A4, "Brandschutzordnung nicht erstellt");
    click_label(&mut h, "Benutzerhilfe");
    assert!(h.state().dlg.help);
}

#[test]
fn planrahmen_und_format_umschalten() {
    let mut h = harness();
    h.state_mut().new_doc(FrameKind::Feuerwehrplan, PaperFormat::A3, true);
    h.run_steps(2);
    h.state_mut().rotate_content(90.0);
    h.state_mut().duplicate_plan();
    h.run_steps(2);
    assert_eq!(h.state().tabs.len(), 2);
    for f in FrameKind::ALL {
        let tab = h.state_mut().tab_mut();
        crate::ui::apply_frame_defaults(&mut tab.doc, f);
        tab.doc.frame = f;
        h.run_steps(1);
    }
}

#[test]
fn alle_dialoge_rendern() {
    let mut h = harness();
    h.state_mut().open_sample(0);
    h.run_steps(2);
    let flags: [fn(&mut App); 20] = [
        |a| a.dlg.welcome = true,
        |a| a.dlg.samples = true,
        |a| a.dlg.legend = true,
        |a| a.dlg.page = true,
        |a| a.dlg.multicopy = true,
        |a| a.dlg.search = true,
        |a| a.dlg.symbol_manager = true,
        |a| a.dlg.scale_dist = Some(42.0),
        |a| a.dlg.options = true,
        |a| a.dlg.about = true,
        |a| a.dlg.help = true,
        |a| a.dlg.bso = true,
        |a| a.dlg.osm = true,
        |a| a.dlg.wms = true,
        |a| a.dlg.check = true,
        |a| a.dlg.offset = true,
        |a| a.dlg.pdf_pages = Some(crate::app::PdfAuswahl { name: "plan.pdf".into(), bytes: Default::default(), seiten: 3, seite: 0 }),
        |a| a.dlg.png = true,
        |a| a.dlg.merge = true,
        |a| a.dlg.extract = true,
    ];
    for f in flags {
        f(h.state_mut());
        h.run_steps(2);
    }
    h.state_mut().dlg.qr = Some(Pos::new(50.0, 50.0));
    h.state_mut().dlg.error = Some("Test".into());
    h.state_mut().tabs[0].dirty = true;
    h.state_mut().dlg.close_confirm = Some(0);
    h.state_mut().dlg.quit_confirm = true;
    h.run_steps(3);
    // zweisprachige Legende
    h.state_mut().tab_mut().doc.legend.langs = Lang::ALL.to_vec();
    h.run_steps(2);
}

#[test]
fn eigenschaften_fuer_jede_objektart() {
    let mut h = harness();
    // Ein Plan mit allen Objektarten
    let doc = {
        let mut d = Doc::default();
        let p = |x: f32, y: f32| Pos::new(x, y);
        let img = crate::import::image_data(&{
            let mut v = Vec::new();
            image::DynamicImage::new_rgb8(4, 4).write_to(&mut std::io::Cursor::new(&mut v), image::ImageFormat::Png).unwrap();
            v
        }, "image/png")
        .unwrap();
        let kinds = vec![
            ObjKind::Line { a: p(150.0, 60.0), b: p(170.0, 60.0) },
            ObjKind::Polyline { pts: vec![p(150.0, 70.0), p(170.0, 75.0), p(160.0, 80.0)], closed: true },
            ObjKind::Rect { a: p(180.0, 60.0), b: p(200.0, 70.0), radius: 2.0 },
            ObjKind::Circle { c: p(210.0, 65.0), r: 5.0 },
            ObjKind::Ellipse { c: p(225.0, 65.0), rx: 6.0, ry: 3.0 },
            ObjKind::Arc { c: p(240.0, 65.0), r: 5.0, start: 0.0, end: 120.0 },
            ObjKind::NGon { c: p(255.0, 65.0), r: 5.0, n: 6, rot: 0.0 },
            ObjKind::Spline { pts: vec![p(150.0, 90.0), p(160.0, 95.0), p(170.0, 88.0)] },
            ObjKind::Hose { pts: vec![p(180.0, 90.0), p(200.0, 95.0)], width: 1.5 },
            ObjKind::Text { pos: p(210.0, 90.0), text: "Text äöü".into(), size: 4.0, bold: true, italic: true, align: TAlign::Mitte, rot: 30.0 },
            ObjKind::Image { a: p(150.0, 100.0), b: p(170.0, 120.0), data: img, opacity: 0.5 },
            ObjKind::Dimension { a: p(180.0, 110.0), b: p(220.0, 110.0), offset: 5.0 },
            ObjKind::Table { pos: p(230.0, 100.0), cols: vec![20.0, 20.0], rows: vec![6.0, 6.0], cells: vec![vec!["A".into(), "B".into()], vec!["1".into(), "2".into()]], size: 3.0 },
            ObjKind::Wall { pts: vec![p(150.0, 130.0), p(200.0, 130.0)], thickness: 1.2, closed: false },
            ObjKind::Room { pts: vec![p(150.0, 135.0), p(180.0, 135.0), p(180.0, 160.0), p(150.0, 160.0)], name: "Raum".into(), show_area: true, size: 3.0 },
            ObjKind::Window { a: p(185.0, 130.0), b: p(195.0, 130.0), thickness: 1.2 },
            ObjKind::Door { pos: p(160.0, 130.0), width: 8.0, rot: 0.0, mirror: true, double: true },
            ObjKind::Stairs { a: p(200.0, 140.0), b: p(230.0, 140.0), width: 10.0, steps: 8 },
            ObjKind::Note { pos: p(240.0, 130.0), text: "Notiz".into() },
            ObjKind::Area { pts: vec![p(150.0, 170.0), p(180.0, 170.0), p(180.0, 190.0)], preset: "rei90a".into() },
            ObjKind::Route { pts: vec![p(190.0, 170.0), p(240.0, 170.0)], width: 6.0, curved: true, arrows: true },
            ObjKind::Symbol { id: "E002".into(), pos: p(260.0, 170.0), size: 8.0, rot: 90.0 },
            ObjKind::Legend { a: p(300.0, 60.0), b: p(400.0, 120.0), cols: 2, size: 3.0 },
            ObjKind::Grid { a: p(150.0, 200.0), b: p(250.0, 240.0), nx: 4, ny: 2 },
            ObjKind::RevisionCloud { pts: vec![p(260.0, 200.0), p(290.0, 200.0), p(290.0, 220.0), p(260.0, 220.0)] },
            ObjKind::Fence { pts: vec![p(300.0, 200.0), p(340.0, 200.0)] },
            ObjKind::Rail { pts: vec![p(300.0, 210.0), p(340.0, 210.0)] },
            ObjKind::ScaleBar { pos: p(300.0, 225.0), meters: 10.0 },
            ObjKind::QrCode { pos: p(350.0, 200.0), size: 20.0, text: "https://example.org".into() },
            ObjKind::FireCurve { c: p(380.0, 160.0), r_out: 20.0, width: 8.0, start: 0.0, end: 90.0 },
            ObjKind::FloorSection { a: p(100.0, 250.0), b: p(125.0, 280.0), floors: vec!["1. OG".into(), "EG".into(), "KG".into()], current: 1 },
        ];
        for k in kinds {
            d.add(k, Style::default(), 4);
        }
        let first = d.objects[0].clone();
        let id = d.alloc_id();
        d.objects.push(Obj { id, kind: ObjKind::Group { children: vec![first] }, ..d.objects[0].clone() });
        crate::ops::sync_legend(&mut d);
        d
    };
    h.state_mut().add_tab(crate::app::DocTab::new(doc, None));
    h.run_steps(2);
    let ids: Vec<u64> = h.state().doc().objects.iter().map(|o| o.id).collect();
    for id in ids {
        h.state_mut().tab_mut().sel = vec![id];
        h.run_steps(2);
    }
    // Mehrfachauswahl
    h.state_mut().select_all();
    h.run_steps(2);
    // PDF/DXF der Testseite
    let d = h.state().doc().clone();
    crate::export::export_pdf(&[&d], crate::export::Stamp::Entwurf, &std::env::temp_dir().join("alle_objekte.pdf")).unwrap();
    crate::export::export_png(&d, 80.0, &std::env::temp_dir().join("alle_objekte.png")).unwrap();
    crate::export::export_dxf(&d, &std::env::temp_dir().join("alle_objekte.dxf")).unwrap();
}

#[test]
fn polygon_fuellen_in_raum() {
    let mut h = harness();
    h.state_mut().tool = Tool::WallOrtho;
    for (dx, dy) in [(-120.0, -80.0), (120.0, -80.0), (120.0, 80.0), (-120.0, 80.0), (-120.0, -80.0)] {
        click_canvas(&mut h, canvas_point(dx, dy));
    }
    h.key_press(egui::Key::Enter);
    h.run_steps(2);
    h.state_mut().tool = Tool::FillPolygon("fluchtbereich".into());
    click_canvas(&mut h, canvas_point(0.0, 0.0));
    let areas = h.state().doc().objects.iter().filter(|o| matches!(o.kind, ObjKind::Area { .. })).count();
    assert_eq!(areas, 1, "Fläche nicht gefüllt: {}", h.state().status);
}

#[test]
fn oberflaechengroesse() {
    let mut h = harness();
    h.state_mut().settings.ui_scale = 1.5;
    h.run_steps(3);
    assert!((h.ctx.zoom_factor() - 1.5).abs() < 0.001);
    // Menüband bleibt bedienbar
    click_label(&mut h, "Einfügen");
    click_label(&mut h, "Linie");
    assert_eq!(h.state().tool, Tool::Line);
    // Strg+Plus darf die Oberfläche nicht verändern
    h.key_press_modifiers(Modifiers::COMMAND, egui::Key::Plus);
    h.run_steps(2);
    assert!((h.ctx.zoom_factor() - 1.5).abs() < 0.001);
}

#[test]
fn versatz_offset() {
    let mut h = harness();
    let doc = {
        let mut d = Doc::default();
        d.add(ObjKind::Line { a: Pos::new(100.0, 100.0), b: Pos::new(200.0, 100.0) }, Style::line(farbe::SCHWARZ, 0.5), 1);
        d.add(ObjKind::Wall { pts: vec![Pos::new(100.0, 150.0), Pos::new(200.0, 150.0), Pos::new(200.0, 200.0)], thickness: 1.2, closed: false }, Style::line(farbe::SCHWARZ, 0.5), 1);
        d
    };
    h.state_mut().add_tab(crate::app::DocTab::new(doc, None));
    h.run_steps(2);
    let ids: Vec<u64> = h.state().tab().doc.objects.iter().map(|o| o.id).collect();
    h.state_mut().tab_mut().sel = ids;
    let n = h.state_mut().offset_selection(5.0);
    assert_eq!(n, 2);
    let objs = &h.state().tab().doc.objects;
    assert_eq!(objs.len(), 4);
    if let ObjKind::Line { a, b } = &objs[2].kind {
        assert!((a.y - b.y).abs() < 1e-4 && (a.y - 100.0).abs() > 4.9 && (a.y - 100.0).abs() < 5.1);
    } else {
        panic!("Linie erwartet");
    }
    // Rückgängig entfernt beide Kopien
    h.state_mut().tab_mut().undo();
    assert_eq!(h.state().tab().doc.objects.len(), 2);
}

/// Rendert die neuen Dialoge als PNG (nur bei gesetzter Umgebungsvariable FLUCHTPLAN_TESTBILD).
#[test]
#[ignore = "Sichtprüfung: braucht GPU-Adapter"]
fn dialoge_als_bild() {
    let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") else { return };
    // Wie im Programm nur DirectX 12 (andere Backends stürzen unter RDP ab)
    let mut setup = egui_kittest::wgpu::default_wgpu_setup();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(s) = &mut setup {
        s.instance_descriptor.backends = if cfg!(windows) { eframe::wgpu::Backends::DX12 } else { eframe::wgpu::Backends::PRIMARY };
    }
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_max_steps(20).with_accessibility_check(false).wgpu_setup(setup).build_eframe(|cc| {
        let mut app = App::new(cc);
        app.dlg.welcome = false;
        app.settings.ui_scale = 1.0;
        app
    });
    h.run_steps(3);
    h.state_mut().dlg.samples = false;
    let doc = crate::samples::feuerwehr_geschoss();
    h.state_mut().add_tab(crate::app::DocTab::new(doc, None));
    h.run_steps(3);
    let shots: [(&str, fn(&mut App)); 4] = [
        ("plan", |_a| {}),
        ("pruefen", |a| a.dlg.check = true),
        ("flurkarte", |a| a.dlg.wms = true),
        ("optionen", |a| { a.dlg.options = true; a.settings.ui_scale = 1.0 }),
    ];
    for (name, f) in shots {
        f(h.state_mut());
        h.run_steps(4);
        let img = h.render().expect("Rendern");
        img.save(std::path::Path::new(&dir).join(format!("ui_{name}.png"))).unwrap();
        let a = h.state_mut();
        a.dlg.check = false;
        a.dlg.wms = false;
        a.dlg.options = false;
        h.run_steps(2);
    }
}

/// Eigener Plan: Import über den Dialog, einzelnen Text ändern, als Bild ausgeben.
#[test]
#[ignore = "braucht eigene PDF-Pläne (FLUCHTPLAN_PDF_ORDNER) und einen GPU-Adapter"]
fn pdf_import_im_programm() {
    let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") else { return };
    let Some(f) = crate::tests::test_pdfs().into_iter().next() else { return };
    let mut setup = egui_kittest::wgpu::default_wgpu_setup();
    if let eframe::egui_wgpu::WgpuSetup::CreateNew(s) = &mut setup {
        s.instance_descriptor.backends = if cfg!(windows) { eframe::wgpu::Backends::DX12 } else { eframe::wgpu::Backends::PRIMARY };
    }
    let mut h = Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_max_steps(20).with_accessibility_check(false).wgpu_setup(setup).build_eframe(|cc| {
        let mut app = App::new(cc);
        app.dlg.welcome = false;
        app.settings.ui_scale = 1.0;
        app
    });
    h.run_steps(3);
    // Dialog wie nach „Plan → Importieren → Grundriss aus PDF“
    h.state_mut().dlg.pdf_pages = Some(crate::app::PdfAuswahl { name: "plan.pdf".into(), bytes: std::sync::Arc::new(std::fs::read(&f).unwrap()), seiten: 1, seite: 0 });
    h.state_mut().dlg.pdf_opt.group = false;
    h.run_steps(3);
    h.render().unwrap().save(std::path::Path::new(&dir).join("ui_pdfdialog.png")).unwrap();
    click_label(&mut h, "Formatfüllend – Blattformat übernehmen, eigenen Rahmen ausblenden");
    click_label(&mut h, "Importieren");
    h.run_steps(4);
    let n = h.state().tab().doc.objects.len();
    assert!(n > 10, "nur {n} Objekte");
    assert_eq!(h.state().tab().doc.frame, FrameKind::Ohne);
    assert!(h.state().tab().doc.layers.iter().any(|l| l.name.starts_with("PDF #")));
    // ersten Text einzeln ändern
    if let Some(o) = h.state_mut().tab_mut().doc.objects.iter_mut().find(|o| matches!(o.kind, ObjKind::Text { .. })) {
        if let ObjKind::Text { text, .. } = &mut o.kind {
            text.push_str(" (geändert)");
        }
    }
    h.run_steps(4);
    h.render().unwrap().save(std::path::Path::new(&dir).join("ui_pdfimport.png")).unwrap();
}

/// Menüband aller Reiter als Bild (Sichtprüfung der Breite).
#[test]
#[ignore = "Sichtprüfung: braucht GPU-Adapter"]
fn menueband_als_bild() {
    let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") else { return };
    for w in [1600.0, 1100.0] {
        let mut setup = egui_kittest::wgpu::default_wgpu_setup();
        if let eframe::egui_wgpu::WgpuSetup::CreateNew(s) = &mut setup {
            s.instance_descriptor.backends = if cfg!(windows) { eframe::wgpu::Backends::DX12 } else { eframe::wgpu::Backends::PRIMARY };
        }
        let mut h = Harness::builder().with_theme(egui::Theme::Light).with_size(egui::vec2(w, 700.0)).with_max_steps(20).with_accessibility_check(false).wgpu_setup(setup).build_eframe(|cc| {
            let mut app = App::new(cc);
            app.dlg.welcome = false;
            app.settings.ui_scale = 1.0;
            app
        });
        h.run_steps(3);
        for (i, tab) in ["Datei", "Plan", "Einfügen", "Bearbeiten", "Ansicht", "Extras"].iter().enumerate() {
            click_label(&mut h, tab);
            h.run_steps(3);
            let img = h.render().unwrap();
            let top = image::imageops::crop_imm(&img, 0, 0, img.width(), 135.min(img.height())).to_image();
            top.save(std::path::Path::new(&dir).join(format!("ribbon_{}_{i}.png", w as u32))).unwrap();
        }
    }
}

#[test]
fn externe_aenderung_wird_neu_geladen() {
    let dir = std::env::temp_dir().join(format!("fluchtplan_reload_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("plan.fplan");
    let ps = p.to_string_lossy().into_owned();
    crate::schnittstelle::ausfuehren("plan_neu", &serde_json::json!({"ziel": ps})).unwrap();
    let mut h = harness();
    h.state_mut().open_path(&p);
    h.run_steps(2);
    assert_eq!(h.state().tab().doc.objects.len(), 0);
    std::thread::sleep(std::time::Duration::from_millis(50));
    crate::schnittstelle::ausfuehren("symbol_einfuegen", &serde_json::json!({"pfad": ps, "symbol": "F001", "x": 100, "y": 100})).unwrap();
    h.state_mut().check_external_changes(1.0e6);
    assert_eq!(h.state().tab().doc.objects.len(), 1, "nicht neu geladen");
    // Rückgängig stellt den alten Stand wieder her
    h.state_mut().tab_mut().undo();
    assert_eq!(h.state().tab().doc.objects.len(), 0);
    // mit ungespeicherten eigenen Änderungen wird gefragt statt überschrieben
    h.state_mut().tab_mut().dirty = true;
    std::thread::sleep(std::time::Duration::from_millis(50));
    crate::schnittstelle::ausfuehren("symbol_einfuegen", &serde_json::json!({"pfad": ps, "symbol": "E002", "x": 120, "y": 100})).unwrap();
    h.state_mut().check_external_changes(2.0e6);
    assert_eq!(h.state().dlg.reload_conflict, Some(h.state().active));
    h.run_steps(2);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Ein zweiter Grundriss-Import landet in einem neuen Tab statt über dem ersten Plan.
#[test]
fn import_in_neuen_tab() {
    let mut h = harness();
    let png = {
        let img = image::RgbImage::from_pixel(40, 30, image::Rgb([200, 200, 200]));
        let mut b = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut b), image::ImageFormat::Png).unwrap();
        b
    };
    let datei = |name: &str| crate::plattform::Datei { zweck: crate::plattform::Zweck::Bild, name: name.into(), pfad: None, bytes: png.clone() };
    h.state_mut().datei_verarbeiten(datei("eg.png"));
    assert_eq!(h.state().tabs.len(), 1, "erster Import in den leeren Plan");
    h.state_mut().datei_verarbeiten(datei("og.png"));
    let app = h.state();
    assert_eq!(app.tabs.len(), 2, "zweiter Import in neuen Tab");
    assert_eq!(app.active, 1);
    assert_eq!(app.tabs[1].doc.objects.len(), 1);
    assert_eq!(app.tabs[0].doc.objects.len(), 1);
    assert_eq!(app.tabs[1].name.as_deref(), Some("og"));
}
