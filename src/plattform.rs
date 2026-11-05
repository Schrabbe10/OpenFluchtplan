//! Plattform-Schicht: alles, was sich zwischen Desktop (Windows) und Browser (WebAssembly)
//! unterscheidet – Dateien wählen, speichern, Einstellungen, PDF anzeigen, Schriften.
//! Der übrige Code ist für beide Plattformen gleich.

use std::cell::RefCell;

/// Wofür eine gewählte Datei bestimmt ist (Dateiauswahl im Browser ist asynchron).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zweck {
    PlanOeffnen,
    PlanEinfuegen,
    Bild,
    Pdf,
    Dxf,
    Dwg,
    SvgSymbol,
    Logo,
    /// Bild für „Grundriss extrahieren“ (Wanderkennung)
    WaendeBild,
    ArchivEntpacken,
}

/// Eine gewählte oder abgelegte Datei.
pub struct Datei {
    pub zweck: Zweck,
    pub name: String,
    pub bytes: Vec<u8>,
    /// Pfad auf dem Desktop (im Browser keiner)
    pub pfad: Option<std::path::PathBuf>,
}

thread_local! {
    static EINGANG: RefCell<Vec<Datei>> = const { RefCell::new(Vec::new()) };
    static CTX: RefCell<Option<eframe::egui::Context>> = const { RefCell::new(None) };
}

pub fn init(ctx: &eframe::egui::Context) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx.clone()));
}

fn ablegen(d: Datei) {
    EINGANG.with(|e| e.borrow_mut().push(d));
    CTX.with(|c| {
        if let Some(c) = c.borrow().as_ref() {
            c.request_repaint();
        }
    });
}

/// Seit dem letzten Aufruf gewählte Dateien (einmal je Bild abholen).
pub fn eingang() -> Vec<Datei> {
    EINGANG.with(|e| std::mem::take(&mut *e.borrow_mut()))
}

pub const IST_WEB: bool = cfg!(target_arch = "wasm32");

// ---------------------------------------------------------------- Desktop

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;
    use std::path::PathBuf;

    pub const SCHRIFTEN: &[&[u8]] = &[];

    pub fn waehlen(zweck: Zweck, filter: &str, endungen: &[&str]) {
        let Some(p) = rfd::FileDialog::new().add_filter(filter, endungen).pick_file() else { return };
        datei_laden(zweck, p);
    }

    /// Datei von der Festplatte in den Eingang legen (auch für Drag & Drop und Startargument).
    pub fn datei_laden(zweck: Zweck, p: PathBuf) {
        match std::fs::read(&p) {
            Ok(bytes) => {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                ablegen(Datei { zweck, name, bytes, pfad: Some(p) });
            }
            Err(e) => fehler_merken(format!("{}: {e}", p.display())),
        }
    }

    /// Per Drag & Drop abgelegte Datei.
    pub fn abgelegt(zweck: Zweck, f: std::sync::Arc<dyn eframe::egui::DroppedFile + Send + Sync>) {
        datei_laden(zweck, f.path().to_path_buf());
    }

    /// Speichern unter … ; liefert den gewählten Pfad.
    pub fn speichern_unter(vorschlag: &str, filter: &str, endung: &str, bytes: &[u8]) -> Result<Option<PathBuf>, String> {
        let Some(p) = rfd::FileDialog::new().add_filter(filter, &[endung]).set_file_name(vorschlag).save_file() else { return Ok(None) };
        std::fs::write(&p, bytes).map_err(|e| format!("Speichern fehlgeschlagen: {e}"))?;
        Ok(Some(p))
    }

    pub fn ordner_waehlen(titel: &str) -> Option<PathBuf> {
        rfd::FileDialog::new().set_title(titel).pick_folder()
    }

    /// PDF zum Ansehen/Drucken im Standardprogramm öffnen.
    pub fn pdf_anzeigen(bytes: &[u8]) -> Result<(), String> {
        let p = std::env::temp_dir().join(format!("Fluchtplan_Druck_{}.pdf", std::process::id()));
        std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
        open::that(&p).map_err(|e| format!("PDF-Anzeige konnte nicht geöffnet werden: {e}"))
    }

    fn einstellungen_pfad() -> Option<PathBuf> {
        std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("Fluchtplan").join("einstellungen.json"))
    }
    pub fn einstellungen_lesen() -> Option<String> {
        einstellungen_pfad().and_then(|p| std::fs::read_to_string(p).ok())
    }
    pub fn einstellungen_schreiben(s: &str) {
        if let Some(p) = einstellungen_pfad() {
            let _ = std::fs::create_dir_all(p.parent().unwrap());
            let _ = std::fs::write(p, s);
        }
    }
}

