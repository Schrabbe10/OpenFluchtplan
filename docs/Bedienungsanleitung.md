# OpenFluchtplan – Bedienungsanleitung

Freie Software (Unlicense) · Stand 11/2026 · Programmversion 0.1

Programm zum Erstellen von **Flucht- und Rettungsplänen** (DIN ISO 23601), **Feuerwehrplänen**
(DIN 14095), **Feuerwehr-Laufkarten** (DIN 14675), **Brandschutzplänen** und der
**Brandschutzordnung Teil A** (DIN 14096).

## Installation

Keine. `OpenFluchtplan.exe` auf den PC, ein Netzlaufwerk oder einen USB-Stick kopieren und starten.
Es werden keine Laufzeitumgebungen und keine Administratorrechte benötigt.
Einstellungen liegen unter `%APPDATA%\Fluchtplan\einstellungen.json`.

Optional für DWG: kostenloser *ODA File Converter* (opendesign.com) – wird automatisch erkannt.

## Arbeitsablauf

1. **Datei › Neu** – Planart und Format wählen (Flucht- und Rettungsplan A3 quer empfohlen,
   A4 nur für Einzelräume; Feuerwehrplan; Laufkarte; Brandschutzplan).
2. **Grundriss** – *Plan › Importieren* (PDF, PNG/JPG/BMP/TIF, DXF, DWG) oder per Drag & Drop
   auf das Fenster. Alternativ selbst zeichnen mit Wand, Raum, Tür, Fenster, Treppe.
   *Grundriss extrahieren* erkennt Wände automatisch aus einem Bild.
3. **Maßstab** – *Plan › Maßstab durch Entfernung*: zwei Punkte mit bekanntem Abstand anklicken
   und die reale Länge eingeben. Danach stimmen Maßlinien, Raumflächen und Kurvenradien.
4. **Fluchtwege** – *Einfügen › Fluchtwege* (Pfeile werden automatisch gesetzt) oder Flächen per
   *Polygon füllen* in einen von Wänden umschlossenen Bereich klicken.
5. **Symbole** – links aus der Bibliothek wählen (Suche oder Kategorie) und in den Plan klicken.
   Der **Standort** („Sie sind hier“) ist Pflicht.
6. **Planangaben** – rechts ausfüllen: Objekt, Anschrift, Geschoss, Sammelstelle, Verfasser, Stand.
7. **Legende** – entsteht automatisch. Texte, Reihenfolge und zusätzliche Sprachen über
   *Plan › Legende …*.
8. **Ausgabe** – *Datei › PDF erstellen* (optional mit Überdruck ENTWURF / MUSTER / VORABZUG),
   *Pläne zusammenführen* (mehrere offene Pläne in ein PDF), *Drucken*, Export als DXF/DWG/SVG/PNG.

**Vorhandene Pläne als PDF weiterbearbeiten** (z. B. CAD-Pläne eines Planungsbüros):
*Plan › Importieren › Grundriss aus PDF*, dann:
- „Als Vektorgrafik“ wählen. Linien, Flächen, Texte und Bilder werden einzeln bearbeitbar, jede Farbe
  bekommt eine eigene Ebene „PDF #rrggbb“.
- „Formatfüllend“ übernimmt das Blattformat 1:1 und blendet den eigenen Rahmen aus.
- „Symbole erkennen“ (Standard) fasst die Einzelteile jedes Sicherheitszeichens (Hintergrund, Rahmen,
  Bildzeichen, Text) zu einer Gruppe zusammen, damit es sich als Ganzes verschieben lässt. Für bereits
  importierte Pläne: *Bearbeiten › Symbole erkennen* (ohne Auswahl für den ganzen Plan).
- „Alles in eine Gruppe zusammenfassen“ legt den ganzen Import als eine Gruppe ab
  (wieder lösen mit *Bearbeiten › Gruppe aufheben*).
- Ungefüllte Rahmen (z. B. der Blattrahmen) lassen sich nur an der Linie anklicken, nicht in der Fläche.

