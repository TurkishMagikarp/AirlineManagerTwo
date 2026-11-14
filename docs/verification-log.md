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

## 2026-11-14 — AIR-51 Bauchfracht

- Rust-Bauchfrachtmarkt mit gemeinsamer Tagesnachfrage je Flughafenpaar/Richtung, Preisreaktion, Kapazitätsüberhang und Passagier-/Gepäckvorrang; Modellkapazitäten und `config/cargo.json`. Standard 1,50 EUR/kg; KI behält diesen Preis.
- Bei Abflug eingefrorene kg/Umsätze; Buchung bei Landung. Separate Frachtkomponente in Finanzen, Gesamtgewinn und Steuerbasis. Routenpreis, Tagesmarkt, bisherige kg/Umsätze sowie getrennte Ticket-/Frachtanzeigen in Route und Finanzen ergänzt.
- Save v22: alte Routen erhalten den Standardpreis, alte Historie und laufende Angebote bleiben ohne nachträgliche Fracht. Godot-/Windows-Fixtures auf aktuelle Versionsnummer angepasst.
- Sieben Rust-Frachttests: volle Nutzlast, Konkurrenz/Überhang, Gegenrichtung, unterschiedliche Frequenzen/Preise, Preiswechsel im Flug, Wartung, verweigerter Verkauf in der Luft und Verkauf am Boden, Spieler-/KI-Migration, beschädigte Nutzlast, Steuerbasis, zwei Jahre KI und 1×/4×/16×/Schrittgrößen/Neustart. KI bleibt solvent; Fracht < 10 % des Ticketumsatzes. Gemeinsame Passagierallokation gegen Einzelangebote mit kollidierenden lokalen IDs und verschiedenen Modellen/Kabinen/Preisen/Saisons geprüft.
- Build und Clippy (`--locked`, bereitgestellte Rust-Toolchain und Zig-cc-Wrapper): bestanden. Finale vollständige Rust-Suite (`--test-threads=2`, weniger CPU-Konkurrenz für die unveränderte 300-s-Performancegrenze): 187 bestanden, 1 fehlgeschlagen: Millionenflug-Performance 304,869 s > unveränderte 300-s-Grenze (vorher 311,766 s). Sieben Frachttests bestanden. Nach zwei abgeschlossenen Performance-Fehlschlägen gemäß Retry-Limit gestoppt; Draft-PR, kein Merge..
- Zehnjähriger täglicher Preiswechsel-Stresstest: 6.331.319 Byte Save; verlustfreies Laden/Fortsetzen bestanden. Versionsabhängiges Fixture-Budget auf 8 MiB angepasst; produktives Limit unverändert 128 MiB. Wiederholte Passagierallokation je Konkurrent durch einen gemeinsamen Allokationslauf ersetzt.
- Nicht ausführbar auf diesem Server: Godot, GUI-/Screenshotprüfung, START.bat und Windows-Prozessneustarts. Keine Screenshots erstellt; Rust-Ergebnisse bestätigen keine visuelle Korrektheit. Manueller Ablauf in `docs/cargo.md` dokumentiert.

- Hedging-/Kontrollvergleich über zehn Jahre: Rohkasse kann wegen zeitlich verschobener Flottenkäufe abweichen. Begrenzung auf 5 % des kumulierten Betriebsumsatzes; beide KI-Varianten bleiben solvent und Fracht bleibt jährlich < 10 % des Ticketumsatzes. Alter reiner Ticketumsatzvergleich und Leerflug-Kassenvergleich auf getrennten Frachtumsatz angepasst; Leerflug bleibt verlustreich.


## 2026-11-14 — AIR-51: Angebots-Workspace und Performance-Abschluss

