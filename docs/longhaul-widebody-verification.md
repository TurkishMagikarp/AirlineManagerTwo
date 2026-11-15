# Widebodies: AIR-179

## Regeln und Kompatibilität

A330-300 und 777-300ER ergänzen den Katalog als `Widebody`. Die bisherigen
Modellwerte bleiben erhalten. Quellen und explizite Spielannahmen stehen in
`data/README.md`; Kaufpreise, Betriebskosten, Wartung und Personal in `config/`.
Handel, Leasing, Versicherung, Kabinen, Fracht und Flughafengebühren wenden ihre
bestehenden gemeinsamen Regeln an. Kein neues Save-Feld, keine Migration und
keine Änderung an Version 30. Die bisherigen Migrationsproben bleiben aktiv.

KI-Suche, zwei Firmen, Flottendeckel sechs, Basislisten, Budget, Reserve,
Finanzierung, Slots und Nachfrage bleiben unverändert. Die zusätzlichen Modelle
laufen durch dieselbe Modellschleife in `Competitor::expand`.

## Nachfrage- und Tarifprobe

Reproduzierbar mit `widebody_longhaul_fare_audit`: alleiniger Anbieter, Abflugabstand
24 Stunden (abwechselnd Hin-/Rückflug), Jahresmittel-Nachfrage, Kraftstoff 80 ct/kg,
Economy-Tarife 100–1.000 EUR in 10-EUR-Schritten, Business zum doppelten Tarif,
Kabinen in 10-Sitz-Schritten. Die Tabelle zeigt das größte Flugbeitragsergebnis
dieser diskreten Probe, keine garantierten Jahresrenditen. Löhne, Wartungschecks,
Leasing/Kapital und Versicherungsprämien werden zusätzlich außerhalb der
Flugquote gebucht. Fracht ist in den Erlösen enthalten.

| Modell / Route | Economy / Business EUR | Business-Sitze | Passagiere (Business) | Erlös EUR | Flugkosten EUR | Beitrag EUR | Fracht kg |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| A330 / EDDF–KJFK | 410 / 820 | 100 | 200 (100) | 123.294 | 72.946 | 50.348 | 196 |
| A330 / EDDF–RJTT | 530 / 1.060 | 100 | 196 (99) | 156.570 | 104.909 | 51.661 | 147 |
| A330 / SBGR–EDDF | 490 / 980 | 100 | 191 (93) | 139.343 | 107.534 | 31.809 | 122 |
| 777 / RJTT–KLAX | 410 / 820 | 120 | 263 (119) | 156.861 | 128.331 | 28.530 | 161 |

Eine kleinere 30-J-Kabine ist keine automatische Langstrecken-Gewinngarantie:
A330 EDDF–KJFK bei 300/600 EUR ergibt +13.548 EUR je Flug; EDDF–RJTT bei
500/1.000 EUR −3.812 EUR, SBGR–EDDF −14.758 EUR, 777 RJTT–KLAX bei
400/800 EUR −26.584 EUR. Abstände von 36/48 Stunden sammeln keine zusätzliche
Tagesnachfrage an. Diese Gegensätze und positive Angebote bei anderen Kabinen
geben keinen Anlass, die Nachfrage-/Tarifregeln für diesen Schritt zu verändern.
Die KI-Angebote müssen weiterhin ihre eigenen Rentabilitätsprüfungen bestehen.

## Neue Prüfungen

- Kauf, Leasingkaution/-rate, Verkauf/Restwert, Gebrauchtkauf und Typenratings.
- EDDF–RJTT mit beiden Typen: 10–14 Stunden Flugzeit, volle Bodenzeit, echte
  deterministisch gewählte technische Verspätung; Wartungsauftrag im Flug,
  eingefrorene Quote, Erlös/Kosten/Fracht genau bei Landung, Check und Neustart.
- EDDF–YSSY wird für beide Typen transaktional mit der bisherigen
  Reichweitenmeldung abgelehnt.
- Zwei Jahre mit A330-Langstrecke und beiden KI-Firmen: Wochen gegen Tage,
  1×/4×/16×, Pause, Neustart nach einem Jahr, kompletter Zustand identisch.
- Zwei 777 im versetzten Tokio–Los-Angeles-Umlauf; echte Flugposition über dem
  Pazifik und unveränderte Position nach Neustart.
- Godot-Layoutprobe erweitert: aus normalen 10 Mio. EUR Gründungskapital beide
  Typen leasen, Personal einstellen, Kabinen konfigurieren und EDDF–KJFK/RJTT
  sowie RJTT–KLAX/KJFK betreiben. Keine Test-Sonderfinanzierung in dieser UI-Probe.

## Zweijahresvergleich der KI

