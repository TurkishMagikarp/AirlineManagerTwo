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

## Finanznot und Unternehmensende

Fristen, einmalige Kreditumschuldung, KI-Marktaustritt und Spielstand v14:
[Restrukturierung und Insolvenz](restructuring.md).

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
Weiterhin keine Saisonalität, Wetter, formale Insolvenz oder Kosten für
geparkte Flugzeuge. Beide Richtungen sind symmetrisch; parallele Routen teilen
jetzt den Tagesmarkt. Sehr billige Tickets können volle, aber verlustreiche Flüge
ergeben. Noch keine harte Quote je Kalendertag und keine Umverteilung unverkaufter
Marktanteile; weitere Grenzen: [Nachfrage](demand.md).

## Kredite und Finanznot (Version 12)

`config/financing.json` steuert alle Kreditstellwerte (nach Änderung neu bauen).
Spieler und KI verwenden dieselben Rust-Methoden und dieselbe Simulationszeit:

- Ein Kredit gleichzeitig, maximal 20 Mio. EUR. Keine automatische Überziehung,
  keine Sicherheitenbewertung. Ein neuer Kredit ist erst nach vollständiger Tilgung
  möglich. Ein bestehendes negatives Konto muss durch die Auszahlung mindestens
  ausgeglichen werden; andernfalls wird der Antrag ohne Zustandsänderung abgelehnt.
- 6 % Jahreszins auf die jeweilige Restschuld, Zahlung alle 30 Simulationstage.
  Formel: `ceil(Restschuld × Jahreszins-Basispunkte × Periodentage / 3_650_000)`.
  Ein Jahr hat hier 365 Tage; kein täglicher Zinseszins und keine untermonatige
  Zinsabgrenzung. Beispiel: 1,2 Mio. EUR → 5.918 EUR Zins für die erste Periode.
- 24 gleichmäßige Tilgungsraten: `ceil(Kreditbetrag / 24)`, letzte Rate höchstens
  Restschuld. Erste Fälligkeit 30 Tage nach Auszahlung. Kostenlose Sondertilgung
  jederzeit aus vorhandenem Guthaben; sie reduziert die Laufzeit, nicht die Rate.
- Am Zahlungstermin werden zuerst Zinsen gebucht, dann fällige Tilgung soweit
  Guthaben vorhanden ist. Fehlende Tilgung bleibt überfällig, ohne Strafzins.
  Späteres Guthaben tilgt Rückstände an der nächsten Abrechnungsgrenze automatisch.
  Künftige Zinsen fallen nur auf verbleibende Hauptschuld an.
- Unter 1 Mio. EUR erfolgt eine Liquiditätswarnung. Negatives Konto **oder**
  überfällige Tilgung bedeuten Finanznot: keine Flugzeugkäufe, Einstellungen,
  neuen Routen oder Flugpläne. Bestehende Flüge landen samt Bodenzeit regulär,
  danach wird der Flugplan ausgesetzt, auch bei lückenlosem Pendelbetrieb.
  Wartung startet nach Finanznot keinen ausgesetzten Flugplan automatisch neu.
- Negative Kontostände repräsentieren offene Betriebs-/Zinsverpflichtungen,
  **kein** frei ausgebbares Kreditlimit. Bereits eingegangene Gehalts- und
  Wartungsverpflichtungen werden weiter gebucht und können das Defizit erhöhen.
  Personal abbauen und Dienste einstellen, einen zulässigen Kredit aufnehmen
  oder laufende Einnahmen nutzen: sobald Konto und Tilgungsrückstände ausgeglichen
  sind, entfällt die Sperre. Kein automatischer Neustart von Diensten.
- Die KI nimmt nur für bestehende Verpflichtungen/Defizite einen Reservekredit
  auf (Ziel: mindestens Warnschwelle oder zwei Gehaltsperioden). Kein Kredit
  für Flugzeugexpansion; mit Restschuld pausiert ihre Flottenerweiterung.
  Überschüsse oberhalb der doppelten Reserve werden sondergetilgt. Nicht rettbare
  Gesellschaften pausieren Expansion und entlassen ungenutztes Personal.

Finanzen zeigt Guthaben, verfügbare Liquidität (`max(Kontostand, 0)`, **ohne**
Reservierung künftiger Kosten), Restschuld, gebuchte Zinsen, nächste Kreditrate,
Gehaltsfälligkeit und Tilgungsrückstände. Warnungen stehen auch in der Übersicht.
Kreditauszahlung und Tilgung ändern das Konto, nicht Gewinn oder Umsatz.
Zinsen gehören zu den zentralen Unternehmenskosten, nicht zu Routenflugkosten.

