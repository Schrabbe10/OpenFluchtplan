//! Symbolbibliothek. Alle Piktogramme sind eigene, vereinfachte Vektorzeichnungen
//! (SVG im Koordinatensystem 0..100) in Anlehnung an die Zeichenarten nach
//! ISO 7010, DIN 14034-6 und GHS. Sie sind keine Kopien fremder Grafiken.

use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Rettung,
    Brandschutz,
    Feuerwehr,
    Brandmeldetechnik,
    Warnung,
    Gebot,
    Verbot,
    Gefahrstoffe,
    Gebaeude,
    Verkehr,
    Nordpfeile,
    Einrichtung,
}

impl Category {
    pub const ALL: [Category; 12] = [
        Category::Rettung,
        Category::Brandschutz,
        Category::Feuerwehr,
        Category::Brandmeldetechnik,
        Category::Warnung,
        Category::Gebot,
        Category::Verbot,
        Category::Gefahrstoffe,
        Category::Gebaeude,
        Category::Verkehr,
        Category::Nordpfeile,
        Category::Einrichtung,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Category::Rettung => "Rettungszeichen (ISO 7010 E)",
            Category::Brandschutz => "Brandschutzzeichen (ISO 7010 F)",
            Category::Feuerwehr => "Feuerwehrpläne (DIN 14034-6)",
            Category::Brandmeldetechnik => "Brandmeldetechnik / Laufkarten",
            Category::Warnung => "Warnzeichen (ISO 7010 W)",
            Category::Gebot => "Gebotszeichen (ISO 7010 M)",
            Category::Verbot => "Verbotszeichen (ISO 7010 P)",
            Category::Gefahrstoffe => "Gefahrstoffe (GHS)",
            Category::Gebaeude => "Gebäudebeschilderung",
            Category::Verkehr => "Verkehrszeichen / Zufahrt",
            Category::Nordpfeile => "Nordpfeile",
            Category::Einrichtung => "Einrichtung / Modelle",
        }
    }
}

pub struct SymbolDef {
    pub id: &'static str,
    pub cat: Category,
    pub name: &'static str,
    pub svg: String,
}

const G: &str = "#237f52";
const R: &str = "#b01e23";
const B: &str = "#1f4e9c";
const Y: &str = "#f9c400";
const K: &str = "#000000";
const W: &str = "#ffffff";

// ---------- Grundformen ----------

fn rescue(inner: &str) -> String {
    format!("<rect x='0' y='0' width='100' height='100' rx='5' fill='{G}'/>{inner}")
}
fn fire(inner: &str) -> String {
    format!("<rect x='0' y='0' width='100' height='100' rx='5' fill='{R}'/>{inner}")
}
fn mandatory(inner: &str) -> String {
    format!("<circle cx='50' cy='50' r='48' fill='{B}'/>{inner}")
}
fn prohibition(inner: &str) -> String {
    format!(
        "<circle cx='50' cy='50' r='48' fill='{W}'/>{inner}\
         <circle cx='50' cy='50' r='42' fill='none' stroke='{R}' stroke-width='11'/>\
         <line x1='20' y1='20' x2='80' y2='80' stroke='{R}' stroke-width='10'/>"
    )
}
fn warning(inner: &str) -> String {
    format!(
        "<polygon points='50,6 96,90 4,90' fill='{Y}' stroke='{K}' stroke-width='6' stroke-linejoin='round'/>\
         <g transform='translate(25,38) scale(0.5)'>{inner}</g>"
    )
}
fn ghs(inner: &str) -> String {
    format!(
        "<polygon points='50,4 96,50 50,96 4,50' fill='{W}' stroke='{R}' stroke-width='7' stroke-linejoin='round'/>\
         <g transform='translate(25,25) scale(0.5)'>{inner}</g>"
    )
}
/// Feuerwehrplan-Zeichen: Quadrat mit farbigem Rand
fn fw_box(inner: &str, c: &str) -> String {
    format!("<rect x='4' y='4' width='92' height='92' fill='{W}' stroke='{c}' stroke-width='7'/>{inner}")
}
fn label(t: &str, c: &str) -> String {
    let n = t.chars().count();
    let fs = match n {
        0 | 1 => 62.0,
        2 => 46.0,
        3 => 36.0,
        4 => 27.0,
        _ => 22.0,
    };
    format!(
        "<text x='50' y='{:.1}' font-family='Arial' font-weight='bold' font-size='{fs}' text-anchor='middle' fill='{c}'>{t}</text>",
        50.0 + fs * 0.36
    )
}
fn fw_text(t: &str, c: &str) -> String {
    fw_box(&label(t, c), c)
}
fn small_flame(c: &str) -> String {
    format!("<g transform='translate(66,6) scale(0.3)'>{}</g>", flame(c))
}
fn mirror(inner: &str) -> String {
    format!("<g transform='translate(100,0) scale(-1,1)'>{inner}</g>")
}

// ---------- Piktogramme (füllen das Feld 0..100) ----------

