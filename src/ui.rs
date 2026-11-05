//! Oberfläche: Menüband, Seitenleisten, Statusleiste, Tastenkürzel.

use eframe::egui::{self, Color32, RichText, Sense, Vec2};

use crate::app::{App, Tool};
use crate::canvas::{self, c32};
use crate::export::Stamp;
use crate::geom::*;
use crate::model::*;
use crate::symbols::{self, Category};
use crate::{ops, presets};

const RIBBON_TABS: [&str; 6] = ["Datei", "Plan", "Einfügen", "Bearbeiten", "Ansicht", "Extras"];

fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
        ui.vertical(|ui| {
            // feste Höchstbreite: große Gruppen brechen intern um, statt das Menüband aufzublähen
            ui.set_max_width(520.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                add(ui);
            });
            ui.label(RichText::new(title).small().weak());
        });
    });
}

pub fn from_c32(c: Color32) -> Color {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    Color(r, g, b, a)
}

fn color_edit(ui: &mut egui::Ui, c: &mut Color) -> bool {
    let mut cc = c32(*c);
    let r = ui.color_edit_button_srgba(&mut cc).changed();
    if r {
        *c = from_c32(cc);
    }
    r
}

impl App {
    fn tool_btn(&mut self, ui: &mut egui::Ui, label: &str, tool: Tool, tip: &str) {
        let active = self.tool == tool;
        if ui.selectable_label(active, label).on_hover_text(tip).clicked() {
            self.draft.clear();
            self.measure = None;
            self.tool = if active { Tool::Select } else { tool };
        }
    }