Das additive Kreditjournal hält Zeit, Kontobewegung, Hauptschuldänderung und
Zinsen fest; Kontodiagramme und Gewinn-/Kostenreihen berücksichtigen es.
Versionen 1–11 bleiben lesbar: keine erfundenen Altkredite, rückwirkenden Zinsen
oder geänderten historischen Flüge/Gehälter. Negative Altkonten starten in
Finanznot. Speichern prüft Restschuld gegen Journal und Konto gegen Historie.
Alle Zahlungstermine, Landungen und Aussetzungen sind unabhängig von 1×/4×/16×
und Schrittgröße. Noch keine Insolvenz, Liquidation, Umschuldung, Verkäufe,
Steuern, Rating-/Sicherheitenmodelle oder ausgefeilte KI-Sanierung.

## Globaler Treibstoffmarkt (Version 16)

Ein gespeicherter Weltseed (`fuel_market.seed`, derzeit gleicher Startseed für
neue Spiele) bestimmt einen weltweiten Preis. `config/economy.json` enthält
Basis 80 Cent/kg, Grenzen 50–120, Volatilität ±12 Cent, Rückkehrdivisor 8 und
Kadenz 604.800 Simulationssekunden (eine Woche). Jede Woche gilt:
`clamp(Preis + Seed-Zufallsschritt + trunc((Basis - Preis) / 8), Minimum, Maximum)`.
SplitMix64 mit explizitem Integer-Wrapping macht die Folge plattformunabhängig.
Es gibt keinen Zufallsaufruf pro Frame und keine zweite Uhr.

Der Preis wird für den genauen Abflugzeitpunkt abgeleitet und im bestehenden
Abflugergebnis als Treibstoffkosten eingefroren. Große Simulationsschritte teilen
Flugabrechnungen an Preisgrenzen; bereits fliegende Flugzeuge behalten ihre
Kosten bis zur Landung. Spieler und KI verwenden dieselbe Folge. KI-Tarifprüfung,
Frequenz- und Flottenplanung rechnen mit dem aktuellen Preis. Die konservative
Liquiditätsprognose verwendet die Obergrenze, damit ein übersprungener Preissprung
keine Finanznot-Ereignisse zwischen großen und kleinen Schritten verändert.

Spielstände vor v16 beginnen beim Laden einen neuen Markt zur Basis, ohne alte
Abflugangebote oder Kontobuchungen zu verändern. Die Finanzansicht zeigt EUR/kg
und die Richtung/Differenz gegenüber vor 30 Simulationstagen; vor Beginn der
Markthistorie gilt die Basis. Kein Chart, keine Absicherung und keine Regionalpreise.

### Prüfung am 13.11.2026 (Linux, Rust)

`cargo test --locked --manifest-path rust/Cargo.toml quality:: -- --nocapture`
wurde vor und nach der Änderung ausgeführt. Alle vier bisherigen Szenarien
bestehen weiter, einschließlich jährlichem Speichern/Laden und Vergleich großer
mit täglichen Schritten. Im ATR-Szenario FRA–MUC mit 90-EUR-Ticket:

| Guthaben nach 10 Jahren | Vorher (80 Cent konstant) | Schwankender Markt |
| --- | ---: | ---: |
| Spieler | 18.054.744 EUR | 17.900.937 EUR |
| Regional Wings | 86.817.438 EUR | 65.252.453 EUR |
| Continental Sky | 170.694.976 EUR | 97.463.052 EUR |

Die KI wächst abhängig von den Preissprüngen anders, bleibt aber in allen
bestehenden Szenarien solvent. Absichtlich unbrauchbare Spielertarife (0 bzw.
100.000 EUR) scheitern weiterhin. Der tägliche Tarifwechsel-Test bleibt unter
3 MiB Savegröße (3.089.102 statt 2.792.194 Bytes).

Ein zusätzlicher Test erzwingt ausschließlich im Testthread durchgehend die
konfigurierte Obergrenze (120 Cent/kg), auch über jährliches Speichern/Laden.
Nach fünf Jahren ohne Finanznot: Spieler 8.251.438 EUR, Regional Wings
9.399.985 EUR, Continental Sky 52.677.116 EUR. Dafür war keine Neukalibrierung
nötig; dies ist eine Szenarioprüfung, keine Garantie für jede Spielerstrategie.

Weitere Rust-Tests prüfen Preisgrenzen, Seedabhängigkeit, Wochenwechsel,
Abflugsnapshot über einen Preissprung, Pause, 1×/4×/16×, Schrittgrößen und
v15-Migration. Godot, START.bat, GUI/Screenshots und Windows-Neustarttests
konnten auf diesem Server nicht ausgeführt werden; die Rust-Tests bestätigen
keine visuelle Darstellung.
