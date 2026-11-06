//! Oberfläche: Menüband, Seitenleisten, Statusleiste, Tastenkürzel.

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Response, RichText, Sense, Stroke, Vec2, WidgetInfo};

use crate::app::{App, Tool};
use crate::canvas::{self, c32};
use crate::export::Stamp;
use crate::geom::*;
use crate::model::*;
use crate::symbols::{self, Category};
use crate::{ops, presets};

const RIBBON_TABS: [&str; 6] = ["Datei", "Plan", "Einfügen", "Bearbeiten", "Ansicht", "Extras"];

// ---------- Farben und Maße des Menübands (an die Windows-Oberfläche angelehnt) ----------

pub const AKZENT: Color32 = Color32::from_rgb(15, 108, 189);
const HOVER: Color32 = Color32::from_rgb(235, 235, 235);
const GEDRUECKT: Color32 = Color32::from_rgb(222, 222, 222);
const AKTIV: Color32 = Color32::from_rgb(214, 231, 247);
const AKTIV_RAND: Color32 = Color32::from_rgb(143, 190, 232);
const TRENNER: Color32 = Color32::from_rgb(222, 222, 222);
const GRUPPENTITEL: Color32 = Color32::from_rgb(110, 110, 110);
const GESPERRT: Color32 = Color32::from_rgb(165, 165, 165);
pub const BAND_HINTERGRUND: Color32 = Color32::from_rgb(250, 250, 250);

/// Höhe des Inhalts einer Gruppe (große Schaltfläche bzw. drei kleine übereinander)
const INHALT_H: f32 = 70.0;
const KLEIN_H: f32 = 22.0;

/// Helles, ruhiges Erscheinungsbild für die ganze Anwendung.
pub fn apply_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    ctx.style_mut_of(egui::Theme::Light, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(243, 243, 243);
        v.window_fill = Color32::WHITE;
        v.extreme_bg_color = Color32::WHITE;
        v.faint_bg_color = Color32::from_rgb(248, 248, 248);
        v.window_corner_radius = egui::CornerRadius::same(8);
        v.menu_corner_radius = egui::CornerRadius::same(6);
        v.window_stroke = Stroke::new(1.0, Color32::from_rgb(210, 210, 210));
        v.selection.bg_fill = AKTIV;
        v.selection.stroke = Stroke::new(1.0, AKZENT);
        v.hyperlink_color = AKZENT;
        let r = egui::CornerRadius::same(4);
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = r;
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(224, 224, 224));
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(36, 36, 36));
        v.widgets.inactive.bg_fill = Color32::WHITE;
        v.widgets.inactive.weak_bg_fill = Color32::WHITE;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(209, 209, 209));
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(36, 36, 36));
        v.widgets.hovered.bg_fill = Color32::from_rgb(245, 245, 245);
        v.widgets.hovered.weak_bg_fill = HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(190, 190, 190));
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::BLACK);
        v.widgets.active.bg_fill = GEDRUECKT;
        v.widgets.active.weak_bg_fill = GEDRUECKT;
        v.widgets.active.bg_stroke = Stroke::new(1.0, AKZENT);
        v.widgets.open.weak_bg_fill = AKTIV;
        s.spacing.button_padding = Vec2::new(8.0, 3.0);
        s.spacing.item_spacing = Vec2::new(6.0, 4.0);
        s.spacing.interact_size.y = 22.0;
        use egui::TextStyle::*;
        s.text_styles.insert(Body, FontId::proportional(13.0));
        s.text_styles.insert(Button, FontId::proportional(13.0));
        s.text_styles.insert(Small, FontId::proportional(10.5));
        s.text_styles.insert(Heading, FontId::proportional(19.0));
    });
}

#[derive(Clone, Copy, Default)]
struct Opt {
    selected: bool,
    disabled: bool,
    menu: bool,
    icon_only: bool,
}
impl Opt {
    fn menu() -> Self {
        Self { menu: true, ..Self::default() }
    }
    fn sel(selected: bool) -> Self {
        Self { selected, ..Self::default() }
    }
    fn enabled(enabled: bool) -> Self {
        Self { disabled: !enabled, ..Self::default() }
    }
}

/// Gruppe im Menüband: Inhalt, darunter der Titel, rechts eine feine Trennlinie.
fn group(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let inner = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
        let content = ui.horizontal_top(|ui| {
            ui.set_min_height(INHALT_H);
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 1.0);
            add(ui);
        });
        let font = FontId::proportional(11.0);
        let galley = ui.painter().layout_no_wrap(title.to_string(), font, GRUPPENTITEL);
        let w = content.response.rect.width().max(galley.size().x + 8.0);
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 16.0), Sense::hover());
        ui.painter().galley(Pos2::new(r.center().x - galley.size().x / 2.0, r.min.y + 1.0), galley, GRUPPENTITEL);
    });
    let h = inner.response.rect.height();
    let (r, _) = ui.allocate_exact_size(Vec2::new(11.0, h), Sense::hover());
    ui.painter().vline(r.center().x, (r.min.y + 4.0)..=(r.max.y - 4.0), Stroke::new(1.0, TRENNER));
}