Gleiche Gründung wie `docs/worldwide-ai-verification.md`: leere Spielerflotte,
10 Mio. EUR, voller 40-Flughafen-Katalog, 104 Wochen plus zwei Tage. Jede Woche
werden Zahlungsnot, Restrukturierung und Flottendeckel geprüft. Vorher ist
`origin/main` bei `ed0a488`, auf diesem Server erneut gemessen.

| Spielerbasis | Firma | Vorher Kasse EUR | Nachher Kasse EUR | Flotte vorher → nachher | Finale Routen (beide Stände) |
| --- | --- | ---: | ---: | ---: | --- |
| EDDF | Regional Wings | 65.250.484 | 65.250.484 | 6 → 6 | EDDF–EDDM/LTFM/EGLL/LIRF/EGCC/LFPG |
| EDDF | Continental Sky | 24.215.164 | 24.215.164 | 2 → 2 | SBGR–SAEZ/SCEL |
| SBGR | Regional Wings | 24.206.395 | 24.206.395 | 2 → 2 | SBGR–SAEZ/SCEL |
| SBGR | Continental Sky | 68.858.000 | 68.858.000 | 5 → 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| RJTT | Regional Wings | 64.206.219 | 64.206.219 | 5 → 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| RJTT | Continental Sky | 24.215.164 | 24.215.164 | 2 → 2 | SBGR–SAEZ/SCEL |

Im real betriebenen EDDF–RJTT-Zweijahrestest (60 J, 450/900 EUR, enger zulässiger
Abflugabstand, normale Wartung/Versicherung/Personal) steigt die Spielerkasse nach
der Anschaffung von 85.999.000 auf 95.593.054 EUR. 1.344 Flüge buchen 167.249.280 EUR
Erlös, 140.621.129 EUR Flugkosten und 197.568 kg Fracht. Die Kassendifferenz von
9.594.054 EUR berücksichtigt zusätzlich die außerhalb der Flugquote verbuchten
Posten. Das Startkapital ist eine Rust-Testausstattung für den Kauf, kein
zusätzliches Geld in Spielergründung oder KI; die Godot-Leasingprobe nutzt normales Kapital.

Die opt-in Diagnose `widebody_ai_expansion_quote_audit` betrachtet am Ende der
zwei Jahre die unveränderten Suchangebote über 6.200 km. Neue Kandidaten werden
wie bisher zunächst als Economy-Angebot und mit dem kleinsten zulässigen
Abflugabstand bewertet. Die obige optimierte Business-Kabine ist deshalb keine
nachträglich unterstellte KI-Kaufentscheidung. Die Diagnose bewertet auch bei
voller Flotte hypothetische Angebote; sie führt keine Expansion aus.

| KI-Basis | Modell | Bestes Ziel >6.200 km | Economy EUR | Abstand Minuten | Erwarteter Wochenbeitrag EUR |
| --- | --- | --- | ---: | ---: | ---: |
| EDDF | A330 | RJTT | 391 | 735 | −551.301 |
| EDDF | 777 | RJTT | 391 | 741 | −967.894 |
| SBGR | A330 | EGLL | 394 | 742 | −619.122 |
| SBGR | 777 | KORD | 357 | 679 | −996.884 |
| RJTT | A330 | OMDB | 339 | 637 | −487.925 |
| RJTT | 777 | OMDB | 339 | 647 | −887.280 |

Diese Suchbeiträge enthalten Flugkosten, Wartungsanteil und zusätzliches Personal,
noch keine Leasingrate; sie liegen bereits deutlich unter der unveränderten
5.000-EUR-Wochenschwelle. Frankfurt ist zusätzlich beim Flottendeckel sechs.
Es wurden bewusst keine zweite Basis, weitere Firma, garantierte Präsenz oder
geänderte Nachfrage-/Kabinen-Suchstrategie hinzugefügt.


## Sichtprüfung

`tools/godot_check.sh shots /paperclip/shots/AIR-179`: 237 PNGs, finaler Lauf ohne
Scriptfehler. Geöffnet wurden bei **1440×1000 und 680×520**:

- `widebody-EDDF-fleet`: beide geleasten Typen mit StatusChip im DataTable;
- `aircraft-3-header` und `aircraft-4-header`: bestehende AircraftCards mit den
  neuen Namen, Klasse, Sitzen, Reichweite, Geschwindigkeit und Besatzung;
- `widebody-EDDF-map`: EDDF–KJFK-Karte und EDDF–RJTT-Verbindung;
- `widebody-RJTT-map`: Tokio–Los Angeles und Tokio–New York mit Datumsgrenzen-Wrap.

Diese zehn finalen Bilder sind als `AIR-179-visual-review.zip` am Paperclip-Issue
hinterlegt. Zusätzlich wurden die Kauf-/Leasingaktionsaufnahmen beider neuer
Typen in beiden Größen und die schmalen Kaufaufnahmen geöffnet. Die
AircraftCard-/DataTable-/StatusChip-Regeln im UI-Kit wurden gelesen.

