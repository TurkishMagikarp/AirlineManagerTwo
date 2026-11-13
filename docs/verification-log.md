# Verifikationsprotokoll

Historische Testläufe, Messungen und offene Vorschläge (aus dem README ausgelagert).

## Vor der KI-Erweiterung

Vor der KI-Erweiterung verifiziert: 54 Rust-Tests, Clippy ohne Warnungen, Nachfrage-UI samt separatem
Ladeprozess, bestehende Godot-Tests und alle vier
Prozess-Neustarttests sowie `START.bat --headless --quit-after 10` erfolgreich.
Reale Grafikprüfung bei 1440×1000, 1100×720 und 680×520 erfolgreich. Gemessen im
Debug-Build auf diesem Rechner: 100 Abfragen neuester/ältester Flugseiten aus einer
Million Flüge ca. 59–72 ms insgesamt (kein Benchmark der Weltkarte).

## KI-Erweiterung

KI-Erweiterung verifiziert: **61 Rust-Tests**, Clippy ohne Warnungen, alle zehn
bisherigen Godot-Tests, der neue Wettbewerbs-UI-Test und alle fünf
Prozess-Neustarttests. `START.bat --headless --quit-after 10` funktioniert weiter.
Die neue Testmatrix prüft gemeinsame Nachfrage bei gemischten Preisen und
Frequenzen, Sitzgrenzen, Preiswettbewerb, getrennte Konten, wöchentliche
Preisentscheidungen/Stilllegung, unveränderte laufende Flüge, Altspielstand-Migration,
beschädigte KI-Daten, transaktionale Fehler sowie identische Ergebnisse bei
großen/kleinen Schritten und 1×/4×/16×. Der Millionentest mit zwei Spielerrouten
bleibt kompakt; tatsächliche KI-Marktänderungen erzeugen zusätzliche Archivblöcke.

KI-Wiedereinstieg verifiziert: **62 Rust-Tests** (inkl. Wiederaufnahme nach
Stilllegung und Warten bei unrentablem Markt), Clippy ohne Warnungen, alle
Godot-Tests und alle fünf Prozess-Neustarttests.

KI-Expansion verifiziert: **66 Rust-Tests**, Clippy ohne Warnungen, elf Godot-Tests
und alle fünf Prozess-Neustarttests. Neu geprüft: echte Käufe und Kontenabgleich,
freie Maschinen statt Neukauf, begrenztes Budget, streckenbezogene Schließungen
mit erhaltenem Archiv, gemeinsamer Passagiermarkt, Flottenlimit nach fünf Jahren,
kompakter speicherbarer Langzeitstand und alte Version-9-KI-Felder. Der Godot-Test
prüft zusätzlich Expansion über ein Jahr und das Laden vergrößerter Flotten.
`START.bat --headless --quit-after 10` weiterhin erfolgreich. Noch keine Überführungen,
Flugzeugverkäufe, Kredite, Insolvenz oder globale Netz-/Frequenzoptimierung;
KI-Entscheidungen bleiben bewusst eine kleine deterministische Heuristik.

## Qualitätscheck / Langzeittest (13.11.2026)

71 Rust-Tests, Clippy ohne Warnungen, 13 Godot-Testskripte (zusätzlich separate
Nachfrage-/Abflugergebnis-Ladeprozesse) und alle fünf Prozess-Neustarttests.
Vier Szenarien laufen jeweils zehn Jahre: ohne Spielerbetrieb, kostenlose Tickets,
90 EUR und 100.000 EUR auf Frankfurt–München. Jährliche Sprünge und tägliche Schritte
mit jährlichem Speichern/Laden ergeben exakt dieselben Zustände. Zusätzliche
Marktprüfung: alle 28 Flughafenpaare mit drei gemischten Angeboten, Preisen,
Frequenzen und umgekehrten Richtungen; keine Überschreitung des Basismarkts.

Behoben: Verluststrecken konnten bei stets fliegender Maschine zum wöchentlichen
Prüfzeitpunkt unendlich weiterfliegen. Die Schließung hat nun eine eigene
Ereignisgrenze am Bodenzeitende, auch ohne freien Abstand zum nächsten Abflug;
Speichern/Laden und kleine/große Schritte bleiben identisch. Frames ohne
Ganzsekundenwechsel kopieren keine Unternehmensarchive; Finanzlisten werden nur
bei tatsächlichen Landungen kopiert und sortiert. Die UI aktualisiert unveränderte
Uhr-/Statusdarstellung nicht mehr pro Frame.

