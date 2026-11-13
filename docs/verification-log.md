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
