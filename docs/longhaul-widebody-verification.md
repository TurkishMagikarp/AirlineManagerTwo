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

| Spielerbasis | Firma | Vorher Kasse EUR | Vorher Flotte | Vorher Routen |
| --- | --- | ---: | ---: | --- |
| EDDF | Regional Wings | 65.250.484 | 6 | EDDF–EDDM/LTFM/EGLL/LIRF/EGCC/LFPG |
| EDDF | Continental Sky | 24.215.164 | 2 | SBGR–SAEZ/SCEL |
| SBGR | Regional Wings | 24.206.395 | 2 | SBGR–SAEZ/SCEL |
| SBGR | Continental Sky | 68.858.000 | 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| RJTT | Regional Wings | 64.206.219 | 5 | RJTT–ZBAA/VHHH/VIDP/WSSS/VTBS |
| RJTT | Continental Sky | 24.215.164 | 2 | SBGR–SAEZ/SCEL |

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