// ---------------------------------------------------------------- Browser

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use std::path::PathBuf;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    pub const SCHRIFTEN: &[&[u8]] = &[
        include_bytes!("../assets/fonts/LiberationSans-Regular.ttf"),
        include_bytes!("../assets/fonts/LiberationSans-Bold.ttf"),
        include_bytes!("../assets/fonts/LiberationSans-Italic.ttf"),
        include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf"),
    ];

    fn document() -> web_sys::Document {
        web_sys::window().and_then(|w| w.document()).expect("document")
    }

    pub fn waehlen(zweck: Zweck, _filter: &str, endungen: &[&str]) {
        let Ok(input) = document().create_element("input") else { return };
        let input: web_sys::HtmlInputElement = input.unchecked_into();
        input.set_type("file");
        input.set_accept(&endungen.iter().filter(|e| **e != "*").map(|e| format!(".{e}")).collect::<Vec<_>>().join(","));
        let inp = input.clone();
        let cb = Closure::once_into_js(move || {
            let Some(file) = inp.files().and_then(|f| f.get(0)) else { return };
            let name = file.name();
            wasm_bindgen_futures::spawn_local(async move {
                match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                    Ok(buf) => ablegen(Datei { zweck, name, bytes: js_sys::Uint8Array::new(&buf).to_vec(), pfad: None }),
                    Err(_) => fehler_merken(format!("{name} konnte nicht gelesen werden.")),
                }
            });
        });
        input.set_onchange(Some(cb.unchecked_ref()));
        input.click();
    }

    pub fn datei_laden(_zweck: Zweck, _p: PathBuf) {}

    /// Per Drag & Drop abgelegte Datei (Inhalt wird asynchron gelesen).
    pub fn abgelegt(zweck: Zweck, f: std::sync::Arc<dyn eframe::egui::DroppedFile + Send + Sync>) {
        wasm_bindgen_futures::spawn_local(async move {
            let name = f.path().file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match f.bytes_async().await {
                Ok(bytes) => ablegen(Datei { zweck, name, bytes, pfad: None }),
                Err(e) => fehler_merken(format!("{name}: {e}")),
            }
        });
    }

    fn blob_url(bytes: &[u8], typ: &str) -> Result<String, String> {
        let arr = js_sys::Array::new();
        arr.push(&js_sys::Uint8Array::from(bytes));
        let opt = web_sys::BlobPropertyBag::new();
        opt.set_type(typ);
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&arr, &opt).map_err(|e| format!("{e:?}"))?;
        web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("{e:?}"))
    }

    fn mime(endung: &str) -> &'static str {
        match endung {
            "pdf" => "application/pdf",
            "png" => "image/png",
            "svg" => "image/svg+xml",
            "zip" => "application/zip",
            "fplan" => "application/json",
            _ => "application/octet-stream",
        }
    }

    /// Im Browser: Datei herunterladen (der Browser fragt ggf. nach dem Speicherort).
    pub fn speichern_unter(vorschlag: &str, _filter: &str, endung: &str, bytes: &[u8]) -> Result<Option<PathBuf>, String> {
        let url = blob_url(bytes, mime(endung))?;
        let a: web_sys::HtmlAnchorElement = document().create_element("a").map_err(|e| format!("{e:?}"))?.unchecked_into();
        a.set_href(&url);
        a.set_download(vorschlag);
        a.click();
        // Adresse erst später freigeben, sonst bricht der Download ab
        let cb = Closure::once_into_js(move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        });
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 60_000);
        }
        Ok(Some(PathBuf::from(vorschlag)))
    }

    pub fn ordner_waehlen(_titel: &str) -> Option<PathBuf> {
        None
    }

    /// PDF in einem neuen Browser-Tab öffnen (dort drucken).
    pub fn pdf_anzeigen(bytes: &[u8]) -> Result<(), String> {
        let url = blob_url(bytes, "application/pdf")?;
        let w = web_sys::window().ok_or("kein Fenster")?;
        match w.open_with_url_and_target(&url, "_blank") {
            Ok(Some(_)) => Ok(()),
            _ => Err("Der Browser hat das neue Fenster blockiert (Pop-up-Blocker).".into()),
        }
    }

    fn speicher() -> Option<web_sys::Storage> {
        web_sys::window().and_then(|w| w.local_storage().ok().flatten())
    }
    pub fn einstellungen_lesen() -> Option<String> {
        speicher().and_then(|s| s.get_item("openfluchtplan.einstellungen").ok().flatten())
    }
    pub fn einstellungen_schreiben(s: &str) {
        if let Some(st) = speicher() {
            let _ = st.set_item("openfluchtplan.einstellungen", s);
        }
    }
}

pub use imp::*;

thread_local! {
    static FEHLER: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn fehler_merken(e: String) {
    FEHLER.with(|f| f.borrow_mut().push(e));
}

/// Fehler aus asynchronen Vorgängen (z. B. Datei nicht lesbar).
pub fn fehler() -> Vec<String> {
    FEHLER.with(|f| std::mem::take(&mut *f.borrow_mut()))
}