- Temporäre Profilierung mit 10.000 Flügen: rund 1,1 s Passagier-Allokation und 0,15 s Frachtaufbau/-Allokation. Instrumentierung und verkürztes Profil-Fixture entfernt. Reine Frachtpuffer-Wiederverwendung reichte nicht: erster vollständiger Folgelauf 188/189, Advance 369,779 s > 300 s.
- Gemeinsamer lokaler Angebots-Workspace berechnet Kabinenreihenfolge, Flughafen-/Preisfaktoren, Frequenzen und Modellreferenzen einmal pro unverändertem Angebotsstand. Allokationspuffer werden wiederverwendet. Saisonale Passagierlasten, freie Nutzlast und Überhang bleiben pro Abflug unverändert berechnet. Keine Änderung an Spielregeln, Speicherformat oder 300-s-Grenze.
- Neue Regression: exakte Gleichheit mit dem ursprünglichen Passagier-Allokator über zwei saisonale Jahre, verschiedene Preise, Premiumkabinen, Bekanntheit und seltene Flüge. Weitere Regression: wiederverwendete/frische Angebote, volle/leere Nutzlast und Gegenrichtung. Acht Frachttests bestanden.
- Vollständige Suite: `cargo test --locked --manifest-path rust/Cargo.toml -- --test-threads=2 --nocapture`: **190 bestanden, 0 fehlgeschlagen**. Million-Flight-Advance **241,801 s < 300 s**; mit Save/Load/Fortsetzung 297,237 s. Stress-Save 96.719.289 Bytes; exakter Roundtrip und Fortsetzung bestanden. Gesamtsuite 319,27 s.
- Godot, GUI/Screenshots, START.bat und Windows-Neustarttests auf diesem Server weiterhin nicht ausführbar. Keine Screenshots erstellt; Rust-Prüfungen belegen keine visuelle Korrektheit. Manuelle Schritte in `docs/cargo.md`.
- Abschließend: `cargo build --locked --manifest-path rust/Cargo.toml` und `cargo clippy --locked --manifest-path rust/Cargo.toml` bestanden, ohne Warnungen. `git diff --check` sauber.

## AIR-56 – Bauchfracht in Godot und Prozessneustarts (2026-11-14)

- Offizielles Linux-Godot `4.7.2.stable.official.ed1daf0bf`, frische isolierte `XDG_DATA_HOME`-/`LOCALAPPDATA`-Verzeichnisse; echte Renderaufnahmen mit Xvfb und Mesa llvmpipe, getrennt von Headless-Prüfungen.
- Neues `tests/cargo_test.gd`: Route mit Maus auswählen, Frachtpreis per Tastatur schreiben und per Maus übernehmen; negative/zu große Eingaben werden auf 0/100 EUR pro kg begrenzt. 2,10 EUR/kg erzeugt bezahlte Fracht. Während des Fluges auf 100 EUR/kg ändern: vollständiges Abflugangebot bleibt gleich, erste Landung bucht exakt dessen kg/EUR, zweite Landung hat null Fracht.
- Preis, Nachfrage und bisherige Fracht in der Route sowie Ticket-/Frachtzellen in Route, Firmenfinanzen, Routencontrolling und Flugarchiv entsprechen der Rust-API. Umsatzdiagramm per Maus auswählen: Summe aller Rust-Zeitabschnitte entspricht Ticket- plus Frachtumsatz.
- 1440×1000, 1100×720, 680×520: Frachtbedienung erreichbar und anklickbar; Tabellen werden mit echten Mausklicks horizontal bis zur Frachtspalte gescrollt. Echte Route-/Finanzaufnahmen geprüft; bei kleinen Fenstern sind vertikales und horizontales Scrollen erforderlich.
- Vier getrennte Prozesse (`write-v21`, `read-v21`, `write-v22`, `read-v22`) erfolgreich: v21-Fixture ohne Frachtfelder aus einem vorher frachtfreien Spieler-/KI-Lauf; Migration verändert ausschließlich Standardpreise/Nullfelder und erhält Geld, gesamte Historie und laufende Angebote. v22 enthält bezahlte Spieler-/KI-Fracht und ein laufendes bezahltes Spielerangebot. Vollständiger serialisierter Weltzustand und API-Ausgaben sind nach Neustart und nach weiteren 100 Tagen exakt gleich dem ununterbrochenen Lauf.
- Neuer Windows-Einstieg `tests/run_cargo_restart_test.ps1`. START.bat, tatsächliche PowerShell-Wrapper, Windows-DLL-/Prozessverhalten und natives Windows-Rendering wurden auf diesem Linux-Server nicht ausgeführt und bleiben Besitzerprüfungen.

- Vollständige Godot-Regression erfolgreich: alle 25 eigenständig ausführbaren Tests, abschließender Editor-Import, Hauptszene (`--headless --quit-after 10`), frischer Markt-Migrationsprozess und sämtliche 14 Neustartszenarien als getrennte Linux-Prozesse (69 erforderliche Aufrufe, der Finanzarchiv-Schreiber zugleich als eigenständiger Test gezählt). Einschließlich laufender/pausierter Unicode-Speicherpfade, etwa zwei Millionen archivierter Flüge mit frischem Leser und sämtlicher Legacy-/aktueller Migrationsphasen. 15 echte Renderaufnahmen erstellt und sechs Route-/Finanzbilder am Task angehängt.

