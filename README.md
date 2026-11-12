# Airline Manager — Neustart

`project.godot` im Godot-4-Editor öffnen, **F5** starten. Die Hauptszene
`scenes/main.tscn` startet auch mit **F6**. Geprüfte Engine: Godot 4.7.2 unter
Windows; macOS ist Zielplattform, noch nicht getestet.

Offline UI-Basis mit Übersicht, Flotte und Routen. Navigation per Maus oder
Tab / Umschalt+Tab und Enter / Leertaste. Keine Spielstände, Wirtschaftswerte
oder Simulation. Rust bleibt die spätere maßgebliche Simulations-Engine;
eine Schnittstelle ist hier noch nicht implementiert. Keine Plugins nötig.

Produktregeln und offene Fragen: [Game Design](docs/game-design/README.md).

Lokaler Navigationstest: `Godot --headless --path . --script res://tests/navigation_test.gd`
(mit dem eigenen Godot-Executable aus diesem Projektordner).