Namen, Fakten und Preise passen in die vorhandenen Karten. Bei 680×520 werden
lange Tabellenzellen wie bisher gekürzt und horizontal gescrollt; beide
Flugzeugzeilen sind sichtbar. Beschaffungsaktionen bleiben über vertikales
Scrollen erreichbar. Map-Karte, Tarif, Ankunft, Distanz, Fortschritt und
Bearbeiten-Button bleiben im Fenster; die Pazifik-Verbindung wird an den
Kartenrändern fortgesetzt. Keine neue Überlappung oder herauslaufenden Werte
gefunden. Kein Produktions-GDScript oder neues UI-Bauteil nötig.

Die erste Probe stellte beide Flugzeuge vor einer einzigen Sammel-Einstellung
bereit (66 Personen > 50 je Einstellung). Die Fixture stellt jetzt nach jeder
Beschaffung Personal ein; die Regel bleibt unverändert. Außerdem wurden die
bisherigen Drei-Modell-Assertions in Fleet/UI-Kit erweitert. Der neue asynchrone
Capture-Helfer meldet fehlgeschlagene Spielaktionen explizit an den Test zurück.
Die Capture-Scrollpositionen richten Tabellen und Kartenüberschriften gezielt
aus, damit schmale Aufnahmen die überprüften Inhalte tatsächlich zeigen.

## Servergrenzen

START.bat, PowerShell-Neustarttreiber und echte GPU wurden hier nicht geprüft.
Godot-Bilder verwenden Software-OpenGL; Windows kann leicht anders rendern.


## Finale Prüfungen

Volle serielle Rust-Suite mit `AIRLINE_ENFORCE_PERF_BUDGET=1`: **258 bestanden,
0 fehlgeschlagen, 2 opt-in Diagnosen ignoriert**, 877,71 Sekunden. Der separate
erzwungene KI-Budgettest besteht (siehe Tabelle unten). Die exakte ursprüngliche
Frankfurt-Regressionsprobe bleibt unverändert; keine abweichende Kaufentscheidung.
Der bestehende Fünfjahres-Hochkraftstofftest besteht, finale KI-Kassen:
147.604.763 / 22.187.247 EUR. `cargo clippy --locked` besteht ohne Warnungen;
`cargo build --locked` besteht ebenfalls.

`tools/godot_check.sh tests`: **40 Testgruppen bestanden, 0 fehlgeschlagen**,
Exitstatus 0. Die Windows-Neustarttreiber werden explizit übersprungen.

Der Millionenflug-Finanztest lief mit `AIRLINE_ENFORCE_PERF_BUDGET=1`:
**245,00 Sekunden** Advance bei 300 Sekunden Grenze; einschließlich Save/Load
und Fortsetzung **360,40 Sekunden**. Der gespeicherte Stress-Spielstand umfasst
196.343.019 Bytes. Kein paralleler Build/Test während dieser Messung.


## Kosten der KI-Suche

Die zusätzliche Reichweite erweitert die bestehende Modellsuche. Innerhalb einer
Tarifprobe ohne Konkurrenz und ohne zugewiesenes Flugzeug wird die vorbereitete
Economy-Quote wiederverwendet: Geografie, Kapazität und Frachtmarkt bleiben gleich,
nur Tarif und Zahlungsbereitschaft werden erneuert. Personalbedarf wird einmal
pro Modellsuche berechnet. Mehranbieter-/Business-Angebote behalten den bisherigen
Pfad. `single_economy_fare_sweep_matches_fresh_quotes` vergleicht jeden Totals-Wert
mit frisch aufgebauten Quoten für A320, A330 und 777, steigende/fallende Tarife
inklusive 0/100.000 EUR und drei Kraftstoffpreise. Keine neue Suchentscheidung.

`AIRLINE_ENFORCE_AI_BUDGET=1 worldwide_ai_two_year_budget` besteht unverändert:

| Spielerbasis | Zwei Jahre | Langsamste Woche | Grenzen |
| --- | ---: | ---: | --- |
| EDDF | 1,253 s | 64,102 ms | 2 s / 150 ms |
| SBGR | 1,137 s | 50,532 ms | 2 s / 150 ms |
| RJTT | 1,250 s | 48,897 ms | 2 s / 150 ms |

Die ersten Prüfungen deckten eine unterfinanzierte alte Kauf-Testfixture und
eine Überschreitung der Wochenzeit auf. Die Kalibrierungsfixture erhält jetzt
explizites Testkapital für jedes Modell; Produktionskapital bleibt unverändert.
Die obige Wiederverwendung reduziert den zusätzlichen Quote-Aufwand. Bei einer
Diagnose gab es weiterhin einen einzelnen 323-ms-Wallclock-Ausreißer; der finale
erzwungene Budgetlauf lief ohne parallelen Build/Test und blieb unter den Grenzen.
