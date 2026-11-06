//! Dialogfenster.

use std::io::{Read, Write};

use eframe::egui::{self, RichText, Vec2};

use crate::app::{App, DocTab, Tool};
use crate::canvas;
use crate::geom::*;
use crate::model::*;
use crate::symbols::{self, Category};
use crate::{bso, export, extract, i18n, import, ops, osm, samples, templates, wms};

fn win(title: &str) -> egui::Window<'_> {
    egui::Window::new(title).collapsible(false).resizable(true).pivot(egui::Align2::CENTER_CENTER).default_pos(egui::pos2(700.0, 380.0))
}

impl App {
    pub fn dialogs(&mut self, ctx: &egui::Context) {
        self.dlg_welcome(ctx);
        self.dlg_samples(ctx);
        self.dlg_legend(ctx);
        self.dlg_page(ctx);
        self.dlg_misc(ctx);
        self.dlg_extract(ctx);
        self.dlg_bso(ctx);
        self.dlg_osm(ctx);
        self.dlg_wms(ctx);
        self.dlg_confirm(ctx);
    }

    fn dlg_welcome(&mut self, ctx: &egui::Context) {
        if !self.dlg.welcome {
            return;
        }
        let mut open = true;
        let mut close = false;
        win("Willkommen").open(&mut open).default_width(620.0).show(ctx, |ui| {
            ui.heading("OpenFluchtplan – Flucht-, Rettungs- und Feuerwehrpläne");
            ui.label("Pläne nach DIN ISO 23601, DIN 14095 und DIN 14675 erstellen.");
            ui.add_space(8.0);
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("Was möchten Sie tun?").strong());
                cols[0].menu_button("Neuen Plan anlegen …", |ui| {
                    self.new_menu(ui);
                });
                if cols[0].button("Plan öffnen …").clicked() {
                    self.open_dialog();
                    close = true;
                }
                if cols[0].button("Musterplan öffnen …").clicked() {
                    self.dlg.samples = true;
                    close = true;
                }
                if cols[0].button("Erste Schritte …").clicked() {
                    self.dlg.help = true;
                    close = true;
                }
                if cols[0].button("Brandschutzordnung erstellen …").clicked() {
                    self.dlg.bso = true;
                    close = true;
                }
                cols[1].label(RichText::new("Zuletzt geöffnete Pläne").strong());
                let recent = self.settings.recent.clone();
                if recent.is_empty() {
                    cols[1].label(RichText::new("Noch keine.").weak());
                }
                for p in recent.iter().take(8) {
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if cols[1].button(name).on_hover_text(p.display().to_string()).clicked() {
                        self.open_path(p);
                        close = true;
                    }
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let mut show = self.settings.show_welcome;
                if ui.checkbox(&mut show, "Beim Start anzeigen").changed() {
                    self.settings.show_welcome = show;
                    self.settings.save();
                }
                if ui.button("Schließen").clicked() {
                    close = true;
                }
            });
        });
        // add_tab() setzt welcome auf false, wenn im Menü ein neuer Plan angelegt wurde
        self.dlg.welcome = self.dlg.welcome && open && !close;
    }

    fn dlg_samples(&mut self, ctx: &egui::Context) {
        if !self.dlg.samples {
            return;
        }
        let mut open = true;
        let mut chosen = None;
        win("Musterpläne").open(&mut open).show(ctx, |ui| {
            ui.label("Eigene Beispielpläne als Ausgangspunkt:");
            for (i, (name, _)) in samples::all().iter().enumerate() {
                if ui.button(*name).clicked() {
                    chosen = Some(i);
                }
            }
        });
        if let Some(i) = chosen {
            self.open_sample(i);
            open = false;
            self.dlg.welcome = false;
        }
        self.dlg.samples = open;
    }

    fn dlg_legend(&mut self, ctx: &egui::Context) {
        if !self.dlg.legend {
            return;
        }
        let mut open = true;
        let mut leg = {
            let tab = self.tab_mut();
            ops::sync_legend(&mut tab.doc);
            tab.doc.legend.clone()
        };
        let used: Vec<String> = ops::used_legend_keys(self.doc()).into_iter().map(|(k, _)| k).collect();
        let defaults: Vec<(String, String)> = ops::used_legend_keys(self.doc());
        let doc_clone = self.doc().clone();
        win("Symbollegende").open(&mut open).default_width(560.0).show(ctx, |ui| {
            ui.label("Die Legende ist Bestandteil des Flucht- und Rettungsplans. Sie wird automatisch aus den verwendeten Symbolen erstellt; hier lassen sich Texte, Reihenfolge und Sprachen anpassen.");
            ui.horizontal(|ui| {
                ui.label("Beschriftung:");
                ui.text_edit_singleline(&mut leg.title);
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Zusätzliche Sprachen:");
                for l in Lang::ALL.iter().skip(1) {
                    let mut on = leg.langs.contains(l);
                    if ui.checkbox(&mut on, l.name()).changed() {
                        if on {
                            leg.langs.push(*l);
                        } else {
                            leg.langs.retain(|x| x != l);
                        }
                    }
                }
            });
            ui.separator();
            let mut swap = None;
            let n = leg.entries.len();
            egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                egui::Grid::new("legend_entries").striped(true).show(ui, |ui| {
                    ui.label("In Legende");
                    ui.label("Symbol");
                    ui.label("Bezeichnung");
                    ui.label("");
                    ui.end_row();
                    for (i, e) in leg.entries.iter_mut().enumerate() {
                        if !used.contains(&e.key) {
                            continue;
                        }
                        ui.checkbox(&mut e.visible, "");
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::hover());
                        if let Some(id) = e.key.strip_prefix("sym:") {
                            canvas::paint_symbol_icon(ui.painter(), ctx, &mut self.textures, &doc_clone, id, r);
                        } else if let Some(k) = e.key.strip_prefix("area:") {
                            if let Some(p) = crate::presets::find(k) {
                                canvas::paint_style_icon(ui.painter(), ctx, &mut self.textures, &doc_clone, &p.style, r);
                            }
                        } else {
                            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "—", egui::FontId::proportional(14.0), egui::Color32::DARK_GREEN);
                        }
                        ui.vertical(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut e.label).desired_width(260.0));
                            for l in &leg.langs {
                                if let Some(t) = i18n::translate(&e.key, *l) {
                                    ui.label(RichText::new(format!("{}: {t}", l.name())).weak().small());
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.add_enabled(i > 0, egui::Button::new("auf").small()).clicked() {
                                swap = Some((i, i - 1));
                            }
                            if ui.add_enabled(i + 1 < n, egui::Button::new("ab").small()).clicked() {
                                swap = Some((i, i + 1));
                            }
                        });
                        ui.end_row();
                    }
                });
            });
            if let Some((a, b)) = swap {
                leg.entries.swap(a, b);
            }
            ui.horizontal(|ui| {
                if ui.button("Standardtexte wiederherstellen").clicked() {
                    for e in leg.entries.iter_mut() {
                        if let Some((_, d)) = defaults.iter().find(|(k, _)| *k == e.key) {
                            e.label = d.clone();
                        }
                    }
                }
                if ui.button("Legende als Element einfügen").on_hover_text("Rahmen mit Legende frei im Plan platzieren").clicked() {
                    self.tool = Tool::Legend;
                }
            });
        });
        if leg != self.doc().legend {
            let now = ctx.input(|i| i.time);
            let tab = self.tab_mut();
            let before = tab.doc.clone();
            tab.doc.legend = leg;
            tab.push_undo_coalesced(before, u64::MAX - 2, now);
        }
        self.dlg.legend = open;
    }

    fn dlg_page(&mut self, ctx: &egui::Context) {
        if !self.dlg.page {
            return;
        }
        let mut open = true;
        let mut cfg = self.doc().frame_cfg.clone();
        let mut frame = self.doc().frame;
        let mut page = self.doc().page;
        let mut logo_import = false;
        win("Rahmen und Titel").open(&mut open).default_width(460.0).show(ctx, |ui| {
            egui::Grid::new("page").num_columns(2).show(ui, |ui| {
                ui.label("Rahmen");
                egui::ComboBox::from_id_salt("frame2").width(280.0).selected_text(frame.name()).show_ui(ui, |ui| {
                    for f in FrameKind::ALL {
                        ui.selectable_value(&mut frame, f, f.name());
                    }
                });
                ui.end_row();
                ui.label("Format");
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("fmt2").selected_text(page.format.name()).show_ui(ui, |ui| {
                        for f in PaperFormat::LISTE {
                            ui.selectable_value(&mut page.format, f, f.name());
                        }
                        let (w, h) = page.format.size();
                        if ui.selectable_label(matches!(page.format, PaperFormat::Eigenes { .. }), "Benutzerdefiniert …").clicked() {
                            page.format = PaperFormat::Eigenes { w, h };
                        }
                    });
                    ui.selectable_value(&mut page.landscape, true, "Quer");
                    ui.selectable_value(&mut page.landscape, false, "Hoch");
                });
                ui.end_row();
                if let PaperFormat::Eigenes { w, h } = &mut page.format {
                    ui.label("Größe");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(w).range(50.0..=2000.0).suffix(" mm"));
                        ui.label("×");
                        ui.add(egui::DragValue::new(h).range(50.0..=2000.0).suffix(" mm"));
                    });
                    ui.end_row();
                }
                ui.label("Titel");
                ui.text_edit_singleline(&mut cfg.title);
                ui.end_row();
                ui.label("2. Zeile");
                ui.text_edit_singleline(&mut cfg.title2);
                ui.end_row();
                ui.label("Farbe");
                ui.horizontal(|ui| {
                    let mut c = canvas::c32(cfg.color);
                    if ui.color_edit_button_srgba(&mut c).changed() {
                        cfg.color = crate::ui::from_c32(c);
                    }
                    if ui.small_button("Grün").clicked() {
                        cfg.color = farbe::GRUEN;
                    }
                    if ui.small_button("Rot").clicked() {
                        cfg.color = farbe::ROT;
                    }
                });
                ui.end_row();
                ui.label("Bestandteile");
                ui.vertical(|ui| {
                    ui.checkbox(&mut cfg.show_rules, "Verhaltensregeln");
                    ui.checkbox(&mut cfg.show_notruf, "Notruf-Feld");
                    ui.checkbox(&mut cfg.show_sammelstelle, "Sammelstelle");
                });
                ui.end_row();
                ui.label("Logo");
                ui.horizontal(|ui| {
                    if ui.button("Logo wählen …").clicked() {
                        logo_import = true;
                    }
                    if cfg.logo.is_some() && ui.button("Entfernen").clicked() {
                        cfg.logo = None;
                    }
                });
                ui.end_row();
            });
            ui.label(RichText::new("Hinweis: Flucht- und Rettungspläne mindestens in DIN A3; DIN A4 nur für einzelne Räume (z. B. Hotelzimmer).").weak().small());
        });
        if cfg != self.doc().frame_cfg || frame != self.doc().frame || page != self.doc().page {
            let tab = self.tab_mut();
            tab.push_undo();
            let alt = crate::templates::layout(&tab.doc).plan;
            if frame != tab.doc.frame {
                crate::ui::apply_frame_defaults(&mut tab.doc, frame);
                let auto = tab.doc.frame_cfg.clone();
                if cfg.title == FrameCfg::default().title || cfg.title == "BRANDSCHUTZPLAN" || cfg.title == "FEUERWEHR-LAUFKARTE" {
                    cfg.title = auto.title;
                    cfg.color = auto.color;
                }
            }
            if page != tab.doc.page {
                tab.fit = true;
            }
            tab.doc.frame_cfg = cfg;
            tab.doc.frame = frame;
            tab.doc.page = page;
            // Inhalt mitnehmen, z. B. beim Wechsel von A3 auf A4
            if let Some(f) = crate::templates::inhalt_umsetzen(&mut tab.doc, alt) {
                tab.fit = true;
                let n = tab.doc.scale_n;
                self.status = format!("Planinhalt an den neuen Planbereich angepasst ({:.0} %), Maßstab jetzt 1:{n}.", f * 100.0);
            }
        }
        if logo_import {
            self.import_dialog("logo");
        }
        self.dlg.page = open;
    }

    fn dlg_misc(&mut self, ctx: &egui::Context) {
        // Mehrfachkopie
        if self.dlg.multicopy {
            let mut open = true;
            let mut go = false;
            win("Mehrfachkopie").open(&mut open).show(ctx, |ui| {
                ui.add(egui::DragValue::new(&mut self.dlg.multicopy_n).range(1..=200).prefix("Anzahl Kopien "));
                ui.add(egui::DragValue::new(&mut self.dlg.multicopy_dx).speed(0.5).prefix("Versatz x ").suffix(" mm"));
                ui.add(egui::DragValue::new(&mut self.dlg.multicopy_dy).speed(0.5).prefix("Versatz y ").suffix(" mm"));
                go = ui.button("Kopieren").clicked();
            });
            if go {
                self.multicopy(self.dlg.multicopy_n, self.dlg.multicopy_dx, self.dlg.multicopy_dy);
                open = false;
            }
            self.dlg.multicopy = open;
        }
        // Plan prüfen
        if self.dlg.check {
            let mut open = true;
            let befunde = crate::pruefung::pruefen(self.doc());
            win("Plan prüfen").open(&mut open).default_width(480.0).show(ctx, |ui| {
                use crate::pruefung::Stufe;
                let fehler = befunde.iter().filter(|b| b.0 == Stufe::Fehler).count();
                let hinweise = befunde.iter().filter(|b| b.0 == Stufe::Hinweis).count();
                let (txt, col) = if fehler == 0 {
                    ("Keine Pflichtangaben fehlen.".to_string(), egui::Color32::from_rgb(0x23, 0x7f, 0x52))
                } else {
                    (format!("{fehler} Pflichtangabe(n) fehlen."), egui::Color32::from_rgb(0xb0, 0x1e, 0x23))
                };
                ui.label(RichText::new(txt).strong().color(col));
                ui.label(RichText::new(format!("{hinweise} Hinweis(e). Die Prüfung wird laufend aktualisiert.")).weak().small());
                ui.separator();
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    for (stufe, text) in &befunde {
                        let (mark, col) = match stufe {
                            Stufe::Ok => ("OK", egui::Color32::from_rgb(0x23, 0x7f, 0x52)),
                            Stufe::Hinweis => ("Hinweis", egui::Color32::from_rgb(0xa0, 0x70, 0x00)),
                            Stufe::Fehler => ("Fehlt", egui::Color32::from_rgb(0xb0, 0x1e, 0x23)),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(mark).strong().color(col));
                            ui.label(text);
                        });
                    }
                });
                ui.separator();
                ui.label(RichText::new("Die Prüfung ersetzt keine fachliche Abnahme durch Brandschutzbeauftragte bzw. Feuerwehr.").weak().small());
            });
            self.dlg.check = open;
        }
        // Versatz (Offset)
        if self.dlg.offset {
            let mut open = true;
            let mut go = None;
            win("Versatz (Offset)").open(&mut open).show(ctx, |ui| {
                ui.label("Erzeugt parallele Kopien der ausgewählten Linien, Wände, Flächen, Kreise und Rechtecke.");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.dlg.offset_mm).range(0.05..=500.0).speed(0.1).prefix("Abstand ").suffix(" mm"));
                    let m = self.doc().real_m(self.dlg.offset_mm);
                    ui.label(RichText::new(format!("= {m:.2} m im Plan")).weak());
                });
                ui.horizontal(|ui| {
                    if ui.button("Nach links / außen").clicked() {
                        go = Some(self.dlg.offset_mm);
                    }
                    if ui.button("Nach rechts / innen").clicked() {
                        go = Some(-self.dlg.offset_mm);
                    }
                });
            });
            if let Some(d) = go {
                let n = self.offset_selection(d);
                self.status = if n == 0 { "Keine versetzbaren Objekte ausgewählt.".into() } else { format!("{n} Objekt(e) versetzt.") };
                open = false;
            }
            self.dlg.offset = open;
        }
        // Suchen / Ersetzen
        if self.dlg.search {
            let mut open = true;
            win("Nach Texten suchen und ersetzen").open(&mut open).show(ctx, |ui| {
                egui::Grid::new("sr").show(ui, |ui| {
                    ui.label("Suchen nach:");
                    ui.text_edit_singleline(&mut self.dlg.search_find);
                    ui.end_row();
                    ui.label("Ersetzen durch:");
                    ui.text_edit_singleline(&mut self.dlg.search_repl);
                    ui.end_row();
                });
                ui.checkbox(&mut self.dlg.search_case, "Groß-/Kleinschreibung beachten");
                let (f, r, c) = (self.dlg.search_find.clone(), self.dlg.search_repl.clone(), self.dlg.search_case);
                let mut doc_copy = self.doc().objects.clone();
                let hits = ops::replace_text(&mut doc_copy, &f, &r, c, false);
                ui.label(format!("{hits} Treffer"));
                ui.horizontal(|ui| {
                    if ui.add_enabled(hits > 0, egui::Button::new("Alle ersetzen")).clicked() {
                        let tab = self.tab_mut();
                        tab.push_undo();
                        let n = ops::replace_text(&mut tab.doc.objects, &f, &r, c, true);
                        self.status = format!("{n} Text(e) ersetzt.");
                    }
                    if ui.add_enabled(hits > 0, egui::Button::new("Treffer auswählen")).clicked() {
                        let tab = self.tab_mut();
                        let ids: Vec<u64> = tab
                            .doc
                            .objects
                            .iter()
                            .filter(|o| {
                                let mut one = vec![(*o).clone()];
                                ops::replace_text(&mut one, &f, &r, c, false) > 0
                            })
                            .map(|o| o.id)
                            .collect();
                        tab.sel = ids;
                    }
                });
            });
            self.dlg.search = open;
        }
        // Symbol-Manager
        if self.dlg.symbol_manager {
            let mut open = true;
            let used: Vec<(String, String)> = ops::used_legend_keys(self.doc()).into_iter().filter_map(|(k, n)| k.strip_prefix("sym:").map(|id| (id.to_string(), n))).collect();
            let mut go = false;
            win("Symbol-Manager").open(&mut open).default_width(480.0).show(ctx, |ui| {
                ui.label("Ein im Plan verwendetes Symbol überall durch ein anderes ersetzen.");
                if used.is_empty() {
                    ui.label("Im Plan sind noch keine Symbole vorhanden.");
                    return;
                }
                if self.dlg.sm_from.is_empty() || !used.iter().any(|(id, _)| *id == self.dlg.sm_from) {
                    self.dlg.sm_from = used[0].0.clone();
                }
                let from_name = used.iter().find(|(id, _)| *id == self.dlg.sm_from).map(|(_, n)| n.clone()).unwrap_or_default();
                egui::ComboBox::from_label("Ersetzen").width(280.0).selected_text(from_name).show_ui(ui, |ui| {
                    for (id, n) in &used {
                        ui.selectable_value(&mut self.dlg.sm_from, id.clone(), n);
                    }
                });
                let to_name = symbols::find(&self.dlg.sm_to).map(|s| s.name).unwrap_or("– bitte wählen –");
                egui::ComboBox::from_label("durch").width(280.0).selected_text(to_name).show_ui(ui, |ui| {
                    for c in Category::ALL {
                        ui.label(RichText::new(c.name()).strong());
                        for s in symbols::library().iter().filter(|s| s.cat == c) {
                            ui.selectable_value(&mut self.dlg.sm_to, s.id.to_string(), s.name);
                        }
                    }
                });
                let count = self.doc().objects.iter().filter(|o| matches!(&o.kind, ObjKind::Symbol { id, .. } if *id == self.dlg.sm_from)).count();
                ui.label(format!("{count} Vorkommen"));
                go = ui.add_enabled(!self.dlg.sm_to.is_empty(), egui::Button::new("Ersetzen")).clicked();
            });
            if go {
                let (f, t) = (self.dlg.sm_from.clone(), self.dlg.sm_to.clone());
                let n = self.replace_symbols(&f, &t);
                self.status = format!("{n} Symbol(e) ersetzt.");
            }
            self.dlg.symbol_manager = open;
        }
        // Maßstab durch Entfernung
        if let Some(d) = self.dlg.scale_dist {
            let mut open = true;
            let mut go = false;
            win("Maßstab durch Entfernung festlegen").open(&mut open).show(ctx, |ui| {
                ui.label(format!("Abstand auf dem Blatt: {d:.2} mm"));
                ui.add(egui::DragValue::new(&mut self.dlg.scale_dist_m).range(0.01..=100000.0).speed(0.1).prefix("Tatsächliche Entfernung ").suffix(" m"));
                let n = self.dlg.scale_dist_m * 1000.0 / d.max(0.001);
                ui.label(format!("Ergibt Maßstab 1:{n:.0}"));
                go = ui.button("Übernehmen").clicked();
            });
            if go {
                let n = (self.dlg.scale_dist_m * 1000.0 / d.max(0.001)).round();
                let tab = self.tab_mut();
                tab.push_undo();
                tab.doc.scale_n = n;
                self.status = format!("Maßstab auf 1:{n} gesetzt.");
                self.reset_tool();
                open = false;
            }
            if !open {
                self.dlg.scale_dist = None;
            }
        }
        // QR-Code
        if let Some(p) = self.dlg.qr {
            let mut open = true;
            let mut go = false;
            win("QR-Code").open(&mut open).show(ctx, |ui| {
                ui.label("Inhalt (z. B. Link zum digitalen Plan, Telefonnummer):");
                ui.text_edit_singleline(&mut self.dlg.qr_text);
                go = ui.button("Einfügen").clicked();
            });
            if go {
                let text = self.dlg.qr_text.clone();
                let tab = self.tab_mut();
                tab.push_undo();
                let layer = tab.doc.active_layer;
                let id = tab.doc.add(ObjKind::QrCode { pos: p, size: 25.0, text }, Style::NONE, layer);
                tab.sel = vec![id];
                open = false;
            }
            if !open {
                self.dlg.qr = None;
            }
        }
        // PDF-Seite wählen
        if let Some(mut auswahl) = self.dlg.pdf_pages.clone() {
            let (n, mut page) = (auswahl.seiten, auswahl.seite);
            let mut open = true;
            let mut go = false;
            win("PDF importieren").open(&mut open).show(ctx, |ui| {
                use crate::app::PdfFit;
                let o = &mut self.dlg.pdf_opt;
                let name = &auswahl.name;
                ui.label(if n == 1 { format!("{name} (1 Seite)") } else { format!("{name} hat {n} Seiten.") });
                if n > 1 {
                    let mut p1 = page + 1;
                    ui.add(egui::DragValue::new(&mut p1).range(1..=n).prefix("Seite "));
                    page = p1 - 1;
                }
                ui.separator();
                ui.radio_value(&mut o.vector, true, "Als Vektorgrafik (Linien, Flächen, Texte einzeln bearbeitbar)");
                ui.radio_value(&mut o.vector, false, "Als Bild (z. B. für gescannte Pläne)");
                if o.vector {
                    ui.checkbox(&mut o.group, "Alles in eine Gruppe zusammenfassen");
                    ui.add_enabled(!o.group, egui::Checkbox::new(&mut o.symbols, "Symbole erkennen (durch Normzeichen ersetzen, Legende ergänzen)"));
                    ui.add(egui::DragValue::new(&mut o.text_pct).range(10.0..=400.0).speed(1.0).prefix("Texte skalieren ").suffix(" %"));
                    ui.label(RichText::new("Skalierung:").strong());
                    ui.radio_value(&mut o.fit, PdfFit::Einpassen, "In die Planfläche einpassen");
                    ui.radio_value(&mut o.fit, PdfFit::Massstab, "Originalgröße (1:1 in mm)");
                    ui.radio_value(&mut o.fit, PdfFit::Formatfuellend, "Formatfüllend – Blattformat übernehmen, eigenen Rahmen ausblenden");
                    ui.label(RichText::new("„Formatfüllend“ eignet sich, um einen vorhandenen fertigen Plan weiterzubearbeiten.").weak().small());
                }
                go = ui.button("Importieren").clicked();
            });
            auswahl.seite = page;
            self.dlg.pdf_pages = if open { Some(auswahl.clone()) } else { None };
            if go {
                self.dlg.pdf_pages = None;
                self.import_ziel(&auswahl.name);
                if self.dlg.pdf_opt.vector {
                    self.import_pdf_vector(&auswahl.bytes, page);
                } else {
                    self.import_pdf_page(&auswahl.bytes, page);
                }
            }
        }
        // PNG
        if self.dlg.png {
            let mut open = true;
            let mut go = false;
            win("Als PNG exportieren").open(&mut open).show(ctx, |ui| {
                ui.add(egui::DragValue::new(&mut self.dlg.png_dpi).range(30.0..=600.0).suffix(" dpi"));
                let (w, h) = self.doc().page.size();
                ui.label(format!("{:.0} × {:.0} Pixel", w / 25.4 * self.dlg.png_dpi, h / 25.4 * self.dlg.png_dpi));
                go = ui.button("Exportieren …").clicked();
            });
            if go {
                self.export_other("png");
                open = false;
            }
            self.dlg.png = open;
        }
        // Zusammenführen
        if self.dlg.merge {
            let mut open = true;
            let mut go = false;
            self.dlg.merge_sel.resize(self.tabs.len(), true);
            win("Pläne zusammenführen").open(&mut open).show(ctx, |ui| {
                ui.label("Ausgewählte geöffnete Pläne in eine PDF-Datei speichern:");
                for (i, t) in self.tabs.iter().enumerate() {
                    let label = format!("{} – {}", t.title(), t.doc.info.geschoss);
                    ui.checkbox(&mut self.dlg.merge_sel[i], label);
                }
                go = ui.button("PDF speichern …").clicked();
            });
            if go {
                let which: Vec<usize> = self.dlg.merge_sel.iter().enumerate().filter(|(_, s)| **s).map(|(i, _)| i).collect();
                self.export_merged(&which);
                open = false;
            }
            self.dlg.merge = open;
        }
        // Optionen
        if self.dlg.options {
            let mut open = true;
            win("Optionen").open(&mut open).show(ctx, |ui| {
                let s = &mut self.settings;
                egui::Grid::new("opt").num_columns(2).show(ui, |ui| {
                    ui.label("Oberflächengröße");
                    ui.horizontal(|ui| {
                        for (v, n) in [(1.0, "100 %"), (1.25, "125 %"), (1.5, "150 %"), (1.75, "175 %"), (2.0, "200 %")] {
                            ui.selectable_value(&mut s.ui_scale, v, n);
                        }
                    });
                    ui.end_row();
                    ui.label("Verfasser (Standard)");
                    ui.text_edit_singleline(&mut s.ersteller);
                    ui.end_row();
                    ui.label("Wandstärke");
                    ui.add(egui::DragValue::new(&mut s.wall_mm).range(0.1..=10.0).speed(0.05).suffix(" mm"));
                    ui.end_row();
                    ui.label("Symbolgröße");
                    ui.add(egui::DragValue::new(&mut s.symbol_mm).range(2.0..=50.0).speed(0.1).suffix(" mm"));
                    ui.end_row();
                    ui.label("Textgröße");
                    ui.add(egui::DragValue::new(&mut s.text_mm).range(1.0..=50.0).speed(0.1).suffix(" mm"));
                    ui.end_row();
                    ui.label("Fluchtwegbreite");
                    ui.add(egui::DragValue::new(&mut s.route_mm).range(1.0..=40.0).speed(0.1).suffix(" mm"));
                    ui.end_row();
                    ui.label("N-Eck: Ecken");
                    ui.add(egui::DragValue::new(&mut s.ngon_n).range(3..=64));
                    ui.end_row();
                    ui.label("Tabelle: Zeilen × Spalten");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut s.table_rows).range(1..=50));
                        ui.add(egui::DragValue::new(&mut s.table_cols).range(1..=20));
                    });
                    ui.end_row();
                    ui.label("Ebenen automatisch");
                    ui.checkbox(&mut s.auto_layers, "Elemente nach Art auf Ebenen verteilen");
                    ui.end_row();
                    ui.label("Willkommensdialog");
                    ui.checkbox(&mut s.show_welcome, "Beim Start anzeigen");
                    ui.end_row();
                });
                ui.label(RichText::new("Hinweis: Nach DIN ISO 23601 sollen Sicherheitszeichen im Plan mindestens ca. 7 mm groß sein.").weak().small());
                if ui.button("Speichern").clicked() {
                    s.save();
                }
            });
            if !open {
                self.settings.save();
            }
            self.dlg.options = open;
        }
        // Über
        if self.dlg.about {
            let mut open = true;
            win("Über OpenFluchtplan").open(&mut open).show(ctx, |ui| {
                ui.heading("OpenFluchtplan");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("Erstellung von Flucht- und Rettungsplänen (DIN ISO 23601), Feuerwehrplänen (DIN 14095), Feuerwehr-Laufkarten (DIN 14675) und Brandschutzplänen.");
                ui.label("Freie Software (gemeinfrei, The Unlicense). Läuft ohne Installation.");
                ui.hyperlink("https://github.com/Schrabbe10/OpenFluchtplan");
                ui.add_space(6.0);
                ui.label(RichText::new("Kartendaten: © OpenStreetMap-Mitwirkende (ODbL). Symbole: eigene Darstellung in Anlehnung an ISO 7010 / DIN 14034-6 / GHS.").small());
            });
            self.dlg.about = open;
        }
        // Hilfe
        if self.dlg.help {
            let mut open = true;
            win("Benutzerhilfe").open(&mut open).default_width(640.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                    ui.heading("Erste Schritte");
                    for l in [
                        "1. Datei › Neu: Planart und Format wählen (Flucht- und Rettungsplan, Feuerwehrplan, Laufkarte, Brandschutzplan).",
                        "2. Plan › Importieren: vorhandenen Grundriss als PDF, Bild oder DXF/DWG einfügen – oder mit Wand, Raum, Tür, Fenster und Treppe selbst zeichnen.",
                        "3. Plan › Maßstab durch Entfernung: zwei Punkte mit bekanntem Abstand anklicken, damit Maße und Flächen stimmen.",
                        "4. Fluchtwege zeichnen (Einfügen › Fluchtwege) oder Flächen per „Polygon füllen“ in umschlossene Bereiche klicken.",
                        "5. Symbole aus der linken Bibliothek wählen und in den Plan klicken. Der Standort („Sie sind hier“) ist Pflicht.",
                        "6. Planangaben rechts ausfüllen (Objekt, Geschoss, Sammelstelle, Verfasser, Stand).",
                        "7. Die Legende entsteht automatisch – Texte, Reihenfolge und Sprachen über „Legende …“ anpassen.",
                        "8. Datei › PDF erstellen (optional mit Überdruck ENTWURF/MUSTER/VORABZUG) oder Drucken.",
                    ] {
                        ui.label(l);
                    }
                    ui.add_space(6.0);
                    ui.heading("Bedienung");
                    for l in [
                        "Mausrad: zoomen · mittlere Maustaste oder Leertaste + Ziehen: verschieben",
                        "Auswahl: Klick, Umschalt+Klick ergänzt, Rahmen von links = vollständig enthalten, von rechts = schneidend",
                        "Griffpunkte der Auswahl ziehen, um Form/Größe zu ändern",
                        "Zeichnen: Umschalt rastet auf 45° ein · Doppelklick, Enter oder Rechtsklick beendet Linienzüge · Esc bricht ab",
                        "Punktfang rastet an Eckpunkten ein (Ansicht › Raster und Fang)",
                        "Pfeiltasten verschieben die Auswahl um 1 mm (mit Umschalt 10 mm) · R dreht um 90°",
                    ] {
                        ui.label(format!("• {l}"));
                    }
                    ui.add_space(6.0);
                    ui.heading("Tastenkürzel");
                    for l in [
                        "Strg+N Neu · Strg+O Öffnen · Strg+S Speichern · Strg+Umschalt+S Speichern unter · Strg+P Drucken",
                        "Strg+Z Rückgängig · Strg+Y Wiederholen · Entf Löschen",
                        "Strg+C/X/V Kopieren/Ausschneiden/Einfügen · Strg+D Duplizieren · Strg+A Alles auswählen",
                        "Strg+G Gruppieren · Strg+Umschalt+G Gruppierung aufheben · Strg+F Suchen/Ersetzen · F Einpassen",
                    ] {
                        ui.label(l);
                    }
                    ui.add_space(6.0);
                    ui.heading("Normhinweise");
                    for l in [
                        "Flucht- und Rettungspläne (DIN ISO 23601 / ASR A2.3): lagerichtig zum Standort, mindestens DIN A3, Standort, Legende, Verhaltensregeln, Sammelstelle, Erstell-/Revisionsdatum.",
                        "Feuerwehrpläne (DIN 14095): Übersichts-, Geschoss- und Sonderpläne, Symbole nach DIN 14034-6, Farben: befahrbar grau, nicht befahrbar gelb, besondere Gefahren rot, Löschwasser blau. Örtliche Vorgaben der Brandschutzdienststelle beachten.",
                        "Feuerwehr-Laufkarten (DIN 14675): je Meldergruppe/Melder, Laufweg ab FIZ/BMZ.",
                        "Kurven von Feuerwehrzufahrten (DIN 14090): Außenradius mindestens 10,5 m; die Fahrbahnbreite ergibt sich aus dem Radius.",
                        "Pläne regelmäßig prüfen (z. B. alle 2 Jahre) und bei baulichen Änderungen aktualisieren.",
                    ] {
                        ui.label(format!("• {l}"));
                    }
                });
            });
            self.dlg.help = open;
        }
        // Fehler
        if let Some(e) = self.dlg.error.clone() {
            let mut open = true;
            let mut ok = false;
            win("Hinweis").open(&mut open).show(ctx, |ui| {
                ui.label(e);
                ok = ui.button("OK").clicked();
            });
            if !open || ok {
                self.dlg.error = None;
            }
        }
    }

    fn dlg_extract(&mut self, ctx: &egui::Context) {
        if !self.dlg.extract {
            return;
        }
        let mut open = true;
        let mut run: Option<bool> = None;
        let sel_image = self.tab().selected_objs().iter().any(|o| matches!(o.kind, ObjKind::Image { .. }));
        win("Grundriss extrahieren").open(&mut open).default_width(420.0).show(ctx, |ui| {
            ui.label("Erkennt waagerechte und senkrechte Wände in einem Grundrissbild (Scan, PDF, Foto) und legt sie als Wand-Objekte an.");
            ui.add(egui::Slider::new(&mut self.dlg.extract_threshold, 30..=220).text("Helligkeitsschwelle"));
            ui.add(egui::Slider::new(&mut self.dlg.extract_min_mm, 3.0..=80.0).text("Mindestlänge (mm)"));
            ui.horizontal(|ui| {
                if ui.add_enabled(sel_image, egui::Button::new("Aus ausgewähltem Bild")).clicked() {
                    run = Some(true);
                }
                if ui.button("Bilddatei wählen …").clicked() {
                    run = Some(false);
                }
            });
            if !sel_image {
                ui.label(RichText::new("Tipp: Erst Grundriss importieren und auswählen – dann passen die Wände genau darauf.").weak().small());
            }
        });
        if let Some(from_sel) = run {
            let (img, a, b) = if from_sel {
                let o = self.tab().selected_objs().into_iter().find(|o| matches!(o.kind, ObjKind::Image { .. })).cloned();
                match o.map(|o| o.kind) {
                    Some(ObjKind::Image { a, b, data, .. }) => {
                        use base64::Engine as _;
                        let bytes = base64::engine::general_purpose::STANDARD.decode(&data.b64).unwrap_or_default();
                        (image::load_from_memory(&bytes).ok(), a, b)
                    }
                    _ => (None, Pos::default(), Pos::default()),
                }
            } else {
                // Bilddatei wählen – im Browser asynchron, weiter in `waende_aus_datei`
                self.dlg.extract = false;
                crate::plattform::waehlen(crate::plattform::Zweck::WaendeBild, "Bilder", &["png", "jpg", "jpeg", "bmp", "tif", "tiff"]);
                self.dateien_verarbeiten();
                return;
            };
            if let Some(img) = img {
                self.waende_extrahieren(&img, a, b);
                open = false;
            }
        }
        self.dlg.extract = open;
    }

    pub fn waende_aus_datei(&mut self, d: crate::plattform::Datei) {
        match image::load_from_memory(&d.bytes) {
            Ok(img) => {
                let area = templates::layout(self.doc()).plan;
                let (a, b) = import::fit_rect(area, img.width(), img.height());
                self.waende_extrahieren(&img, a, b);
            }
            Err(e) => self.error(format!("Bild nicht lesbar: {e}")),
        }
    }

    fn waende_extrahieren(&mut self, img: &image::DynamicImage, a: Pos, b: Pos) {
        let walls = extract::extract_walls(img, a, b, self.dlg.extract_threshold, self.dlg.extract_min_mm);
        let n = walls.len();
        let tab = self.tab_mut();
        tab.push_undo();
        let layer = tab.doc.find_layer_by_name("Wände");
        let mut ids = Vec::new();
        for w in walls {
            ids.push(tab.doc.add(w, Style::line(farbe::SCHWARZ, 1.0), layer));
        }
        tab.sel = ids;
        self.status = format!("{n} Wände erkannt. Nicht passende Wände einfach löschen.");
    }

    fn dlg_bso(&mut self, ctx: &egui::Context) {
        if !self.dlg.bso {
            return;
        }
        let mut open = true;
        let mut go = false;
        let id = egui::Id::NULL.with("bso_input");
        let mut inp: (String, String, String, String, String) = ctx.data_mut(|d| d.get_temp(id)).unwrap_or_else(|| {
            let i = &self.doc().info;
            (i.objekt.clone(), i.notruf.clone(), String::new(), i.sammelstelle.clone(), i.stand.clone())
        });
        win("Brandschutzordnung Teil A").open(&mut open).show(ctx, |ui| {
            ui.label("Erstellt einen Aushang „Brandschutzordnung Teil A“ (in Anlehnung an DIN 14096) als bearbeitbaren Plan.");
            egui::Grid::new("bso").num_columns(2).show(ui, |ui| {
                ui.label("Objekt");
                ui.text_edit_singleline(&mut inp.0);
                ui.end_row();
                ui.label("Notruf");
                ui.text_edit_singleline(&mut inp.1);
                ui.end_row();
                ui.label("Interne Notrufnummer");
                ui.text_edit_singleline(&mut inp.2);
                ui.end_row();
                ui.label("Sammelstelle");
                ui.text_edit_singleline(&mut inp.3);
                ui.end_row();
                ui.label("Stand");
                ui.text_edit_singleline(&mut inp.4);
                ui.end_row();
            });
            go = ui.button("Erstellen").clicked();
        });
        ctx.data_mut(|d| d.insert_temp(id, inp.clone()));
        if go {
            let doc = bso::teil_a(&bso::BsoInput { objekt: inp.0, notruf: inp.1, intern: inp.2, sammelstelle: inp.3, stand: inp.4 });
            self.add_tab(DocTab::new(doc, None));
            open = false;
        }
        self.dlg.bso = open;
    }

    fn dlg_osm(&mut self, ctx: &egui::Context) {
        if !self.dlg.osm {
            return;
        }
        let mut open = true;
        let mut fetch = false;
        let mut search = false;
        win("OSM-Karte einfügen").open(&mut open).default_width(440.0).show(ctx, |ui| {
            ui.label("Fügt einen Kartenausschnitt von OpenStreetMap als Grundlage für Übersichtspläne ein.");
            ui.horizontal(|ui| {
                ui.label("Adresse:");
                ui.text_edit_singleline(&mut self.dlg.osm_query);
                search = ui.button("Suchen").clicked();
            });
            ui.label(RichText::new("Die Adresse wird zur Suche an nominatim.openstreetmap.org übertragen.").weak().small());
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut self.dlg.osm_lat).speed(0.0001).range(-85.0..=85.0).prefix("Breite ").max_decimals(6));
                ui.add(egui::DragValue::new(&mut self.dlg.osm_lon).speed(0.0001).range(-180.0..=180.0).prefix("Länge ").max_decimals(6));
            });
            ui.add(egui::Slider::new(&mut self.dlg.osm_zoom, 14..=19).text("Zoomstufe"));
            fetch = ui.button("Karte laden und einfügen").clicked();
            if !self.dlg.osm_status.is_empty() {
                ui.label(&self.dlg.osm_status);
            }
        });
        if search {
            match osm::geocode(&self.dlg.osm_query) {
                Ok((lat, lon)) => {
                    self.dlg.osm_lat = lat;
                    self.dlg.osm_lon = lon;
                    self.dlg.osm_status = format!("Gefunden: {lat:.5}, {lon:.5}");
                }
                Err(e) => self.dlg.osm_status = e,
            }
        }
        if fetch {
            match osm::map_image(self.dlg.osm_lat, self.dlg.osm_lon, self.dlg.osm_zoom, 5, 3) {
                Ok(data) => {
                    let area = templates::layout(self.doc()).plan;
                    self.insert_image(data, area, 1.0);
                    let (a, _) = import::fit_rect(area, 5 * 256, 3 * 256);
                    let tab = self.tab_mut();
                    let layer = tab.doc.find_layer_by_name("Grundriss");
                    tab.doc.add(
                        ObjKind::Text { pos: a + Pos::new(1.0, 1.0), text: "© OpenStreetMap-Mitwirkende".into(), size: 2.2, bold: false, italic: false, align: TAlign::Links, rot: 0.0 },
                        Style::line(farbe::SCHWARZ, 0.2),
                        layer,
                    );
                    // Maßstab aus Zoomstufe (Web-Mercator, 256-px-Kacheln)
                    let m_per_px = 156543.03392 * self.dlg.osm_lat.to_radians().cos() / 2f64.powi(self.dlg.osm_zoom as i32);
                    let mm_per_px = (area.w() / (5.0 * 256.0)).min(area.h() / (3.0 * 256.0)) as f64;
                    let n = (m_per_px * 1000.0 / mm_per_px).round() as f32;
                    let tab = self.tab_mut();
                    tab.doc.scale_n = n;
                    self.status = format!("Karte eingefügt, Maßstab ca. 1:{n}.");
                    open = false;
                }
                Err(e) => self.dlg.osm_status = e,
            }
        }
        self.dlg.osm = open;
    }

    fn dlg_wms(&mut self, ctx: &egui::Context) {
        if !self.dlg.wms {
            return;
        }
        let mut open = true;
        let (mut load_caps, mut search, mut fetch) = (false, false, false);
        win("Flurkarte / WMS-Karte einfügen").open(&mut open).default_width(520.0).show(ctx, |ui| {
            ui.label("Fügt eine maßstäbliche Flurkarte (ALKIS) oder andere amtliche Karte über einen WMS-Dienst ein, z. B. für Feuerwehr-Übersichtspläne.");
            ui.horizontal(|ui| {
                ui.label("Dienst:");
                egui::ComboBox::from_id_salt("wms_preset").selected_text("Vorlage wählen …").width(260.0).show_ui(ui, |ui| {
                    for (name, url, _) in wms::PRESETS {
                        if ui.selectable_label(self.dlg.wms_url == *url, *name).clicked() {
                            self.dlg.wms_url = url.to_string();
                            self.dlg.wms_caps = None;
                            load_caps = true;
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                ui.label("WMS-Adresse:");
                ui.add(egui::TextEdit::singleline(&mut self.dlg.wms_url).desired_width(340.0));
                load_caps |= ui.button("Verbinden").clicked();
            });
            if let Some(caps) = &self.dlg.wms_caps {
                ui.separator();
                ui.label(RichText::new(&caps.title).strong());
                if !caps.supports_3857 {
                    ui.colored_label(egui::Color32::from_rgb(180, 0, 0), "Der Dienst bietet kein EPSG:3857 an; die Karte kann verzerrt oder leer sein.");
                }
                ui.label("Ebenen:");
                egui::ScrollArea::vertical().max_height(150.0).id_salt("wms_layers").show(ui, |ui| {
                    for (l, on) in caps.layers.iter().zip(self.dlg.wms_sel.iter_mut()) {
                        ui.checkbox(on, &l.title).on_hover_text(&l.name);
                    }
                });
                if !caps.terms.is_empty() {
                    ui.label(RichText::new("Nutzungsbedingungen des Dienstes:").strong());
                    egui::ScrollArea::vertical().max_height(110.0).id_salt("wms_terms").show(ui, |ui| {
                        ui.label(RichText::new(&caps.terms).small());
                    });
                }
                ui.checkbox(&mut self.dlg.wms_terms_ok, "Ich habe die Nutzungsbedingungen geprüft; die Nutzung für diesen Plan ist zulässig.");
                ui.label(RichText::new(format!("Quellenvermerk im Plan: {}", caps.attribution)).weak().small());
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Adresse:");
                    ui.text_edit_singleline(&mut self.dlg.osm_query);
                    search = ui.button("Suchen").clicked();
                });
                ui.label(RichText::new("Die Adresse wird zur Suche an nominatim.openstreetmap.org übertragen.").weak().small());
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut self.dlg.osm_lat).speed(0.0001).range(-85.0..=85.0).prefix("Breite ").max_decimals(6));
                    ui.add(egui::DragValue::new(&mut self.dlg.osm_lon).speed(0.0001).range(-180.0..=180.0).prefix("Länge ").max_decimals(6));
                });
                ui.horizontal(|ui| {
                    ui.label("Maßstab 1:");
                    for s in [250.0, 500.0, 1000.0, 2000.0, 5000.0] {
                        ui.selectable_value(&mut self.dlg.wms_scale, s, format!("{s}"));
                    }
                    ui.add(egui::DragValue::new(&mut self.dlg.wms_scale).range(100.0..=50000.0).speed(10.0));
                });
                let area = templates::layout(self.doc()).plan;
                let s = self.dlg.wms_scale as f64 / 1000.0;
                ui.label(RichText::new(format!("Ausschnitt in der Natur: {:.0} × {:.0} m (Planfläche {:.0} × {:.0} mm)", area.w() as f64 * s, area.h() as f64 * s, area.w(), area.h())).weak());
                let any = self.dlg.wms_sel.iter().any(|b| *b);
                fetch = ui.add_enabled(any && self.dlg.wms_terms_ok, egui::Button::new("Karte laden und einfügen")).clicked();
            }
            if !self.dlg.wms_status.is_empty() {
                ui.label(&self.dlg.wms_status);
            }
        });
        if load_caps {
            match wms::capabilities(&self.dlg.wms_url) {
                Ok(caps) => {
                    let defaults = wms::PRESETS.iter().find(|p| p.1 == self.dlg.wms_url.trim()).map(|p| p.2).unwrap_or("");
                    self.dlg.wms_sel = caps.layers.iter().map(|l| defaults.split(',').any(|d| d == l.name)).collect();
                    if !self.dlg.wms_sel.iter().any(|b| *b) {
                        if let Some(first) = self.dlg.wms_sel.first_mut() {
                            *first = true;
                        }
                    }
                    self.dlg.wms_status = format!("{} Ebenen gefunden.", caps.layers.len());
                    self.dlg.wms_caps = Some(caps);
                    self.dlg.wms_terms_ok = false;
                }
                Err(e) => {
                    self.dlg.wms_caps = None;
                    self.dlg.wms_status = e;
                }
            }
        }
        if search {
            match osm::geocode(&self.dlg.osm_query) {
                Ok((lat, lon)) => {
                    self.dlg.osm_lat = lat;
                    self.dlg.osm_lon = lon;
                    self.dlg.wms_status = format!("Gefunden: {lat:.5}, {lon:.5}");
                }
                Err(e) => self.dlg.wms_status = e,
            }
        }
        if fetch {
            if let Some(caps) = self.dlg.wms_caps.clone() {
                let layers: Vec<String> = caps.layers.iter().zip(&self.dlg.wms_sel).filter(|(_, on)| **on).map(|(l, _)| l.name.clone()).collect();
                let area = templates::layout(self.doc()).plan;
                let s = self.dlg.wms_scale as f64 / 1000.0;
                let (w_m, h_m) = (area.w() as f64 * s, area.h() as f64 * s);
                // ca. 200 dpi auf dem Blatt, höchstens 4096 Pixel je Seite
                let ppm = (8.0f64).min(4096.0 / area.w().max(area.h()) as f64);
                let (pw, ph) = ((area.w() as f64 * ppm).round() as u32, (area.h() as f64 * ppm).round() as u32);
                match wms::get_map(&caps.map_url, &layers, self.dlg.osm_lat, self.dlg.osm_lon, w_m, h_m, pw, ph) {
                    Ok(data) => {
                        self.insert_image(data, area, 1.0);
                        let scale = self.dlg.wms_scale;
                        let tab = self.tab_mut();
                        let layer = tab.doc.find_layer_by_name("Grundriss");
                        tab.doc.add(
                            ObjKind::Text { pos: area.min + Pos::new(1.0, 1.0), text: caps.attribution.clone(), size: 2.2, bold: false, italic: false, align: TAlign::Links, rot: 0.0 },
                            Style::line(farbe::SCHWARZ, 0.2),
                            layer,
                        );
                        tab.doc.scale_n = scale;
                        self.status = format!("Karte eingefügt, Maßstab 1:{scale}.");
                        open = false;
                    }
                    Err(e) => self.dlg.wms_status = e,
                }
            }
        }
        self.dlg.wms = open;
    }

    fn dlg_confirm(&mut self, ctx: &egui::Context) {
        if let Some(i) = self.dlg.reload_conflict {
            let mut action = 0;
            win("Datei außerhalb geändert").collapsible(false).show(ctx, |ui| {
                ui.label(format!(
                    "„{}“ wurde außerhalb des Programms geändert (z. B. über die KI-Schnittstelle), hier gibt es aber ungespeicherte Änderungen.",
                    self.tabs.get(i).map(|t| t.title()).unwrap_or_default()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Neu laden (eigene Änderungen verwerfen)").clicked() {
                        action = 1;
                    }
                    if ui.button("Eigene Fassung behalten").clicked() {
                        action = 2;
                    }
                });
            });
            match action {
                1 => {
                    if i < self.tabs.len() {
                        self.reload_tab(i);
                    }
                    self.dlg.reload_conflict = None;
                }
                2 => {
                    if let Some(t) = self.tabs.get_mut(i) {
                        // externe Fassung ignorieren, bis erneut gespeichert/geändert wird
                        t.mtime = t.path.as_deref().and_then(crate::app::file_mtime);
                    }
                    self.dlg.reload_conflict = None;
                }
                _ => {}
            }
        }
        if let Some(i) = self.dlg.close_confirm {
            let mut open = true;
            let mut action = 0;
            win("Plan schließen").open(&mut open).show(ctx, |ui| {
                ui.label(format!("„{}“ wurde geändert. Änderungen speichern?", self.tabs.get(i).map(|t| t.title()).unwrap_or_default()));
                ui.horizontal(|ui| {
                    if ui.button("Speichern").clicked() {
                        action = 1;
                    }
                    if ui.button("Nicht speichern").clicked() {
                        action = 2;
                    }
                    if ui.button("Abbrechen").clicked() {
                        action = 3;
                    }
                });
            });
            match action {
                1 => {
                    self.active = i;
                    if self.save(false) {
                        self.close_tab(i, true);
                    }
                    self.dlg.close_confirm = None;
                }
                2 => {
                    self.close_tab(i, true);
                    self.dlg.close_confirm = None;
                }
                3 => self.dlg.close_confirm = None,
                _ => {
                    if !open {
                        self.dlg.close_confirm = None;
                    }
                }
            }
        }
        if self.dlg.quit_confirm {
            let mut open = true;
            let mut action = 0;
            win("Programm beenden").open(&mut open).show(ctx, |ui| {
                let n = self.tabs.iter().filter(|t| t.dirty).count();
                ui.label(format!("{n} Plan/Pläne mit ungespeicherten Änderungen. Trotzdem beenden?"));
                ui.horizontal(|ui| {
                    if ui.button("Alle speichern und beenden").clicked() {
                        action = 1;
                    }
                    if ui.button("Ohne Speichern beenden").clicked() {
                        action = 2;
                    }
                    if ui.button("Abbrechen").clicked() {
                        action = 3;
                    }
                });
            });
            match action {
                1 => {
                    let mut ok = true;
                    for i in 0..self.tabs.len() {
                        if self.tabs[i].dirty {
                            self.active = i;
                            ok &= self.save(false);
                        }
                    }
                    if ok {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    self.dlg.quit_confirm = false;
                }
                2 => {
                    self.allow_close = true;
                    self.dlg.quit_confirm = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                3 => self.dlg.quit_confirm = false,
                _ => {
                    if !open {
                        self.dlg.quit_confirm = false;
                    }
                }
            }
        }
    }

    // ---------- Archiv ----------

    pub fn archive(&mut self) {
        let base = crate::app::sanitize(&self.doc().info.objekt, &self.doc().info.geschoss);
        let res = (|| -> Result<(usize, Vec<u8>), String> {
            let mut buf = std::io::Cursor::new(Vec::new());
            let mut z = zip::ZipWriter::new(&mut buf);
            let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
            let mut n = 0;
            for (i, t) in self.tabs.iter().enumerate() {
                let name = t.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).or_else(|| t.name.clone()).unwrap_or_else(|| format!("Plan_{}", i + 1));
                z.start_file(format!("{name}.fplan"), opt).map_err(|e| e.to_string())?;
                z.write_all(serde_json::to_string(&t.doc).map_err(|e| e.to_string())?.as_bytes()).map_err(|e| e.to_string())?;
                z.start_file(format!("{name}.pdf"), opt).map_err(|e| e.to_string())?;
                z.write_all(&export::pdf_bytes(&[&t.doc], export::Stamp::Keiner)?).map_err(|e| e.to_string())?;
                n += 1;
            }
            z.finish().map_err(|e| e.to_string())?;
            Ok((n, buf.into_inner()))
        })();
        match res.and_then(|(n, b)| crate::plattform::speichern_unter(&format!("{base}.zip"), "ZIP-Archiv", "zip", &b).map(|p| (n, p))) {
            Ok((n, Some(p))) => self.status = format!("{n} Plan/Pläne mit PDF archiviert: {}", p.display()),
            Ok((_, None)) => {}
            Err(e) => self.error(format!("Archivieren fehlgeschlagen: {e}")),
        }
    }

    pub fn unarchive(&mut self) {
        crate::plattform::waehlen(crate::plattform::Zweck::ArchivEntpacken, "ZIP-Archiv", &["zip"]);
        self.dateien_verarbeiten();
    }

    /// Archiv öffnen: Desktop entpackt in einen Ordner, im Browser werden die Pläne direkt geöffnet.
    pub fn archiv_entpacken(&mut self, d: crate::plattform::Datei) {
        let dir = if crate::plattform::IST_WEB {
            None
        } else {
            match crate::plattform::ordner_waehlen("Zielordner wählen") {
                Some(d) => Some(d),
                None => return,
            }
        };
        let res = (|| -> Result<Vec<(String, Vec<u8>, Option<std::path::PathBuf>)>, String> {
            let mut z = zip::ZipArchive::new(std::io::Cursor::new(&d.bytes)).map_err(|e| e.to_string())?;
            let mut plans = Vec::new();
            for i in 0..z.len() {
                let mut file = z.by_index(i).map_err(|e| e.to_string())?;
                let Some(name) = file.enclosed_name() else { continue };
                let fname = name.file_name().unwrap_or_default().to_string_lossy().into_owned();
                let mut buf = Vec::new();
                file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                let out = dir.as_ref().map(|d| d.join(&fname));
                if let Some(o) = &out {
                    std::fs::write(o, &buf).map_err(|e| e.to_string())?;
                }
                if fname.ends_with(".fplan") {
                    plans.push((fname, buf, out));
                }
            }
            Ok(plans)
        })();
        match res {
            Ok(plans) => {
                let n = plans.len();
                for (name, bytes, pfad) in plans {
                    self.datei_verarbeiten(crate::plattform::Datei { zweck: crate::plattform::Zweck::PlanOeffnen, name, bytes, pfad });
                }
                self.status = match dir {
                    Some(d) => format!("{n} Plan/Pläne entpackt nach {}", d.display()),
                    None => format!("{n} Plan/Pläne aus dem Archiv geöffnet."),
                };
            }
            Err(e) => self.error(format!("Dearchivieren fehlgeschlagen: {e}")),
        }
    }
}
