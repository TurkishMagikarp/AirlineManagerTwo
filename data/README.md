# Kleiner Flughafenkatalog

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
