# Weltweite KI: AIR-174

## Regeln und Kompatibilität

Es bleiben zwei Firmen mit maximal sechs Flugzeugen. Regional Wings startet am
konfigurierten Hub mit der kleinsten Großkreisentfernung zur Spielerbasis;
Continental Sky am entferntesten Eintrag aus `remote_hubs`. Gleiche Entfernungen
werden nach ICAO sortiert. `config/competition.json` enthält die Starter-Hubs
Frankfurt, São Paulo, Tokio, New York, Sydney und Johannesburg samt erster
Strecke, Modell und Tarif. Der Fernpool beschränkt sich zunächst auf Frankfurt,
São Paulo und Tokio: dort bietet der vorhandene Katalog genügend verschiedene
Märkte für weitere Expansion. Ein Sydney-Fernstart blieb im ersten Versuch
solvent, aber bei einer Maschine und wurde deshalb nicht in den Fernpool aufgenommen.

Die Basis ist ein regionaler Starter-Hub, nicht zwingend genau der vom Spieler
gewählte Flughafen. Das ist bewusst die kleine Version: keine zusätzlichen
KI-Airlines, keine zweite KI-Basis, keine neuen Save-Felder. Mehr Präsenz in dünn
besetzten Regionen wäre eine spätere Produktentscheidung; sie wird hier nicht
über zusätzliche Firmen oder künstliche Gewinne erzwungen.

Alle Firmen kaufen Flugzeuge, bezahlen Besatzung und Versicherung und legen
Flugpläne über die normalen Rust-Spielerfunktionen an. Für Expansion bleiben
Budget, Reserve, Finanzierungsregeln, Slots, Reichweite und Wirtschaftlichkeit
unverändert. Bestehende Spielstände übernehmen die gespeicherten Firmen
vollständig. Auch die historische Migration aus Versionen ohne KI behält ihren
Frankfurt-Start. Version 30 bleibt gültig; die neue Auswahl läuft nur bei Gründung.

## Roter Vertragstest: Ursache und Ersatz

Auf `77e8eb7` reproduziert: Continental Sky hat im ersten Jahr 24.979.183 EUR
mit und 32.276.657 EUR ohne Verträge. Die tatsächlich gebuchte Treibstoffwirkung
beträgt **−143.234 EUR**, also eine Ersparnis, keinen Verlust von 7,3 Mio. EUR.
Die erste unterschiedliche Expansion kommt an Tag 56: Amsterdam mit Vertrag,
Zürich ohne. Die Kassen nach dieser Entscheidung betragen 4.207.167 gegenüber
4.157.488 EUR; bis dahin beträgt die Vertragswirkung −49.812 EUR. Die kleinen
Liquiditätsunterschiede verändern später Kauf-/Leasingalternativen, Maschinen
und Routen. Nach einem Jahr fliegt der Kontrolllauf u. a. Madrid mit einem
zusätzlichen A320, der Vertragslauf München mit einer ATR.

Die Vertragspolitik wird nicht geändert. Die zehnjährige Probe prüft weiterhin
beide unabhängig expandierenden Welten täglich auf Zahlungsnot und lädt jährlich
neu. Die unveränderte 5-%-Grenze bezieht sich jetzt auf die tatsächliche
Treibstoffwirkung im Verhältnis zu den Erlösen derselben Firma. Monatliche
Gegenproben kopieren einen aktiven Vertrag und betreiben dieselbe einzelne Firma
mit und ohne Vertrag für einen Tag ohne KI-Expansion. Flüge, Passagiere und
Erlöse müssen gleich sein; Fuel- und Kassendifferenz müssen exakt dem verbuchten
Effekt entsprechen. Mindestens eine Gegenprobe muss einen Effekt ungleich null
haben. Rundung, Vertragsende und eingefrorene Abflugkosten bleiben separat geprüft.
Ein zusätzlicher Test reproduziert die alte Frankfurt-Verzweigung weiterhin mit
allen 40 Flughäfen; kein Rückzug auf den Acht-Flughafen-Katalog.

## Suchkosten und Determinismus

Die Expansion berechnet identische Modell-/Standort-/Kabinenangebote einmal je
Review und verwendet sie für die verschiedenen Gebraucht-/Neukaufoptionen.
Anschaffungspreis, Restwert, Wartungsrückstand und Miete bleiben separat bewertet.
Die Reihenfolge der Optionen und der strikte Scorevergleich bleiben erhalten.
Unerreichbare Ziele werden vor dem Aufbau der Kandidaten verworfen. Angebotslisten
werden für jeden Markt auf dessen tatsächlich relevante Einträge reduziert.
Der bestehende Nachfrage-Allocator berechnet Geografie und Referenztarif einmal
pro vorbereitetem Markt statt nochmals für jedes Angebot/jede Kabine. Keine
persistente Cache-Schicht, kein zusätzlicher Zustand und kein neuer Zufallsstrom.