    pub fn ribbon(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (i, t) in RIBBON_TABS.iter().enumerate() {
                if ui.selectable_label(self.ribbon == i, RichText::new(*t).strong()).clicked() {
                    self.ribbon = i;
                }
            }
            ui.separator();
            ui.label(RichText::new(format!("Werkzeug: {}", self.tool.name())).color(Color32::from_rgb(0, 90, 160)));
        });
        ui.add_space(2.0);
        // Gruppen in einer Zeile; bei schmalem Fenster waagerecht scrollen (wie Office-Menübänder)
        egui::ScrollArea::horizontal().id_salt("ribbon").auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal_top(|ui| match self.ribbon {
                0 => self.ribbon_datei(ui),
                1 => self.ribbon_plan(ui),
                2 => self.ribbon_einfuegen(ui),
                3 => self.ribbon_bearbeiten(ui),
                4 => self.ribbon_ansicht(ui),
                _ => self.ribbon_extras(ui),
            });
        });
    }

    pub fn new_menu(&mut self, ui: &mut egui::Ui) {
        let items: [(&str, FrameKind, PaperFormat, bool); 10] = [
            ("Flucht- und Rettungsplan A3 quer (empfohlen)", FrameKind::FrVollstaendig, PaperFormat::A3, true),
            ("Flucht- und Rettungsplan A3 hoch", FrameKind::FrVollstaendig, PaperFormat::A3, false),
            ("Flucht- und Rettungsplan zweizeilig A3", FrameKind::FrZweizeilig, PaperFormat::A3, true),
            ("Flucht- und Rettungsplan A4 (Einzelraum, z. B. Hotelzimmer)", FrameKind::FrVollstaendig, PaperFormat::A4, true),
            ("Feuerwehrplan DIN 14095 (A3 quer)", FrameKind::Feuerwehrplan, PaperFormat::A3, true),
            ("Brandschutzplan A3 quer", FrameKind::BrandschutzVollstaendig, PaperFormat::A3, true),
            ("Brandschutzplan A2 quer", FrameKind::BrandschutzVollstaendig, PaperFormat::A2, true),
            ("Feuerwehr-Laufkarte A4 quer", FrameKind::Laufkarte, PaperFormat::A4, true),
            ("Feuerwehr-Laufkarte A3 quer", FrameKind::Laufkarte, PaperFormat::A3, true),
            ("Leeres Blatt A3 quer", FrameKind::Ohne, PaperFormat::A3, true),
        ];
        for (name, f, p, l) in items {
            if ui.button(name).clicked() {
                self.new_doc(f, p, l);
                ui.close();
            }
        }
    }

    fn ribbon_datei(&mut self, ui: &mut egui::Ui) {
        group(ui, "Plan", |ui| {
            ui.menu_button("Neu", |ui| self.new_menu(ui));
            if ui.button("Öffnen …").clicked() {
                self.open_dialog();
            }
            if ui.button("Speichern").clicked() {
                self.save(false);
            }
            if ui.button("Speichern unter …").clicked() {
                self.save(true);
            }
            if ui.button("Schließen").clicked() {
                self.close_tab(self.active, false);
            }
        });
        group(ui, "Zuletzt geöffnet", |ui| {
            let recent = self.settings.recent.clone();
            ui.menu_button("Letzte Pläne …", |ui| {
                if recent.is_empty() {
                    ui.label("Noch keine.");
                }
                for p in recent {
                    if ui.button(p.display().to_string()).clicked() {
                        self.open_path(&p);
                        ui.close();
                    }
                }
            });
            if ui.button("Musterpläne …").clicked() {
                self.dlg.samples = true;
            }
            if ui.button("Willkommen …").clicked() {
                self.dlg.welcome = true;
            }
        });
        group(ui, "Ausgabe", |ui| {
            if ui.button("Drucken").on_hover_text("Öffnet den Plan als PDF in der Druckvorschau").clicked() {
                self.print();
            }
            self.pdf_menu(ui);
        });
        group(ui, "Programm", |ui| {
            if ui.button("Beenden").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }

    fn pdf_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("PDF erstellen …", |ui| {
            if ui.button("Erstellen").clicked() {
                self.export_pdf(Stamp::Keiner);
                ui.close();
            }
            if ui.button("Entwurf erstellen (Überdruck ENTWURF)").clicked() {
                self.export_pdf(Stamp::Entwurf);
                ui.close();
            }
            if ui.button("Muster erstellen (Überdruck MUSTER)").clicked() {
                self.export_pdf(Stamp::Muster);
                ui.close();
            }
            if ui.button("Vorabzug erstellen (Überdruck VORABZUG)").clicked() {
                self.export_pdf(Stamp::Vorabzug);
                ui.close();
            }
            ui.separator();
            if ui.button("Pläne zusammenführen …").clicked() {
                self.dlg.merge = true;
                self.dlg.merge_sel = vec![true; self.tabs.len()];
                ui.close();
            }
        });
    }

    fn ribbon_plan(&mut self, ui: &mut egui::Ui) {
        group(ui, "Plan", |ui| {
            ui.menu_button("Neu", |ui| self.new_menu(ui));
            if ui.button("Öffnen").clicked() {
                self.open_dialog();
            }
            if ui.button("Speichern").clicked() {
                self.save(false);
            }
            ui.menu_button("Importieren …", |ui| {
                for (k, n) in [("bild", "Grundriss als Bild (PNG/JPG/BMP/TIF) …"), ("pdf", "Grundriss aus PDF …"), ("dxf", "AutoCAD DXF …"), ("dwg", "AutoCAD DWG … (ODA File Converter)"), ("plan", "Anderen Plan einfügen …"), ("svg", "Eigenes Symbol (SVG) …")] {
                    // DWG braucht ein externes Programm – nur Desktop
                    if k == "dwg" && crate::plattform::IST_WEB {
                        continue;
                    }
                    if ui.button(n).clicked() {
                        self.import_dialog(k);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Exportieren …", |ui| {
                for (k, n) in [("dxf", "AutoCAD DXF …"), ("dwg", "AutoCAD DWG … (ODA File Converter)"), ("svg", "SVG-Vektorgrafik …")] {
                    if k == "dwg" && crate::plattform::IST_WEB {
                        continue;
                    }
                    if ui.button(n).clicked() {
                        self.export_other(k);
                        ui.close();
                    }
                }
                if ui.button("PNG-Bild …").clicked() {
                    self.dlg.png = true;
                    ui.close();
                }
            });
            if ui.button("Drucken").clicked() {
                self.print();
            }
            self.pdf_menu(ui);
            if ui.button("Plan prüfen …").on_hover_text("Pflichtinhalte nach DIN ISO 23601 / DIN 14095 prüfen").clicked() {
                self.dlg.check = true;
            }
        });
        group(ui, "Plan einrichten", |ui| {
            let mut page = self.doc().page;
            let mut frame = self.doc().frame;
            let mut scale = self.doc().scale_n;
            egui::ComboBox::from_id_salt("fmt").selected_text(page.format.name()).show_ui(ui, |ui| {
                for f in PaperFormat::LISTE {
                    ui.selectable_value(&mut page.format, f, f.name());
                }
            });
            ui.selectable_value(&mut page.landscape, true, "Quer");
            ui.selectable_value(&mut page.landscape, false, "Hoch");
            egui::ComboBox::from_id_salt("frame").width(220.0).selected_text(frame.name()).show_ui(ui, |ui| {
                for f in FrameKind::ALL {
                    ui.selectable_value(&mut frame, f, f.name());
                }
            });
            if ui.button("Rahmen und Titel …").clicked() {
                self.dlg.page = true;
            }
            ui.label("Maßstab 1:");
            ui.add(egui::DragValue::new(&mut scale).range(1.0..=100000.0).speed(5.0));
            let changed = page != self.doc().page || frame != self.doc().frame || scale != self.doc().scale_n;
            if changed {
                let tab = self.tab_mut();
                tab.push_undo();
                if frame != tab.doc.frame {
                    apply_frame_defaults(&mut tab.doc, frame);
                }
                tab.doc.page = page;
                tab.doc.frame = frame;
                tab.doc.scale_n = scale;
                tab.fit = true;
            }
            self.tool_btn(ui, "Maßstab durch Entfernung", Tool::ScaleByDistance, "Zwei Punkte mit bekanntem Abstand anklicken");
            if ui.button("Bereinigen").on_hover_text("Leere Elemente und ungenutzte Legendeneinträge entfernen").clicked() {
                self.cleanup();
            }
            if ui.button("Grundriss extrahieren …").on_hover_text("Wände automatisch aus einem Bild erkennen").clicked() {
                self.dlg.extract = true;
            }
            if ui.button("Legende …").clicked() {
                self.dlg.legend = true;
            }
        });
        group(ui, "Lagerichtig ausrichten", |ui| {
            for (label, deg) in [("Inhalt −90°", -90.0), ("Inhalt +90°", 90.0), ("Inhalt 180°", 180.0)] {
                if ui.button(label).on_hover_text("Gesamten Planinhalt um den Planbereich drehen (Standort-Ausrichtung)").clicked() {
                    self.rotate_content(deg);
                }
            }
            if ui.button("Plan duplizieren").on_hover_text("Kopie für einen weiteren Aushangort (Standort versetzen)").clicked() {
                self.duplicate_plan();
            }
        });
    }

    fn ribbon_einfuegen(&mut self, ui: &mut egui::Ui) {
        group(ui, "Einfügen", |ui| {
            self.tool_btn(ui, "Legende", Tool::Legend, "Legende als Rahmen aufziehen (wird automatisch befüllt)");
            if ui.button("Legende …").on_hover_text("Legenden-Assistent").clicked() {
                self.dlg.legend = true;
            }
            if ui.button("Rahmen für Laufkarten").on_hover_text("Planrahmen auf Feuerwehr-Laufkarte (DIN 14675) umstellen").clicked() {
                let tab = self.tab_mut();
                tab.push_undo();
                apply_frame_defaults(&mut tab.doc, FrameKind::Laufkarte);
                tab.doc.frame = FrameKind::Laufkarte;
                tab.fit = true;
            }
            if ui.button("Rahmen und Titel …").clicked() {
                self.dlg.page = true;
            }
            self.tool_btn(ui, "Container", Tool::Container, "Kasten (z. B. für Textblöcke, Übersichtsplan, Stempelfeld)");
        });
        group(ui, "Auswahl und Zoom", |ui| {
            self.tool_btn(ui, "Auswahl", Tool::Select, "Elemente auswählen und bearbeiten (Esc)");
            self.tool_btn(ui, "Verschieben", Tool::Pan, "Ansicht verschieben (auch mittlere Maustaste oder Leertaste)");
            self.tool_btn(ui, "Bereich zoomen", Tool::ZoomBox, "Auf einen Bereich zoomen");
        });
        group(ui, "Zeichnen", |ui| {
            self.tool_btn(ui, "Linie", Tool::Line, "Linie");
            self.tool_btn(ui, "Kreis", Tool::Circle, "Kreis (Mittelpunkt, Radius)");
            self.tool_btn(ui, "Ellipse", Tool::Ellipse, "Ellipse");
            self.tool_btn(ui, "Bogen", Tool::Arc, "Kreisbogen");
            self.tool_btn(ui, "Rechteck", Tool::Rect, "Rechteck");
            self.tool_btn(ui, "Polygon", Tool::Polygon, "Geschlossenes Polygon");
            self.tool_btn(ui, "N-Eck", Tool::NGon, "Regelmäßiges Vieleck");
            self.tool_btn(ui, "Linienzug", Tool::Polyline, "Offener Linienzug");
            self.tool_btn(ui, "Schlauch", Tool::Hose, "Schlauchleitung (Doppellinie)");
            self.tool_btn(ui, "Spline", Tool::Spline, "Kurve durch Punkte");
            self.tool_btn(ui, "Text", Tool::Text, "Text");
            if ui.button("Bild").on_hover_text("Bild einfügen").clicked() {
                self.import_dialog("bild");
            }
            if ui.button("Eingebettetes PDF").on_hover_text("PDF-Seite einfügen").clicked() {
                self.import_dialog("pdf");
            }
            self.tool_btn(ui, "Maßlinie", Tool::Dimension, "Bemaßung in Metern (nach Planmaßstab)");
            if ui.button("Externe Referenz").on_hover_text("Anderen Plan als Gruppe einfügen").clicked() {
                self.import_dialog("plan");
            }
            self.tool_btn(ui, "Tabelle", Tool::Table, "Tabelle");
            self.tool_btn(ui, "Notiz", Tool::Note, "Notiz (wird nicht gedruckt)");
        });
        group(ui, "Bau und Fluchtwege", |ui| {
            self.tool_btn(ui, "Wand", Tool::Wall, "Wand (Linienzug mit Wandstärke)");
            self.tool_btn(ui, "Wand (orth.)", Tool::WallOrtho, "Wand – nur waagerecht/senkrecht");
            self.tool_btn(ui, "Raum", Tool::Room, "Raum mit Bezeichnung");
            self.tool_btn(ui, "Raummaß", Tool::RoomArea, "Raumfläche (m²) anzeigen");
            ui.menu_button("Weitere …", |ui| {
                for (n, t) in [
                    ("Revisionswolke", Tool::Cloud),
                    ("Gleis", Tool::Rail),
                    ("Maßstabsleiste", Tool::ScaleBar),
                    ("Zaun", Tool::Fence),
                    ("QR-Code", Tool::Qr),
                    ("Doppeltür", Tool::DoubleDoor),
                    ("Hilfslinie", Tool::Guide),
                ] {
                    if ui.button(n).clicked() {
                        self.tool = t;
                        self.draft.clear();
                        ui.close();
                    }
                }
                if ui.button("Stuhl / Tisch (Modelle) …").clicked() {
                    self.lib_tab = 0;
                    self.lib_cat = Category::ALL.iter().position(|c| *c == Category::Einrichtung).unwrap_or(0);
                    ui.close();
                }
            });
            self.tool_btn(ui, "Fenster", Tool::Window, "Fenster in Wand");
            self.tool_btn(ui, "Tür", Tool::Door, "Tür mit Schwenkbereich");
            self.tool_btn(ui, "Treppe", Tool::Stairs, "Treppe mit Laufrichtung");
            ui.menu_button("Fluchtwege …", |ui| {
                for (n, t) in [("Fluchtweg (mit Pfeilen)", Tool::Route), ("Fluchtweg (Bogen)", Tool::RouteCurved), ("Laufweg der Feuerwehr (rot)", Tool::Laufweg)] {
                    if ui.button(n).clicked() {
                        self.tool = t;
                        self.draft.clear();
                        ui.close();
                    }
                }
                ui.separator();
                for p in presets::PRESETS.iter().filter(|p| p.group == presets::Group::Flaechen) {
                    if ui.button(format!("Fläche: {}", p.name)).clicked() {
                        self.tool = Tool::Area(p.key.into());
                        self.draft.clear();
                        ui.close();
                    }
                }
            });
            self.tool_btn(ui, "Richtungspfeil", Tool::Symbol("PFEIL_GRUEN".into()), "Richtungspfeil Rettungsweg");
            self.tool_btn(ui, "Planraster", Tool::Grid, "Planraster mit Feldbezeichnungen (A1 …)");
            self.tool_btn(ui, "Geschoss-Schnitt", Tool::FloorSection, "Geschossübersicht für Feuerwehrpläne (DIN 14095), dargestelltes Geschoss markiert");
        });
    }

    fn ribbon_bearbeiten(&mut self, ui: &mut egui::Ui) {
        let has_sel = !self.tab().sel.is_empty();
        group(ui, "Bearbeiten", |ui| {
            if ui.add_enabled(has_sel, egui::Button::new("Löschen")).clicked() {
                self.delete_selection();
            }
            if ui.add_enabled(!self.tab().undo.is_empty(), egui::Button::new("Rückgängig")).clicked() {
                self.tab_mut().undo();
            }
            if ui.add_enabled(!self.tab().redo.is_empty(), egui::Button::new("Wiederholen")).clicked() {
                self.tab_mut().redo();
            }
            ui.menu_button("Polygon füllen …", |ui| {
                for p in presets::PRESETS.iter() {
                    if ui.button(p.name).clicked() {
                        self.tool = Tool::FillPolygon(p.key.into());
                        ui.close();
                    }
                }
            });
            self.tool_btn(ui, "Plan schneiden", Tool::CutPlan, "Alles außerhalb eines Rechtecks entfernen");
            self.tool_btn(ui, "Punkt einfügen", Tool::InsertPoint, "Punkt in Linienzug/Wand einfügen");
            if ui.button("Symbol-Manager …").clicked() {
                self.dlg.symbol_manager = true;
            }
            if ui.button("Suchen / Ersetzen …").clicked() {
                self.dlg.search = true;
            }
        });
        group(ui, "Zwischenablage", |ui| {
            if ui.add_enabled(!self.clipboard.is_empty(), egui::Button::new("Einfügen")).clicked() {
                self.paste(Pos::new(5.0, 5.0));
            }
            if ui.add_enabled(has_sel, egui::Button::new("Ausschneiden")).clicked() {
                self.cut();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Kopieren")).clicked() {
                self.copy();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Duplizieren")).clicked() {
                self.duplicate();
            }
        });
        group(ui, "Auswahl", |ui| {
            if ui.button("Alles").clicked() {
                self.select_all();
            }
            if ui.button("Invertieren").clicked() {
                self.invert_selection();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Nach Inhalt (gleiche)")).clicked() {
                self.select_similar();
            }
        });
        group(ui, "Verändern", |ui| {
            if ui.add_enabled(has_sel, egui::Button::new("Gruppieren")).clicked() {
                self.group();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Gruppe aufheben")).clicked() {
                self.ungroup();
            }
            if ui
                .button("Symbole erkennen")
                .on_hover_text("Einzelteile importierter Symbole (Hintergrund, Rahmen, Zeichen, Text) zu je einer Gruppe zusammenfassen – in der Auswahl oder, ohne Auswahl, im ganzen Plan")
                .clicked()
            {
                self.symbole_erkennen();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Mehrfachkopie …")).clicked() {
                self.dlg.multicopy = true;
            }
            if ui.add_enabled(has_sel, egui::Button::new("Versatz (Offset) …")).on_hover_text("Parallele Kopie im festen Abstand").clicked() {
                self.dlg.offset = true;
            }
            if ui.add_enabled(has_sel, egui::Button::new("Drehen -90°")).clicked() {
                self.transform_sel(|k, c| ops::rotate(k, c, -90.0));
            }
            if ui.add_enabled(has_sel, egui::Button::new("Drehen +90°")).clicked() {
                self.transform_sel(|k, c| ops::rotate(k, c, 90.0));
            }
            if ui.add_enabled(has_sel, egui::Button::new("Spiegeln waagerecht")).clicked() {
                self.transform_sel(|k, c| ops::mirror(k, c, true));
            }
            if ui.add_enabled(has_sel, egui::Button::new("Spiegeln senkrecht")).clicked() {
                self.transform_sel(|k, c| ops::mirror(k, c, false));
            }
            if ui.add_enabled(has_sel, egui::Button::new("Nach vorne")).clicked() {
                self.order(true, true);
            }
            if ui.add_enabled(has_sel, egui::Button::new("Nach hinten")).clicked() {
                self.order(false, true);
            }
            ui.menu_button("Ausrichten …", |ui| {
                for (i, n) in ["Links", "Horizontal zentriert", "Rechts", "Oben", "Vertikal zentriert", "Unten"].iter().enumerate() {
                    if ui.button(*n).clicked() {
                        self.align(i as u8);
                        ui.close();
                    }
                }
            });
        });
        group(ui, "Wand", |ui| {
            self.tool_btn(ui, "Segment ausschneiden", Tool::SegmentCut, "Wandsegment entfernen (z. B. für Durchgänge)");
            self.tool_btn(ui, "Schnittpunkt einfügen", Tool::InsertPoint, "Zusätzlichen Eckpunkt einfügen");
            self.tool_btn(ui, "Ecke abrunden", Tool::Fillet, "Ecke mit Radius abrunden");
            ui.label("Radius");
            ui.add(egui::DragValue::new(&mut self.settings.fillet_mm).range(0.5..=100.0).suffix(" mm"));
        });
        group(ui, "Messen", |ui| {
            self.tool_btn(ui, "Kurve Feuerwehrzufahrt", Tool::FireCurve, "Kurve nach DIN 14090 (Außenradius ≥ 10,5 m)");
            self.tool_btn(ui, "Distanz [m]", Tool::MeasureM, "Strecke im Planmaßstab");
            self.tool_btn(ui, "Distanz [mm]", Tool::MeasureMm, "Strecke auf dem Blatt");
            self.tool_btn(ui, "Winkel", Tool::MeasureAngle, "Winkel messen");
        });
    }

    fn ribbon_ansicht(&mut self, ui: &mut egui::Ui) {
        group(ui, "Zoom", |ui| {
            if ui.button("Hereinzoomen").clicked() {
                self.tab_mut().zoom *= 1.25;
            }
            if ui.button("Herauszoomen").clicked() {
                self.tab_mut().zoom /= 1.25;
            }
            if ui.button("Automatisch anpassen").clicked() {
                self.tab_mut().fit = true;
            }
            self.tool_btn(ui, "In Bereich zoomen", Tool::ZoomBox, "Rechteck aufziehen");
            if ui.add_enabled(!self.tab().sel.is_empty(), egui::Button::new("Auf Auswahl")).clicked() {
                let b = self.tab().sel_bounds().expand(10.0);
                self.zoom_to(b);
            }
        });
        group(ui, "Willkommensdialog", |ui| {
            if ui.button("Anzeigen").clicked() {
                self.dlg.welcome = true;
            }
        });
        group(ui, "Raster und Fang", |ui| {
            ui.checkbox(&mut self.settings.grid_show, "Gitternetz");
            ui.checkbox(&mut self.settings.grid_snap, "Am Raster fangen");
            ui.checkbox(&mut self.settings.point_snap, "Punktfang");
            ui.add(egui::DragValue::new(&mut self.settings.grid_mm).range(0.5..=100.0).speed(0.5).suffix(" mm"));
        });
        group(ui, "Hilfen", |ui| {
            ui.checkbox(&mut self.settings.rulers, "Lineal");
            ui.checkbox(&mut self.settings.guides, "Hilfslinien");
            self.tool_btn(ui, "Hilfslinie setzen", Tool::Guide, "Klick: waagerecht, Umschalt+Klick: senkrecht");
            if ui.button("Hilfslinien löschen").clicked() {
                let tab = self.tab_mut();
                tab.push_undo();
                tab.doc.guides.clear();
            }
        });
        group(ui, "Hintergrund und Qualität", |ui| {
            let mut c = Color32::from_rgb(self.settings.bg[0], self.settings.bg[1], self.settings.bg[2]);
            if ui.color_edit_button_srgba(&mut c).changed() {
                self.settings.bg = [c.r(), c.g(), c.b()];
            }
            let before = self.settings.quality_high;
            ui.selectable_value(&mut self.settings.quality_high, true, "Hoch");
            ui.selectable_value(&mut self.settings.quality_high, false, "Gering");
            if before != self.settings.quality_high {
                self.textures.quality_high = self.settings.quality_high;
                self.textures.clear_symbols();
            }
        });
    }

    fn ribbon_extras(&mut self, ui: &mut egui::Ui) {
        group(ui, "Optionen", |ui| {
            if ui.button("OSM-Karte …").on_hover_text("Kartenausschnitt aus OpenStreetMap einfügen").clicked() {
                self.dlg.osm = true;
            }
            if ui.button("Flurkarte (WMS) …").on_hover_text("Maßstäbliche Flurkarte (ALKIS) oder amtliche Karte über WMS einfügen").clicked() {
                self.dlg.wms = true;
            }
            if ui.button("Optionen …").clicked() {
                self.dlg.options = true;
            }
            if ui.button("Archivieren …").on_hover_text("Plan inkl. PDF als ZIP sichern").clicked() {
                self.archive();
            }
            if ui.button("Dearchivieren …").clicked() {
                self.unarchive();
            }
        });
        group(ui, "Vorlagen", |ui| {
            if ui.button("Brandschutzordnung (Teil A) …").clicked() {
                self.dlg.bso = true;
            }
            if ui.button("Musterpläne …").clicked() {
                self.dlg.samples = true;
            }
            if ui.button("Eigenes Symbol (SVG) …").clicked() {
                self.import_dialog("svg");
            }
            if ui.button("Logo für Planrahmen …").clicked() {
                self.import_dialog("logo");
            }
        });
        group(ui, "Hilfe", |ui| {
            if ui.button("Benutzerhilfe").clicked() {
                self.dlg.help = true;
            }
            if ui.button("Über").clicked() {
                self.dlg.about = true;
            }
        });
    }

    // ---------- linke Leiste ----------

    pub fn library_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (i, n) in ["Symbole", "Flächen", "Eigene"].iter().enumerate() {
                if ui.selectable_label(self.lib_tab == i, *n).clicked() {
                    self.lib_tab = i;
                }
            }
        });
        ui.separator();
        let ctx = ui.ctx().clone();
        match self.lib_tab {
            0 => {
                ui.horizontal(|ui| {
                    ui.label("Suche");
                    ui.add(egui::TextEdit::singleline(&mut self.lib_search).hint_text("Nach Symbolen suchen …").desired_width(f32::INFINITY));
                });
                let searching = !self.lib_search.trim().is_empty();
                if !searching {
                    egui::ComboBox::from_id_salt("cat").width(ui.available_width()).selected_text(Category::ALL[self.lib_cat].name()).show_ui(ui, |ui| {
                        for (i, c) in Category::ALL.iter().enumerate() {
                            ui.selectable_value(&mut self.lib_cat, i, c.name());
                        }
                    });
                }
                let q = self.lib_search.to_lowercase();
                let cat = Category::ALL[self.lib_cat];
                let list: Vec<&symbols::SymbolDef> = symbols::library().iter().filter(|s| if searching { s.name.to_lowercase().contains(&q) || s.id.to_lowercase().contains(&q) } else { s.cat == cat }).collect();
                egui::ScrollArea::vertical().id_salt("symlist").show(ui, |ui| {
                    let cell = 74.0;
                    let cols = ((ui.available_width() / cell).floor() as usize).max(1);
                    egui::Grid::new("symgrid").spacing(Vec2::new(4.0, 4.0)).show(ui, |ui| {
                        for (i, s) in list.iter().enumerate() {
                            let active = self.tool == Tool::Symbol(s.id.into());
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(cell - 4.0, 70.0), Sense::click());
                            if active || resp.hovered() {
                                let v = ui.visuals();
                                ui.painter().rect_filled(r, 4.0, if active { v.selection.bg_fill.gamma_multiply(0.5) } else { v.widgets.hovered.weak_bg_fill });
                            }
                            let icon = egui::Rect::from_center_size(egui::pos2(r.center().x, r.min.y + 22.0), Vec2::splat(36.0));
                            let doc = &self.tabs[self.active].doc;
                            canvas::paint_symbol_icon(ui.painter(), &ctx, &mut self.textures, doc, s.id, icon);
                            // Beschriftung höchstens zweizeilig, in Textfarbe des Farbschemas
                            let mut job = egui::text::LayoutJob::simple(s.name.to_string(), egui::FontId::proportional(10.0), ui.visuals().text_color(), cell - 6.0);
                            job.wrap.max_rows = 2;
                            job.wrap.break_anywhere = false;
                            job.halign = egui::Align::Center;
                            let galley = ui.painter().layout_job(job);
                            ui.painter().galley(egui::pos2(r.center().x, r.min.y + 42.0), galley, ui.visuals().text_color());
                            if resp.on_hover_text(format!("{} ({})", s.name, s.id)).clicked() {
                                self.tool = Tool::Symbol(s.id.into());
                                self.draft.clear();
                            }
                            if (i + 1) % cols == 0 {
                                ui.end_row();
                            }
                        }
                    });
                });
            }
            1 => {
                egui::ScrollArea::vertical().id_salt("arealist").show(ui, |ui| {
                    for (grp, title) in [(presets::Group::Flaechen, "Fluchtwege / Feuerwehrplan (DIN 14095)"), (presets::Group::Brandschutz, "Brandschutz (Feuerwiderstand, Baustoffe)")] {
                        ui.label(RichText::new(title).strong());
                        for p in presets::PRESETS.iter().filter(|p| p.group == grp) {
                            ui.horizontal(|ui| {
                                let (r, _) = ui.allocate_exact_size(Vec2::new(34.0, 18.0), Sense::hover());
                                let doc = &self.tabs[self.active].doc;
                                canvas::paint_style_icon(ui.painter(), &ctx, &mut self.textures, doc, &p.style, r);
                                let active = self.tool == Tool::Area(p.key.into());
                                if ui.selectable_label(active, p.name).on_hover_text("Fläche zeichnen (Punkte klicken). Rechtsklick: Polygon füllen").clicked() {
                                    self.tool = Tool::Area(p.key.into());
                                    self.draft.clear();
                                }
                                if ui.small_button("füllen").on_hover_text("In umschlossene Fläche klicken (Polygon füllen)").clicked() {
                                    self.tool = Tool::FillPolygon(p.key.into());
                                }
                            });
                        }
                        ui.add_space(6.0);
                    }
                });
            }
            _ => {
                if ui.button("SVG-Symbol importieren …").clicked() {
                    self.import_dialog("svg");
                }
                let customs = self.doc().custom_symbols.clone();
                if customs.is_empty() {
                    ui.label("Noch keine eigenen Symbole in diesem Plan.");
                }
                for c in customs {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::hover());
                        let doc = &self.tabs[self.active].doc;
                        canvas::paint_symbol_icon(ui.painter(), &ctx, &mut self.textures, doc, &c.id, r);
                        if ui.selectable_label(self.tool == Tool::Symbol(c.id.clone()), &c.name).clicked() {
                            self.tool = Tool::Symbol(c.id.clone());
                        }
                    });
                }
            }
        }
    }

    // ---------- rechte Leiste ----------

    pub fn right_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().id_salt("right").show(ui, |ui| {
            egui::CollapsingHeader::new(RichText::new("Eigenschaften").strong()).default_open(true).show(ui, |ui| self.properties(ui));
            egui::CollapsingHeader::new(RichText::new("Planangaben").strong()).default_open(true).show(ui, |ui| self.plan_info(ui));
            egui::CollapsingHeader::new(RichText::new("Ebenen").strong()).default_open(true).show(ui, |ui| self.layers_ui(ui));
        });
    }

    fn plan_info(&mut self, ui: &mut egui::Ui) {
        let mut info = self.doc().info.clone();
        let frame = self.doc().frame;
        egui::Grid::new("info").num_columns(2).striped(true).show(ui, |ui| {
            let row = |ui: &mut egui::Ui, l: &str, v: &mut String| {
                ui.label(l);
                ui.add(egui::TextEdit::singleline(v).desired_width(170.0));
                ui.end_row();
            };
            row(ui, "Objekt", &mut info.objekt);
            ui.label("Anschrift");
            ui.add(egui::TextEdit::multiline(&mut info.anschrift).desired_rows(2).desired_width(170.0));
            ui.end_row();
            row(ui, "Geschoss", &mut info.geschoss);
            if frame == FrameKind::Feuerwehrplan {
                row(ui, "Planart", &mut info.planart);
            }
            row(ui, "Auftraggeber", &mut info.auftraggeber);
            row(ui, "Verfasser", &mut info.ersteller);
            row(ui, "Plannummer", &mut info.plannummer);
            row(ui, "Stand", &mut info.stand);
            row(ui, "Revision", &mut info.revision);
            row(ui, "Sonstiges", &mut info.sonstiges);
            row(ui, "Notruf", &mut info.notruf);
            ui.label("Sammelstelle");
            ui.add(egui::TextEdit::multiline(&mut info.sammelstelle).desired_rows(2).desired_width(170.0));
            ui.end_row();
            if frame == FrameKind::Laufkarte {
                row(ui, "Meldergruppe", &mut info.meldergruppe);
                row(ui, "Melder", &mut info.melder);
                row(ui, "Meldertyp", &mut info.meldertyp);
                row(ui, "Raum", &mut info.raum);
                row(ui, "BMZ-Standort", &mut info.bmz_standort);
            }
        });
        if info != self.doc().info {
            let now = ui.input(|i| i.time);
            let tab = self.tab_mut();
            let before = tab.doc.clone();
            tab.doc.info = info;
            tab.push_undo_coalesced(before, u64::MAX, now);
        }
    }

    fn layers_ui(&mut self, ui: &mut egui::Ui) {
        let mut doc_layers = self.doc().layers.clone();
        let mut active = self.doc().active_layer;
        let mut remove = None;
        let mut swap = None;
        let n = doc_layers.len();
        for (i, l) in doc_layers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.radio_value(&mut active, l.id, "").on_hover_text("Aktive Ebene");
                ui.checkbox(&mut l.visible, "").on_hover_text("Sichtbar");
                let icon = if l.locked { "gesperrt" } else { "frei" };
                ui.toggle_value(&mut l.locked, icon).on_hover_text("Gesperrt");
                ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(95.0));
                let swatch = match l.color {
                    Some(c) => RichText::new("Farbe").strong().color(Color32::from_rgb(c.0, c.1, c.2)),
                    None => RichText::new("Farbe").weak(),
                };
                ui.menu_button(swatch, |ui| {
                    let mut on = l.color.is_some();
                    if ui.checkbox(&mut on, "Ebenenfarbe verwenden").changed() {
                        l.color = on.then_some(farbe::ROT);
                    }
                    if let Some(c) = &mut l.color {
                        let mut c32 = Color32::from_rgb(c.0, c.1, c.2);
                        if ui.color_edit_button_srgba(&mut c32).changed() {
                            *c = from_c32(c32);
                        }
                    }
                    ui.label(RichText::new("Überschreibt Linien- und Textfarbe aller Objekte auf dieser Ebene.").weak().small());
                })
                .response
                .on_hover_text("Ebenenfarbe");
                if ui.add_enabled(i > 0, egui::Button::new("auf").small()).clicked() {
                    swap = Some((i, i - 1));
                }
                if ui.add_enabled(i + 1 < n, egui::Button::new("ab").small()).clicked() {
                    swap = Some((i, i + 1));
                }
                if ui.add_enabled(n > 1, egui::Button::new("x").small()).on_hover_text("Ebene löschen (Elemente wandern auf die erste Ebene)").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some((a, b)) = swap {
            doc_layers.swap(a, b);
        }
        let add = ui.button("+ Ebene").clicked();
        if doc_layers != self.doc().layers || active != self.doc().active_layer || remove.is_some() || add {
            let tab = self.tab_mut();
            tab.push_undo();
            if let Some(i) = remove {
                let id = doc_layers[i].id;
                doc_layers.remove(i);
                let first = doc_layers[0].id;
                for o in tab.doc.objects.iter_mut().filter(|o| o.layer == id) {
                    o.layer = first;
                }
                if active == id {
                    active = first;
                }
            }
            if add {
                let id = doc_layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
                doc_layers.push(Layer { id, name: format!("Ebene {id}"), visible: true, locked: false, color: None });
                active = id;
            }
            tab.doc.layers = doc_layers;
            tab.doc.active_layer = active;
        }
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        let tab = &self.tabs[self.active];
        if tab.sel.is_empty() {
            ui.label("Nichts ausgewählt.");
            ui.label(RichText::new(self.tool.hint()).weak().small());
            return;
        }
        let now = ui.input(|i| i.time);
        let layers = tab.doc.layers.clone();
        let scale_n = tab.doc.scale_n;
        if tab.sel.len() > 1 {
            ui.label(format!("{} Elemente ausgewählt", tab.sel.len()));
            let first = tab.doc.objects.iter().find(|o| tab.sel.contains(&o.id)).cloned();
            if let Some(first) = first {
                let mut st = first.style;
                let mut layer = first.layer;
                let ch1 = style_ui(ui, &mut st);
                let ch2 = layer_combo(ui, &layers, &mut layer);
                if ch1 || ch2 {
                    let tab = self.tab_mut();
                    let before = tab.doc.clone();
                    let sel = tab.sel.clone();
                    for o in tab.doc.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
                        if ch1 {
                            o.style = st;
                        }
                        if ch2 {
                            o.layer = layer;
                        }
                    }
                    tab.push_undo_coalesced(before, u64::MAX - 1, now);
                }
            }
            return;
        }
        let id = tab.sel[0];
        let Some(i) = tab.doc.index_of(id) else { return };
        let mut o = tab.doc.objects[i].clone();
        ui.label(RichText::new(ops::kind_name(&o.kind)).strong());
        let focus = std::mem::take(&mut self.focus_text);
        kind_ui(ui, &mut o.kind, focus, scale_n);
        ui.separator();
        if !matches!(o.kind, ObjKind::Symbol { .. } | ObjKind::Image { .. } | ObjKind::Group { .. }) {
            style_ui(ui, &mut o.style);
        }
        if let ObjKind::Area { preset, .. } = &mut o.kind {
            let mut p = preset.clone();
            egui::ComboBox::from_id_salt("preset").width(200.0).selected_text(presets::find(&p).map(|x| x.name).unwrap_or("–")).show_ui(ui, |ui| {
                for pr in presets::PRESETS {
                    ui.selectable_value(&mut p, pr.key.to_string(), pr.name);
                }
            });
            if p != *preset {
                *preset = p.clone();
                if let Some(pr) = presets::find(&p) {
                    o.style = pr.style;
                }
            }
        }
        layer_combo(ui, &layers, &mut o.layer);
        ui.horizontal(|ui| {
            ui.checkbox(&mut o.print, "Drucken");
            ui.checkbox(&mut o.locked, "Gesperrt");
        });
        ui.horizontal(|ui| {
            if ui.button("Drehen -90°").clicked() {
                let c = ops::bounds(&o.kind).center();
                ops::rotate(&mut o.kind, c, -90.0);
            }
            if ui.button("Drehen +90°").clicked() {
                let c = ops::bounds(&o.kind).center();
                ops::rotate(&mut o.kind, c, 90.0);
            }
            if ui.button("Drehen +45°").clicked() {
                let c = ops::bounds(&o.kind).center();
                ops::rotate(&mut o.kind, c, 45.0);
            }
            if ui.button("Spiegeln").on_hover_text("Spiegeln").clicked() {
                let c = ops::bounds(&o.kind).center();
                ops::mirror(&mut o.kind, c, true);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Größe");
            for (l, f) in [("50 %", 0.5), ("80 %", 0.8), ("125 %", 1.25), ("200 %", 2.0)] {
                if ui.button(l).clicked() {
                    let c = ops::bounds(&o.kind).center();
                    ops::scale(&mut o.kind, c, f);
                }
            }
        });
        let tab = self.tab_mut();
        if o != tab.doc.objects[i] {
            let before = tab.doc.clone();
            tab.doc.objects[i] = o;
            tab.push_undo_coalesced(before, id, now);
            ops::sync_legend(&mut tab.doc);
        }
        if ui.button("Löschen").clicked() {
            self.delete_selection();
        }
    }

    // ---------- Tastatur ----------

    pub fn shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.memory(|m| m.focused().is_some());
        use egui::Key;
        let ev = ctx.input(|i| {
            let c = i.modifiers.command;
            let s = i.modifiers.shift;
            (
                [
                    c && i.key_pressed(Key::N),
                    c && i.key_pressed(Key::O),
                    c && !s && i.key_pressed(Key::S),
                    c && s && i.key_pressed(Key::S),
                    c && i.key_pressed(Key::P),
                    c && i.key_pressed(Key::Z),
                    c && i.key_pressed(Key::Y),
                    c && i.key_pressed(Key::C),
                    c && i.key_pressed(Key::X),
                    c && i.key_pressed(Key::V),
                    c && i.key_pressed(Key::D),
                    c && i.key_pressed(Key::A),
                    c && !s && i.key_pressed(Key::G),
                    c && s && i.key_pressed(Key::G),
                    i.key_pressed(Key::Delete),
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::Enter),
                    !c && i.key_pressed(Key::R),
                    c && i.key_pressed(Key::F),
                    !c && i.key_pressed(Key::F),
                ],
                [i.key_pressed(Key::ArrowLeft), i.key_pressed(Key::ArrowRight), i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown)],
                s,
            )
        });
        let ([new, open, save, save_as, print, undo, redo, copy, cut, paste, dup, all, grp, ungrp, del, esc, enter, rot, find, fit], arrows, shift) = ev;
        if new {
            self.new_doc(FrameKind::FrVollstaendig, PaperFormat::A3, true);
        }
        if open {
            self.open_dialog();
        }
        if save {
            self.save(false);
        }
        if save_as {
            self.save(true);
        }
        if print {
            self.print();
        }
        if typing {
            return;
        }
        if undo {
            self.tab_mut().undo();
        }
        if redo {
            self.tab_mut().redo();
        }
        if copy {
            self.copy();
        }
        if cut {
            self.cut();
        }
        if paste {
            self.paste(Pos::new(5.0, 5.0));
        }
        if dup {
            self.duplicate();
        }
        if all {
            self.select_all();
        }
        if grp {
            self.group();
        }
        if ungrp {
            self.ungroup();
        }
        if del {
            self.delete_selection();
        }
        if find {
            self.dlg.search = true;
        }
        if fit {
            self.tab_mut().fit = true;
        }
        if rot {
            self.transform_sel(|k, c| ops::rotate(k, c, 90.0));
        }
        if enter && self.draft.len() >= 2 && self.tool.need() == crate::app::Need::Multi {
            let pts = std::mem::take(&mut self.draft);
            self.finish_tool(pts);
        }
        if esc {
            if !self.draft.is_empty() {
                self.draft.clear();
            } else if self.tool != Tool::Select {
                self.reset_tool();
            } else {
                self.tab_mut().sel.clear();
            }
            self.measure = None;
        }
        let step = if shift { 10.0 } else { 1.0 };
        let d = [Pos::new(-step, 0.0), Pos::new(step, 0.0), Pos::new(0.0, -step), Pos::new(0.0, step)];
        for (k, pressed) in arrows.iter().enumerate() {
            if *pressed {
                self.nudge(d[k]);
            }
        }
    }
}

