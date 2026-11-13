# Bordservice pro Route

Im Routenpanel gilt Basic / Standard / Premium für alle Flugzeuge einer Route.
Änderungen wirken ab dem nächsten Abflug. Passagiere, Erlöse und Kosten bereits
abgeflogener Flüge bleiben als Abflugergebnis gespeichert, auch nach Laden.

`config/service.json` enthält die zusätzlichen Kosten je befördertem Passagier
(Economy / Business) und den Attraktivitätsfaktor im gemeinsamen Markt:

| Service | Economy EUR | Business EUR | Attraktivität |
| --- | ---: | ---: | ---: |
| Basic | -1 | -4 | 0,94 |
| Standard | 0 | 0 | 1,00 |
| Premium | 3 | 14 | 1,08 |

Die Kosten sind Differenzen gegenüber den bestehenden Standard-Grundkosten.
Basic bucht eine Ersparnis als negative Servicekosten; Standard verändert weder
Kosten noch Nachfrage gegenüber Version 17. Die Formel lautet
`Economy-Passagiere × Economy-Satz + Business-Passagiere × Business-Satz`.
Leere Sitze verursachen keine Servicekosten. Der Faktor multipliziert das Gewicht
im bestehenden Nachfrage-/Sitzüberlauf-Verteiler, ohne Preisgrenzen oder Kapazität
zu erhöhen. Ein Faktor von 1,08 bedeutet nicht acht Prozent mehr Passagiere.

Routen zeigen die Klassensätze, die aktuell geschätzten Servicekosten je Abflug
(Durchschnitt über aktive Flugzeuge) und die bereits gebuchten Servicekosten.
Finanzen enthält eine eigene Servicekostenzeile sowie die Aufschlüsselung in den
Flug-Tooltips. Die Gesamtkosten und das Flugarchiv enthalten diese Beträge bereits.

KI-Airlines verwenden dieselben Kosten und denselben Verteiler. Bei der
wöchentlichen Prüfung wählen sie Premium, wenn eine Business-Kabine vorhanden ist
und der Business-Aufpreis mindestens den konfigurierten Grenzwert erreicht;
sonst Standard. Version 18 migriert ältere Routen zu Standard und setzt den neuen
Kostenposten in bisherigen Summen, Archiv und laufenden Flügen auf null.

Rust-Prüfungen: `cargo test --locked --manifest-path rust/Cargo.toml service_`.
Godot-Prüfung: `Godot --headless --path . --script res://tests/service_test.gd`.
Ein erfolgreicher Headless-Test ersetzt keine visuelle Prüfung des Routenpanels.

## Zehnjahresvergleich (AIR-30)

Verglichen mit `a5e1a19` (vor Service) bleiben beide KI-Airlines in allen vier
bestehenden Audit-Szenarien solvent, ohne Restrukturierungsfrist. Jahresweise
Speichern/Laden und tägliche gegenüber jährlicher Schrittweite bleiben identisch.
Zusätzlich prüft der Audit, dass Servicekosten unter 20 % der KI-Ticketumsätze
bleiben. Die Beträge unten sind kumulierte Flugergebnisse ohne zentrale Gehälter,
Leasing, Zinsen und Wartungschecks; die Solvenzprüfungen berücksichtigen diese.

| Spielerpreis | KI | Fluggewinn vorher EUR | Mit Service EUR | Änderung |
| --- | --- | ---: | ---: | ---: |
| Keine Route | Regional Wings | 357.696.127 | 351.604.215 | -1,7 % |
| Keine Route | Continental Sky | 290.664.002 | 299.898.386 | +3,2 % |
| 0 | Regional Wings | 19.592.113 | 19.667.802 | +0,4 % |
| 0 | Continental Sky | 21.531.795 | 18.292.451 | -15,0 % |
| 90 | Regional Wings | 306.394.920 | 310.419.524 | +1,3 % |
| 90 | Continental Sky | 339.537.236 | 355.647.831 | +4,7 % |
| 100.000 | Regional Wings | 38.033.379 | 39.080.953 | +2,8 % |
| 100.000 | Continental Sky | 21.038.150 | 18.611.911 | -11,5 % |

Dies sind die Endstände des vorhandenen Audits. Bei terminaler Spielerinsolvenz
endet der Simulationsfortschritt früher; spätere nominelle Auditjahre verändern
diese Szenarien nicht mehr. Im normalen 90-EUR-Szenario erreichen die KI-Airlines
nach zehn Jahren 240,8 / 278,5 Mio. EUR Guthaben (vorher 235,2 / 253,3 Mio.).
Servicekosten, andere Marktanteile und dadurch andere Investitionsentscheidungen
ändern die Ergebnisse; gleiche Flotten oder Gewinne werden nicht vorausgesetzt.

Auf diesem Server sind Godot, START.bat, visuelle GUI-/Screenshot-Prüfungen und
Windows-Neustarttests nicht ausführbar. Der neue Godot-Selektortest ist vorbereitet,
aber hier nicht ausgeführt. Eine visuelle Abnahme bleibt offen.