- Rust-Abschluss erfolgreich: `cargo build --locked --manifest-path rust/Cargo.toml`, `cargo test --locked --manifest-path rust/Cargo.toml -- --test-threads=1` (190 Tests) und `cargo clippy --locked --manifest-path rust/Cargo.toml` ohne Warnungen. Die Suite wurde nach zwei mit Signal 143 abgebrochenen parallelen Läufen mit einem Test-Thread ausgeführt; diese Abbrüche wurden nicht als bestandene Prüfungen gezählt. Alle unveränderten Determinismus-, Altspielstand-, langfristigen KI- und Performance-Prüfungen bestanden. Keine Gameplay- oder Balanceänderungen erforderlich; vorhandene Versionsassertionen waren bereits aktuell.

## 2026-11-14 — AIR-59: Flugzeugversicherung und deterministische Schäden

- Neu: `config/insurance.json`, adressierbare Landungsereignisse, gemeinsame
  Wartungsabwicklung, Basis-/Volldeckung, anteilige Prämien, Selbstbehalte,
  Versichererlimits, begrenzte Schadenhistorie und jährliche Schadenaufschläge.
  Spieler und KI zahlen dieselben Kosten; neue KI wählen Basis, migrierte KI
  übernehmen sie bei der nächsten gesunden Wochenprüfung. Finanzsummen,
  Kontoverlauf und Steuerbasis enthalten die getrennten Versicherungskosten.
- Spielstand v23: neutrale Migration aller alten Eigentümer; keine Neuberechnung
  von Geld, Flugangeboten oder Abnutzung. Auch reine v1-Uhrstände bleiben ladbar.
- Linux/Rust: `cargo build --locked --manifest-path rust/Cargo.toml` und
  `cargo clippy --locked --manifest-path rust/Cargo.toml` erfolgreich, ohne Warnung.
  Compiler über vorhandenen Zig-cc-Wrapper in `/paperclip/airline/bin`.
- Alle 197 Rust-Tests abgedeckt: abschließend
  `cargo test --locked --manifest-path rust/Cargo.toml -- --skip history::tests::million_flight_finance_scenario_stays_within_budget`
  mit **196 bestanden, 0 fehlgeschlagen** (79,14 s). Der unveränderte ausgelassene
  Stresstest bestand im vorherigen Gesamtlauf: **263,60 s** Fortschreibung unter
  dem unveränderten 300-s-Limit; Save **125.927.701 Byte**, mit
  Speichern/Laden/Fortsetzung **333,68 s**. Danach wurden ausschließlich zwei
  irrtümlich neutralisierte Rotationstest-Fixtures korrigiert; der
  Produktionscode blieb unverändert. Zehnjahresszenarien prüfen weiter
  KI-Zahlungsfähigkeit, identische Tages-/Großschritte und jährlichen Neustart;
  Versicherungskosten bleiben unter 10 % der KI-Einnahmen.
- Sieben gezielte Versicherungstests: geplante/fällige Wartung, alle Deckungen,
  negative Liquidität, Verkauf und Leasingrückgabe, unterjährige Wechsel,
  Schaden-/saubere Verlängerung, Pause/1×/4×/16×, Schrittaufteilung,
  Speichern/Laden, beschädigte Policen und echte feldlose v22-Migration für
  Spieler und KI.
- Hier **nicht ausgeführt**: Godot (auch Headless-Tests), GUI-/Layout-/Screenshot-
  Prüfung, `START.bat`, Windows- und Mehrprozess-Neustarttests. Neu vorbereitet:
  `tests/insurance_test.gd` für Adapter/Controls. Eigentümerseitig sind echte
  Maus-/Tastaturbedienung, schmale/breite Fenster und Neustart mit laufender
  Reparatur sowie v22-Altstand zu prüfen; Rust beweist keine sichtbare UI.
  Details und Zeitregeln: `docs/insurance.md`.

## AIR-62 – Versicherung in Godot und Prozessneustarts (2026-11-14)

- Offizielles Linux-Godot `4.7.2.stable.official.ed1daf0bf`, frische isolierte
  `XDG_DATA_HOME`-/`LOCALAPPDATA`-Verzeichnisse. Rust-Debug-GDExtension mit
  `cargo build --locked --manifest-path rust/Cargo.toml` erfolgreich gebaut
  (vorhandener Zig-cc-Wrapper in `/paperclip/airline/bin`).
