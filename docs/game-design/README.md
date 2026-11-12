# Airline Manager — Game Design Document

**Stand:** konsolidierte Produktvision v0.2 · **67 beschlossene Entscheidungen:** GDD-001 bis GDD-067 · **Arbeitssprache der Dokumentation:** Deutsch (bestehende Fachkapitel liegen teilweise noch auf Englisch vor; deren Übersetzung ist redaktionelle Folgearbeit).

## Verbindlichkeit und Abgrenzung

Die Entscheidungen im [Entscheidungsregister](decisions/DECISION-REGISTER.md) sind verbindlich, soweit sie nicht ausdrücklich als überholt gekennzeichnet sind. Spätere explizit bestätigte Entscheidungen ersetzen widersprechende ältere Formulierungen. Die verlinkten Fachkapitel sind **thematische Zusammenfassungen**, noch keine bis ins Detail ausführbaren technischen Spezifikationen.

**Nicht als beschlossen behandeln:** Beispielwerte in UI-Konzepten, frühere Empfehlungen, vorgeschlagene Algorithmen, technische Varianten und die vorläufige Vertical-Slice-Scope. Offen gebliebene Details stehen im [Fragenregister](decisions/OPEN-QUESTIONS.md).

## Lesepfade

- **Produkt und UX:** [Vision](00-vision.md) → [Gameplay](01-gameplay.md) → [UI](06-ui-automation.md)
- **Simulationslogik:** [Welt](02-world.md) → [Betrieb](03-operations.md) → [Wirtschaft](04-economy.md) → [Personal](05-people.md)
- **Technik:** [Technik](07-technical.md) → [Entwicklung](08-delivery.md) → [offene Punkte](decisions/OPEN-QUESTIONS.md)
- **Rückverfolgbarkeit:** [GDD-001–067](decisions/DECISION-REGISTER.md)

## Änderungsregeln

1. Entscheidung nur durch ausdrücklichen neuen Produktbeschluss ersetzen.
2. Beschlüsse und noch offene Details klar getrennt dokumentieren.
3. Bei Änderung das Register sowie alle betroffenen Kapitel aktualisieren.
4. Bei Unklarheit keine stillschweigende Spielregel erfinden; offenen Punkt erfassen.

## Technischer Zielrahmen

**Godot 4 + Rust**, native **Windows- und macOS-Versionen**, kein Browser, vollständiger Offline-Einzelspieler, echte geografische Grundlage mit spielerischen Freiheiten, globale adaptive Simulationswelt, tiefes Management mit vollständiger optionaler Automatisierung, hochwertige erklärbare UI. Entwicklung: zuerst ein spielbarer Kern, anschließend schrittweise Erweiterung.

## Redaktionelle Übernahme für den Neustart

Kapitel 00–07, Entscheidungsregister und offene Fragen sind inhaltlich unverändert
aus dem bisherigen GDD übernommen. In Kapitel 08 wurde ausschließlich der Absatz
zum alten Repository-/Agentenworkflow entfernt. Alte Agenten-Leseanweisungen und
Links auf nicht mitgenommene Audit- und Vertical-Slice-Unterlagen wurden aus diesem
Index entfernt. Es werden keine alten Workflow-Skripte benötigt. Der Vertical Slice
bleibt ein **nicht freigegebener Vorschlag**, kein Implementierungsauftrag.
