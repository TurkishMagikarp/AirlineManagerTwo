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
- Flughäfen: Jeder Endpunkt berechnet `ceil((Modellbasis + 25 × Pax) × charge_percent / 100)` EUR.
  Modellbasis aus `config/economy.json`: Jets 1.400, ATR 600 EUR. Start und Ziel
  werden einzeln aufgerundet und addiert, auch auf dem Rückflug. Leerflüge zahlen
  weiterhin die Modellbasis anteilig. `charge_percent` in `data/airports.json`
  ist eine vereinfachte Spielkalibrierung, kein realer Flughafentarif: FRA 60,
  MUC 55, IST/JFK/LAX 65, SAW 45, PAD 25, HHN 30 Prozent. So liegen große Hubs
  etwa doppelt so hoch wie Regionalflughäfen. 50 % je Endpunkt entspräche dem
  früheren Gesamttarif. Beispiel ATR mit 40 Pax: FRA–MUC 1.840 EUR, PAD–HHN 880 EUR.
  Godot zeigt die Stufe in der Flughafenauswahl, eine Kostenvorschau vor Erstellung
  (Jahresmittel, Standardpreise, ein Abflug täglich je Richtung) und erwartete
  Gebühren je Abflug der aktiven Dienste. Rust berechnet alle Beträge.
  Save-Version 24 ist eine Identitätsmigration: keine neuen Zustandsfelder,
  Geld, Historie und gespeicherte Abflugangebote aller Eigentümer bleiben exakt
  erhalten. Neue Tarife gelten erst für neue Abflüge; keine Nachbelastung.
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
Saisonale Segmentnachfrage: [Nachfrage](demand.md#saisonale-nachfrage-365-tage-jahr).
Weiterhin kein Wetter oder Kosten für
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

## Saisonvergleich bei dauerhafter Fuel-Obergrenze

`quality::sustained_high_fuel_keeps_viable_player_and_both_ai_alive`, fünf Jahre
mit 120 Cent/kg, Spieler ATR FRA–MUC zu 90 EUR, gleiche Ausgangslage und beide KI.
Baseline `c2c71e6` ohne Saison gegenüber saisonaler Segmentnachfrage und
Jahresplanung. Kontostand am jeweiligen Jahresende in Mio. EUR:

| Jahr | Spieler vorher / nachher | Regional Wings vorher / nachher | Continental Sky vorher / nachher |
| --- | ---: | ---: | ---: |
| 1 | 7,655 / 7,668 | 8,099 / 8,070 | 12,053 / 11,537 |
| 2 | 7,777 / 7,803 | 9,086 / 9,032 | 22,194 / 21,128 |
| 3 | 7,893 / 7,940 | 6,944 / 6,856 | 32,250 / 30,637 |
| 4 | 8,194 / 8,215 | 8,161 / 8,131 | 42,426 / 40,277 |
| 5 | 8,251 / 8,241 | 9,400 / 9,394 | 52,677 / 49,996 |

Beide Läufe überleben ohne Finanznot an den jährlichen Prüfungen und speichern/
laden jedes Jahr. Der saisonale Lauf prüft zusätzlich ungefähr monatlich:
keine Finanznot/Restrukturierungsfrist und keine mehrmonatige Dienstaufgabe.
Regional Wings plant im dritten Jahr kurz ohne aktiven Dienst neu; liquide,
ohne Kredit oder Restrukturierung, anschließend wieder aktiv. Keine jährliche
Winter-Stilllegung. Nach fünf Jahren verändert sich der Kontostand um −0,13 % /
−0,06 % / −5,09 %. Normiertes Marktmittel verhindert keine Gewinneffekte durch
Sitzlimits, Rundung, Konkurrenz oder andere Planungsentscheidungen.
Dies ist ein deterministisches Qualitätsszenario, keine allgemeine Balancegarantie.
Godot-/GUI- und Windows-Neustartprüfung bleiben offen; siehe [Nachfrage](demand.md).

## Vereinfachte Ertragsteuer

`config/tax.json` enthält feste Heimatland-Sätze in Basispunkten (100 = 1 %):
Deutschland 30 %, Türkei 25 %, USA 26 %, sonst 25 % als Spielstandard.
Die gerundeten Näherungen orientieren sich an den kombinierten gesetzlichen
Sätzen aus [OECD Corporate Tax Statistics 2026](https://www.oecd.org/en/publications/corporate-tax-statistics-2026_73af6222-en/full-report/statutory-corporate-income-tax-rates_ce84abb9.html)
und der [Länderübersicht 2025 der Tax Foundation](https://taxfoundation.org/data/all/global/corporate-tax-rates-by-country-2025/).
Sie sind Spielparameter, keine aktuelle steuerliche Beratung. Kommunale Unterschiede,
Progression, Sonderregeln, Abkommen und Transferpreise werden nicht modelliert.

Spieljahre dauern 365 Tage ab Simulationsbeginn. Am Jahresende werden zuerst die
Landungen und fälligen Rechnungen (Gehälter, Wartung, Leasing, Zinsen/Gebühren)
gebucht, dann die Steuer und anschließend die Tilgung und Liquiditätsprüfung.
Das Vorsteuerergebnis entspricht den in Finanzen erfassten Einnahmen minus
Kosten, ohne Steuer. Käufe, Verkäufe, Kautionen und Kredit-Tilgungen sind wie
bisher Kapitalbewegungen und kein Aufwand/Ertrag dieser Ergebnisrechnung.
Verluste werden unbegrenzt vorgetragen; positive Ergebnisse verbrauchen zuerst
den Verlustvortrag. Auf den verbleibenden Gewinn wird der Heimatflughafen-Satz
angewandt, kaufmännisch auf ganze EUR gerundet (exakte Ganzzahlarithmetik).
Die Steuer wird auch bei fehlendem Guthaben gebucht und nutzt dieselbe
Verschuldungs-/Restrukturierungslogik wie andere Ausgaben. KI-Airlines folgen
denselben Regeln. Steuerkosten werden nicht nochmals auf einzelne Routen verteilt.

Finanzen zeigt den letzten Jahresabschluss mit Jahr, Steuer, Verlustvortrag und
Heimatsteuersatz; Jahr 0 bedeutet noch kein Abschluss. Das Jahresarchiv enthält
auch Nullabschlüsse und Vorsteuerergebnisse. Unternehmensdiagramme und Gesamtkosten
enthalten gebuchte Steuern; eine eigene Steuerkurve zeigt die Jahresbuchungen.
Godot rendert ausschließlich Rust-Werte.

Save-Version 19 speichert Steuerbeginn, Ergebnisbasis und Jahresabschlüsse.
Ältere Saves beginnen am Ladezeitpunkt ohne Verlustvortrag, mit dem bereits
aufgelaufenen Vorsteuerergebnis als Basis. Nur die danach erfassten Ergebnisse
werden beim nächsten 365-Tage-Jahreswechsel besteuert, nie vergangene Jahre.

## Treibstoff-Lieferverträge

Rust-Regeln: `config/fuel_contracts.json`, `rust/src/fuel_contracts.rs`. Eine Airline kann 25/50/75 % ihres Treibstoffs für 3/6/12 Monate (je 30 Tage) zum aktuellen Weltmarktpreis plus 2 % sichern. Der Preis wird auf ganze ct/kg aufgerundet. Keine Vorauszahlung, Lagerhaltung oder Kündigung. Neue Verträge sind bei Zahlungsnot, Restrukturierung oder Insolvenz gesperrt; bestehende Verträge laufen weiter. Nach Ablauf ist ein neuer Abschluss möglich.

Der Anteil gilt für die physische Treibstoffmenge jedes Abflugs in `[Beginn, Ende)`. Markt- und Vertragspreis werden vor der einmaligen Aufrundung auf ganze EUR gewichtet. Ein Abflug genau am Ende nutzt vollständig den Marktpreis. Bereits gestartete Flüge behalten ihre Kosten, auch wenn während des Flugs ein Vertrag beginnt oder endet. Abrechnung erfolgt wie bisher bei Landung. `fuel_contract_effect` dokumentiert die Differenz zur reinen Marktrechnung: positiv = Mehrkosten, negativ = Ersparnis; sie ist bereits in den Treibstoffkosten enthalten und wird nicht nochmals belastet. Finanzen zeigt den kumulierten Effekt gebuchter Landungen.

KI prüft beim bestehenden wöchentlichen Review: unter dem langfristigen Mittelwert und ab 2 Mio. EUR Kasse sichert sie 25 % für 3 Monate. Gleiche Abschluss- und Abrechnungsregeln; keine Sonderkonditionen. Bereits an der Review-Grenze abgeflogene Flüge bleiben unverändert. Bei Insolvenz wird die KI samt Vertrag aus der Welt entfernt, ohne Stornoerlös oder Folgekosten.

Save-Version 21: alte Spielstände erhalten für alle Airlines keinen Vertrag, bestehende Flugkosten bleiben erhalten. Abgelaufene Vertragsdaten bleiben bis zum nächsten Abschluss gespeichert, haben aber keine Preiswirkung. Der Vertrag verwendet ausschließlich die Rust-Spielzeit; Pausen, Geschwindigkeit und Frame-Größe ändern seine Laufzeit nicht.

Rust-Vergleichstest über zehn Jahre, tägliche Schritte und jährliches Save/Load: beide KI-Airlines bleiben ohne Zahlungsnot / Restrukturierung. Jede Jahresprobe bleibt innerhalb 5 % der Kasse des Kontrolllaufs ohne KI-Verträge. Im zehnten Jahr: Regional Wings 166.657.714 EUR mit Vertragspolitik gegenüber 170.261.977 EUR ohne; Continental Sky 172.315.707 gegenüber 177.245.265 EUR. Dies ist ein reproduzierbarer Szenariovergleich, keine Garantie für jede Weltkonfiguration.

## Flughafenentgelte: Vergleich AIR-65

Zehn Jahre mit identischer ATR-Spielerroute FRA–MUC, gleicher Welt und KI-Regeln,
jährlichem Save/Load des neuen Laufs. Kontrolllauf nutzt ausschließlich den alten
flachen Tarif; keine andere Regel ist geändert. Beide KI-Airlines bleiben bei
allen Jahresproben liquide, ohne Zahlungsnot, und fliegen weiter. Jahr 10:
Regional Wings 166.538.470 statt 242.665.558 EUR (−31,4 %), Continental Sky
122.567.904 statt 140.425.485 EUR (−12,7 %). Jede Jahresprobe liegt zwischen 50 %
und 150 % der Kontrollkasse. Die KI reagiert auch mit anderen Routen/Flotten;
dieser Vergleich misst daher die Gesamtwirkung einschließlich ihrer Entscheidungen.
Keine zusätzliche KI-Subvention oder Tarifsonderregel nötig. Der vorhandene
Vier-Szenarien-Zehnjahrestest besteht ebenfalls mit Jahres-/Tagesschritten und
jährlichem Neustart. Kein Nachweis für beliebige Konfigurationen.

Serverprüfung: Rust-Tests einschließlich Tarifrundung/Leerflug/Rückflug, alter
Abflugangebote aller Eigentümer und Pause/1×/4×/16×/Save-Load. Godot, GUI/Screenshots
bei den relevanten Fenstergrößen, START.bat und Windows-Prozessneustarts können
hier nicht ausgeführt werden. Der ergänzte Godot-Routen-UI-Test bleibt ungetestet.

Der abschließende isolierte Million-Flüge-Test besteht: 293,7 s für den Advance
bei unverändertem 300-s-Limit; Save 111.396.587 Bytes, einschließlich Laden und
Fortsetzung 366,2 s. Die übrigen 201 Rust-Tests sowie Build/Clippy bestehen separat.
Vorherige Gesamtläufe überschritten das Zeitlimit (310/335 s). Der vorbereitete
Quote-Kontext hält deshalb feste Flughafensätze, Flugzeit, Kabine und Modellraten;
Rückflüge klonen nicht mehr den ganzen Flugplan samt altem Abflugangebot.
Zeitabhängige Nachfrage, Nutzlast und Treibstoff bleiben je Abflug berechnet.
