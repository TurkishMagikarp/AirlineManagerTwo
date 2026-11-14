# Airline Manager UI

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

- Night-Theme aus `docs/ui-kit/tokens.json`: warme dunkle Flächen, Amber für
  Auswahl und primäre Aktionen; `go`, `stop` und `sky` für Zustände.
- Alle Farb-, Abstands-, Radius- und zehn Textstil-Tokens sind benannte Konstanten
  in `operations_theme.gd`; Karten und Diagramme lesen dieselben Werte.
- Outfit (variable) für Titel und Spaltenköpfe, Nunito (variable) für Text und
  Buttonbeschriftungen; DM Mono Regular/Medium für Zahlen und Codes. Die Originale
  aus google/fonts samt OFL liegen unter `resources/fonts/`.
- Buttons, Eingabefelder, Select-Menüs, Tabellen, Tabs, Panels, Scrollbars und
  Tooltips erben die neuen Zustände. Fokus: 2px `focus`-Ring mit 2px Abstand.
- Die sechs weißen SVG-Navigationsicons erhalten pro Buttonzustand die Textfarbe.
  Bildschirmaufbau, Aktionen und deutsche Texte bleiben erhalten.
- Keine neue Spiellogik, Save-Version, Wirtschaftswerte oder zweite Uhr.

## Fester Bildschirmrahmen

Die Navigation steht links mit 208px Breite; unter 900px Fensterbreite wird sie
56px breit und zeigt die sechs Icons mit deutschen Tooltips. Airline-Name und
Basis kommen aus `get_airline_info()`. Einen Airline-Code liefert die Simulation
noch nicht: dafür steht ein Gedankenstrich statt erfundener Daten. Badges zählen
nur die Einträge des vorhandenen Aufmerksamkeitssnapshots je Zielseite.

Das Flugarchiv lässt die Seitennavigation bei langen, simulationsbasierten
Flugzahlen umbrechen. So bleiben „Neuere Flüge“ und „Ältere Flüge“ auch im
680-Pixel-Fenster vollständig erreichbar; die Flugzahlen werden nicht gekürzt.

Die ClockBar bleibt oben außerhalb des vertikalen Inhalts-Scrollbereichs. Jede
Stelle der unveränderten Rust-Anzeige „Tag N · HH:MM:SS“ hat eine dunkle 22×32px
Split-Flap-Zelle mit DM Mono und Naht, ohne Animation. Kalenderdatum/Wochentag
liefert die bestehende Uhr nicht; es gibt keine erfundene Epoche oder zweite Uhr.
Pause und Geschwindigkeiten bilden ein Segmentfeld. Start und die bisherige
Semantik (Geschwindigkeitswechsel hebt Pause nicht auf) bleiben erhalten.
Guthaben kommt aus der Airline und wird unter null `stop`. Speichern/Laden und
Statusmeldungen bleiben erreichbar; auf schmalen Fenstern umbrechen die Gruppen.

Über dem Seiteninhalt stehen die einklappbaren Hinweise in unveränderter
Rust-Reihenfolge. Jede 44px-Zeile zeigt einen Statuschip, einen hervorgehobenen
Satzanfang und das Ziel. Lange Sätze werden im vorhandenen Tooltip vollständig
angezeigt; bei schmaler Breite steht dort auch das Klickziel. Nach vier Zeilen
scrollt die Liste in ihrem eigenen Bereich, mit sichtbarem Tastaturfokus.
Alle sechs Seiten sind bereits in der Screenshot-Liste des Layouttests enthalten.

## Wiederverwendbare Bausteine

`scripts/ui/components.gd` bietet `panel`, `metric`, `table`, `label` sowie
`money` zur Darstellung ganzer EUR mit Tausenderpunkten. Die Hilfen lesen und
verändern keinen Spielzustand. Buttons verwenden bei Bedarf die Theme-Varianten
`PrimaryButton`, `DangerButton`, `DangerSolidButton` und `GhostButton`.
`UI.label` erwartet einen benannten Textstil statt einer freien Schriftgröße;
Tabellenzellen erhalten über `UI.cell_text` Text-, Daten- oder Code-Schrift.
Numerische SpinBox-Felder verwenden `NumericInput`.

AIR-118 ergänzt die gemeinsamen Bausteine des Kits:

- `panel(parent, title, accessory, flush)` liefert weiterhin ausschließlich den
  Body zurück. Darüber stehen ein mindestens 48px hoher Kopf, optional genau eine
  Aktion (28px) oder ein Status und eine Trennlinie. Der Body hat 16px Innenabstand;
  `flush` lässt eine alleinstehende Tabelle ohne diesen Abstand zu.
- `metric` liefert weiterhin ein `Label` mit unverändertem `.text` zurück. Ein
  RichText-Kind rendert die Zahl in DM Mono 24 und die Einheit in DM Mono 14.
  Zahlen und Einheiten umbrechen innerhalb der mindestens 232px breiten Karte;
  lange Beschriftungen ebenfalls. Optional folgt genau eine unterstützende Zeile
  oder ein Fortschritt. Gute Werte bleiben `ink`, schlechte Werte werden `stop`.
