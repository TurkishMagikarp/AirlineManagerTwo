# Streckennachfrage – erste Umsetzung

## Markt statt Passagiere pro Flug

Rust berechnet einen **saisonalen Tagesdurchschnitt je Flughafenpaar und
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

- `B`: Tagesmarkt des Segments × saisonaler Segmentfaktor, vor Preisreaktion.
- `w = max(0, 1 − Preis / (Referenzpreis × Preisgrenze))`: Kaufbereitschaft.
- `a = w / max(1e-9, 1 − w)`: Preisattraktivität, mit endlichem Grenzwert für Freiflüge.
- `F`: Summe der Frequenzen aller für dieses Segment bezahlbaren Angebote (`w > 0`).
- Anteil pro Abflug: `min(B × w, B × a / (max(1, F) + Summe(Frequenz × a)))`.

Dieser anfängliche Anteil wird mit der Frequenz zur Tagesrate umgerechnet.
Die gemeinsame Sitzkapazität eines Angebots ist Frequenz × Sitze (aus der
jeweiligen Flotte). Überschreiten Geschäft + Privat diese Kapazität, werden beide
Anteile proportional gekürzt. Der Überhang wird getrennt je Segment unter den
übrigen Angeboten mit freien Sitzen im Verhältnis Frequenz × Attraktivität
verteilt. Bei erneutem Sitzmangel wird wiederholt, bis kein aufnehmendes Angebot
mehr übrig ist. Beide Segmente teilen sich dieselben Sitze; `w = 0` erhält niemals
Überhang. Pro Angebot bleibt auch die kaufbereite Tagesmenge begrenzt:
`B × w × min(1, Frequenz)`. Seltene Flüge sammeln keine Nachfrage an.
Erst danach werden die beiden Segmente pro Abflug addiert und auf ganze
Passagiere abgerundet; Rundungsreste werden nicht weiterverteilt.
Der Term `max(1,F)` ist die vereinfachte Alternative „nicht fliegen“. Vor der Sitzumverteilung entspricht die Formel bei gleichen Preisen weiterhin
der preisabhängigen Nachfrage geteilt durch die Gesamtfrequenz (bei seltenen
Flügen maximal ein Tagesmarkt). Günstigere Konkurrenz verringert den Anteil
teurerer Angebote. Die Tagesbeförderung aller Airlines zusammen überschreitet
niemals `B`; Preiswettbewerb erzeugt keinen separaten Markt je Airline.

Zusätzliche Kapazität kann zuvor abgewiesene Passagiere aufnehmen, aber keinen
neuen Markt erzeugen. Umverteilt wird ausschließlich Sitzüberhang; die Alternative
„nicht fliegen“ wird dadurch nicht zusätzlich zu Nachfrage. Sind alle bezahlbaren
Angebote voll, verfällt der Rest. Die Sortierung nach Preis, Frequenz und Sitzen
macht auch die Gleitkommasummen unabhängig von der Reihenfolge der Airlines.
Gleiche wirtschaftliche Angebote erhalten gleiche Anteile. Auch ein aktiver Plan
mit zukünftigem Erstabflug zählt bereits zum angebotenen Dauerfahrplan; ungeplante Routen zählen nicht.

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
Jeder künftige Abflug wird mit seinem eigenen saisonalen Faktor berechnet;
das Archiv fasst weiterhin aufeinanderfolgende identische Flugergebnisse zusammen.

## Anzeige, Spielstände und Tests

Die Routenübersicht zeigt je Route **Basismarkt / erwartete eigene Passagiere /
wegen Sitzmangel abgewiesen**, jeweils pro Tag und Richtung. Rust liefert alle
Werte; Godot formatiert sie nur. Erwartete Beförderung berücksichtigt Umverteilung,
Sitzgrenze und Abrundung pro Abflug. Abgewiesen ist der positive Unterschied
zwischen anfänglichem eigenen Tagesanteil und zugeteilter Beförderung vor
Rundung. Diese Reisenden können bei der Konkurrenz unterkommen; die Zahl ist
kein zusätzlicher Markt und nicht der unerfüllte Rest aller Airlines. Ohne Plan
sind Beförderung und Sitzmangel null. Tageswerte werden zur Anzeige abgerundet.
Tatsächliche Auslastung = archivierte Passagiere / archivierte angebotene Sitze,
sitzgewichtet über alle Landungen dieser Route. Ohne Archivflüge steht „Keine
Flüge“. Der Wettbewerbsbereich zeigt alle passenden Spieler-/KI-Angebote mit
denselben kapazitätsbegrenzten Schätzungen pro Tag und Richtung.

Angebotskapazitäten sind abgeleitete, temporäre Daten. Die Umverteilung fügt
keinen Speicherzustand hinzu und braucht keinen Versionssprung oder neue
Migration. Bestehende Abflugergebnisse bleiben unverändert.

