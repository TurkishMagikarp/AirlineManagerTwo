# Personal (Version 1)

Alle Regeln stehen in Rust (`rust/src/staff.rs`), alle Werte in `config/staff.json`.
Spieler und KI-Airlines nutzen exakt dieselben Funktionen.

## Rollen und Bedarf
Piloten, Flugbegleiter, Techniker. Jeder Mitarbeiter hat Rolle, Gehalt (je 30-Tage-Zeitraum) und einen
abgeleiteten Status: *Verfügbar*, *Im Flugdienst* (Pilot/Flugbegleiter) oder *In der Wartung* (Techniker).
Der Bedarf ist pro Flugzeugmodell konfiguriert (`models`). Es gibt bewusst keine Einzelqualifikationen,
Schichtpläne, Gewerkschaften oder Schulungen: Personal wird als Pool je Rolle gezählt.

## Regeln
- **Flüge:** Ein Flugplan *reserviert* die Piloten und Flugbegleiter seines Flugzeugmodells, solange er
  besteht – auch während er für eine Wartung ruht und danach automatisch fortgesetzt wird. Ohne genug
  unreserviertes Personal lässt sich kein Flugplan aktivieren („Besatzung fehlt …“), also startet kein Flug.
- **Wartung:** Ein Hangaraufenthalt braucht die Techniker des Modells. Fehlen freie Techniker, steht das
  Flugzeug am Boden in der Wartung und *wartet*; die Wartungsdauer beginnt erst, wenn genug Techniker frei
  sind (Einstellung oder Ende einer anderen Wartung). Wartende Flugzeuge erzeugen kein Zeitereignis und
  halten die Simulation nicht auf.
- **Entlassen:** Nur verfügbare Mitarbeiter; wer im Einsatz ist, kann nicht entlassen werden. Bei der Entlassung
  wird das bis dahin angefallene Gehalt anteilig gezahlt.
- **Einstellen:** Sofort, atomar, begrenzt (`max_hire_batch`, `max_employees`); das erste Monatsgehalt muss
  als Guthaben vorhanden sein, es wird aber nicht im Voraus abgebucht.

## Gehälter
An jeder Zahlungsgrenze (Vielfache von `payroll_period_days`, ganze Sekunden, wie Wartungs- und Prüfungs-
ereignisse) wird das aufgelaufene Gehalt aller Mitarbeiter abgebucht; wer mitten im Zeitraum eingestellt wurde,
erhält nur den anteiligen Betrag. Die Zahlungen stehen in `history.payroll` (wie Wartungskosten) und werden in
der Kontostand-Validierung und im Finanz-Tab („Gehälter insgesamt“) berücksichtigt. Das Ergebnis hängt
nicht von Schrittweite, Geschwindigkeit (1×/4×/16×) oder Speichern/Laden ab.

Hinweis: Die bestehenden Flugkosten enthalten weiterhin `crew_eur_per_hour` (jetzt als Flugzulage für die
Besatzung zu lesen); die Grundgehälter kommen zusätzlich über die Gehaltsabrechnung hinzu.

## KI-Airlines
Beim Start stellen sie Besatzung und Techniker ein. Vor jeder Expansion bzw. jedem Wiedereinstieg stellen sie
fehlendes Personal ein (bricht ab, wenn das Geld fehlt) und rechnen die Wochengehälter in die erwartete
Rentabilität ein. Nach jeder wöchentlichen Prüfung entlassen sie Personal ohne Einsatz; die Techniker für die
Flotte bleiben.

## Spielstände
Version 11. Ältere Spielstände laden weiterhin: Beim Laden wird die Mindestbesatzung für bereits bestehende
Flugpläne und die Technikerzahl für die Flotte eingestellt (ab dem Ladezeitpunkt, ohne Nachzahlung), damit
nichts stehen bleibt. Wartende Wartungen sind additiv (`staffed`, Standard `true`).

## Noch nicht enthalten
Individuelle Qualifikationen/Musterberechtigungen, Schulungen, Schichten und Ruhezeiten, Gehaltsverhandlung
oder -erhöhungen, Fluktuation, Einstellungsverzögerung, Kündigungsfristen, Personalmangel *während* eines
laufenden Flugplans (Personal im Einsatz ist gesperrt, statt Flüge zu kippen).
