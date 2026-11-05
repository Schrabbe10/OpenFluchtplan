# Mitmachen bei OpenFluchtplan

Danke für jede Verbesserung, ob Fehlerkorrektur, neues Symbol, Übersetzung oder Doku.

## Ablauf

1. Repository forken und einen Branch anlegen.
2. Änderung umsetzen, im Stil des vorhandenen Codes (deutsche Bezeichner und Kommentare).
3. `cargo fmt`, dann `cargo test --release`. Alle Tests müssen grün sein.
4. Für neue Funktionen bitte einen Test ergänzen (`src/tests.rs`). Für Bedienabläufe gibt es
   Oberflächentests ohne Bildschirm (`src/ui_tests.rs`, egui_kittest).
5. Pull Request mit kurzer Beschreibung: was, warum, wie getestet.

Jede Pull-Request-Änderung wird automatisch unter Windows gebaut und getestet (GitHub Actions).

## Bitte beachten

- **Keine echten Pläne einchecken.** Grundrisse, Feuerwehrpläne und Laufkarten realer Gebäude
  enthalten sicherheitsrelevante Angaben. Für Tests die Musterpläne (`src/samples.rs`) nutzen.
  Tests mit eigenen PDFs laufen lokal über die Umgebungsvariable `FLUCHTPLAN_PDF_ORDNER`
  (`cargo test --release -- --ignored`).
- **Keine geschützten Inhalte:** keine Normtexte, keine Grafiken oder Texte aus kommerziellen
  Programmen. Symbole bitte selbst als SVG zeichnen.
- **Lizenz:** Mit einem Beitrag stimmst du zu, dass er unter der Unlicense (gemeinfrei) steht.
- Fachliche Hinweise zu den Normen, etwa was eine Brandschutzdienststelle tatsächlich verlangt,
  sind als Issue sehr willkommen.

## Ideen für Beiträge

- Übersetzung der Oberfläche (zurzeit nur Deutsch)
- Weitere Symbole (z. B. Schweiz/Österreich: TRVB, SN EN ISO 7010)
- Weitere Planrahmen, z. B. nach Vorgaben einzelner Feuerwehren
- Linux/macOS-Builds
- DWG-Import ohne externes Werkzeug