Seit Speicherformat **Version 9** werden auch KI-Unternehmen und
Entscheidungszeiten gespeichert;
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

Noch nicht enthalten: werktägliche Spitzen, Stadtmärkte über mehrere
Flughäfen, Umsteiger, Buchungsvorlauf und echte Quoten pro Kalendertag.


## Zehnjahresvergleich der KI (Sitzumverteilung)

Audit `quality::ten_year_scenarios_reconcile_and_survive_annual_restart`,
Basis `aa50c46` gegenüber Sitzumverteilung. Kumuliertes Betriebsergebnis nach zehn
Jahren in Mio. EUR, jeweils Regional Wings / Continental Sky:

| Spielertarif | Vorher | Nachher |
| --- | ---: | ---: |
| kein Spielerangebot | 175,52 / 263,13 | 175,59 / 263,61 |
| 0 EUR | 5,72 / 13,32 | 11,98 / 13,24 |
| 90 EUR | 143,57 / 232,67 | 143,57 / 232,70 |
| 100.000 EUR | 17,78 / 19,64 | 17,85 / 19,68 |

In beiden Läufen keine KI-Insolvenz; beide Airlines überleben alle jährlichen
Prüfungen ohne offene Restrukturierungsfrist. Meist bleiben die Änderungen unter
0,5 %. Beim kostenlosen Spielerangebot profitiert Regional Wings deutlich von
aufnehmbarer Überlaufnachfrage (+109 % Betriebsergebnis); Continental Sky endet
mit zwei statt drei Flugzeugen und zwei statt drei Routen bei rund 0,6 % weniger
Betriebsergebnis. Das sind deterministische Szenarien, keine allgemeine
Balancegarantie. Jährliches Speichern/Laden und tägliche gegenüber jährlichen
Zeitschritten stimmen weiterhin exakt überein.

Auf dem Linux-Server bestanden Rust Build, alle 126 Tests und Clippy. Godot,
START.bat, GUI/Screenshots und Windows-Prozessneustarts konnten dort nicht geprüft
werden. Der angepasste Godot-Anzeigetest wurde nicht ausgeführt; das tatsächliche
Layout benötigt eine Godot-Prüfung.


## Saisonale Nachfrage (365-Tage-Jahr)

`config/demand.json` enthält je zwölf Stützwerte für Geschäft und Urlaub/Privat.
Tag 1 ist der Januar-Anfang; die zwölf Modellmonate sind gleich lang
(365/12 Tage). Zwischen benachbarten Werten interpoliert Rust mit
`(1 − cos(π × Monatsanteil)) / 2`, einschließlich Dezember → Januar.
Wert und Steigung sind an allen Übergängen stetig. Jede Kurve wird durch den
Mittelwert ihrer zwölf Werte geteilt: Das kontinuierliche Jahresmittel ist
exakt 1,0, auch nach einer Änderung der Amplituden.

Urlaub erreicht 1,35 im Sommer und 1,20 im Dezember, mit 0,70 im Februar.
Geschäft bleibt ruhiger: 0,94 im Sommer, 0,90 im Dezember, maximal 1,04.
Die Kurven sind geschätzte Spielregeln, keine empirischen Reisedaten.
Kein Hemisphärenwechsel: Die vorhandenen Flughäfen liegen nördlich des Äquators;
ein späterer Wechsel sollte Sommer und Jahresendfeiertage getrennt behandeln.

Nur der Basismarkt `B` wird je Segment skaliert. Preisreaktion, Sitzgrenzen,
Umverteilung und mehrere Flugzeuge pro Route bleiben dieselben Regeln.
Die gemeinsame Simulationszeit am **Abflug** bestimmt den Faktor für Spieler
und KI. Pause und Geschwindigkeit erzeugen keine eigene Saisonzeit.
Die Routenansicht zeigt beide aktuellen Rust-Faktoren unter der Tabelle;
Markt- und Wettbewerbswerte nutzen ebenfalls die aktuelle Saison.

KI-Preise, Expansion, Wiedereinstieg und Frequenzplanung verwenden den
Jahresbasismarkt (Segmentfaktoren 1,0), mit dem aktuellen Fuelpreis und realem
Angebot. Eine beobachtete Verlustwoche zählt zur Stilllegung nur, wenn auch die
Jahresplanung negativ ist, einschließlich Wartungsanteil und Leasing;
bei mehreren Maschinen zusätzlich Personal. Die Auslastungsprüfung verwendet
ebenfalls die Jahresplanung. Saisonale Spitzen lösen deshalb keine zusätzliche
Expansionsentscheidung aus; Liquiditätsregeln und echte Kosten gelten weiterhin.
Das ist eine geglättete Planungsnäherung: Wegen Sitzlimits und Rundung entspricht
der Gewinn bei mittlerem `B` nicht exakt dem mittleren Jahresgewinn.

