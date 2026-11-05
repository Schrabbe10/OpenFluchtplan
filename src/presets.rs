//! Flächenvorlagen: Fluchtwege/Feuerwehrplan-Flächen und Brandschutz-Kennzeichnungen.

use crate::model::{Color, Fill, Hatch, Style, StrokeStyle, farbe};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Flaechen,
    Brandschutz,
}

pub struct AreaPreset {
    pub key: &'static str,
    pub name: &'static str,
    pub group: Group,
    pub style: Style,
}

const fn hatch(bg: Option<Color>, color: Color, angle: f32, spacing: f32, width: f32) -> Fill {
    Fill::Schraffur(Hatch {
        bg,
        color,
        angle,
        spacing,
        width,
        cross: false,
    })
}

const fn area(fill: Fill) -> Style {
    Style {
        stroke: None,
        fill,
    }
}

const fn area_outline(fill: Fill, c: Color) -> Style {
    Style {
        stroke: Some(StrokeStyle::new(c, 0.3)),
        fill,
    }
}

const LILA: Color = Color::rgb(0x7b, 0x2d, 0x8e);
const ROT: Color = Color::rgb(0xe3, 0x1e, 0x24);
const ORANGE: Color = Color::rgb(0xf0, 0x8c, 0x00);
const GELB: Color = Color::rgb(0xff, 0xe6, 0x00);
const BRAUN_HELL: Color = Color::rgb(0xd9, 0xb3, 0x8c);
const BRAUN: Color = Color::rgb(0x8b, 0x5a, 0x2b);
const BRAUN_DUNKEL: Color = Color::rgb(0x5c, 0x3a, 0x1e);
const GRUEN_DUNKEL: Color = Color::rgb(0x1b, 0x7a, 0x3a);
const GRUEN_MITTEL: Color = Color::rgb(0x3c, 0xb4, 0x4a);
const BLAU_HELL: Color = Color::rgb(0x2f, 0x8f, 0xd8);
const GRAU_MITTEL: Color = Color::rgb(0xa0, 0xa0, 0xa0);
const BEIGE: Color = Color::rgb(0xec, 0xd9, 0xb8);
const DUNKELROT: Color = Color::rgb(0xa3, 0x23, 0x2b);

