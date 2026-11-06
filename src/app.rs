//! Programmzustand, Dokument-Tabs, Datei- und Bearbeitungsfunktionen.

use std::path::PathBuf;

use eframe::egui::Vec2;
use serde::{Deserialize, Serialize};

use crate::canvas::Textures;
use crate::export::{self, Stamp};
use crate::geom::*;
use crate::model::*;
use crate::plattform::{self, Datei, Zweck};
use crate::{import, ops, presets, samples, templates};

pub const UNDO_MAX: usize = 60;

#[derive(Clone, Debug, PartialEq)]
pub enum Tool {
    Select,
    Pan,
    ZoomBox,
    Line,
    Rect,
    Circle,
    Ellipse,
    Arc,
    Polyline,
    Polygon,
    NGon,
    Hose,
    Spline,
    Text,
    Dimension,
    Table,
    Wall,
    WallOrtho,
    Room,
    RoomArea,
    Window,
    Door,
    DoubleDoor,
    Stairs,
    Note,
    Route,
    RouteCurved,
    Laufweg,
    Area(String),
    Symbol(String),
    Grid,
    FloorSection,
    Cloud,
    Fence,
    Rail,
    ScaleBar,
    Qr,
    Legend,
    FireCurve,
    MeasureM,
    MeasureMm,
    MeasureAngle,
    ScaleByDistance,
    FillPolygon(String),
    InsertPoint,
    SegmentCut,
    Fillet,
    CutPlan,
    Guide,
    Container,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Need {
    One,
    Two,
    Three,
    Multi,
}

impl Tool {
    pub fn need(&self) -> Need {
        use Tool::*;
        match self {
            Select | Pan => Need::One,
            Text | Symbol(_) | Note | ScaleBar | Qr | RoomArea | FillPolygon(_) | InsertPoint | SegmentCut | Fillet | Table | Guide => Need::One,
            Line | Rect | Container | Circle | Ellipse | NGon | Dimension | Window | Door | DoubleDoor | Stairs | Grid | FloorSection | Legend | MeasureM | MeasureMm | ScaleByDistance | ZoomBox | CutPlan => Need::Two,
            Arc | MeasureAngle | FireCurve => Need::Three,
            Polyline | Polygon | Hose | Spline | Wall | WallOrtho | Room | Area(_) | Route | RouteCurved | Laufweg | Cloud | Fence | Rail => Need::Multi,
        }
    }

    pub fn hint(&self) -> String {
        use Tool::*;
        match self.need() {
            _ if *self == Select => "Auswahl: klicken, Umschalt+Klick ergänzt, ziehen verschiebt, Rahmen ziehen wählt aus. Griffpunkte verändern die Form.".into(),
            _ if *self == Pan => "Ansicht verschieben: mit gedrückter Maustaste ziehen.".into(),
            Need::One => match self {
                RoomArea => "Raum anklicken, um die Fläche (m²) ein-/auszublenden.".into(),
                FillPolygon(_) => "In eine von Wänden umschlossene Fläche klicken.".into(),
                InsertPoint => "Auf einen Linienzug/eine Wand klicken, um einen Punkt einzufügen.".into(),
                SegmentCut => "Auf ein Wandsegment klicken, um es zu entfernen.".into(),
                Fillet => "Auf eine Ecke einer Wand/eines Linienzugs klicken, um sie abzurunden.".into(),
                Guide => "Klicken setzt eine Hilfslinie (Umschalt = senkrecht).".into(),
                _ => "Klicken setzt das Element. Esc beendet das Werkzeug.".into(),
            },
            Need::Two => match self {
                Door | DoubleDoor => "Angelpunkt klicken, dann Richtung/Breite klicken.".into(),
                Dimension => "Anfangs- und Endpunkt klicken.".into(),
                ScaleByDistance => "Zwei Punkte mit bekanntem Abstand klicken.".into(),
                MeasureM | MeasureMm => "Zwei Punkte klicken, um zu messen.".into(),
                _ => "Ersten und zweiten Punkt klicken. Esc bricht ab.".into(),
            },
            Need::Three => match self {
                Arc => "Mittelpunkt, Startpunkt, Endpunkt klicken.".into(),
                MeasureAngle => "Ersten Schenkel, Scheitel, zweiten Schenkel klicken.".into(),
                FireCurve => "Kurvenmittelpunkt, Beginn (Außenradius), Ende klicken.".into(),
                _ => String::new(),
            },
            Need::Multi => "Punkte klicken (Umschalt = 45°-Raster). Doppelklick oder Enter beendet, Esc bricht ab.".into(),
        }
    }