Karte: statischer Hintergrund getrennt, Landflächen einmal trianguliert,
Offscreen-Weltkopien ausgespart; keine erneute Kartenprojektion je Flugzeug-Update.
Gemessen mit echtem OpenGL-Renderer auf RTX 3080, Godot 4.7.2, Debug-Build,
1100×720, VSync aus, je 120 Frames (Median / P95 in ms):

| Aktion | Vorher | Nachher |
| --- | ---: | ---: |
| Flugzeug-Layer neu zeichnen | 52,06 / 71,34 | 4,17 / 4,66 |
| Verschieben | 51,26 / 59,32 | 10,71 / 16,36 |
| Zoomen | 51,08 / 57,30 | 6,71 / 10,48 |

Diese Messung isoliert die Karte, nicht die gesamte laufende Simulation.
Reproduktion: `Godot --path . --script res://tests/map_benchmark.gd --disable-vsync --max-fps 0`.
Rust-Messungen/Szenarien: `cargo test --locked --manifest-path rust/Cargo.toml quality:: -- --nocapture --test-threads=1`.
Im isolierten Rust-Debug-Test kosten 8.192 Frames mit 1/64 s nach einem Jahr
im Mittel ca. 1,2 / 4,0 / 15,7 µs bei 1× / 4× / 16×. Mit 3.650 Preiswechseln
sind es bei 16× ca. 0,42 ms/Frame; transaktionale Kopien an echten
Sekundengrenzen wachsen weiterhin mit dem Archiv (kein unbegrenzter Stresstest).
Normale Zehnjahresstände benötigen jetzt ca. 47–68 KB statt 116–159 KB.
3.650 tägliche echte Preiswechsel erzeugen berechtigt 3.650 Archivblöcke;
der Stand benötigt ca. 2,55 MB und ist nun speicher-/ladbar (vorher über 1-MiB-Limit).
Das Archiv wird nicht gekürzt; die neue Größenobergrenze schützt weiterhin die
vorherige Datei. Beliebig viele Wirtschaftsänderungen sind weiterhin nicht unbegrenzt speicherbar.

**Offene Wirtschaftsvorschläge, nicht umgesetzt:**

- Ohne Spielerangebot wachsen Regional Wings / Continental Sky nach zehn Jahren
  auf ca. 54 / 167 Mio. EUR Guthaben bei nur einem / zwei Flugzeugen. Buchungen
  stimmen; dauerhaft konstante Nachfrage, niedrige Testkaufpreise und fehlende
  Fixkosten, Alterung, Finanzierung und Steuern begünstigen die hohen Renditen.
  Diese Größen gemeinsam kalibrieren, nicht einfach KI-Konten deckeln.
- Kostenlose Spielertickets führen nach zehn Jahren zu ca. −168 Mio. EUR Guthaben,
  überteuerte Leerflüge zu −89 Mio. EUR. Negative Salden sind ausdrücklich erlaubt;
  Liquiditätswarnung, Kreditlimit oder Insolvenz brauchen eine eigene Spielregel.
- Der gemeinsame Markt bleibt begrenzt, aber wegen der bewusst fehlenden
  Kapazitätsumverteilung können ausgebuchte Billigangebote Nachfrage binden,
  die andere Airlines nicht bekommen. Eine kapazitätsbewusste zweite Verteilung
  wäre eine Wirtschaftsänderung, kein reiner technischer Fix.

## Wartung verifiziert (13.11.2026)

**80 Rust-Tests**, Clippy ohne Warnungen, 14 Godot-Testskripte und alle sechs
Prozess-Neustarttests; `START.bat --headless --quit-after 10` funktioniert weiter.
Neu geprüft: exakte Abnutzung je Landung, sofortige Wartung geparkter Flugzeuge,
Wartungsauftrag in der Luft (Flug wird beendet und regulär abgerechnet), keine
Abflüge während der Wartung, einmalige Abbuchung, Fortsetzung vom tatsächlichen
Standort, erzwungene ungeplante Wartung an der Grenzlandung, fehlendes Guthaben,
gelöschte Routen, identische Ergebnisse bei 1×/4×/16× und großen/kleinen Schritten,
Speichern/Laden in jeder Phase (auch in getrennten Prozessen), Version-9-Migration,
beschädigte Wartungsdaten und KI-Wartung (regulär geplant, ungeplant ohne Guthaben).
Ein echter lokaler Version-6-Spielstand lädt, läuft weiter und wird als Version 10 gespeichert.

