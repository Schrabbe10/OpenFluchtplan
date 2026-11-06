//! Datenmodell eines Plans (Dateiformat `.fplan`, JSON). Koordinaten in mm auf dem Blatt.

use serde::{Deserialize, Serialize};

pub use crate::geom::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color(r, g, b, 255)
    }
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    pub fn alpha(self) -> f32 {
        self.3 as f32 / 255.0
    }
}

/// Sicherheitsfarben (DIN EN ISO 7010), nicht alle werden im Code direkt verwendet.
#[allow(dead_code)]
pub mod farbe {
    use super::Color;
    pub const SCHWARZ: Color = Color::rgb(0, 0, 0);
    pub const WEISS: Color = Color::rgb(255, 255, 255);
    /// Sicherheitsgrün (Rettungszeichen)
    pub const GRUEN: Color = Color::rgb(0x23, 0x7f, 0x52);
    /// Sicherheitsrot (Brandschutzzeichen)
    pub const ROT: Color = Color::rgb(0xb0, 0x1e, 0x23);
    /// Sicherheitsblau (Gebotszeichen, Standort)
    pub const BLAU: Color = Color::rgb(0x1f, 0x4e, 0x9c);
    /// Sicherheitsgelb (Warnzeichen)
    pub const GELB: Color = Color::rgb(0xf9, 0xc4, 0x00);
    pub const FLUCHT_HELL: Color = Color::rgb(0xb5, 0xd8, 0xa6);
    pub const GRAU: Color = Color::rgb(0x99, 0x99, 0x99);
    pub const HELLGRAU: Color = Color::rgb(0xdd, 0xdd, 0xdd);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dash {
    Durchgehend,
    Gestrichelt,
    Gepunktet,
    StrichPunkt,
}

impl Dash {
    pub const ALL: [Dash; 4] = [Dash::Durchgehend, Dash::Gestrichelt, Dash::Gepunktet, Dash::StrichPunkt];
    pub fn name(self) -> &'static str {
        match self {
            Dash::Durchgehend => "Durchgehend",
            Dash::Gestrichelt => "Gestrichelt",
            Dash::Gepunktet => "Gepunktet",
            Dash::StrichPunkt => "Strich-Punkt",
        }
    }
    /// Strichmuster als Vielfache der Linienstärke (mindestens 0,5 mm).
    pub fn pattern(self, width: f32) -> Option<Vec<f32>> {
        let u = width.max(0.5);
        match self {
            Dash::Durchgehend => None,
            Dash::Gestrichelt => Some(vec![4.0 * u, 2.0 * u]),
            Dash::Gepunktet => Some(vec![u, 1.5 * u]),
            Dash::StrichPunkt => Some(vec![5.0 * u, 1.5 * u, u, 1.5 * u]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrokeStyle {
    pub color: Color,
    pub width: f32,
    pub dash: Dash,
}

impl StrokeStyle {
    pub const fn new(color: Color, width: f32) -> Self {
        Self {
            color,
            width,
            dash: Dash::Durchgehend,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hatch {
    pub bg: Option<Color>,
    pub color: Color,
    pub angle: f32,
    pub spacing: f32,
    pub width: f32,
    pub cross: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fill {
    Keine,
    Farbe(Color),
    Schraffur(Hatch),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub stroke: Option<StrokeStyle>,
    pub fill: Fill,
}

impl Style {
    pub const fn line(color: Color, width: f32) -> Self {
        Self {
            stroke: Some(StrokeStyle::new(color, width)),
            fill: Fill::Keine,
        }
    }
    pub const fn filled(fill: Color) -> Self {
        Self {
            stroke: None,
            fill: Fill::Farbe(fill),
        }
    }
    pub const NONE: Style = Style {
        stroke: None,
        fill: Fill::Keine,
    };
}

impl Default for Style {
    fn default() -> Self {
        Style::line(farbe::SCHWARZ, 0.35)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TAlign {
    Links,
    Mitte,
    Rechts,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    pub mime: String,
    pub b64: String,
    pub w: u32,
    pub h: u32,
}

/// Art eines Planbausteins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockArt {
    Regeln,
    Schriftfeld,
    Notruf,
    Sammelstelle,
}

impl BlockArt {
    pub fn name(self) -> &'static str {
        match self {
            BlockArt::Regeln => "Verhaltensregeln",
            BlockArt::Schriftfeld => "Schriftfeld",
            BlockArt::Notruf => "Notruf",
            BlockArt::Sammelstelle => "Sammelstelle",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjKind {
    Line { a: Pos, b: Pos },
    Polyline { pts: Vec<Pos>, closed: bool },
    Rect { a: Pos, b: Pos, radius: f32 },
    Circle { c: Pos, r: f32 },
    Ellipse { c: Pos, rx: f32, ry: f32 },
    Arc { c: Pos, r: f32, start: f32, end: f32 },
    NGon { c: Pos, r: f32, n: u32, rot: f32 },
    Spline { pts: Vec<Pos> },
    /// Schlauch: Doppellinie entlang eines Pfades
    Hose { pts: Vec<Pos>, width: f32 },
    Text { pos: Pos, text: String, size: f32, bold: bool, italic: bool, align: TAlign, rot: f32 },
    Image { a: Pos, b: Pos, data: ImageData, opacity: f32 },
    Dimension { a: Pos, b: Pos, offset: f32 },
    Table { pos: Pos, cols: Vec<f32>, rows: Vec<f32>, cells: Vec<Vec<String>>, size: f32 },
    Wall { pts: Vec<Pos>, thickness: f32, closed: bool },
    Room { pts: Vec<Pos>, name: String, show_area: bool, size: f32 },
    Window { a: Pos, b: Pos, thickness: f32 },
    Door { pos: Pos, width: f32, rot: f32, mirror: bool, double: bool },
    Stairs { a: Pos, b: Pos, width: f32, steps: u32 },
    Note { pos: Pos, text: String },
    /// Fläche nach Vorlage (Fluchtbereich, befahrbare Fläche, Feuerwiderstand …)
    Area { pts: Vec<Pos>, preset: String },
    /// Fluchtweg mit Richtungspfeilen
    Route { pts: Vec<Pos>, width: f32, curved: bool, arrows: bool },
    Symbol { id: String, pos: Pos, size: f32, rot: f32 },
    Legend { a: Pos, b: Pos, cols: u32, size: f32 },
    /// Planbaustein (Verhaltensregeln, Schriftfeld …), aus den Plandaten gefüllt
    Block { a: Pos, b: Pos, art: BlockArt },
    Grid { a: Pos, b: Pos, nx: u32, ny: u32 },
    /// Pfad aus mehreren Teilzügen (z. B. aus PDF-Import); Füllung nach Gerade-Ungerade-Regel
    Path { subpaths: Vec<Vec<Pos>>, closed: bool },
    /// Geschoss-Schnitt (DIN 14095): Geschosse von oben nach unten, das dargestellte ist markiert
    FloorSection { a: Pos, b: Pos, floors: Vec<String>, current: usize },
    RevisionCloud { pts: Vec<Pos> },
    Fence { pts: Vec<Pos> },
    Rail { pts: Vec<Pos> },
    ScaleBar { pos: Pos, meters: f32 },
    QrCode { pos: Pos, size: f32, text: String },
    /// Kurve einer Feuerwehrzufahrt (DIN 14090)
    FireCurve { c: Pos, r_out: f32, width: f32, start: f32, end: f32 },
    Group { children: Vec<Obj> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obj {
    pub id: u64,
    pub layer: u32,
    pub kind: ObjKind,
    pub style: Style,
    #[serde(default = "ja")]
    pub print: bool,
    #[serde(default)]
    pub locked: bool,
}

fn ja() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: u32,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// Ebenenfarbe: überschreibt Linien- und Textfarbe der Objekte auf dieser Ebene
    #[serde(default)]
    pub color: Option<Color>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaperFormat {
    A4,
    A3,
    A2,
    A1,
    A0,
    Letter,
    Cm40x30,
    Cm50x40,
    Cm60x40,
    Cm60x60,
    Cm80x60,
    Cm80x80,
    Eigenes { w: f32, h: f32 },
}

impl PaperFormat {
    pub const LISTE: [PaperFormat; 12] = [
        PaperFormat::A4,
        PaperFormat::A3,
        PaperFormat::A2,
        PaperFormat::A1,
        PaperFormat::A0,
        PaperFormat::Letter,
        PaperFormat::Cm40x30,
        PaperFormat::Cm50x40,
        PaperFormat::Cm60x40,
        PaperFormat::Cm60x60,
        PaperFormat::Cm80x60,
        PaperFormat::Cm80x80,
    ];
    /// Breite x Höhe im Hochformat (mm)
    pub fn size(self) -> (f32, f32) {
        match self {
            PaperFormat::A4 => (210.0, 297.0),
            PaperFormat::A3 => (297.0, 420.0),
            PaperFormat::A2 => (420.0, 594.0),
            PaperFormat::A1 => (594.0, 841.0),
            PaperFormat::A0 => (841.0, 1189.0),
            PaperFormat::Letter => (215.9, 279.4),
            PaperFormat::Cm40x30 => (300.0, 400.0),
            PaperFormat::Cm50x40 => (400.0, 500.0),
            PaperFormat::Cm60x40 => (400.0, 600.0),
            PaperFormat::Cm60x60 => (600.0, 600.0),
            PaperFormat::Cm80x60 => (600.0, 800.0),
            PaperFormat::Cm80x80 => (800.0, 800.0),
            PaperFormat::Eigenes { w, h } => (w.min(h), w.max(h)),
        }
    }
    pub fn name(self) -> String {
        match self {
            PaperFormat::A4 => "DIN A4 (nur Einzelräume)".into(),
            PaperFormat::A3 => "DIN A3 (empfohlen)".into(),
            PaperFormat::A2 => "DIN A2".into(),
            PaperFormat::A1 => "DIN A1".into(),
            PaperFormat::A0 => "DIN A0".into(),
            PaperFormat::Letter => "Letter".into(),
            PaperFormat::Cm40x30 => "40 x 30 cm".into(),
            PaperFormat::Cm50x40 => "50 x 40 cm".into(),
            PaperFormat::Cm60x40 => "60 x 40 cm".into(),
            PaperFormat::Cm60x60 => "60 x 60 cm".into(),
            PaperFormat::Cm80x60 => "80 x 60 cm".into(),
            PaperFormat::Cm80x80 => "80 x 80 cm".into(),
            PaperFormat::Eigenes { w, h } => format!("Eigenes Format {w:.0} x {h:.0} mm"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub format: PaperFormat,
    pub landscape: bool,
}

impl Page {
    pub fn size(&self) -> (f32, f32) {
        let (w, h) = self.format.size();
        if self.landscape { (h, w) } else { (w, h) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameKind {
    Ohne,
    /// Flucht- und Rettungsplan, vollständig (Kopf, Regeln, Legende, Plankopf)
    FrVollstaendig,
    /// wie vollständig, Titel zweizeilig (z. B. zweisprachig)
    FrZweizeilig,
    /// nur Kopfzeile
    FrNurOben,
    BrandschutzVollstaendig,
    BrandschutzNurOben,
    /// Feuerwehrplan nach DIN 14095
    Feuerwehrplan,
    /// Feuerwehr-Laufkarte (DIN 14675)
    Laufkarte,
}

impl FrameKind {
    pub const ALL: [FrameKind; 8] = [
        FrameKind::Ohne,
        FrameKind::FrVollstaendig,
        FrameKind::FrZweizeilig,
        FrameKind::FrNurOben,
        FrameKind::BrandschutzVollstaendig,
        FrameKind::BrandschutzNurOben,
        FrameKind::Feuerwehrplan,
        FrameKind::Laufkarte,
    ];
    pub fn name(self) -> &'static str {
        match self {
            FrameKind::Ohne => "Ohne Rahmen",
            FrameKind::FrVollstaendig => "Flucht- und Rettungsplan – vollständig",
            FrameKind::FrZweizeilig => "Flucht- und Rettungsplan – zweizeilig",
            FrameKind::FrNurOben => "Flucht- und Rettungsplan – nur oben",
            FrameKind::BrandschutzVollstaendig => "Brandschutzplan – vollständig",
            FrameKind::BrandschutzNurOben => "Brandschutzplan – nur oben",
            FrameKind::Feuerwehrplan => "Feuerwehrplan (DIN 14095)",
            FrameKind::Laufkarte => "Feuerwehr-Laufkarte (DIN 14675)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FrameCfg {
    pub title: String,
    pub title2: String,
    pub color: Color,
    pub show_rules: bool,
    pub show_notruf: bool,
    pub show_sammelstelle: bool,
    pub logo: Option<ImageData>,
}

impl Default for FrameCfg {
    fn default() -> Self {
        Self {
            title: "FLUCHT- UND RETTUNGSPLAN".into(),
            title2: "ESCAPE AND RESCUE PLAN".into(),
            color: farbe::GRUEN,
            show_rules: true,
            show_notruf: true,
            show_sammelstelle: true,
            logo: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanInfo {
    pub objekt: String,
    pub anschrift: String,
    /// Gebäude / Bauteil (z. B. „Halle 8“)
    pub gebaeude: String,
    pub geschoss: String,
    pub auftraggeber: String,
    pub ersteller: String,
    pub plannummer: String,
    pub planart: String,
    pub stand: String,
    pub revision: String,
    pub sonstiges: String,
    pub notruf: String,
    pub sammelstelle: String,
    // Laufkarte
    pub meldergruppe: String,
    pub melder: String,
    pub meldertyp: String,
    pub raum: String,
    pub bmz_standort: String,
}

impl Default for PlanInfo {
    fn default() -> Self {
        Self {
            objekt: "Name des Gebäudes".into(),
            anschrift: String::new(),
            gebaeude: String::new(),
            geschoss: "Erdgeschoss".into(),
            auftraggeber: String::new(),
            ersteller: String::new(),
            plannummer: String::new(),
            planart: String::new(),
            stand: String::new(),
            revision: String::new(),
            sonstiges: String::new(),
            notruf: "112".into(),
            sammelstelle: String::new(),
            meldergruppe: String::new(),
            melder: String::new(),
            meldertyp: String::new(),
            raum: String::new(),
            bmz_standort: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    De,
    En,
    Fr,
    It,
    Es,
    Pl,
    Tr,
    Nl,
}

impl Lang {
    pub const ALL: [Lang; 8] = [Lang::De, Lang::En, Lang::Fr, Lang::It, Lang::Es, Lang::Pl, Lang::Tr, Lang::Nl];
    pub fn name(self) -> &'static str {
        match self {
            Lang::De => "Deutsch",
            Lang::En => "Englisch",
            Lang::Fr => "Französisch",
            Lang::It => "Italienisch",
            Lang::Es => "Spanisch",
            Lang::Pl => "Polnisch",
            Lang::Tr => "Türkisch",
            Lang::Nl => "Niederländisch",
        }
    }
    pub fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendEntry {
    /// z. B. `sym:E001`, `area:fluchtbereich`, `route`
    pub key: String,
    pub label: String,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LegendCfg {
    pub title: String,
    pub entries: Vec<LegendEntry>,
    pub langs: Vec<Lang>,
}

impl Default for LegendCfg {
    fn default() -> Self {
        Self {
            title: "Legende".into(),
            entries: Vec::new(),
            langs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomSymbol {
    pub id: String,
    pub name: String,
    /// SVG-Inhalt im Koordinatensystem 0..100
    pub svg: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Guide {
    pub vertical: bool,
    pub at: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub version: u32,
    pub info: PlanInfo,
    pub page: Page,
    pub frame: FrameKind,
    pub frame_cfg: FrameCfg,
    /// Maßstab 1:N
    pub scale_n: f32,
    pub layers: Vec<Layer>,
    pub active_layer: u32,
    pub objects: Vec<Obj>,
    pub legend: LegendCfg,
    #[serde(default)]
    pub custom_symbols: Vec<CustomSymbol>,
    #[serde(default)]
    pub guides: Vec<Guide>,
    pub next_id: u64,
}

impl Default for Doc {
    fn default() -> Self {
        Self {
            version: 2,
            info: PlanInfo::default(),
            page: Page {
                format: PaperFormat::A3,
                landscape: true,
            },
            frame: FrameKind::FrVollstaendig,
            frame_cfg: FrameCfg::default(),
            scale_n: 100.0,
            layers: vec![
                Layer { id: 0, name: "Grundriss".into(), visible: true, locked: false, color: None },
                Layer { id: 1, name: "Wände".into(), visible: true, locked: false, color: None },
                Layer { id: 2, name: "Flächen".into(), visible: true, locked: false, color: None },
                Layer { id: 3, name: "Fluchtwege".into(), visible: true, locked: false, color: None },
                Layer { id: 4, name: "Symbole".into(), visible: true, locked: false, color: None },
                Layer { id: 5, name: "Beschriftung".into(), visible: true, locked: false, color: None },
            ],
            active_layer: 4,
            objects: Vec::new(),
            legend: LegendCfg::default(),
            custom_symbols: Vec::new(),
            guides: Vec::new(),
            next_id: 1,
        }
    }
}

impl Doc {
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add(&mut self, kind: ObjKind, style: Style, layer: u32) -> u64 {
        let id = self.alloc_id();
        self.objects.push(Obj {
            id,
            layer,
            kind,
            style,
            print: true,
            locked: false,
        });
        id
    }

    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    pub fn layer(&self, id: u32) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn layer_visible(&self, id: u32) -> bool {
        self.layer(id).map(|l| l.visible).unwrap_or(true)
    }

    pub fn layer_locked(&self, id: u32) -> bool {
        self.layer(id).map(|l| l.locked).unwrap_or(false)
    }

    /// Reale Länge in m einer Strecke auf dem Blatt (mm).
    pub fn real_m(&self, mm: f32) -> f32 {
        mm * self.scale_n / 1000.0
    }

    /// Strecke auf dem Blatt (mm) für eine reale Länge in m.
    pub fn sheet_mm(&self, m: f32) -> f32 {
        m * 1000.0 / self.scale_n
    }

    pub fn find_layer_by_name(&mut self, name: &str) -> u32 {
        if let Some(l) = self.layers.iter().find(|l| l.name == name) {
            return l.id;
        }
        let id = self.layers.iter().map(|l| l.id).max().unwrap_or(0) + 1;
        self.layers.push(Layer {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            color: None,
        });
        id
    }

    /// Objekte bekommen nach dem Laden/Einfügen eindeutige IDs.
    pub fn renumber(&mut self) {
        let mut next = 1;
        fn walk(objs: &mut [Obj], next: &mut u64) {
            for o in objs {
                o.id = *next;
                *next += 1;
                if let ObjKind::Group { children } = &mut o.kind {
                    walk(children, next);
                }
            }
        }
        walk(&mut self.objects, &mut next);
        self.next_id = next;
    }
}
