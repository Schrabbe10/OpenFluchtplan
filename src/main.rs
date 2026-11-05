// Im Release kein Konsolenfenster
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bso;
mod canvas;
mod dialogs;
mod export;
mod extract;
mod geom;
mod i18n;
mod import;
mod model;
mod ops;
mod osm;
mod wms;
mod pruefung;
mod pdfvec;
mod presets;
mod render;
mod samples;
mod schnittstelle;
mod symbols;
mod templates;
#[cfg(test)]
mod tests;
mod tools;
mod ui;
#[cfg(test)]
mod ui_tests;

use eframe::egui;

impl eframe::App for app::App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Oberflächengröße aus den Optionen (Strg+/- ist dem Plan-Zoom vorbehalten)
        let scale = self.settings.ui_scale.clamp(0.75, 3.0);
        if ctx.options(|o| o.zoom_with_keyboard) {
            ctx.options_mut(|o| o.zoom_with_keyboard = false);
        }
        if (ctx.zoom_factor() - scale).abs() > 0.001 {
            ctx.set_zoom_factor(scale);
        }

        // Schließen mit ungespeicherten Änderungen abfangen
        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.allow_close && self.tabs.iter().any(|t| t.dirty) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.dlg.quit_confirm = true;
            } else {
                self.settings.save();
            }
        }
        // Dateien per Drag & Drop
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        for p in dropped {
            match p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
                Some("fplan") => self.open_path(&p),
                Some("png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff") => match import::load_image_file(&p) {
                    Ok(d) => {
                        let area = templates::layout(self.doc()).plan;
                        self.insert_image(d, area, 0.6);
                    }
                    Err(e) => self.error(e),
                },
                Some("pdf") => self.import_pdf_page(&p, 0),
                _ => self.error(format!("Dateityp nicht unterstützt: {}", p.display())),
            }
        }

        self.shortcuts(&ctx);

        // Änderungen von außen (KI-Schnittstelle, Kommandozeile) übernehmen; regelmäßig nachsehen
        let now = ctx.input(|i| i.time);
        self.check_external_changes(now);
        ctx.request_repaint_after(std::time::Duration::from_secs(1));

        let title = format!("OpenFluchtplan – {}", self.tab().title());
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        egui::Panel::top("ribbon").show(ui, |ui| {
            ui.add_space(2.0);
            self.ribbon(ui);
            ui.add_space(2.0);
        });
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut close = None;
                for i in 0..self.tabs.len() {
                    let t = &self.tabs[i];
                    let label = format!("{} · {}", t.title(), t.doc.info.geschoss);
                    if ui.selectable_label(i == self.active, label).clicked() {
                        self.active = i;
                        self.reset_tool();
                    }
                    if ui.small_button("x").on_hover_text("Plan schließen").clicked() {
                        close = Some(i);
                    }
                    ui.separator();
                }
                if ui.small_button("+").on_hover_text("Neuer Flucht- und Rettungsplan").clicked() {
                    self.new_doc(model::FrameKind::FrVollstaendig, model::PaperFormat::A3, true);
                }
                if let Some(i) = close {
                    self.close_tab(i, false);
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(self.tool.hint());
                ui.separator();
                if let Some(m) = self.mouse_mm {
                    let d = self.doc();
                    ui.label(format!("x {:.1}  y {:.1} mm  ({:.2} m / {:.2} m)", m.x, m.y, d.real_m(m.x), d.real_m(m.y)));
                }
                ui.separator();
                ui.label(format!("M 1:{}", self.doc().scale_n));
                ui.label(format!("Zoom {:.0} %", self.tab().zoom / 3.7795 * 100.0));
                ui.label(format!("{} Objekte", self.doc().objects.len()));
                if !self.status.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(&self.status).color(egui::Color32::from_rgb(0, 90, 160)));
                }
            });
        });
        egui::Panel::left("bibliothek").default_size(260.0).show(ui, |ui| self.library_panel(ui));
        egui::Panel::right("eigenschaften").default_size(300.0).show(ui, |ui| self.right_panel(ui));
        egui::CentralPanel::no_frame().show(ui, |ui| self.canvas_ui(ui));

        self.dialogs(&ctx);
    }
}

/// Fenstersymbol aus dem eigenen Notausgang-Piktogramm.
fn app_icon() -> egui::IconData {
    let size = 64u32;
    let svg = symbols::find("E002").map(|s| s.svg.clone()).unwrap_or_default();
    let src = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100' width='{size}' height='{size}'>{svg}</svg>");
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    if let (Ok(tree), Some(mut pm)) = (resvg::usvg::Tree::from_str(&src, &resvg::usvg::Options::default()), resvg::tiny_skia::Pixmap::new(size, size)) {
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(size as f32 / 100.0, size as f32 / 100.0), &mut pm.as_mut());
        for (i, px) in pm.pixels().iter().enumerate() {
            let c = px.demultiply();
            rgba[i * 4..i * 4 + 4].copy_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
        }
    }
    egui::IconData { rgba, width: size, height: size }
}

fn main() -> eframe::Result<()> {
    use eframe::{egui_wgpu, wgpu};

    // Schnittstellen ohne Fenster: MCP-Server für KI-Assistenten und Kommandozeile
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("--mcp") => {
            schnittstelle::mcp();
            return Ok(());
        }
        Some("--cli") | Some("--hilfe") | Some("-h") | Some("--help") => {
            let rest = if args[0] == "--cli" { &args[1..] } else { &[][..] };
            std::process::exit(schnittstelle::cli(rest));
        }
        _ => {}
    }

    // Nur DirectX 12: OpenGL stürzt unter Remote-Desktop/Server ab, DX12 läuft überall
    // (ohne Grafikkarte über den Windows-Software-Renderer). Überschreibbar mit WGPU_BACKEND.
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = wgpu::Backends::from_env().unwrap_or(wgpu::Backends::DX12);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("OpenFluchtplan").with_icon(app_icon()).with_inner_size([1500.0, 950.0]).with_min_inner_size([1000.0, 650.0]).with_maximized(true),
        wgpu_options: egui_wgpu::WgpuConfiguration { wgpu_setup: egui_wgpu::WgpuSetup::CreateNew(setup), ..Default::default() },
        ..Default::default()
    };
    eframe::run_native("OpenFluchtplan", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
