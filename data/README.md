# Reale Kataloge

## Flughäfen

`airports.json` enthält reale Identitätsdaten und geografische Koordinaten sowie
ausdrücklich geschätzte Nachfrageparameter `market_size` und `business_percent`
([Annahmen](../docs/demand.md)), keine gemessenen Streckenpassagierzahlen.
Die Auswahl umfasst 40 Flughäfen in 29 Ländern auf allen bewohnten Kontinenten;
sie ist bewusst kein vollständiges Flughafenverzeichnis. Auch `charge_percent`
und `daily_departure_slots` sind Spielschätzungen, keine echten Gebühren oder
behördlichen Kapazitäten. Frankfurt bleibt der Größenmaßstab 1,00.
Die acht bisherigen Einträge bleiben einschließlich fehlender Slotlimits unverändert.

- Länder werden über ISO-3166-1-Alpha-2-Codes referenziert.
- Flughäfen werden über ICAO-Codes referenziert; IATA ist optionale Anzeigeinformation.
- Die Airline speichert Codes, keine Array-Indizes oder kopierten Flughafennamen.
- Godot erhält Länder und gefilterte Flughäfen ausschließlich aus Rust.
- `latitude` / `longitude`: geografische Breite/Länge in Dezimalgrad (WGS84),
  gerundete Flughafen-Referenzpositionen, nicht einzelne Startbahnen oder Gates.
  Referenz: [OurAirports Flughafen-Datensätze](https://ourairports.com/data/)
  (Public Domain), für die Ergänzungen aus dem
  [OurAirports-CSV](https://github.com/davidmegginson/ourairports-data/blob/main/airports.csv),
  abgerufen am 14.11.2026. Verwendet: ICAO aus `gps_code` (ersatzweise `ident`), `iata_code`, `name`,
  `iso_country`, `latitude_deg`, `longitude_deg` (auf sechs Dezimalstellen gerundet).
  Bestehende Positionen und Namen bleiben unverändert.
  Rust leitet daraus die Großkreisentfernung ab; Spielstände speichern nur Codes.
- Der Katalog wird einmal in Rust eingelesen. Später kann der Loader eine umfangreichere
  Datenquelle verwenden; die Referenzen im Spielstand bleiben stabil.
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

`aircraft_models.json` enthält fünf reale Modelle mit stabilen Modell-IDs.
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

`cruise_kmh` verwendet vereinfachte typische Reisegeschwindigkeiten: A320 **830**,
737-800 **840**, ATR **510 km/h**, keine garantierte durchschnittliche Blockgeschwindigkeit.
Flugzeit = auf volle Sekunden aufgerundete Luftlinie / Geschwindigkeit.
`turnaround_minutes` ist eine vorläufige Spielregel: **45** für Jets, **30** für ATR.
Keine zusätzliche Roll-/Steig-/Sinkflugzeit, Wetter- oder Beladungskorrektur.
Aktive Spielstände referenzieren diese Modellwerte; Änderungen können bestehende
Flugpläne ungültig machen und benötigen künftig eine gezielte Migration.

Die **vorläufigen Spielpreise** stehen bewusst getrennt in
`config/aircraft_prices.json` (positive ganze EUR pro Modell-ID). Es sind keine
Hersteller-Listenpreise, Marktpreise oder endgültigen Wirtschaftsregeln. Änderungen
werden beim nächsten Build über `START.bat` übernommen; bereits bezahlte Kaufpreise
und Guthaben in Spielständen bleiben unverändert. Für bestehende Spielstände
referenzierte Modell-IDs beibehalten, auch wenn später weitere Varianten hinzukommen.

### Widebodies

`a330_300` (300 Sitze, 11.750 km) und `b777_300er` (396 Sitze, 13.650 km)
ergänzen die Klasse `Widebody`. Quellen, abgerufen am 15.11.2026:
[Airbus A330-300](https://www.aircraft.airbus.com/en/aircraft/a330/a330-300)
und [Boeing 777 Design Highlights](https://www.boeing.com/commercial/777/design-highlights).
Airbus nennt 250–290 typische Sitze und maximal 440; 300 ist unsere feste,
dichtere Ausgangskabine. Boeing nennt 396 Sitze in zwei Klassen; im Spiel sind
396 die Economy-Flächeneinheiten, aus denen die bestehende Kabinenregel
Business-Sitze bildet. Keine zusätzliche Sitzkonfiguration oder Reichweitenkurve.

871/905 km/h sind gerundete typische Reisegeschwindigkeiten. 90/120 Minuten
Bodenzeit, 12/18 t freie Bauchfracht und 45/65 t Gesamt-Nutzlast sind vereinfachte
Spielannahmen für Passagiere einschließlich Gepäck und zusätzliche Fracht,
keine garantierten Herstellerwerte bei maximaler Reichweite. Wie bisher zieht
die Frachtregel 100 kg pro Passagier von der Nutzlast ab und begrenzt dann auf
Bauchfracht. Volle Tanks, Wetter und konkrete Triebwerksvarianten werden nicht modelliert.

Die neuen Modellregeln stehen ausschließlich in `config/`: 24/36 Mio. EUR
Spielkaufpreis, 5.500/7.500 kg stündlicher Reisefuel, eigene Zykluskosten,
Flughafentarife, Overhead, Wartungsintervalle und Personalbedarf. Die Besatzungen
(6/8 Piloten mit Rating A330/777, 12/16 Kabinenkräfte) sind Flotten-Personalbedarf
mit vereinfachter Reserve für lange Umläufe; keine neue Dienstzeitregel.
Leasing, Gebrauchtmarkt, Restwert, Versicherung, Kabine und Gebührenmultiplikatoren
nutzen unverändert die gemeinsamen konfigurierten Regeln. Alte Modelle und alle
bestehenden Werte bleiben erhalten. Keine neuen Save-Felder und keine Migration;
Spielstandversion 30 bleibt unverändert.
