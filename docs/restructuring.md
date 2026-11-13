# Restrukturierung und Insolvenz

Rust verwaltet Finanznot, Fristen, Umschuldung und Insolvenz. GDScript zeigt nur
Status und Frist im Finanzbereich und leitet die Umschuldung weiter.
Alle Stellwerte stehen in `config/financing.json` und gelten nach Neubau.

- Finanznot bleibt definiert als negativer Kontostand oder überfällige Tilgung.
  Nach 30 ununterbrochenen Tagen beginnt eine Restrukturierung mit 60 Tagen Frist.
  Gehälter, Leasing, Wartung und Kredite laufen während dieser Zeit weiter;
  bestehende Regeln sperren Expansion und neue Abflüge in Finanznot.
- Einmal je Unternehmen kann der bestehende Kredit umgeschuldet werden:
  Restschuld auf 48 Raten verteilen (ganze EUR aufrunden), Rückstände stunden,
  nächste Rate nach 30 Tagen. Der Zinssatz bleibt bei 6 % p.a. Die Gebühr beträgt
  2 % der Restschuld, auf volle EUR aufgerundet. Sie wird sofort vom Konto
  abgezogen und als Finanzierungskosten im bisherigen Zinsjournal verbucht.
  Es gibt weder eine Geldzufuhr noch eine Verlängerung der Restrukturierungsfrist.
  Ohne Kredit bleibt der Verkauf oder die Rückgabe von Flugzeugen möglich.
- Zahlungsfähigkeit nach vollständiger Abrechnung eines Ereignisses beendet die
  Restrukturierung. Bei erneuter Finanznot beginnt eine neue Schonfrist; die
  einmalige Umschuldung bleibt verbraucht, auch nach kompletter Kredittilgung.
  Zahlungsfähigkeit genau bei der Schlussabrechnung am Fristende hat Vorrang.
- Ohne Erholung tritt zum Fristende Insolvenz ein. Das Spielerunternehmen und
  seine Welt bleiben zu diesem Zeitpunkt eingefroren und einsehbar. Management,
  Fortsetzen und Neugründung in derselben Welt sind gesperrt. Speichern und Laden
  bleiben möglich; es erfolgt kein automatischer Neustart.
- KI-Unternehmen nutzen dieselben Fristen, Kreditbedingungen und Umschuldung.
  Insolvenz entfernt das gesamte KI-Unternehmen samt Flugzeugen, Strecken,
  Flugplänen und Personal aus der aktiven Welt. Danach entstehen weder Flüge
  noch Kosten dieser KI. Historische Ergebnisse anderer Unternehmen bleiben.

Spielstand v14 speichert Schonfristbeginn, Restrukturierungsfrist, verbrauchte
Umschuldung und Insolvenzzeitpunkt getrennt vom Kredit. v1–v13 bleiben lesbar:
keine rückwirkende Finanznot, Kosten oder Insolvenz; ein bereits notleidendes
Unternehmen erhält ab dem ersten Simulationsschritt nach Laden die volle
Schonfrist. Bestehende Finanzen und Abflugangebote bleiben unverändert.

## Manuell auf Windows prüfen

Godot, `START.bat`, GUI/Screenshots und Windows-Neustarttests sind auf dem
Linux-Entwicklungsserver nicht verfügbar. Rust-Tests belegen keine UI-Darstellung.

1. Godot schließen, Rust-DLL neu bauen, mit `START.bat` starten. Im Finanzbereich
   Liquiditätsstatus, zusätzliche Regeltexte und deaktivierte Umschuldung prüfen;
   bei schmalem Fenster müssen Texte umbrechen und die Schaltflächen erreichbar sein.
2. Mit einer notleidenden Test-Airline nach 30 Simulationstagen Statuswechsel und
   absolute Frist prüfen. Bei 1×/4×/16× muss dieselbe Spielzeit gelten; Pause darf
   die Frist nicht verändern. Ein Restrukturierungs-Save muss nach Prozessneustart
   dieselbe Frist zeigen.
3. Mit bestehendem Kredit umschulden: Gebühr, Kontostand, Restschuld, gestundete
   Rückstände, kleinere Rate und neues Fälligkeitsdatum vergleichen. Zweiter Klick
   muss gesperrt sein, auch nach Speichern/Neustart. Frist bleibt bei ausbleibender
   Erholung bestehen; bei Erholung verschwindet sie. Verkauf/Rückgabe als Rettung
   ebenfalls prüfen.
4. Ohne Rettung bis Fristende laufen lassen: deutliches Insolvenz-Ende im
   Finanzbereich, eingefrorene Zeit, keine weiteren Flüge/Kosten; Managementaktionen
   und Neugründung müssen abgewiesen werden. Speichern, Prozess neu starten und
   Laden muss denselben Endzustand zeigen.
5. Bei KI-Insolvenz dürfen Marktübersicht, Karte und Operationsansicht keine
   Flugzeuge/Flüge/Strecken der ausgeschiedenen KI zeigen; andere KI läuft weiter.
6. Zusätzlich einen echten v13-Spielstand laden, ohne rückwirkende Buchungen.
   Vorhandene Windows-Prüfungen ausführen (Godot-Pfad entsprechend Installation):

   ```powershell
   Godot --headless --path . --script res://tests/financing_test.gd
   Godot --headless --path . --script res://tests/finances_test.gd
   Godot --headless --path . --script res://tests/competition_test.gd
   Godot --headless --path . --script res://tests/ui_layout_test.gd
   powershell -ExecutionPolicy Bypass -File tests/run_financing_restart_test.ps1
   powershell -ExecutionPolicy Bypass -File tests/run_competition_restart_test.ps1
   START.bat --headless --quit-after 10
   ```