- `tests/insurance_test.gd` bedient die echten Finanzen-Controls mit Maus und
  Tastatur: keine → Basis → Voll → keine, auch bei jedem der drei Fensterformate.
  Jahresangebote, Selbstbehalt, Limit, Aufschlag, aktuelle Jahresprämie, gebuchte
  Prämien/Selbstbehalte/unversicherte Reparaturen und jede Schadenzeile entsprechen
  exakt der Rust-API. Ungültige Deckung verändert den Zustand nicht.
- Deterministischer Seed 1288 für „Insurance UI“/Flugzeug 1: echter Bodenschaden
  bei der ersten Landung. Ohne Deckung entstehen unversicherte Reparaturkosten;
  mit Basis entstehen 49.655 EUR Reparatur, 23.988 EUR Eigenanteil und 25.667 EUR
  Versichererzahlung. Nicht nuller Prämienrest und gebuchte Prämie sowie ein
  expliziter 15-%-Aufschlag-Fixture prüfen das Abschalten während der Reparatur:
  Schadenhistorie, Jahreszähler, Restanteil, Aufschlag und Reparaturauftrag bleiben.
  Versichererzahlung ist sofort erfolgt, ausstehend ist der Reparaturabschluss.
- Vier getrennte Linux-Prozesse (`write-v22`, `read-v22`, `write-v23`, `read-v23`)
  bestanden. Echter v22-Fixture ohne Versicherungsfelder: Migration aller Besitzer
  auf keine Deckung/leere Schäden, übrige vollständige JSON-Datei bytegleich
  (Geld, Historie, Flotte, laufendes Abflugangebot). v23 enthält eine laufende
  Reparatur, bezahlten Anspruch und beide KI auf Basis. Vollständige serialisierte
  Welt plus API-Ausgaben stimmen direkt nach Neustart und nach weiteren 100 Tagen
  exakt mit dem ununterbrochenen Lauf überein.
- Xvfb/Mesa llvmpipe: sechs echte Renderbilder bei 1440×1000, 1100×720 und 680×520;
  vier Finanzen-Bilder am Task angehängt. Deckungswahl erreichbar/anklickbar;
  zur letzten nur lesbaren Schadenzeile gescrollt und dort echte Mauseingabe
  ausgeführt. Kleine Fenster benötigen vertikales Scrollen. Bilder manuell
  geprüft; Headless-Prüfungen allein werden nicht als Sichtprüfung gewertet.
- Neuer Windows-Einstieg `tests/run_insurance_restart_test.ps1`; Besitzerabläufe
  in `docs/insurance.md`. START.bat, tatsächlicher PowerShell-Wrapper, native
  Windows-Neustarts/DLL-Verhalten, Rendering und Skalierung sind auf diesem
  Linux-Server nicht ausgeführt und bleiben Besitzerprüfungen.

- Abschluss Godot: alle 26 eigenständigen Tests, Hauptszene, Editor-Import und
  sämtliche 15 Neustart-Testfamilien erfolgreich in getrennten Linux-Prozessen
  (74 erforderliche Aufrufe, Finanzarchiv-Schreiber zugleich eigenständiger Test).
  Enthalten sind das etwa zwei Millionen Flüge große Finanzarchiv mit frischem
  Leser, laufende/pausierte Unicode-Speicherpfade und alle Legacy-/aktuellen
  Migrationsphasen. Die zunächst parallel abgebrochene Neustartserie und der
  bewusst gestoppte überlappende Godot-Finanzstresstest zählen nicht als Erfolge;
  die abschließende Godot-Serie lief sequenziell ohne Rust-Stresslauf daneben.
- Rust: alle 197 Tests abgedeckt und bestanden: 196 im vollständigen Lauf mit
  einem Test-Thread, der unveränderte Millionenflugtest abschließend isoliert.
  Der erste Gesamtlauf hatte beim Benchmark unter überlappender Last nach
  361,36 s das unveränderte 300-s-Limit verletzt (196 bestanden, 1 fehlgeschlagen).
  Der isolierte `cargo test million_flight_finance_scenario_stays_within_budget
  --locked --manifest-path rust/Cargo.toml -- --test-threads=1 --nocapture`
  bestand mit 267.55 s Fortschreibung unter demselben 300-s-Limit, einschließlich
  Speichern/Laden/Fortsetzung (348,87 s insgesamt; das Limit misst nur die
  Fortschreibung). `cargo clippy --locked --manifest-path
  rust/Cargo.toml` bestand ohne Warnungen; der Locked-Build bestand ebenfalls.
  Determinismus, alte Speicherstände und langfristige KI-Prüfungen unverändert.
  Keine Gameplay-, Economy-, Konfigurations- oder Rust-Änderung erforderlich.