Keine neuen gespeicherten Felder, Speicherformat bleibt **Version 16**.
Bestehende Abflugangebote und historische Landungen werden nicht neu berechnet.
Alte Migrationspfade bleiben erhalten. Rust-Tests prüfen kontinuierliche Kurven,
Jahresmittel, Segment-/Preisregeln, ein ganzes Jahr mit 1×/4×/16×,
unterschiedliche Schritte, Pause und Sommer-Neustart sowie das unveränderte
Abflugangebot eines alten Spielstands. KI-Tests prüfen saisonunabhängige
Expansion und den Erhalt jährlich tragfähiger Dienste trotz Verlustwochen.
Der fünfjährige Fuel-Obergrenzenlauf prüft zusätzlich ungefähr monatlich, dass
Spieler und beide KI ohne Finanznot leben. Eine kurze KI-Neuplanung ohne
aktiven Dienst ist erlaubt, mehrere aufeinanderfolgende Monatsproben ohne
Dienst oder Wartung dagegen nicht.

Godot/GUI, Screenshots, START.bat und Windows-Prozessneustarts sind auf diesem
Linux-Server nicht ausführbar. Der ergänzte Godot-Anzeigetest ist ungetestet;
Rust-Tests bestätigen kein tatsächliches Layout.

Der Zehnjahreslauf mit täglich wechselnden Tarifen speichert rund 3,89 MB;
Saisonwechsel erzeugen zusätzliche legitime Archivblöcke. Das Testbudget liegt
jetzt bei 5 MiB, die produktive Grenze bleibt unverändert bei 8 MiB.
Die lokale Fuelpreis-Zwischenspeicherung gilt nur innerhalb eines Fortschritts;
Saison und laufende Abflugangebote bleiben unabhängig von Schrittgrößen.

## Zwei Kabinenklassen (Save-Version 17)

`config/cabins.json` regelt Fläche, Umbaugebühr, Zahlungsbereitschaft,
Business-Präferenz und KI-Schwellen. Ein Business-Sitz belegt zunächst zwei
Economy-Flächeneinheiten: `Economy = Modellsitze − Business × Verhältnis`.
Der Umbau kostet einmalig 500 EUR (Flugzeugjournal), ohne zusätzliche Bodenzeit;
er ist nur geparkt oder während der Bodenzeit ohne beauftragte/aktive Wartung
möglich. Unveränderte Layouts kosten nichts. Flottenansicht: Business-Sitze
bearbeiten, Economy und Obergrenze kommen aus Rust.

Jede Route hat einen Economy- und einen Business-Preis, auch über alle ihre
Flugzeugrotationen hinweg. Beide Kabinen sind Angebote im bisherigen gemeinsamen
Segmentmarkt und derselben Sitzüberhangverteilung. Nur Geschäftsreisende bekommen
ein Business-Angebot: doppelte Zahlungsbereitschaft und dreifache Attraktivität
(jeweils konfigurierbar). Freizeitpassagiere bleiben in Economy. Ist Business voll,
wechselt kaufbereiter Überhang auch in freie Economy-Sitze; ist Business zu teuer,
bleibt das Economy-Angebot verfügbar. Es entsteht kein zweiter Tagesmarkt.
Die Frequenz der Alternative „nicht fliegen“ wird pro Flugzeug nur einmal gezählt.
Nach der gemeinsamen Verteilung wird je Kabine auf ganze Passagiere abgerundet.

Abflug-Snapshots und Archive behalten Passagiere, Sitze und Erlöse für Business;
Economy ergibt sich aus Gesamtsumme minus Business. Preisänderungen und spätere
Umbauten schreiben keine alten Ergebnisse um. Routenergebnisse zeigen Y/J,
Einnahmen-Tooltips und Routencontrolling die getrennten Erlöse/Passagiere.
Bestehende Spielstände migrieren für Spieler und KI auf null Business-Sitze,
mit unveränderter Economy-Kapazität, Guthaben und alten Abflug-Snapshots.
KI-Prüfungen versuchen bei mindestens 40 % Geschäftsreiseanteil eine kleine
Business-Kabine (20 % der Fläche) am Boden über dieselbe kostenpflichtige API;
bei Geldmangel/Wartung/Flug wird der Umbau übersprungen. Bestehende Preis- und
Wirtschaftlichkeitsprüfungen nutzen die tatsächliche Kabinenkapazität; die KI
setzt den Business-Preis auf den konfigurierten Aufschlag zum Economy-Preis.

Nullwerte der Business-Summen werden im Archiv ausgelassen; das begrenzte
Save-/Ladelimit beträgt wegen zusätzlicher Klassendetails jetzt 128 MiB.