Grenzen:
- Sicherheitszeichen kommen als Linien und Flächen an, nicht als Bibliothekssymbole. Für die
  automatische Legende die Zeichen durch Bibliothekssymbole ersetzen.
- Raumnamen in CAD-Strichschrift sind Linien, keine Texte.
- Gescannte PDFs als Bild importieren.

Nach Änderungen Stand und Planersteller im Plankopf anpassen.

**Mehrere Aushangorte:** *Plan › Plan duplizieren*, dann den Standort versetzen.
**Lagerichtig ausrichten:** *Plan › Inhalt ±90° / 180°* – Zeichen und Texte bleiben aufrecht.

## Bedienung

| Aktion | Bedienung |
|---|---|
| Zoomen | Mausrad (Touchpad: zwei Finger spreizen) |
| Ansicht verschieben | Strg+Mausrad senkrecht · Umschalt+Mausrad waagerecht · mittlere Maustaste, Leertaste + Ziehen oder Werkzeug „Verschieben“ |
| Auswählen | Klick · Umschalt+Klick ergänzt · Rahmen von links = vollständig, von rechts = schneidend · Alt+Ziehen = Rahmen auch über Objekten |
| Form ändern | Griffpunkte der Auswahl ziehen |
| Linienzüge beenden | Doppelklick, Enter oder Rechtsklick · Esc bricht ab |
| 45°-Raster beim Zeichnen | Umschalt gedrückt halten |
| Punktfang / Rasterfang | Ansicht › Raster und Fang |
| Verschieben in Schritten | Pfeiltasten (1 mm), mit Umschalt 10 mm |
| Kontextmenü | Rechtsklick auf ein Element |

**Tastenkürzel:** Strg+N/O/S Neu/Öffnen/Speichern · Strg+Umschalt+S Speichern unter · Strg+P Drucken ·
Strg+Z/Y Rückgängig/Wiederholen · Strg+C/X/V/D Kopieren/Ausschneiden/Einfügen/Duplizieren ·
Strg+A Alles · Strg+G / Strg+Umschalt+G Gruppieren/Aufheben · Strg+F Suchen/Ersetzen ·
Entf Löschen · R Drehen 90° · F Einpassen

## Funktionsübersicht

- **Menüband:** Datei · Plan · Einfügen · Bearbeiten · Ansicht · Extras
- **Zeichnen:** Linie, Linienzug, Polygon, Rechteck, Container, Kreis, Ellipse, Bogen, N-Eck, Spline,
  Schlauch, Text, Tabelle, Maßlinie, Wand, Wand orthogonal, Raum, Raummaß (m²), Fenster, Tür,
  Doppeltür, Treppe, Notiz (nicht gedruckt), Revisionswolke, Zaun, Gleis, Maßstabsleiste, QR-Code,
  Planraster (A1 …), Legende, Hilfslinien
- **Flucht- und Feuerwehrflächen:** Fluchtweg mit Pfeilen (gerade/Bogen), Laufweg der Feuerwehr,
  Flächen nach DIN 14095 (befahrbar, nicht befahrbar, besondere Gefahren, Löschwasser, betroffene
  bauliche Anlage, Gaslöschbereich, Sprinklerbereich)
- **Brandschutzpläne:** Feuerwiderstandsklassen REI 30/60/90 (F30/F60/F90, -A, -AB), Brandwand,
  Baustoffe A1/A2/B1, notwendige Treppe/Flur, Sicherheitstreppenraum, Schleuse, Brandabschnitt
- **Symbolbibliothek (222 Zeichen, eigene Darstellung):** Rettungs-, Brandschutz-, Warn-,
  Gebots- und Verbotszeichen (ISO 7010), Feuerwehrplan-Symbole (DIN 14034-6), Brandmeldetechnik
  für Laufkarten, GHS-Gefahrstoffe, Gebäudebeschilderung, Verkehrszeichen/Zufahrt, Nordpfeile,
  Einrichtung; eigene SVG-Symbole importierbar