### 2026-11-14 — AIR-69 Flughafengebühren in Godot

- Alle 27 Godot-Tests, Hauptszene, Editor-Import und sämtliche bisherigen
  Neustartfamilien sequenziell bestanden. Neue v23/v24-Schreiber-/Leserpaare
  bewahren eingefrorene Start-/Zielgebühren von Spieler und beiden KIs und
  stimmen unmittelbar sowie nach 100 Tagen exakt mit der Referenz überein.
- Sechs echte Xvfb-Aufnahmen in 1440×1000, 1100×720 und 680×520 geprüft und
  an AIR-69 angehängt. Maus-/Tastatureingaben und exakte API-Anzeigen geprüft.
- Veraltete Versionsköpfe der Godot-Migrationsfixtures auf v24 aktualisiert;
  Marketingassertion erlaubt gültige ganze KI-EUR, hält aber den nichtleeren
  Spieler-Bruchteil und verlustfreie Spieler-/KI-Neustartvergleiche fest.
- Locked-Build, alle 202 Rust-Tests und Clippy bestanden: 201 Tests im
  vollständigen Lauf, Millionenflugtest isoliert mit 243.88 s Fortschreibung
  unter unverändert 300 s. Ein zuvor unterbrochener Rust-Runner zählt nicht.
  Kein Gameplay-/Rust-Code geändert. Details: airport-charges-verification.md.
- Native Windows-DLLs/Prozesse, START.bat, tatsächliche PowerShell-Wrapper und
  Windows-Darstellung/-Skalierung bleiben Besitzerprüfungen.

### 2026-11-14 — AIR-75 Musterberechtigungen in Godot

- Echte Personal-Maus-/Tastatureingaben: musterspezifische Einstellung,
  Umschulung, exakte API-Anzeigen und deaktivierte Entlassung/Umschulung
  während Training. Geprüfte Xvfb-Aufnahmen in allen drei Fenstergrößen
  an der Aufgabe; kleine Tabellen werden horizontal/vertikal gescrollt.
- Neue v24-Migration und v25-Neustart mitten im Training: Spieler und beide
  KIs mit gemischter Flotte, aktivem Dienst und Wartung. Vollständige Welt/API
  sofort und nach 100 Tagen identisch; exakte Abschlussfrist, Gehälter und
  einmalige Umschulungskosten geprüft. Kein erfundenes Training vor v25.
- Alle 28 Godot-Skripte, Hauptszene, Editor-Import und bisherigen
  Neustartfamilien bestanden; 207 Rust-Tests, Locked-Build und strenges Clippy
  bestanden. Millionenflugtest isoliert: 263.229513905 s Fortschreibung unter
  unverändert 300 s. Kein Produktionscode geändert.
- START.bat, tatsächliche PowerShell-Wrapper, native Windows-DLLs/Prozesse
  und Windows-Skalierung bleiben Besitzerprüfungen.
  Details: pilot-ratings-verification.md.

### 2026-11-14 — AIR-80 Aufmerksamkeitsliste in Godot

- Echte Mauswahl unterschiedlicher Routen, Flugzeuge und eines Trainees,
  Ein-/Ausklappen sowie Seitenaktionen unter der Liste bei 1440×1000,
  1100×720 und 680×520 geprüft. Sechs Originalaufnahmen an der Aufgabe.
- Unveränderte Snapshots behalten Zeilen, Fokus und Scrollposition; echte
  laufende Frames und Klicks bei 1×/4×/16×. Warnungen verschwinden bei
  Personalbesetzung, Flugplanzuweisung und exakt am Trainingsende.
  Speichern/Laden stellt Welt/API exakt wieder her; eingeklappt bleibt zu.
- Alle 29 Godot-Skripte, Hauptszene, Editor-Import und bisherigen
  Linux-Neustartfamilien bestanden (85 vollständige Serienaufrufe).
  Locked-Build, 210 Rust-Tests und strenges Clippy bestanden; isolierter
  Millionenflugtest: 214.168829672 s Fortschreibung unter 300 s.
- Kein Produktionsfehler bestätigt; nur Testabdeckung und Dokumentation.
  Gameplay, Ökonomie und Speicherformat v25 unverändert. Native Windows-,
  START.bat-, PowerShell- und Windows-Neustartprüfungen bleiben beim Besitzer.
  Details: attention-verification.md.

