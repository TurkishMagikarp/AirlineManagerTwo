# KI-Kabinen bei Expansion: AIR-184

## Regeln

Neue KI-Kandidaten vergleichen 0/20/40 Prozent Business-Fläche aus
`config/cabins.json` über `cabins::layout` / `cabins::offer` und den normalen
Premiumtarif. Der höchste zulässige Wochenbeitrag gewinnt; bei Gleichstand
bleibt die erste Option. Vorhandene freie Maschinen behalten ihre Kabine.
Kauf, Gebrauchtkauf und Leasing installieren das Angebot über die Spieler-APIs;
die 500-EUR-Umbaugebühr wird normal gebucht und zählt bei Reserve und
104-Wochen-Amortisation als Anschaffungsausgabe. `review_cabins` behält bereits
installierte Business-Kabinen. Die normalen späteren Preis-/Serviceprüfungen
bleiben aktiv.

Keine neuen Save-Felder, keine Migration, weiterhin Version 30. Budget,
Finanzierung, Reserve, Slots, Reichweite, 5.000-EUR-Wochenschwelle, maximal sechs
Maschinen, Leasingrate und Personal bleiben erhalten. Keine Änderungen an
Nachfrage, Tarifmodell, Basislisten oder Firmenzahl. Ein lokaler vorbereiteter
Markt je Route/Modell/Kabine wird bei wechselnden Tarifen wiederverwendet;
konkurrierende Angebote werden weiter frisch berechnet. Keine persistente Cache-Schicht.

## Zwei Jahre vorher/nachher

Leere Spielerflotte, reguläre 10 Mio. EUR Gründung, voller Weltkatalog,
104 Wochen plus zwei Tage; beide Firmen jede Woche solvent, ohne Restrukturierung,
maximal sechs Maschinen. Reihenfolge von Flotte und Routen wie im Audit; Y/J
bezeichnet die tatsächlichen Economy-/Business-Sitze. ATR = ATR 72-600,
737 = 737-800, A320 = A320-200, A330 = A330-300. Keine 777 erworben.

| Stand | Spielerbasis | Firma | Kasse EUR | Modelle und Kabinen | Finale Routen |
| --- | --- | --- | ---: | --- | --- |
| Vorher (5908ac3) | EDDF | Regional Wings | 65,250,484 | ATR 58Y/7J, 737 153Y/18J, A320 144Y/18J, ATR 58Y/7J, ATR 58Y/7J, ATR 58Y/7J | EDDF–EDDM, EDDF–LTFM, EDDF–EGLL, EDDF–LIRF, EDDF–EGCC, EDDF–LFPG |
| Vorher (5908ac3) | EDDF | Continental Sky | 24,215,164 | A320 144Y/18J, 737 153Y/18J | SBGR–SAEZ, SBGR–SCEL |
| Vorher (5908ac3) | SBGR | Regional Wings | 24,206,395 | A320 144Y/18J, 737 153Y/18J | SBGR–SAEZ, SBGR–SCEL |
| Vorher (5908ac3) | SBGR | Continental Sky | 68,858,000 | A320 144Y/18J, A320 144Y/18J, A320 144Y/18J, A320 144Y/18J, A320 144Y/18J | RJTT–ZBAA, RJTT–VHHH, RJTT–VIDP, RJTT–WSSS, RJTT–VTBS |
| Vorher (5908ac3) | RJTT | Regional Wings | 64,206,219 | A320 144Y/18J, A320 144Y/18J, A320 144Y/18J, A320 144Y/18J, A320 180Y/0J | RJTT–ZBAA, RJTT–VHHH, RJTT–VIDP, RJTT–WSSS, RJTT–VTBS |
| Vorher (5908ac3) | RJTT | Continental Sky | 24,215,164 | A320 144Y/18J, 737 153Y/18J | SBGR–SAEZ, SBGR–SCEL |
| Nachher | EDDF | Regional Wings | 107,809,067 | ATR 58Y/7J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, A330 180Y/60J, 737 115Y/37J | EDDF–EDDM, EDDF–OMDB, EDDF–EGLL, EDDF–LFPG, EDDF–EHAM, EDDF–LEMD |
| Nachher | EDDF | Continental Sky | 34,207,235 | A320 144Y/18J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, 737 115Y/37J | SBGR–SAEZ, SBGR–SCEL, SBGR–SPJC, EGLL–EDDM, SBGR–EGLL |
| Nachher | SBGR | Regional Wings | 34,317,471 | A320 144Y/18J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, 737 115Y/37J | SBGR–SAEZ, SBGR–SCEL, SBGR–SPJC, EGLL–EDDF, SBGR–EGLL |
| Nachher | SBGR | Continental Sky | 107,310,890 | A320 144Y/18J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, A320 108Y/36J, 737 115Y/37J | RJTT–ZBAA, RJTT–VHHH, RJTT–EGLL, RJTT–OMDB, RJTT–WSSS, RJTT–VTBS |
| Nachher | RJTT | Regional Wings | 106,286,287 | A320 144Y/18J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, A320 108Y/36J, 737 115Y/37J | RJTT–ZBAA, RJTT–VHHH, RJTT–EGLL, RJTT–OMDB, RJTT–WSSS, RJTT–VTBS |
| Nachher | RJTT | Continental Sky | 34,406,772 | A320 144Y/18J, 737 115Y/37J, A330 180Y/60J, A330 180Y/60J, 737 115Y/37J | SBGR–SAEZ, SBGR–SCEL, SBGR–SPJC, EGLL–EDDF, SBGR–EGLL |