/// Laufende Person (nach rechts), kräftige Vollfigur wie bei den Rettungszeichen
fn man(c: &str) -> String {
    format!(
        "<g fill='none' stroke='{c}' stroke-linecap='round' stroke-linejoin='round'>\
         <path d='M57 35 L47 59' stroke-width='15'/>\
         <path d='M59 37 L70 49 L83 45' stroke-width='10.5'/><path d='M54 37 L41 43 L31 55' stroke-width='10.5'/>\
         <path d='M49 60 L65 68 L63 87 L70 88' stroke-width='11.5'/><path d='M46 61 L36 77 L19 79' stroke-width='11.5'/></g>\
         <circle cx='65' cy='16' r='9' fill='{c}'/>"
    )
}
/// Stehende Person (Vorderansicht), Vollfigur
fn standing_man(c: &str) -> String {
    format!("<g transform='translate(50,50)'>{}</g>", person(c))
}
/// Person mit Mittelpunkt (0,0), Höhe von −46 bis +40
fn person(c: &str) -> String {
    format!(
        "<circle cx='0' cy='-37' r='9' fill='{c}'/>\
         <path d='M-11 -25 H11 L9 6 H-9 Z' fill='{c}' stroke='{c}' stroke-width='5' stroke-linejoin='round'/>\
         <g fill='none' stroke='{c}' stroke-linecap='round'><path d='M-13.5 -22 L-16 4 M13.5 -22 L16 4' stroke-width='7'/>\
         <path d='M-5.5 6 L-5.5 39 M5.5 6 L5.5 39' stroke-width='8.5'/></g>"
    )
}
/// Kopf und Schultern (Vorderansicht) für Gebotszeichen; `bg` trennt Ausrüstung und Kopf
fn bust(c: &str) -> String {
    format!(
        "<ellipse cx='50' cy='48' rx='17' ry='21' fill='{c}'/>\
         <path d='M18 94 C18 78 30 70 42 69 L50 76 L58 69 C70 70 82 78 82 94 Z' fill='{c}'/><rect x='44' y='62' width='12' height='10' fill='{c}'/>"
    )
}
fn door(c: &str) -> String {
    format!("<rect x='72' y='8' width='22' height='84' fill='none' stroke='{c}' stroke-width='5'/>")
}
fn arrow(c: &str) -> String {
    format!("<polygon points='10,40 56,40 56,18 92,50 56,82 56,60 10,60' fill='{c}'/>")
}
fn diag_arrow(c: &str) -> String {
    format!("<g transform='rotate(-45 50 50)'>{}</g>", arrow(c))
}
fn cross(c: &str) -> String {
    format!(
        "<rect x='38' y='14' width='24' height='72' fill='{c}'/><rect x='14' y='38' width='72' height='24' fill='{c}'/>"
    )
}
fn phone(c: &str) -> String {
    format!(
        "<path d='M24 18 C15 21 12 31 15 42 C22 64 37 79 58 86 C69 89 79 85 82 76 L72 64 C67 60 61 62 58 67 \
         C47 62 39 54 34 43 C39 40 41 34 37 29 Z' fill='{c}'/>"
    )
}
fn flame(c: &str) -> String {
    format!(
        "<path d='M50 6 C64 28 82 40 76 64 C72 82 60 92 50 92 C36 92 24 82 24 64 C24 50 34 42 38 30 \
         C44 42 48 46 52 50 C56 38 54 22 50 6 Z' fill='{c}'/>"
    )
}
fn extinguisher(c: &str) -> String {
    format!(
        "<rect x='30' y='34' width='26' height='56' rx='6' fill='{c}'/><rect x='37' y='22' width='12' height='12' fill='{c}'/>\
         <path d='M43 24 L66 24 L74 34' fill='none' stroke='{c}' stroke-width='5'/><path d='M49 28 L60 18' stroke='{c}' stroke-width='5'/>"
    )
}
fn hose_reel(c: &str, bg: &str) -> String {
    format!(
        "<circle cx='40' cy='50' r='28' fill='{c}'/><circle cx='40' cy='50' r='17' fill='{bg}'/><circle cx='40' cy='50' r='7' fill='{c}'/>\
         <path d='M40 78 L80 78 L84 70' fill='none' stroke='{c}' stroke-width='6'/>"
    )
}
fn ladder(c: &str) -> String {
    format!(
        "<g stroke='{c}' stroke-width='7'><path d='M32 8 L32 92 M60 8 L60 92 M32 22 L60 22 M32 40 L60 40 M32 58 L60 58 M32 76 L60 76'/></g>"
    )
}
fn bolt(c: &str) -> String {
    format!("<polygon points='58,4 26,54 48,54 38,96 76,40 54,40 66,4' fill='{c}'/>")
}
fn exclam(c: &str) -> String {
    format!("<rect x='42' y='8' width='16' height='58' rx='6' fill='{c}'/><circle cx='50' cy='82' r='9' fill='{c}'/>")
}
/// Explodierender Körper: Kugel mit wegfliegenden Splittern
fn burst(c: &str) -> String {
    format!(
        "<circle cx='50' cy='54' r='17' fill='{c}'/>\
         <g fill='{c}'><polygon points='50,4 56,28 44,28'/><polygon points='50,96 44,80 56,80'/><polygon points='4,54 26,48 26,60'/><polygon points='96,54 74,60 74,48'/>\
         <polygon points='16,18 34,32 28,38'/><polygon points='84,18 72,38 66,32'/><polygon points='16,90 28,70 34,76'/><polygon points='84,90 66,76 72,70'/>\
         <rect x='26' y='8' width='7' height='7' transform='rotate(20 29 11)'/><rect x='68' y='6' width='6' height='6' transform='rotate(35 71 9)'/>\
         <rect x='6' y='30' width='6' height='6'/><rect x='88' y='74' width='6' height='6' transform='rotate(15 91 77)'/></g>"
    )
}
fn skull(c: &str) -> String {
    format!(
        "<path d='M50 8 C28 8 18 24 20 42 C21 52 28 56 30 62 L30 70 L70 70 L70 62 C72 56 79 52 80 42 C82 24 72 8 50 8 Z' fill='{c}'/>\
         <circle cx='38' cy='40' r='8' fill='#fff'/><circle cx='62' cy='40' r='8' fill='#fff'/>\
         <g stroke='{c}' stroke-width='8' stroke-linecap='round'><path d='M14 78 L86 94 M14 94 L86 78'/></g>"
    )
}
/// Ätzende Stoffe: zwei Reagenzgläser tropfen auf eine Hand und eine Oberfläche
fn drops_hand(c: &str) -> String {
    format!(
        "<g stroke='{c}' stroke-width='11' stroke-linecap='round'><path d='M40 4 L26 28'/><path d='M60 4 L74 28'/></g>\
         <path d='M22 38 C18 45 18 49 22 51 C26 49 26 45 22 38 Z M78 38 C74 45 74 49 78 51 C82 49 82 45 78 38 Z' fill='{c}'/>\
         <path d='M2 68 L34 68 C40 68 44 71 44 75 C44 79 40 82 34 82 L2 82 Z' fill='{c}'/><path d='M8 68 C8 58 16 56 22 60 L18 68 Z' fill='{c}'/>\
         <polygon points='16,68 28,68 22,76' fill='{Y}'/>\
         <rect x='56' y='66' width='42' height='18' fill='{c}'/><polygon points='71,66 85,66 78,77' fill='{Y}'/>"
    )
}
fn gas_bottle(c: &str) -> String {
    format!("<rect x='34' y='24' width='32' height='68' rx='14' fill='{c}'/><rect x='42' y='10' width='16' height='14' fill='{c}'/>")
}
fn tree_fish(c: &str) -> String {
    format!(
        "<path d='M28 92 L28 40 M28 50 L14 34 M28 46 L42 28 M28 62 L44 52' stroke='{c}' stroke-width='6' fill='none'/>\
         <path d='M52 74 C62 62 80 62 88 74 C80 86 62 86 52 74 Z M52 74 L44 66 L44 82 Z' fill='{c}'/><rect x='6' y='90' width='88' height='5' fill='{c}'/>"
    )
}
fn health(c: &str) -> String {
    format!(
        "<circle cx='50' cy='16' r='10' fill='{c}'/><path d='M22 92 C22 50 30 34 50 34 C70 34 78 50 78 92 Z' fill='{c}'/>\
         <polygon points='50,44 54,56 66,56 56,63 60,75 50,68 40,75 44,63 34,56 46,56' fill='#fff'/>"
    )
}
fn flame_circle(c: &str) -> String {
    format!("<g transform='translate(15,0) scale(0.7)'>{}</g><circle cx='50' cy='80' r='14' fill='none' stroke='{c}' stroke-width='7'/>", flame(c))
}
fn cigarette(c: &str) -> String {
    format!(
        "<rect x='16' y='54' width='62' height='12' fill='{c}'/><rect x='80' y='54' width='6' height='12' fill='{c}'/>\
         <path d='M66 48 C60 40 72 34 66 24 M76 48 C70 40 82 34 76 24' stroke='{c}' stroke-width='4' fill='none'/>"
    )
}
fn hand(c: &str) -> String {
    format!(
        "<path d='M34 90 L30 52 L22 40 C20 36 26 32 30 36 L36 46 L36 18 C36 13 44 13 44 18 L44 40 L46 12 C46 7 54 7 54 12 L54 40 \
         L58 16 C58 11 66 11 66 16 L64 42 L70 26 C71 21 78 22 77 27 L72 58 C70 74 64 82 62 90 Z' fill='{c}'/>"
    )
}
fn forklift(c: &str) -> String {
    format!(
        "<rect x='22' y='40' width='40' height='30' fill='{c}'/><path d='M30 40 L36 18 L56 18 L58 40' fill='none' stroke='{c}' stroke-width='5'/>\
         <rect x='66' y='14' width='6' height='62' fill='{c}'/><rect x='72' y='68' width='20' height='6' fill='{c}'/>\
         <circle cx='32' cy='76' r='9' fill='{c}'/><circle cx='56' cy='76' r='9' fill='{c}'/>"
    )
}
fn waves(c: &str) -> String {
    format!(
        "<g fill='none' stroke='{c}' stroke-width='6'><path d='M28 74 C20 62 36 54 28 40 C20 28 34 20 30 10'/>\
         <path d='M50 74 C42 62 58 54 50 40 C42 28 56 20 52 10'/><path d='M72 74 C64 62 80 54 72 40 C64 28 78 20 74 10'/></g>\
         <rect x='12' y='80' width='76' height='10' fill='{c}'/>"
    )
}
fn battery(c: &str) -> String {
    format!(
        "<rect x='16' y='30' width='62' height='44' rx='4' fill='none' stroke='{c}' stroke-width='7'/><rect x='80' y='42' width='8' height='20' fill='{c}'/>\
         <path d='M30 52 L42 52 M36 46 L36 58 M56 52 L68 52' stroke='{c}' stroke-width='5'/>"
    )
}
fn trefoil(c: &str) -> String {
    format!(
        "<circle cx='50' cy='52' r='8' fill='{c}'/>\
         <path d='M50 40 L36 16 A36 36 0 0 1 64 16 Z' fill='{c}'/>\
         <path d='M60 58 L88 58 A36 36 0 0 1 74 84 Z' fill='{c}'/>\
         <path d='M40 58 L26 84 A36 36 0 0 1 12 58 Z' fill='{c}'/>"
    )
}
/// Zeichen für Biogefährdung (drei offene Ringe um einen Mittelring)
fn biohazard(c: &str) -> String {
    let (cx, cy) = (50.0_f32, 56.0_f32);
    let mut out = String::new();
    let dirs: Vec<(f32, f32)> = [-90.0_f32, 30.0, 150.0].iter().map(|a| (a.to_radians().cos(), a.to_radians().sin())).collect();
    for (dx, dy) in &dirs {
        out += &format!("<circle cx='{:.2}' cy='{:.2}' r='22' fill='{c}'/>", cx + dx * 22.0, cy + dy * 22.0);
    }
    for (dx, dy) in &dirs {
        out += &format!("<circle cx='{:.2}' cy='{:.2}' r='15.5' fill='{Y}'/>", cx + dx * 27.0, cy + dy * 27.0);
    }
    out += &format!("<circle cx='{cx}' cy='{cy}' r='14' fill='none' stroke='{c}' stroke-width='4.5'/><circle cx='{cx}' cy='{cy}' r='6' fill='{Y}'/>");
    for (dx, dy) in &dirs {
        out += &format!(
            "<line x1='{:.2}' y1='{:.2}' x2='{:.2}' y2='{:.2}' stroke='{Y}' stroke-width='2.6'/>",
            cx + dx * 4.0,
            cy + dy * 4.0,
            cx + dx * 16.0,
            cy + dy * 16.0
        );
    }
    out
}
fn slip(c: &str) -> String {
    format!(
        "<circle cx='34' cy='22' r='9' fill='{c}'/><g fill='none' stroke='{c}' stroke-width='9' stroke-linecap='round'>\
         <path d='M38 34 L58 56'/><path d='M58 56 L80 50'/><path d='M58 56 L56 78'/><path d='M40 36 L22 48'/><path d='M44 40 L64 30'/></g>\
         <rect x='8' y='86' width='84' height='6' fill='{c}'/>"
    )
}
fn magnet(c: &str) -> String {
    format!(
        "<path d='M24 20 L24 56 C24 74 36 86 50 86 C64 86 76 74 76 56 L76 20 L60 20 L60 56 C60 64 56 70 50 70 C44 70 40 64 40 56 L40 20 Z' fill='{c}'/>"
    )
}
fn laser(c: &str) -> String {
    format!(
        "<polygon points='50,38 56,46 66,44 60,52 66,62 56,58 50,66 44,58 34,62 40,52 34,44 44,46' fill='{c}'/>\
         <path d='M8 52 L36 52' stroke='{c}' stroke-width='6'/><path d='M60 52 L94 52 M58 40 L84 22 M58 64 L84 82 M50 30 L50 8 M50 72 L50 94' stroke='{c}' stroke-width='4'/>"
    )
}
fn boot(c: &str) -> String {
    format!("<path d='M30 14 L56 14 L56 56 C70 58 86 62 86 74 L86 84 L24 84 L24 60 Z' fill='{c}'/>")
}
fn glove(c: &str) -> String {
    format!(
        "<path d='M34 92 L30 56 L20 44 C17 39 24 34 28 39 L34 48 L34 20 C34 14 44 14 44 20 L44 42 L46 14 C46 8 56 8 56 14 \
         L56 42 L60 18 C60 12 70 12 70 18 L68 44 L74 30 C76 24 84 26 82 32 L76 60 C74 74 68 84 66 92 Z' fill='{c}'/>"
    )
}
fn book(c: &str) -> String {
    format!(
        "<path d='M14 24 C30 18 42 20 48 28 L48 84 C42 76 30 74 14 80 Z M86 24 C70 18 58 20 52 28 L52 84 C58 76 70 74 86 80 Z' fill='{c}'/>"
    )
}
fn elevator(c: &str) -> String {
    format!(
        "<rect x='20' y='12' width='60' height='76' fill='none' stroke='{c}' stroke-width='6'/>\
         <polygon points='50,20 60,34 40,34' fill='{c}'/><polygon points='50,80 60,66 40,66' fill='{c}'/>\
         <circle cx='50' cy='44' r='5' fill='{c}'/><rect x='45' y='50' width='10' height='14' fill='{c}'/>"
    )
}
fn stairs(c: &str) -> String {
    format!("<path d='M14 86 L14 70 L32 70 L32 54 L50 54 L50 38 L68 38 L68 22 L86 22 L86 86 Z' fill='{c}'/>")
}
fn drop(c: &str) -> String {
    format!("<path d='M50 10 C62 32 76 46 76 62 C76 78 64 90 50 90 C36 90 24 78 24 62 C24 46 38 32 50 10 Z' fill='{c}'/>")
}
fn sprinkler(c: &str) -> String {
    format!(
        "<rect x='44' y='8' width='12' height='26' fill='{c}'/><polygon points='34,34 66,34 58,46 42,46' fill='{c}'/>\
         <g stroke='{c}' stroke-width='4' stroke-linecap='round'><path d='M50 52 L50 66 M40 50 L30 64 M60 50 L70 64 M32 48 L16 56 M68 48 L84 56 M50 74 L50 88 M36 70 L28 84 M64 70 L72 84'/></g>"
    )
}
fn hydrant_over(c: &str) -> String {
    format!(
        "<circle cx='50' cy='52' r='38' fill='none' stroke='{c}' stroke-width='7'/><circle cx='50' cy='52' r='22' fill='{c}'/>\
         <rect x='44' y='4' width='12' height='20' fill='{c}'/>"
    )
}
fn hydrant_under(c: &str) -> String {
    format!(
        "<circle cx='50' cy='50' r='38' fill='none' stroke='{c}' stroke-width='7'/><circle cx='50' cy='50' r='22' fill='{c}'/>\
         <path d='M50 12 L50 88 M12 50 L88 50' stroke='#fff' stroke-width='4'/>"
    )
}
fn switch_sym(c: &str) -> String {
    format!(
        "<g stroke='{c}' stroke-width='6' stroke-linecap='round' fill='none'><path d='M14 64 L36 64 L68 38'/><path d='M70 64 L88 64'/></g>\
         <circle cx='36' cy='64' r='5' fill='{c}'/><circle cx='70' cy='64' r='5' fill='{c}'/>"
    )
}
fn smoke(c: &str) -> String {
    format!(
        "<g fill='none' stroke='{c}' stroke-width='6' stroke-linecap='round'>\
         <path d='M30 86 C22 74 38 66 30 54 C22 42 36 34 30 22'/><path d='M50 86 C42 74 58 66 50 54 C42 42 56 34 50 22'/>\
         <path d='M70 86 C62 74 78 66 70 54 C62 42 76 34 70 22'/></g><path d='M18 14 L82 14' stroke='{c}' stroke-width='8'/>"
    )
}
fn thermometer(c: &str) -> String {
    format!(
        "<rect x='42' y='10' width='16' height='56' rx='8' fill='none' stroke='{c}' stroke-width='6'/><circle cx='50' cy='76' r='14' fill='{c}'/>\
         <rect x='47' y='34' width='6' height='36' fill='{c}'/>"
    )
}
fn press_button(c: &str) -> String {
    format!(
        "<rect x='18' y='18' width='64' height='64' fill='none' stroke='{c}' stroke-width='7'/><circle cx='50' cy='50' r='16' fill='{c}'/>"
    )
}
fn beacon(c: &str) -> String {
    format!(
        "<path d='M30 74 L30 46 C30 30 70 30 70 46 L70 74 Z' fill='{c}'/><rect x='22' y='74' width='56' height='10' fill='{c}'/>\
         <g stroke='{c}' stroke-width='5' stroke-linecap='round'><path d='M50 8 L50 20 M18 22 L26 30 M82 22 L74 30 M8 50 L20 50 M80 50 L92 50'/></g>"
    )
}
fn horn(c: &str) -> String {
    format!(
        "<polygon points='14,40 30,40 56,18 56,82 30,60 14,60' fill='{c}'/>\
         <g fill='none' stroke='{c}' stroke-width='5'><path d='M66 34 C74 42 74 58 66 66'/><path d='M76 24 C90 38 90 62 76 76'/></g>"
    )
}
fn north(variant: u8) -> String {
    match variant {
        0 => format!(
            "<polygon points='50,4 74,90 50,74 26,90' fill='{K}'/><polygon points='50,4 50,74 26,90' fill='{W}' stroke='{K}' stroke-width='3'/>"
        ),
        1 => format!(
            "<circle cx='50' cy='56' r='34' fill='none' stroke='{K}' stroke-width='4'/><polygon points='50,4 62,60 50,52 38,60' fill='{K}'/>\
             <text x='50' y='94' font-family='Arial' font-weight='bold' font-size='22' text-anchor='middle' fill='{K}'>N</text>"
        ),
        2 => format!(
            "<polygon points='50,6 60,50 50,94 40,50' fill='{W}' stroke='{K}' stroke-width='3'/><polygon points='50,6 60,50 40,50' fill='{K}'/>\
             <polygon points='6,50 50,40 94,50 50,60' fill='{W}' stroke='{K}' stroke-width='3'/>\
             <text x='50' y='34' font-family='Arial' font-weight='bold' font-size='16' text-anchor='middle' fill='{W}'>N</text>"
        ),
        _ => format!(
            "<path d='M50 8 L50 92' stroke='{K}' stroke-width='5'/><polygon points='50,4 66,36 50,28 34,36' fill='{K}'/>\
             <text x='72' y='30' font-family='Arial' font-weight='bold' font-size='24' fill='{K}'>N</text>"
        ),
    }
}
fn outline(d: &str) -> String {
    format!("<g fill='none' stroke='{K}' stroke-width='3' stroke-linejoin='round'>{d}</g>")
}