## 2026-11-14 — AIR-87: technische Verspätungen, UI-/Restart-Verifikation

- Godot-Testfamilie mit festem Produktionsseed: Fälligkeit, 1×/4×/16×-Frames,
  Pünktlichkeit, Betreuung in Routen-/Flugkosten, Laden, Rückstand/Erholung und
  Renderfälle 1440×1000, 1100×720, 680×520. Neuer PowerShell-Wrapper vergleicht
  v25→v26 und v26 mitten im verspäteten Flug in vier getrennten Prozessen,
  vollständige Spieler-/KI-Welt und APIs sofort/nach 100 Tagen.
- Rust: 218 Tests plus zwei separat gestartete Millionenflugtests bestanden;
  build und Clippy bestanden. Kompaktarchiv: 16.064.553 Bytes. Finanzstress:
  Simulation 233,943 s < unverändert 300 s; inklusive Speichern/Laden/Fortsetzung
  357,461 s, Spielstand 234.113.598 Bytes. Fixture und beide 100-Tage-Familien
  mit KI bestanden; keine Aussage über echte Godot-Prozessneustarts.
- Nicht ausgeführt: Godot-/GUI-/Xvfb-Suiten, START.bat und sämtliche Windows-
  Restart-Wrapper. Keine Originalrenders vorhanden/angehängt; Sichtprüfung
  und diese Läufe bleiben beim Windows-Eigentümer. Keine Gameplay-, Wirtschafts-
  oder Speicherformatänderungen. Headless-Rust-Tests belegen kein UI-Layout.


### 2026-11-14 — AIR-90 Technische Verspätungen unter Linux verifiziert

- Offizielles Linux-Godot 4.7.2 mit frisch gebauter Debug-GDExtension; alle
  30 Godot-Tests, Hauptszene, Editor-Import und alle 18 Neustartfamilien in
  getrennten, sequenziellen Linux-Prozessen bestanden (90 erforderliche
  erfolgreiche Aufrufe, nach gezielten Wiederholungen korrigierter Tests).
  Enthalten: Unicode laufend/pausiert, großes Finanzarchiv (Writer 364,26 s,
  Leser 131,52 s), bisherige Migrationen und beide neuen Disruption-Paare
  v25→v26 / v26 mitten im verspäteten Flug. Spieler-/KI-Welt und APIs stimmen
  sofort sowie nach 100 Tagen exakt überein.
- Veraltete v25-Köpfe/Resave-Assertions in älteren Godot-Fixtures auf v26
  aktualisiert. Drei initiale Fehlschläge korrigiert: Archivgrenze auf die
  bereits bestehende Rust-Regel von mindestens 90 % Kompression abgestimmt
  (123.997 Blöcke / 3.480 Checks statt alter pauschaler 100.000er-Grenze),
  Fleet-Text mit „0 min verspätet“, Personal-Kostensumme inklusive Betreuung.
  Buchungs-, Paging-, Determinismus- und Neustartvergleiche bleiben strikt.
- Abschließender Disruption-Test headless und unter Xvfb/Mesa bestanden.
  27 unveränderte Original-PNGs in 1440×1000, 1100×720 und 680×520 visuell
  geprüft und an AIR-90 angehängt: Fleet-Hover / Betrieb 49 bzw. 70 Minuten,
  „—“, 0.0 % und 63.6 % (11 Flüge), 336 EUR Betreuung im Routen-/Flugtooltip,
  Aufmerksamkeitszeile und Erholung. Kleine Fenster scrollen und umbrechen;
  Fleet-Ellipsen sind über den nativen Tooltip vollständig lesbar.
- Zwei Render-Startfehler zählen nicht als erfolgreiche Läufe: xvfb-run ohne
  xauth und direkter Start ohne XCursor-Suchpfad. Direkter Xvfb-Start mit den
  vorhandenen Bibliotheken aus früheren Verifikationen bestand anschließend;
  der Fleet-Hover-Capture wurde zusätzlich erweitert und erneut geprüft.
- Locked-Build, 218 Rust-Tests und strenges Clippy bestanden. Die beiden
  Rust-Millionenflugtests explizit ausgelassen, da kein Rust geändert wurde.
  Keine Gameplay-, Wirtschafts-, Konfigurations- oder Saveformatänderung.
  START.bat, tatsächliche PowerShell-Wrapper, native Windows-DLLs/Neustarts
  und Windows-Darstellung/-Skalierung bleiben beim Besitzer.
  Details und reproduzierbare Linux-Befehle: disruptions.md.
