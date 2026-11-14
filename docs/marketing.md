# Marketing (v20)

Ein unternehmensweites Monatsbudget in Finanzen: 0 bis zum konfigurierten Maximum,
sofort änderbar. Rust besitzt Budget, Bekanntheit, Trend und Buchungen;
Godot zeigt Werte und leitet Eingaben weiter.

`config/marketing.json` enthält alle wirtschaftlichen Stellschrauben. Ein Monat
sind zunächst 30 Tage. Kosten fallen zeitanteilig an und werden täglich sowie
bei Budgetwechsel gebucht. EUR-Bruchteile bleiben über Wechsel und Neustarts
erhalten. Das eigene Marketingarchiv zeigt die letzten 100 Buchungen;
Unternehmenssummen, Kontoverlauf und Betriebskosten enthalten alle Buchungen.
Marketing mindert das Vorsteuerergebnis; Routenbeiträge bleiben ohne zentrale Kosten.

Bekanntheit liegt zwischen 0 und 100 (intern ganzzahlige Millionstel).
An absoluten Tagesgrenzen wird das zeitgewichtete Tagesbudget B ausgewertet:
Ziel = 100 × B / (B + 50.000 EUR). Pro Tag nähert sich Bekanntheit um 1/90
der Differenz an das Ziel an; der letzte Rest sinkt um ein Millionstel.
Ohne Budget ist das Ziel null. Budgetwechsel bewirken keinen sofortigen Sprung.
Der angezeigte Trend ist die Veränderung am letzten Tagesabschluss.

Bekanntheit verstärkt das Angebotsgewicht höchstens um 8 %. Preisbereitschaft,
Außenoption, Kabinen und Kapazitätsgrenzen gelten weiterhin. Bereits bei Abflug
fixierte Ergebnisse ändern sich nicht. Alle Unternehmen nutzen denselben
Angebotsschnappschuss vor dem Tagesabschluss; neue Bekanntheit gilt danach.

KI überprüft wöchentlich: Budget = min(20.000 EUR, max(0, Guthaben − 2 Mio.) / 200).
In Finanznot oder Restrukturierung ist es null; bei der täglichen Buchung stoppt
Marketing bei Finanznot auch zwischen KI-Prüfungen. Dieselben Kosten- und
Bekanntheitsregeln gelten für alle Airlines.

v19 und ältere Spielstände starten beim Ladezeitpunkt für alle Unternehmen mit
Budget/Bekanntheit null. Keine vergangenen Ausgaben werden nachberechnet.
`marketing_tests` prüft Regeln, Marktgrenzen, Migration, Zeitaufteilung und Neustart.
`tests/marketing_test.gd` nutzt echte Tastatur-/Mauseingaben und prüft drei
Fenstergrößen. `tests/run_marketing_restart_test.ps1` führt v19-Migration und
v20-Neustart in getrennten Prozessen aus. Godot-/Windows-Prüfungen müssen auf
einem entsprechenden Rechner laufen; Rust-Tests ersetzen keine Sichtprüfung.
