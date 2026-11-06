//! Symbole für das Menüband: schlichte Strichzeichnungen im Stil der Windows-Oberfläche
//! (Raster 24 × 24). Eigene Zeichnungen, als SVG hinterlegt und bei Bedarf gerastert.

use std::collections::HashMap;

use eframe::egui::{self, TextureHandle};

/// Strichfarbe der Zeichnungen
const D: &str = "#424242";
/// Akzentfarben
const A: &str = "#0f6cbd";
const G: &str = "#107c41";
const R: &str = "#c42b1c";
const Y: &str = "#e8a400";

fn body(name: &str) -> Option<String> {
    let s = match name {
        // ---------- Datei ----------
        "neu" => format!("<path d='M6 3h8l4 4v14H6z'/><path d='M14 3v4h4'/><path d='M18 14v6M15 17h6' stroke='{A}' stroke-width='2'/>"),
        "oeffnen" => format!("<path d='M3 6V19h15l3-8H7l-4 8'/><path d='M3 6h6l2 2h7v3'/>"),
        "speichern" => format!("<path d='M4 4h13l3 3v13H4z'/><path d='M8 4v5h8V4'/><rect x='7' y='13' width='10' height='7' fill='{A}' stroke='{A}'/>"),
        "speichern_unter" => format!("<path d='M4 4h13l3 3v6M4 4v16h8'/><path d='M8 4v5h8V4'/><path d='M14 21l1-3 5-5 2 2-5 5z' stroke='{A}'/>"),
        "schliessen" => format!("<path d='M6 3h8l4 4v14H6z'/><path d='M9.5 11.5l5 5M14.5 11.5l-5 5' stroke='{R}' stroke-width='1.8'/>"),
        "zuletzt" => format!("<circle cx='12' cy='12' r='8.5'/><path d='M12 7v5l3.5 2' stroke='{A}' stroke-width='1.8'/>"),
        "muster" => format!("<rect x='3' y='4' width='18' height='16' rx='1.5'/><path d='M3 8h18'/><rect x='6' y='11' width='5' height='6' fill='{G}' stroke='none'/><path d='M13 12h5M13 15h5'/>"),
        "willkommen" => format!("<path d='M4 11l8-7 8 7v9H4z'/><path d='M10 20v-6h4v6' stroke='{A}'/>"),
        "drucken" => format!("<path d='M7 8V3h10v5'/><rect x='3' y='8' width='18' height='8' rx='1.5'/><path d='M7 13h10v8H7z' fill='#fff'/><path d='M9.5 16h5M9.5 18.5h5'/><circle cx='17.5' cy='10.8' r='.6' fill='{G}'/>"),
        "pdf" => format!("<path d='M6 3h8l4 4v14H6z'/><path d='M14 3v4h4'/><rect x='3' y='11' width='13' height='7' rx='1' fill='{R}' stroke='{R}'/><text x='9.5' y='16.4' font-family='Arial' font-weight='bold' font-size='5.4' text-anchor='middle' fill='#fff' stroke='none'>PDF</text>"),
        "beenden" => format!("<path d='M12 3v8' stroke='{R}' stroke-width='2'/><path d='M7 6.5a7.5 7.5 0 1 0 10 0'/>"),
        "importieren" => format!("<path d='M4 15v5h16v-5'/><path d='M12 3v12M7.5 10.5L12 15l4.5-4.5' stroke='{A}' stroke-width='1.8'/>"),
        "exportieren" => format!("<path d='M4 15v5h16v-5'/><path d='M12 15V3M7.5 7.5L12 3l4.5 4.5' stroke='{A}' stroke-width='1.8'/>"),
        "pruefen" => format!("<path d='M6 3h8l4 4v6M6 3v18h6'/><path d='M14 3v4h4'/><path d='M13.5 18l2.5 2.5 5-5' stroke='{G}' stroke-width='2'/>"),

        // ---------- Plan ----------
        "format" => format!("<rect x='5' y='3' width='14' height='18' rx='1'/><path d='M8 7h8M8 10h8M8 13h5'/><path d='M14 21v-4h5' stroke='{A}'/>"),
        "hoch" => format!("<rect x='7' y='3' width='10' height='18' rx='1'/>"),
        "quer" => format!("<rect x='3' y='7' width='18' height='10' rx='1'/>"),
        "rahmen" => format!("<rect x='3' y='3' width='18' height='18' rx='1'/><rect x='3' y='3' width='18' height='4' fill='{G}' stroke='{G}'/><rect x='6' y='10' width='12' height='8'/>"),
        "massstab" => format!("<path d='M2.5 9h19v6h-19z'/><path d='M6 9v3M9.5 9v4M13 9v3M16.5 9v4M20 9v3'/>"),
        "massstab_distanz" => format!("<path d='M4 18L18 4' stroke-dasharray='2 2'/><circle cx='4' cy='18' r='2' fill='{A}' stroke='{A}'/><circle cx='18' cy='4' r='2' fill='{A}' stroke='{A}'/><path d='M10 21h11'/>"),
        "bereinigen" => format!("<path d='M14 3l-6 9'/><path d='M5 13h8l1 8H4z' fill='{Y}' stroke='{D}'/><path d='M17 14l3-1M17 18h3M16 10l2.5-2'/>"),
        "extrahieren" => format!("<rect x='3' y='3' width='18' height='18' rx='1' stroke-dasharray='2 2'/><path d='M7 7h10v10H7zM12 7v6' stroke='{A}' stroke-width='2'/>"),
        "legende" => format!("<rect x='3' y='3' width='18' height='18' rx='1'/><rect x='6' y='6.5' width='3' height='3' fill='{G}' stroke='none'/><rect x='6' y='10.5' width='3' height='3' fill='{R}' stroke='none'/><rect x='6' y='14.5' width='3' height='3' fill='{A}' stroke='none'/><path d='M11 8h7M11 12h7M11 16h7'/>"),
        "drehen_l" => format!("<path d='M4 9a8 8 0 1 1 1 7'/><path d='M4 4v5h5' stroke='{A}' stroke-width='1.8'/>"),
        "drehen_r" => format!("<path d='M20 9a8 8 0 1 0-1 7'/><path d='M20 4v5h-5' stroke='{A}' stroke-width='1.8'/>"),
        "drehen_180" => format!("<path d='M5 12a7 7 0 0 1 14 0'/><path d='M19 12a7 7 0 0 1-14 0' stroke-dasharray='2 2'/><path d='M16 9l3 3 3-3' stroke='{A}' stroke-width='1.8'/>"),
        "plan_duplizieren" => format!("<rect x='8' y='8' width='13' height='13' rx='1'/><path d='M16 4H4v12'/><circle cx='14.5' cy='14.5' r='2' fill='{A}' stroke='{A}'/>"),
        "laufkarte" => format!("<rect x='2.5' y='5' width='19' height='14' rx='1'/><rect x='2.5' y='5' width='19' height='3' fill='{R}' stroke='{R}'/><path d='M6 16l4-4 3 2 5-4' stroke='{R}'/>"),
        "container" => format!("<rect x='3' y='5' width='18' height='14' rx='1'/><path d='M6 9h8M6 12h12M6 15h10'/>"),

        // ---------- Auswahl / Zoom ----------
        "auswahl" => format!("<path d='M6 3l12 9-5.5 1L10 19z' fill='#fff'/>"),
        "verschieben" => format!("<path d='M8 13V5.5a1.5 1.5 0 0 1 3 0V11M11 10V4.5a1.5 1.5 0 0 1 3 0V11M14 10.5V6a1.5 1.5 0 0 1 3 0v8c0 4-2.5 7-6 7-2.5 0-4-1.5-5.5-4L3.5 13a1.4 1.4 0 0 1 2.3-1.6L8 14'/>"),
        "zoom_bereich" => format!("<rect x='3' y='3' width='11' height='11' stroke-dasharray='2 1.6'/><circle cx='14.5' cy='14.5' r='4'/><path d='M17.5 17.5L21 21' stroke-width='2'/>"),
        "zoom_in" => format!("<circle cx='10' cy='10' r='6.5'/><path d='M15 15l6 6' stroke-width='2'/><path d='M7 10h6M10 7v6' stroke='{A}' stroke-width='1.8'/>"),
        "zoom_out" => format!("<circle cx='10' cy='10' r='6.5'/><path d='M15 15l6 6' stroke-width='2'/><path d='M7 10h6' stroke='{A}' stroke-width='1.8'/>"),
        "zoom_fit" => format!("<path d='M3 8V3h5M16 3h5v5M21 16v5h-5M8 21H3v-5'/><rect x='7' y='7' width='10' height='10' rx='1' stroke='{A}'/>"),
        "zoom_auswahl" => format!("<rect x='4' y='4' width='10' height='10' stroke='{A}' stroke-dasharray='2 1.6'/><circle cx='15' cy='15' r='4'/><path d='M18 18l3 3' stroke-width='2'/>"),

        // ---------- Zeichnen ----------
        "linie" => format!("<path d='M4 20L20 4'/><circle cx='4' cy='20' r='1.6' fill='{A}' stroke='{A}'/><circle cx='20' cy='4' r='1.6' fill='{A}' stroke='{A}'/>"),
        "kreis" => format!("<circle cx='12' cy='12' r='8'/><circle cx='12' cy='12' r='1.3' fill='{A}' stroke='{A}'/>"),
        "ellipse" => format!("<ellipse cx='12' cy='12' rx='9' ry='5.5'/>"),
        "bogen" => format!("<path d='M4 18a10 10 0 0 1 16-8'/><circle cx='4' cy='18' r='1.5' fill='{A}' stroke='{A}'/><circle cx='20' cy='10' r='1.5' fill='{A}' stroke='{A}'/>"),
        "rechteck" => format!("<rect x='3.5' y='6' width='17' height='12'/>"),
        "polygon" => format!("<path d='M5 7l8-3 7 6-3 9-11-2z'/>"),
        "neck" => format!("<path d='M12 3.5l7.4 4.25v8.5L12 20.5l-7.4-4.25v-8.5z'/>"),
        "linienzug" => format!("<path d='M3 18l5-8 5 5 8-10'/>"),
        "schlauch" => format!("<path d='M3 9c5 0 5 6 10 6s5-6 8-6' stroke='{R}'/><path d='M3 13c5 0 5 6 10 6s5-6 8-6' stroke='{R}'/>"),
        "spline" => format!("<path d='M3 18C7 2 13 22 21 6'/><circle cx='3' cy='18' r='1.4' fill='{A}' stroke='{A}'/><circle cx='21' cy='6' r='1.4' fill='{A}' stroke='{A}'/>"),
        "text" => format!("<path d='M5 6V4h14v2M12 4v16M9 20h6' stroke-width='1.8'/>"),
        "bild" => format!("<rect x='3' y='4' width='18' height='16' rx='1.5'/><circle cx='8.5' cy='9' r='1.8' fill='{Y}' stroke='{Y}'/><path d='M3 18l5-5 4 4 3-3 6 6' stroke='{G}'/>"),
        "pdf_einbetten" => format!("<rect x='3' y='4' width='18' height='16' rx='1.5'/><rect x='6' y='9' width='12' height='6' rx='1' fill='{R}' stroke='{R}'/><text x='12' y='13.8' font-family='Arial' font-weight='bold' font-size='5' text-anchor='middle' fill='#fff' stroke='none'>PDF</text>"),
        "masslinie" => format!("<path d='M4 6v12M20 6v12'/><path d='M4 12h16' stroke='{A}'/><path d='M7 9.5L4 12l3 2.5M17 9.5l3 2.5-3 2.5' stroke='{A}'/>"),
        "xref" => format!("<rect x='3' y='3' width='12' height='12' rx='1'/><path d='M13 21h8v-8M21 21l-8-8' stroke='{A}' stroke-width='1.8'/>"),
        "tabelle" => format!("<rect x='3' y='4' width='18' height='16' rx='1'/><path d='M3 9h18M3 14.5h18M9 4v16M15 4v16'/>"),
        "notiz" => format!("<path d='M4 4h16v11l-5 5H4z' fill='#fff4c2'/><path d='M15 20v-5h5M7 9h10M7 12h6'/>"),
        "weitere" => format!("<circle cx='5' cy='12' r='1.6' fill='{D}'/><circle cx='12' cy='12' r='1.6' fill='{D}'/><circle cx='19' cy='12' r='1.6' fill='{D}'/>"),

        // ---------- Bau / Fluchtwege ----------
        "wand" => format!("<path d='M3 18V6h14v12' stroke-width='3.5' stroke-linecap='square' stroke-linejoin='miter'/>"),
        "wand_ortho" => format!("<path d='M3 15h12V4' stroke-width='3.5' stroke-linecap='square' stroke-linejoin='miter'/><path d='M15 15l5 5' stroke='{A}'/><path d='M19 15h-2v-2' stroke='{A}'/>"),
        "raum" => format!("<rect x='3' y='4' width='18' height='16' stroke-width='2.2'/><path d='M8 12h8' stroke='{A}'/><path d='M3 14h3' stroke='#fff' stroke-width='3'/>"),
        "raummass" => format!("<rect x='3' y='4' width='18' height='16' stroke-width='2'/><text x='12' y='15' font-family='Arial' font-weight='bold' font-size='7' text-anchor='middle' fill='{A}' stroke='none'>m²</text>"),
        "fenster" => format!("<path d='M2 12h5M17 12h5' stroke-width='3.5' stroke-linecap='butt'/><rect x='7' y='10' width='10' height='4'/><path d='M7 12h10'/>"),
        "tuer" => format!("<path d='M2 20h5M19 20h3' stroke-width='3.5' stroke-linecap='butt'/><path d='M7 20V7'/><path d='M7 7a13 13 0 0 1 12 13' stroke='{A}' stroke-dasharray='2 1.5'/>"),
        "treppe" => format!("<rect x='4' y='3' width='16' height='18'/><path d='M4 7h16M4 11h16M4 15h16'/><path d='M12 19V5M9.5 7.5L12 5l2.5 2.5' stroke='{A}' stroke-width='1.6'/>"),
        "fluchtweg" => format!("<rect x='2' y='7' width='20' height='10' rx='1' fill='#c5e5cf' stroke='none'/><path d='M5 9.5l3 2.5-3 2.5M11 9.5l3 2.5-3 2.5M17 9.5l3 2.5-3 2.5' stroke='{G}' stroke-width='1.8'/>"),
        "pfeil" => format!("<rect x='2.5' y='2.5' width='19' height='19' rx='1.5' fill='{G}' stroke='{G}'/><path d='M5.5 12h10M12 7.5l4.5 4.5-4.5 4.5' stroke='#fff' stroke-width='2.4'/>"),
        "raster" => format!("<rect x='4' y='4' width='17' height='17'/><path d='M4 9.7h17M4 15.3h17M9.7 4v17M15.3 4v17'/><text x='2' y='3.5' font-family='Arial' font-size='4' fill='{A}' stroke='none'>A1</text>"),
        "geschoss" => format!("<path d='M3 21h18M5 21V5h14v16'/><path d='M5 10h14M5 15.5h14'/><rect x='5' y='10' width='14' height='5.5' fill='{R}' stroke='{R}' opacity='.75'/>"),

        // ---------- Bearbeiten ----------
        "loeschen" => format!("<path d='M4 7h16M9 7V4h6v3'/><path d='M6.5 7l1 13h9l1-13'/><path d='M10 11v6M14 11v6' stroke='{R}'/>"),
        "rueckgaengig" => format!("<path d='M8 5L4 9l4 4' stroke='{A}' stroke-width='1.8'/><path d='M4 9h10a6 6 0 0 1 0 12h-4'/>"),
        "wiederholen" => format!("<path d='M16 5l4 4-4 4' stroke='{A}' stroke-width='1.8'/><path d='M20 9H10a6 6 0 0 0 0 12h4'/>"),
        "fuellen" => format!("<path d='M11 3l8 8-7 7-8-8z'/><path d='M4 10h15'/><path d='M4 10l8 8 7-7z' fill='{A}' stroke='{A}'/><path d='M21 15c-1.5 2-1.5 3 0 4' stroke='{A}'/>"),
        "plan_schneiden" => format!("<rect x='5' y='5' width='14' height='14' stroke-dasharray='2 1.6'/><circle cx='5' cy='19' r='2.5'/><circle cx='5' cy='5' r='2.5'/><path d='M7 6.5l14 10M7 17.5l14-10' stroke='{R}'/>"),
        "punkt" => format!("<path d='M3 18l9-9 9 9'/><circle cx='12' cy='9' r='2.6' fill='{A}' stroke='{A}'/><path d='M17 3v5M14.5 5.5h5' stroke='{A}'/>"),
        "symbolmanager" => format!("<rect x='3' y='3' width='8' height='8' rx='1' fill='{G}' stroke='{G}'/><rect x='13' y='3' width='8' height='8' rx='1' fill='{R}' stroke='{R}'/><circle cx='7' cy='17' r='4' fill='{A}' stroke='{A}'/><path d='M13 21l4-7 4 7z' fill='{Y}' stroke='{D}'/>"),
        "suchen" => format!("<circle cx='10' cy='10' r='6.5'/><path d='M15 15l6 6' stroke-width='2'/>"),
        "einfuegen" => format!("<rect x='5' y='4' width='14' height='17' rx='1.5'/><rect x='9' y='2.5' width='6' height='3.5' rx='1' fill='#fff'/><rect x='10' y='10' width='10' height='11' rx='1' fill='#fff' stroke='{A}'/><path d='M12.5 14h5M12.5 17h5' stroke='{A}'/>"),
        "ausschneiden" => format!("<circle cx='6.5' cy='17.5' r='3'/><circle cx='17.5' cy='17.5' r='3'/><path d='M8.5 15.5L17 3M15.5 15.5L7 3' stroke='{A}'/>"),
        "kopieren" => format!("<rect x='8' y='8' width='12' height='13' rx='1.5'/><path d='M16 5V4.5A1.5 1.5 0 0 0 14.5 3h-9A1.5 1.5 0 0 0 4 4.5v11A1.5 1.5 0 0 0 5.5 17H6'/>"),
        "duplizieren" => format!("<rect x='8' y='8' width='12' height='12' rx='1.5'/><rect x='4' y='4' width='12' height='12' rx='1.5' fill='#fff'/><path d='M10 7v6M7 10h6' stroke='{A}' stroke-width='1.8'/>"),
        "alles" => format!("<rect x='3' y='3' width='18' height='18' rx='1' stroke-dasharray='2.5 1.8'/><rect x='7' y='7' width='4' height='4' fill='{A}' stroke='{A}'/><rect x='13' y='7' width='4' height='4' fill='{A}' stroke='{A}'/><rect x='7' y='13' width='4' height='4' fill='{A}' stroke='{A}'/><rect x='13' y='13' width='4' height='4' fill='{A}' stroke='{A}'/>"),
        "invertieren" => format!("<rect x='3' y='3' width='18' height='18' rx='1' stroke-dasharray='2.5 1.8'/><rect x='7' y='7' width='4' height='4' fill='{A}' stroke='{A}'/><rect x='13' y='7' width='4' height='4'/><rect x='7' y='13' width='4' height='4'/><rect x='13' y='13' width='4' height='4' fill='{A}' stroke='{A}'/>"),
        "gleiche" => format!("<circle cx='6.5' cy='7' r='3' fill='{A}' stroke='{A}'/><circle cx='17.5' cy='7' r='3' fill='{A}' stroke='{A}'/><rect x='3.5' y='14' width='6' height='6'/><circle cx='17.5' cy='17' r='3' fill='{A}' stroke='{A}'/>"),
        "gruppieren" => format!("<rect x='2.5' y='2.5' width='19' height='19' rx='1' stroke-dasharray='2 1.6'/><rect x='6' y='6' width='7' height='6'/><circle cx='15' cy='15.5' r='3.5' fill='{A}' stroke='{A}'/>"),
        "gruppe_aufheben" => format!("<rect x='3' y='3' width='8' height='7'/><circle cx='16.5' cy='16.5' r='4' fill='{A}' stroke='{A}'/><path d='M14 4l6 6M20 4l-6 6' stroke='{R}'/>"),
        "symbole_erkennen" => format!("<rect x='3' y='3' width='18' height='18' rx='1.5' stroke-dasharray='2 1.6'/><rect x='7' y='7' width='10' height='10' rx='1' fill='{G}' stroke='{G}'/><path d='M9.5 12l2 2 3.5-4' stroke='#fff' stroke-width='1.8'/>"),
        "mehrfach" => format!("<rect x='3' y='9' width='5' height='6'/><rect x='10' y='9' width='5' height='6' stroke='{A}'/><rect x='17' y='9' width='5' height='6' stroke='{A}'/>"),
        "versatz" => format!("<path d='M3 16l6-9h12'/><path d='M3 20l8-10h10' stroke='{A}' stroke-dasharray='2 1.6'/>"),
        "drehen" => format!("<path d='M20 9a8 8 0 1 0-1 7'/><path d='M20 4v5h-5' stroke='{A}' stroke-width='1.8'/>"),
        "spiegeln_h" => format!("<path d='M12 3v18' stroke-dasharray='2 1.6'/><path d='M9 6L3 18h6z'/><path d='M15 6l6 12h-6z' fill='{A}' stroke='{A}'/>"),
        "spiegeln_v" => format!("<path d='M3 12h18' stroke-dasharray='2 1.6'/><path d='M6 9L18 3v6z'/><path d='M6 15l12 6v-6z' fill='{A}' stroke='{A}'/>"),
        "vorne" => format!("<rect x='3' y='3' width='11' height='11'/><rect x='10' y='10' width='11' height='11' fill='{A}' stroke='{A}'/>"),
        "hinten" => format!("<rect x='10' y='10' width='11' height='11' fill='{A}' stroke='{A}'/><rect x='3' y='3' width='11' height='11' fill='#fff'/>"),
        "ausrichten" => format!("<path d='M4 3v18'/><rect x='4' y='6' width='14' height='4' fill='{A}' stroke='{A}'/><rect x='4' y='14' width='9' height='4' fill='{A}' stroke='{A}'/>"),
        "segment" => format!("<path d='M3 12h5M16 12h5' stroke-width='3.5' stroke-linecap='butt'/><path d='M8 12h8' stroke='{R}' stroke-dasharray='1.6 1.6'/>"),
        "ecke" => format!("<path d='M4 20V12a8 8 0 0 1 8-8h8' stroke-width='3' stroke-linecap='butt'/><path d='M4 4h4M4 4v4' stroke='{A}'/>"),
        "kurve" => format!("<path d='M3 21V12A9 9 0 0 1 12 3h9' stroke='{R}' stroke-width='2'/><path d='M9 21v-9a3 3 0 0 1 3-3h9' stroke='{R}' stroke-dasharray='2 1.6'/>"),
        "distanz" => format!("<path d='M3 15l12-12 6 6-12 12z'/><path d='M7 11l2 2M10 8l3 3M13 5l2 2' stroke='{A}'/>"),
        "winkel" => format!("<path d='M4 20h17M4 20L16 5'/><path d='M11 20a7 7 0 0 0-2.6-5.4' stroke='{A}'/>"),

        // ---------- Ansicht ----------
        "gitter" => format!("<path d='M4 4h16v16H4zM4 9.3h16M4 14.7h16M9.3 4v16M14.7 4v16' stroke-dasharray='1.5 1.3'/>"),
        "fang" => format!("<path d='M4 9.3h16M4 14.7h16M9.3 4v16M14.7 4v16' stroke-dasharray='1.5 1.3'/><circle cx='14.7' cy='9.3' r='2.6' fill='{A}' stroke='{A}'/>"),
        "punktfang" => format!("<path d='M4 18L18 6'/><rect x='14' y='2' width='8' height='8' stroke='{A}'/><circle cx='18' cy='6' r='1.3' fill='{A}' stroke='{A}'/>"),
        "lineal" => format!("<rect x='2.5' y='8' width='19' height='8' rx='1'/><path d='M6 8v3M9.5 8v4M13 8v3M16.5 8v4'/>"),
        "hilfslinie" => format!("<path d='M2 8h20M15 2v20' stroke='{A}' stroke-dasharray='3 2'/>"),
        "hilfslinie_loeschen" => format!("<path d='M2 8h20M15 2v20' stroke='{A}' stroke-dasharray='3 2'/><path d='M4 14l6 6M10 14l-6 6' stroke='{R}' stroke-width='1.8'/>"),
        "hintergrund" => format!("<path d='M12 3a9 9 0 1 0 0 18c1.5 0 2-1 1.5-2-.6-1.4.4-3 2-3H18a3 3 0 0 0 3-3c0-5.5-4-10-9-10z'/><circle cx='7.5' cy='11' r='1.5' fill='{R}' stroke='none'/><circle cx='10' cy='7' r='1.5' fill='{Y}' stroke='none'/><circle cx='15' cy='7' r='1.5' fill='{G}' stroke='none'/><circle cx='8.5' cy='15.5' r='1.5' fill='{A}' stroke='none'/>"),
        "qualitaet" => format!("<path d='M12 3l2.6 5.6 6 .7-4.5 4.1 1.2 6L12 16.5l-5.3 2.9 1.2-6L3.4 9.3l6-.7z' fill='{Y}' stroke='{D}'/>"),

        // ---------- Extras ----------
        "osm" => format!("<path d='M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z'/><path d='M9 4v14M15 6v14'/><path d='M5 12c3-2 5 2 8 0s4-1 6 1' stroke='{A}'/>"),
        "wms" => format!("<path d='M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z'/><path d='M6 9h4v4H6zM12 11h5v5h-5z' stroke='{G}'/>"),
        "optionen" => format!("<circle cx='12' cy='12' r='3'/><path d='M12 2.5v2.8M12 18.7v2.8M2.5 12h2.8M18.7 12h2.8M5.3 5.3l2 2M16.7 16.7l2 2M5.3 18.7l2-2M16.7 7.3l2-2'/><circle cx='12' cy='12' r='6.5'/>"),
        "archiv" => format!("<rect x='3' y='4' width='18' height='5' rx='1'/><path d='M4.5 9v11h15V9'/><path d='M10 13h4' stroke='{A}' stroke-width='1.8'/>"),
        "dearchiv" => format!("<rect x='3' y='4' width='18' height='5' rx='1'/><path d='M4.5 9v11h15V9'/><path d='M12 19v-7M9.5 14.5L12 12l2.5 2.5' stroke='{A}' stroke-width='1.6'/>"),
        "bso" => format!("<path d='M6 3h12v18H6z'/><rect x='6' y='3' width='12' height='4' fill='{R}' stroke='{R}'/><path d='M9 11h6M9 14h6M9 17h4'/>"),
        "svg_symbol" => format!("<rect x='3' y='3' width='18' height='18' rx='2'/><path d='M7 16l5-9 5 9z' fill='{Y}' stroke='{D}'/><path d='M12 11v2' stroke-width='1.6'/>"),
        "logo" => format!("<rect x='3' y='5' width='18' height='14' rx='1.5'/><circle cx='9' cy='12' r='3' fill='{A}' stroke='{A}'/><path d='M14 10h4M14 13h4'/>"),
        "hilfe" => format!("<circle cx='12' cy='12' r='9'/><path d='M9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.7v.5' stroke='{A}' stroke-width='1.8'/><circle cx='12' cy='17' r='1' fill='{A}' stroke='{A}'/>"),
        "info" => format!("<circle cx='12' cy='12' r='9'/><path d='M12 11v6' stroke='{A}' stroke-width='2'/><circle cx='12' cy='7.5' r='1.1' fill='{A}' stroke='{A}'/>"),
        _ => return None,
    };
    Some(s)
}