    pub fn name(&self) -> String {
        use Tool::*;
        match self {
            Select => "Auswahl".into(),
            Pan => "Verschieben".into(),
            ZoomBox => "Bereich zoomen".into(),
            Line => "Linie".into(),
            Rect => "Rechteck".into(),
            Circle => "Kreis".into(),
            Ellipse => "Ellipse".into(),
            Arc => "Bogen".into(),
            Polyline => "Linienzug".into(),
            Polygon => "Polygon".into(),
            NGon => "N-Eck".into(),
            Hose => "Schlauch".into(),
            Spline => "Spline".into(),
            Text => "Text".into(),
            Dimension => "Maßlinie".into(),
            Table => "Tabelle".into(),
            Wall => "Wand".into(),
            WallOrtho => "Wand (orthogonal)".into(),
            Room => "Raum".into(),
            RoomArea => "Raummaß".into(),
            Window => "Fenster".into(),
            Door => "Tür".into(),
            DoubleDoor => "Doppeltür".into(),
            Stairs => "Treppe".into(),
            Note => "Notiz".into(),
            Route => "Fluchtweg".into(),
            RouteCurved => "Fluchtweg (Bogen)".into(),
            Laufweg => "Laufweg Feuerwehr".into(),
            Area(k) => presets::find(k).map(|p| p.name.to_string()).unwrap_or_else(|| "Fläche".into()),
            Symbol(id) => crate::symbols::find(id).map(|s| s.name.to_string()).unwrap_or_else(|| id.clone()),
            Grid => "Planraster".into(),
            FloorSection => "Geschoss-Schnitt".into(),
            Cloud => "Revisionswolke".into(),
            Fence => "Zaun".into(),
            Rail => "Gleis".into(),
            ScaleBar => "Maßstabsleiste".into(),
            Qr => "QR-Code".into(),
            Legend => "Legende".into(),
            FireCurve => "Kurve Feuerwehrzufahrt".into(),
            MeasureM => "Distanz [m]".into(),
            MeasureMm => "Distanz [mm]".into(),
            MeasureAngle => "Winkel".into(),
            ScaleByDistance => "Maßstab durch Entfernung".into(),
            FillPolygon(_) => "Polygon füllen".into(),
            InsertPoint => "Punkt einfügen".into(),
            SegmentCut => "Segment ausschneiden".into(),
            Fillet => "Ecke abrunden".into(),
            CutPlan => "Plan schneiden".into(),
            Guide => "Hilfslinie".into(),
            Container => "Container".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub recent: Vec<PathBuf>,
    pub ersteller: String,
    pub grid_mm: f32,
    pub grid_show: bool,
    pub grid_snap: bool,
    pub point_snap: bool,
    pub rulers: bool,
    pub guides: bool,
    pub wall_mm: f32,
    pub symbol_mm: f32,
    pub text_mm: f32,
    pub route_mm: f32,
    pub fillet_mm: f32,
    pub auto_layers: bool,
    pub quality_high: bool,
    pub show_welcome: bool,
    pub bg: [u8; 3],
    pub table_rows: usize,
    pub table_cols: usize,
    pub ngon_n: u32,
    /// Oberflächengröße (1.0 = 100 %)
    pub ui_scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            ersteller: String::new(),
            grid_mm: 5.0,
            grid_show: false,
            grid_snap: false,
            point_snap: true,
            rulers: true,
            guides: true,
            wall_mm: 1.2,
            symbol_mm: 8.0,
            text_mm: 3.5,
            route_mm: 7.0,
            fillet_mm: 5.0,
            auto_layers: true,
            quality_high: true,
            show_welcome: true,
            bg: [120, 120, 120],
            table_rows: 4,
            table_cols: 3,
            ngon_n: 6,
            ui_scale: 1.0,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        plattform::einstellungen_lesen().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        plattform::einstellungen_schreiben(&serde_json::to_string_pretty(self).unwrap_or_default());
    }
    pub fn add_recent(&mut self, p: &std::path::Path) {
        self.recent.retain(|r| r != p);
        self.recent.insert(0, p.to_path_buf());
        self.recent.truncate(10);
        self.save();
    }
}

pub use crate::pdfvec::{PdfFit, PdfImportOpt};

/// Gewählte PDF-Datei für den Import-Dialog (Inhalt statt Pfad – funktioniert auch im Browser).
#[derive(Clone)]
pub struct PdfAuswahl {
    pub name: String,
    pub bytes: std::sync::Arc<Vec<u8>>,
    pub seiten: usize,
    pub seite: usize,
}

pub enum Drag {
    Move { last: Pos },
    Handle { id: u64, idx: usize },
    Box { start: Pos },
}

pub struct DocTab {
    pub doc: Doc,
    pub path: Option<PathBuf>,
    /// Dateiname ohne Pfad (Browser: zuletzt geöffnet/heruntergeladen)
    pub name: Option<String>,
    pub dirty: bool,
    pub undo: Vec<Doc>,
    pub redo: Vec<Doc>,
    pub sel: Vec<u64>,
    pub zoom: f32,
    pub offset: Vec2,
    pub fit: bool,
    pub last_edit: Option<(u64, f64)>,
    /// Änderungszeit der Datei beim letzten Laden/Speichern (für automatisches Neuladen)
    pub mtime: Option<std::time::SystemTime>,
}

pub fn file_mtime(p: &std::path::Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl DocTab {
    pub fn new(doc: Doc, path: Option<PathBuf>) -> Self {
        let mtime = path.as_deref().and_then(file_mtime);
        Self { doc, path, name: None, dirty: false, undo: Vec::new(), redo: Vec::new(), sel: Vec::new(), zoom: 2.0, offset: Vec2::ZERO, fit: true, last_edit: None, mtime }
    }
    pub fn title(&self) -> String {
        let n = self.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).or_else(|| self.name.clone()).unwrap_or_else(|| "Unbenannt".into());
        if self.dirty { format!("{n} *") } else { n }
    }
    pub fn push_undo(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > UNDO_MAX {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
        self.last_edit = None;
    }
    /// Eigenschafts-Änderungen am selben Objekt kurz hintereinander zusammenfassen.
    pub fn push_undo_coalesced(&mut self, before: Doc, key: u64, now: f64) {
        let same = matches!(self.last_edit, Some((k, t)) if k == key && now - t < 1.0);
        if !same {
            self.undo.push(before);
            if self.undo.len() > UNDO_MAX {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.last_edit = Some((key, now));
        self.dirty = true;
    }
    pub fn undo(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, prev));
            self.sel.retain(|id| self.doc.index_of(*id).is_some());
            self.dirty = true;
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, next));
            self.sel.retain(|id| self.doc.index_of(*id).is_some());
            self.dirty = true;
        }
    }
    pub fn selected_objs(&self) -> Vec<&Obj> {
        self.doc.objects.iter().filter(|o| self.sel.contains(&o.id)).collect()
    }
    pub fn sel_bounds(&self) -> BBox {
        self.selected_objs().iter().fold(BBox::EMPTY, |b, o| b.union(ops::bounds_doc(&self.doc, &o.kind)))
    }
}

#[derive(Default)]
pub struct Dialogs {
    pub welcome: bool,
    pub samples: bool,
    pub legend: bool,
    pub page: bool,
    pub multicopy: bool,
    pub multicopy_n: u32,
    pub multicopy_dx: f32,
    pub offset: bool,
    pub check: bool,
    pub offset_mm: f32,
    pub multicopy_dy: f32,
    pub search: bool,
    pub search_find: String,
    pub search_repl: String,
    pub search_case: bool,
    pub symbol_manager: bool,
    pub sm_from: String,
    pub sm_to: String,
    pub scale_dist: Option<f32>,
    pub scale_dist_m: f32,
    pub options: bool,
    pub about: bool,
    pub help: bool,
    pub bso: bool,
    pub osm: bool,
    pub osm_query: String,
    pub osm_lat: f64,
    pub osm_lon: f64,
    pub osm_zoom: u32,
    pub osm_status: String,
    /// Flurkarte / WMS (Standort aus den OSM-Feldern)
    pub wms: bool,
    pub wms_url: String,
    pub wms_caps: Option<crate::wms::Caps>,
    pub wms_sel: Vec<bool>,
    pub wms_scale: f32,
    pub wms_terms_ok: bool,
    pub wms_status: String,
    pub qr: Option<Pos>,
    pub qr_text: String,
    pub pdf_pages: Option<PdfAuswahl>,
    pub pdf_opt: PdfImportOpt,
    pub png_dpi: f32,
    pub png: bool,
    pub merge: bool,
    pub merge_sel: Vec<bool>,
    pub extract: bool,
    pub extract_threshold: u8,
    pub extract_min_mm: f32,
    pub close_confirm: Option<usize>,
    pub reload_conflict: Option<usize>,
    pub quit_confirm: bool,
    pub error: Option<String>,
}

