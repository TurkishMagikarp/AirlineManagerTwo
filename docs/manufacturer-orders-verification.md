# Herstellerbestellungen: Vergleich und Prüfung

Alle Werte sind vorläufig. Vergleichsbasis: `origin/main` bei `6e9f30d`.
Standard-Neugründung in EDDF, Spieler ohne Flugzeuge/Bestellungen, tägliche
Schritte bis Tag 3650. Identischer Test
`manufacturer_orders_eddf_rival_comparison_two_and_ten_years` auf Basis und Branch.

| Tag | Airline | Flotte main / Branch | Guthaben main / Branch (EUR) | Zahlungsfähig |
|---|---|---|---|---|
| 730 | Regional Wings | 6 / 6 | 116.187.129 / 116.187.129 | beide ja |
| 730 | Continental Sky | 5 / 5 | 42.705.584 / 42.705.584 | beide ja |
| 3650 | Regional Wings | 6 / 6 | 599.495.783 / 599.495.783 | beide ja |
| 3650 | Continental Sky | 6 / 6 | 403.738.665 / 403.738.665 | beide ja |

Auch keine Finanznot an den Vergleichsterminen. Der erste Ansatz erlaubte
Bestellungen der dritten **und** sechsten Maschine. Damit blieben die Flotten
identisch, aber das Zehnjahresguthaben sank auf 567.472.797 / 353.362.040 EUR
(−5,3 % / −12,5 %). Deshalb begrenzt `ai_max_fleet: 3` diese erste Scheibe auf
die dritte Maschine. Nur eine ohnehin voll finanzierbare, fabrikneue
Kaufentscheidung wird ersetzt; Leasing, Gebrauchtkauf und Betriebsreserve
bleiben erhalten. Im EDDF-Vergleich wird dieser konservative Pfad nicht gewählt;
der gezielte KI-Test erzwingt ihn und prüft Anzahlung und keine Doppelbestellung.

Die gezielten Rust-Tests prüfen Staffelpreise mit tatsächlichen Katalogpreisen
und ungeraden Testpreisen, exakte Investitionsbuchungen, gestaffelte Lieferungen,
volle Slots, Zahlungsfrist mit Erholung und Verfall, manuelle Stornierung,
Restrukturierung beider Eigentümer, Pause/1×/4×/16×, große/stündliche Schritte,
Zeitsprünge auch ab Mittag und Save/Load vor Lieferung, im Zahlungsrückstand und
nach Stornierung. Alte Spielstände migrieren ohne Bestellungen; beschädigte
Positionen werden abgewiesen. Der Godot-Test bedient das echte Flottenpanel und
die Bestell-/Stornodialoge, einschließlich Aufmerksamkeit → richtige Position.

Die Flottenbilder liegen unter `/paperclip/shots/AIR-235`: Vorschau, offene
Positionen, unbezahlte Position und beide Bestätigungsdialoge jeweils in
1440×1000 und 680×520. Geprüft wurden Preise/Termine, Tabellen, Chips, Frist,
Schaltflächen und Dialoggrenzen. Die kleine Ansicht scrollt; Modellnamen dürfen
mit vollständigem Tooltip gekürzt werden. Godot nutzt hier Software-OpenGL.
Windows-`START.bat`, PowerShell-Neustarttests und echte GPU-Darstellung wurden
auf diesem Linux-Server nicht geprüft.

## Abschließende Prüfungen

- `cargo test --locked --manifest-path rust/Cargo.toml -- --test-threads=4 --skip million_flight`: 344 bestanden, sieben bestehende Tests ignoriert; die beiden Stressfälle werden separat mit erzwungenem Budget geprüft.
- `cargo clippy --locked --manifest-path rust/Cargo.toml`: bestanden, keine Warnungen.
- `tools/godot_check.sh tests`: 51 bestanden, null Fehler; ein Windows-Neustarttreiber übersprungen.
- Die bisherigen Rust-/Godot-Migrationsfixtures wurden auf die neue aktuelle Version 39 umgestellt; historische Zielversionen bleiben erhalten.
- `AIRLINE_ENFORCE_PERF_BUDGET=1 cargo test --locked --manifest-path rust/Cargo.toml million_flight -- --test-threads=1 --nocapture`: beide Tests bestanden. Gemessener Finance-Advance **283,961326566 s** bei 300 s Budget; 5.779.250 Flüge einschließlich KI, Transferzähler `(37450, 106623, 106623)`. Der frühere veröffentlichte Wert war 259,468 s.

### Untersuchung des Zeitunterschieds

Ein direkt anschließender, allein laufender Test auf main bestand ebenfalls:
Advance **273,252597309 s**, einschließlich Save/Load/Fortsetzung 397,934811810 s.
Der Branch benötigte dafür 283,961326566 s beziehungsweise 413,527532312 s.
Beide hatten exakt 5.779.250 Flüge und dieselben Transferzähler. Die Save-Datei
wuchs nur um die 90 Bytes der drei leeren Bestellbücher/Kennungsfelder.

Die Advance-Differenz beträgt +3,919 %; die unabhängige Phase danach
(Speichern/Laden/Fortsetzen) skaliert mit praktisch demselben Faktor (+3,917 %).
Während des Branch-Advance zeigte der Prozess ungefähr 94–95 % eines CPU-Kerns,
beim main-Vergleich ungefähr 98 %. Zusammen mit dem ebenfalls über dem alten
259,468-s-Wert liegenden main-Lauf spricht dies für CPU-Verfügbarkeit bzw.
Laufzeitvariation als Ursache des Wallclock-Unterschieds, nicht für zusätzliche
Flüge oder Marktallokationen. Das ist eine Schlussfolgerung aus den Messungen,
keine isolierte Profiler-Messung. Die neue Regel bleibt außerhalb der
Flugprojektion; beide erzwungenen Branch-Stressfälle und der main-Vergleich
sind grün. Das unveränderte 300-s-Limit bleibt aktiv.
