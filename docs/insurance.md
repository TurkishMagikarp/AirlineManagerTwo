# Flugzeugversicherung und Betriebsschäden

Alle Regeln liegen in `rust/src/insurance.rs`, alle Stellwerte in
`config/insurance.json`. Ein unabhängiger adressierbarer Hash entscheidet je
Landung (Standard: 200 pro Million, also 0,02 %). Schlüssel sind Weltseed,
Airline-Name, Flugzeugkennung und lebenslange Landungsnummer. Ein abgeleiteter,
nicht gespeicherter Cache sucht den nächsten Treffer; Frames und UI verwenden
keinen Zufall. Typ und Reparaturhöhe verwenden einen getrennten Hash.

Vogelschlag, Bodenschaden und harte Landung kosten standardmäßig 1–5 % des
aktuellen Zeitwerts und mindestens 12 Stunden Standzeit. Zeitwert verwendet
dieselbe Alters-/Zustandsformel wie der Verkauf; bei betriebenen Leasingmaschinen
zählt der Rumpfwert, nicht die rückzahlbare Kaution. Verkauf/Rückgabe entfernt
das Flugzeug aus der Risikomenge. Das Spiel hat keinen Verleih eigener Maschinen
an andere Airlines.

Reparaturen nutzen den bestehenden Wartungshangar einschließlich Technikerbedarf,
Abflugsperre und Wiederaufnahme am tatsächlichen Standort. Bereits beauftragte
oder gleichzeitig fällige Checks bleiben bezahlt; die längere der beiden
Standzeiten gilt. Schäden werden genau einmal abgerechnet, auch ohne ausreichend
Bargeld. Der normale Notlagen-/Restrukturierungsablauf verarbeitet negative
Kontostände. Eine Reparatur schließt wie andere Hangarbesuche den Check ab.

Deckung je Airline: keine, Basis oder Voll. Standard-Jahresprämien sind 0,12 %
oder 0,30 % des Flottenzeitwerts, Selbstbehalte 1 % oder 0,1 % je Schaden;
Versichererlimit jeweils 100 % des Zeitwerts. Der Versicherer zahlt den Schaden
abzüglich Selbstbehalt bis zum Limit; ein darüber hinausgehender Rest bleibt
unversichert. Auch betriebene Leasingmaschinen sind enthalten. Neue KI-Airlines
wählen Basis; migrierte Airlines behalten zunächst keine Deckung.

Die Jahresprämie wird täglich aus dem Flottenwert neu bestimmt und sekundengenau
mit ganzzahligem Restanteil täglich sowie beim Deckungswechsel abgerechnet.
Käufe, Verkäufe und Zustandsänderungen wirken beim nächsten Tageswechsel auf die
Prämie. Deckungswechsel wirken sofort auf künftige Landungen; bereits entstandene
Schäden bleiben unverändert. Abgeschaltete Deckung löscht weder Ansprüche noch
Prämienreste oder Schadenaufschläge.

Erneuerung nach jeweils 365 Tagen ab Gründung bzw. Migration: pro bezahlt gedecktem
Schaden +15 Prozentpunkte Prämienaufschlag (maximal 100 %), pro sauberem Jahr
−5 Prozentpunkte bis null. Der Jahreszähler ist unabhängig von der auf 20 Einträge
begrenzten Anzeigehistorie. Prämien, Selbstbehalte und ungedeckte Reparaturen
stehen getrennt im Finanzjournal und in der Finanzen-Ansicht; sie zählen zu
Gesamtkosten, Ergebnis, Kontoverlauf und steuerlich abziehbaren Aufwendungen.

Spielstand v23 migriert ältere Dateien für Spieler und KI auf keine Deckung,
leere Schadenhistorie und einen Beginn zum Ladezeitpunkt. Geld, Flugpläne,
Abflugangebote, Abnutzung und bestehende Wirtschaftshistorie werden nicht neu
berechnet.

## Automatische Prüfung

