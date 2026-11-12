# Kleine reale Kataloge

## Flughäfen

`airports.json` enthält ausschließlich reale Identitätsdaten, keine erfundenen
Nachfrage-, Kapazitäts-, Gebühren- oder Wirtschaftswerte. Die kleine Auswahl ist
keine weltweite Abdeckung und noch keine vollständige geografische Datenbasis.

- Länder werden über ISO-3166-1-Alpha-2-Codes referenziert.
- Flughäfen werden über ICAO-Codes referenziert; IATA ist optionale Anzeigeinformation.
- Die Airline speichert Codes, keine Array-Indizes oder kopierten Flughafennamen.
- Godot erhält Länder und gefilterte Flughäfen ausschließlich aus Rust.
- Der Katalog wird einmal in Rust eingelesen. Später kann der Loader eine größere,
  lizenzierte Datenquelle verwenden; die Referenzen im Spielstand bleiben stabil.
- Neue Einträge ergänzen, `START.bat` erneut starten: Cargo baut die eingebettete
  Datei neu. Weitere Attribute können später ergänzt werden, ohne die UI-Liste
  mit Wirtschaftsdaten zu vermischen.

Identitäten: Frankfurt **EDDF/FRA**, München **EDDM/MUC**, Istanbul **LTFM/IST**,
Istanbul Sabiha Gökçen **LTFJ/SAW**, New York JFK **KJFK/JFK**, Los Angeles
**KLAX/LAX**. Informationsseiten der jeweiligen Betreiber:
[Frankfurt](https://www.frankfurt-airport.com/), [München](https://www.munich-airport.com/),
[Istanbul](https://www.istairport.com/), [Sabiha Gökçen](https://www.sabihagokcen.aero/),
[JFK](https://www.jfkairport.com/), [LAX](https://www.flylax.com/).

## Flugzeugmodelle

`aircraft_models.json` enthält drei reale Modelle mit stabilen Modell-IDs.
Sitzplätze sind jeweils eine fest gewählte Konfiguration, keine Behauptung,
dass jede reale Airline diese Bestuhlung nutzt. Reichweiten sind vereinfachte
Hersteller-Referenzwerte, keine berechnete Reichweite für eine konkrete Beladung,
Wetterlage oder Route. Es gibt noch kein Kraftstoff- oder Flugleistungsmodell.

| Modell-ID | Typ | Sitzplätze | Referenzreichweite |
| --- | --- | ---: | ---: |
| `a320_200` | Airbus A320-200 | 180 | 6.200 km (A320ceo mit Sharklets) |
| `b737_800` | Boeing 737-800 | 189 | 5.190 km (aktuelle Herstellerübersicht) |
| `atr72_600` | ATR 72-600 | 72 | 1.370 km (Hersteller: max. Passagiere) |

Referenzen: [Airbus A320ceo](https://www.aircraft.airbus.com/en/aircraft/a320-family/a320ceo),
[Boeing 737 Next Generation](https://www.boeing.com/commercial/737ng),
[ATR 72-600](https://www.atr-aircraft.com/our-aircraft/atr-72-600/).

Die **vorläufigen Spielpreise** stehen bewusst getrennt in
`config/aircraft_prices.json` (positive ganze EUR pro Modell-ID). Es sind keine
Hersteller-Listenpreise, Marktpreise oder endgültigen Wirtschaftsregeln. Änderungen
werden beim nächsten Build über `START.bat` übernommen; bereits bezahlte Kaufpreise
und Guthaben in Spielständen bleiben unverändert. Für bestehende Spielstände
referenzierte Modell-IDs beibehalten, auch wenn später weitere Varianten hinzukommen.
