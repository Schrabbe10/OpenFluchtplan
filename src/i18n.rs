//! Übersetzungen für mehrsprachige Legenden (Reihenfolge wie `Lang::ALL`).

use crate::model::Lang;

type T = [&'static str; 8];

static TABLE: &[(&str, T)] = &[
    ("route", ["Fluchtweg", "Escape route", "Voie d'évacuation", "Via di fuga", "Ruta de evacuación", "Droga ewakuacyjna", "Kaçış yolu", "Vluchtweg"]),
    ("area:fluchtbereich", ["Fluchtweg", "Escape route", "Voie d'évacuation", "Via di fuga", "Ruta de evacuación", "Droga ewakuacyjna", "Kaçış yolu", "Vluchtweg"]),
    ("sym:STANDORT", ["Standort", "You are here", "Vous êtes ici", "Voi siete qui", "Usted está aquí", "Jesteś tutaj", "Buradasınız", "U bent hier"]),
    ("sym:STANDORT_PFEIL", ["Standort", "You are here", "Vous êtes ici", "Voi siete qui", "Usted está aquí", "Jesteś tutaj", "Buradasınız", "U bent hier"]),
    ("sym:E001", ["Notausgang", "Emergency exit", "Sortie de secours", "Uscita di emergenza", "Salida de emergencia", "Wyjście ewakuacyjne", "Acil çıkış", "Nooduitgang"]),
    ("sym:E002", ["Notausgang", "Emergency exit", "Sortie de secours", "Uscita di emergenza", "Salida de emergencia", "Wyjście ewakuacyjne", "Acil çıkış", "Nooduitgang"]),
    ("sym:PFEIL_GRUEN", ["Richtungsangabe", "Direction", "Direction", "Direzione", "Dirección", "Kierunek", "Yön", "Richting"]),
    ("sym:E003", ["Erste Hilfe", "First aid", "Premiers secours", "Pronto soccorso", "Primeros auxilios", "Pierwsza pomoc", "İlk yardım", "Eerste hulp"]),
    ("sym:E004", ["Notruftelefon", "Emergency telephone", "Téléphone d'urgence", "Telefono di emergenza", "Teléfono de emergencia", "Telefon alarmowy", "Acil telefon", "Noodtelefoon"]),
    ("sym:E007", ["Sammelstelle", "Assembly point", "Point de rassemblement", "Punto di raccolta", "Punto de reunión", "Miejsce zbiórki", "Toplanma alanı", "Verzamelplaats"]),
    ("sym:E009", ["Arzt", "Doctor", "Médecin", "Medico", "Médico", "Lekarz", "Doktor", "Arts"]),
    ("sym:E010", ["Defibrillator (AED)", "Defibrillator (AED)", "Défibrillateur (DAE)", "Defibrillatore (DAE)", "Desfibrilador (DEA)", "Defibrylator (AED)", "Defibrilatör (OED)", "Defibrillator (AED)"]),
    ("sym:E011", ["Augenspüleinrichtung", "Eyewash station", "Douche oculaire", "Lavaocchi", "Lavaojos", "Płuczka do oczu", "Göz duşu", "Oogdouche"]),
    ("sym:E012", ["Notdusche", "Safety shower", "Douche de sécurité", "Doccia di emergenza", "Ducha de emergencia", "Prysznic bezpieczeństwa", "Acil duş", "Nooddouche"]),
    ("sym:E013", ["Krankentrage", "Stretcher", "Civière", "Barella", "Camilla", "Nosze", "Sedye", "Brancard"]),
    ("sym:F001", ["Feuerlöscher", "Fire extinguisher", "Extincteur", "Estintore", "Extintor", "Gaśnica", "Yangın söndürücü", "Brandblusser"]),
    ("sym:F002", ["Wandhydrant", "Fire hose", "Robinet d'incendie armé", "Idrante a muro", "Boca de incendio", "Hydrant wewnętrzny", "Yangın dolabı", "Brandslanghaspel"]),
    ("sym:F003", ["Feuerleiter", "Fire ladder", "Échelle d'incendie", "Scala antincendio", "Escalera de incendios", "Drabina pożarowa", "Yangın merdiveni", "Brandladder"]),
    ("sym:F004", ["Brandbekämpfungsgeräte", "Firefighting equipment", "Matériel d'incendie", "Attrezzature antincendio", "Equipos contra incendios", "Sprzęt gaśniczy", "Yangınla mücadele ekipmanı", "Brandbestrijdingsmiddelen"]),
    ("sym:F005", ["Brandmelder", "Fire alarm call point", "Déclencheur manuel", "Pulsante allarme incendio", "Pulsador de alarma", "Ręczny ostrzegacz pożarowy", "Yangın ihbar butonu", "Brandmelder"]),
    ("sym:F006", ["Brandmeldetelefon", "Fire emergency telephone", "Téléphone d'alarme incendie", "Telefono allarme incendio", "Teléfono de alarma", "Telefon alarmowy pożarowy", "Yangın ihbar telefonu", "Brandmeldtelefoon"]),
    ("sym:EIN_AUFZUG", ["Aufzug", "Lift", "Ascenseur", "Ascensore", "Ascensor", "Winda", "Asansör", "Lift"]),
    ("sym:GEB_TREPPE", ["Treppe", "Stairs", "Escalier", "Scala", "Escalera", "Schody", "Merdiven", "Trap"]),
    ("sym:P020", ["Aufzug im Brandfall nicht benutzen", "Do not use lift in case of fire", "Ne pas utiliser l'ascenseur en cas d'incendie", "Non usare l'ascensore in caso di incendio", "No usar el ascensor en caso de incendio", "Nie używać windy w razie pożaru", "Yangında asansörü kullanmayın", "Lift niet gebruiken bij brand"]),
    ("title", ["Legende", "Legend", "Légende", "Legenda", "Leyenda", "Legenda", "Açıklama", "Legenda"]),
];

pub fn translate(key: &str, lang: Lang) -> Option<&'static str> {
    TABLE.iter().find(|(k, _)| *k == key).map(|(_, t)| t[lang.idx()])
}