fn build() -> Vec<SymbolDef> {
    use Category::*;
    let mut v: Vec<SymbolDef> = Vec::new();
    let mut add = |id: &'static str, cat: Category, name: &'static str, svg: String| {
        v.push(SymbolDef { id, cat, name, svg });
    };

    // --- Standort & Rettung ---
    add("STANDORT", Rettung, "Standort", format!("<circle cx='50' cy='50' r='46' fill='{B}'/><circle cx='50' cy='50' r='14' fill='{W}'/>"));
    add(
        "STANDORT_PFEIL",
        Rettung,
        "Standort (Sie sind hier)",
        format!(
            "<rect x='2' y='2' width='96' height='40' fill='{B}'/><polygon points='30,42 50,42 18,96' fill='{B}'/>\
             <text x='50' y='29' font-family='Arial' font-weight='bold' font-size='17' text-anchor='middle' fill='{W}'>Standort</text>"
        ),
    );
    add("E002", Rettung, "Notausgang (rechts)", rescue(&format!("<g transform='translate(-6,6) scale(0.82)'>{}</g>{}", man(W), door(W))));
    add("E001", Rettung, "Notausgang (links)", rescue(&mirror(&format!("<g transform='translate(-6,6) scale(0.82)'>{}</g>{}", man(W), door(W)))));
    add("PFEIL_GRUEN", Rettung, "Richtungsangabe (Rettungsweg)", rescue(&arrow(W)));
    add("PFEIL_GRUEN_DIAG", Rettung, "Richtungsangabe schräg", rescue(&diag_arrow(W)));
    add("E003", Rettung, "Erste Hilfe", rescue(&cross(W)));
    add(
        "E004",
        Rettung,
        "Notruftelefon",
        rescue(&format!("<g transform='translate(-4,8) scale(0.85)'>{}</g><g transform='translate(58,6) scale(0.38)'>{}</g>", phone(W), cross(W))),
    );
    add(
        "E007",
        Rettung,
        "Sammelstelle",
        rescue(&format!(
            "{arrows}<g transform='translate(35,61) scale(0.5)'>{p}</g><g transform='translate(65,61) scale(0.5)'>{p}</g>\
             <g transform='translate(50,68) scale(0.36)'>{p}</g>",
            arrows = [0, 90, 180, 270]
                .iter()
                .map(|a| format!("<g transform='rotate({a} 50 50)'><polygon points='0,-3.5 10,-3.5 10,-9 21,0 10,9 10,3.5 0,3.5' transform='translate(7,7) rotate(45)' fill='{W}'/></g>"))
                .collect::<String>(),
            p = person(W)
        )),
    );
    add("E009", Rettung, "Arzt", rescue(&format!("<g transform='translate(8,8) scale(0.5)'>{}</g><g transform='translate(42,20) scale(0.62)'>{}</g>", cross(W), standing_man(W))));
    add(
        "E010",
        Rettung,
        "Automatisierter externer Defibrillator (AED)",
        rescue(&format!(
            "<path d='M50 86 L16 50 C4 36 14 14 32 16 C42 17 47 24 50 30 C53 24 58 17 68 16 C86 14 96 36 84 50 Z' fill='{W}'/>\
             <polygon points='54,26 38,54 50,54 44,76 64,46 52,46 60,26' fill='{G}'/><g transform='translate(66,4) scale(0.3)'>{}</g>",
            cross(W)
        )),
    );
    add(
        "E011",
        Rettung,
        "Augenspüleinrichtung",
        rescue(&format!(
            "<path d='M14 56 C30 36 70 36 86 56 C70 76 30 76 14 56 Z' fill='{W}'/><circle cx='50' cy='56' r='10' fill='{G}'/>\
             <g transform='translate(30,4) scale(0.25)'>{0}</g><g transform='translate(50,4) scale(0.25)'>{0}</g>",
            drop(W)
        )),
    );
    add(
        "E012",
        Rettung,
        "Notdusche",
        rescue(&format!(
            "<path d='M28 10 L72 10 L72 18 L60 18 L60 26' fill='none' stroke='{W}' stroke-width='5'/><polygon points='48,26 72,26 66,34 54,34' fill='{W}'/>\
             <g transform='translate(14,36) scale(0.6)'>{}</g>",
            standing_man(W)
        )),
    );
    add(
        "E013",
        Rettung,
        "Krankentrage",
        rescue(&format!(
            "<rect x='10' y='56' width='80' height='8' fill='{W}'/><circle cx='22' cy='46' r='8' fill='{W}'/><rect x='32' y='42' width='50' height='12' rx='5' fill='{W}'/>\
             <path d='M18 64 L18 80 M82 64 L82 80' stroke='{W}' stroke-width='5'/><g transform='translate(64,4) scale(0.3)'>{}</g>",
            cross(W)
        )),
    );
    add("E016", Rettung, "Notausstieg mit Fluchtleiter", rescue(&format!("{}<rect x='10' y='10' width='40' height='30' fill='none' stroke='{W}' stroke-width='5'/>", ladder(W))));
    add(
        "E017",
        Rettung,
        "Rettungsausstieg",
        rescue(&format!("<rect x='54' y='10' width='38' height='46' fill='none' stroke='{W}' stroke-width='5'/><g transform='translate(-8,18) scale(0.78)'>{}</g>", man(W))),
    );
    add(
        "E008",
        Rettung,
        "Notausgangsvorrichtung (Scheibe einschlagen)",
        rescue(&format!(
            "<rect x='40' y='14' width='48' height='60' fill='none' stroke='{W}' stroke-width='5'/><path d='M50 30 L60 44 L54 52 L70 64' stroke='{W}' stroke-width='3' fill='none'/>\
             <path d='M14 80 L44 50' stroke='{W}' stroke-width='8'/><rect x='34' y='38' width='22' height='14' fill='{W}' transform='rotate(-45 45 45)'/>"
        )),
    );
    add("E_SAMMEL_TEXT", Rettung, "Sammelstelle (Schild)", rescue(&label("S", W)));

    // --- Brandschutzzeichen ---
    add("F001", Brandschutz, "Feuerlöscher", fire(&format!("{}{}", extinguisher(W), small_flame(W))));
    add("F002", Brandschutz, "Löschschlauch / Wandhydrant", fire(&format!("{}{}", hose_reel(W, R), small_flame(W))));
    add("F003", Brandschutz, "Feuerleiter", fire(&format!("{}{}", ladder(W), small_flame(W))));
    add(
        "F004",
        Brandschutz,
        "Mittel und Geräte zur Brandbekämpfung",
        fire(&format!("<g transform='translate(-6,8) scale(0.85)'>{}</g><g transform='translate(54,52) scale(0.4)'>{}</g>{}", extinguisher(W), hose_reel(W, R), small_flame(W))),
    );
    add(
        "F005",
        Brandschutz,
        "Brandmelder (manuell)",
        fire(&format!("<g transform='translate(0,8) scale(0.85)'>{}</g><rect x='10' y='10' width='30' height='30' fill='none' stroke='{W}' stroke-width='5'/>{}", hand(W), small_flame(W))),
    );
    add("F006", Brandschutz, "Brandmeldetelefon", fire(&format!("<g transform='translate(-4,8) scale(0.85)'>{}</g>{}", phone(W), small_flame(W))));
    add("PFEIL_ROT", Brandschutz, "Richtungsangabe (Brandschutz)", fire(&arrow(W)));
    add("PFEIL_ROT_DIAG", Brandschutz, "Richtungsangabe schräg (Brandschutz)", fire(&diag_arrow(W)));
    add("F_LOESCHDECKE", Brandschutz, "Löschdecke", fire(&format!("<rect x='22' y='14' width='44' height='70' fill='{W}'/><path d='M30 84 L30 92 M58 84 L58 92' stroke='{W}' stroke-width='5'/>{}", small_flame(W))));

    // --- Feuerwehrpläne (DIN 14034-6, eigene Darstellung) ---
    for (id, t, name) in [
        ("FW_BMZ", "BMZ", "Brandmelderzentrale"),
        ("FW_FAT", "FAT", "Feuerwehr-Anzeigetableau"),
        ("FW_FBF", "FBF", "Feuerwehr-Bedienfeld"),
        ("FW_FSD", "FSD", "Feuerwehr-Schlüsseldepot"),
        ("FW_FSE", "FSE", "Freischaltelement"),
        ("FW_FIZ", "FIZ", "Feuerwehr-Informationszentrale"),
        ("FW_FGB", "FGB", "Feuerwehr-Gebäudefunkbedienfeld"),
        ("FW_SPZ", "SPZ", "Sprinklerzentrale"),
        ("FW_RWA", "RWA", "Rauch- und Wärmeabzug, Bedienstelle"),
        ("FW_NSHV", "NSHV", "Niederspannungshauptverteilung"),
        ("FW_T30", "T30", "Feuerschutztür T30"),
        ("FW_T90", "T90", "Feuerschutztür T90"),
        ("FW_RS", "RS", "Rauchschutztür"),
        ("FW_PV", "PV", "Photovoltaikanlage"),
        ("FW_EDV", "EDV", "Elektronische Datenverarbeitung"),
        ("FW_AZ", "AZ", "Aufzug"),
        ("FW_FWAZ", "FwA", "Feuerwehraufzug"),
        ("FW_NEA", "NEA", "Netzersatzanlage / Notstrom"),
        ("FW_CO2", "CO2", "CO2-Löschanlage"),
        ("FW_GAS", "GAS", "Gaslöschanlage"),
    ] {
        add(id, Feuerwehr, name, fw_text(t, R));
    }
    add("FW_INFO", Feuerwehr, "Information für die Feuerwehr", fw_box(&format!("<circle cx='50' cy='24' r='9' fill='{R}'/><rect x='42' y='40' width='16' height='44' fill='{R}'/>"), R));
    add("FW_HAUPTSCHALTER", Feuerwehr, "Hauptschalter (Elektro)", fw_box(&switch_sym(R), R));
    add("FW_GASHAHN", Feuerwehr, "Hinweis auf Gashaupthahn", format!("<rect x='4' y='4' width='92' height='92' fill='#f08c00' stroke='{K}' stroke-width='4'/>{}", label("Gas", K)));
    add("FW_WASSERHAHN", Feuerwehr, "Hinweis auf Wasserhaupthahn", format!("<rect x='4' y='4' width='92' height='92' fill='{B}' stroke='{K}' stroke-width='4'/>{}", label("H2O", W)));
    add("FW_UEBERFLUR", Feuerwehr, "Überflurhydrant", hydrant_over(B));
    add("FW_UNTERFLUR", Feuerwehr, "Unterflurhydrant", hydrant_under(B));
    add("FW_WANDHYDRANT", Feuerwehr, "Wandhydrant", fw_box(&hose_reel(B, W), B));
    add("FW_EINSPEISUNG", Feuerwehr, "Löschwasser-Einspeisung (B-Anschluss)", fw_box(&format!("<circle cx='50' cy='50' r='30' fill='none' stroke='{B}' stroke-width='6'/>{}", label("B", B)), B));
    add("FW_STEIG_TROCKEN", Feuerwehr, "Steigleitung trocken", fw_box(&format!("<path d='M50 88 L50 16 M36 30 L50 14 L64 30' stroke='{B}' stroke-width='7' fill='none'/><text x='74' y='86' font-family='Arial' font-size='20' fill='{B}'>tr</text>"), B));
    add("FW_STEIG_NASS", Feuerwehr, "Steigleitung nass", fw_box(&format!("<path d='M50 88 L50 16 M36 30 L50 14 L64 30' stroke='{B}' stroke-width='7' fill='none'/><text x='70' y='86' font-family='Arial' font-size='20' fill='{B}'>na</text>"), B));
    add("FW_BRUNNEN", Feuerwehr, "Löschwasserbrunnen", fw_box(&format!("<path d='M24 30 L24 84 L76 84 L76 30' fill='none' stroke='{B}' stroke-width='7'/><path d='M30 60 L70 60' stroke='{B}' stroke-width='4' stroke-dasharray='6 4'/><path d='M50 14 L50 60' stroke='{B}' stroke-width='5'/>"), B));
    add("FW_BEHAELTER", Feuerwehr, "Löschwasserbehälter / -teich", fw_box(&format!("<rect x='16' y='30' width='68' height='50' fill='none' stroke='{B}' stroke-width='7'/><path d='M20 50 C30 44 40 56 50 50 C60 44 70 56 80 50' stroke='{B}' stroke-width='5' fill='none'/>"), B));
    add("FW_SPRINKLER", Feuerwehr, "Sprinkleranlage", fw_box(&sprinkler(B), B));
    add("FW_RAUCHABZUG", Feuerwehr, "Rauchabzug", fw_box(&smoke(R), R));
    add("FW_TREPPENRAUM", Feuerwehr, "Treppenraum (notwendige Treppe)", fw_box(&stairs(R), R));
    add("FW_BRANDWAND", Feuerwehr, "Brandwand", fw_box(&format!("<rect x='38' y='12' width='24' height='76' fill='{R}'/><path d='M26 12 L26 88 M74 12 L74 88' stroke='{R}' stroke-width='6'/>"), R));
    add("FW_ANLEITER", Feuerwehr, "Anleiterstelle", fw_box(&format!("<path d='M20 20 L80 20' stroke='{R}' stroke-width='8'/><path d='M50 20 L50 30' stroke='{R}' stroke-width='6'/>"), R));
    add("FW_HAUPTZUGANG", Feuerwehr, "Hauptzugang Feuerwehr", format!("<polygon points='6,94 50,6 94,94' fill='{K}'/>"));
    add("FW_ZUGANG", Feuerwehr, "Gebäudezugang", format!("<polygon points='26,80 50,30 74,80' fill='{K}'/>"));
    add("FW_ZUFAHRT", Feuerwehr, "Hauptzufahrt Feuerwehr", arrow(G));
    add("FW_NEBENZUFAHRT", Feuerwehr, "Nebenzufahrt Feuerwehr", format!("<polygon points='10,44 62,44 62,28 92,50 62,72 62,56 10,56' fill='{G}'/>"));
    add("FW_HOEHE", Feuerwehr, "Durchfahrtshöhe", format!("<circle cx='50' cy='50' r='46' fill='{W}' stroke='{R}' stroke-width='8'/><polygon points='50,12 40,26 60,26' fill='{K}'/><polygon points='50,88 40,74 60,74' fill='{K}'/>{}", label("3,5", K).replace("font-size='36'", "font-size='26'")));
    add("FW_GEWICHT", Feuerwehr, "Gewicht maximal", format!("<circle cx='50' cy='50' r='46' fill='{W}' stroke='{R}' stroke-width='8'/>{}", label("16t", K)));
    add("FW_GEFAHR", Feuerwehr, "Gefahrenstelle", warning(&exclam(K)));
    add("FW_SPANNUNG", Feuerwehr, "Elektrische Spannung", warning(&bolt(K)));
    add("FW_GEFAHRSTOFF", Feuerwehr, "Feuergefährliche Stoffe", warning(&flame(K)));
    add("FW_EXPLOSION", Feuerwehr, "Explosionsfähige Atmosphäre", warning(&label("EX", K)));
    add("FW_RADIOAKTIV", Feuerwehr, "Radioaktive Stoffe", warning(&trefoil(K)));
    add("FW_GIFTIG", Feuerwehr, "Giftige Stoffe", warning(&skull(K)));
    add("FW_GASFLASCHEN", Feuerwehr, "Gasflaschen", warning(&gas_bottle(K)));
    add("FW_BATTERIE", Feuerwehr, "Batteriespeicher", fw_box(&battery(R), R));
    add("FW_LADESTATION", Feuerwehr, "E-Ladestation", fw_box(&format!("<rect x='26' y='14' width='34' height='70' rx='4' fill='none' stroke='{R}' stroke-width='6'/><polygon points='46,26 34,50 44,50 38,70 54,44 44,44 50,26' fill='{R}'/><path d='M60 40 L74 40 L74 76 L84 76' fill='none' stroke='{R}' stroke-width='5'/>"), R));
    add("FW_TRAFO", Feuerwehr, "Transformator", fw_box(&format!("<circle cx='38' cy='50' r='20' fill='none' stroke='{R}' stroke-width='6'/><circle cx='62' cy='50' r='20' fill='none' stroke='{R}' stroke-width='6'/>"), R));
    add("FW_LUEFTUNG", Feuerwehr, "Lüftungsanlage (Abschaltung)", fw_box(&format!("<circle cx='50' cy='50' r='34' fill='none' stroke='{R}' stroke-width='6'/><path d='M50 50 C40 30 58 22 62 30 Z M50 50 C70 46 72 66 64 68 Z M50 50 C42 68 26 60 30 52 Z' fill='{R}'/>"), R));
    add("FW_SCHLUESSELROHR", Feuerwehr, "Schlüsselrohr / Schlüsselkasten", fw_box(&format!("<circle cx='34' cy='50' r='14' fill='none' stroke='{R}' stroke-width='6'/><path d='M48 50 L86 50 M76 50 L76 62 M66 50 L66 60' stroke='{R}' stroke-width='6'/>"), R));
    add("FW_POLLER", Feuerwehr, "Poller (herausnehmbar)", format!("<circle cx='50' cy='50' r='26' fill='{K}'/><circle cx='50' cy='50' r='10' fill='{W}'/>"));
    add("FW_SCHRANKE", Feuerwehr, "Schranke / Tor", format!("<rect x='6' y='44' width='88' height='12' fill='{W}' stroke='{R}' stroke-width='4'/><rect x='24' y='44' width='14' height='12' fill='{R}'/><rect x='56' y='44' width='14' height='12' fill='{R}'/><circle cx='8' cy='50' r='8' fill='{K}'/>"));
    add("FW_ABWASSER", Feuerwehr, "Abwasser-Absperrschieber", fw_box(&format!("<path d='M14 50 L86 50' stroke='{B}' stroke-width='8'/><polygon points='34,30 66,70 66,30 34,70' fill='{B}'/>"), B));
    add("FW_RUECKHALT", Feuerwehr, "Löschwasser-Rückhaltung", fw_box(&format!("<path d='M16 30 L16 80 L84 80 L84 30' fill='none' stroke='{B}' stroke-width='7'/><path d='M50 18 L50 58 M38 46 L50 60 L62 46' stroke='{B}' stroke-width='6' fill='none'/>"), B));
    add("FW_SAMMELSTELLE", Feuerwehr, "Sammelstelle", rescue(&format!("<g transform='translate(20,20) scale(0.6)'>{}</g><polygon points='8,8 30,14 14,30' fill='{W}'/><polygon points='92,92 70,86 86,70' fill='{W}'/>", standing_man(W))));

    // --- Brandmeldetechnik / Laufkarten ---
    add("BMT_RAUCH", Brandmeldetechnik, "Automatischer Rauchmelder", fw_box(&smoke(R), R));
    add("BMT_WAERME", Brandmeldetechnik, "Wärmemelder", fw_box(&thermometer(R), R));
    add("BMT_FLAMME", Brandmeldetechnik, "Flammenmelder", fw_box(&format!("<g transform='translate(15,12) scale(0.7)'>{}</g>", flame(R)), R));
    add("BMT_HAND", Brandmeldetechnik, "Handfeuermelder (Druckknopfmelder)", fw_box(&press_button(R), R));
    add("BMT_MULTI", Brandmeldetechnik, "Mehrkriterienmelder", fw_text("OT", R));
    add("BMT_LINIE", Brandmeldetechnik, "Linienförmiger Rauchmelder", fw_box(&format!("<path d='M10 50 L90 50' stroke='{R}' stroke-width='6' stroke-dasharray='10 6'/><polygon points='10,38 22,50 10,62' fill='{R}'/><polygon points='90,38 78,50 90,62' fill='{R}'/>"), R));
    add("BMT_ANSAUG", Brandmeldetechnik, "Ansaugrauchmelder", fw_text("ASD", R));
    add("BMT_ZD", Brandmeldetechnik, "Melder in Zwischendecke", fw_box(&format!("{}<path d='M10 24 L90 24' stroke='{R}' stroke-width='5'/>", label("ZD", R)), R));
    add("BMT_DB", Brandmeldetechnik, "Melder im Doppelboden", fw_box(&format!("{}<path d='M10 76 L90 76' stroke='{R}' stroke-width='5'/>", label("DB", R)), R));
    add("BMT_PARALLEL", Brandmeldetechnik, "Parallelanzeige", fw_box(&format!("<circle cx='50' cy='50' r='20' fill='{R}'/><circle cx='50' cy='50' r='34' fill='none' stroke='{R}' stroke-width='4'/>"), R));
    add("BMT_BLITZ", Brandmeldetechnik, "Blitzleuchte", fw_box(&beacon(R), R));
    add("BMT_SIRENE", Brandmeldetechnik, "Akustischer Signalgeber", fw_box(&horn(R), R));
    add("BMT_START", Brandmeldetechnik, "Laufweg-Beginn (FIZ/BMZ)", format!("<circle cx='50' cy='50' r='44' fill='{R}'/>{}", label("S", W)));
    add("BMT_ZIEL", Brandmeldetechnik, "Melderstandort (Ziel)", format!("<circle cx='50' cy='50' r='44' fill='{W}' stroke='{R}' stroke-width='8'/><circle cx='50' cy='50' r='24' fill='{R}'/>"));

    // --- Warnzeichen ---
    add("W001", Warnung, "Allgemeines Warnzeichen", warning(&exclam(K)));
    add("W012", Warnung, "Warnung vor elektrischer Spannung", warning(&bolt(K)));
    add("W021", Warnung, "Warnung vor feuergefährlichen Stoffen", warning(&flame(K)));
    add("W002", Warnung, "Warnung vor explosionsgefährlichen Stoffen", warning(&burst(K)));
    add("W003", Warnung, "Warnung vor radioaktiven Stoffen", warning(&trefoil(K)));
    add("W016", Warnung, "Warnung vor giftigen Stoffen", warning(&skull(K)));
    add("W023", Warnung, "Warnung vor ätzenden Stoffen", warning(&drops_hand(K)));
    add("W029", Warnung, "Warnung vor Gasflaschen", warning(&gas_bottle(K)));
    add("W011", Warnung, "Warnung vor Rutschgefahr", warning(&slip(K)));
    add("W014", Warnung, "Warnung vor Flurförderzeugen", warning(&forklift(K)));
    add("W009", Warnung, "Warnung vor Biogefährdung", warning(&biohazard(K)));
    add("W017", Warnung, "Warnung vor heißer Oberfläche", warning(&waves(K)));
    add("W004", Warnung, "Warnung vor Laserstrahl", warning(&laser(K)));
    add("W006", Warnung, "Warnung vor magnetischem Feld", warning(&magnet(K)));
    add("W026", Warnung, "Warnung vor Gefahren durch Batterien", warning(&battery(K)));

    // --- Gebotszeichen ---
    add("M001", Gebot, "Allgemeines Gebotszeichen", mandatory(&exclam(W)));
    add("M004", Gebot, "Augenschutz benutzen", mandatory(&format!("{}<path d='M29 40 C29 37 31 36 34 36 L66 36 C69 36 71 37 71 40 L71 50 C71 53 69 55 66 55 L56 55 C53 55 52 50 50 50 C48 50 47 55 44 55 L34 55 C31 55 29 53 29 50 Z' fill='{W}' stroke='{B}' stroke-width='3.5'/><path d='M29 44 L22 46 M71 44 L78 46' stroke='{B}' stroke-width='3.5'/>", bust(W))));
    add("M003", Gebot, "Gehörschutz benutzen", mandatory(&format!("{}<path d='M25 50 C25 16 75 16 75 50' fill='none' stroke='{B}' stroke-width='12'/><path d='M25 50 C25 16 75 16 75 50' fill='none' stroke='{W}' stroke-width='5'/><rect x='17' y='38' width='15' height='24' rx='6' fill='{W}' stroke='{B}' stroke-width='3'/><rect x='68' y='38' width='15' height='24' rx='6' fill='{W}' stroke='{B}' stroke-width='3'/>", bust(W))));
    add("M014", Gebot, "Kopfschutz benutzen", mandatory(&format!("{}<path d='M28 38 C28 18 38 10 50 10 C62 10 72 18 72 38 Z' fill='{W}' stroke='{B}' stroke-width='3'/><rect x='20' y='35' width='60' height='7' rx='3' fill='{W}' stroke='{B}' stroke-width='3'/><path d='M50 11 L50 36' stroke='{B}' stroke-width='3'/>", bust(W))));
    add("M008", Gebot, "Fußschutz benutzen", mandatory(&format!("<g transform='translate(12,10) scale(0.78)'>{}</g>", boot(W))));
    add("M009", Gebot, "Handschutz benutzen", mandatory(&format!("<g transform='translate(12,10) scale(0.78)'>{}</g>", glove(W))));
    add("M002", Gebot, "Gebrauchsanweisung beachten", mandatory(&format!("<g transform='translate(10,10) scale(0.8)'>{}</g>", book(W))));
    add("M010", Gebot, "Schutzkleidung benutzen", mandatory(&format!("<g transform='translate(50,52) scale(0.92)'><circle cx='0' cy='-37' r='9' fill='{W}'/><path d='M-14 -26 H14 L18 22 H-18 Z' fill='{W}' stroke='{W}' stroke-width='4' stroke-linejoin='round'/><path d='M-16 -22 L-22 8 M16 -22 L22 8' stroke='{W}' stroke-width='8' stroke-linecap='round'/><path d='M-6 22 L-6 40 M6 22 L6 40' stroke='{W}' stroke-width='9' stroke-linecap='round'/><path d='M0 -24 L0 20' stroke='{B}' stroke-width='2.5'/></g>")));
    add("M017", Gebot, "Atemschutz benutzen", mandatory(&format!("{}<path d='M36 50 L64 50 L62 64 C58 71 42 71 38 64 Z' fill='{W}' stroke='{B}' stroke-width='3.5' stroke-linejoin='round'/><path d='M44 60 L56 60' stroke='{B}' stroke-width='2.5'/><circle cx='31' cy='62' r='7' fill='{W}' stroke='{B}' stroke-width='3'/><circle cx='69' cy='62' r='7' fill='{W}' stroke='{B}' stroke-width='3'/><path d='M33 45 L24 40 M67 45 L76 40' stroke='{B}' stroke-width='3'/>", bust(W))));
    add("M015", Gebot, "Warnweste benutzen", mandatory(&format!("<path d='M34 14 L44 14 L50 34 L56 14 L66 14 L78 26 L74 86 L26 86 L22 26 Z' fill='{W}'/><path d='M24 52 L76 52 M25 66 L75 66' stroke='{B}' stroke-width='5'/><path d='M50 34 L50 86' stroke='{B}' stroke-width='2.5'/>")));

    // --- Verbotszeichen ---
    add("P001", Verbot, "Allgemeines Verbotszeichen", prohibition(""));
    add("P002", Verbot, "Rauchen verboten", prohibition(&cigarette(K)));
    add("P003", Verbot, "Keine offene Flamme", prohibition(&format!("<path d='M34 82 L58 46' stroke='{K}' stroke-width='7' stroke-linecap='round'/><ellipse cx='60' cy='43' rx='6' ry='8' transform='rotate(34 60 43)' fill='{K}'/><g transform='translate(48,10) scale(0.32)'>{}</g>", flame(K))));
    add("P004", Verbot, "Für Fußgänger verboten", prohibition(&format!("<g transform='translate(20,14) scale(0.6)'>{}</g>", standing_man(K))));
    add("P020", Verbot, "Aufzug im Brandfall nicht benutzen", prohibition(&format!("<g transform='translate(20,14) scale(0.6)'>{}</g>", elevator(K))));
    add("P006", Verbot, "Für Flurförderzeuge verboten", prohibition(&format!("<g transform='translate(18,18) scale(0.62)'>{}</g>", forklift(K))));
    add("P011", Verbot, "Mit Wasser löschen verboten", prohibition(&format!("<g transform='rotate(35 63 34)'><path d='M53 26 L73 26 L70 44 L56 44 Z' fill='{K}'/><path d='M53 26 C53 19 73 19 73 26' fill='none' stroke='{K}' stroke-width='2.5'/></g><path d='M50 42 C47.5 47 47.5 49 50 50.5 C52.5 49 52.5 47 50 42 Z M43 50 C40.5 55 40.5 57 43 58.5 C45.5 57 45.5 55 43 50 Z' fill='{K}'/><g transform='translate(23,52) scale(0.3)'>{}</g>", flame(K))));
    add("P013", Verbot, "Mobiltelefone verboten", prohibition(&format!("<rect x='38' y='22' width='24' height='54' rx='4' fill='{K}'/><rect x='42' y='28' width='16' height='30' fill='{W}'/>")));
    add("P010", Verbot, "Berühren verboten", prohibition(&format!("<g transform='translate(18,16) scale(0.64)'>{}</g>", hand(K))));
    add("P023", Verbot, "Abstellen oder Lagern verboten", prohibition(&format!("<rect x='30' y='44' width='40' height='30' fill='{K}'/><rect x='38' y='26' width='26' height='18' fill='{K}'/>")));
    add("P005", Verbot, "Kein Trinkwasser", prohibition(&format!("<path d='M30 30 L56 30 L56 40 L62 40 L62 50 L56 50 L56 46 L30 46 Z' fill='{K}'/><g transform='translate(48,52) scale(0.2)'>{}</g>", drop(K))));

    // --- GHS ---
    add("GHS01", Gefahrstoffe, "GHS01 Explodierende Bombe", ghs(&burst(K)));
    add("GHS02", Gefahrstoffe, "GHS02 Flamme", ghs(&flame(K)));
    add("GHS03", Gefahrstoffe, "GHS03 Flamme über einem Kreis", ghs(&flame_circle(K)));
    add("GHS04", Gefahrstoffe, "GHS04 Gasflasche", ghs(&format!("<g transform='rotate(-60 50 50)'>{}</g>", gas_bottle(K))));
    add("GHS05", Gefahrstoffe, "GHS05 Ätzwirkung", ghs(&drops_hand(K)));
    add("GHS06", Gefahrstoffe, "GHS06 Totenkopf", ghs(&skull(K)));
    add("GHS07", Gefahrstoffe, "GHS07 Ausrufezeichen", ghs(&exclam(K)));
    add("GHS08", Gefahrstoffe, "GHS08 Gesundheitsgefahr", ghs(&health(K)));
    add("GHS09", Gefahrstoffe, "GHS09 Umwelt", ghs(&tree_fish(K)));

    // --- Gebäudebeschilderung ---
    let info = |inner: String| format!("<rect x='0' y='0' width='100' height='100' rx='6' fill='{B}'/>{inner}");
    add("GEB_WC_D", Gebaeude, "WC Damen", info(format!("<g transform='translate(14,10) scale(0.72)'><circle cx='50' cy='14' r='10' fill='{W}'/><polygon points='50,28 76,76 24,76' fill='{W}'/><path d='M40 76 L40 96 M60 76 L60 96' stroke='{W}' stroke-width='8'/></g>")));
    add("GEB_WC_H", Gebaeude, "WC Herren", info(format!("<g transform='translate(14,10) scale(0.72)'>{}</g>", standing_man(W))));
    add("GEB_WC_B", Gebaeude, "WC barrierefrei", info(format!("<circle cx='42' cy='20' r='8' fill='{W}'/><path d='M42 30 L42 56 L66 56 L74 80' fill='none' stroke='{W}' stroke-width='8'/><path d='M30 44 A24 24 0 1 0 64 74' fill='none' stroke='{W}' stroke-width='6'/>")));
    add("GEB_AUFZUG", Gebaeude, "Aufzug", info(elevator(W)));
    add("GEB_TREPPE", Gebaeude, "Treppe", info(stairs(W)));
    add("GEB_INFO", Gebaeude, "Information", info(format!("<circle cx='50' cy='22' r='10' fill='{W}'/><rect x='41' y='38' width='18' height='48' fill='{W}'/>")));
    add("GEB_PARKEN", Gebaeude, "Parkplatz", info(label("P", W)));
    add("GEB_RAUCHER", Gebaeude, "Raucherbereich", info(cigarette(W)));

    // --- Verkehrszeichen / Zufahrt (eigene Darstellung) ---
    let ring = |inner: &str| format!("<circle cx='50' cy='50' r='47' fill='{W}' stroke='{R}' stroke-width='11'/>{inner}");
    add("VZ_FW_ZUFAHRT", Verkehr, "Feuerwehrzufahrt (Schild)", format!(
        "<rect x='2' y='22' width='96' height='56' fill='{W}' stroke='{R}' stroke-width='6'/>\
         <text x='50' y='46' font-family='Arial' font-weight='bold' font-size='15' text-anchor='middle' fill='{R}'>Feuerwehr-</text>\
         <text x='50' y='66' font-family='Arial' font-weight='bold' font-size='15' text-anchor='middle' fill='{R}'>zufahrt</text>"
    ));
    add("VZ_FW_AUFSTELL", Verkehr, "Fläche für die Feuerwehr (Schild)", format!(
        "<rect x='2' y='22' width='96' height='56' fill='{W}' stroke='{R}' stroke-width='6'/>\
         <text x='50' y='46' font-family='Arial' font-weight='bold' font-size='13' text-anchor='middle' fill='{R}'>Fläche für die</text>\
         <text x='50' y='66' font-family='Arial' font-weight='bold' font-size='13' text-anchor='middle' fill='{R}'>Feuerwehr</text>"
    ));
    add("VZ_DURCHFAHRT_VERBOTEN", Verkehr, "Verbot für Fahrzeuge aller Art", ring(""));
    add("VZ_EINFAHRT_VERBOTEN", Verkehr, "Verbot der Einfahrt", format!("<circle cx='50' cy='50' r='47' fill='{R}'/><rect x='18' y='42' width='64' height='16' fill='{W}'/>"));
    add("VZ_HALTVERBOT", Verkehr, "Absolutes Haltverbot", format!("<circle cx='50' cy='50' r='47' fill='{B}' stroke='{R}' stroke-width='9'/><path d='M22 22 L78 78 M78 22 L22 78' stroke='{R}' stroke-width='9'/>"));
    add("VZ_HOEHE", Verkehr, "Verbot über angegebene Höhe", ring(&format!("<polygon points='50,14 42,26 58,26' fill='{K}'/><polygon points='50,86 42,74 58,74' fill='{K}'/><text x='50' y='59' font-family='Arial' font-weight='bold' font-size='26' text-anchor='middle' fill='{K}'>3,5</text>")));
    add("VZ_GEWICHT", Verkehr, "Verbot über angegebenes Gewicht", ring(&format!("<text x='50' y='60' font-family='Arial' font-weight='bold' font-size='30' text-anchor='middle' fill='{K}'>7,5t</text>")));
    add("VZ_EINBAHN", Verkehr, "Einbahnstraße", format!("<rect x='2' y='30' width='96' height='40' fill='{B}'/><polygon points='12,44 66,44 66,36 88,50 66,64 66,56 12,56' fill='{W}'/>"));
    add("VZ_PARKEN", Verkehr, "Parken", format!("<rect x='8' y='8' width='84' height='84' rx='8' fill='{B}'/>{}", label("P", W)));
    add("VZ_STOP", Verkehr, "Halt! Vorfahrt gewähren", format!("<polygon points='30,4 70,4 96,30 96,70 70,96 30,96 4,70 4,30' fill='{R}'/><text x='50' y='60' font-family='Arial' font-weight='bold' font-size='26' text-anchor='middle' fill='{W}'>STOP</text>"));
    add("VZ_SACKGASSE", Verkehr, "Sackgasse", format!("<rect x='10' y='10' width='80' height='80' fill='{B}'/><rect x='42' y='38' width='16' height='44' fill='{W}'/><rect x='24' y='22' width='52' height='16' fill='{R}'/>"));

    // --- Nordpfeile ---
    add("N1", Nordpfeile, "Nordpfeil (klassisch)", north(0));
    add("N2", Nordpfeile, "Nordpfeil (Kreis)", north(1));
    add("N3", Nordpfeile, "Windrose", north(2));
    add("N4", Nordpfeile, "Nordpfeil (einfach)", north(3));

    // --- Einrichtung (Draufsicht) ---
    add("EIN_TISCH", Einrichtung, "Tisch", outline("<rect x='10' y='25' width='80' height='50'/>"));
    add("EIN_STUHL", Einrichtung, "Stuhl", outline("<rect x='25' y='30' width='50' height='45' rx='4'/><rect x='25' y='20' width='50' height='10'/>"));
    add("EIN_SCHREIBTISCH", Einrichtung, "Schreibtisch mit Stuhl", outline("<rect x='5' y='10' width='90' height='45'/><rect x='35' y='62' width='30' height='28' rx='4'/>"));
    add("EIN_BESPRECHUNG", Einrichtung, "Besprechungstisch", outline("<rect x='20' y='30' width='60' height='40'/><rect x='24' y='14' width='16' height='12'/><rect x='60' y='14' width='16' height='12'/><rect x='24' y='74' width='16' height='12'/><rect x='60' y='74' width='16' height='12'/>"));
    add("EIN_BETT", Einrichtung, "Bett", outline("<rect x='20' y='5' width='60' height='90'/><rect x='28' y='10' width='44' height='16' rx='4'/><path d='M20 34 L80 34'/>"));
    add("EIN_WC", Einrichtung, "WC", outline("<rect x='28' y='8' width='44' height='18'/><ellipse cx='50' cy='58' rx='20' ry='30'/>"));
    add("EIN_WASCHBECKEN", Einrichtung, "Waschbecken", outline("<rect x='15' y='20' width='70' height='55' rx='6'/><ellipse cx='50' cy='50' rx='24' ry='16'/><circle cx='50' cy='28' r='3'/>"));
    add("EIN_URINAL", Einrichtung, "Urinal", outline("<path d='M25 15 L75 15 L70 60 C66 80 34 80 30 60 Z'/>"));
    add("EIN_DUSCHE", Einrichtung, "Dusche", outline("<rect x='10' y='10' width='80' height='80'/><path d='M10 10 L90 90 M90 10 L10 90'/><circle cx='50' cy='50' r='5'/>"));
    add("EIN_BADEWANNE", Einrichtung, "Badewanne", outline("<rect x='5' y='25' width='90' height='50' rx='6'/><rect x='12' y='31' width='76' height='38' rx='16'/>"));
    add("EIN_SPUELE", Einrichtung, "Spüle", outline("<rect x='5' y='25' width='90' height='50'/><rect x='12' y='32' width='34' height='36' rx='4'/><rect x='54' y='32' width='34' height='36' rx='4'/>"));
    add("EIN_HERD", Einrichtung, "Herd", outline("<rect x='10' y='10' width='80' height='80'/><circle cx='32' cy='32' r='12'/><circle cx='68' cy='32' r='12'/><circle cx='32' cy='68' r='12'/><circle cx='68' cy='68' r='12'/>"));
    add("EIN_KUEHLSCHRANK", Einrichtung, "Kühlschrank", outline("<rect x='15' y='15' width='70' height='70'/><path d='M15 15 L85 85'/><text x='50' y='60' font-family='Arial' font-size='22' text-anchor='middle'>KS</text>"));
    add("EIN_SCHRANK", Einrichtung, "Schrank", outline("<rect x='5' y='30' width='90' height='40'/><path d='M5 30 L95 70 M5 70 L95 30'/>"));
    add("EIN_REGAL", Einrichtung, "Regal", outline("<rect x='5' y='35' width='90' height='30'/><path d='M35 35 L35 65 M65 35 L65 65'/>"));
    add("EIN_SOFA", Einrichtung, "Sofa", outline("<rect x='5' y='30' width='90' height='45' rx='6'/><rect x='5' y='30' width='90' height='12'/><path d='M5 42 L5 75 M95 42 L95 75 M50 42 L50 75'/>"));
    add("EIN_PFLANZE", Einrichtung, "Pflanze", outline("<circle cx='50' cy='50' r='34'/><path d='M50 16 L50 84 M16 50 L84 50 M26 26 L74 74 M74 26 L26 74'/>"));
    add("EIN_PKW", Einrichtung, "PKW", outline("<rect x='28' y='5' width='44' height='90' rx='14'/><rect x='34' y='22' width='32' height='16' rx='4'/><rect x='34' y='66' width='32' height='12' rx='4'/>"));
    add("EIN_LKW", Einrichtung, "LKW", outline("<rect x='30' y='3' width='40' height='22' rx='4'/><rect x='28' y='28' width='44' height='69'/>"));
    add("EIN_STAPLER", Einrichtung, "Gabelstapler", outline("<rect x='30' y='30' width='40' height='55' rx='6'/><path d='M36 30 L36 5 M64 30 L64 5'/>"));
    add("EIN_DRUCKER", Einrichtung, "Drucker / Kopierer", outline("<rect x='10' y='25' width='80' height='55'/><rect x='25' y='10' width='50' height='15'/><path d='M25 80 L25 92 L75 92 L75 80'/>"));
    add("EIN_TUER", Einrichtung, "Türflügel (Symbol)", outline("<path d='M10 90 L10 10'/><path d='M10 10 A80 80 0 0 1 90 90'/><path d='M10 90 L90 90'/>"));
    add("EIN_AUFZUG", Einrichtung, "Aufzugsschacht", outline("<rect x='10' y='10' width='80' height='80'/><path d='M10 10 L90 90 M90 10 L10 90'/>"));
    // Gewerbe / Produktion / Lager
    add("EIN_DRUCKMASCHINE", Einrichtung, "Druckmaschine", outline("<rect x='3' y='30' width='94' height='40'/><path d='M22 30 L22 70 M41 30 L41 70 M60 30 L60 70 M79 30 L79 70'/><circle cx='12' cy='50' r='5'/><circle cx='88' cy='50' r='5'/>"));
    add("EIN_MASCHINE", Einrichtung, "Maschine (allgemein)", outline("<rect x='10' y='20' width='80' height='60'/><rect x='18' y='28' width='26' height='18'/><circle cx='68' cy='56' r='12'/>"));
    add("EIN_PAPIERROLLE", Einrichtung, "Papierrolle (liegend)", outline("<rect x='10' y='30' width='80' height='40'/><ellipse cx='10' cy='50' rx='6' ry='20'/><circle cx='10' cy='50' r='4'/>"));
    add("EIN_PALETTE", Einrichtung, "Palette", outline("<rect x='10' y='20' width='80' height='60'/><path d='M10 35 L90 35 M10 50 L90 50 M10 65 L90 65'/>"));
    add("EIN_PALETTENREGAL", Einrichtung, "Palettenregal", outline("<rect x='3' y='30' width='94' height='40'/><path d='M34 30 L34 70 M65 30 L65 70 M3 30 L34 70 M34 30 L65 70 M65 30 L97 70'/>"));
    add("EIN_HUBWAGEN", Einrichtung, "Hubwagen", outline("<path d='M30 85 L30 25 M50 85 L50 25'/><rect x='25' y='5' width='30' height='20' rx='4'/><circle cx='40' cy='12' r='3'/>"));
    add("EIN_WERKBANK", Einrichtung, "Werkbank", outline("<rect x='5' y='30' width='90' height='40'/><rect x='70' y='34' width='20' height='12'/><path d='M5 60 L95 60'/>"));
    add("EIN_SCHNEIDEMASCHINE", Einrichtung, "Schneidemaschine", outline("<rect x='15' y='15' width='70' height='70'/><path d='M15 50 L85 50' stroke-dasharray='6 4'/><rect x='35' y='5' width='30' height='10'/>"));
    add("EIN_FOERDERBAND", Einrichtung, "Förderband", outline("<rect x='3' y='38' width='94' height='24' rx='12'/><circle cx='15' cy='50' r='6'/><circle cx='85' cy='50' r='6'/>"));
    add("EIN_ROLLTOR", Einrichtung, "Rolltor / Sektionaltor", outline("<path d='M5 50 L95 50' stroke-width='6'/><path d='M5 40 L5 60 M95 40 L95 60'/><path d='M15 62 L85 62' stroke-dasharray='5 4'/>"));
    add("EIN_RAMPE", Einrichtung, "Laderampe", outline("<rect x='10' y='20' width='80' height='60'/><path d='M10 80 L90 20'/><path d='M60 70 L80 70 L80 50' fill='none'/>"));
    add("EIN_CONTAINER", Einrichtung, "Abfallcontainer", outline("<rect x='10' y='25' width='80' height='50'/><path d='M10 25 L90 75 M90 25 L10 75'/>"));
    add("EIN_MUELLTONNE", Einrichtung, "Mülltonne", outline("<rect x='25' y='20' width='50' height='60' rx='4'/><path d='M25 32 L75 32'/>"));
    add("EIN_GITTERBOX", Einrichtung, "Gitterbox", outline("<rect x='15' y='20' width='70' height='60'/><path d='M15 40 L85 40 M15 60 L85 60 M38 20 L38 80 M62 20 L62 80'/>"));
    add("EIN_SPIND", Einrichtung, "Spind (Reihe)", outline("<rect x='5' y='35' width='90' height='30'/><path d='M23 35 L23 65 M41 35 L41 65 M59 35 L59 65 M77 35 L77 65'/>"));
    add("EIN_GARDEROBE", Einrichtung, "Garderobe", outline("<path d='M5 50 L95 50'/><path d='M15 50 L15 40 M35 50 L35 40 M55 50 L55 40 M75 50 L75 40'/><path d='M5 44 L5 56 M95 44 L95 56'/>"));
    add("EIN_BANK", Einrichtung, "Sitzbank", outline("<rect x='5' y='40' width='90' height='20'/>"));
    add("EIN_RUNDTISCH", Einrichtung, "Runder Tisch mit Stühlen", outline("<circle cx='50' cy='50' r='24'/><rect x='42' y='6' width='16' height='12' rx='3'/><rect x='42' y='82' width='16' height='12' rx='3'/><rect x='6' y='42' width='12' height='16' rx='3'/><rect x='82' y='42' width='12' height='16' rx='3'/>"));
    add("EIN_STUHLREIHE", Einrichtung, "Stuhlreihe", outline("<rect x='4' y='38' width='18' height='22' rx='3'/><rect x='28' y='38' width='18' height='22' rx='3'/><rect x='52' y='38' width='18' height='22' rx='3'/><rect x='76' y='38' width='18' height='22' rx='3'/>"));
    add("EIN_PULT", Einrichtung, "Rednerpult", outline("<path d='M30 30 L70 30 L65 75 L35 75 Z'/>"));
    add("EIN_BILDSCHIRM", Einrichtung, "Bildschirmarbeitsplatz", outline("<rect x='5' y='15' width='90' height='45'/><rect x='35' y='20' width='30' height='6'/><rect x='35' y='68' width='30' height='26' rx='4'/>"));
    add("EIN_SERVERSCHRANK", Einrichtung, "Serverschrank / Rack", outline("<rect x='25' y='15' width='50' height='70'/><path d='M25 28 L75 28 M25 41 L75 41 M25 54 L75 54 M25 67 L75 67'/>"));
    add("EIN_KOPFTEIL", Einrichtung, "Empfangstheke", outline("<path d='M5 30 L95 30 L95 70 L75 70 L75 45 L25 45 L25 70 L5 70 Z'/>"));
    add("EIN_SPUELMASCHINE", Einrichtung, "Geschirrspüler", outline("<rect x='15' y='15' width='70' height='70'/><text x='50' y='60' font-family='Arial' font-size='22' text-anchor='middle'>GS</text>"));
    add("EIN_KAFFEE", Einrichtung, "Kaffeeautomat", outline("<rect x='20' y='15' width='60' height='70'/><rect x='35' y='50' width='30' height='20'/><circle cx='50' cy='32' r='8'/>"));
    add("EIN_WASCHRINNE", Einrichtung, "Waschrinne", outline("<rect x='5' y='35' width='90' height='30' rx='6'/><circle cx='25' cy='42' r='3'/><circle cx='50' cy='42' r='3'/><circle cx='75' cy='42' r='3'/>"));
    add("EIN_FAHRRAD", Einrichtung, "Fahrradstellplatz", outline("<circle cx='25' cy='60' r='16'/><circle cx='75' cy='60' r='16'/><path d='M25 60 L45 35 L70 35 L75 60 M45 35 L55 60 L70 35'/>"));
    add("EIN_STELLPLATZ", Einrichtung, "PKW-Stellplatz", outline("<path d='M15 10 L15 90 M85 10 L85 90 M15 90 L85 90'/><text x='50' y='62' font-family='Arial' font-size='34' text-anchor='middle'>P</text>"));
    add("EIN_BAUM", Einrichtung, "Baum", outline("<circle cx='50' cy='50' r='40'/><circle cx='50' cy='50' r='4'/><path d='M50 10 C60 30 40 30 50 50 C60 70 40 70 50 90'/>"));
    add("EIN_SAEULE", Einrichtung, "Stütze / Säule", outline("<rect x='35' y='35' width='30' height='30'/><path d='M35 35 L65 65 M65 35 L35 65'/>"));

    v
}