pub struct App {
    pub tabs: Vec<DocTab>,
    pub active: usize,
    pub tool: Tool,
    pub draft: Vec<Pos>,
    pub drag: Option<Drag>,
    pub textures: Textures,
    pub icons: crate::icons::Icons,
    pub settings: Settings,
    pub clipboard: Vec<Obj>,
    pub ribbon: usize,
    pub lib_cat: usize,
    pub lib_search: String,
    pub lib_tab: usize,
    pub status: String,
    pub measure: Option<String>,
    pub dlg: Dialogs,
    pub mouse_mm: Option<Pos>,
    pub focus_text: bool,
    pub allow_close: bool,
    pub last_title: String,
    pub last_file_check: f64,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::canvas::install_fonts(&cc.egui_ctx);
        crate::ui::apply_theme(&cc.egui_ctx);
        plattform::init(&cc.egui_ctx);
        let settings = Settings::load();
        let mut textures = Textures::default();
        textures.quality_high = settings.quality_high;
        let mut doc = Doc::default();
        doc.info.ersteller = settings.ersteller.clone();
        let mut app = Self {
            tabs: vec![DocTab::new(doc, None)],
            active: 0,
            tool: Tool::Select,
            draft: Vec::new(),
            drag: None,
            textures,
            icons: Default::default(),
            dlg: Dialogs {
                welcome: settings.show_welcome,
                multicopy_n: 3,
                multicopy_dx: 20.0,
                offset_mm: 2.0,
                png_dpi: 200.0,
                extract_threshold: 110,
                extract_min_mm: 15.0,
                scale_dist_m: 10.0,
                osm_lat: 54.0917,
                osm_lon: 9.0706,
                osm_zoom: 17,
                wms_url: crate::wms::PRESETS[0].1.into(),
                wms_scale: 1000.0,
                ..Default::default()
            },
            settings,
            clipboard: Vec::new(),
            ribbon: 0,
            lib_cat: 0,
            lib_search: String::new(),
            lib_tab: 0,
            status: String::new(),
            measure: None,
            mouse_mm: None,
            focus_text: false,
            allow_close: false,
            last_title: String::new(),
            last_file_check: 0.0,
        };
        // Datei als Startargument (Doppelklick auf .fplan)
        if !plattform::IST_WEB {
            if let Some(arg) = std::env::args().nth(1) {
                let p = PathBuf::from(arg);
                if p.exists() {
                    app.open_path(&p);
                    app.dlg.welcome = false;
                }
            }
        }
        app
    }

    pub fn tab(&self) -> &DocTab {
        &self.tabs[self.active]
    }
    pub fn tab_mut(&mut self) -> &mut DocTab {
        &mut self.tabs[self.active]
    }
    pub fn doc(&self) -> &Doc {
        &self.tabs[self.active].doc
    }

    pub fn error(&mut self, e: impl Into<String>) {
        self.dlg.error = Some(e.into());
    }

    // ---------- Dateien ----------

    pub fn new_doc(&mut self, frame: FrameKind, format: PaperFormat, landscape: bool) {
        let mut doc = Doc { frame, page: Page { format, landscape }, ..Doc::default() };
        doc.info.ersteller = self.settings.ersteller.clone();
        if matches!(frame, FrameKind::BrandschutzVollstaendig | FrameKind::BrandschutzNurOben) {
            doc.frame_cfg.title = "BRANDSCHUTZPLAN".into();
            doc.frame_cfg.color = farbe::ROT;
        }
        if frame == FrameKind::Laufkarte {
            doc.frame_cfg.title = "FEUERWEHR-LAUFKARTE".into();
            doc.frame_cfg.color = farbe::ROT;
        }
        if frame == FrameKind::Feuerwehrplan {
            doc.info.planart = "Geschossplan".into();
        }
        self.add_tab(DocTab::new(doc, None));
    }

    pub fn add_tab(&mut self, tab: DocTab) {
        self.dlg.welcome = false;
        // leeren, unveränderten Starttab ersetzen
        if self.tabs.len() == 1 && !self.tabs[0].dirty && self.tabs[0].path.is_none() && self.tabs[0].doc.objects.is_empty() {
            self.tabs[0] = tab;
            self.active = 0;
        } else {
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }
        self.reset_tool();
    }

    pub fn open_dialog(&mut self) {
        plattform::waehlen(Zweck::PlanOeffnen, "Fluchtplan", &["fplan"]);
    }

    pub fn open_path(&mut self, p: &std::path::Path) {
        if let Some(i) = self.tabs.iter().position(|t| t.path.as_deref() == Some(p)) {
            self.active = i;
            return;
        }
        plattform::datei_laden(Zweck::PlanOeffnen, p.to_path_buf());
        self.dateien_verarbeiten();
    }

    /// Gewählte Dateien (Dialog, Drag & Drop, im Browser asynchron) ihrem Zweck zuführen.
    pub fn dateien_verarbeiten(&mut self) {
        for e in plattform::fehler() {
            self.error(e);
        }
        for d in plattform::eingang() {
            self.datei_verarbeiten(d);
        }
    }

    pub fn datei_verarbeiten(&mut self, d: Datei) {
        let area = templates::layout(self.doc()).plan;
        let plan = |d: &Datei| std::str::from_utf8(&d.bytes).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Doc>(s).map_err(|e| e.to_string()));
        match d.zweck {
            Zweck::PlanOeffnen => match plan(&d) {
                Ok(mut doc) => {
                    // Legende um Zeichen ergänzen, die noch nicht eingetragen sind (z. B. ältere oder fremd erstellte Dateien)
                    ops::sync_legend(&mut doc);
                    let mut tab = DocTab::new(doc, d.pfad.clone());
                    if d.pfad.is_none() {
                        // Browser: Name merken, damit „Speichern“ denselben Dateinamen vorschlägt
                        tab.name = Some(d.name.trim_end_matches(".fplan").to_string());
                    }
                    self.add_tab(tab);
                    if let Some(p) = &d.pfad {
                        self.settings.add_recent(p);
                    }
                    self.status = format!("Geöffnet: {}", d.name);
                }
                Err(e) => self.error(format!("Die Datei konnte nicht geöffnet werden:\n{e}")),
            },
            Zweck::PlanEinfuegen => match plan(&d) {
                Ok(other) => {
                    let tab = self.tab_mut();
                    tab.push_undo();
                    let id = tab.doc.alloc_id();
                    tab.doc.objects.push(Obj { id, layer: tab.doc.active_layer, kind: ObjKind::Group { children: other.objects }, style: Style::default(), print: true, locked: false });
                    tab.doc.custom_symbols.extend(other.custom_symbols);
                    tab.doc.renumber();
                    tab.sel = vec![tab.doc.objects.last().unwrap().id];
                    ops::sync_legend(&mut tab.doc);
                }
                Err(e) => self.error(e),
            },
            Zweck::Bild => match import::load_image_bytes(&d.name, &d.bytes) {
                Ok(data) => {
                    self.import_ziel(&d.name);
                    self.insert_image(data, area, 0.6)
                }
                Err(e) => self.error(e),
            },
            // Seite und Importoptionen immer abfragen (wie im Referenzprodukt)
            Zweck::Pdf => match import::pdf_page_count_bytes(d.bytes.clone()) {
                Ok(n) => self.dlg.pdf_pages = Some(PdfAuswahl { name: d.name, bytes: std::sync::Arc::new(d.bytes), seiten: n, seite: 0 }),
                Err(e) => self.error(e),
            },
            Zweck::Dxf | Zweck::Dwg => {
                let bytes = if d.zweck == Zweck::Dwg {
                    let Some(p) = d.pfad.as_deref() else { return self.error("DWG wird nur in der Desktop-Version unterstützt.") };
                    match import::dwg_to_dxf(p, "ACAD2010", false).and_then(|x| std::fs::read(x).map_err(|e| e.to_string())) {
                        Ok(b) => b,
                        Err(e) => return self.error(e),
                    }
                } else {
                    d.bytes
                };
                match import::load_dxf(&bytes, area, self.doc().scale_n, false) {
                    Ok(objs) => {
                        self.import_ziel(&d.name);
                        let tab = self.tab_mut();
                        tab.push_undo();
                        let layer = tab.doc.find_layer_by_name("Grundriss");
                        let id = tab.doc.alloc_id();
                        let mut children = objs;
                        for c in &mut children {
                            c.layer = layer;
                        }
                        tab.doc.objects.insert(0, Obj { id, layer, kind: ObjKind::Group { children }, style: Style::default(), print: true, locked: false });
                        tab.doc.renumber();
                        ops::sync_legend(&mut tab.doc);
                        tab.sel.clear();
                        self.status = "CAD-Grundriss importiert (als Gruppe auf Ebene „Grundriss“).".into();
                    }
                    Err(e) => self.error(e),
                }
            }
            Zweck::SvgSymbol => match std::str::from_utf8(&d.bytes).map_err(|e| e.to_string()).and_then(custom_symbol_from_svg) {
                Ok(svg) => {
                    let name = std::path::Path::new(&d.name).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Eigenes Symbol".into());
                    let tab = self.tab_mut();
                    tab.push_undo();
                    let id = format!("CUSTOM_{}", tab.doc.custom_symbols.len() + 1);
                    tab.doc.custom_symbols.push(CustomSymbol { id: id.clone(), name, svg });
                    self.tool = Tool::Symbol(id);
                    self.status = "Eigenes Symbol importiert – in den Plan klicken.".into();
                }
                Err(e) => self.error(e),
            },
            Zweck::Logo => match import::load_image_bytes(&d.name, &d.bytes) {
                Ok(img) => {
                    let tab = self.tab_mut();
                    tab.push_undo();
                    tab.doc.frame_cfg.logo = Some(img);
                }
                Err(e) => self.error(e),
            },
            Zweck::WaendeBild => self.waende_aus_datei(d),
            Zweck::ArchivEntpacken => self.archiv_entpacken(d),
        }
    }

    pub fn save(&mut self, save_as: bool) -> bool {
        let json = match serde_json::to_string(&self.doc()) {
            Ok(j) => j,
            Err(e) => {
                self.error(e.to_string());
                return false;
            }
        };
        // Desktop: vorhandene Datei direkt überschreiben; sonst „Speichern unter“ bzw. im Browser herunterladen
        let res = match (&self.tab().path, save_as, plattform::IST_WEB) {
            (Some(p), false, false) => std::fs::write(p, &json).map(|_| Some(p.clone())).map_err(|e| format!("Speichern fehlgeschlagen: {e}")),
            _ => {
                let base = self.tab().name.clone().unwrap_or_else(|| sanitize(&self.doc().info.objekt, &self.doc().info.geschoss));
                plattform::speichern_unter(&format!("{base}.fplan"), "Fluchtplan", "fplan", json.as_bytes())
            }
        };
        match res {
            Ok(Some(path)) => {
                let t = self.tab_mut();
                t.dirty = false;
                if plattform::IST_WEB {
                    t.name = path.file_stem().map(|s| s.to_string_lossy().into_owned());
                } else {
                    t.mtime = file_mtime(&path);
                    t.path = Some(path.clone());
                    self.settings.add_recent(&path);
                }
                self.status = format!("Gespeichert: {}", path.display());
                true
            }
            Ok(None) => false,
            Err(e) => {
                self.error(e);
                false
            }
        }
    }

    /// Exportdatei speichern (Desktop: Dialog, Browser: Download) und Status melden.
    fn ausgeben(&mut self, name: String, filter: &str, endung: &str, inhalt: Result<Vec<u8>, String>, meldung: impl FnOnce(&std::path::Path) -> String) {
        match inhalt.and_then(|b| plattform::speichern_unter(&name, filter, endung, &b)) {
            Ok(Some(p)) => self.status = meldung(&p),
            Ok(None) => {}
            Err(e) => self.error(e),
        }
    }

    /// Extern geänderte Dateien (z. B. durch die KI-Schnittstelle) neu laden. Höchstens einmal je Sekunde.
    pub fn check_external_changes(&mut self, now: f64) {
        if now - self.last_file_check < 1.0 || self.dlg.reload_conflict.is_some() {
            return;
        }
        self.last_file_check = now;
        for i in 0..self.tabs.len() {
            let Some(p) = self.tabs[i].path.clone() else { continue };
            let m = file_mtime(&p);
            if m.is_none() || m == self.tabs[i].mtime {
                continue;
            }
            if self.tabs[i].dirty {
                self.dlg.reload_conflict = Some(i);
                return;
            }
            self.reload_tab(i);
        }
    }

    pub fn reload_tab(&mut self, i: usize) {
        let Some(p) = self.tabs[i].path.clone() else { return };
        // halb geschriebene Datei: beim nächsten Durchlauf erneut versuchen
        let Ok(doc) = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| serde_json::from_str::<Doc>(&s).map_err(|e| e.to_string())) else { return };
        let t = &mut self.tabs[i];
        t.undo.push(std::mem::replace(&mut t.doc, doc));
        t.redo.clear();
        t.sel.retain(|id| t.doc.objects.iter().any(|o| o.id == *id));
        t.dirty = false;
        t.mtime = file_mtime(&p);
        self.status = format!("{} wurde außerhalb geändert und neu geladen (Rückgängig möglich).", p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default());
    }

    pub fn close_tab(&mut self, i: usize, force: bool) {
        if !force && self.tabs[i].dirty {
            self.dlg.close_confirm = Some(i);
            return;
        }
        self.tabs.remove(i);
        if self.tabs.is_empty() {
            self.tabs.push(DocTab::new(Doc::default(), None));
        }
        self.active = self.active.min(self.tabs.len() - 1);
        self.reset_tool();
    }

    pub fn open_sample(&mut self, i: usize) {
        if let Some((_, mut doc)) = samples::all().into_iter().nth(i) {
            doc.info.ersteller = if self.settings.ersteller.is_empty() { doc.info.ersteller } else { self.settings.ersteller.clone() };
            self.add_tab(DocTab::new(doc, None));
        }
    }

    fn dateiname(&self) -> String {
        self.tab().name.clone().unwrap_or_else(|| sanitize(&self.doc().info.objekt, &self.doc().info.geschoss))
    }

    pub fn export_pdf(&mut self, stamp: Stamp) {
        let name = format!("{}.pdf", self.dateiname());
        let pdf = export::pdf_bytes(&[self.doc()], stamp);
        self.ausgeben(name, "PDF", "pdf", pdf, |p| format!("PDF erstellt: {}", p.display()));
    }

    pub fn export_merged(&mut self, which: &[usize]) {
        let docs: Vec<&Doc> = which.iter().filter_map(|i| self.tabs.get(*i)).map(|t| &t.doc).collect();
        let n = docs.len();
        let pdf = export::pdf_bytes(&docs, Stamp::Keiner);
        self.ausgeben("Plaene.pdf".into(), "PDF", "pdf", pdf, |p| format!("{n} Pläne in {} zusammengeführt.", p.display()));
    }

    pub fn print(&mut self) {
        match export::pdf_bytes(&[self.doc()], Stamp::Keiner).and_then(|b| plattform::pdf_anzeigen(&b)) {
            Ok(()) => self.status = "Druckvorschau geöffnet – bitte dort drucken (Seitenanpassung: „Tatsächliche Größe“).".into(),
            Err(e) => self.error(e),
        }
    }

    pub fn export_other(&mut self, kind: &str) {
        let base = self.dateiname();
        let meldung = |p: &std::path::Path| format!("Exportiert: {}", p.display());
        match kind {
            "dxf" => {
                let b = export::dxf_bytes(self.doc());
                self.ausgeben(format!("{base}.dxf"), "AutoCAD DXF", "dxf", b, meldung);
            }
            "svg" => {
                let b = Ok(export::plan_svg(self.doc(), Stamp::Keiner).into_bytes());
                self.ausgeben(format!("{base}.svg"), "SVG", "svg", b, meldung);
            }
            "png" => {
                let b = export::render_png(self.doc(), self.dlg.png_dpi);
                self.ausgeben(format!("{base}.png"), "PNG", "png", b, meldung);
            }
            "dwg" => {
                // nur Desktop: über den ODA File Converter
                let tmp = std::env::temp_dir().join(format!("{base}.dxf"));
                let b = export::export_dxf(self.doc(), &tmp).and_then(|_| import::dwg_to_dxf(&tmp, "ACAD2010", true)).and_then(|out| std::fs::read(out).map_err(|e| e.to_string()));
                self.ausgeben(format!("{base}.dwg"), "AutoCAD DWG", "dwg", b, meldung);
            }
            _ => {}
        }
    }

    /// Grundriss-Importe (Bild, PDF, DXF/DWG) kommen in einen eigenen Tab, wenn der aktuelle Plan
    /// schon Inhalt hat – sonst lägen zwei Pläne übereinander. Rahmen, Format und Planangaben werden übernommen.
    pub fn import_ziel(&mut self, name: &str) {
        let src = self.doc();
        if src.objects.is_empty() {
            return;
        }
        let mut doc = Doc { frame: src.frame, page: src.page, scale_n: src.scale_n, ..Doc::default() };
        doc.frame_cfg = src.frame_cfg.clone();
        doc.info = src.info.clone();
        doc.legend.title = src.legend.title.clone();
        doc.legend.langs = src.legend.langs.clone();
        let mut tab = DocTab::new(doc, None);
        tab.name = std::path::Path::new(name).file_stem().map(|s| s.to_string_lossy().into_owned());
        self.add_tab(tab);
    }

    pub fn import_dialog(&mut self, kind: &str) {
        match kind {
            "bild" => plattform::waehlen(Zweck::Bild, "Bilder", &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff"]),
            "pdf" => plattform::waehlen(Zweck::Pdf, "PDF", &["pdf"]),
            "dxf" => plattform::waehlen(Zweck::Dxf, "DXF", &["dxf"]),
            "dwg" => plattform::waehlen(Zweck::Dwg, "DWG", &["dwg"]),
            "plan" => plattform::waehlen(Zweck::PlanEinfuegen, "Fluchtplan", &["fplan"]),
            "svg" => plattform::waehlen(Zweck::SvgSymbol, "SVG-Symbol", &["svg"]),
            "logo" => plattform::waehlen(Zweck::Logo, "Bilder", &["png", "jpg", "jpeg"]),
            _ => {}
        }
        self.dateien_verarbeiten();
    }

    pub fn import_pdf_page(&mut self, bytes: &[u8], page: usize) {
        let area = templates::layout(self.doc()).plan;
        match import::pdf_page_image_bytes(bytes.to_vec(), page, 200.0) {
            Ok(data) => self.insert_image(data, area, 0.6),
            Err(e) => self.error(e),
        }
    }

    /// PDF-Seite als bearbeitbare Vektorobjekte importieren.
    pub fn import_pdf_vector(&mut self, bytes: &[u8], page: usize) {
        let opt = self.dlg.pdf_opt;
        let pg = match crate::pdfvec::read_page_bytes(bytes.to_vec(), page) {
            Ok(pg) => pg,
            Err(e) => return self.error(e),
        };
        if pg.items.is_empty() {
            return self.error("Die PDF-Seite enthält keine Vektordaten (vermutlich ein Scan). Bitte als Bild importieren.");
        }
        let n = pg.items.len();
        let tab = self.tab_mut();
        tab.push_undo();
        tab.sel = crate::pdfvec::place(&mut tab.doc, pg, opt);
        tab.fit = true;
        let sel = tab.sel.clone();
        let zeichen = tab.doc.objects.iter().filter(|o| sel.contains(&o.id) && matches!(o.kind, ObjKind::Symbol { .. })).count();
        self.status = format!(
            "PDF als Vektorgrafik importiert: {n} Objekte{}{}, Ebenen je Farbe „PDF #…“.",
            if opt.group { " (gruppiert – „Gruppe aufheben“ zum Bearbeiten einzelner Elemente)" } else { "" },
            if zeichen > 0 { format!(", {zeichen} Sicherheitszeichen erkannt und in die Legende übernommen") } else { String::new() }
        );
    }

    pub fn insert_image(&mut self, data: ImageData, area: BBox, opacity: f32) {
        let (a, b) = import::fit_rect(area, data.w, data.h);
        let tab = self.tab_mut();
        tab.push_undo();
        let layer = tab.doc.find_layer_by_name("Grundriss");
        let id = tab.doc.alloc_id();
        tab.doc.objects.insert(0, Obj { id, layer, kind: ObjKind::Image { a, b, data, opacity }, style: Style::NONE, print: true, locked: false });
        tab.sel = vec![id];
        self.status = "Grundriss eingefügt. Größe über die Eckpunkte anpassen, Maßstab über „Maßstab durch Entfernung“ festlegen.".into();
    }

    // ---------- Bearbeiten ----------

    pub fn reset_tool(&mut self) {
        self.tool = Tool::Select;
        self.draft.clear();
        self.drag = None;
        self.measure = None;
    }

    pub fn delete_selection(&mut self) {
        let tab = self.tab_mut();
        if tab.sel.is_empty() {
            return;
        }
        tab.push_undo();
        let sel = std::mem::take(&mut tab.sel);
        tab.doc.objects.retain(|o| !sel.contains(&o.id) || o.locked);
    }

    pub fn copy(&mut self) {
        self.clipboard = self.tab().selected_objs().into_iter().cloned().collect();
        self.status = format!("{} Element(e) kopiert.", self.clipboard.len());
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }

    pub fn paste(&mut self, offset: Pos) {
        if self.clipboard.is_empty() {
            return;
        }
        let clip = self.clipboard.clone();
        let tab = self.tab_mut();
        tab.push_undo();
        let mut new_ids = Vec::new();
        for mut o in clip {
            o.id = tab.doc.alloc_id();
            ops::translate(&mut o.kind, offset);
            new_ids.push(o.id);
            tab.doc.objects.push(o);
        }
        tab.sel = new_ids;
        ops::sync_legend(&mut tab.doc);
    }

    pub fn duplicate(&mut self) {
        self.copy();
        self.paste(Pos::new(5.0, 5.0));
    }

    pub fn multicopy(&mut self, n: u32, dx: f32, dy: f32) {
        let objs: Vec<Obj> = self.tab().selected_objs().into_iter().cloned().collect();
        if objs.is_empty() {
            return;
        }
        let tab = self.tab_mut();
        tab.push_undo();
        for i in 1..=n {
            for o in &objs {
                let mut c = o.clone();
                c.id = tab.doc.alloc_id();
                ops::translate(&mut c.kind, Pos::new(dx * i as f32, dy * i as f32));
                tab.doc.objects.push(c);
            }
        }
    }

    /// Versatz (Offset): parallele Kopie von Linien, Linienzügen, Wänden, Flächen, Kreisen und Rechtecken
    /// im Abstand `d` mm (positiv = links bzw. außen).
    pub fn offset_selection(&mut self, d: f32) -> usize {
        use crate::geom::offset_polyline;
        let objs: Vec<Obj> = self.tab().selected_objs().into_iter().cloned().collect();
        let mut new = Vec::new();
        for o in objs {
            let kind = match &o.kind {
                ObjKind::Line { a, b } => {
                    let p = offset_polyline(&[*a, *b], d, false);
                    Some(ObjKind::Line { a: p[0], b: p[1] })
                }
                ObjKind::Polyline { pts, closed } => Some(ObjKind::Polyline { pts: offset_polyline(pts, d, *closed), closed: *closed }),
                ObjKind::Wall { pts, thickness, closed } => Some(ObjKind::Wall { pts: offset_polyline(pts, d, *closed), thickness: *thickness, closed: *closed }),
                ObjKind::Room { pts, name, show_area, size } => Some(ObjKind::Room { pts: offset_polyline(pts, d, true), name: name.clone(), show_area: *show_area, size: *size }),
                ObjKind::Area { pts, preset } => Some(ObjKind::Area { pts: offset_polyline(pts, d, true), preset: preset.clone() }),
                ObjKind::Spline { pts } => Some(ObjKind::Spline { pts: offset_polyline(pts, d, false) }),
                ObjKind::Circle { c, r } => Some(ObjKind::Circle { c: *c, r: (r + d).max(0.1) }),
                ObjKind::Rect { a, b, radius } => {
                    let bb = crate::geom::BBox::from_two(*a, *b).expand(d);
                    (bb.w() > 0.1 && bb.h() > 0.1).then(|| ObjKind::Rect { a: bb.min, b: bb.max, radius: *radius })
                }
                _ => None,
            };
            if let Some(kind) = kind {
                new.push(Obj { kind, ..o });
            }
        }
        if new.is_empty() {
            return 0;
        }
        let tab = self.tab_mut();
        tab.push_undo();
        let mut ids = Vec::new();
        for mut o in new {
            o.id = tab.doc.alloc_id();
            ids.push(o.id);
            tab.doc.objects.push(o);
        }
        tab.sel = ids.clone();
        ids.len()
    }

    pub fn transform_sel(&mut self, f: impl Fn(&mut ObjKind, Pos)) {
        let tab = self.tab_mut();
        if tab.sel.is_empty() {
            return;
        }
        let c = tab.sel_bounds().center();
        tab.push_undo();
        let sel = tab.sel.clone();
        for o in tab.doc.objects.iter_mut().filter(|o| sel.contains(&o.id) && !o.locked) {
            f(&mut o.kind, c);
        }
    }

    pub fn group(&mut self) {
        let tab = self.tab_mut();
        if tab.sel.len() < 2 {
            return;
        }
        tab.push_undo();
        let sel = tab.sel.clone();
        let first = tab.doc.objects.iter().position(|o| sel.contains(&o.id)).unwrap_or(0);
        let children: Vec<Obj> = tab.doc.objects.iter().filter(|o| sel.contains(&o.id)).cloned().collect();
        let layer = children[0].layer;
        tab.doc.objects.retain(|o| !sel.contains(&o.id));
        let id = tab.doc.alloc_id();
        tab.doc.objects.insert(first.min(tab.doc.objects.len()), Obj { id, layer, kind: ObjKind::Group { children }, style: Style::default(), print: true, locked: false });
        tab.sel = vec![id];
    }

    pub fn symbole_erkennen(&mut self) {
        let tab = self.tab_mut();
        tab.push_undo();
        let ids: Vec<u64> = if tab.sel.is_empty() { tab.doc.objects.iter().map(|o| o.id).collect() } else { tab.sel.clone() };
        let vorher: std::collections::HashSet<u64> = tab.doc.objects.iter().map(|o| o.id).collect();
        let n = crate::ops::symbole_zusammenfassen(&mut tab.doc, &ids, crate::pdfvec::SYMBOL_MAX_MM);
        // neue Gruppen und gewählte Objekte mit der Symbolbibliothek vergleichen
        let pruefen: Vec<u64> = tab.doc.objects.iter().map(|o| o.id).filter(|id| !vorher.contains(id) || ids.contains(id)).collect();
        let m = crate::erkennung::normzeichen_einsetzen(&mut tab.doc, Some(&pruefen));
        tab.sel.clear();
        tab.dirty = true;
        self.status = format!("{n} Symbole zu Gruppen zusammengefasst, {m} davon als Normzeichen erkannt und ersetzt (Legende aktualisiert).");
    }

    pub fn ungroup(&mut self) {
        let tab = self.tab_mut();
        let sel = tab.sel.clone();
        if !tab.doc.objects.iter().any(|o| sel.contains(&o.id) && matches!(o.kind, ObjKind::Group { .. })) {
            return;
        }
        tab.push_undo();
        let mut out = Vec::new();
        let mut new_sel = Vec::new();
        for o in std::mem::take(&mut tab.doc.objects) {
            if sel.contains(&o.id) {
                if let ObjKind::Group { children } = o.kind {
                    for mut c in children {
                        c.id = tab.doc.next_id;
                        tab.doc.next_id += 1;
                        new_sel.push(c.id);
                        out.push(c);
                    }
                    continue;
                }
            }
            out.push(o);
        }
        tab.doc.objects = out;
        tab.sel = new_sel;
    }

    pub fn order(&mut self, front: bool, one_step: bool) {
        let tab = self.tab_mut();
        if tab.sel.is_empty() {
            return;
        }
        tab.push_undo();
        let sel = tab.sel.clone();
        if one_step {
            let objs = &mut tab.doc.objects;
            if front {
                for i in (0..objs.len().saturating_sub(1)).rev() {
                    if sel.contains(&objs[i].id) && !sel.contains(&objs[i + 1].id) {
                        objs.swap(i, i + 1);
                    }
                }
            } else {
                for i in 1..objs.len() {
                    if sel.contains(&objs[i].id) && !sel.contains(&objs[i - 1].id) {
                        objs.swap(i, i - 1);
                    }
                }
            }
        } else {
            let (mut a, b): (Vec<Obj>, Vec<Obj>) = std::mem::take(&mut tab.doc.objects).into_iter().partition(|o| sel.contains(&o.id));
            tab.doc.objects = if front {
                let mut v = b;
                v.append(&mut a);
                v
            } else {
                a.extend(b);
                a
            };
        }
    }

    pub fn align(&mut self, mode: u8) {
        let tab = self.tab_mut();
        if tab.sel.len() < 2 {
            return;
        }
        let all = tab.sel_bounds();
        tab.push_undo();
        let doc = &mut tab.doc;
        let sel = tab.sel.clone();
        let scale = doc.scale_n;
        for o in doc.objects.iter_mut().filter(|o| sel.contains(&o.id)) {
            let tmp = Doc { scale_n: scale, ..Doc::default() };
            let b = ops::bounds_doc(&tmp, &o.kind);
            let d = match mode {
                0 => Pos::new(all.min.x - b.min.x, 0.0),
                1 => Pos::new(all.center().x - b.center().x, 0.0),
                2 => Pos::new(all.max.x - b.max.x, 0.0),
                3 => Pos::new(0.0, all.min.y - b.min.y),
                4 => Pos::new(0.0, all.center().y - b.center().y),
                _ => Pos::new(0.0, all.max.y - b.max.y),
            };
            ops::translate(&mut o.kind, d);
        }
    }

    pub fn select_all(&mut self) {
        let tab = self.tab_mut();
        let doc = &tab.doc;
        tab.sel = doc.objects.iter().filter(|o| doc.layer_visible(o.layer) && !doc.layer_locked(o.layer)).map(|o| o.id).collect();
    }

    pub fn invert_selection(&mut self) {
        let tab = self.tab_mut();
        let doc = &tab.doc;
        let sel = tab.sel.clone();
        tab.sel = doc.objects.iter().filter(|o| doc.layer_visible(o.layer) && !sel.contains(&o.id)).map(|o| o.id).collect();
    }

    /// Alle Objekte gleicher Art (bzw. gleiches Symbol / gleiche Fläche) wie die Auswahl.
    pub fn select_similar(&mut self) {
        let tab = self.tab_mut();
        let keys: Vec<String> = tab.selected_objs().iter().map(|o| similar_key(&o.kind)).collect();
        tab.sel = tab.doc.objects.iter().filter(|o| keys.contains(&similar_key(&o.kind))).map(|o| o.id).collect();
    }

    pub fn cleanup(&mut self) {
        let tab = self.tab_mut();
        tab.push_undo();
        let before = tab.doc.objects.len();
        tab.doc.objects.retain(|o| match &o.kind {
            ObjKind::Text { text, .. } => !text.trim().is_empty(),
            ObjKind::Polyline { pts, .. } | ObjKind::Wall { pts, .. } | ObjKind::Route { pts, .. } | ObjKind::Spline { pts } | ObjKind::Fence { pts } | ObjKind::Rail { pts } => pts.len() >= 2 && polyline_len(pts) > 0.05,
            ObjKind::Area { pts, .. } | ObjKind::Room { pts, .. } => polygon_area(pts) > 0.01,
            ObjKind::Line { a, b } => a.dist(*b) > 0.05,
            ObjKind::Group { children } => !children.is_empty(),
            _ => true,
        });
        let used: Vec<String> = ops::used_legend_keys(&tab.doc).into_iter().map(|(k, _)| k).collect();
        tab.doc.legend.entries.retain(|e| used.contains(&e.key));
        let used_layers: Vec<u32> = tab.doc.objects.iter().map(|o| o.layer).collect();
        let active = tab.doc.active_layer;
        tab.doc.layers.retain(|l| used_layers.contains(&l.id) || l.id == active || l.id <= 5);
        let removed = before - tab.doc.objects.len();
        self.status = format!("Plan bereinigt: {removed} leere Element(e) entfernt, Legende abgeglichen.");
    }

    pub fn replace_symbols(&mut self, from: &str, to: &str) -> usize {
        let tab = self.tab_mut();
        tab.push_undo();
        let mut n = 0;
        fn walk(objs: &mut [Obj], from: &str, to: &str, n: &mut usize) {
            for o in objs {
                match &mut o.kind {
                    ObjKind::Symbol { id, .. } if id == from => {
                        *id = to.to_string();
                        *n += 1;
                    }
                    ObjKind::Group { children } => walk(children, from, to, n),
                    _ => {}
                }
            }
        }
        walk(&mut tab.doc.objects, from, to, &mut n);
        tab.doc.legend.entries.retain(|e| e.key != format!("sym:{from}"));
        ops::sync_legend(&mut tab.doc);
        n
    }

    /// Gesamten Planinhalt drehen (lagerichtige Darstellung). Zeichen und Texte bleiben aufrecht,
    /// Richtungspfeile und Nordpfeile drehen mit.
    pub fn rotate_content(&mut self, deg: f32) {
        let c = templates::layout(self.doc()).plan.center();
        let tab = self.tab_mut();
        tab.push_undo();
        fn turns_with(id: &str) -> bool {
            id.contains("PFEIL") || id.contains("ZUFAHRT") || id.starts_with('N') && id.len() <= 2 || id == "STANDORT_PFEIL"
        }
        fn walk(objs: &mut [Obj], c: Pos, deg: f32) {
            for o in objs {
                match &mut o.kind {
                    ObjKind::Symbol { id, pos, rot, .. } => {
                        *pos = pos.rotate_around(c, deg);
                        if turns_with(id) {
                            *rot = (*rot + deg).rem_euclid(360.0);
                        }
                    }
                    ObjKind::Text { pos, .. } | ObjKind::Note { pos, .. } => *pos = pos.rotate_around(c, deg),
                    ObjKind::Group { children } => walk(children, c, deg),
                    k => ops::rotate(k, c, deg),
                }
            }
        }
        walk(&mut tab.doc.objects, c, deg);
        self.status = format!("Planinhalt um {deg}° gedreht. Symbole und Texte bleiben lesbar.");
    }

    pub fn duplicate_plan(&mut self) {
        let mut doc = self.doc().clone();
        doc.info.plannummer = if doc.info.plannummer.is_empty() { String::new() } else { format!("{} (Kopie)", doc.info.plannummer) };
        self.tabs.push(DocTab::new(doc, None));
        self.active = self.tabs.len() - 1;
        self.tab_mut().dirty = true;
        self.status = "Plan dupliziert – Standort-Symbol für den neuen Aushangort versetzen und speichern.".into();
    }

    pub fn nudge(&mut self, d: Pos) {
        let tab = self.tab_mut();
        if tab.sel.is_empty() {
            return;
        }
        let now = tab.sel[0];
        let before = tab.doc.clone();
        let sel = tab.sel.clone();
        for o in tab.doc.objects.iter_mut().filter(|o| sel.contains(&o.id) && !o.locked) {
            ops::translate(&mut o.kind, d);
        }
        tab.push_undo_coalesced(before, now, f64::MAX / 2.0);
    }
}

