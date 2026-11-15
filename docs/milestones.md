# Optionale Meilensteine

Die Spieler-Airline erhält rein informative Ziele aus `config/milestones.json`.
Die Datei wird beim ersten Zugriff eingelesen und validiert; Änderungen gelten
nach einem Neustart. Fehlende Dateien, unbekannte Arten, doppelte Kennungen und
nichtpositive Schwellen liefern einen Fehler beim Gründen oder Laden.
Kennungen bleiben bei der Abstimmung stabil; neue Kennungen benötigen DE/EN-Texte
in `resources/localization/milestones.csv`. Mengenangaben in den übersetzten Titeln verwenden die konfigurierte Schwelle,
nicht die Zahl in der stabilen Kennung. Die Reihenfolge bestimmt die nächsten
offenen Ziele in der Übersicht.

Rust prüft nach der vollständigen Tagesabrechnung um Mitternacht. Der gespeicherte
Zeitpunkt ist der Prüfzeitpunkt, nicht der möglicherweise frühere Flugzeitpunkt.
Erreichte Ziele bleiben erhalten. Es gibt keine Belohnungen, Freischaltungen,
Nachfrageeffekte, Aufmerksamkeitsmeldungen oder Sprungunterbrechungen. KI-Airlines
haben keinen Meilensteinzustand. Kein Meilenstein-Code läuft pro Flug.

| Art | Schwellen | Bedeutung |
| --- | --- | --- |
| Flotte | 1 / 5 / 15 | Flugzeuge einschließlich Leasing |
| Aktive Routen | 1 / 5 | Öffentliche Routen, Mehrflugzeugdienste zählen einmal |
| Flughäfen | 3 / 10 | Verschiedene Endpunkte aktiver Routen, inklusive Heimatbasis |
| Passagiere | 1.000 / 1.000.000 | Kumuliert tatsächlich befördert |
| Langstrecke | 5.000 km | Längste aktive Route, Katalogdistanz |
| Umsteiger | 1 | Tatsächlich befördert, nicht nur Angebotsprognose |
| Profitables Jahr | 1 | Abgeschlossenes Kalenderjahr mit positivem Vorsteuerergebnis |
| Jahresumsatz | 10.000.000 EUR | Ticketerlöse und Fracht im selben Kalenderjahr |
| Schuldenfreiheit | 1 | Nichtleere Flotte, weder Restschuld noch Rückstand |
| Ruf | 30 Tage | Aufeinanderfolgende Tage mit mindestens einem Flug und Ruf ≥ 0,5 |

Kalibrierung: Der bestehende ATR-72-/90-EUR-Flugplan EDDF–EDDM erreicht
543.265 Passagiere nach zwei und 2.722.703 nach zehn Jahren. Die 1.000er-Schwelle
fällt auf Tag 3, 10 Mio. Jahresumsatz auf Tag 150 und eine Million Passagiere auf
Tag 1342 (etwa 3,7 Jahre). Die beiden KI-Airlines erreichen nach zwei Jahren
5–6 Flugzeuge und Routen: 5 ist eine mittlere Wachstumsmarke, 15 ein deutliches
Ausbauziel für den Spieler. Die übrigen Netzwerkziele belohnen freiwillige
Expansion statt bloßer Wartezeit.

Save-Version 33 speichert Tageszähler, erreichte Zeitpunkte und
bereits angezeigte Toasts. Fehlt dieser Zustand in einem älteren Save, werden
aktuell erfüllte Bedingungen einmal am Ladezeitpunkt als erreicht gespeichert;
Toasts für diese nachgetragenen Ziele werden unterdrückt. Eine frühere Rufserie
wird nicht erfunden. Bekannte Umsätze im laufenden Kalenderjahr stammen aus dem
bestehenden Finanzarchiv. Bereits vorhandene gültige additive Felder bleiben in
Kompatibilitätsfixtures erhalten. Neue Spiele starten mit leerem Zustand.

Die Übersicht zeigt zwei zuletzt erreichte und zwei nächste offene Ziele mit
aktuellen Fortschrittszahlen; der Rufserienzähler folgt dem Tagesabschluss. Die vollständige Liste ist scrollbar; der
Schließen-Button bleibt sichtbar. Gleichzeitige neue Ziele werden in einem
übersetzten Kit-Toast zusammengefasst, dessen Zustellung Rust speichert.
