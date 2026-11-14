# Technische Verspätungen

Rust berechnet je Flugzeug und Lebenszeitzyklus einen deterministischen Wurf
(mit dem Unternehmens-/Flugzeug-Seed der Versicherung, eigener Zufallsdomäne).
Alter seit Erwerb und Zustand seit dem letzten Check erhöhen Häufigkeit und
mögliche Dauer. Alle Wahrscheinlichkeiten, Dauern und Schwellen stehen in
`config/disruptions.json`; es gibt keinen Bezug zur Systemzeit.

Der tatsächliche Abflug ist das Maximum aus geplantem Abflug und Ende der
vorherigen Mindestbodenzeit, zuzüglich einer eigenen technischen Verzögerung.
Freie Zeit im Flugplan baut Rückstand ab. Flugzeit und Mindestbodenzeit werden
nicht gekürzt. Wartung und Versicherungsereignisse verwenden die tatsächliche
Landung. Die vorhandenen Personal- und Wartungssperren gelten weiterhin.

Beim Abflug werden Preis, Passagiere und Kosten eingefroren; die Buchung erfolgt
wie bisher bei Landung. Über der Betreuungsschwelle entstehen pro Passagier
Betreuungs-/Entschädigungskosten als eigene Position `care`. Sie fließen über die
Flugkosten in Ergebnis, Archiv und Steuerbasis ein. Auch weitergegebene
Verspätung zählt; keine erneute Buchung bei Laden oder zwischen Bildern.

Pünktlichkeit misst Ankünfte innerhalb der konfigurierten Toleranz (Flugzeiten
bleiben konstant, deshalb entspricht das der Abflugverspätung). Route und
Airline zeigen den Anteil der seit Einführung erfassten pünktlichen Flüge;
„—“ bedeutet keine erfassten Flüge. Flotte und Betrieb zeigen Verspätung, die
Aufmerksamkeitsliste zeigt große Rückstände. Die Nachfrage erhält keinen
Pünktlichkeitsfaktor. KI-Airlines verwenden dieselbe Simulation.

Spielstandversion 26 speichert einen kompakten Zeitcursor pro Flugplan sowie
Pünktlichkeits- und Betreuungssummen im bestehenden Finanzarchiv. Der kompakte
Zähler `otp` erfasst alle beobachteten Flüge, `late` nur verspätete; pünktliche
Flüge ergeben sich aus der Differenz. Nullwerte werden im Archiv ausgelassen. Ältere
Spielstände haben durch neutrale Standardwerte keine laufenden Verspätungen und
keine Pünktlichkeitshistorie. Bereits fliegende Abflugzitate und Flugzeiten bleiben
unverändert; neue Würfe beginnen danach. Checks und bewusste Neuplanung beginnen
wie bisher einen neuen Flugplan, erhalten aber Lebenszeitverschleiß und Seed.

Technische Ereignisse erzeugen zusätzliche Archivgrenzen. Der unveränderte
Millionenflug-Laufzeitgrenzwert beträgt 300 Sekunden für die Simulation; die
begrenzte maximale Spielstandgröße steigt wegen der jahrhundertelangen
KI-/Spielerhistorie von 128 auf 256 MiB. Normale Zehnjahresspielstände bleiben
im einstelligen MiB-Bereich; der Belastungstest umfasst mehrere Millionen Flüge.

## Verifikationsfamilie

`tests/disruptions_test.gd` verwendet unveränderte Produktionskonfiguration und
Seed 33364 mit 570 Check-/Lebenszeitzyklen. Der Rust-Fixturetest fixiert die
Zeitpunkte: Abflug fällig bei 60 s, tatsächlicher Abflug 2992 s, Landung 5104 s;
der zweite Abflug bei 9612 s hat 4152 s Rückstand. Der dritte Flug erhält ohne
neues technisches Ereignis 2664 s Rückstand; die Rotation holt ihn mit freier
Flugplanzeit auf. Normale KI-Airlines bleiben im Weltzustand enthalten.

Der Godot-Test führt echte Engineframes und kontrollierte Grenzframes über
`main._process` bei 1×/4×/16× aus. Er prüft Flotte/Betrieb, „—“ und spätere
Pünktlichkeitsanteile, einmalige Betreuung in Routen-/Flugkosten, Laden sowie
Aufmerksamkeitszeilen. Layout-/Renderfälle: 1440×1000, 1100×720 und 680×520.
Mit `-- --capture-dir=<vorhandenes Verzeichnis>` schreibt der gerenderte Lauf
unveränderte Viewport-PNGs. Die Größen-/Erreichbarkeitsprüfungen ersetzen keine
Sichtprüfung der Originalbilder; diese anschließend an das Issue anhängen.

`tests/run_disruption_restart_test.ps1` startet vier getrennte Prozesse:
Writer/Reader für v25→v26 (neutrale Historie und eingefrorenes altes Flugzitat)
und v26 (verspätetes Flugzeug in der Luft mit Betreuungskosten). Vollständiger
Spieler-/KI-Spielstand und API-Snapshots werden sofort sowie nach 100 Tagen
verglichen. Die v25-Datei entfernt nur neue Felder ohne JSON-Neukodierung,
damit große Seeds und Bruchteilsjournale unverändert bleiben. Ein v25-Flug kann
noch keine technische Verspätung haben. Der Rust-Test prüft beide Familien auch
ohne Engine; nur der PowerShell-Lauf prüft einen echten Prozessneustart.

Servergrenze für AIR-87: Godot, GUI/Xvfb, START.bat und Windows-Neustarts wurden
hier nicht ausgeführt; es wurden keine Renderbilder erzeugt oder angehängt.
Beim Windows-Eigentümer verbleiben der neue Godot-Test samt Bildsichtprüfung,
der neue und die bestehenden Restart-Wrapper, die bestehenden Godot-Suiten
und START.bat. Änderungen betreffen ausschließlich Tests und Dokumentation;
Spielregeln, Wirtschaft, Konfiguration und Speicherformat bleiben unverändert.
