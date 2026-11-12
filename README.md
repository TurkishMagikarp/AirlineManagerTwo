# Airline Manager — Neustart

## Starten

Unter Windows `START.bat` ausführen. Voraussetzung: Godot 4.6+ (getestet mit
4.7.2), Rust über rustup und Visual Studio C++ Build Tools inklusive Windows SDK.
Der Launcher baut die Rust-Bibliothek, importiert die GDExtension und startet Godot.
Godot wird über `AIRLINE_GODOT`, den PATH oder den bisherigen lokalen Pfad
`%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe` gesucht.
Bei Buildfehlern wird das Spiel nicht mit einer veralteten Bibliothek gestartet.
Ein fehlgeschlagener Godot-Erstimport wird einmal wiederholt (hier trat sporadisch
ein Editor-Absturz beim Import-Ende auf); bei erneutem Fehler wird abgebrochen.
Der erste Rust-Build benötigt Internet für die Cargo-Abhängigkeiten; das Spiel läuft offline.

Alternativ `cargo build --locked --manifest-path rust/Cargo.toml`, danach
`project.godot` im Editor öffnen und **F5** starten. Vor einem erneuten Rust-Build
Spiel und Editor schließen (Windows sperrt geladene DLLs). Für Release-Exporte
zuvor mit `cargo build --release --locked --manifest-path rust/Cargo.toml` bauen.
macOS/Linux sind in der Extension konfiguriert, aber noch nicht getestet;
die Bibliothek muss auf der jeweiligen Zielplattform gebaut werden.

## Kleiner Simulationskern

Rust (`rust/src/game.rs`, `rust/src/clock.rs`) besitzt Airline und Spielzeit,
Pause und Geschwindigkeit.
`rust/src/lib.rs` ist nur der godot-rust/GDExtension-Adapter. Godot reicht die
Frame-Dauer und Bedienaktionen weiter und stellt den Rust-Zustand dar. Es gibt
keine zweite Uhr oder Zeitberechnung in GDScript und keine Beispieldaten.

Jede Gründung beginnt pausiert bei **Tag 1 · 00:00:00** (verstrichene Spielzeit,
kein erfundenes Kalenderdatum). **Start / Pause**, **1× / 4× / 16×** steuern die Uhr;
1× entspricht einer Spielsekunde pro realer Sekunde. Diese Stufen sind vorläufige
technische Testwerte, keine endgültige Balancing-Entscheidung. Navigation bleibt
per Maus oder Tab / Umschalt+Tab und Enter / Leertaste bedienbar.

Noch keine Offline-Fortschreibung, Ereignisse, Zeitsprünge zu
Ereignissen/Zeitpunkten, Wirtschaft, Flotten oder Routen. Spätere Spielregeln und
Berechnungen gehören in den Rust-Kern, nicht in Godot.

Produktregeln und offene Fragen: [Game Design](docs/game-design/README.md).

## Eigene Airline gründen

Beim Start erscheint der Gründungsbildschirm. Namen eingeben, Heimatland und
einen dort gelegenen Flughafen auswählen, **Airline gründen** drücken. Danach
zeigt die bisherige Übersicht Name, Heimatland, Heimatflughafen und Startkapital.
Alternativ **Spielstand laden** drücken. Namen werden in Rust getrimmt und auf
1–80 Zeichen ohne Steuerzeichen geprüft; auch Land und Flughafen werden dort
validiert. Fehlgeschlagene Gründungen lassen einen bestehenden Zustand unverändert.

**Neues Spiel** öffnet die Gründung erneut. Bis zur Bestätigung bleibt der alte
Zustand erhalten, **Abbrechen** kehrt zurück. Während des Formulars läuft die Zeit
nicht weiter. Erst erfolgreiches Gründen ersetzt die laufende Airline und setzt
die Uhr zurück. Die gespeicherte Datei wird erst durch **Speichern** überschrieben.

Startkapital: `config/new_game.json`, Feld `starting_capital_eur` (nichtnegative,
ganze Euro). Der mitgelieferte Wert **10.000.000 EUR** und die vorläufige
Darstellungswährung EUR sind technische Testkonfiguration, keine beschlossenen
Wirtschafts- oder Länderregeln. Datei ändern, Spiel/Editor schließen und über
`START.bat` neu bauen. Die Einstellung gilt nur für neue Gründungen; geladene
Airlines behalten ihr gespeichertes Kapital. Noch keine Buchungen oder Kontobewegungen.

