# Flugzeugversicherung und Betriebsschäden

Alle Regeln liegen in `rust/src/insurance.rs`, alle Stellwerte in
`config/insurance.json`. Ein unabhängiger adressierbarer Hash entscheidet je
Landung (Standard: 200 pro Million, also 0,02 %). Schlüssel sind Weltseed,
Airline-Name, Flugzeugkennung und lebenslange Landungsnummer. Ein abgeleiteter,
nicht gespeicherter Cache sucht den nächsten Treffer; Frames und UI verwenden
keinen Zufall. Typ und Reparaturhöhe verwenden einen getrennten Hash.

Vogelschlag, Bodenschaden und harte Landung kosten standardmäßig 1–5 % des
aktuellen Zeitwerts und mindestens 12 Stunden Standzeit. Zeitwert verwendet
dieselbe Alters-/Zustandsformel wie der Verkauf; bei betriebenen Leasingmaschinen
zählt der Rumpfwert, nicht die rückzahlbare Kaution. Verkauf/Rückgabe entfernt
das Flugzeug aus der Risikomenge. Das Spiel hat keinen Verleih eigener Maschinen
an andere Airlines.

Reparaturen nutzen den bestehenden Wartungshangar einschließlich Technikerbedarf,
Abflugsperre und Wiederaufnahme am tatsächlichen Standort. Bereits beauftragte
oder gleichzeitig fällige Checks bleiben bezahlt; die längere der beiden
Standzeiten gilt. Schäden werden genau einmal abgerechnet, auch ohne ausreichend
Bargeld. Der normale Notlagen-/Restrukturierungsablauf verarbeitet negative
Kontostände. Eine Reparatur schließt wie andere Hangarbesuche den Check ab.

Deckung je Airline: keine, Basis oder Voll. Standard-Jahresprämien sind 0,12 %
oder 0,30 % des Flottenzeitwerts, Selbstbehalte 1 % oder 0,1 % je Schaden;
Versichererlimit jeweils 100 % des Zeitwerts. Der Versicherer zahlt den Schaden
abzüglich Selbstbehalt bis zum Limit; ein darüber hinausgehender Rest bleibt
unversichert. Auch betriebene Leasingmaschinen sind enthalten. Neue KI-Airlines
wählen Basis; migrierte Airlines behalten zunächst keine Deckung.

Die Jahresprämie wird täglich aus dem Flottenwert neu bestimmt und sekundengenau
mit ganzzahligem Restanteil täglich sowie beim Deckungswechsel abgerechnet.
Käufe, Verkäufe und Zustandsänderungen wirken beim nächsten Tageswechsel auf die
Prämie. Deckungswechsel wirken sofort auf künftige Landungen; bereits entstandene
Schäden bleiben unverändert. Abgeschaltete Deckung löscht weder Ansprüche noch
Prämienreste oder Schadenaufschläge.

Erneuerung nach jeweils 365 Tagen ab Gründung bzw. Migration: pro bezahlt gedecktem
Schaden +15 Prozentpunkte Prämienaufschlag (maximal 100 %), pro sauberem Jahr
−5 Prozentpunkte bis null. Der Jahreszähler ist unabhängig von der auf 20 Einträge
begrenzten Anzeigehistorie. Prämien, Selbstbehalte und ungedeckte Reparaturen
stehen getrennt im Finanzjournal und in der Finanzen-Ansicht; sie zählen zu
Gesamtkosten, Ergebnis, Kontoverlauf und steuerlich abziehbaren Aufwendungen.

Spielstand v23 migriert ältere Dateien für Spieler und KI auf keine Deckung,
leere Schadenhistorie und einen Beginn zum Ladezeitpunkt. Geld, Flugpläne,
Abflugangebote, Abnutzung und bestehende Wirtschaftshistorie werden nicht neu
berechnet.

## Prüfung beim Eigentümer

Dieser Server hat keine Godot-Ausführung. Unter Linux mit Godot:
`Godot --headless --path . --script res://tests/insurance_test.gd`.
Zusätzlich unter Windows: `START.bat`, echte Maus-/Tastaturbedienung der
Versicherungswahl, schmale/breite Fenster, sichtbare Schadenliste sowie
Speichern/Beenden/Neustart mit laufender Reparatur und mit alter v22-Datei.
Rust-Tests ersetzen keine visuelle UI-Prüfung oder Mehrprozessprüfung von Godot.
