# Umsteiger: Vergleich und Verifikation (AIR-197)

## CPU-Vergleich vom 15.11.2026

Die Freigabe vergleicht nach Rückmeldung jetzt die **User-CPU-Zeit des gesamten
exakten Finanz-Stresstests**, einschließlich Save/Reload/Fortsetzung. Beide
Testprogramme wurden vorab gebaut; Bash `time` umschließt nur das Testprogramm,
nicht Cargo oder die Übersetzung. `AIRLINE_ENFORCE_PERF_BUDGET` ist nicht gesetzt,
damit die unveränderte 300-s-Assertion die nachfolgenden Prüfungen nicht abbricht.
Je Paar lief zuerst main `6639c18`, danach der Branch; keine anderen
AirlineManager-Prüfungen liefen parallel. Fremde Serverprozesse wurden nicht beendet.

```bash
cargo test --locked --manifest-path rust/Cargo.toml million_flight_finance_scenario_stays_within_budget --no-run
unset AIRLINE_ENFORCE_PERF_BUDGET
TIMEFORMAT='real %R user %U sys %S'
time <von Cargo ausgegebenes Testprogramm> --exact history::tests::million_flight_finance_scenario_stays_within_budget --nocapture --test-threads=1
```

Erstes Paar, Branch `64930df` (gleicher Simulationscode wie `9cb9b22`):

| Stand | User-CPU gesamt | System-CPU | Wall gesamt | Wall Fortschritt |
| --- | ---: | ---: | ---: | ---: |
| main `6639c18` | 411,573 s | 21,426 s | 532,414 s | 330,273429 s |
| Branch vor Tagescache | 464,955 s | 21,185 s | 590,038 s | 416,633029 s |

Beide Tests einschließlich Save/Reload/Fortsetzung bestanden. Der Branch lag
**12,97 %** über main in User-CPU. Deshalb wurde die ausdrücklich erlaubte
Tagesquantisierung umgesetzt: **nur Umsteigeallokationen** nutzen die Saison am
Tagesanfang; lokale Direktnachfrage wird weiterhin sekundengenau berechnet.
Beide Transfer-Caches verwenden Tages-/Jahresschlüssel, während die lokalen
Abflugangebote frisch aus dem bisherigen Direktallokator stammen. Keine
Änderung am 300-s-Limit in `history.rs`.

Alle 15 gezielten Transfertests bestehen, einschließlich der neuen Prüfung über
365 Tage: wiederverwendete Umsteigerangebote innerhalb eines Tages, exakte lokale
Nachfrage, Tages-/Jahresgrenzen und höchstens 365 Tagesangebote pro unverändertem
Netz. Der Langzeitvergleich gespeicherter/frischer Angebote, Pause/1×/4×/16×,
Schrittgrößen, Restart und Altspeichermigration bestehen ebenfalls.

Das absolute 300-s-Budget wurde auf diesem Server verfehlt: im ersten CPU-Paar
benötigte bereits main am selben Tag 330,273429 s für den Fortschritt. Die früheren
erzwungenen Läufe und ihre Einschränkungen stehen unten.

Zweites und letztes Paar nach Tagesquantisierung:

| Stand | User-CPU gesamt | System-CPU | Wall gesamt | Wall Fortschritt |
| --- | ---: | ---: | ---: | ---: |
| main `6639c18` | 350,024 s | 17,132 s | 410,259 s | 246,653232 s |
| Branch mit Tagescache | 339,112 s | 12,725 s | 356,695 s | 244,911020 s |

**CPU-Freigabe bestanden: −3,12 %** gegenüber main, unter der akzeptierten
+10-%-Grenze. Beide vollständigen Tests bestanden samt Save/Reload/Fortsetzung.
Im zweiten Paar liegen auch beide Fortschrittszeiten unter 300 s; das hebt die
früheren absoluten Budgetüberschreitungen auf diesem Server nicht auf. Die
300-s-Assertion bleibt unverändert; in den CPU-Läufen war sie nicht erzwungen.