Sechs echte Flughäfen in Deutschland, Türkei und USA stehen zur Verfügung.
Der [Katalog](data/README.md) liegt getrennt in `data/airports.json` und verwendet
stabile ICAO-/Ländercodes als Grundlage für eine spätere weltweite Datenquelle.

## Speichern und Laden

**Speichern** ersetzt den einen lokalen Spielstand; **Laden** stellt ihn wieder
her. Nach einem Neustart **Spielstand laden** drücken: Airline einschließlich
Name, Land, Heimatflughafen und Kapital sowie Spielzeit (nanosekundengenau),
Pausenstatus und Geschwindigkeit werden wiederhergestellt. Ein laufender
Spielstand läuft anschließend weiter, ein pausierter bleibt pausiert. Die Zeit
während des geschlossenen Spiels wird nicht angerechnet. Es gibt bewusst kein
automatisches Speichern oder Laden.

Datei: `user://savegame.json`, von Godot auf den beschreibbaren Benutzerordner
aufgelöst (unabhängig von Installationsordner und Arbeitsverzeichnis):

- Windows: `%APPDATA%\Godot\app_userdata\<Projektname>\savegame.json`
- macOS: `~/Library/Application Support/Godot/app_userdata/<Projektname>/savegame.json`

Der aktuelle Projektname ist `Airline Manager — UI-Basis`. Der tatsächliche Pfad
steht auch im Tooltip der Speichermeldung. macOS ist hier nicht praktisch getestet.

Rust (`rust/src/save.rs`) übernimmt Dateizugriff, JSON-Format und Validierung.
Das Format hat Version **2** und einen `state`-Bereich mit `clock` und `airline`,
der später um weitere Simulationsdaten ergänzt werden kann. Alte Zeit-Spielstände
der Version **1** bleiben lesbar, enthalten jedoch noch keine Airline. Ihre Uhr
wird ohne erfundene Unternehmensdaten geladen; der Gründungsbildschirm erklärt,
dass eine anschließende Gründung ein neues Spiel beginnt. Inkompatible Änderungen
brauchen eine neue Version und gegebenenfalls eine Migration; unbekannte Versionen
werden jetzt abgelehnt. Keine Serialisierung von Godot-Objekten.

Eine vollständig geschriebene und synchronisierte temporäre Datei im selben
Ordner ersetzt erst zum Schluss den bisherigen Spielstand. Fehler beim Laden
(fehlende, defekte, zu große oder ungültige Dateien) erscheinen als Meldung und
verändern die laufende Simulation nicht. Noch keine mehreren Speicherplätze,
Backups oder automatische Wiederherstellung. Ungültige Airline-Daten werden
ebenso abgelehnt wie defekte Zeitdaten, ohne den laufenden Zustand zu ersetzen.

## Tests

Nach dem Rust-Build und dem ersten Godot-Import aus diesem Projektordner:

```text
cargo test --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --script res://tests/navigation_test.gd
Godot --headless --path . --script res://tests/simulation_test.gd
Godot --headless --path . --script res://tests/founding_test.gd
powershell -ExecutionPolicy Bypass -File tests/run_save_restart_test.ps1
```

`Godot` durch den eigenen Executable-Pfad ersetzen. Der Verbindungstest lädt die
echte DLL, klickt die tatsächlichen UI-Buttons und prüft auch den Frame-Betrieb.
Die Godot-Tests können ohne `--headless` mit echtem Fenster ausgeführt werden.
`START.bat --headless --quit-after 10` prüft den gesamten Startpfad.

Der Neustarttest startet vier vollständig getrennte Godot-Prozesse: jeweils
Speichern und Wiederherstellen eines laufenden und eines pausierten Spielstands.
Er gründet eine Airline über die UI, klickt die echten Buttons und prüft nach
vollständigem Neustart Airline, Kapital, exakte Zeit, Geschwindigkeit und Fehlerfälle.
Die Testdateien liegen isoliert im Temp-Ordner, nicht im persönlichen Spielstand.
Optional `-Godot "C:\Pfad\Godot.exe"` angeben. Auf macOS kann dasselbe GDScript
in separaten Prozessen mit `--save-test-mode=write-running` / `read-running` /
`write-paused` / `read-paused` und `--save-test-path=/absoluter/testpfad.json`
nach dem `--`-Trenner aufgerufen werden.