`Godot --headless --path . --script res://tests/insurance_test.gd`
prüft echte Maus-/Tastatureingaben (keine manuell ausgelösten UI-Signale):
keine → Basis → Voll → keine. Quotes, Jahresprämie, Selbstbehalt, Limit,
Aufschlag, gebuchte Prämien/Selbstbehalte/unversicherte Reparaturen und jede
Schadenzeile werden mit der Rust-API verglichen. Seed 1288 erzeugt für
„Insurance UI“/Flugzeug 1 beim ersten Anflug einen echten Schaden; getrennte
ungedeckte und gedeckte Läufe vermeiden zufallsabhängiges Warten. Ein expliziter
Aufschlag-Fixture prüft das Abschalten während der Reparatur: Schadenhistorie,
Jahreszähler, nicht nuller Prämienrest, Aufschlag und Reparaturauftrag bleiben
unverändert. Versichererzahlungen erfolgen bereits beim Schaden; die Anzeige
ist eine nur lesbare Historie, kein weiterer Auszahlungs-/Auswahlknopf.

Mehrprozessprüfung unter Linux (Schreiber jeweils vollständig beenden, dann
Leser starten; für v22 und v23 wiederholen):

```sh
Godot --headless --path . --script res://tests/insurance_test.gd -- --insurance-mode=write-v23 --insurance-save=/tmp/insurance-v23.json
Godot --headless --path . --script res://tests/insurance_test.gd -- --insurance-mode=read-v23 --insurance-save=/tmp/insurance-v23.json
```

Der v22-Fixture enthält keine Versicherungsfelder und hat ein laufendes
Abflugangebot. Migration erhält die übrige vollständige JSON-Datei bytegenau,
setzt alle Eigentümer auf keine Deckung/leere Schäden. v23 enthält einen
bezahlten gedeckten Schaden, eine laufende Reparatur und KI-Basisdeckung.
Leser vergleichen komplette serialisierte Welt und API-Ausgaben direkt nach
Neustart sowie nach weiteren 100 Tagen mit dem ununterbrochenen Schreiber.
JSON wird nicht über Godot-Doubles neu serialisiert.

Für echte Renderaufnahmen zusätzlich `--capture-dir=/tmp/insurance-captures`
übergeben (Verzeichnis vorher anlegen), ohne `--headless`, z. B. unter Xvfb
mit `--rendering-method gl_compatibility`, `LIBGL_ALWAYS_SOFTWARE=1`.
Der UI-Modus prüft 1440×1000, 1100×720, 680×520 und schreibt je ein Bild der
Deckungswahl und der Schadenhistorie. Kleine Fenster benötigen vertikales
Scrollen; der Test erreicht und klickt die Deckungswahl sowie die letzte
Schadenzeile. Frische `XDG_DATA_HOME`-/`LOCALAPPDATA`-Verzeichnisse verwenden.

## Prüfung beim Eigentümer

Linux-Godot 4.7.2, separate Prozesse und Xvfb/llvmpipe wurden für AIR-62
verifiziert; siehe `docs/verification-log.md`. Diese Linux-Ergebnisse ersetzen
keine nativen Windows-Prüfungen. Auf diesem Server nicht ausgeführt:

1. Godot schließen, Windows-Rust-DLL neu bauen, `START.bat` starten; normales
   Fenster und Speichern/Laden prüfen (einschließlich Unicode-Pfad).
2. `powershell -ExecutionPolicy Bypass -File tests/run_insurance_restart_test.ps1`
   ausführen, bei Bedarf `-Godot <Pfad>` setzen. Der neue Wrapper startet vier
   getrennte Prozesse für v22/v23; PowerShell- und Windows-DLL-Verhalten bleiben
   hier ungeprüft.
3. Finanzen in breitem und schmalem Fenster öffnen; Deckung mit Maus und Tastatur
   umschalten, zur Schadenliste scrollen, Prämien/Selbstbehalte/Reparaturen und
   Schadenzeilen lesen. Natives Windows-Rendering, Skalierung und Eingabe prüfen.
4. Spiel mit laufender Reparatur beenden und neu starten, Anspruch/Restanteile
   und Fortsetzung prüfen; auch eine vorhandene echte v22-Datei laden.