pub fn library() -> &'static [SymbolDef] {
    static LIB: OnceLock<Vec<SymbolDef>> = OnceLock::new();
    LIB.get_or_init(build)
}

pub fn find(id: &str) -> Option<&'static SymbolDef> {
    library().iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alle_symbole_sind_gueltiges_svg() {
        let opt = resvg::usvg::Options::default();
        let mut ids = std::collections::HashSet::new();
        for s in library() {
            assert!(ids.insert(s.id), "doppelte ID {}", s.id);
            let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'>{}</svg>", s.svg);
            resvg::usvg::Tree::from_str(&svg, &opt).unwrap_or_else(|e| panic!("{}: {e}", s.id));
        }
        assert!(library().len() > 120);
    }
}

#[cfg(test)]
mod sichtpruefung {
    /// Alle Symbole als Übersichtsblatt (FLUCHTPLAN_TESTBILD=Ordner cargo test symbole_als_bild -- --ignored)
    #[test]
    #[ignore = "Sichtprüfung"]
    fn symbole_als_bild() {
        let Ok(dir) = std::env::var("FLUCHTPLAN_TESTBILD") else { return };
        let lib: Vec<_> = super::library().iter().filter(|s| s.id.len() == 4 && s.id[1..].chars().all(|c| c.is_ascii_digit())).collect();
        let mut cells = String::new();
        for (k, s) in lib.iter().enumerate() {
            let (x, y) = ((k % 10) as f32 * 120.0 + 10.0, (k / 10) as f32 * 135.0 + 10.0);
            cells += &format!("<g transform='translate({x},{y})'>{}<text x='50' y='118' font-size='12' font-family='Arial' text-anchor='middle'>{}</text></g>", s.svg, s.id);
        }
        let h = (lib.len() as f32 / 10.0).ceil() * 135.0 + 20.0;
        let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='1210' height='{h}'><rect width='100%' height='100%' fill='#ddd'/>{cells}</svg>");
        let tree = resvg::usvg::Tree::from_str(&svg, crate::export::usvg_options()).unwrap();
        let mut pm = resvg::tiny_skia::Pixmap::new(1210, h as u32).unwrap();
        resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pm.as_mut());
        pm.save_png(std::path::Path::new(&dir).join("symbole.png")).unwrap();
        // ausgewählte Zeichen groß
        let ids = std::env::var("FLUCHTPLAN_IDS").unwrap_or_default();
        let ids: Vec<&str> = ids.split(',').filter(|s| !s.is_empty()).collect();
        if !ids.is_empty() {
            let mut cells = String::new();
            for (k, id) in ids.iter().enumerate() {
                let s = super::find(id).unwrap();
                cells += &format!("<g transform='translate({},10) scale(3)'>{}</g>", k as f32 * 320.0 + 10.0, s.svg);
            }
            let w = ids.len() as u32 * 320 + 10;
            let svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='320'><rect width='100%' height='100%' fill='#ddd'/>{cells}</svg>");
            let tree = resvg::usvg::Tree::from_str(&svg, crate::export::usvg_options()).unwrap();
            let mut pm = resvg::tiny_skia::Pixmap::new(w, 320).unwrap();
            resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pm.as_mut());
            pm.save_png(std::path::Path::new(&dir).join("symbole_gross.png")).unwrap();
        }
    }
}
