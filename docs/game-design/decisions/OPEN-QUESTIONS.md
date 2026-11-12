# Offene Entscheidungen — priorisierte Bearbeitung

**Regel:** Die folgenden Punkte sind unbeschlossen. Agenten dürfen sie nicht als bereits genehmigte Spielregeln behandeln. Priorisierung und Empfehlungen sind keine Produktbeschlüsse.

## P0 — Vor dem ersten Vertical Slice klären

| ID | Offener Punkt | Betroffene GDDs | Benötigtes Ergebnis |
|---|---|---|---|
| OQ-001 | Exakter Umfang des ersten spielbaren Kerns | 035, 064 | Eindeutige In-/Out-of-Scope-Liste und Abnahmekriterien |
| OQ-002 | Rust–Godot-Schnittstelle und Versionen | 036, 037, 067 | Kompatibilitätstest, Commands/Events/Snapshots, Fehlerbehandlung |
| OQ-003 | Zeitmodell und Reproduzierbarkeit | 003, 023, 025 | Ereignisreihenfolge, Seed, Zeitsprünge, Determinismus |
| OQ-004 | Finanzbuchungen und Kostenmodell des Prototyps | 011, 033, 056, 057 | Konsistente Geldbewegungen und Definitionen von Ertrag/Kosten |
| OQ-005 | Kleinster erlaubter Passagier- und Nachfrageprototyp | 007, 033, 035 | Klare Abgrenzung zwischen vereinfachtem Slice und langfristiger individueller Simulation |
| OQ-006 | Initiale Flugzeug-/Flughafendaten und Rechte | 002, 040, 044 | Rechtssicherer Minimaldatensatz und dokumentierte Herkunft |
| OQ-007 | Persistenz und Wiederherstellung im Slice | 026, 035 | Dateiformat, deterministischer Lade-/Speichertest |
| OQ-008 | Godot-UI-Stilgrundlage und Qualitätstest | 020, 065, 066 | Wiederverwendbare Controls, Tokens, Screenshots und Lesegrößen |

## P1 — Vor Ausbau der Wirtschaftssimulation klären

| ID | Thema | Betroffene GDDs |
|---|---|---|
| OQ-009 | Individuelle Passagiere versus adaptive Aggregation und globale Wechsel der Detailstufe | 007, 023, 044 |
| OQ-010 | KI-Airline-Budgets, Verhalten und strategischer Planungstakt | 008, 021, 050 |
| OQ-011 | Arbeitsmarkt, Qualifikationen, Rostering und Personalentwicklung | 012, 051, 052, 053 |
| OQ-012 | Flugzeugkomponenten, Sicherheit, Kapazität, Cargo und Wartungsmodelle | 009, 010, 018, 032 |
| OQ-013 | Gemeinsame Verträge, Gesellschaften, Eigentumsrechte und Konsolidierung | 011, 046, 058, 059 |
| OQ-014 | Nachfrageschätzung, Analystenfähigkeiten und Informationszugriff | 047, 048, 049 |
| OQ-015 | Dynamische Treibstoffabsicherung versus bewusst ausgeschlossenes Währungshedging | 055, 057 |

## P2 — Vor Veröffentlichung bzw. langfristigen Systemen klären

| ID | Thema | Betroffene GDDs |
|---|---|---|
| OQ-016 | Mod-Sicherheitsmodell und Speicherslot-Kompatibilität | 026, 027 |
| OQ-017 | Kommerzielle Rechte an Airline-Namen, Logos, Liveries, Flotten- und Kartendaten | 031, 040 |
| OQ-018 | macOS-Signierung/Notarisierung, Update-Installation und Rollback | 024, 039 |
| OQ-019 | Historienkompression, Snapshot-Verzweigung und Speicherplatzgrenzen | 026 |
| OQ-020 | Langzeit-Performance, Validierungs-Benchmarks, Testhardware | 023, 038, 044 |
| OQ-021 | Offline-generierte Medieninhalte, Templates und Wahrheitsbedingungen | 060 |
| OQ-022 | Einheitliches UI-Kit, Icons, Asset-Metadaten, 3D-Modell-/Texturrechte | 020, 027, 030, 031, 066 |

## Bereits geklärte Änderungen (keine offenen Fragen)

- **GDD-024:** Windows **und** macOS sind Zielplattformen; nicht nur Windows.
- **GDD-037/067:** Godot 4 + Rust ersetzt Unity.
- **GDD-042:** Detailliertes Kabinen- und Servicemanagement wurde gewählt, nicht die vollständige Catering-Logistik.
- **GDD-043:** Restrukturierung vor möglichem endgültigem Insolvenzende; kein beschlossener Wechsel zu einer anderen Airline nach Scheitern.
- **GDD-056:** Vereinfachte länderabhängige Körperschafts-/Unternehmenssteuersätze, kein internationales Vollsteuersystem.
- **GDD-057:** Eine gemeinsame Spielwährung, keine Währungsrisiken.

## Dokumentationsqualität

Das Register ist eine konsolidierte Paraphrase der bisherigen Gespräche, kein vollständiges Transkript. Vor einer feingranularen Implementierung ist die entsprechende Entscheidung im Fachkapitel zu präzisieren. UI-Demonstrationswerte sind **keine** verbindlichen Balancingwerte.
