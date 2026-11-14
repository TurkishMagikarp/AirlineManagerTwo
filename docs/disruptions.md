# Technische Verspätungen

Rust berechnet je Flugzeug und Lebenszeitzyklus einen deterministischen Wurf
(mit dem Unternehmens-/Flugzeug-Seed der Versicherung, eigener Zufallsdomäne).
Alter seit Erwerb und Zustand seit dem letzten Check erhöhen Häufigkeit und
mögliche Dauer. Alle Wahrscheinlichkeiten, Dauern und Schwellen stehen in
`config/disruptions.json`; es gibt keinen Bezug zur Systemzeit.

Der tatsächliche Abflug ist das Maximum aus geplantem Abflug und Ende der
vorherigen Mindestbodenzeit, zuzüglich einer eigenen technischen Verzögerung.
Freie Zeit im Flugplan baut Rückstand ab. Flugzeit und Mindestbodenzeit werden
nicht gekürzt. Wartung und Versicherungsereignisse verwenden die tatsächliche
Landung. Die vorhandenen Personal- und Wartungssperren gelten weiterhin.

Beim Abflug werden Preis, Passagiere und Kosten eingefroren; die Buchung erfolgt
wie bisher bei Landung. Über der Betreuungsschwelle entstehen pro Passagier
Betreuungs-/Entschädigungskosten als eigene Position `care`. Sie fließen über die
Flugkosten in Ergebnis, Archiv und Steuerbasis ein. Auch weitergegebene
Verspätung zählt; keine erneute Buchung bei Laden oder zwischen Bildern.

Pünktlichkeit misst Ankünfte innerhalb der konfigurierten Toleranz (Flugzeiten
bleiben konstant, deshalb entspricht das der Abflugverspätung). Route und
Airline zeigen den Anteil der seit Einführung erfassten pünktlichen Flüge;
„—“ bedeutet keine erfassten Flüge. Flotte und Betrieb zeigen Verspätung, die
Aufmerksamkeitsliste zeigt große Rückstände. Seit v28 ergänzt der unten
beschriebene Pünktlichkeitsruf das Angebotsgewicht. KI-Airlines verwenden
dieselbe Simulation.

Spielstandversion 26 speichert einen kompakten Zeitcursor pro Flugplan sowie
Pünktlichkeits- und Betreuungssummen im bestehenden Finanzarchiv. Der kompakte
Zähler `otp` erfasst alle beobachteten Flüge, `late` nur verspätete; pünktliche
Flüge ergeben sich aus der Differenz. Nullwerte werden im Archiv ausgelassen. Ältere
Spielstände haben durch neutrale Standardwerte keine laufenden Verspätungen und
keine Pünktlichkeitshistorie. Bereits fliegende Abflugzitate und Flugzeiten bleiben
unverändert; neue Würfe beginnen danach. Checks und bewusste Neuplanung beginnen
wie bisher einen neuen Flugplan, erhalten aber Lebenszeitverschleiß und Seed.

Technische Ereignisse erzeugen zusätzliche Archivgrenzen. Der unveränderte
Millionenflug-Laufzeitgrenzwert beträgt 300 Sekunden für die Simulation; die
begrenzte maximale Spielstandgröße steigt wegen der jahrhundertelangen
KI-/Spielerhistorie von 128 auf 256 MiB. Normale Zehnjahresspielstände bleiben
im einstelligen MiB-Bereich; der Belastungstest umfasst mehrere Millionen Flüge.

## Verifikationsfamilie

`tests/disruptions_test.gd` verwendet unveränderte Produktionskonfiguration und
Seed 33364 mit 570 Check-/Lebenszeitzyklen. Der Rust-Fixturetest fixiert die
Zeitpunkte: Abflug fällig bei 60 s, tatsächlicher Abflug 2992 s, Landung 5104 s;
der zweite Abflug bei 9612 s hat 4152 s Rückstand. Der dritte Flug erhält ohne
neues technisches Ereignis 2664 s Rückstand; die Rotation holt ihn mit freier
Flugplanzeit auf. Normale KI-Airlines bleiben im Weltzustand enthalten.

Der Godot-Test führt echte Engineframes und kontrollierte Grenzframes über
`main._process` bei 1×/4×/16× aus. Er prüft Flotte/Betrieb, „—“ und spätere
Pünktlichkeitsanteile, einmalige Betreuung in Routen-/Flugkosten, Laden sowie
Aufmerksamkeitszeilen. Layout-/Renderfälle: 1440×1000, 1100×720 und 680×520.
Mit `-- --capture-dir=<vorhandenes Verzeichnis>` schreibt der gerenderte Lauf
unveränderte Viewport-PNGs. Die Größen-/Erreichbarkeitsprüfungen ersetzen keine
Sichtprüfung der Originalbilder; diese anschließend an das Issue anhängen.