Ein Regressionstest behält die ursprünglichen Frankfurt-Firmen und prüft nach
zwei Jahren die exakten bisherigen Kassen und sechs Zielrouten. Der bestehende
Allocator-Referenzvergleich prüft die Rechenergebnisse einschließlich Saisons.
Der Zweijahrestest prüft für Frankfurt, São Paulo und Tokio wöchentliche gegen
tägliche Schritte, 1×/4×/16×, Pause und Neustart nach einem Jahr mit vollständiger
Zustandsgleichheit. Für alle 40 Gründungsflughäfen werden gültige Starts und die
nächste regionale Basis geprüft; eine Version-30-Rundreise erhält den vollständigen
alten Frankfurt-Zustand exakt.

## Zweijahresmessung

Reproduzierbarer Vergleich auf diesem Server, Debug-Build, 104 Wochen plus zwei
Tage. Jeweils regulär finanzierte Spielergründung mit 10 Mio. EUR und zunächst
leerer Spielerflotte, damit dieselbe Ausgangslage in allen drei Regionen gilt.
Diese regionale Tabelle ist deshalb getrennt von der aktiven FRA–MUC-Spielerroute
in PR #59. Zeitwerte messen den gesamten `advance`, nicht nur `review`, und sind
auf dem geteilten Server lastabhängig. Die früheren 834 ms aus PR #59 wurden in
dieser Sitzung nicht reproduziert: aktuelles Main lag bei 130–212 ms.

| Stand | Spielerbasis | Advance (730 Tage) | Langsamste Woche |
| --- | --- | ---: | ---: |
| Main 77e8eb7 | EDDF | 2.689 ms | 138 ms |
| Main 77e8eb7 | SBGR | 2.781 ms | 212 ms |
| Main 77e8eb7 | RJTT | 2.493 ms | 130 ms |
| Nachher | EDDF | 1.053 ms | 76 ms |
| Nachher | SBGR | 1.301 ms | 79 ms |
| Nachher | RJTT | 1.170 ms | 84 ms |

Der opt-in Wochen-Budgetlauf mit diesen finalen Startpreisen ist bestanden.
Die anfänglich gewählten Jet-Startpreise waren zu hoch: bei 120 ct/kg Kraftstoff
schloss die Brasilien-Firma ihre Strecke. Eine Fare-/Zielprobe mit den gemeinsamen
Quote-Funktionen ergab 128 EUR für SBGR–SAEZ und 172 EUR für RJTT–ZBAA. Mit diesen
konfigurierten Preisen decken alle sechs Starter einschließlich voller Besatzung
und Wartungsanteil ihre jährlichen Betriebskosten auch am maximalen Kraftstoffpreis.
Der bestehende fünfjährige Hochpreis-Test besteht mit unveränderten Aktivitäts-,
Solvenz- und Neustartassertions; ein neuer Guard prüft die Startangebote direkt.

| Stand | Spielerbasis | Firma | Kasse EUR | Flotte | Finale Routen |
| --- | --- | --- | ---: | ---: | --- |
| Vorher | EDDF | Regional Wings | 59.131.209 | 6 | EDDF–EDDM/EGLL/LFPG/LIRF/EGCC/EHAM |
| Vorher | EDDF | Continental Sky | 67.641.347 | 6 | EDDF–LTFM/EGCC/EHAM/EDDM/LIRF/EGLL |
| Vorher | SBGR | Regional Wings | 59.131.209 | 6 | EDDF–EDDM/EGLL/LFPG/LIRF/EGCC/EHAM |
| Vorher | SBGR | Continental Sky | 67.641.347 | 6 | EDDF–LTFM/EGCC/EHAM/EDDM/LIRF/EGLL |
| Vorher | RJTT | Regional Wings | 59.131.209 | 6 | EDDF–EDDM/EGLL/LFPG/LIRF/EGCC/EHAM |
| Vorher | RJTT | Continental Sky | 67.641.347 | 6 | EDDF–LTFM/EGCC/EHAM/EDDM/LIRF/EGLL |
| Nachher | EDDF | Regional Wings | 65.250.484 | 6 | EDDF–EDDM/LTFM/EGLL/LIRF/EGCC/LFPG |
| Nachher | EDDF | Continental Sky | 24.215.164 | 2 | SBGR–SAEZ/SCEL |
| Nachher | SBGR | Regional Wings | 24.206.395 | 2 | SBGR–SAEZ/SCEL |
| Nachher | SBGR | Continental Sky | 68.858.000 | 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| Nachher | RJTT | Regional Wings | 64.206.219 | 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| Nachher | RJTT | Continental Sky | 24.215.164 | 2 | SBGR–SAEZ/SCEL |

