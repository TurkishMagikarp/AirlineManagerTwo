# Wirtschaft: Referenzen, Annahmen und Kalibrierung

Stand: 12.11.2026. Bewusst einfacher, deterministischer **Einzelflug** in ganzen
EUR. Alle Stellwerte stehen in `config/economy.json` (anschließend neu bauen).
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
- Marktgröße `140 + floor(3 × km / 1000)` statt `140 + floor(20 × km / 1000)`.
  Basis bei Referenzpreis maximal `floor(Sitze × 83 / 100)`; damit hat eine ATR
  nicht bei fast jedem vernünftigen Preis automatisch volle Sitze.
  Unter 300 km wird diese Basis mit `km / 300` reduziert. Das verhindert
  besonders lukrative 63-km-Flüge zwischen zwei Flughäfen derselben Stadt.
  Auch das ist nur ein Entfernungsproxy, keine erhobene Streckennachfrage.
- Preisreaktion bleibt einfach: `floor(Basis × max(0, 2 × Referenz − Preis) / Referenz)`.
  Bei 150 % des Referenzpreises ungefähr halbe Basisnachfrage, bei 190 % ungefähr
  ein Zehntel, ab 200 % null. Sitzplätze sind die harte Obergrenze; kostenlose
  Tickets erzeugen keinen Erlös. Es gibt bewusst keinen Rest wohlhabender Käufer
  bei beliebig hohen Preisen. Keine neue Preis-/Konkurrenzsimulation.
- Treibstoff: unveränderte Stundenverbräuche, aber zusätzlich 550 kg (Jets) bzw.
  120 kg (ATR) je Flug für Start/Steigflug/Rollen als **Mehrverbrauch gegenüber
  der reinen Reisezeitformel**. Keine mitberechnete, unverbrauchte Reserve.
  Fuelpreis jetzt in Cent/kg, nicht mehr auf volle EUR/kg eingeschränkt.
- Flughäfen einschließlich vereinfachter Abfertigung beider Endpunkte zusammen:
  Jets 1.400, ATR 600 EUR je Einzelflug plus 25 EUR je Passagier; bisher einheitlich
  400 + 12 EUR. Keine nochmalige Verdoppelung für beide Endpunkte.
- Personal: weiterhin 650/650/350 EUR/h, nun Reisezeit plus 30 Minuten
  Vor-/Nachbereitung statt ausschließlich Reisezeit.
- Neu nur **eine Kostenpauschale**, kein neues System: Wartung/Verwaltung
  Jets `700 + ceil(2.400 × Flugstunden)`, ATR `300 + ceil(800 × Flugstunden)` EUR.
  Schließt vereinfacht Wartungsrücklage, Navigation, Betrieb/Verwaltung ein;
  sind kalibrierte Schätzungen, keine veröffentlichten Vertragspreise. Kaufpreis
  bleibt separat: keine doppelte Abschreibung oder Leasingrate in der Pauschale.

km wird zuerst auf ganze km gerundet; Nachfrage-Zwischenwerte werden abgerundet.
Stundenkosten werden erst pro Kostenkategorie auf ganze EUR aufgerundet; Fuel
wird aus Stunden- und Zyklusmenge gemeinsam berechnet und einmal aufgerundet.
Die bisherige Flugzeit `ceil(Luftlinie / Reisegeschwindigkeit × 3600)` bleibt
unverändert. Kostenaufschläge ändern **keine** Abflug-/Landetermine.

## Reproduzierbare Beispiele aus dem Rust-Test

| Strecke | Modell | Ticket EUR | Passagiere | Auslastung | Erlös EUR | Kosten EUR | Ergebnis EUR |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| FRA–MUC | ATR 72 | 71 | 58/72 | 80,6 % | 4.118 | 3.649 | +469 |
| FRA–MUC | ATR 72 | 134 | 6/72 | 8,3 % | 804 | 2.349 | −1.545 |
| FRA–MUC | ATR 72 | 142 | 0/72 | 0 % | 0 | 2.199 | −2.199 |
| FRA–MUC | A320 | 71 | 139/180 | 77,2 % | 9.869 | 8.163 | +1.706 |
| FRA–IST | A320 | 133 | 145/180 | 80,6 % | 19.285 | 17.672 | +1.613 |
| FRA–IST | A320 | 199 | 73/180 | 40,6 % | 14.527 | 15.872 | −1.345 |
| JFK–LAX | 737-800 | 218 | 151/189 | 79,9 % | 32.918 | 30.914 | +2.004 |
| JFK–LAX | 737-800 | 327 | 75/189 | 39,7 % | 24.525 | 29.014 | −4.489 |
| IST–SAW | ATR 72 | 62 | 12/72 | 16,7 % | 744 | 1.688 | −944 |

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
Guthaben werden **nicht** neu berechnet. Erst künftige Landungen verwenden neue
Werte, auch bei einem bereits laufenden Flug. Bestehende Flugpläne und deren
Zeitgrenzen bleiben gültig. Persönliche Spielstände werden von Tests nicht benutzt.

Weiterhin keine tägliche Marktnachfrage, Konkurrenz, Frequenzsättigung,
Saisonalität, Wetter, Insolvenz oder Kosten für geparkte Flugzeuge. Beide
Richtungen sind symmetrisch; parallele Routen teilen keinen Nachfragepool.
83 % ist eine Referenz-Obergrenze der Basis, keine garantierte Auslastung.
Sehr billige Tickets können volle, aber verlustreiche Flüge ergeben.