`tests/run_disruption_restart_test.ps1` startet vier getrennte Prozesse:
Writer/Reader für v25→v26 (neutrale Historie und eingefrorenes altes Flugzitat)
und v26 (verspätetes Flugzeug in der Luft mit Betreuungskosten). Vollständiger
Spieler-/KI-Spielstand und API-Snapshots werden sofort sowie nach 100 Tagen
verglichen. Die v25-Datei entfernt nur neue Felder ohne JSON-Neukodierung,
damit große Seeds und Bruchteilsjournale unverändert bleiben. Ein v25-Flug kann
noch keine technische Verspätung haben. Der Rust-Test prüft beide Familien auch
ohne Engine; getrennte Godot-Prozesse prüfen dagegen einen echten Neustart
(unter Linux direkt, unter Windows über den PowerShell-Wrapper).

## Linux-Verifikation (AIR-90)

Am 14. November 2026 mit der offiziellen Linux-Version Godot 4.7.2 und einer
frisch gebauten Rust-Debug-GDExtension geprüft. Build mit `cargo build --locked
--manifest-path rust/Cargo.toml`, Zig-CC-Wrapper aus `/paperclip/airline/bin`.
Isolierte neue XDG-Benutzerverzeichnisse; alle Godot-Aufrufe sequenziell,
Schreiber vollständig beendet vor dem jeweiligen frischen Leser.

Die bisherigen Godot-Migrationsfixtures verwendeten noch v25 als aktuellen
Versionskopf. Diese Literale und die Resave-Assertions sind auf v26 korrigiert;
die alten Zielversionen und verlustfreien Welt-/API-Vergleiche bleiben erhalten.
Die Fleet-Purchase-Assertion erwartet jetzt den bereits vorhandenen Zusatz
„0 min verspätet“ auch bei geparkten Flugzeugen. Der Personaltest zählt die
bereits bestehenden Betreuungskosten in seiner vollständigen Kostensumme mit;
Gehalts-/Schrittgrößen- und Routenabgleich bleiben strikt.
Der erste Godot-Finanzarchivlauf scheiterte an der alten pauschalen Grenze
von 100.000 Blöcken (123.997 Blöcke, 3.480 Checks). Technische Verspätungen
bilden zusätzliche reale Archivgrenzen. Die Assertion entspricht jetzt der
bereits bestehenden Rust-Regel für dieses gemischte Zwei-Routen-Szenario:
mindestens 90 % Kompression aller gelandeten Spielerflüge. Check-Untergrenze,
Buchungsabgleich, Random-Access und Neustartvergleiche bleiben erhalten.
Historische Modusnamen wie `write-v25` der älteren Testfamilien bleiben
kompatibel; ihre heutigen Writer speichern v26. Die neue Disruption-Familie
erzeugt dagegen ausdrücklich einen feldbereinigten v25-Spielstand.
Keine Änderung an Rust, Spielregeln, Wirtschaft, Konfiguration oder Saveformat.

Renderreproduktion (Zielverzeichnis vorher anlegen):

```sh
export XDG_DATA_HOME="$(mktemp -d)"
Godot --headless --path . --editor --import
Godot --headless --path . --script res://tests/disruptions_test.gd
export LD_LIBRARY_PATH=/tmp/air33-libs/extracted/usr/lib/x86_64-linux-gnu
export LIBGL_ALWAYS_SOFTWARE=1 LP_NUM_THREADS=2
Xvfb :190 -screen 0 1600x1200x24 -nolisten tcp -ac &
capture_xvfb_pid=$!
sleep 1
DISPLAY=:190 Godot --path . --display-driver x11 \
  --rendering-method gl_compatibility --audio-driver Dummy \
  --script res://tests/disruptions_test.gd -- --capture-dir=/tmp/disruption-renders
kill "$capture_xvfb_pid"
```

`xvfb-run` scheiterte vor dem Engine-Start am fehlenden `xauth`; der
Renderlauf verwendet deshalb direkt Xvfb (lokaler Display-Socket, kein TCP).
Der erste direkte Start scheiterte an fehlendem XCursor im System-Suchpfad;
mit dem vorhandenen Bibliotheksverzeichnis aus den früheren Verifikationen
und Mesa-Software-Rendering startet die Engine. Diese Startfehler zählen nicht
als bestandene Renderprüfungen.