pub static PRESETS: &[AreaPreset] = &[
    // Flucht- und Rettungspläne / Feuerwehrpläne (DIN 14095)
    AreaPreset { key: "fluchtbereich", name: "Horizontaler Rettungsweg (Fläche)", group: Group::Flaechen, style: area(Fill::Farbe(farbe::FLUCHT_HELL)) },
    AreaPreset { key: "fluchttreppe", name: "Geschosse durchdringende Treppe / Rampe", group: Group::Flaechen, style: area(Fill::Farbe(GRUEN_DUNKEL)) },
    AreaPreset { key: "befahrbar", name: "Befahrbare Flächen", group: Group::Flaechen, style: area(Fill::Farbe(GRAU_MITTEL)) },
    AreaPreset { key: "nicht_befahrbar", name: "Nicht befahrbare Flächen", group: Group::Flaechen, style: area(Fill::Farbe(GELB)) },
    AreaPreset { key: "gefahr", name: "Besondere Gefahren", group: Group::Flaechen, style: area(Fill::Farbe(DUNKELROT)) },
    AreaPreset { key: "loeschwasser", name: "Löschwasser", group: Group::Flaechen, style: area(Fill::Farbe(BLAU_HELL)) },
    AreaPreset { key: "bezugsgebaeude", name: "Betroffene bauliche Anlage", group: Group::Flaechen, style: area_outline(Fill::Farbe(BEIGE), farbe::SCHWARZ) },
    AreaPreset { key: "gasloeschbereich", name: "Löschbereich Gaslöschanlage", group: Group::Flaechen, style: area(hatch(None, BLAU_HELL, 45.0, 2.0, 0.3)) },
    AreaPreset { key: "sprinklerbereich", name: "Gesprinklerter Bereich", group: Group::Flaechen, style: area(hatch(None, BLAU_HELL, -45.0, 3.0, 0.25)) },
    // Brandschutzpläne: Feuerwiderstand und Baustoffe
    AreaPreset { key: "rei90m", name: "REI 90-M A1/A2 (Brandwand)", group: Group::Brandschutz, style: area(Fill::Farbe(LILA)) },
    AreaPreset { key: "rei90", name: "REI 90 (F90-Bauteil)", group: Group::Brandschutz, style: area(Fill::Farbe(ROT)) },
    AreaPreset { key: "rei90_ersatz", name: "Bauart REI 90-M (Bauart Brandwand)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), LILA, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "rei90a", name: "REI 90 A1/A2 (F90-A-Bauteil)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), ROT, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "rei90ab", name: "REI 90 wesentliche Teile A1/A2 (F90-AB)", group: Group::Brandschutz, style: area(hatch(Some(GRAU_MITTEL), ROT, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "rei60", name: "REI 60 (F60-Bauteil)", group: Group::Brandschutz, style: area(Fill::Farbe(ORANGE)) },
    AreaPreset { key: "rei60a", name: "REI 60 A1/A2 (F60-A-Bauteil)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), ORANGE, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "rei60ab", name: "REI 60 wesentliche Teile A1/A2 (F60-AB)", group: Group::Brandschutz, style: area(hatch(Some(GRAU_MITTEL), ORANGE, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "rei60k260", name: "REI 60 K2 60 (F60-BA)", group: Group::Brandschutz, style: area(hatch(Some(ORANGE), farbe::SCHWARZ, 45.0, 1.5, 0.5)) },
    AreaPreset { key: "rei30", name: "REI 30 (F30-Bauteil)", group: Group::Brandschutz, style: area(Fill::Farbe(GELB)) },
    AreaPreset { key: "rei30a", name: "REI 30 A1/A2 (F30-A-Bauteil)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), GELB, 45.0, 1.5, 0.8)) },
    AreaPreset { key: "rei30ab", name: "REI 30 wesentliche Teile A1/A2 (F30-AB)", group: Group::Brandschutz, style: area(hatch(Some(GRAU_MITTEL), GELB, 45.0, 1.5, 0.8)) },
    AreaPreset { key: "rei30bk", name: "REI 30 nichtbrennbar bekleidet (fluchtwegseitig)", group: Group::Brandschutz, style: area(hatch(Some(GELB), farbe::SCHWARZ, 45.0, 1.5, 0.5)) },
    AreaPreset { key: "a1", name: "Baustoff A1 (nichtbrennbar)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), BRAUN_HELL, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "a2", name: "Baustoff A2 (nichtbrennbar)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), BRAUN, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "b1", name: "Baustoff B1 (schwer entflammbar)", group: Group::Brandschutz, style: area(hatch(Some(farbe::WEISS), BRAUN_DUNKEL, 45.0, 1.5, 0.6)) },
    AreaPreset { key: "notw_treppe", name: "Notwendige Treppe / Treppenraum", group: Group::Brandschutz, style: area(Fill::Farbe(GRUEN_DUNKEL)) },
    AreaPreset { key: "sicherheitstreppe", name: "Sicherheitstreppenraum", group: Group::Brandschutz, style: area(hatch(Some(BLAU_HELL), GRUEN_DUNKEL, 45.0, 1.5, 0.8)) },
    AreaPreset { key: "notw_flur", name: "Notwendiger Flur", group: Group::Brandschutz, style: area(Fill::Farbe(GRUEN_MITTEL)) },
    AreaPreset { key: "schleuse", name: "Schleuse / Aufzugsvorraum", group: Group::Brandschutz, style: area(hatch(Some(GRUEN_MITTEL), farbe::SCHWARZ, 45.0, 1.5, 0.5)) },
    AreaPreset { key: "brandabschnitt", name: "Brandabschnitt (Umrandung)", group: Group::Brandschutz, style: Style { stroke: Some(StrokeStyle { color: ROT, width: 1.2, dash: crate::model::Dash::StrichPunkt }), fill: Fill::Keine } },
];

pub fn find(key: &str) -> Option<&'static AreaPreset> {
    PRESETS.iter().find(|p| p.key == key)
}
