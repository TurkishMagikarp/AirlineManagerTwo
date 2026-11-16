# Vielfliegerprogramm

Rust verwaltet ein Programm je Airline: `none`, `basic`, `generous`. Die UI zeigt
Generous platzsparend als „Plus“. Alle Raten stehen in `config/loyalty.json`.

- None: keine neuen Prämien oder Aufnahmen. Basic: 1 EUR Economy / 3 EUR Business,
  12 % Aufnahme. Generous: 3 / 8 EUR, 30 % Aufnahme.
- Bei Abflug werden Prämienkosten und Aufnahmezähler anhand der tatsächlich
  zugeteilten Economy-/Business-Passagiere fixiert. Bei Landung bucht der Betreiber
  seine eigenen Kosten und sammelt den Aufnahmezähler für den Tagesabschluss.
  Das gilt auch für Umsteiger und Codeshare-Teilstrecken. Frachter und
  Überführungen haben keine Passagiere und keine Prämien.
- Nach allen Airline-Bewegungen am absoluten Tagesende: Aufnahmen minus 0,1 %
  Abgänge des vorherigen Mitgliederbestands. Beide Divisionen verwenden ganze
  Zahlen mit gespeichertem Rest (Nenner 10.000). Mehrere kleine Flüge werden vor
  der Rundung zusammengezählt. Der Trend ist die letzte tägliche Nettoänderung.
  Ein Wechsel zu None löscht keine Mitglieder und ändert keine laufenden Flüge.
- Der vorhandene gemeinsame Nachfrageverteiler multipliziert das Angebotsgewicht
  mit `1 + 0,06 × min(Mitglieder, 100.000) / 100.000`. Kein eigener Nachfragepass;
  alle Airlines verwenden denselben Snapshot vor dem Tagesabschluss. Ein Bonus
  einer Zweiteilstrecke mittelt die Boni ihrer Betreiber (weiter höchstens 6 %).
  Der Bonus garantiert keine zusätzlichen Passagiere, wenn Angebot oder Nachfrage begrenzen.
- Prämien stehen separat in Flugsummen, Routenbericht, Finanzhistorie und der
  Finanzen-Seite; sie sind in Betriebskosten und Vorsteuerergebnis enthalten.
- KI: wöchentliche Prüfung, Basic ab 10 Mio. EUR verfügbarem Kontostand,
  None bei weniger Geld, Finanznot oder laufender Restrukturierungsfrist.
  Keine Zufallswahl, kein Generous und kein gemeinsames Partnerprogramm.
- v41 → v42 initialisiert alle Betreiber und Routen auf None / null Mitglieder.
  Vergangene und laufende Flüge bekommen null Kosten und null Aufnahmezähler;
  historische Geldbeträge bleiben erhalten. Keine rückwirkende Berechnung.

Ohne Mitglieder oder neue Aufnahmen gibt es keinen zusätzlichen Tages-Event.
None verwendet die bisherigen Gewichte ohne zusätzliche Fließkommaoperationen.
Mitglieder, Bruchteilsreste und eingefrorene Flugergebnisse werden gespeichert;
Schrittgröße, Geschwindigkeit und Neustarts ändern die Regeln nicht.