Auswirkungen: Jede Wartungsunterbrechung beginnt einen neuen Archivblock, ebenso
Marktänderungen durch KI-Flugzeuge in Wartung. Normale Zehnjahresstände benötigen
jetzt ca. 92–243 KB (vorher 47–68 KB); der Extremtest mit rund zwei Millionen Flügen
über ca. 228 Spieljahre ca. 4,9 MB (Limit 8 MiB).

## Wartung: KI-Kosten, Routen-Tab, Stornierung (13.11.2026)

**83 Rust-Tests**, Clippy ohne Warnungen, alle Godot-Testskripte und alle sechs
Prozess-Neustarttests; `START.bat --headless --quit-after 10` endet mit Code 0.
Neu geprüft: KI-Streckenprüfung zählt eine Woche als Verlust, wenn der Betriebsgewinn
den Wartungsanteil der Landungen nicht deckt (ältere Spielstände ohne Landungszähler
werden in der ersten Woche wie bisher bewertet); Stornierung eines noch nicht
begonnenen Auftrags nach Speichern/Laden bei 1×/4×/16× ergibt exakt denselben Zustand
wie ohne Auftrag, keine Abbuchung; laufende Wartung nicht stornierbar; Routen-Tab
sperrt „Flugplan aktivieren“ während der Wartung und zeigt den Status, der nach
Wartungsende wieder verschwindet. Spielstandversion unverändert (10), neues Feld additiv.

## 2026-11-13 — Linux Godot regression verification (AIR-8)

- Official Godot 4.7.2 Linux x86_64 (`ed1daf0bf`), downloaded outside the repo; debug Rust `.so` built with the temporary external Zig linker wrapper (server has no `cc`). `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml` passed; 126 Rust tests, including migrations, determinism and AI economy coverage.
- All 19 `tests/*_test.gd` scripts passed: 18 standalone scripts plus `save_restart_test.gd` through its four explicit modes. `map_benchmark.gd` excluded. Main scene `--headless --quit-after 10` passed. Layout sizes: 1440×1000, 1100×720, 680×520; staff UI also covers 800 pixels.
- All eight restart scenarios passed as separate Linux Godot processes: competition, economy, finances, financing, flight, maintenance, staff, and save (running/paused, Unicode filename). These run the `.gd` phases from the PowerShell scripts, not PowerShell or START.bat.
- Fixed stale fleet ownership and competition/spill assertions. Added owned/leased disposal blocking checks in flight and maintenance, the real restructuring deadline/action in Finances, and positive Routes spill/capacity checks. Made the route-results reachability assertion run without screenshot capture. No gameplay, balance, save-format or toolchain changes.
- Screenshots unavailable: Xvfb exists, but display startup lacks `libXcursor.so.1`/Wayland libraries. Headless geometry checks do not verify appearance. Owner still needs Windows START.bat, the PowerShell restart scripts, and visual inspection of fleet trading, restructuring and spill panels.
- First editor import completed assets but crashed during exit; one subsequent `--headless --editor --import --quit` passed. Runtime tests and startup were unaffected; no engine workaround committed.


## 2026-11-13 — Linux Godot verification after PR #15 (AIR-22)