- **Bearbeiten:** Rückgängig/Wiederholen, Zwischenablage, Gruppieren, Mehrfachkopie, Drehen,
  Spiegeln, Größe ändern, Ausrichten, Reihenfolge, Ebenen (sichtbar/gesperrt), Suchen/Ersetzen,
  Symbol-Manager (Symbol planweit tauschen), Polygon füllen, Plan schneiden, Punkt einfügen,
  Segment ausschneiden, Ecke abrunden, Bereinigen
- **Messen:** Distanz in m (Planmaßstab) und mm (Blatt), Winkel,
  Kurve der Feuerwehrzufahrt nach DIN 14090 (Breite automatisch aus dem Außenradius)
- **Rahmen:** Flucht- und Rettungsplan (vollständig, zweizeilig, nur oben), Brandschutzplan,
  Feuerwehrplan DIN 14095, Feuerwehr-Laufkarte; Titel, Farbe und Logo einstellbar;
  Formate A4–A0, Letter, 40×30 bis 80×80 cm, benutzerdefiniert, hoch/quer
- **Legende:** automatisch, mehrspaltig, mehrsprachig (DE, EN, FR, IT, ES, PL, TR, NL)
- **Extras:** OpenStreetMap-Kartenausschnitt für Übersichtspläne (Maßstab wird übernommen),
  Brandschutzordnung Teil A, Musterpläne, Archivieren/Dearchivieren (ZIP mit Plan + PDF)
- **Flurkarte (WMS)** (Extras → Flurkarte): maßstäbliche Liegenschaftskarte (ALKIS: Flurstücke,
  Gebäude, Verkehr …) oder andere amtliche Karte über einen WMS-Dienst als Grundlage für
  Feuerwehr-Übersichtspläne. Vorlagen: ALKIS Schleswig-Holstein, ALKIS NRW, basemap.de; jede
  andere WMS-Adresse ist möglich. Ablauf: Dienst wählen → Ebenen ankreuzen → Adresse suchen →
  Maßstab (z. B. 1:1000) → einfügen. Die Karte füllt die Planfläche, der Planmaßstab wird gesetzt,
  der Quellenvermerk eingefügt.
  **Nutzungsbedingungen beachten:** Der Dialog zeigt sie an, und erst nach der Bestätigung lässt sich
  die Karte einfügen. ALKIS Schleswig-Holstein darf nur mit schriftlicher Einwilligung des LVermGeo SH
  verwendet werden. ALKIS NRW (dl-de/zero) und basemap.de (CC BY 4.0) sind frei nutzbar.
- **Plan prüfen** (Plan → Plan prüfen): prüft laufend die Pflichtinhalte.
  - Flucht- und Rettungspläne: Standort, Fluchtwege, Notausgang, Sammelstelle, Legende, Stand, Format ≥ A3 und Zeichengröße ≥ 7 mm.
  - Feuerwehrpläne: Nordpfeil, Plannummer, Geschoss-Schnitt, Zugang und Löschwasser.
  - Laufkarten: Meldergruppe und Laufweg.

  Die fachliche Abnahme ersetzt das nicht.
- **Geschoss-Schnitt** (Einfügen → Geschoss-Schnitt): Geschossübersicht für Feuerwehr-Geschosspläne
  nach DIN 14095. Das dargestellte Geschoss ist rot markiert, die Geländeoberfläche ist eingezeichnet.
  Die Geschosse lassen sich in den Eigenschaften bearbeiten.
- **Versatz (Offset)** (Bearbeiten): erzeugt parallele Kopien von Linien, Wänden, Flächen, Kreisen und
  Rechtecken im festen Abstand.
- **Ebenenfarbe** (Ebenen → „Farbe“): überschreibt Linien- und Textfarbe aller Objekte auf einer Ebene.
- **Optionen** (Extras → Optionen): Oberflächengröße 100–200 % für große Monitore,
  Standardwerte für Wand-, Symbol-, Text- und Fluchtwegbreite, Verfasser
