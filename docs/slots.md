# Flughafen-Slots

`daily_departure_slots` im Flughafenkatalog begrenzt FRA, IST, JFK und LAX
zunächst auf jeweils 120 tägliche Abflüge. Ohne dieses optionale Feld gibt es
keine Grenze. `config/slots.json` setzt die Warnschwelle auf 90 Prozent.

Ein Flugplan pendelt zwischen beiden Endpunkten. Pro Flugzeug werden an jedem
Endpunkt `ceil(86400 / (2 * interval_seconds))` Slots reserviert: die maximale
Abflugzahl pro rollendem Tag. Beispiel: 120 Minuten Abstand = sechs Slots je
Endpunkt. Die Reservierung hängt nicht von Uhrzeit, Verspätungen oder Wartung
ab. Es gibt keine Schonfrist. Beendete Flugpläne, entfernte Frequenzen und
Verkauf/Leasingrückgabe geben die Reservierung frei. Bestehende Wartungsresumes
halten sie weiter. Ungeplante Routen reservieren keine Slots.

Spieler und KI verwenden dieselbe Zulassungsprüfung. Vor jeder KI-Prüfung wird
der Verbrauch der anderen Betreiber neu abgeleitet; Entscheidungen laufen in
stabiler Betreiberreihenfolge. Die KI überspringt volle Märkte vor Kauf oder
Personalaufbau. Bei vollem Heimatflughafen wartet die Expansion; es gibt keine
kostenlose Überführung zu einem anderen Ausgangspunkt. Bei vollem Ziel sucht
sie andere Märkte. Gestoppte, blockierte Routen werden nach dem bestehenden
Wiederholungsintervall freigegeben, damit die Maschine neu eingesetzt wird.

Speicherformat 27 ist eine Identitätsmigration: alte Frequenzen und alle
Finanzdaten bleiben unverändert, auch bei Überbelegung. Neue Zulassungen bleiben
bis zur Freigabe ausreichender Slots gesperrt. Reservierungen werden aus den
bestehenden Flugplänen und Wartungsresumes abgeleitet, nicht separat gespeichert.

Die Routenplanung zeigt belegte/freie Slots an beiden Endpunkten. Flughäfen sind
auf der Karte anklickbar; das vorhandene Detailpanel zeigt ihre Slots. Eine
Aufmerksamkeitszeile fasst fast volle Spielerflughäfen zusammen.

Prüfung auf einem Godot-Rechner:
`Godot --headless --path . --script res://tests/slots_test.gd`.
Zusätzlich die bestehenden Karten-, Flughafen- und Aufmerksamkeits-Tests sowie
native Darstellung/Skalierung prüfen. Rust-Tests ersetzen diese Sichtprüfung nicht.