Der Test stellt selbst 1440×1000, 1100×720 und 680×520 ein. Zusätzliche
Renderzustände zeigen Pünktlichkeit „—“ vor der ersten Landung und den späteren
Anteil nach Erholung; Mausbewegungen zeigen die nativen Fleet-Status- und
Routen-/Flugkostentooltips. Alle 27 Originale wurden tatsächlich visuell geprüft:
49/70 Minuten Rückstand im Betrieb, vollständiger 70-Minuten-Fleet-Tooltip,
„—“, 0.0 % und später 63.6 % (11 Flüge), 336 EUR Betreuung jeweils im Routen-
und Flugtooltip, Aufmerksamkeitszeile und deren Verschwinden nach Erholung.
Die Fleet-Zelle kürzt langen Status mit Ellipse; der native Hover zeigt ihn
vollständig. Kleine Fenster benötigen vertikales/horizontales Scrollen; bei
680×520 bricht die Prozentzeile um, bleibt aber lesbar. Kein Produktionsfehler
bestätigt. Originale und Linux-Transkripte sind an
[AIR-90](https://paperclip.iamsec.de/AIR/issues/AIR-90) gespeichert.

Alle 30 eigenständigen Godot-Tests, Hauptszene, Editor-Import und sämtliche
18 Neustartfamilien sind sequenziell bestanden (90 erforderliche erfolgreiche
Aufrufe; zunächst fehlgeschlagene Assertions wurden korrigiert und gezielt
wiederholt). Die neue Disruption-Familie bestand alle vier Prozesse, inklusive
vollständigem Spieler-/KI-Abgleich sofort und nach 100 Tagen. Der große
Godot-Finanzarchiv-Writer bestand in 364,26 s, sein frischer Leser in 131,52 s.
Der abschließend erweiterte Disruption-Test bestand erneut headless und mit
Xvfb/Mesa-llvmpipe. Frische isolierte XDG-Verzeichnisse für Suite und Renderlauf.

Locked-Rust-Build, 218 Rust-Tests und Clippy mit `-D warnings` bestanden.
Testkommando: `cargo test --locked --manifest-path rust/Cargo.toml -- --skip
million_flight --test-threads=1`. Die zwei Rust-Millionenflugtests wurden wie
beauftragt ausgelassen, da Rust unverändert ist; kein neuer Benchmarknachweis.

START.bat, die tatsächlichen PowerShell-Wrapper, native Windows-DLLs und
Windows-Neustarts sowie Windows-Darstellung/-Skalierung bleiben Besitzerchecks.
Linux-Neustarts und Linux-Renderbilder ersetzen diese Prüfungen nicht.

## Pünktlichkeitsruf (Spielstand v28)

Jede Airline beginnt neutral. Rust sammelt nur neu gelandete, erfasste Flüge
je absolutem Spieltag. Um Mitternacht folgt ein ganzzahliger Rufwert dem
Tagesergebnis mit 1/21 des Abstands zum Ziel. 90 % pünktliche Ankünfte sind
neutral; 0 % entsprechen dem negativen, 100 % dem positiven Endpunkt.
Ein einzelner Flug ändert den Ruf nicht sofort; Tage ohne erfasste Landungen
halten den Wert und zeigen einen stabilen Trend. Alle Parameter einschließlich
Warnschwelle stehen in `config/disruptions.json`.

Der Ruf verändert ausschließlich das vorhandene Angebotsgewicht neben Marke
und Service, höchstens um ±3 %. Zahlungsbereitschaft, Preisgrenzen, Kapazität und
Fracht bleiben unverändert. Spieler und KI verwenden dieselbe Regel. Abflugzitate
werden weiterhin eingefroren, auch am Tageswechsel. Die Airlineübersicht zeigt
den aktuellen Gewichtsbonus/-malus und den letzten Tagestrend; bei deutlich
negativem und weiter sinkendem Ruf erscheint ein Aufmerksamkeitseintrag.

v28 speichert Ruf, Tagestrend und noch nicht abgeschlossene Tageszähler.
Fehlende Felder aus v27 und älteren Spielständen beginnen für alle Unternehmen
neutral, ohne die Finanzhistorie erneut auszuwerten. Bereits fliegende Zitate,
Flugzeiten und Betreuungskosten bleiben erhalten. Absolute Tagesgrenzen und
Ganzzahlarithmetik machen Pause, Geschwindigkeit, Schrittgröße und Laden
unabhängig. Rust-Tests prüfen Migration mit Flügen in der Luft, schlechte Serien
und Erholung, Grenzen, KI-Parität, Neustart sowie den Zehnjahresvergleich mit
neutralem Nachfragefaktor. Godot-/Windows-Ausführung und Sichtprüfung dieser
Erweiterung sind auf dem Entwicklungsserver nicht verifiziert.

Der Rust-Zehnjahresvergleich mit identischem Startzustand und ausgeschaltetem
Rufgewicht als Kontrolllauf ergibt für Regional Wings −2,65 % und für
Continental Sky +9,34 % kumulierten Umsatz nach zehn Jahren. Beide KI-Airlines
bleiben ohne Notlage; wöchentliche Wachstumsentscheidungen können auseinanderlaufen.
Diese langfristige Umsatzdifferenz ist deshalb nicht mit der harten ±3-%-Grenze
für ein einzelnes Angebotsgewicht gleichzusetzen.