- Verified main `417d0b9` with official Godot 4.7.2 Linux x86_64 (`ed1daf0bf`) and a freshly built debug Rust extension; external Zig `cc` wrapper, isolated `LOCALAPPDATA` / `XDG_DATA_HOME`, 1,800-second timeout per Godot process. `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml` passed: 155 Rust tests, no Clippy warnings.
- All 20 `tests/*_test.gd` scripts passed (19 standalone scripts, plus `save_restart_test.gd` in its four explicit modes). The finance script passed in 250.54 s (overlapping the Rust checks); the separate-process finance write/restore phases passed in 235.48 / 98.76 s. Both include the funded century-scale advance, archive save, chart/page/UI assertions, and continuation; timings include full process startup and exit. Editor import and main scene `--headless --quit-after 10` passed; `map_benchmark.gd` is not part of the test suite.
- All eight restart scenarios passed with each phase in a separate Linux Godot process: competition, economy, finances, financing, flight, maintenance, staff, and save (running/paused, Unicode filename). Used the same `.gd` commands/arguments as `tests/run_*_restart_test.ps1`; substituted direct Linux main-scene startup for the save wrapper’s Windows `START.bat` preflight. The migration script also passed its separate `--restore-migration` invocation after the suite’s write phase.
- No stale assertions or shared-logic bugs found; no gameplay/balance changes, no performance changes, save version remains 16. Headless layout coverage includes 1440×1000, 1100×720, 680×520, and the staff panel at 800 pixels.
- Owner still needs a real Windows/GUI machine for `START.bat --headless --quit-after 10`, all eight actual PowerShell restart wrappers (including Windows startup/process/DLL behavior), and visual inspection/screenshots of the populated fleet trading, Routes capacity/spill/frequency, global fuel/seasonal demand, restructuring/Finances, and staff panels. Linux headless geometry checks do not establish that the UI looks right.


## 2026-11-13 — Two-class cabin Godot verification (AIR-28 / PR #17)

- Base `c4768fc`; official Godot 4.7.2 Linux x86_64 (`ed1daf0bf`), debug extension built with the external Zig `cc` wrapper, isolated test data. Rust build/test/clippy passed with `--locked`: 161 tests, no Clippy warnings.
- All 21 `tests/*_test.gd` scripts, editor import and main-scene `--headless --quit-after 10` passed. All eight existing restart scenarios passed with separate Linux processes, plus legacy market migration and new v16/v17 cabin restart phases. The finance stress test required the established 1,800-second process limit; an initial 120-second harness limit interrupted it and was corrected before successful verification.
- Added real cabin/fare button coverage: quoted fee and single debit, unchanged layout free, parked/landed refits, blocked airborne/hangar refits, independent fares, per-class load/revenue, and controls reachable at 1440×1000, 1100×720 and 680×520. A v16 fixture without cabin fields restores all-economy; v17 preserves business seats, both fares and class results across processes. Added `run_cabin_restart_test.ps1` for owner-side reproduction.
- Fixed stale Fleet label and legacy fixture/save-version assertions. No Rust gameplay, balance or migration changes were needed. Existing Rust tests cover deterministic steps/speeds, old saves and AI cabin/economy effects.
- Still requires the owner’s Windows/GUI machine: `START.bat`, all actual PowerShell wrappers (including the new cabin wrapper), Windows DLL/restart behavior, screenshots and visual review of Fleet cabin controls, Routes fares/results and the three window sizes. Linux headless geometry checks do not establish visual appearance.

## 2026-11-13 — AIR-33: Bordservice in Godot

- Auf `414095f` mit offizieller Linux-Version Godot 4.7.2 (`ed1daf0bf`) und neu gebauter Rust-Erweiterung geprüft. `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml` erfolgreich: 166 Tests, keine Clippy-Warnungen; bestehende Service-Tests prüfen auch KI und Determinismus. Keine Spielregel-, Balance- oder Rust-Änderungen.
- Alle 21 eigenständig ausführbaren `*_test.gd`-Skripte sowie Start der Hauptszene (`--headless --quit-after 10`) erfolgreich. Der erste Editor-Import stürzte beim Beenden ab; der zweite Import mit Dummy-Audio war erfolgreich. `map_benchmark.gd` gehört nicht zur Testsuite.
- Neuer Service-Test bedient Auswahlmenü und Flugplan-/Speicher-/Ladeknöpfe mit echten Mausereignissen: unabhängige Routen, Basic/Standard/Premium, unveränderte Abflugangebote trotz Servicewechsel, spätere Basic-Kosten sowie exakte Übereinstimmung von Archiv, Tooltips und Finanzen. Separate v17/v18-Schreib-/Leseprozesse prüfen Standard/Nullkosten bei Migration, erneutes Speichern als v18 und erhaltene Routenauswahl samt Premium-Angebot eines laufenden Fluges. Neuer Windows-Wrapper: `tests/run_service_restart_test.ps1`.
- Veraltete Versionsannahmen in Kabinen- und v14/v15-Migrationsfixtures korrigiert; alte Fixtures entfernen Servicefelder ohne Rundung von Rust-u64-Werten. Kabinenphasen v16/v17 und erneuter Migrationsprozess erfolgreich.
- Service-Auswahl, Kosten, Flugergebnisse und Finanzzeile bei 1440×1000, 1100×720 und 680×520 per Scroll erreichbar. Neun echte Linux-Aufnahmen der Routen-/Service-/Finanzansichten auf AIR-33 hochgeladen: Xvfb/X11, Mesa llvmpipe, GL-Kompatibilität, externe Xcursor-/Wayland-Bibliotheken außerhalb des Repos. Reproduzierbar mit `service_test.gd -- --capture-dir=<vorhandenes Verzeichnis>` im Rendererbetrieb.
- Weiter auf Windows zu prüfen: `START.bat`, PowerShell-Wrapper einschließlich Service-Neustart, DLL-/Prozessverhalten und Darstellung mit dem nativen Windows-Renderer. Headless-Geometrie allein beweist keine korrekte Darstellung; Linux-Aufnahmen ersetzen diese Windows-Prüfung nicht.

