# Flughafengebühren: Godot- und Neustartprüfung

Stand: 14. November 2026, Linux, Godot 4.7.2, Rust-Debug-Erweiterung,
Basis PR #31. Ergebnisse und sechs geprüfte Originalaufnahmen:
[AIR-69](https://paperclip.iamsec.de/AIR/issues/AIR-69).

`tests/airport_charges_test.gd` öffnet die echte Routenoberfläche mit
Maus-/Tastaturereignissen. Alle Flughafen-Gebührenstufen sowie die exakten
Passagier-, Gesamtkosten- und Flughafenkostenangaben der Planung werden gegen
die Rust-API geprüft. Nach Routenerstellung und Flugplanaktivierung per Maus
wird auch die Gebührenangabe des aktiven Dienstes verglichen.

Die Modi `write-v23`/`read-v23` und `write-v24`/`read-v24` laufen jeweils in
getrennten Prozessen mit expliziten Speicherpfaden. Der v23-Schreiber erzeugt
ein Legacy-Testfixture: nur Versionskopf und eingefrorene Flughafentarife
werden textuell angepasst; große Rust-Ganzzahlen und Bruchteilsjournale werden
nicht über Godots JSON-Fließkommazahlen neu serialisiert. Der alte Tarif
(Modellgrundpreis plus Passagiertarif, einmalig) unterscheidet sich sichtbar
vom neuen Start-/Zieltarif:

| Abflug | v23 eingefroren | v24 Start + Ziel |
| --- | ---: | ---: |
| Spieler | 1.475 EUR | 1.697 EUR |
| Regional Wings | 1.950 EUR | 2.243 EUR |
| Continental Sky | 5.900 EUR | 7.375 EUR |

Unmittelbar nach dem Laden müssen die vollständige serialisierte Spieler-/KI-Welt
und die API-Snapshots identisch sein. Die ersten Landungen aller drei Besitzer
übernehmen sämtliche eingefrorenen Abflugwerte exakt. Die folgenden Abflüge
verwenden die neuen Endpunkttarife. Nach exakt weiteren 100 Tagen stimmen Welt
und API wieder mit dem ununterbrochenen Lauf überein.

Linux-Aufruf für jeden Modus (Schreiber zuerst beenden, dann Leser starten):

```sh
Godot --headless --path . --script res://tests/airport_charges_test.gd -- \
  --airport-mode=write-v23 --airport-save=/tmp/airport-v23.json
Godot --headless --path . --script res://tests/airport_charges_test.gd -- \
  --airport-mode=read-v23 --airport-save=/tmp/airport-v23.json
# Dasselbe mit write-v24/read-v24 und einem eigenen Speicherpfad.
```

`tests/run_airport_restart_test.ps1` liefert dieselbe Vier-Prozess-Sequenz für
den Besitzer unter Windows (`AIRLINE_GODOT` oder den bisherigen Standardpfad).
Die PowerShell-Datei selbst wurde hier nicht ausgeführt.

Die vollständige Godot-Regression lief sequenziell: alle 27 eigenständigen
Tests, Hauptszene mit `--quit-after 10`, Editor-Import, laufende/pausierte
Unicode-Speicherpfade sowie alle bisherigen Neustartfamilien bestanden.
Der Finanzarchiv-Test mit etwa zwei Millionen Flügen bestand in 343,97 s
(einschließlich Speichern, UI und Fortsetzung), sein frischer Leser in 124,33 s.
Das ist nicht die separate Rust-Messung der reinen Fortschreibung.

Dabei wurden veraltete v23-Kopfersetzungen und aktuelle Versionsassertionen
in den Migrationsfixtures auf v24 korrigiert. Ein Marketingtest verlangte
auch bei jeder KI einen von null verschiedenen EUR-Rest; der aktuelle
KI-Etat kann zum Prüfzeitpunkt jedoch korrekt auf ganzen EUR aufgehen.
Der Test verlangt weiterhin einen echten Spieler-Bruchteil, positive Etats,
Bekanntheit und gebuchte Kosten aller Besitzer, gültige KI-Reste und die
unveränderten verlustfreien Neustartvergleiche. Gameplay blieb unverändert.

Echte Xvfb-/OpenGL-Aufnahmen mit Mesa llvmpipe in 1440×1000, 1100×720 und
680×520 wurden einzeln geprüft und an der Aufgabe hinterlegt. Planung:
72 Passagiere, 3.978 EUR Gesamtkosten, 2.760 EUR Flughafenkosten. Aktiver Dienst:
1.697 EUR Flughafenkosten. Die betreffenden Textzeilen waren lesbar; bei
680×520 werden Navigation umgebrochen und Inhalte vertikal gescrollt.
Headless-Assertions allein gelten nicht als Sichtprüfung.

Native Windows-DLL-/Prozessprüfung, START.bat, die tatsächlichen
PowerShell-Aufrufe und Windows-Darstellung/-Skalierung bleiben Besitzerprüfungen.

Rust: Locked-Build und Clippy bestanden. Alle 202 Tests abgedeckt: 201 Tests
im vollständigen Lauf (240,86 s Testzeit), danach der Millionenflugtest allein.
Gemessene Fortschreibung: **243.88 s** unter dem unveränderten
**300-s-Limit**; Speichern/Laden/Fortsetzung ebenfalls bestanden.
Das 300-s-Limit wurde weder angehoben noch umgangen. Rust- und Gameplaycode
blieben unverändert. Ein erster Rust-Runner wurde vor einem Ergebnis durch
SIGTERM unterbrochen und zählt nicht als bestandener Testlauf.
