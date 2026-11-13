# Wirtschaft: Referenzen, Annahmen und Kalibrierung

Stand: 12.11.2026. Bewusst einfache, deterministische Flugabrechnung in ganzen
EUR. Nachfrage jetzt als [geteilter Tagesmarkt](demand.md), Stellwerte in
`config/demand.json`, `data/airports.json` und `config/economy.json` (neu bauen).
Keine Livepreise und keine Behauptung, reale Flugpreise vorhersagen zu können.

## Belastbare Anhaltspunkte

- [Lufthansa Group, Passenger Airlines 2024](https://report.lufthansagroup.com/2024/annual-report/en/combined-management-report/business-segments/passenger-airlines-business-segment/):
  83,1 % Sitzladefaktor, 3,5 % Adjusted-EBIT-Marge; Europaverkehr 11.784 Mio.
  EUR Nettoverkehrserlös bei 106,788 Mio. Passagieren, also rund 110 EUR pro
  befördertem Passagier. Das ist ein Netz-/Kabinenmix, kein FRA–MUC-Tarif.
- [IATA, Ausblick vom 9.12.2025](https://www.iata.org/en/pressroom/2025-releases/2025-12-09-01/):
  damalige 2026-Prognose 83,8 % Auslastung, 6,9 % operative und 3,9 % Nettomarge;
  Kerosin 90 USD/Barrel für 2025. Bewusst datierter Normalbetriebsanker, **nicht**
  der aktuelle Krisen-/Spotpreis. Mit 159 Litern/Barrel, ca. 0,8 kg/L und
  angenommenen 0,92 EUR/USD entspricht das rund 0,65 EUR/kg vor Lieferung.
  Spielwert 0,80 EUR/kg einschließlich pauschalem Beschaffungsaufschlag.
- [ATR 72-600 Herstellerdatenblatt (2020)](https://www.atr-aircraft.com/wp-content/uploads/2020/07/Factsheets_-_ATR_72-600.pdf):
  762 kg/h Reiseverbrauch. Bisherige 750 kg/h bleiben als gerundeter Modellwert;
  die A320-/737-Werte 2.500/2.600 kg/h bleiben vereinfachte Verbrauchsannahmen
  für ältere Jets, keine garantierten Herstellerwerte.
- [Fraport: Flughafenentgelte](https://www.fraport.com/en/business-areas/operations/airport-charges.html):
  Entgelte für Start/Landung, Abstellen und Passagiereinrichtungen; reale Beträge
  hängen u. a. von Flugzeug, Lärm und Verkehr ab. Daher kein vorgeblich genauer
  Tarif für alle Flughäfen, sondern transparente Modell-/Passagierpauschalen.

Branchenmargen sind **keine** Zielmarge jedes einzelnen Flugs. Gute Verbindungen
dürfen darüber liegen, falsche Preise und Flugzeugwahl müssen Verluste erzeugen.
Das Spielergebnis ist weiterhin kein vollständiger Unternehmens-Nettogewinn.

## Änderungen und einfache Regeln

- Referenzticket: `60 + floor(4 × km / 100)` statt `60 + floor(12 × km / 100)`.
  FRA–MUC 71 statt 95 EUR, FRA–IST 133 statt 280 EUR, JFK–LAX 218 statt 536 EUR.
  Gemeint ist der vereinnahmte Einwegpreis, **nicht** Endkundenpreis plus nochmals
  dieselben Gebühren. Keine länderspezifische Steuer-/Mehrwertsteuerberechnung.
  Gespeicherte und neue Standardtickets bleiben bei 100 EUR.
- Die frühere sitzabhängige Einzelflugnachfrage wurde ersetzt: Flughafenpaar,
  Größe/Bedeutung, Entfernung und Geschäfts-/Urlaubssegmente bestimmen jetzt den
  Tagesmarkt. Aktive Flugpläne teilen ihn über ihre Frequenzen. Formeln und
  ausdrückliche Vereinfachungen stehen in [Nachfrage](demand.md).
- Treibstoff: unveränderte Stundenverbräuche, aber zusätzlich 550 kg (Jets) bzw.
  120 kg (ATR) je Flug für Start/Steigflug/Rollen als **Mehrverbrauch gegenüber
  der reinen Reisezeitformel**. Keine mitberechnete, unverbrauchte Reserve.
  Fuelpreis jetzt in Cent/kg, nicht mehr auf volle EUR/kg eingeschränkt.
- Flughäfen einschließlich vereinfachter Abfertigung beider Endpunkte zusammen:
  Jets 1.400, ATR 600 EUR je Einzelflug plus 25 EUR je Passagier; bisher einheitlich
  400 + 12 EUR. Keine nochmalige Verdoppelung für beide Endpunkte.
- Personal: ausschließlich die Mitarbeitergehälter nach `config/staff.json`
  (siehe [Personal](staff.md)). Keine zusätzliche Crewpauschale je neuem Flug.
  Historische Crewkosten und bereits gespeicherte Abflugangebote bleiben erhalten.
  Unternehmenssummen und -diagramme enthalten gebuchte Gehälter und Wartungschecks;
  Routenberichte bleiben Flugdeckungsbeiträge ohne diese zentralen Ausgaben.
- Neu nur **eine Kostenpauschale**, kein neues System: Wartung/Verwaltung
  Jets `700 + ceil(2.400 × Flugstunden)`, ATR `300 + ceil(800 × Flugstunden)` EUR.
  Schließt vereinfacht Wartungsrücklage, Navigation, Betrieb/Verwaltung ein;
  sind kalibrierte Schätzungen, keine veröffentlichten Vertragspreise. Kaufpreis
  bleibt separat: keine doppelte Abschreibung oder Leasingrate in der Pauschale.
  Seit Version 10 gibt es zusätzlich echte Wartungschecks (Zeit und Kosten nach
  Flugstunden/Zyklen, `config/maintenance.json`, siehe README „Wartung“); die
  Pauschale bleibt als laufender Betriebskostenanteil unverändert.

Der Referenzpreis verwendet auf ganze km gerundete Entfernung; die neue
Marktberechnung verwendet ungerundete Entfernung und rundet erst Passagiere ab.
Stundenkosten werden erst pro Kostenkategorie auf ganze EUR aufgerundet; Fuel
wird aus Stunden- und Zyklusmenge gemeinsam berechnet und einmal aufgerundet.
Die bisherige Flugzeit `ceil(Luftlinie / Reisegeschwindigkeit × 3600)` bleibt
unverändert. Kostenaufschläge ändern **keine** Abflug-/Landetermine.

## Reproduzierbare Matrix aus dem Rust-Test

Passagierzahlen hängen jetzt auch vom Flugplan und parallelen Routen ab; die
alten Einzelflug-Beispielzahlen gelten nicht mehr. Die Matrix gibt aktuelle
Ergebnisse ohne KI-Konkurrenz mit dem jeweils kürzesten zulässigen Abflugabstand aus:

`cargo test --locked --manifest-path rust/Cargo.toml calibrated_route_matrix -- --nocapture`
prüft vier Strecken, alle Modelle innerhalb ihrer Reichweite, jeweils sieben
Preisstufen: gratis, Standardpreis 100 EUR, Referenz, 150 %, 190 %, 200 % und
100.000 EUR. Jede Kombination wird auch tatsächlich bis zur Landung simuliert
und auf korrekte Kontobuchung geprüft. Ein voller JFK–LAX-Flug für 100 EUR
verliert trotz voller Sitze Geld.
Zusätzlich prüfen Tests Kontobewegungen, leere Flüge, Fortschreibung, Überläufe,
Speichern/Neustart und historische Ergebnisse. ATR auf FRA–IST/JFK–LAX bleibt
wegen Reichweite ausgeschlossen; keine wirtschaftlich unmöglichen Testflüge.

## Spielstände und Grenzen

Version 6 bleibt lesbar: das additive Kostenfeld `overhead` erhält bei alten
Finanzsummen/Ergebnissen den Wert null. Historische Kosten, Tickets, Käufe und
Guthaben werden **nicht** neu berechnet. Seit Version 8 behalten bereits laufende
Flüge ihr gespeichertes Ergebnis; neue Werte gelten ab künftigem Abflug. Bestehende Flugpläne und deren
Zeitgrenzen bleiben gültig. Persönliche Spielstände werden von Tests nicht benutzt.

Zwei erste KI-Konkurrenten nutzen dieselben Kosten- und Nachfrageregeln wie der
Spieler; Details: [README](../README.md#erste-ki-fluggesellschaften).
Weiterhin keine Saisonalität, Wetter, Insolvenz oder Kosten für
geparkte Flugzeuge. Beide Richtungen sind symmetrisch; parallele Routen teilen
jetzt den Tagesmarkt. Sehr billige Tickets können volle, aber verlustreiche Flüge
ergeben. Noch keine harte Quote je Kalendertag und keine Umverteilung unverkaufter
Marktanteile; weitere Grenzen: [Nachfrage](demand.md).