- Abschließend auch alle acht bisherigen Neustartszenarien in separaten Linux-Prozessen erfolgreich: Konkurrenz, Wirtschaft, Finanzarchiv, Finanzierung, Flugbetrieb, Wartung, Personal und laufender/pausierter Spielstand mit Unicode-Dateiname. Damit alle 22 `*_test.gd`-Skripte inklusive der vier expliziten Speicherphasen erfolgreich; Finanzarchiv mit ungefähr zwei Millionen Flügen und frischem Leseprozess geprüft.

## 2026-11-13 — AIR-38: Ertragsteuer in Godot und Prozess-Neustarts

- Basis `82ceb77`, offizielles Linux-Godot 4.7.2 (`ed1daf0bf`), frisch gebaute Rust-Debug-Erweiterung, externer Zig-Wrapper und isolierte Datenverzeichnisse. Keine Spielregel-, Balance-, Rust- oder Toolchain-Änderungen.
- Neuer `tax_test.gd` bedient die Finanznavigation und den Ertragsteuer-Diagrammwähler mit Mausereignissen. Jahr 0, kompletter Steuerhinweis, alle Tabellenzellen einschließlich Nullsteuerjahr und Diagrammsumme stimmen mit Rust-API/Archiv überein. Echte Gehälter erzeugen im ersten Jahr 578.400 EUR Verlustvortrag; im zweiten Jahr werden bei 2.764.043 EUR Vorsteuergewinn 655.693 EUR Steuer gebucht, Vortrag danach null.
- Vier getrennte Linux-Prozesse prüfen v18 ohne Steuerfelder (auch KI), keine Nachbesteuerung/Verlustübernahme, ausschließlich Gewinn seit Laden im nächsten Abschluss, erneutes Speichern als v19 sowie einen v19-Mittejahresstand mit Verlustvortrag. Vollständiger serialisierter Zustand und API-Sichten stimmen vor und nach Fortsetzung exakt mit dem ununterbrochenen Referenzlauf überein. Windows-Reproduktion: `tests/run_tax_restart_test.ps1`.
- Alte Kabinen-, Service- und Markt-Migrationsfixtures auf Version 19 angepasst; die benannten v17/v18-Servicefixtures verwenden tatsächlich ihr historisches Format ohne Steuerfelder. Sechs Linux-Finanzaufnahmen auf der Aufgabe: Hinweis, Verlust-/Gewinnjahr, Tabelle, Auswahl und Diagramm; Xvfb/X11, GL-Kompatibilität und llvmpipe mit externen Xcursor-/Wayland-Bibliotheken. Steuerbereich und Auswahl bei 1440×1000, 1100×720 und 680×520 per Scroll erreichbar; schmale Tabellen nutzen zusätzlich internes Scrollen.
- Auf Windows weiter offen: `START.bat`, die tatsächlichen PowerShell-Wrapper (einschließlich Steuerneustart), DLL-/Prozessverhalten und native Darstellung. Linux-Aufnahmen und Headless-Geometrie ersetzen diese Windows-Prüfung nicht.