**Ja, alle sechs Rivalen-Szenarien betreiben jetzt Widebodies.** Die EDDF-Firma
least drei A330 für europäische Kurzstrecken; SBGR least zwei, RJTT ebenfalls zwei.
Die bisherige Wochensuche optimiert Beitrag und Reserve, nicht die Flugzeugklasse
zur Streckenlänge. Daher ist auch eine Kurzstrecken-A330 eine normale zulässige
Entscheidung; es wurde keine Langstrecke erzwungen.

Die SBGR-Firma eröffnet in Woche 36 SBGR–EGLL mit 60 J (erwartet 100.510 EUR/Woche
nach Leasingrate) und in Woche 44 zunächst SBGR–KORD (193.555 EUR/Woche).
Die zweite Strecke wird später geschlossen; dieselbe freie A330 startet über die
vorhandene Standortlogik von EGLL nach EDDM (Spieler EDDF) beziehungsweise EDDF
(Spieler SBGR/RJTT). Das ist kein neuer zweiter Basismechanismus.
Nach zwei Jahren fliegt SBGR–EGLL für 502/1.004 EUR; RJTT–EGLL für 497/994 EUR,
RJTT–OMDB für 425/850 EUR, jeweils mit 180 Y / 60 J.
In Spielerbasis SBGR ist RJTT–VTBS beim Endstichtag ohne aktiven Flugplan;
bei Spielerbasis RJTT ist sie aktiv. Eine Flotten-/Routenliste allein garantiert
also keinen ununterbrochenen Betrieb jeder Verbindung.

Die schmalen Typen ändern sich ebenfalls: Frankfurt eröffnet zunächst OMDB
mit 737 und 115 Y/37 J (Woche 4, 442.383 EUR/Woche nach Miete), später LEMD statt
der früheren ATR-Routen LIRF/EGCC. Tokio ersetzt VHHH-A320 durch 737 mit 37 J,
nimmt EGLL/OMDB statt VIDP auf und kauft WSSS-A320 mit 36 J und VTBS-737 mit 37 J.
São Paulo behält SAEZ/SCEL, hat auf SCEL jetzt 37 statt 18 J und kauft später eine
737 für SPJC. Höhere ehrlich quotierte Beiträge ermöglichen frühere/weitere
Beschaffungen; die folgenden Geldflüsse verändern auch spätere Entscheidungen.
Starter bleiben bei 7 J (ATR) beziehungsweise 18 J (A320).

Startzustandsprobe mit `cabin_expansion_behavior_audit` (vor Expansion, normale
Angebote; Wochenbeitrag einschließlich Personal/Wartung, vor Miete und
Einmalgebühr). Economy und Zwei-Klassen wählen jeweils ihren besten bisherigen
Suchtarif, nicht einen künstlich schlechten Vergleichstarif:

| Markt / Modell | Economy bester Tarif / Beitrag EUR | Zwei-Klassen Tarif Y/J / J-Sitze / Beitrag EUR |
| --- | ---: | ---: |
| EDDF–OMDB / 737 | 228 / 52.892 | 304/608 / 37 / 513.117 |
| SBGR–SCEL / 737 | 148 / 12.227 | 180/360 / 37 / 320.871 |
| RJTT–VHHH / 737 | 194 / 195.011 | 229/458 / 37 / 506.147 |
| RJTT–WSSS / A320 | 272 / 110.871 | 354/708 / 36 / 522.295 |
| SBGR–EGLL / A330 | 394 / −524.222 | 482/964 / 60 / 77.431 |
| RJTT–EGLL / A330 | 399 / −422.601 | 487/974 / 60 / 242.973 |

Die Business-Option kostet zusätzlich 500 EUR einmalig. Spätere Kraftstoffpreise,
Geldbestände und Konkurrenz erklären, warum die tatsächliche Entscheidung
abweichende Beiträge hat und nicht jedes positive Startangebot sofort gekauft wird.

