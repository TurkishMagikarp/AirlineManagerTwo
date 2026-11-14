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

- Abschließende Regression erfolgreich: `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml` (170 Rust-Tests, keine Clippy-Warnungen), alle 23 `*_test.gd`-Skripte einschließlich vier expliziter Speicherphasen, Editor-Import und Hauptszene `--headless --quit-after 10`. Alle acht bisherigen Neustartszenarien sowie Kabinen-v16/v17, Service-v17/v18, Steuer-v18/v19 und erneuter Markt-Migrationsprozess erfolgreich. Rust-Tests enthalten Determinismus, alte Spielstände und KI-Wirtschaft/Steuer. Lange Archivprüfungen nach Abbruch durch das Befehlszeitlimit mit unverändertem Test und ausreichend langem externen Runner vollständig abgeschlossen.


## 2026-11-14 — AIR-44: Marketing v20 unter Linux

- Grundlage: zusammengeführte PR #23 (`03bab99`); offizielles Linux Godot 4.7.2 (`ed1daf0bf`), Rust-Debug-GDExtension, isolierte `XDG_DATA_HOME`-/`LOCALAPPDATA`-Verzeichnisse. Build mit bereitgestelltem Zig-`cc`-Wrapper; keine Toolchain-/Engine-Anpassungen im Repository.
- `tests/marketing_test.gd` erfolgreich headless und mit Xvfb/X11/OpenGL-Kompatibilität/Mesa llvmpipe ausgeführt. Tastatur-/Mausereignisse übernehmen 73.333 EUR, 30.000 EUR und null; Budgetwechsel ändern Bekanntheit/Trend nicht sofort. Tagesabschlüsse erhöhen Bekanntheit mit positivem Trend; nach Abschalten sinkt sie mit negativem Trend. Statusanzeige und neueste Archivbuchungen (Zeit und Betrag) stimmen mit Rust überein; Archiv, Kostenreihe und Unternehmenssummen sind im Leerlaufszenario abgestimmt.
- Marketingstatus, Budgeteingabe, Übernahmebutton und Archiv bei 1440×1000, 1100×720 und 680×520 per Scroll erreichbar. Neun echte gerenderte Finanzen-Aufnahmen (Status/Eingabe/Archiv je Größe) als Anhänge auf AIR-44; ausgewählte Aufnahmen visuell geprüft. Auf kleinen Fenstern werden verschiedene Scrollpositionen benötigt; das Archiv hat zusätzlich eine eigene Scrollleiste. Headless-Prüfungen allein belegen keine Darstellung.
- Vier getrennte Linux-Godot-Prozesse (`write-v19`, `read-v19`, `write-v20`, `read-v20`) erfolgreich. v19-Fixture ohne Marketingzustand: alle Unternehmen mit Budget/Bekanntheit/EUR-Rest null und leerem Marketingjournal. v20 mitten im Monat: Spieler und beide KI behalten Budget, Bekanntheit und einen tatsächlich nichtnulligen EUR-Rest. Vollständige serialisierte Zustände vor/nach Fortsetzung sind bytegleich zur ununterbrochenen Referenz, einschließlich KI und ganzzahliger Zeit-/Rundungsdaten. JSON wird nur für Assertions geparst, nicht neu serialisiert.
- Testabdeckung ergänzt: vollständiger Save statt ausschließlich UI-Snapshots, explizites Schließen der Snapshot-Dateien, Screenshotoption und Vergleich von Archivzeit/Betrag. Rust prüft zusätzlich Finanznot zwischen Wochenprüfungen: Budget beim nächsten Tagesabschluss null, danach keine weiteren Marketingkosten bis in die Restrukturierung. Im bestehenden Langzeitaudit wird die KI-Zahlungsfähigkeit nun auch täglich geprüft. Keine Spielregel-/Balanceänderungen.
- Zehnjahresaudit mit vier Spielerszenarien erfolgreich (tägliche gegen große Schritte, jährlicher Save/Load, beide KI). KI-Marketing bleibt unter 10 % des Umsatzes, keine tägliche Finanznot/Restrukturierung. Im durchlaufenden 90-EUR-Szenario nach zehn Jahren: KI-Guthaben 170.261.977 / 177.245.265 EUR; Marketing kumuliert 2.428.666 / 2.400.051 EUR. In insolventen Spielerszenarien endet die Weltuhr regulär früher; diese sind kein Beleg für zehn tatsächlich verstrichene Simulationsjahre.
- Windows bleibt offen: `START.bat`, tatsächliche PowerShell-Wrapper, Windows-DLL-/Prozessverhalten und native Windows-Darstellung. Linux-Prozessphasen ersetzen nicht die Ausführung der `.ps1`-Dateien auf Windows.

- Abschließende Regression: `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml` erfolgreich; 177 Rust-Tests, keine Clippy-Warnungen. Der verschärfte Zehnjahresaudit zusätzlich separat erfolgreich. Alle 23 eigenständig ausführbaren Godot-Tests, vier explizite Save-Modi, Editor-Import und Hauptszene (`--headless --quit-after 10`) erfolgreich. Der Finanzen-Stresstest prüfte etwa zwei Millionen Flüge inklusive Wartungsunterbrechungen und kompaktem Archiv. Steuer-/Service-/Kabinen-/Markt-Migrationsprüfungen und ihre separaten Prozessphasen erfolgreich; zusätzlich Marketing-v19/v20 und die übrigen Neustartszenarien geprüft.


