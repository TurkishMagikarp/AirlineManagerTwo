# Streckennachfrage – erste Umsetzung

## Markt statt Passagiere pro Flug

Rust berechnet einen **gleichmäßigen Tagesdurchschnitt je Flughafenpaar und
Richtung**, unabhängig von Flugzeug und Sitzplätzen. Hin- und Rückrichtung sind
symmetrisch. Mehrere Routen-IDs, auch mit umgekehrtem Start/Ziel, gehören zum selben
Markt. Das ist ein Ratenmodell, **kein hartes Kontingent je Kalendertag**:
Abflugphasen und ganze Flüge können einzelne Tage vom Durchschnitt abweichen lassen.
Es gibt keine angesammelten oder über Tage wartenden Passagiere.

Für Entfernung `d` in km gilt vor Preisreaktion:

`1700 × sqrt(Größe A × Größe B) × min(d / 300, 1) / (1 + d / 2500)`.

Die geometrische Größe berücksichtigt beide Endpunkte; ein Hub macht einen kleinen
Regionalflughafen nicht zum zweiten Hub. Unter 300 km drücken alternative
Verkehrsmittel die Nachfrage pauschal; auf großen Entfernungen nimmt die Zahl der
Reisenden ab. Keine Behauptung, reale O&D-Verkehrszahlen zu prognostizieren.

## Plausible, ausdrücklich geschätzte Ausgangsdaten

`data/airports.json` enthält einen kombinierten Größen-/Bedeutungsindex und einen
Geschäftsreiseanteil, **keine gemessenen Passagierzahlen**:

| Flughafen | Index | Geschäft |
| --- | ---: | ---: |
| Frankfurt | 1,00 | 65 % |
| München | 0,85 | 60 % |
| Istanbul IST | 1,10 | 45 % |
| Istanbul SAW | 0,55 | 30 % |
| New York JFK | 1,15 | 55 % |
| Los Angeles | 1,10 | 45 % |
| Paderborn | 0,08 | 25 % |
| Hahn | 0,12 | 15 % |

Große internationale Wirtschaftszentren erhalten hohe Indizes, kleinere regionale
Einzugsgebiete niedrige. Frankfurt/München sind geschäftsorientierter, Hahn/SAW
stärker privat-/urlaubsorientiert. Der Streckenanteil ist der Mittelwert beider
Flughafenanteile; der Rest zählt als Urlaub/Privatreise.

Referenzpreis unverändert: `60 + floor(4 × round(d) / 100)` EUR.
Die kaufbereite Nachfrage eines Segments ist dessen Basis ×
`max(0, 1 − Ticketpreis / (Referenzpreis × Preisgrenze))`.
Preisgrenze: Geschäft **2,0**, Urlaub **1,5**. Damit schrecken hohe Preise beide
Gruppen ab, Urlauber früher. Kostenlose Tickets erzeugen keine Einnahmen.

## Verteilung und Abrechnung

Jeder aktive Pendelflugplan bietet `43200 / Abflugabstand_in_Sekunden` Flüge pro
Tag **und Richtung**. Für alle aktiven Pläne eines Flughafenpaars wird diese
Frequenz addiert. Jeder Flug erhält:

`floor(kaufbereite Tagesnachfrage bei seinem Preis / max(1, Gesamtfrequenz))`.

Danach begrenzen die Sitze die Beförderung. Bei gleichem Preis ist die Summe der
beförderten Passagiere pro Tag im Dauerbetrieb höchstens der Tagesmarkt – mehr
Frequenz teilt ihn nur feiner. Zusätzliche Kapazität kann vorher abgewiesene
Passagiere aufnehmen, aber keinen neuen Markt erzeugen. Seltene Flüge sammeln
nicht mehrere Tagesmärkte an. Nicht verkaufte Anteile teurer oder kleiner Flüge
werden noch **nicht umverteilt**. Auch ein aktiver Plan mit zukünftigem Erstabflug
zählt bereits zum angebotenen Dauerfahrplan; ungeplante Routen zählen nicht.

Wie bisher werden Ergebnisse erst bei Landung ermittelt und verbucht. Dabei gilt
das aktuelle Flugplanangebot; ein zusätzlicher/beendeter Plan kann deshalb auch
die Nachfrage eines schon gestarteten Flugs beeinflussen. Noch keine separaten
Buchungen bei Abflug. Preise eigener laufender Flugpläne bleiben unveränderbar.

Alle Werte und Berechnungen liegen in Rust; Stellwerte in `config/demand.json`,
Referenzpreis/Kosten in `config/economy.json`. Anschließend über `START.bat` neu bauen.
Konstante Fahrpläne können weiterhin millionenfach gebündelt abgerechnet werden.

## Anzeige, Spielstände und Tests

Die Routenübersicht zeigt den **gesamten preisabhängigen Markt/Tag/Richtung**,
nicht zusätzlich verfügbare Passagiere für jede Routen-ID. Tooltip: Segmente und
geteilter Anteil vor Sitzlimit. Tatsächliche Auslastung = archivierte Passagiere /
archivierte angebotene Sitze, sitzgewichtet über alle Landungen dieser Route.
Ohne Archivflüge steht „Keine Flüge“.

Das Speicherformat bleibt **Version 7**: Nachfrage leitet sich aus bereits
gespeicherten Flughafencodes, Preisen und Flugplänen ab. Kein zusätzlicher Pool,
der beim Laden zurückgesetzt werden könnte. Finanzarchiv und Kontostand bleiben
unverändert; neue Regeln gelten erst für künftige Landungen. Versionen 1–6 nutzen
die bestehenden Migrationen; deren Route-Auslastung beginnt am Archivbeginn.

Rust- und Godot-Tests prüfen Größen, Preise, beide Segmente, Sitzlimits,
Parallelrouten, umgekehrte Strecken, Frequenzen, Pause/16×, große/kleine Zeitschritte,
Speichern/Laden und unveränderte historische Ergebnisse.

Noch nicht enthalten: saisonale/werktägliche Spitzen, Stadtmärkte über mehrere
Flughäfen, Umsteiger, Wettbewerber, Buchungsvorlauf und echte Quoten pro Kalendertag.