Bei gleichen 5.784.017 verbuchten Flügen und 28.560 Netzaufbauten sinken die
Allokationen von 577.299 auf **99.061** (−82,84 %), alle zeitbezogen, keine
hypothetischen KI-Allokationen. Pro verbuchtem Flug: **0,00494 Aufbauten /
0,01713 Allokationen**. Das zweite Paar ist der abschließende CPU-Vergleich;
keine weitere Optimierungsrunde oder Timing-Wiederholung.

## Frische Wirtschaftstests nach direkter KI-Planung und Tagescache

Die vollständige Rust-Suite besteht: **292 bestanden, 4 ignoriert, 0 Fehler**
(918,96 s); dazu `cargo build --locked` und `cargo clippy --locked`.
Die vier explizit ignorierten Diagnosetests bleiben unverändert. Der vollständige
Finanz-Stresstest lief dabei nochmals funktional durch, ohne Budget-Erzwingung.

Die sechs Zweijahres-Zeilen aus der frischen vollständigen Suite sind gegenüber
dem aufgezeichneten main `6639c18` unverändert (ganze EUR). Alle sechs Rivalen
überleben ohne Notlage; Pause, 1×/4×/16×, tägliche/wöchentliche Schritte und
Reload nach dem ersten Jahr ergeben identische Zustände.

| Spielerbasis / Rival | Cash main → Branch | Betriebsgewinn main → Branch | Gewinnänderung | Flotte main → Branch |
| --- | ---: | ---: | ---: | ---: |
| FRA / Regional Wings | 107.809.067 → 107.809.067 | 194.364.701 → 194.364.701 | 0 % | 6 → 6 |
| FRA / Continental Sky | 34.207.235 → 34.207.235 | 79.143.280 → 79.143.280 | 0 % | 5 → 5 |
| GRU / Regional Wings | 34.317.471 → 34.317.471 | 79.079.356 → 79.079.356 | 0 % | 5 → 5 |
| GRU / Continental Sky | 107.310.890 → 107.310.890 | 200.481.789 → 200.481.789 | 0 % | 6 → 6 |
| HND / Regional Wings | 106.286.287 → 106.286.287 | 199.009.463 → 199.009.463 | 0 % | 6 → 6 |
| HND / Continental Sky | 34.406.772 → 34.406.772 | 79.379.183 → 79.379.183 | 0 % | 5 → 5 |

Der frische Zehnjahres-Reputationsvergleich besteht mit maximal **0,06184 %**
Umsatzunterschied (Jahr 2, Continental Sky: **300.181.396 EUR** gegenüber
**299.995.877 EUR** bei neutraler Reputation). Alle 20 Jahres-/Airline-Zeilen
bleiben unter der unveränderten 10-%-Grenze. Der frühere Wert 2,55 % gehört zum
älteren KI-Planungsstand und ist nicht das Ergebnis dieses finalen Tests.
Transferbereitschaft bleibt 0,10; keine Retuning- oder Grenzwertänderung.

## Abschließende Godot-Prüfung

Die vollständige Suite `AIRLINE_GODOT=/paperclip/airline/godot/godot tools/godot_check.sh tests`
besteht mit **0 Fehlern** (484,481 s Wall gesamt). Der Finanz-Stresstest mit etwa
zwei Millionen Flügen einschließlich Wartungsunterbrechungen, kompaktem Archiv,
begrenzten Diagrammen/Seiten, Layout und Save/Reload/Fortsetzung besteht ohne
Timeout; deshalb war kein zusätzlicher main-Vergleich nötig. Der Windows-
Restarttreiber bleibt ausdrücklich übersprungen. Clippy wurde bei der
Wiederaufnahme nochmals erfolgreich geprüft.

