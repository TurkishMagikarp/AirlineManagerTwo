# Aufmerksamkeitsliste: Godot-Prüfung

Stand: 14. November 2026, Linux, Godot 4.7.2, Rust-Debug-Erweiterung,
Basis PR #35. Originalaufnahmen und Ergebnisse:
[AIR-80](https://paperclip.iamsec.de/AIR/issues/AIR-80).

`tests/attention_test.gd` erzeugt zwei ATR 72, zwei unterschiedliche Routen,
fehlende Besatzungen und einen Piloten in laufender Umschulung. Die fünf
Aufmerksamkeitszeilen kommen ausschließlich aus dem Rust-Snapshot. Der Test
vergleicht Anzahl, Reihenfolge und jeden angezeigten Text mit dieser API.

Echte Godot-Mausereignisse wählen nacheinander Route 1 und 2 sowie Flugzeug
1 und 2; geprüft werden die aktive Seite und die tatsächliche Tabellenwahl,
nicht nur die erste Standardauswahl. Die Trainingszeile wählt den konkreten
zweiten Mitarbeiter im Personal-Tab. Ein-/Ausklappen wird ebenfalls per Maus
bedient. Eine Leasingaktion unterhalb der ausgeklappten Liste vergrößert die
Flotte tatsächlich: Die Liste fängt den Seitenklick nicht ab. Diese Abläufe
laufen bei 1440×1000, 1100×720 und 680×520.

Unveränderte Snapshots behalten dieselben Button-Instanzen, den Fokus und die
Scrollposition über mehrere Frames. Für jede Geschwindigkeit 1×/4×/16× werden
Start und Geschwindigkeit angeklickt; der echte `_process` läuft über mehrere
Frames, einschließlich eines weiteren Mausklicks auf eine Flugzeugzeile.
Weitere Fortschreibungen rufen denselben Produktionscallback mit kontrolliertem
Delta auf, damit die Rust-Frist ohne stundenlanges Warten exakt erreicht wird:

- Personal einstellen entfernt die Besatzungswarnungen im nächsten Frame.
- Ein Flugplan entfernt nur den zugewiesenen Flieger aus der Leerlaufliste;
  der andere Flieger und das Training bleiben erreichbar.
- Speichern/Laden über die sichtbaren Buttons stellt die vollständige
  serialisierte Spieler-/KI-Welt und den Aufmerksamkeitssnapshot exakt wieder
  her. Die zuvor eingeklappte Liste bleibt eingeklappt.
- Eine Sekunde vor dem Trainingsende bleibt die Zeile bestehen. Genau am Ende
  verschwindet sie ohne erneutes Ausklappen. Die verbleibende Flugzeugzeile
  navigiert nach Entfernung der anderen Zeilen weiterhin zum richtigen Ziel.
- Der letzte Flugplan leert die Liste; der deaktivierte Kopf öffnet keinen
  leeren Scrollbereich.

Sechs originale Xvfb-/Mesa-llvmpipe-Aufnahmen (ausgeklappt/eingeklappt bei
allen drei Größen) wurden visuell geprüft und an der Aufgabe gespeichert.
Die Aufnahmen zeigen den Zustand nach Auswahl des Trainees, also eine bereits
nach unten gescrollte Liste. Die Liste und die umgebende Seite scrollen
unabhängig. Bei 680×520 ist Scrollen nötig, um alle Zeilen oder darunterliegende
Seitenaktionen zu erreichen. Die Eingabeprüfungen bedienen auch diese Zeilen;
Headless-Assertions allein gelten nicht als Sichtprüfung.

Der anfangs verwendete gemeinsame Klickhelfer konnte bei kleiner Höhe eine
verdeckte Zeile anvisieren. Der neue Aufmerksamkeitstest zentriert das Ziel
nacheinander in den inneren und äußeren Scrollbereichen, wartet deren Layout
ab und klickt nur auf die Schnittfläche aller sichtbaren Viewports. Ein
verfehlter Testklick wird nicht als Produktionsfehler oder erfolgreiche
Navigation gewertet. Es wurde kein Produktionsfehler bestätigt; Gameplay,
Ökonomie, Rust-Regeln und Speicherformat bleiben unverändert (Version 25).

Reproduktion mit frischem Benutzerverzeichnis:

```sh
cargo build --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --editor --import
Godot --headless --path . --script res://tests/attention_test.gd
# Mit echtem Display oder Xvfb/Mesa; Zielverzeichnis vorher anlegen:
Godot --path . --rendering-method gl_compatibility --audio-driver Dummy \
  --script res://tests/attention_test.gd -- --capture-dir=/tmp/attention-renders
```

Native Windows-DLL-/Prozessprüfung, START.bat, tatsächliche PowerShell-Wrapper,
Windows-Neustarts und Windows-Darstellung/-Skalierung bleiben Besitzerprüfungen.
Die Linux-Prüfungen ersetzen diese nicht.

Abschluss: Alle **29** eigenständigen Godot-Tests, Hauptszene
(`--quit-after 10`), Editor-Import und alle bisherigen Linux-Neustartfamilien
bestanden (**85** erfolgreiche Aufrufe der vollständigen Serie, zusätzlich
initialer Import, ursprünglicher Aufmerksamkeitstest und abschließender
erweiterter Aufmerksamkeitstest). Die zuletzt ergänzte Eingabe während
automatischer Frames bestand außerdem headless und mit echtem Rendern.
Der Finanzarchivtest benötigte **344.26 s**,
der frische Leser **103.51 s** insgesamt.

Locked-Rust-Build, alle **210 Rust-Tests** und Clippy mit `-D warnings`
bestanden. 209 Tests liefen im vollständigen Ein-Thread-Lauf ohne den
Millionenflugtest; dieser lief anschließend allein, ohne Godot daneben.
Gemessene Fortschreibung **214.168829672 s** unter unverändertem **300-s-Limit**;
Speichern/Laden/Fortsetzung ebenfalls bestanden (Kommandodauer
**269.73 s**). Keine Produktionsänderung und kein Versionssprung.
