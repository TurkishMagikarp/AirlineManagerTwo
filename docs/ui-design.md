# Operations Center UI

## Zielbild: UI Kit

Das Zielbild der Oberfläche steht in `docs/ui-kit/` (Regeln in `README.md`, Werte in
`tokens.json`, Vorgaben je Baustein unter `components/`). Es löst das unten
beschriebene Navy/Cyan-Design schrittweise ab. Wo dieses Dokument dem Kit
widerspricht – etwa bei Farben oder dem Verzicht auf externe Schriftarten – gilt
das Kit. Die Abschnitte unten beschreiben den Stand vor der Umstellung und werden
mit jedem umgesetzten Schritt angepasst.

## Gemeinsames Design

`resources/operations_theme.tres` ist das gemeinsame Godot-Theme. Die Ressource
verwendet `scripts/ui/operations_theme.gd` für zentrale Farben, Zustände,
Abstände, Eingabefelder, Tabellen, Menüs und Scrollbars. `scenes/main.tscn`
setzt es am Wurzel-Control; alle darunter eingefügten Bildschirme erben es
automatisch. Eigenständig gestartete Szenen erhalten dieselbe Theme-Ressource.

- Navy-Flächen, zurückhaltendes Cyan für Auswahl und primäre Aktionen.
- Grün für positive Ergebnisse/geparkte Flugzeuge, Rosé für Verluste und
  Löschaktionen, Sand für Bodenzeit. Status bleibt zusätzlich ausgeschrieben.
- Tastaturfokus hat eine sichtbare Kontur; deaktivierte Aktionen bleiben erkennbar.
- Keine Animationen, externen Schriftarten oder zusätzlichen Asset-Abhängigkeiten.
- Kleine eigene SVG-Icons liegen in `resources/icons/`.

## Wiederverwendbare Bausteine

`scripts/ui/components.gd` bietet `panel`, `metric`, `table`, `label` sowie
`money` zur Darstellung ganzer EUR mit Tausenderpunkten. Die Hilfen lesen und
verändern keinen Spielzustand. Buttons verwenden bei Bedarf die Theme-Varianten
`PrimaryButton` und `DangerButton`.

Für neue Seiten ein Control unter dem bestehenden Root einfügen, Panels mit
`UI.panel(parent, title)` aufbauen und Standard-Controls verwenden. Keine lokalen
StyleBoxes kopieren. Tabellen dürfen intern horizontal scrollen; die Seite selbst
scrollt vertikal. `ScrollContainer.follow_focus` erhält die Tastaturbedienung.

Kennzahlen wechseln abhängig vom verfügbaren Platz zwischen vier und zwei
Spalten; Arbeitsbereiche zwischen nebeneinander und untereinander. Navigation,
Uhr und Speichern/Laden bleiben außerhalb des scrollbaren Inhalts erreichbar.
Flottenregister und laufender Betrieb stehen vor Beschaffungs-/Planungsformularen.
Kostenaufschlüsselungen pro Landung sind als Tabellen-Tooltips erhalten.

## Prüfung und Screenshots

Zusätzlich zu den bestehenden Rust-, UI- und Prozess-Neustarttests:

```text
Godot --headless --path . --script res://tests/ui_layout_test.gd
Godot --path . --script res://tests/ui_layout_test.gd -- --capture-dir=docs/screenshots
```

Der zweite Aufruf benötigt einen echten Grafikrenderer. Er erzeugt echte
Viewport-Aufnahmen für Übersicht, Flotte, Routen, tieferliegende Routenaktionen
und Flugergebnisse bei **1440×1000, 1100×720 und 680×520**.
Die Test-Airline wird ausschließlich über die bestehenden Rust-APIs gegründet:
drei regulär gekaufte ATR, EDDF–EDDM mit Ticketpreis 71 EUR und acht Landungen.
Keine künstlichen UI-Zahlen, keine Änderungen an Wirtschaftskonfiguration oder
Spiellogik, kein Zugriff auf den persönlichen Spielstand.

Layouttests prüfen sichtbare Hauptaktionen, minimale Fenstergröße, Erreichbarkeit
der Flugplansteuerung und unveränderte Finanzsummen nach Navigation/Größenwechseln.
Die bestehenden UI-Tests prüfen weiterhin echte Maus-/Tastaturaktionen und
Speichern/Laden; ihre Anzeigeprüfungen verwenden jetzt die Kennzahlen-Karten und
Ergebnistabellen statt der früheren Fließtexte.

### Verifiziert am 12.11.2026

Windows, Godot 4.7.2, Compatibility-Renderer (NVIDIA RTX 3080):

- `cargo test --locked --manifest-path rust/Cargo.toml`: 37 Tests erfolgreich.
- Alle sieben bestehenden Godot-UI-/Integrationstestskripte und der neue
  `ui_layout_test.gd`: erfolgreich.
- Alle drei PowerShell-Neustarttests (Spielstand, Flugbetrieb, Finanzen): erfolgreich.
- `START.bat --headless --quit-after 10`: Build, Import und Spielstart erfolgreich.
- Echte Grafikaufnahmen und Sichtprüfung der drei Fenstergrößen erfolgreich.
- Rust-Code, Wirtschaftskonfiguration, Spielstandformat und Launcher unverändert.

## Priorisierte Aufmerksamkeit

Oberhalb jeder Seite zeigt eine einklappbare, scrollbar begrenzte Liste die
aktuellen Hinweise der Spieler-Airline. `SimulationClock.get_attention_snapshot()`
leitet alle Einträge ausschließlich in Rust aus dem aktuellen Zustand ab; es gibt
keinen zusätzlichen Speicherstand und keine neue Save-Version.

`config/attention.json` legt Prioritäten (kleinere Zahl zuerst) und die Grenze für
negatives Guthaben fest. Bei gleicher Priorität sortieren Art, öffentliche Route,
Flugzeug-ID und Mitarbeiter-ID stabil. Die Personalreserve verwendet dieselben
Perioden aus `config/staff.json` wie die Einstellung und enthält offene Gehälter.
Negatives Guthaben ersetzt den weniger dringenden Reservehinweis.

Angezeigt werden zugewiesene Routen ohne startbaren Flugplan bei fehlender
Besatzung (inklusive Musterberechtigung), auf Techniker wartende Wartungen,
Guthabenwarnungen, geparkte Flugzeuge ohne Flugplan sowie laufende Umschulungen
mit Endzeit. Ein Klick öffnet Routen/Flotte/Finanzen/Personal und selektiert, sofern
vorhanden, das betroffene Objekt. Die Liste wird auch während Pause nach Änderungen
aktualisiert. KI-Entscheidungen und Wirtschaft verwenden diese Anzeige nicht.

Rust: `cargo test --locked --manifest-path rust/Cargo.toml attention_`.
Godot-Mauseingabe: `Godot --headless --path . --script res://tests/attention_test.gd`.
Der Godot-Test sowie GUI, Screenshots, START.bat und Windows-Neustarts wurden auf
dem Linux-Implementierungsserver nicht ausgeführt; Rust-Tests sind kein visueller
Layoutnachweis.
