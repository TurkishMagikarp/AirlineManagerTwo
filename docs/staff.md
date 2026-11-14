# Personal und Musterberechtigungen

Alle Regeln stehen in Rust (`rust/src/staff.rs`), alle Werte in `config/staff.json`.
Spieler und KI-Airlines nutzen exakt dieselben Funktionen.

## Rollen und Bedarf
Piloten, Flugbegleiter, Techniker. Jeder Mitarbeiter hat Rolle, Gehalt (je 30-Tage-Zeitraum) und einen
abgeleiteten Status: *Verfügbar*, *Im Flugdienst* (Pilot/Flugbegleiter) oder *In der Wartung* (Techniker).
Der Bedarf und die Musterberechtigung sind pro Flugzeugmodell konfiguriert (`models`). Jeder Pilot
hat genau eine Berechtigung (A320 family, 737 oder ATR 72); die Reservierung erfolgt je Muster,
in Mitarbeiterreihenfolge. Andere Rollen bleiben Pools ohne Berechtigungen.

Im Personal-Tab wird beim Einstellen das Muster gewählt. Verfügbare Piloten können auf ein anderes
Muster umgeschult werden: `conversion_cost_eur` (25.000 EUR sofort) und `conversion_days` (14 Tage).
Währenddessen bleiben sie angestellt und bezahlt, sind aber weder einsetzbar noch entlassbar.
Nach Abschluss ersetzt das neue Muster das alte; die Rust-Ereignisuhr verarbeitet den Abschluss auch
innerhalb großer Zeitschritte. Ausbildungskosten werden im bestehenden Personalaufwandskonto
`history.payroll` verbucht (die Finanzanzeige fasst Gehälter und Ausbildung dort zusammen).

## Regeln
- **Flüge:** Ein Flugplan *reserviert* die Piloten und Flugbegleiter seines Flugzeugmodells, solange er
  besteht – auch während er für eine Wartung ruht und danach automatisch fortgesetzt wird. Ohne genug
  unreserviertes Personal lässt sich kein Flugplan aktivieren („Besatzung fehlt …“), also startet kein Flug.
- **Wartung:** Ein Hangaraufenthalt braucht die Techniker des Modells. Fehlen freie Techniker, steht das
  Flugzeug am Boden in der Wartung und *wartet*; die Wartungsdauer beginnt erst, wenn genug Techniker frei
  sind (Einstellung oder Ende einer anderen Wartung). Wartende Flugzeuge erzeugen kein Zeitereignis und
  halten die Simulation nicht auf.
- **Entlassen:** Nur verfügbare Mitarbeiter; wer im Einsatz ist, kann nicht entlassen werden. Bei der Entlassung
  wird das bis dahin angefallene Gehalt anteilig auf volle EUR aufgerundet gezahlt (keine kostenlose Kurzzeitbeschäftigung).
- **Einstellen:** Sofort, atomar, begrenzt (`max_hire_batch`, `max_employees`); offene Gehälter plus
  `salary_reserve_periods` Gehaltsperioden des gesamten Bestands inklusive Neueinstellungen müssen
  als Guthaben vorhanden sein. Standard: eine Periode; konfigurierbar von 0 bis 12. Keine Vorausabbuchung.
  Bereits geschuldete Gehälter werden auch bei negativem Guthaben beglichen; Entlassen erlässt keine Schulden.

## Gehälter
An jeder Zahlungsgrenze (Vielfache von `payroll_period_days`, ganze Sekunden, wie Wartungs- und Prüfungs-
ereignisse) wird das aufgelaufene Gehalt aller Mitarbeiter abgebucht; wer mitten im Zeitraum eingestellt wurde,
erhält nur den anteiligen Betrag. Die Zahlungen stehen in `history.payroll` (wie Wartungskosten) und werden in
der Kontostand-Validierung und im Finanz-Tab („Gehälter insgesamt“) berücksichtigt. Das Ergebnis hängt
nicht von Schrittweite, Geschwindigkeit (1×/4×/16×) oder Speichern/Laden ab.

Neue Flugangebote enthalten keine separaten Crewkosten: Personalaufwand entsteht ausschließlich über
Gehälter. Historische Crewkosten und bereits gespeicherte Abflugangebote bleiben unverändert und werden
nicht nachberechnet. Routenberichte enthalten Flugkosten; zentrale Gehälter und Wartungschecks stehen
im Unternehmensergebnis und in dessen Zeitreihe. Gehaltsrundung erfolgt kumulativ seit Einstellung,
damit Zahlungsgrenzen keine Bruchteile mehrfach verwerfen.

## KI-Airlines
Beim Start stellen sie Besatzung und Techniker ein. Vor jeder Expansion bzw. jedem Wiedereinstieg stellen sie
fehlendes Personal ein (bricht ab, wenn das Geld fehlt) und rechnen die Wochengehälter in die erwartete
Rentabilität ein. Nach jeder wöchentlichen Prüfung entlassen sie Personal ohne Einsatz; die Techniker für die
Flotte bleiben.

## Spielstände
Version 25. v24 erhält deterministisch zuerst Berechtigungen für alle laufenden und für Wartung
reservierten Pläne, übrige Piloten das Muster des ersten Flugzeugs (ohne Flotte: erstes sortiertes Muster).
Namen, Gehälter, Geld und Abflugangebote bleiben erhalten. Noch ältere Spielstände laden weiterhin: Beim Laden wird die Mindestbesatzung für bereits bestehende
Flugpläne und die Technikerzahl für die Flotte eingestellt (ab dem Ladezeitpunkt, ohne Nachzahlung), damit
nichts stehen bleibt. Wartende Wartungen sind additiv (`staffed`, Standard `true`).

## Noch nicht enthalten
Mehrere Berechtigungen pro Pilot, Schichten und Ruhezeiten, Gehaltsverhandlung
oder -erhöhungen, Fluktuation, Einstellungsverzögerung, Kündigungsfristen, Personalmangel *während* eines
laufenden Flugplans (Personal im Einsatz ist gesperrt, statt Flüge zu kippen).
