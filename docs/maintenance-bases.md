# Wartungsbasen

Jede Airline (Spieler und KI) hat am Heimatflughafen automatisch eine kostenlose,
nicht schließbare Wartungsbasis. Weitere Basen verwaltet der Spieler im bestehenden
Flotten-/Wartungsbereich. Die Flughafenauswahl kennzeichnet Heimatbasis und weitere
Basen. Rust liefert Preise, Aktionssperren und Wartungsvorschauen; GDScript stellt
sie dar und leitet Eingaben weiter.

## Kosten und Zeit

`config/maintenance.json` enthält:

- `base_setup_cost_eur`: einmalig 250.000 EUR beim Eröffnen.
- `base_recurring_fee_eur`: 10.000 EUR alle 30 Spieltage ab Eröffnung.
- `outstation_cost_percent`: 110 % der normalen Checkkosten außerhalb einer Basis.
- `outstation_duration_percent`: 110 % der normalen Checkdauer außerhalb einer Basis.

Die Außenstationsprozente werden nach den bestehenden Zuschlägen für ungeplante
Checks angewendet, jeweils auf volle EUR bzw. Sekunden aufgerundet. An einer Basis
bleiben die bisherigen Modellwerte unverändert. Alle Airlines nutzen denselben
Wartungspfad. Die KI eröffnet vorerst keine zusätzlichen Basen; ihre bisherigen
Planungsschätzungen bleiben Schätzungen für einen regulären Check an einer Basis.

Die Vorschau nennt den tatsächlichen nächsten Landeort, wenn das Flugzeug fliegt.
Ein Auftrag wird bei der Landung anhand der dann vorhandenen Basis neu bewertet.
Kosten und Dauer werden beim Beginn eingefroren, einmalig gebucht und danach nicht
mehr durch Basisänderungen verändert. Fehlende Techniker verzögern wie bisher die
Ausführung; die eingefrorene Arbeitsdauer bleibt erhalten.

Schließen erstattet nichts und beendet künftige Gebühren. Während einer laufenden
Wartung (auch beim Warten auf Techniker) kann deren Basis nicht geschlossen werden.
Ein noch fliegendes Flugzeug sperrt das Schließen nicht. Eröffnen benötigt genügend
Bargeld und ein handlungsfähiges, nicht notleidendes Unternehmen. Wiederkehrende
Gebühren werden auch bei fehlendem Guthaben fällig und wirken wie andere Kosten auf
die bestehende Insolvenzlogik.

Versicherungsreparaturen behalten ihre separat gebuchten Kosten und ihre eigene
Ausfallzeit. Treffen Reparatur und Wartungsauftrag zusammen, bleibt die
Reparaturzeit eine Mindestdauer des Aufenthalts.

## Buchung, Spielstand und Determinismus

Einrichtung und Gebühren verwenden die vorhandene Wartungskostenhistorie und sind
in deren Finanzsummen enthalten. Die Heimatbasis wird aus `home_airport_icao`
abgeleitet. Nur zusätzliche Basen mit ICAO und nächster Fälligkeit werden gespeichert.
Gleichzeitige Gebühren werden nach ICAO gebucht. Fälligkeiten sind Ereignisse der
gemeinsamen Rust-Simulation, ohne zusätzliche Uhr in der Oberfläche.

Save v29 migriert ältere Spielstände für alle Eigentümer auf ausschließlich ihre
Heimatbasis. Bestehende Checks, Kosten, Endzeiten und Historien werden nicht neu
berechnet. Rust-Tests prüfen u.a. 1×/4×/16×, Pause, große Schritte, Save/Load sowie
einen v29-Schreiber/Leser in getrennten Linux-Testprozessen. Das ersetzt weder
Godot-/START.bat-Tests noch PowerShell-Wrapper, native Windows-DLL-/Prozessneustarts
oder Sichtprüfung von Rendering und Skalierung durch den Owner.