- Die OSM-Karte nutzt die Zertifikate von Windows und funktioniert dadurch auch in Firmennetzen,
  die verschlüsselte Verbindungen prüfen (TLS-Inspektion).

## Automatisierung und KI-Schnittstelle

Die exe hat zwei Schnittstellen ohne Fenster. Beide bieten dieselben 19 Werkzeuge:
Plan anlegen, Infos, Objekte auflisten, lesen, schreiben, löschen, verschieben, Gruppen auflösen,
Symbole erkennen,
Planangaben, Text ersetzen und einfügen, Symbole suchen und einfügen, Fluchtwege, Prüfen, Vorschau,
Export und PDF-Import.

**Kommandozeile** – für Stapelarbeit über viele Pläne:

```
OpenFluchtplan.exe --cli                                   (Hilfe mit allen Werkzeugen)
OpenFluchtplan.exe --cli pdf_importieren pdf="Pläne\EG.pdf" ziel=EG.fplan
OpenFluchtplan.exe --cli planangaben_setzen pfad=EG.fplan stand=11/2026 "ersteller=Haustechnik"
OpenFluchtplan.exe --cli text_ersetzen pfad=EG.fplan "suchen=Alt" "ersetzen=Neu"
OpenFluchtplan.exe --cli plan_pruefen pfad=EG.fplan
OpenFluchtplan.exe --cli exportieren pfad=EG.fplan ziel=EG.pdf stempel=entwurf
```

In PowerShell die Ausgabe mit `| Out-String` abholen, dann wartet PowerShell auf das Ende.
Exit-Code 0 bedeutet OK, 1 einen Fehler. JSON-Werte in Parametern lassen sich als
`felder.stand=11/2026` schreiben, weil PowerShell 5.1 Anführungszeichen im JSON entfernt.

**MCP-Server für KI-Assistenten** (Claude Code, Claude Desktop): `OpenFluchtplan.exe --mcp`.
Für Claude Code eine Datei `.mcp.json` im Arbeitsordner anlegen:

```json
{ "mcpServers": { "fluchtplan": { "command": "C:\\Pfad\\zu\\OpenFluchtplan.exe", "args": ["--mcp"] } } }
```

Claude Code fragt beim nächsten Start einmal nach der Freigabe. Danach lassen sich Pläne im Chat
bearbeiten, zum Beispiel: „Setze im Plan EG
neben die Küchentür einen Feuerlöscher und zeig mir den Ausschnitt.“

**Live-Ansicht:** Ist ein Plan im Programm geöffnet und wird die Datei über eine der
Schnittstellen geändert, lädt das Fenster ihn innerhalb einer Sekunde neu. Die Änderung lässt sich
mit *Rückgängig* zurücknehmen. Gibt es im Fenster ungespeicherte eigene Änderungen, fragt das
Programm vorher nach.

## Dateien

- `.fplan` – Plandatei (enthält eingebettete Grundrisse und Logos, also eigenständig weitergebbar)
- Doppelklick-Öffnen: `.fplan` mit `OpenFluchtplan.exe` verknüpfen (Windows „Öffnen mit“)

## Wichtige Hinweise

- Fachliche Prüfung der Pläne (Brandschutzbeauftragte/r bzw. Feuerwehr) bleibt erforderlich;
  örtliche Vorgaben der Brandschutzdienststelle für Feuerwehrpläne beachten.
- Pläne regelmäßig (z. B. alle 2 Jahre) und nach baulichen Änderungen aktualisieren.
- Symbole sind eigene Darstellungen in Anlehnung an ISO 7010 / DIN 14034-6. Wo amtlich
  vorgeschriebene Originalgrafiken verlangt werden, diese als SVG über *Eigenes Symbol* einbinden.
- Kartendaten © OpenStreetMap-Mitwirkende (ODbL) – Quellenangabe wird automatisch eingefügt.