fn similar_key(k: &ObjKind) -> String {
    match k {
        ObjKind::Symbol { id, .. } => format!("sym:{id}"),
        ObjKind::Area { preset, .. } => format!("area:{preset}"),
        other => ops::kind_name(other).to_string(),
    }
}

pub fn sanitize(objekt: &str, geschoss: &str) -> String {
    let s = format!("{objekt} {geschoss}");
    let s: String = s.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    let s = s.trim().replace(' ', "_");
    if s.is_empty() { "Fluchtplan".into() } else { s }
}

/// SVG-Datei des Nutzers in ein Symbol (0..100) einpassen.
pub fn custom_symbol_from_svg(src: &str) -> Result<String, String> {
    let tree = resvg::usvg::Tree::from_str(src, export::usvg_options()).map_err(|e| format!("SVG nicht lesbar: {e}"))?;
    let size = tree.size();
    let (w, h) = (size.width(), size.height());
    let f = 100.0 / w.max(h);
    let ox = (100.0 - w * f) / 2.0;
    let oy = (100.0 - h * f) / 2.0;
    // Inhalt ohne äußeres <svg> übernehmen
    let start = src.find("<svg").ok_or("Kein <svg>-Element gefunden")?;
    let open_end = src[start..].find('>').ok_or("Ungültiges SVG")? + start + 1;
    let close = src.rfind("</svg>").ok_or("Ungültiges SVG")?;
    let inner = &src[open_end..close];
    Ok(format!("<g transform='translate({ox:.3} {oy:.3}) scale({f:.5})'>{inner}</g>"))
}