## 2026-11-14 — AIR-49: Treibstoffverträge in Godot und v21-Neustarts

- Basis `b400ec8`, offizielles Linux-Godot 4.7.2 (`ed1daf0bf`), frisch gebaute Rust-Debug-Erweiterung mit externem Zig-Wrapper. Isolierte Verzeichnisse für `XDG_DATA_HOME`, `LOCALAPPDATA` und Renderdaten außerhalb des Repos; kein produktiver Spielstand benutzt. Keine Spielregel-, Balance-, Rust- oder Speicherformatänderungen.
- Neuer `tests/fuel_contract_test.gd`: echte Mausereignisse wählen die Vertragsoptionen; die Leertaste bestätigt den fokussierten Abschlussknopf. Alle neun Kombinationen aus 25/50/75 % und 3/6/12 Monaten prüfen Anteil, sekundengenaue Laufzeit und aufgerundeten Marktpreis plus konfiguriertem Aufschlag. Zweiter Abschluss, Finanznot und Restrukturierung sind in Oberfläche und Rust transaktionssicher gesperrt. Eine testinterne negative Anfangseinlage erzeugt Finanznot mit passendem Eröffnungssaldo; der Rust-Zeitablauf führt anschließend in die Restrukturierung.
- Abgerechnete Flugangebote werden mit der gleichen physischen Treibstoffmenge zu ihrem Abflug-Marktpreis verglichen. Positive Mehrkosten und negative Ersparnis werden geprüft; Marktpreise stammen aus einer separaten Rust-Simulation, nicht aus nachgebautem GDScript-Zufall. Status, Restsekunden, kumulierter Vertragseffekt und Freigabe nach Ablauf stimmen mit der Rust-API überein.
- Vier getrennte Linux-Prozesse für v20/v21: echtes v20-Fixture ohne Vertrags-/Effektfelder, ohne historische Vertragsabrechnung bei Spieler oder KI; nach Migration ist der vollständig serialisierte Zustand einschließlich Historie bytegleich zum neutralen Referenzzustand. v21 erhält aktive Verträge und nichtnull kumulierte Effekte bei Spieler und beiden KI-Gesellschaften sowie laufende Abflugangebote. Vollständige Saves und API-Sichten stimmen direkt nach Neustart und nach weiteren 100,125 Tagen einschließlich Ablauf exakt mit dem ununterbrochenen Lauf überein. Windows-Reproduktion: `tests/run_fuel_contract_restart_test.ps1`.
- Veraltete Writer-/Resave-Annahmen von Version 20 auf 21 korrigiert: Kabinen-, Service-, Markt-, Steuer- und Marketingtests. Die aktuellen Steuer-/Marketing-Neustartmodi und zugehörigen Wrapper heißen jetzt v21; historische v14/v15/v16/v17/v18/v19-Migrationsphasen bleiben erhalten.
- Alle Vertragskontrollen per Scroll erreichbar und Auswahlmenüs bedienbar bei 1440×1000, 1100×720 und 680×520. Sechs echte Finanzaufnahmen (aktiver/abgelaufener Vertrag an allen Größen) auf AIR-49 hochgeladen und visuell geprüft: Xvfb/X11, OpenGL-Kompatibilität, Mesa llvmpipe, Dummy-Audio und externe Bibliotheken. Beispiel: kumuliert +4.856 EUR bei aktivem Vertrag, −30.544 EUR nach Ablauf. Diese Linux-Aufnahmen ersetzen keine native Windows-Darstellung.
- Umgebungsbefunde: erster Editor-Import stürzte beim Beenden ab; erneuter Import mit explizitem `--quit` erfolgreich. `xvfb-run` fehlte `xauth`, daher Xvfb direkt gestartet. Erster Renderlauf bestand die Tests, meldete aber fehlendes Audio; abschließender Lauf mit `--audio-driver Dummy` erfolgreich. Lange Rust-/Finanzprüfungen wurden zunächst vom Ausführungswrapper beendet und anschließend mit dauerhafter Ergebniserfassung neu gestartet.
- Weiter auf einem echten Windows-Rechner offen: `START.bat`, Ausführung aller tatsächlichen `.ps1`-Wrapper einschließlich Treibstoffneustart, Windows-DLL-/Prozessverhalten und native Darstellung. Linux-Prozessphasen und Headless-Geometrie sind kein Ersatz dafür.

- Abschließende Regression erfolgreich: `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml`, 181 Rust-Tests, keine Clippy-Warnungen einschließlich Zehnjahres-KI-Vergleich und Determinismus-/Migrationsprüfungen. Alle 24 eigenständig ausführbaren Godot-Tests, Editor-Import, Hauptszene (`--headless --quit-after 10`), vier explizite laufende/pausierte Save-Modi mit Unicode-Dateiname und sämtliche 13 Neustartszenarien als separate Linux-Prozessphasen erfolgreich; außerdem frischer Markt-Migrationsprozess. Insgesamt 65 erforderliche Godot-Aufrufe geprüft. Finanzarchiv-Stresstest mit ungefähr zwei Millionen Flügen einschließlich Wartungsunterbrechungen und frischem Leser erfolgreich. Die tatsächlichen Windows-Wrapper wurden weiterhin nicht ausgeführt.