Keine neuen Layoutänderungen oder Bildprüfungen in diesem Cache-Follow-up. Die
vorherige Prüfung der zehn Routen-/Umsteiger-/Ergebnis-/Wettbewerbsbilder bei
1440×1000 und 680×520 bleibt die visuelle Evidenz. START.bat, Windows-PowerShell-
Restarttreiber und echte GPU-Darstellung sind hier weiterhin nicht verifiziert.

## Früherer Wall-Zeit-Vergleich (historisch)

Am 15.11.2026 wurden `main` (`6639c18`) und der Transfer-Stand `9cb9b22`
auf demselben Linux-Server direkt nacheinander geprüft. Je Stand lief genau
`AIRLINE_ENFORCE_PERF_BUDGET=1 cargo test --locked --manifest-path rust/Cargo.toml million_flight_finance_scenario_stays_within_budget -- --exact history::tests::million_flight_finance_scenario_stays_within_budget --nocapture --test-threads=1`.
Keine anderen Repository-Prüfungen liefen parallel; keine Wiederholung.

| Stand | Simulationsfortschritt | Ergebnis |
| --- | ---: | --- |
| main `6639c18` | 295,439521 s | 300-s-Grenze bestanden; Save/Reload/Fortsetzung bestanden |
| Transfer `9cb9b22` | 382,204357 s | unveränderte 300-s-Assertion fehlgeschlagen |

Der Branch liegt **29,37 %** über dem unmittelbar vorher gemessenen main.
Damit ist die für diesen Merge akzeptierte relative Grenze von 10 % ebenfalls
verfehlt. Das absolute Budget wurde auf diesem Server vom Branch verfehlt;
main blieb am selben Tag mit 295,44 s knapp darunter. Die Assertion in
`history.rs` bleibt unverändert. main benötigte einschließlich Save/Reload und
Fortsetzung 450,267979 s (Test insgesamt 451,72 s); der Branch brach vor diesen
Assertions ab (Test insgesamt 382,71 s, Exit 101).

Der Branch verbuchte 5.784.017 Flüge einschließlich Rivalen, 28.560 Netzaufbauten
und 577.299 Allokationen, sämtlich zeitbezogen, keine hypothetischen
KI-Allokationen. Pro verbuchtem Flug: **0,00494 Aufbauten / 0,09981 Allokationen**.
Diese Zähler entsprechen dem vorherigen Lauf mit 311,970758 s.

Einschränkung: Bei etwa 362 s Branch-Laufzeit zeigte die Prozessliste einen
fremden `npm ci` mit rund 110 % CPU und etwa 20 s Laufzeit. Der Test selbst
zeigte rund 88 % CPU. Ein vollständig ruhiger Server ist daher für den gesamten
Branch-Lauf nicht belegt; fremde Prozesse wurden nicht beendet. Der Vergleich
beweist unter diesen Bedingungen keine kausale Transfer-Verlangsamung, erfüllt
aber auch nicht die verlangte Freigabe. Kein zweites Paar: main weicht von den
vorherigen heutigen main-Langzeitwerten nicht stark ab; die Anweisung verlangt
bei mehr als 10 % Branch-Aufschlag Bericht und Stopp.

Zum Zeitpunkt dieses früheren Vergleichs blieb PR #66 Draft und ungemergt. Die vollständige Rust-Suite, Clippy, die sechs
Zweijahres-KI-Zeilen, der Zehnjahres-Reputationsvergleich, die vollständige
Godot-Suite samt Finanz-Stresstest und ein Branch-Lauf ohne Budget-Erzwingung
wurden in diesem Lauf nicht ausgeführt. Frühere Ergebnisse sind keine neue
Verifikation nach der Umstellung auf direkte KI-Planungsangebote. Die
Branch-Save/Reload-Assertions hinter der Budget-Assertion bleiben offen.

Keine UI-Änderungen und keine neuen Screenshot-Prüfungen in diesem Lauf.
Vorherige Bildprüfung bleibt im PR dokumentiert. START.bat, PowerShell-
Restarttreiber und echte GPU-Darstellung bleiben ungeprüft.