fn layer_combo(ui: &mut egui::Ui, layers: &[Layer], layer: &mut u32) -> bool {
    let before = *layer;
    let name = layers.iter().find(|l| l.id == *layer).map(|l| l.name.clone()).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.label("Ebene");
        egui::ComboBox::from_id_salt("layer").selected_text(name).show_ui(ui, |ui| {
            for l in layers {
                ui.selectable_value(layer, l.id, &l.name);
            }
        });
    });
    before != *layer
}

pub fn style_ui(ui: &mut egui::Ui, st: &mut Style) -> bool {
    let before = *st;
    ui.horizontal(|ui| {
        let mut has = st.stroke.is_some();
        ui.checkbox(&mut has, "Linie");
        if has && st.stroke.is_none() {
            st.stroke = Some(StrokeStyle::new(farbe::SCHWARZ, 0.35));
        }
        if !has {
            st.stroke = None;
        }
        if let Some(s) = &mut st.stroke {
            color_edit(ui, &mut s.color);
            ui.add(egui::DragValue::new(&mut s.width).range(0.05..=50.0).speed(0.05).suffix(" mm"));
            egui::ComboBox::from_id_salt("dash").width(90.0).selected_text(s.dash.name()).show_ui(ui, |ui| {
                for d in Dash::ALL {
                    ui.selectable_value(&mut s.dash, d, d.name());
                }
            });
        }
    });
    ui.horizontal(|ui| {
        ui.label("Füllung");
        let mut mode = match st.fill {
            Fill::Keine => 0,
            Fill::Farbe(_) => 1,
            Fill::Schraffur(_) => 2,
        };
        let old = mode;
        ui.selectable_value(&mut mode, 0, "keine");
        ui.selectable_value(&mut mode, 1, "Farbe");
        ui.selectable_value(&mut mode, 2, "Schraffur");
        if mode != old {
            st.fill = match mode {
                0 => Fill::Keine,
                1 => Fill::Farbe(farbe::HELLGRAU),
                _ => Fill::Schraffur(Hatch { bg: None, color: farbe::SCHWARZ, angle: 45.0, spacing: 2.0, width: 0.25, cross: false }),
            };
        }
    });
    match &mut st.fill {
        Fill::Farbe(c) => {
            ui.horizontal(|ui| {
                color_edit(ui, c);
                let mut a = c.3;
                ui.add(egui::Slider::new(&mut a, 0..=255).text("Deckkraft"));
                c.3 = a;
            });
        }
        Fill::Schraffur(h) => {
            ui.horizontal(|ui| {
                color_edit(ui, &mut h.color);
                ui.add(egui::DragValue::new(&mut h.angle).range(-180.0..=180.0).suffix("°"));
                ui.add(egui::DragValue::new(&mut h.spacing).range(0.3..=50.0).speed(0.1).prefix("Abst. ").suffix(" mm"));
            });
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut h.width).range(0.05..=10.0).speed(0.05).prefix("Stärke ").suffix(" mm"));
                ui.checkbox(&mut h.cross, "Kreuz");
                let mut bg = h.bg.is_some();
                ui.checkbox(&mut bg, "Hintergrund");
                if bg && h.bg.is_none() {
                    h.bg = Some(farbe::WEISS);
                }
                if !bg {
                    h.bg = None;
                }
                if let Some(b) = &mut h.bg {
                    color_edit(ui, b);
                }
            });
        }
        Fill::Keine => {}
    }
    before != *st
}