/// Vollständiges SVG eines Menüband-Symbols in der gewünschten Pixelgröße.
fn svg(name: &str, px: u32) -> Option<String> {
    let b = body(name)?;
    Some(format!(
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' width='{px}' height='{px}'>\
         <g fill='none' stroke='{D}' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'>{b}</g></svg>"
    ))
}

#[derive(Default)]
pub struct Icons {
    cache: HashMap<(&'static str, u32), Option<TextureHandle>>,
}

impl Icons {
    /// Textur für ein Symbol in logischer Größe `size` (berücksichtigt die Bildschirmauflösung).
    pub fn get(&mut self, ctx: &egui::Context, name: &'static str, size: f32) -> Option<TextureHandle> {
        let px = (size * ctx.pixels_per_point()).round().max(8.0) as u32;
        self.cache
            .entry((name, px))
            .or_insert_with(|| {
                let src = svg(name, px)?;
                let tree = resvg::usvg::Tree::from_str(&src, crate::export::usvg_options()).ok()?;
                let mut pm = resvg::tiny_skia::Pixmap::new(px, px)?;
                resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pm.as_mut());
                let img = egui::ColorImage::from_rgba_premultiplied([px as usize, px as usize], pm.data());
                Some(ctx.load_texture(format!("icon_{name}_{px}"), img, egui::TextureOptions::LINEAR))
            })
            .clone()
    }
}

