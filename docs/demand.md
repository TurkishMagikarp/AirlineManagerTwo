# Streckennachfrage – erste Umsetzung

## Markt statt Passagiere pro Flug

Rust berechnet einen **gleichmäßigen Tagesdurchschnitt je Flughafenpaar und
Richtung**, unabhängig von Flugzeug und Sitzplätzen. Hin- und Rückrichtung sind
symmetrisch groß, aber getrennte Märkte: A→B ist nicht B→A. Mehrere Routen-IDs,
auch mit umgekehrtem Start/Ziel, teilen jeweils den passenden Richtungsmarkt.
Das ist ein Ratenmodell, **kein hartes Kontingent je Kalendertag**:
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
Tag **und Richtung**. Spieler- und KI-Angebote stehen in **einer gemeinsamen
Angebotsliste**, ohne Eigentümerbonus. Pro Segment (Geschäft/Privat) gilt:

- `B`: ursprünglicher Tagesmarkt des Segments vor Preisreaktion.
- `w = max(0, 1 − Preis / (Referenzpreis × Preisgrenze))`: Kaufbereitschaft.
- `a = w / max(1e-9, 1 − w)`: Preisattraktivität, mit endlichem Grenzwert für Freiflüge.
- `F`: Summe der Frequenzen aller für dieses Segment bezahlbaren Angebote (`w > 0`).
- Anteil pro Abflug: `min(B × w, B × a / (max(1, F) + Summe(Frequenz × a)))`.

Die beiden Segmente werden addiert und auf ganze Passagiere abgerundet. Danach
begrenzen die Sitze die Beförderung. Der Term `max(1,F)` ist die vereinfachte
Alternative „nicht fliegen“. Bei gleichen Preisen entspricht die Formel weiterhin
der preisabhängigen Nachfrage geteilt durch die Gesamtfrequenz (bei seltenen
Flügen maximal ein Tagesmarkt). Günstigere Konkurrenz verringert den Anteil
teurerer Angebote. Die Tagesbeförderung aller Airlines zusammen überschreitet
niemals `B`; Preiswettbewerb erzeugt keinen separaten Markt je Airline.

Zusätzliche Kapazität kann zuvor abgewiesene Passagiere aufnehmen, aber keinen
neuen Markt erzeugen. Sitzbedingt nicht verkaufte Anteile werden noch **nicht
umverteilt**. Auch ein aktiver Plan mit zukünftigem Erstabflug zählt bereits zum
angebotenen Dauerfahrplan; ungeplante Routen zählen nicht.

**Beim Abflug** werden Richtung, Landetermin, Ticketpreis, Passagierzahl, Sitze,
Ticketeinnahmen und Betriebskosten als unveränderliches Flugergebnis gespeichert.
Damit steht auch die Auslastung fest. Aktivieren/Beenden anderer Flugpläne ändert
nur künftige Abflüge, nicht bereits gestartete Flüge. **Bei Landung** wird genau
dieses Ergebnis einmal in Guthaben, Finanzsummen und Archiv übernommen. Es gibt
weiterhin keine vorgezogene Auszahlung. Spieler und KI können die Preise auch
bei laufendem Flugplan ändern; die Änderung betrifft nur künftige Abflüge.

Die Verteilung erhält alle aktiven Routen (`Game::market_routes`) ohne Airlinebonus.
Hin- und Rückflug eines Pendelplans liefern je ein Angebot für ihren jeweiligen
Markt, nicht zwei für dieselbe Richtung.
Noch keine individuellen Buchungen oder Reservierungsliste: Bei Fahrplanwechseln
gilt ab dem nächsten Abflug die neue Durchschnittsrate; eine strenge Tagesobergrenze
wird nicht nachträglich erzwungen.

Alle Werte und Berechnungen liegen in Rust; Stellwerte in `config/demand.json`,
Referenzpreis/Kosten in `config/economy.json`. Anschließend über `START.bat` neu bauen.
Konstante Fahrpläne können weiterhin millionenfach gebündelt abgerechnet werden.

## Anzeige, Spielstände und Tests

Die Routenübersicht zeigt die **preisabhängige Nachfrage ohne Preiswettbewerb**,
nicht zusätzlich verfügbare Passagiere für jede Routen-ID. Tooltip: Segmente und
geteilter Anteil vor Sitzlimit. Tatsächliche Auslastung = archivierte Passagiere /
archivierte angebotene Sitze, sitzgewichtet über alle Landungen dieser Route.
Ohne Archivflüge steht „Keine Flüge“. Der Wettbewerbsbereich zeigt Basismarkt und
alle passenden Spieler-/KI-Angebote mit kapazitätsbegrenzten Schätzungen pro Tag
und Richtung. Prozentwerte sind Anteile der geschätzt beförderten Passagiere.

Speicherformat **Version 9** speichert auch KI-Unternehmen und Entscheidungszeiten;
seit Version 8 steht das Abflugergebnis im laufenden Flugplan.
Laden berechnet es nicht neu. Fehlende/widersprüchliche Ergebnisse in neuen
Spielständen werden abgelehnt. Versionen **1–7** bleiben ladbar: Für einen bereits
laufenden Altflug wird einmal beim Laden anhand des gespeicherten Angebots ein
Ergebnis eingefroren, da sein tatsächlicher Abflugzustand früher nicht gespeichert
wurde. Vergangene Landungen, Finanzarchiv und Kontostand bleiben unverändert.
Danach wird der migrierte Flug wie jeder andere nur bei seiner Landung abgerechnet.
Alte Spielstände erhalten KI-Angebote erst ab dem Ladezeitpunkt. Die neue
Konkurrenz schreibt keine historischen Spielerflüge oder gespeicherten Ergebnisse um.

Rust- und Godot-Tests prüfen Größen, Preise, beide Segmente, Sitzlimits,
Parallelrouten, umgekehrte Strecken, Frequenzen, Pause/16×, große/kleine Zeitschritte,
Speichern/Laden und unveränderte historische Ergebnisse.

Noch nicht enthalten: saisonale/werktägliche Spitzen, Stadtmärkte über mehrere
Flughäfen, Umsteiger, Buchungsvorlauf und echte Quoten pro Kalendertag.
