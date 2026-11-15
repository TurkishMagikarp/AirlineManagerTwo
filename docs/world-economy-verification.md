# Weltwirtschaft: Prüfung und Balancing

Basis: `origin/main` bei `4ea6a12` (nach AIR-214), separater Baseline-Worktree mit unveränderten Spielregeln; dieselben Szenarien mit dem vollständigen Flughafenkatalog.
Beträge sind EUR. Bestehende KI-Regeln bleiben erhalten; nur ihr gemeinsamer
Nachfragemarkt und die Eingaben für die vorhandene Planung ändern sich.

## Zwei Jahre, drei Spielerregionen

`quality::two_year_catalogue_ai_audit` prüft Pause, 1×/4×/16×, Tages- gegen
Wochenschritte und Jahres-Restart. Alle sechs Rivalenläufe bleiben solvent,
ohne Zahlungsnot oder Restrukturierungsfrist.

| Spielerbasis | Rivale | Cash main → Zyklus | Flotte main → Zyklus | O&D-Routen main → Zyklus | Dienste main → Zyklus |
| --- | --- | ---: | ---: | ---: | ---: |
| EDDF | Regional Wings | 107.809.067 → 116.187.129 | 6 → 6 | 6 → 6 | 6 → 6 |
| EDDF | Continental Sky | 34.207.235 → 42.705.584 | 5 → 5 | 5 → 5 | 5 → 5 |
| SBGR | Regional Wings | 34.317.471 → 46.300.566 | 5 → 5 | 5 → 5 | 5 → 5 |
| SBGR | Continental Sky | 107.310.890 → 138.958.998 | 6 → 6 | 6 → 6 | 6 → 7 |
| RJTT | Regional Wings | 106.286.287 → 138.396.145 | 6 → 6 | 6 → 6 | 6 → 7 |
| RJTT | Continental Sky | 34.406.772 → 42.705.584 | 5 → 5 | 5 → 5 | 5 → 5 |

Die frühe Konjunktur ist im Standard-Seed überwiegend gut; höhere Reserven und
andere Expansionsentscheidungen sind beabsichtigt. Ein zusätzlicher Dienst auf
RJTT–ZBAA entsteht mit den vorhandenen Rotationsregeln.

## Zehn Jahre einschließlich Abschwung

`quality::ten_year_scenarios_reconcile_and_survive_annual_restart` prüft jeden
Tag beide Rivalen auf Zahlungsnot und Restrukturierungsfrist und verlangt nach
jedem Jahr beide solventen Rivalen, identische Zustände bei Jahres-/Tagesschritten
und verlustfreie Save/Reload-Fortsetzung. Der Standardindex erreicht 87,94 % am
Tag 2513 und bleibt über den zehnjährigen Lauf zwischen 87,94 % und 110,11 %.
Kein Rivale tritt in eine Restrukturierung ein oder wird insolvent. Die zwei
bewusst untragbaren Spielerpreis-Szenarien (0 und 100.000 EUR) enden dagegen wie
auf main früh in der Spielerinsolvenz; ihre eingefrorenen Endstände sind kein
zehnjähriger KI-Balancinglauf.

| Spielerszenario | Rivale | Cash main → Zyklus | Flotte main → Zyklus | Dienste main → Zyklus |
| --- | --- | ---: | ---: | ---: |
| Ohne Spielerflüge | Regional Wings | 586.486.842 → 599.495.783 | 6 → 6 | 6 → 6 |
| Ohne Spielerflüge | Continental Sky | 414.774.426 → 403.738.665 | 6 → 6 | 6 → 6 |
| ATR, 90 EUR | Regional Wings | 532.587.006 → 531.669.308 | 6 → 6 | 6 → 6 |
| ATR, 90 EUR | Continental Sky | 414.774.426 → 403.738.665 | 6 → 6 | 6 → 6 |

## Schwacher Spielerstart

Zusätzlicher Seed 349: Zwei Jahre mit mittlerem Tagesindex 89,608 % und einem
Tief von 85 %. Derselbe typische ATR-Start EDDF–EDDM zu 90 EUR bleibt ohne
Zahlungsnot, hat aber nur 14.580.489 EUR Cash statt 17.860.191 EUR auf main
(3.279.702 EUR weniger). Fluggewinn: 13.227.590 statt 17.738.335 EUR.
Der finanzielle Puffer wächst langsamer; der Start bleibt in diesem starken
Abschwung tragfähig. Die Rivalen enden bei 85.041.150 EUR / sechs Flugzeugen /
sechs Diensten beziehungsweise 25.345.597 EUR / vier Flugzeugen / vier Diensten;
main: 103.817.996 / sechs / sechs und 34.207.235 / fünf / fünf. Keine Zahlungsnot,
Restrukturierung oder Insolvenz in den täglichen Prüfungen. Band oder KI mussten
nicht mit Rettungs-Sonderregeln angepasst werden.

## Laufzeit und Archiv

`AIRLINE_ENFORCE_PERF_BUDGET=1 cargo test --locked --manifest-path rust/Cargo.toml million_flight -- --nocapture --test-threads=1`
besteht: **235,45 s Advance** gegenüber **194,57 s** aus AIR-214, also +40,88 s
(+21,0 %), unter dem unveränderten 300-s-Budget. 5.779.250 abgerechnete Flüge
inklusive Rivalen; 250.041.427 Byte Save, exakter Reload und weitere Fortsetzung
bestehen. Der Index wird einmal je Periode abgeleitet und aus einem abgeleiteten
Timeline-Cache gelesen, ohne Zufallsberechnung pro Flug oder Passagier.

Der Stresstest entdeckte eine untertägige Überbuchung durch Tages-Umsteigequoten.
Eine kapazitätsverletzende Cachequote wird jetzt für den gesamten Markt exakt
zum Abflugzeitpunkt neu berechnet. Der fokussierte siebenjährige Test und der
Millionenflug-Archivtest belegen Sitzgrenzen und savebare Fortsetzung.

## Weitere Prüfungen und Ansichten

- Alle Rust-Tests: 308 bestanden (vier opt-in ignoriert), dazu zwei separat bestandene `million_flight`-Fälle; außerdem Build und Clippy, jeweils `--locked`.
- Indexband, Wochenkadenz, unterschiedliche Seeds/Abfragefolgen, Geschwindigkeiten, Pause, Schrittgrößen, Zeitsprünge, Restart und v33-Migration mit neutralem ersten Tag.
- Geschäfts-/Urlaubs-/Frachtreaktion und exakte neutrale Quotes mit Reaktion 0 einschließlich Umsteigern; ungültige Zeitursprünge, Zustellpositionen und abweichende Rivalenmärkte werden abgelehnt.
- `tools/godot_check.sh tests`: alle ausführbaren Tests; der aktuelle Save ist v34, frühere Migrationsfixtures berücksichtigen den neuen neutralen Markt.
- `tools/godot_check.sh shots /paperclip/shots/AIR-218`: Overview und Routes jeweils bei 1440×1000 und 680×520 geöffnet, außerdem normale/deutsche und Abschwung/deutsche sowie Boom/englische Economy-Captures in beiden Größen. Werte und Chips passen; die kleinen Routenfaktoren sind vollständig lesbar und umbrechen ohne Überlappung.
- Nicht geprüft: START.bat, PowerShell-Restarttreiber und echte GPU-/Windows-Darstellung. Die Screenshots stammen aus Software-OpenGL.