#[cfg(test)]
mod tests {
    /// Jedes Menüband-Symbol muss sich fehlerfrei zeichnen lassen.
    #[test]
    fn alle_symbole_lesbar() {
        for name in super::NAMEN {
            let src = super::svg(name, 32).unwrap_or_else(|| panic!("Symbol fehlt: {name}"));
            resvg::usvg::Tree::from_str(&src, crate::export::usvg_options()).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}

/// Alle bekannten Namen (für den Test)
#[cfg(test)]
const NAMEN: &[&str] = &[
    "neu", "oeffnen", "speichern", "speichern_unter", "schliessen", "zuletzt", "muster", "willkommen", "drucken", "pdf", "beenden", "importieren",
    "exportieren", "pruefen", "format", "hoch", "quer", "rahmen", "massstab", "massstab_distanz", "bereinigen", "extrahieren", "legende", "drehen_l",
    "drehen_r", "drehen_180", "plan_duplizieren", "laufkarte", "container", "auswahl", "verschieben", "zoom_bereich", "zoom_in", "zoom_out", "zoom_fit",
    "zoom_auswahl", "linie", "kreis", "ellipse", "bogen", "rechteck", "polygon", "neck", "linienzug", "schlauch", "spline", "text", "bild",
    "pdf_einbetten", "masslinie", "xref", "tabelle", "notiz", "weitere", "wand", "wand_ortho", "raum", "raummass", "fenster", "tuer", "treppe",
    "fluchtweg", "pfeil", "raster", "geschoss", "loeschen", "rueckgaengig", "wiederholen", "fuellen", "plan_schneiden", "punkt", "symbolmanager",
    "suchen", "einfuegen", "ausschneiden", "kopieren", "duplizieren", "alles", "invertieren", "gleiche", "gruppieren", "gruppe_aufheben",
    "symbole_erkennen", "mehrfach", "versatz", "drehen", "spiegeln_h", "spiegeln_v", "vorne", "hinten", "ausrichten", "segment", "ecke", "kurve",
    "distanz", "winkel", "gitter", "fang", "punktfang", "lineal", "hilfslinie", "hilfslinie_loeschen", "hintergrund", "qualitaet", "osm", "wms",
    "optionen", "archiv", "dearchiv", "bso", "svg_symbol", "logo", "hilfe", "info",
];

