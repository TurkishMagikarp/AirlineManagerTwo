# Musterberechtigungen: Godot- und Neustartprüfung

Stand: 14. November 2026, Linux, Godot 4.7.2, Rust-Debug-Erweiterung,
Basis PR #33. Ergebnisse und geprüfte Originalaufnahmen:
[AIR-75](https://paperclip.iamsec.de/AIR/issues/AIR-75).

`tests/pilot_ratings_test.gd` öffnet die echte Personal-Seite. Mausereignisse
wählen die Musterberechtigung, stellen zwei ATR-72-Piloten ein und starten
die Umschulung eines Piloten auf 737; die Anzahl wird per Tastatur eingegeben. Die Prüfung
vergleicht jede Musterzeile (berechtigt, im Einsatz, verfügbar), die
Mitarbeiterberechtigung, den vollständigen Fristtext und die konkreten
Flugzeugengpässe mit der Rust-API. Ein zusätzliches A320-Flugzeug hat keine
passenden Piloten. Während der Umschulung sind Entlassung und erneute
Umschulung deaktiviert; echte Mausklicks auf beide Aktionen lassen Welt und
APIs unverändert.

Die Neustartmodi laufen in getrennten Prozessen mit expliziten Speicherpfaden.
Spieler und beide KI-Eigentümer haben jeweils eine ATR 72 und einen geleasten
A320, einen laufenden Dienst sowie eine gewartete Maschine mit zurückgehaltenem
Personal und fortzusetzendem Flugplan. Die KI-Testwelten entstehen über dieselbe
Rust-API; beim Einsetzen in das KI-Format werden nur die SaveState-Hüllen für
Uhr, Flotte und Routen angepasst. Keine Welt wird über Godots JSON-Zahlen neu
serialisiert: große Seeds und Finanzjournale bleiben bytegetreu erhalten.

- `write-v24` erzeugt ein Legacy-Fixture ohne `rating`/`training`, lädt es
  mit der aktuellen Migration und prüft sämtliche anderen Weltbytes gegen
  den Ausgangszustand. `read-v24` lädt diese Datei in einem frischen Prozess
  und vergleicht die vollständige migrierte Welt und API sofort mit dem
  Referenzlauf. Beide Läufe stellen anschließend denselben freien Piloten
  ein und starten die Umschulung.
- `write-v25` speichert bereits **eine Stunde nach Umschulungsbeginn**;
  die Wartungen aller drei Eigentümer laufen noch. `read-v25` muss Training,
  Zielmuster, Restzeit und die komplette Welt/API sofort exakt wiederherstellen.
- Beide Paare prüfen eine Sekunde vor und genau an der Rust-Abschlussfrist,
  dass der Pilot erst dann wieder verfügbar und auf 737 berechtigt ist.
  Nach exakt weiteren 100 Tagen werden Welt und sämtliche Snapshots erneut
  mit dem ununterbrochenen Lauf verglichen, einschließlich Mitarbeiter,
  Gehälter, Personalkosten, Finanzen und KI-Zuständen. Die einmaligen
  25.000 EUR Umschulungskosten erscheinen exakt einmal als Personalkosten;
  alle Eigentümer buchen echte Gehaltszahlungen.

Ein echter v24-Spielstand kennt keine Umschulung. Deshalb prüft das erste Paar
Migration plus anschließende Umschulung; das zweite Paar prüft zusätzlich den
Neustart mitten in der Umschulung. Es wird kein v24-Training erfunden.

Linux-Aufruf (jeden Schreiber vor seinem Leser beenden):

```sh
Godot --headless --path . --script res://tests/pilot_ratings_test.gd -- \
  --pilot-mode=write-v24 --pilot-save=/tmp/pilot-v24.json
Godot --headless --path . --script res://tests/pilot_ratings_test.gd -- \
  --pilot-mode=read-v24 --pilot-save=/tmp/pilot-v24.json
# Dasselbe mit write-v25/read-v25 und eigenem Speicherpfad.
```

`tests/run_pilot_restart_test.ps1` stellt dieselbe Vier-Prozess-Sequenz für
Windows bereit (`AIRLINE_GODOT` oder bisheriger Standardpfad). Der tatsächliche
PowerShell-Aufruf wurde auf diesem Linux-Server nicht ausgeführt.

Echte Xvfb-/OpenGL-Aufnahmen mit Mesa llvmpipe in 1440×1000, 1100×720 und
680×520 wurden einzeln geprüft und neun nützliche Originale an der Aufgabe hinterlegt. Die Personal-Seite scrollt vertikal; schmale
Mitarbeitertabellen scrollen zusätzlich horizontal zum Status und zur Frist.
Die deaktivierten Aktionen bleiben erreichbar. Ein erster Testhelfer klickte
bei kleiner Höhe auf eine verdeckte Tabellenzeile und traf die Navigation;
die Screenshotprüfung deckte den falschen Erfolg auf. Der Helfer zentriert
jetzt die Zeile im sichtbaren Inhalt und verlangt weiterhin die Personal-Seite.
Headless-Assertions allein gelten nicht als Sichtprüfung.

Native Windows-DLL-/Prozessprüfung, START.bat, tatsächliche PowerShell-Wrapper
und Windows-Darstellung/-Skalierung bleiben Besitzerprüfungen.

Die vollständige Godot-Regression bestand: alle **28** eigenständigen Skripte,
Hauptszene (`--quit-after 10`), Editor-Import sowie alle bisherigen
Neustartfamilien einschließlich laufender/pausierter Unicode-Speicherpfade.
Insgesamt **84** erfolgreiche abschließende Aufrufe; der erste Editor-Import
kommt zusätzlich hinzu. Nach Korrektur des neuen Screenshothelfers wurde ab
diesem Skript fortgesetzt; bereits bestandene, unveränderte Skripte wurden
nicht unnötig wiederholt. Finanzarchiv: **348.76 s**,
frischer Leser: **111.10 s** (jeweils einschließlich
Speichern/UI/Fortsetzung, kein reiner Fortschreibungsbenchmark). Beide neuen
v24/v25-Neustartpaare und alle API-/Fristvergleiche bestanden.

Locked-Rust-Build und Clippy mit `-D warnings` bestanden. Alle **207 Rust-Tests**
abgedeckt: 206 im vollständigen Ein-Thread-Lauf ohne Millionenflugtest, danach
der Millionenflugtest allein. Gemessene Fortschreibung: **263.229513905 s** unter
dem unveränderten **300-s-Limit**. Auch Speichern/Laden/Fortsetzung bestanden.
Gesamtdauer des isolierten Testkommandos: **334.63 s**.
Rust-, Gameplay- und UI-Produktionscode blieben unverändert.