/// Spalte mit bis zu drei kleinen Schaltflächen übereinander.
fn col(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(2.0, 1.0);
        add(ui);
    });
}

/// Zeile innerhalb einer Spalte (für Auswahllisten und Zahlenfelder).
fn row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(Vec2::new(0.0, KLEIN_H + 1.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        ui.spacing_mut().interact_size.y = 20.0;
        add(ui);
    });
}

fn chevron(p: &egui::Painter, c: Pos2, col: Color32) {
    p.add(egui::Shape::convex_polygon(vec![Pos2::new(c.x - 3.0, c.y - 1.5), Pos2::new(c.x + 3.0, c.y - 1.5), Pos2::new(c.x, c.y + 1.8)], col, Stroke::NONE));
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
    /// Schaltfläche des Menübands: groß (Symbol über Text) oder klein (Symbol neben Text).
    fn rb(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, big: bool, o: Opt, tip: &str) -> Response {
        let txt = if o.disabled { GESPERRT } else { Color32::from_rgb(36, 36, 36) };
        let font = FontId::proportional(12.5);
        let (size, galley) = if big {
            // große Schaltflächen ohne Auslassungspunkte (wie in Office)
            let shown = label.trim_end_matches('…').trim_end();
            // Umbruch nur zwischen Wörtern; lange Wörter verbreitern die Schaltfläche
            let longest = shown.split(' ').map(|w| ui.painter().layout_no_wrap(w.to_string(), font.clone(), txt).size().x).fold(0.0_f32, f32::max);
            let mut job = egui::text::LayoutJob::simple(shown.to_string(), font, txt, longest.max(78.0) + 1.0);
            job.wrap.max_rows = 2;
            job.wrap.break_anywhere = false;
            job.halign = egui::Align::Center;
            let g = ui.painter().layout_job(job);
            (Vec2::new((g.size().x + 14.0).max(48.0), INHALT_H), Some(g))
        } else if o.icon_only {
            (Vec2::new(if o.menu { 34.0 } else { 26.0 }, KLEIN_H), None)
        } else {
            let g = ui.painter().layout_no_wrap(label.to_string(), font, txt);
            (Vec2::new(g.size().x + 32.0 + if o.menu { 12.0 } else { 0.0 }, KLEIN_H), Some(g))
        };
        let sense = if o.disabled { Sense::hover() } else { Sense::click() };
        let (rect, resp) = ui.allocate_exact_size(size, sense);
        if ui.is_rect_visible(rect) {
            let p = ui.painter();
            let open = o.menu && egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&resp));
            let (fill, stroke) = if o.disabled {
                (Color32::TRANSPARENT, Stroke::NONE)
            } else if resp.is_pointer_button_down_on() {
                (GEDRUECKT, Stroke::NONE)
            } else if o.selected || open {
                (AKTIV, Stroke::new(1.0, AKTIV_RAND))
            } else if resp.hovered() {
                (HOVER, Stroke::NONE)
            } else {
                (Color32::TRANSPARENT, Stroke::NONE)
            };
            p.rect(rect, 4.0, fill, stroke, egui::StrokeKind::Inside);
            let tint = if o.disabled { Color32::from_white_alpha(90) } else { Color32::WHITE };
            let isz = if big { 32.0 } else { 16.0 };
            let irect = if big {
                Rect::from_min_size(Pos2::new(rect.center().x - isz / 2.0, rect.min.y + 6.0), Vec2::splat(isz))
            } else {
                Rect::from_min_size(Pos2::new(rect.min.x + 5.0, rect.center().y - isz / 2.0), Vec2::splat(isz))
            };
            if let Some(t) = self.icons.get(ui.ctx(), icon, isz) {
                ui.painter().image(t.id(), irect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint);
            }
            let p = ui.painter();
            if let Some(g) = galley {
                if big {
                    let y = rect.min.y + 42.0;
                    p.galley(Pos2::new(rect.center().x, y), g.clone(), txt);
                    if o.menu {
                        let lines = g.rows.len().max(1) as f32;
                        chevron(p, Pos2::new(rect.center().x, (y + lines * 14.5 + 4.0).min(rect.max.y - 4.0)), txt);
                    }
                } else {
                    p.galley(Pos2::new(rect.min.x + 26.0, rect.center().y - g.size().y / 2.0), g, txt);
                    if o.menu {
                        chevron(p, Pos2::new(rect.max.x - 9.0, rect.center().y), txt);
                    }
                }
            } else if o.menu {
                chevron(p, Pos2::new(rect.max.x - 8.0, rect.center().y), txt);
            }
        }
        let enabled = !o.disabled;
        let selected = o.selected;
        resp.widget_info(|| WidgetInfo::selected(egui::Role::Button, enabled, selected, label));
        let tip = if !tip.is_empty() { tip.to_string() } else if o.icon_only || label.contains('\n') { label.to_string() } else { String::new() };
        if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
    }

    fn big(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, o: Opt, tip: &str) -> Response {
        self.rb(ui, icon, label, true, o, tip)
    }
    fn small(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, o: Opt, tip: &str) -> Response {
        self.rb(ui, icon, label, false, o, tip)
    }
    fn tiny(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, mut o: Opt, tip: &str) -> Response {
        o.icon_only = true;
        self.rb(ui, icon, label, false, o, tip)
    }

    fn set_tool(&mut self, tool: Tool) {
        self.draft.clear();
        self.measure = None;
        self.tool = if self.tool == tool { Tool::Select } else { tool };
    }

    fn tool_rb(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, tool: Tool, tip: &str, size: u8) {
        let o = Opt::sel(self.tool == tool);
        let r = match size {
            2 => self.big(ui, icon, label, o, tip),
            1 => self.small(ui, icon, label, o, tip),
            _ => self.tiny(ui, icon, label, o, tip),
        };
        if r.clicked() {
            self.set_tool(tool);
        }
    }

    /// Umschalter (Häkchen-Ersatz) als kleine Schaltfläche.
    fn toggle(&mut self, ui: &mut egui::Ui, icon: &'static str, label: &str, v: &mut bool, tip: &str) -> bool {
        if self.small(ui, icon, label, Opt::sel(*v), tip).clicked() {
            *v = !*v;
            return true;
        }
        false
    }

    pub fn ribbon(&mut self, ui: &mut egui::Ui) {
        // Reiterzeile
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_space(4.0);
            for (i, t) in RIBBON_TABS.iter().enumerate() {
                let active = self.ribbon == i;
                let font = FontId::proportional(13.5);
                let col = if active { AKZENT } else { Color32::from_rgb(50, 50, 50) };
                let galley = ui.painter().layout_no_wrap(t.to_string(), font, col);
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(galley.size().x + 22.0, 30.0), Sense::click());
                if resp.hovered() && !active {
                    ui.painter().rect_filled(rect.shrink2(Vec2::new(0.0, 3.0)), 4.0, HOVER);
                }
                let gw = galley.size().x;
                ui.painter().galley(Pos2::new(rect.center().x - gw / 2.0, rect.center().y - galley.size().y / 2.0 - 1.0), galley, col);
                if active {
                    let y = rect.max.y - 2.0;
                    ui.painter().line_segment([Pos2::new(rect.center().x - gw / 2.0, y), Pos2::new(rect.center().x + gw / 2.0, y)], Stroke::new(3.0, AKZENT));
                }
                resp.widget_info(|| WidgetInfo::selected(egui::Role::Button, true, active, *t));
                if resp.clicked() {
                    self.ribbon = i;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                ui.label(RichText::new(format!("Werkzeug: {}", self.tool.name())).color(GRUPPENTITEL));
            });
        });
        // Band mit den Gruppen; bei schmalem Fenster waagerecht scrollen (wie Office-Menübänder)
        egui::Frame::new().fill(BAND_HINTERGRUND).stroke(Stroke::new(1.0, TRENNER)).corner_radius(6.0).inner_margin(egui::Margin { left: 6, right: 6, top: 4, bottom: 2 }).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
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


    fn recent_menu(&mut self, ui: &mut egui::Ui) {
        let recent = self.settings.recent.clone();
        if recent.is_empty() {
            ui.label("Noch keine.");
        }
        for p in recent {
            if ui.button(p.display().to_string()).clicked() {
                self.open_path(&p);
                ui.close();
            }
        }
    }

    fn import_menu(&mut self, ui: &mut egui::Ui) {
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
    }

    fn export_menu(&mut self, ui: &mut egui::Ui) {
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
    }

    fn pdf_menu_items(&mut self, ui: &mut egui::Ui) {
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
    }

    fn pdf_menu(&mut self, ui: &mut egui::Ui, big: bool) {
        let r = self.rb(ui, "pdf", "PDF erstellen …", big, Opt::menu(), "PDF erstellen, mit oder ohne Überdruck");
        egui::Popup::menu(&r).show(|ui| self.pdf_menu_items(ui));
    }

    fn ribbon_datei(&mut self, ui: &mut egui::Ui) {
        group(ui, "Plan", |ui| {
            let r = self.big(ui, "neu", "Neu", Opt::menu(), "Neuen Plan anlegen");
            egui::Popup::menu(&r).show(|ui| self.new_menu(ui));
            if self.big(ui, "oeffnen", "Öffnen …", Opt::default(), "Gespeicherten Plan öffnen").clicked() {
                self.open_dialog();
            }
            if self.big(ui, "speichern", "Speichern", Opt::default(), "Plan speichern (Strg+S)").clicked() {
                self.save(false);
            }
            col(ui, |ui| {
                if self.small(ui, "speichern_unter", "Speichern unter …", Opt::default(), "").clicked() {
                    self.save(true);
                }
                if self.small(ui, "schliessen", "Schließen", Opt::default(), "Aktuellen Plan schließen").clicked() {
                    self.close_tab(self.active, false);
                }
            });
        });
        group(ui, "Zuletzt geöffnet", |ui| {
            col(ui, |ui| {
                let r = self.small(ui, "zuletzt", "Letzte Pläne …", Opt::menu(), "");
                egui::Popup::menu(&r).show(|ui| self.recent_menu(ui));
                if self.small(ui, "muster", "Musterpläne …", Opt::default(), "").clicked() {
                    self.dlg.samples = true;
                }
                if self.small(ui, "willkommen", "Willkommen …", Opt::default(), "").clicked() {
                    self.dlg.welcome = true;
                }
            });
        });
        group(ui, "Ausgabe", |ui| {
            if self.big(ui, "drucken", "Drucken", Opt::default(), "Öffnet den Plan als PDF in der Druckvorschau").clicked() {
                self.print();
            }
            self.pdf_menu(ui, true);
        });
        if !crate::plattform::IST_WEB {
            group(ui, "Programm", |ui| {
                if self.big(ui, "beenden", "Beenden", Opt::default(), "").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        }
    }

    fn ribbon_plan(&mut self, ui: &mut egui::Ui) {
        group(ui, "Plan", |ui| {
            let r = self.big(ui, "neu", "Neu", Opt::menu(), "Neuen Plan anlegen");
            egui::Popup::menu(&r).show(|ui| self.new_menu(ui));
            col(ui, |ui| {
                if self.small(ui, "oeffnen", "Öffnen", Opt::default(), "").clicked() {
                    self.open_dialog();
                }
                if self.small(ui, "speichern", "Speichern", Opt::default(), "").clicked() {
                    self.save(false);
                }
                if self.small(ui, "drucken", "Drucken", Opt::default(), "Öffnet den Plan als PDF in der Druckvorschau").clicked() {
                    self.print();
                }
            });
            col(ui, |ui| {
                let r = self.small(ui, "importieren", "Importieren …", Opt::menu(), "Grundriss, CAD-Datei oder Symbol einlesen");
                egui::Popup::menu(&r).show(|ui| self.import_menu(ui));
                let r = self.small(ui, "exportieren", "Exportieren …", Opt::menu(), "Als DXF, SVG oder PNG speichern");
                egui::Popup::menu(&r).show(|ui| self.export_menu(ui));
                self.pdf_menu(ui, false);
            });
            if self.big(ui, "pruefen", "Plan prüfen …", Opt::default(), "Pflichtinhalte nach DIN ISO 23601 / DIN 14095 prüfen").clicked() {
                self.dlg.check = true;
            }
        });
        group(ui, "Plan einrichten", |ui| {
            let mut page = self.doc().page;
            let mut frame = self.doc().frame;
            let mut scale = self.doc().scale_n;
            col(ui, |ui| {
                row(ui, |ui| {
                    egui::ComboBox::from_id_salt("fmt").width(64.0).selected_text(page.format.name()).show_ui(ui, |ui| {
                        for f in PaperFormat::LISTE {
                            ui.selectable_value(&mut page.format, f, f.name());
                        }
                    });
                    if self.tiny(ui, "quer", "Quer", Opt::sel(page.landscape), "Querformat").clicked() {
                        page.landscape = true;
                    }
                    if self.tiny(ui, "hoch", "Hoch", Opt::sel(!page.landscape), "Hochformat").clicked() {
                        page.landscape = false;
                    }
                    ui.label("1:");
                    ui.add(egui::DragValue::new(&mut scale).range(1.0..=100000.0).speed(5.0)).on_hover_text("Maßstab");
                });
                row(ui, |ui| {
                    egui::ComboBox::from_id_salt("frame").width(214.0).selected_text(frame.name()).show_ui(ui, |ui| {
                        for f in FrameKind::ALL {
                            ui.selectable_value(&mut frame, f, f.name());
                        }
                    });
                });
                self.tool_rb(ui, "massstab_distanz", "Maßstab durch Entfernung", Tool::ScaleByDistance, "Zwei Punkte mit bekanntem Abstand anklicken", 1);
            });
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
            if self.big(ui, "rahmen", "Rahmen und Titel …", Opt::default(), "Planrahmen, Titel und Angaben").clicked() {
                self.dlg.page = true;
            }
            if self.big(ui, "legende", "Legende …", Opt::default(), "Legenden-Assistent").clicked() {
                self.dlg.legend = true;
            }
            col(ui, |ui| {
                if self.small(ui, "bereinigen", "Bereinigen", Opt::default(), "Leere Elemente und ungenutzte Legendeneinträge entfernen").clicked() {
                    self.cleanup();
                }
                if self.small(ui, "extrahieren", "Grundriss extrahieren …", Opt::default(), "Wände automatisch aus einem Bild erkennen").clicked() {
                    self.dlg.extract = true;
                }
            });
        });
        group(ui, "Lagerichtig ausrichten", |ui| {
            col(ui, |ui| {
                for (icon, label, deg) in [("drehen_l", "Inhalt −90°", -90.0), ("drehen_r", "Inhalt +90°", 90.0), ("drehen_180", "Inhalt 180°", 180.0)] {
                    if self.small(ui, icon, label, Opt::default(), "Gesamten Planinhalt um den Planbereich drehen (Standort-Ausrichtung)").clicked() {
                        self.rotate_content(deg);
                    }
                }
            });
            if self.big(ui, "plan_duplizieren", "Plan duplizieren", Opt::default(), "Kopie für einen weiteren Aushangort (Standort versetzen)").clicked() {
                self.duplicate_plan();
            }
        });
    }

    fn ribbon_einfuegen(&mut self, ui: &mut egui::Ui) {
        group(ui, "Auswahl", |ui| {
            self.tool_rb(ui, "auswahl", "Auswahl", Tool::Select, "Elemente auswählen und bearbeiten (Esc)", 2);
            col(ui, |ui| {
                self.tool_rb(ui, "verschieben", "Verschieben", Tool::Pan, "Ansicht verschieben (auch mittlere Maustaste oder Leertaste)", 1);
                self.tool_rb(ui, "zoom_bereich", "Bereich zoomen", Tool::ZoomBox, "Auf einen Bereich zoomen", 1);
            });
        });
        group(ui, "Bau und Fluchtwege", |ui| {
            self.tool_rb(ui, "wand", "Wand", Tool::Wall, "Wand (Linienzug mit Wandstärke)", 2);
            self.tool_rb(ui, "tuer", "Tür", Tool::Door, "Tür mit Schwenkbereich", 2);
            let r = self.big(ui, "fluchtweg", "Fluchtwege …", Opt::menu(), "Fluchtweg, Laufweg oder Fläche zeichnen");
            egui::Popup::menu(&r).show(|ui| {
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
            col(ui, |ui| {
                self.tool_rb(ui, "wand_ortho", "Wand (orth.)", Tool::WallOrtho, "Wand – nur waagerecht/senkrecht", 1);
                self.tool_rb(ui, "raum", "Raum", Tool::Room, "Raum mit Bezeichnung", 1);
                self.tool_rb(ui, "raummass", "Raummaß", Tool::RoomArea, "Raumfläche (m²) anzeigen", 1);
            });
            col(ui, |ui| {
                self.tool_rb(ui, "fenster", "Fenster", Tool::Window, "Fenster in Wand", 1);
                self.tool_rb(ui, "treppe", "Treppe", Tool::Stairs, "Treppe mit Laufrichtung", 1);
                self.tool_rb(ui, "pfeil", "Richtungspfeil", Tool::Symbol("PFEIL_GRUEN".into()), "Richtungspfeil Rettungsweg", 1);
            });
            col(ui, |ui| {
                self.tool_rb(ui, "raster", "Planraster", Tool::Grid, "Planraster mit Feldbezeichnungen (A1 …)", 1);
                self.tool_rb(ui, "geschoss", "Geschoss-Schnitt", Tool::FloorSection, "Geschossübersicht für Feuerwehrpläne (DIN 14095), dargestelltes Geschoss markiert", 1);
                let r = self.small(ui, "weitere", "Weitere …", Opt::menu(), "Revisionswolke, Gleis, Zaun, QR-Code …");
                egui::Popup::menu(&r).show(|ui| {
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
            });
        });
        group(ui, "Formen", |ui| {
            // Formenkatalog wie in Office: kleine Symbole in drei Reihen
            let formen: [(&'static str, &str, Tool, &str); 12] = [
                ("linie", "Linie", Tool::Line, "Linie"),
                ("rechteck", "Rechteck", Tool::Rect, "Rechteck"),
                ("kreis", "Kreis", Tool::Circle, "Kreis (Mittelpunkt, Radius)"),
                ("ellipse", "Ellipse", Tool::Ellipse, "Ellipse"),
                ("linienzug", "Linienzug", Tool::Polyline, "Offener Linienzug"),
                ("polygon", "Polygon", Tool::Polygon, "Geschlossenes Polygon"),
                ("neck", "N-Eck", Tool::NGon, "Regelmäßiges Vieleck"),
                ("bogen", "Bogen", Tool::Arc, "Kreisbogen"),
                ("spline", "Spline", Tool::Spline, "Kurve durch Punkte"),
                ("schlauch", "Schlauch", Tool::Hose, "Schlauchleitung (Doppellinie)"),
                ("masslinie", "Maßlinie", Tool::Dimension, "Bemaßung in Metern (nach Planmaßstab)"),
                ("container", "Container", Tool::Container, "Kasten (z. B. für Textblöcke, Übersichtsplan, Stempelfeld)"),
            ];
            for spalte in formen.chunks(3) {
                col(ui, |ui| {
                    for (icon, label, tool, tip) in spalte {
                        self.tool_rb(ui, icon, label, tool.clone(), tip, 0);
                    }
                });
            }
        });
        group(ui, "Text und Objekte", |ui| {
            self.tool_rb(ui, "text", "Text", Tool::Text, "Text", 2);
            col(ui, |ui| {
                if self.small(ui, "bild", "Bild", Opt::default(), "Bild einfügen").clicked() {
                    self.import_dialog("bild");
                }
                if self.small(ui, "pdf_einbetten", "Eingebettetes PDF", Opt::default(), "PDF-Seite einfügen").clicked() {
                    self.import_dialog("pdf");
                }
                if self.small(ui, "xref", "Externe Referenz", Opt::default(), "Anderen Plan als Gruppe einfügen").clicked() {
                    self.import_dialog("plan");
                }
            });
            col(ui, |ui| {
                self.tool_rb(ui, "tabelle", "Tabelle", Tool::Table, "Tabelle", 1);
                self.tool_rb(ui, "notiz", "Notiz", Tool::Note, "Notiz (wird nicht gedruckt)", 1);
            });
        });
        group(ui, "Planelemente", |ui| {
            self.tool_rb(ui, "legende", "Legende", Tool::Legend, "Legende als Rahmen aufziehen (wird automatisch befüllt)", 2);
            col(ui, |ui| {
                if self.small(ui, "legende", "Legende …", Opt::default(), "Legenden-Assistent").clicked() {
                    self.dlg.legend = true;
                }
                if self.small(ui, "laufkarte", "Rahmen für Laufkarten", Opt::default(), "Planrahmen auf Feuerwehr-Laufkarte (DIN 14675) umstellen").clicked() {
                    let tab = self.tab_mut();
                    tab.push_undo();
                    apply_frame_defaults(&mut tab.doc, FrameKind::Laufkarte);
                    tab.doc.frame = FrameKind::Laufkarte;
                    tab.fit = true;
                }
                if self.small(ui, "rahmen", "Rahmen und Titel …", Opt::default(), "").clicked() {
                    self.dlg.page = true;
                }
            });
        });
    }

    fn ribbon_bearbeiten(&mut self, ui: &mut egui::Ui) {
        let has_sel = !self.tab().sel.is_empty();
        group(ui, "Zwischenablage", |ui| {
            if self.big(ui, "einfuegen", "Einfügen", Opt::enabled(!self.clipboard.is_empty()), "Aus der Zwischenablage einfügen (Strg+V)").clicked() {
                self.paste(Pos::new(5.0, 5.0));
            }
            col(ui, |ui| {
                if self.small(ui, "ausschneiden", "Ausschneiden", Opt::enabled(has_sel), "").clicked() {
                    self.cut();
                }
                if self.small(ui, "kopieren", "Kopieren", Opt::enabled(has_sel), "").clicked() {
                    self.copy();
                }
                if self.small(ui, "duplizieren", "Duplizieren", Opt::enabled(has_sel), "").clicked() {
                    self.duplicate();
                }
            });
        });
        group(ui, "Bearbeiten", |ui| {
            col(ui, |ui| {
                if self.small(ui, "rueckgaengig", "Rückgängig", Opt::enabled(!self.tab().undo.is_empty()), "Strg+Z").clicked() {
                    self.tab_mut().undo();
                }
                if self.small(ui, "wiederholen", "Wiederholen", Opt::enabled(!self.tab().redo.is_empty()), "Strg+Y").clicked() {
                    self.tab_mut().redo();
                }
                if self.small(ui, "loeschen", "Löschen", Opt::enabled(has_sel), "Entf").clicked() {
                    self.delete_selection();
                }
            });
            col(ui, |ui| {
                let r = self.small(ui, "fuellen", "Polygon füllen …", Opt::menu(), "In umschlossene Fläche klicken");
                egui::Popup::menu(&r).show(|ui| {
                    for p in presets::PRESETS.iter() {
                        if ui.button(p.name).clicked() {
                            self.tool = Tool::FillPolygon(p.key.into());
                            ui.close();
                        }
                    }
                });
                self.tool_rb(ui, "plan_schneiden", "Plan schneiden", Tool::CutPlan, "Alles außerhalb eines Rechtecks entfernen", 1);
                self.tool_rb(ui, "punkt", "Punkt einfügen", Tool::InsertPoint, "Punkt in Linienzug/Wand einfügen", 1);
            });
            col(ui, |ui| {
                if self.small(ui, "symbolmanager", "Symbol-Manager …", Opt::default(), "").clicked() {
                    self.dlg.symbol_manager = true;
                }
                if self.small(ui, "suchen", "Suchen / Ersetzen …", Opt::default(), "").clicked() {
                    self.dlg.search = true;
                }
            });
        });
        group(ui, "Auswahl", |ui| {
            col(ui, |ui| {
                if self.small(ui, "alles", "Alles", Opt::default(), "Alles auswählen (Strg+A)").clicked() {
                    self.select_all();
                }
                if self.small(ui, "invertieren", "Invertieren", Opt::default(), "Auswahl umkehren").clicked() {
                    self.invert_selection();
                }
                if self.small(ui, "gleiche", "Nach Inhalt (gleiche)", Opt::enabled(has_sel), "Alle gleichartigen Elemente auswählen").clicked() {
                    self.select_similar();
                }
            });
        });
        group(ui, "Anordnen", |ui| {
            col(ui, |ui| {
                if self.small(ui, "gruppieren", "Gruppieren", Opt::enabled(has_sel), "").clicked() {
                    self.group();
                }
                if self.small(ui, "gruppe_aufheben", "Gruppe aufheben", Opt::enabled(has_sel), "").clicked() {
                    self.ungroup();
                }
                if self
                    .small(ui, "symbole_erkennen", "Symbole erkennen", Opt::default(), "Importierte Symbole erkennen und durch die Normzeichen der Bibliothek ersetzen (Legende wird ergänzt); nicht erkannte Symbole werden zu Gruppen zusammengefasst – in der Auswahl oder, ohne Auswahl, im ganzen Plan")
                    .clicked()
                {
                    self.symbole_erkennen();
                }
            });
            col(ui, |ui| {
                if self.small(ui, "mehrfach", "Mehrfachkopie …", Opt::enabled(has_sel), "").clicked() {
                    self.dlg.multicopy = true;
                }
                if self.small(ui, "versatz", "Versatz (Offset) …", Opt::enabled(has_sel), "Parallele Kopie im festen Abstand").clicked() {
                    self.dlg.offset = true;
                }
                let r = self.small(ui, "ausrichten", "Ausrichten …", Opt { menu: true, disabled: !has_sel, ..Opt::default() }, "");
                egui::Popup::menu(&r).show(|ui| {
                    for (i, n) in ["Links", "Horizontal zentriert", "Rechts", "Oben", "Vertikal zentriert", "Unten"].iter().enumerate() {
                        if ui.button(*n).clicked() {
                            self.align(i as u8);
                            ui.close();
                        }
                    }
                });
            });
            col(ui, |ui| {
                if self.tiny(ui, "drehen_l", "Drehen -90°", Opt::enabled(has_sel), "").clicked() {
                    self.transform_sel(|k, c| ops::rotate(k, c, -90.0));
                }
                if self.tiny(ui, "drehen_r", "Drehen +90°", Opt::enabled(has_sel), "").clicked() {
                    self.transform_sel(|k, c| ops::rotate(k, c, 90.0));
                }
                if self.tiny(ui, "spiegeln_h", "Spiegeln waagerecht", Opt::enabled(has_sel), "").clicked() {
                    self.transform_sel(|k, c| ops::mirror(k, c, true));
                }
            });
            col(ui, |ui| {
                if self.tiny(ui, "spiegeln_v", "Spiegeln senkrecht", Opt::enabled(has_sel), "").clicked() {
                    self.transform_sel(|k, c| ops::mirror(k, c, false));
                }
                if self.tiny(ui, "vorne", "Nach vorne", Opt::enabled(has_sel), "").clicked() {
                    self.order(true, true);
                }
                if self.tiny(ui, "hinten", "Nach hinten", Opt::enabled(has_sel), "").clicked() {
                    self.order(false, true);
                }
            });
        });
        group(ui, "Wand", |ui| {
            col(ui, |ui| {
                self.tool_rb(ui, "segment", "Segment ausschneiden", Tool::SegmentCut, "Wandsegment entfernen (z. B. für Durchgänge)", 1);
                self.tool_rb(ui, "punkt", "Schnittpunkt einfügen", Tool::InsertPoint, "Zusätzlichen Eckpunkt einfügen", 1);
                row(ui, |ui| {
                    self.tool_rb(ui, "ecke", "Ecke abrunden", Tool::Fillet, "Ecke mit Radius abrunden", 1);
                    ui.add(egui::DragValue::new(&mut self.settings.fillet_mm).range(0.5..=100.0).suffix(" mm")).on_hover_text("Radius");
                });
            });
        });
        group(ui, "Messen", |ui| {
            col(ui, |ui| {
                self.tool_rb(ui, "distanz", "Distanz [m]", Tool::MeasureM, "Strecke im Planmaßstab", 1);
                self.tool_rb(ui, "distanz", "Distanz [mm]", Tool::MeasureMm, "Strecke auf dem Blatt", 1);
                self.tool_rb(ui, "winkel", "Winkel", Tool::MeasureAngle, "Winkel messen", 1);
            });
            self.tool_rb(ui, "kurve", "Kurve Feuerwehrzufahrt", Tool::FireCurve, "Kurve nach DIN 14090 (Außenradius ≥ 10,5 m)", 2);
        });
    }

    fn ribbon_ansicht(&mut self, ui: &mut egui::Ui) {
        group(ui, "Zoom", |ui| {
            if self.big(ui, "zoom_fit", "Automatisch anpassen", Opt::default(), "Ganzen Plan zeigen").clicked() {
                self.tab_mut().fit = true;
            }
            col(ui, |ui| {
                if self.small(ui, "zoom_in", "Hereinzoomen", Opt::default(), "").clicked() {
                    self.tab_mut().zoom *= 1.25;
                }
                if self.small(ui, "zoom_out", "Herauszoomen", Opt::default(), "").clicked() {
                    self.tab_mut().zoom /= 1.25;
                }
            });
            col(ui, |ui| {
                self.tool_rb(ui, "zoom_bereich", "In Bereich zoomen", Tool::ZoomBox, "Rechteck aufziehen", 1);
                if self.small(ui, "zoom_auswahl", "Auf Auswahl", Opt::enabled(!self.tab().sel.is_empty()), "").clicked() {
                    let b = self.tab().sel_bounds().expand(10.0);
                    self.zoom_to(b);
                }
            });
        });
        group(ui, "Raster und Fang", |ui| {
            let mut s = std::mem::take(&mut self.settings);
            col(ui, |ui| {
                self.toggle(ui, "gitter", "Gitternetz", &mut s.grid_show, "Gitternetz anzeigen");
                self.toggle(ui, "fang", "Am Raster fangen", &mut s.grid_snap, "");
                self.toggle(ui, "punktfang", "Punktfang", &mut s.point_snap, "An End- und Eckpunkten fangen");
            });
            col(ui, |ui| {
                row(ui, |ui| {
                    ui.label("Raster");
                    ui.add(egui::DragValue::new(&mut s.grid_mm).range(0.5..=100.0).speed(0.5).suffix(" mm"));
                });
            });
            self.settings = s;
        });
        group(ui, "Hilfen", |ui| {
            let mut s = std::mem::take(&mut self.settings);
            col(ui, |ui| {
                self.toggle(ui, "lineal", "Lineal", &mut s.rulers, "");
                self.toggle(ui, "hilfslinie", "Hilfslinien", &mut s.guides, "Hilfslinien anzeigen");
            });
            self.settings = s;
            col(ui, |ui| {
                self.tool_rb(ui, "hilfslinie", "Hilfslinie setzen", Tool::Guide, "Klick: waagerecht, Umschalt+Klick: senkrecht", 1);
                if self.small(ui, "hilfslinie_loeschen", "Hilfslinien löschen", Opt::default(), "").clicked() {
                    let tab = self.tab_mut();
                    tab.push_undo();
                    tab.doc.guides.clear();
                }
            });
        });
        group(ui, "Darstellung", |ui| {
            col(ui, |ui| {
                row(ui, |ui| {
                    let mut c = Color32::from_rgb(self.settings.bg[0], self.settings.bg[1], self.settings.bg[2]);
                    if ui.color_edit_button_srgba(&mut c).on_hover_text("Hintergrundfarbe der Arbeitsfläche").changed() {
                        self.settings.bg = [c.r(), c.g(), c.b()];
                    }
                    ui.label("Hintergrund");
                });
                let before = self.settings.quality_high;
                row(ui, |ui| {
                    if self.small(ui, "qualitaet", "Hoch", Opt::sel(self.settings.quality_high), "Symbole in hoher Qualität darstellen").clicked() {
                        self.settings.quality_high = true;
                    }
                    if self.small(ui, "qualitaet", "Gering", Opt::sel(!self.settings.quality_high), "Schnellere Darstellung bei großen Plänen").clicked() {
                        self.settings.quality_high = false;
                    }
                });
                if before != self.settings.quality_high {
                    self.textures.quality_high = self.settings.quality_high;
                    self.textures.clear_symbols();
                }
            });
            if self.big(ui, "willkommen", "Willkommen", Opt::default(), "Willkommensdialog anzeigen").clicked() {
                self.dlg.welcome = true;
            }
        });
    }

    fn ribbon_extras(&mut self, ui: &mut egui::Ui) {
        group(ui, "Karten", |ui| {
            if self.big(ui, "osm", "OSM-Karte …", Opt::default(), "Kartenausschnitt aus OpenStreetMap einfügen").clicked() {
                self.dlg.osm = true;
            }
            if self.big(ui, "wms", "Flurkarte (WMS) …", Opt::default(), "Maßstäbliche Flurkarte (ALKIS) oder amtliche Karte über WMS einfügen").clicked() {
                self.dlg.wms = true;
            }
        });
        group(ui, "Vorlagen", |ui| {
            if self.big(ui, "bso", "Brandschutzordnung (Teil A) …", Opt::default(), "Brandschutzordnung nach DIN 14096 erstellen").clicked() {
                self.dlg.bso = true;
            }
            col(ui, |ui| {
                if self.small(ui, "muster", "Musterpläne …", Opt::default(), "").clicked() {
                    self.dlg.samples = true;
                }
                if self.small(ui, "svg_symbol", "Eigenes Symbol (SVG) …", Opt::default(), "").clicked() {
                    self.import_dialog("svg");
                }
                if self.small(ui, "logo", "Logo für Planrahmen …", Opt::default(), "").clicked() {
                    self.import_dialog("logo");
                }
            });
        });
        group(ui, "Daten", |ui| {
            col(ui, |ui| {
                if self.small(ui, "archiv", "Archivieren …", Opt::default(), "Plan inkl. PDF als ZIP sichern").clicked() {
                    self.archive();
                }
                if self.small(ui, "dearchiv", "Dearchivieren …", Opt::default(), "").clicked() {
                    self.unarchive();
                }
            });
            if self.big(ui, "optionen", "Optionen …", Opt::default(), "Einstellungen").clicked() {
                self.dlg.options = true;
            }
        });
        group(ui, "Hilfe", |ui| {
            if self.big(ui, "hilfe", "Benutzerhilfe", Opt::default(), "").clicked() {
                self.dlg.help = true;
            }
            if self.big(ui, "info", "Über", Opt::default(), "").clicked() {
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