Beide Firmen starten, expandieren und bleiben an allen Wochenproben solvent und
außerhalb der Restrukturierung. Der Flottendeckel bleibt sechs, neue Ausgaben
werden ausschließlich aus normalen Erlösen/Finanzierungen getragen. Das sind
Szenarioresultate, keine Gewinn- oder Wettbewerbsbalance-Garantie für jede
Spielerstrategie.

Die opt-in Zeitprüfung läuft ohne parallele Builds/Tests auf einem ruhigen Server:

```sh
AIRLINE_ENFORCE_AI_BUDGET=1 cargo test --locked --manifest-path rust/Cargo.toml worldwide_ai_two_year_budget -- --ignored --test-threads=1 --nocapture
```

Sie verwendet den vollständigen Weltkatalog und begrenzt die langsamste Woche auf
150 ms und den gesamten Zweijahres-Advance auf zwei Sekunden je Region.
Normale Tests enthalten keine empfindliche Wanduhr-Assertion.

## Sichtprüfung

`tools/godot_check.sh shots /paperclip/shots/AIR-174` hat 202 PNGs erzeugt.
Geöffnet wurden `overview`, `fleet`, `routes`, `map-competitors` und
`map-competitor` jeweils bei 1440×1000 und 680×520 sowie die drei schmalen
`overview-content`, `fleet-content` und `routes-content`-Ansichten. Die 13
angesehenen PNGs sind zusätzlich als ZIP an AIR-174 hinterlegt.

Die gestrichelte Brasilien-Verbindung erscheint auf der Weltkarte. Die finale
SBGR–SAEZ-Karte zeigt Continental Sky, A320, Frequenz und 128/200-EUR-Tarife
innerhalb der Kartengrenzen und des Fensters. Titel und Werte passen auch bei
680×520. Marker-/Kartenstil wurde mit `MapMarkers` und `AirportCard` im UI-Kit
verglichen. Die Weltansicht hat weiterhin dicht beieinander liegende europäische
Marker; im Europa-Zoom sind die Codes lesbar. Keine neue Überlappung oder
abgeschnittene rivalenbezogene Anzeige gefunden. Die schmalen Flotten- und
Routentabellen verwenden weiterhin Ellipsen und horizontales Scrollen, die
Übersichtswerte bleiben innerhalb ihrer Karten. Kein Produktions-UI-Code geändert.
Die Capture-Probe wählt den tatsächlichen Fern-Konkurrenten und prüft seinen
Namen, damit eine leere Auswahlkarte nicht unbemerkt als Sichtprüfung durchgeht.

## Servergrenzen

START.bat, die PowerShell-Neustarttreiber und echtes GPU-Rendering sind auf diesem
Linux-Server nicht geprüft. Godot verwendet für Bilder Software-OpenGL; Windows
kann leicht anders rendern.

## Finale Rust-Verifikation

Vollständiger Suite-Lauf seriell (`--test-threads=1`), Tests vorher kompiliert,
keine parallelen Builds: **252 bestanden, 0 fehlgeschlagen, 1 ignoriert** in
768,44 Sekunden; Doc-Tests ebenfalls grün. `cargo clippy --locked` bestanden.
`AIRLINE_ENFORCE_PERF_BUDGET=1` war für den gesamten Lauf gesetzt: der
Millionenflug-Advance brauchte **206,31 Sekunden** bei 300 Sekunden Grenze,
inklusive Save/Load und Fortsetzung 325,76 Sekunden. Auch der kompakte
Millionenflug-Archivtest bestand (55.157 Batches, 16.065.459 Bytes).

Der separat aktivierte regionale Wochenbudgettest bestand erneut: Frankfurt
1.223 ms / 63 ms, São Paulo 1.119 ms / 52 ms und Tokio 814 ms / 24 ms
(gesamter Advance / langsamste Woche). Die unterschiedlichen Zeitmessungen
auf dem geteilten Server ändern weder Firmenzustände noch finale Routen.