fn pos_ui(ui: &mut egui::Ui, label: &str, p: &mut Pos) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(&mut p.x).speed(0.5).prefix("x ").suffix(" mm"));
        ui.add(egui::DragValue::new(&mut p.y).speed(0.5).prefix("y ").suffix(" mm"));
    });
}

fn kind_ui(ui: &mut egui::Ui, k: &mut ObjKind, focus: bool, scale_n: f32) {
    let text_box = |ui: &mut egui::Ui, t: &mut String, multi: bool| {
        let r = if multi { ui.add(egui::TextEdit::multiline(t).desired_rows(3).desired_width(f32::INFINITY)) } else { ui.add(egui::TextEdit::singleline(t).desired_width(f32::INFINITY)) };
        if focus {
            r.request_focus();
        }
    };
    match k {
        ObjKind::Text { pos, text, size, bold, italic, align, rot } => {
            text_box(ui, text, true);
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(size).range(0.5..=200.0).speed(0.1).prefix("Größe ").suffix(" mm"));
                ui.toggle_value(bold, RichText::new("F").strong());
                ui.toggle_value(italic, RichText::new("K").italics());
            });
            ui.horizontal(|ui| {
                ui.selectable_value(align, TAlign::Links, "Links");
                ui.selectable_value(align, TAlign::Mitte, "Mitte");
                ui.selectable_value(align, TAlign::Rechts, "Rechts");
                ui.add(egui::DragValue::new(rot).range(-360.0..=360.0).suffix("°"));
            });
            pos_ui(ui, "Position", pos);
        }
        ObjKind::Note { text, .. } => {
            text_box(ui, text, true);
            ui.label(RichText::new("Notizen werden nicht gedruckt.").weak().small());
        }
        ObjKind::Symbol { id, pos, size, rot } => {
            ui.label(symbols::find(id).map(|s| s.name).unwrap_or(id));
            ui.add(egui::Slider::new(size, 2.0..=80.0).text("Größe (mm)"));
            ui.horizontal(|ui| {
                ui.label("Drehung");
                for d in [0.0, 90.0, 180.0, 270.0] {
                    if ui.selectable_label((*rot - d).abs() < 0.01, format!("{d}°")).clicked() {
                        *rot = d;
                    }
                }
                ui.add(egui::DragValue::new(rot).range(0.0..=360.0).suffix("°"));
            });
            pos_ui(ui, "Position", pos);
        }
        ObjKind::Wall { thickness, closed, pts } => {
            ui.add(egui::DragValue::new(thickness).range(0.1..=20.0).speed(0.05).prefix("Wandstärke ").suffix(" mm"));
            ui.label(format!("Wandstärke real: {:.2} m", *thickness * scale_n / 1000.0));
            ui.checkbox(closed, "Geschlossen");
            ui.label(format!("{} Punkte, Länge {:.2} m", pts.len(), polyline_len(pts) * scale_n / 1000.0));
        }
        ObjKind::Room { name, show_area, size, pts } => {
            text_box(ui, name, false);
            ui.checkbox(show_area, "Fläche anzeigen");
            ui.add(egui::DragValue::new(size).range(0.5..=50.0).speed(0.1).prefix("Schrift ").suffix(" mm"));
            ui.label(format!("Fläche: {:.2} m²", polygon_area(pts) * (scale_n / 1000.0).powi(2)));
        }
        ObjKind::Route { width, curved, arrows, pts } => {
            ui.add(egui::DragValue::new(width).range(0.5..=40.0).speed(0.1).prefix("Breite ").suffix(" mm"));
            ui.checkbox(curved, "Bogenform");
            ui.checkbox(arrows, "Richtungspfeile");
            if ui.button("Richtung umkehren").clicked() {
                pts.reverse();
            }
            ui.label(format!("Weglänge: {:.1} m", polyline_len(pts) * scale_n / 1000.0));
        }
        ObjKind::Door { width, rot, mirror, double, pos } => {
            ui.add(egui::DragValue::new(width).range(1.0..=100.0).speed(0.1).prefix("Breite ").suffix(" mm"));
            ui.add(egui::DragValue::new(rot).range(-360.0..=360.0).prefix("Drehung ").suffix("°"));
            ui.checkbox(mirror, "Aufschlag gespiegelt");
            ui.checkbox(double, "Zweiflügelig");
            pos_ui(ui, "Angel", pos);
        }
        ObjKind::Window { thickness, .. } => {
            ui.add(egui::DragValue::new(thickness).range(0.2..=20.0).speed(0.05).prefix("Tiefe ").suffix(" mm"));
        }
        ObjKind::Stairs { width, steps, a, b } => {
            ui.add(egui::DragValue::new(width).range(1.0..=100.0).speed(0.1).prefix("Breite ").suffix(" mm"));
            ui.add(egui::DragValue::new(steps).range(2..=60).prefix("Stufen "));
            if ui.button("Laufrichtung umkehren").clicked() {
                std::mem::swap(a, b);
            }
        }
        ObjKind::Rect { radius, .. } => {
            ui.add(egui::DragValue::new(radius).range(0.0..=100.0).speed(0.1).prefix("Eckradius ").suffix(" mm"));
        }
        ObjKind::Circle { r, c } => {
            ui.add(egui::DragValue::new(r).range(0.1..=1000.0).speed(0.1).prefix("Radius ").suffix(" mm"));
            pos_ui(ui, "Mitte", c);
        }
        ObjKind::NGon { n, .. } => {
            ui.add(egui::DragValue::new(n).range(3..=64).prefix("Ecken "));
        }
        ObjKind::Hose { width, .. } => {
            ui.add(egui::DragValue::new(width).range(0.2..=20.0).speed(0.05).prefix("Breite ").suffix(" mm"));
        }
        ObjKind::Polyline { closed, .. } => {
            ui.checkbox(closed, "Geschlossen");
        }
        ObjKind::Image { opacity, data, .. } => {
            ui.add(egui::Slider::new(opacity, 0.05..=1.0).text("Deckkraft"));
            ui.label(format!("{} × {} Pixel", data.w, data.h));
        }
        ObjKind::Dimension { offset, a, b } => {
            ui.add(egui::DragValue::new(offset).range(-200.0..=200.0).speed(0.2).prefix("Abstand ").suffix(" mm"));
            ui.label(format!("Länge: {:.2} m", a.dist(*b) * scale_n / 1000.0));
        }
        ObjKind::Table { cells, cols, rows, size, .. } => {
            ui.add(egui::DragValue::new(size).range(0.5..=30.0).speed(0.1).prefix("Schrift ").suffix(" mm"));
            ui.horizontal(|ui| {
                if ui.button("+ Zeile").clicked() {
                    rows.push(7.0);
                    cells.push(vec![String::new(); cols.len()]);
                }
                if ui.button("− Zeile").clicked() && rows.len() > 1 {
                    rows.pop();
                    cells.pop();
                }
                if ui.button("+ Spalte").clicked() {
                    cols.push(30.0);
                    for r in cells.iter_mut() {
                        r.push(String::new());
                    }
                }
                if ui.button("− Spalte").clicked() && cols.len() > 1 {
                    cols.pop();
                    for r in cells.iter_mut() {
                        r.pop();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (i, c) in cols.iter_mut().enumerate() {
                    ui.add(egui::DragValue::new(c).range(3.0..=300.0).prefix(format!("Sp{} ", i + 1)));
                }
            });
            egui::Grid::new("cells").show(ui, |ui| {
                for row in cells.iter_mut() {
                    for cell in row.iter_mut() {
                        ui.add(egui::TextEdit::singleline(cell).desired_width(70.0));
                    }
                    ui.end_row();
                }
            });
        }
        ObjKind::Legend { cols, size, .. } => {
            ui.add(egui::DragValue::new(cols).range(1..=8).prefix("Spalten "));
            ui.add(egui::DragValue::new(size).range(1.0..=20.0).speed(0.1).prefix("Schrift ").suffix(" mm"));
            ui.label(RichText::new("Inhalt über „Legende …“ (Assistent) bearbeiten.").weak().small());
        }
        ObjKind::Grid { nx, ny, .. } => {
            ui.add(egui::DragValue::new(nx).range(1..=26).prefix("Felder waagerecht "));
            ui.add(egui::DragValue::new(ny).range(1..=60).prefix("Felder senkrecht "));
        }
        ObjKind::FloorSection { floors, current, .. } => {
            ui.label("Geschosse (von oben nach unten, eins je Zeile):");
            let mut s = floors.join("\n");
            if ui.add(egui::TextEdit::multiline(&mut s).desired_rows(4).desired_width(160.0)).changed() {
                *floors = s.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
                if floors.is_empty() {
                    floors.push("EG".into());
                }
                *current = (*current).min(floors.len() - 1);
            }
            egui::ComboBox::from_label("Dargestelltes Geschoss").selected_text(floors.get(*current).cloned().unwrap_or_default()).show_ui(ui, |ui| {
                for (i, f) in floors.iter().enumerate() {
                    ui.selectable_value(current, i, f);
                }
            });
        }
        ObjKind::ScaleBar { meters, .. } => {
            ui.add(egui::DragValue::new(meters).range(0.5..=10000.0).prefix("Länge ").suffix(" m"));
        }
        ObjKind::QrCode { text, size, .. } => {
            text_box(ui, text, false);
            ui.add(egui::DragValue::new(size).range(5.0..=200.0).prefix("Größe ").suffix(" mm"));
        }
        ObjKind::FireCurve { r_out, width, start, end, .. } => {
            let mut r_m = *r_out * scale_n / 1000.0;
            if ui.add(egui::DragValue::new(&mut r_m).range(10.5..=200.0).speed(0.1).prefix("Außenradius ").suffix(" m")).changed() {
                *r_out = r_m * 1000.0 / scale_n;
                *width = crate::tools::din14090_width(r_m) * 1000.0 / scale_n;
            }
            ui.label(format!("Fahrbahnbreite nach DIN 14090: {:.1} m", *width * scale_n / 1000.0));
            ui.add(egui::DragValue::new(start).prefix("Beginn ").suffix("°"));
            ui.add(egui::DragValue::new(end).prefix("Ende ").suffix("°"));
        }
        ObjKind::Arc { r, start, end, .. } => {
            ui.add(egui::DragValue::new(r).range(0.1..=1000.0).prefix("Radius ").suffix(" mm"));
            ui.add(egui::DragValue::new(start).prefix("Beginn ").suffix("°"));
            ui.add(egui::DragValue::new(end).prefix("Ende ").suffix("°"));
        }
        ObjKind::Group { children } => {
            ui.label(format!("{} Elemente", children.len()));
        }
        ObjKind::Area { pts, .. } => {
            ui.label(format!("Fläche: {:.2} m²", polygon_area(pts) * (scale_n / 1000.0).powi(2)));
        }
        _ => {}
    }
}

pub fn apply_frame_defaults(doc: &mut Doc, frame: FrameKind) {
    let def = FrameCfg::default();
    match frame {
        FrameKind::BrandschutzVollstaendig | FrameKind::BrandschutzNurOben => {
            if doc.frame_cfg.title == def.title || doc.frame_cfg.title == "FEUERWEHR-LAUFKARTE" {
                doc.frame_cfg.title = "BRANDSCHUTZPLAN".into();
            }
            doc.frame_cfg.color = farbe::ROT;
        }
        FrameKind::Laufkarte => {
            if doc.frame_cfg.title == def.title || doc.frame_cfg.title == "BRANDSCHUTZPLAN" {
                doc.frame_cfg.title = "FEUERWEHR-LAUFKARTE".into();
            }
            doc.frame_cfg.color = farbe::ROT;
        }
        FrameKind::FrVollstaendig | FrameKind::FrZweizeilig | FrameKind::FrNurOben => {
            if doc.frame_cfg.title == "BRANDSCHUTZPLAN" || doc.frame_cfg.title == "FEUERWEHR-LAUFKARTE" {
                doc.frame_cfg.title = def.title;
            }
            doc.frame_cfg.color = farbe::GRUEN;
        }
        _ => {}
    }
}