## Prüfungen

Abschluss: `cargo build/test/clippy --locked --manifest-path rust/Cargo.toml`
bestanden; vollständige Rust-Suite **259 bestanden, 0 fehlgeschlagen, 3 ignoriert**
(gezielte optionale Budget-/Verhaltensaudits separat ausgeführt), Format-/Diffcheck
bestanden. `tools/godot_check.sh tests`: **40 bestanden, 0 fehlgeschlagen**;
Windows-Neustarttreiber vom Runner übersprungen. Zusätzliches Clippy über alle
Targets besteht mit vier bereits vorhandenen Testcode-Warnungen.

- `widebody_expansion_installs_quoted_cabin_fare_and_books_fee`: echte
  Langstreckenquote EDDF–KJFK mit Business, Ausführung sowohl Kauf als auch
  Leasing, exakt angebotene Kabine/Tarif/Intervall, Gebührenjournal/Kasse,
  Kabinenreview und tatsächlicher Abflug mit Business-Passagieren. Nur die
  Ausführungsfixture hat Testkapital; die Produktion behält alle Kaufprüfungen.
- `single_cabin_fare_sweep_matches_fresh_quotes`: vorbereitete gegen frische
  Angebote für schmale/breite Typen, 0/20/40/50/100 Prozent Fläche,
  steigende/fallende Tarife einschließlich null/Preisdeckel, drei Fuelpreise.
- `two_year_catalogue_ai_audit`: wöchentlich gegen täglich, 1×/4×/16×,
  Pause und Save/Load nach einem Jahr; vollständiger Zustand gleich für alle
  drei Spielerbasen. Bestehende v30-/ältere Migrationsproben unverändert aktiv.
- `sustained_high_fuel_keeps_viable_player_and_both_ai_alive`: unveränderte
  fünfjährige Hochpreisprobe samt Solvenz-, Aktivitäts- und Neustartprüfungen.
- Budgetprobe mit `AIRLINE_ENFORCE_AI_BUDGET=1`, seriell ohne parallelen Build:

| Spielerbasis | Advance zwei Jahre | Langsamste Woche |
| --- | ---: | ---: |
| EDDF | 1.663 s | 61 ms |
| SBGR | 1.699 s | 79 ms |
| RJTT | 1.745 s | 89 ms |

Grenzen unverändert: 2 s / 150 ms. Ein erster Lauf direkt nach dem Build lag
für EDDF bei 2.254 s / 151 ms; der anschließende explizit erzwungene ruhige
Budgetlauf bestand. Wanduhrwerte hängen von der Serverlast ab.

Die alte exakte KI-Kassen-/Routengoldenprobe für Economy-Suche wird durch eine
Solvenz-/Flottengrenzenprobe für historische Frankfurt-Firmen ersetzt. Kauf-
und Gebrauchtkauftests rechnen die normale Kabinengebühr ein; der
Unrentabilitätstest prüft die Wiederanlaufkadenz isoliert mit kostenlosen
Sitzen in beiden Klassen, damit legitime Expansion in andere Märkte ihn nicht verdeckt.
Der Frequenz-Kauftest finanziert den teureren gewählten Typ ausreichend;
Kauf- und Leasingzweig bleiben separat geprüft.

## Sichtprüfung und Grenzen

`tools/godot_check.sh shots /paperclip/shots/AIR-184` erzeugt 239 PNGs.
Geöffnet und geprüft wurden die finalen `ai-cabin-map-1440x1000.png` und
`ai-cabin-map-680x520.png`: Continental Skys organisch erworbene A330 auf
SBGR–EGLL, Economy 502 EUR / Business 1.004 EUR, eine tägliche Abfahrt je Richtung.
Kartenname, Flugzeugname, Frequenz und beide Geldwerte bleiben vollständig in
der Karte. Keine neue Textüberlappung, falsche Farbe/Schrift oder abgeschnittene
Kartensteuerung gefunden. Schmale Ansicht nutzt den vorhandenen vertikalen Scroll.
Die erste Aufnahme zeigte die Karte unterhalb des Viewports angeschnitten;
die Fixture scrollt jetzt ausdrücklich zur Karte und prüft deren globale
Sichtbarkeit. Danach neu aufgenommen und beide Größen erneut geöffnet.
Nur die Test-Captureliste wurde erweitert, kein Produktions-GDScript geändert.
Die beiden geprüften Bilder sind als `AIR-184-visual-review.zip` am Issue hinterlegt.

START.bat, PowerShell-Neustarttreiber und echte GPU wurden hier nicht geprüft.
Die Bilder verwenden Software-OpenGL; unter Windows können sie leicht abweichen.