- `status_chip` zeichnet Kreis, Raute, Strich, Dreieck oder Quadrat zu den fünf
  Zuständen. Tabellen verwenden über `cell_status` native Icon-/Text-Zellen mit
  derselben Schrift und Zustandsfläche; auch ausgewählte Zeilen behalten Glyph
  und ausgeschriebenen Zustand. Lange Zustände erweitern die intern scrollbare
  Spalte. Deutsche Texte bleiben erhalten.
- `table` vereinheitlicht auch Flotten- und Routenregister: dunkler Header,
  40px-Zeilen mit Trennlinien, Zahlen rechts in DM Mono, Codes im Code-Stil.
  Auswahl, Maus, Tastatur, Metadaten und bestehende Tooltips bleiben im `Tree`.
- `progress` zeigt einen 8px-Balken und die Zahl; `route_leg` zeichnet die geflogene
  Strecke und die Flugzeugraute. Ohne Abflug entfällt die Raute. Die Helfer
  erhalten sämtliche Werte vom Aufrufer und berechnen keine Spielregeln.
- `dialog` baut einen abgedunkelten Overlay mit einer 440px breiten Fragekarte,
  Konsequenztext und zwei Buttons. Die sichere Wahl links erhält den Fokus,
  die gefährliche rechts `DangerSolidButton`. Tab bleibt in der Entscheidung,
  Escape schließt sie ohne Aktion. Der Helfer steht für spätere Dialogaufrufer
  bereit; bestehende Spielaktionen werden durch dieses Ticket nicht verändert.
- `empty_state` ersetzt die leeren Flotten- und Routentabellen durch Titel,
  vorhandenen Erklärungstext und eine primäre Aktion zum bestehenden Formular.
  Sein Rahmen ist gestrichelt; lange Texte umbrechen, die Seite bleibt scrollbar.

`ui_kit_test.gd` baut alle Helfer headless auf und prüft deutsche Langtexte,
Einheitenumbruch bei Mindestbreite, 40px-Zeilen, Schrift/Alignment, Statusglyphen
und sicheren Dialogfokus. Die Screenshot-Liste enthält zusätzlich leere Flotte
samt erreichbarer Aktion sowie Bausteine und Dialog bei allen drei Fenstergrößen.
Die dort gezeigten Zahlen stammen aus derselben regulären Testsimulation.

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

Der zweite Aufruf benötigt einen echten Grafikrenderer. `tools/godot_check.sh
shots /paperclip/shots/AIR-118` startet diesen unter Linux über Xvfb. Er erzeugt
echte Viewport-Aufnahmen für Übersicht, Flotte, Routen, Karte, Finanzen, Personal,
Gründung sowie gescrollte Inhalte, tieferliegende Routenaktionen und Flugergebnisse bei **1440×1000, 1100×720 und 680×520**.
Die Test-Airline wird ausschließlich über die bestehenden Rust-APIs gegründet:
drei regulär gekaufte ATR, EDDF–EDDM mit Ticketpreis 71 EUR und acht Landungen.
Keine künstlichen UI-Zahlen, keine Änderungen an Wirtschaftskonfiguration oder
Spiellogik, kein Zugriff auf den persönlichen Spielstand.

Layouttests prüfen sichtbare Hauptaktionen, minimale Fenstergröße, Erreichbarkeit
der Flugplansteuerung und unveränderte Finanzsummen nach Navigation/Größenwechseln.
Die bestehenden UI-Tests prüfen weiterhin echte Maus-/Tastaturaktionen und
Speichern/Laden; ihre Anzeigeprüfungen verwenden jetzt die Kennzahlen-Karten und
Ergebnistabellen statt der früheren Fließtexte.

`ui_theme_test.gd` prüft die tatsächlich beim TextServer ankommenden variablen
Schriftgewichte. OpenType-Achsen werden als numerische Tags übergeben; der String
`wght` würde bei Godot unbemerkt auf die dünnste Schriftstärke zurückfallen.
DM Mono verwendet ausschließlich die mitgelieferten Schnitte 400 und 500.
Die Gründungsmaske reserviert im Scrollbereich Platz für den äußeren Fokusring.

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

## UI Kit Schritt 1 (AIR-117)

Die Umstellung betrifft die gemeinsamen Schrift- und Theme-Grundlagen. Weitere
Kit-Bausteine und Bildschirmumbauten sowie Tag-Theme und Lokalisierung folgen
in späteren Tickets. Die 40px-Tabellenzeilen und 36px-Controls orientieren sich
an `bundle.css`; bestehende größere Mindestmaße bleiben erhalten.

Auf dem Linux-Server sind Godot-UI-Tests, START.bat, echte Screenshots und
Windows-Neustarttests nicht ausführbar. Die drei Fenstergrößen müssen auf
Windows mit `ui_layout_test.gd -- --capture-dir` geprüft werden; Rust-Checks
belegen weder die Darstellung noch die Erreichbarkeit der UI-Aktionen.
