# Umsteiger: Vergleich und offene Verifikation (AIR-197)

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

PR #66 bleibt Draft und ungemergt. Die vollständige Rust-Suite, Clippy, die sechs
Zweijahres-KI-Zeilen, der Zehnjahres-Reputationsvergleich, die vollständige
Godot-Suite samt Finanz-Stresstest und ein Branch-Lauf ohne Budget-Erzwingung
wurden in diesem Lauf nicht ausgeführt. Frühere Ergebnisse sind keine neue
Verifikation nach der Umstellung auf direkte KI-Planungsangebote. Die
Branch-Save/Reload-Assertions hinter der Budget-Assertion bleiben offen.

Keine UI-Änderungen und keine neuen Screenshot-Prüfungen in diesem Lauf.
Vorherige Bildprüfung bleibt im PR dokumentiert. START.bat, PowerShell-
Restarttreiber und echte GPU-Darstellung bleiben ungeprüft.
