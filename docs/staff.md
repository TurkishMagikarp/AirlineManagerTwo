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
Mehrere Berechtigungen pro Pilot, Schichten und Ruhezeiten, individuelle
Gehaltsverhandlungen, Einstellungsverzögerung und Kündigungsfristen.
Manuelles Entlassen von Personal im Einsatz bleibt gesperrt; freiwillige
Kündigungen können einen laufenden Flugplan bis zur Nachbesetzung anhalten.

## Gehaltsstufen und Arbeitskampf (AIR-244)

Jede Rolle hat eine Gehaltsstufe von 85–120 % des Marktgehalts (Start: 100 %)
und Moral von 0–100 (Start: 75). Alle Grenzen und Reaktionswerte stehen unter
`labour` in `config/staff.json`. Der Personal-Tab zeigt die Kosten und die
Differenz für den aktuellen Personalbestand vor der Bestätigung. Eine Änderung
wird an der nächsten 30-Tage-Grenze wirksam: die endende Periode wird vollständig
zum alten Satz abgerechnet, danach entsteht Gehalt zum neuen Satz. Bruchteile
bleiben auch bei einem Satzwechsel erhalten. Die Finanzhistorie enthält weiter
alle tatsächlichen Gehaltsbuchungen.

Moral bewegt sich ausschließlich am Zahltag um höchstens 10 Punkte zum Ziel:
bei 85 % Gehalt 20, bei Marktgehalt 75, bei 120 % Gehalt 95 (linear dazwischen).
Personalmangel, nicht durch Guthaben gedeckte Gehälter und Restrukturierung
senken das Ziel um 15, 25 bzw. 20 Punkte. Es gibt keinen Zufall und keinen
zusätzlichen Nachfrage- oder Pünktlichkeitsbonus. Unter 40 Moral kündigen 5 %
(aufgerundet) der Gruppe, in absteigender Mitarbeiter-ID. Auch gebundene und
musterberechtigte Piloten können kündigen. Bereits gestartete Flüge landen
regulär; fehlende Besatzung blockiert weitere Abflüge bis zur Nachbesetzung.

Unter 25 Moral wird ein Streik mit zehn Tagen Vorlauf angekündigt. Eine
vorgemerkte Gehaltsstufe mit einem Moralziel von mindestens 60 wendet die Warnung
ab. Bleibt die Moral am angekündigten Datum unter 25, streikt die Gruppe drei
Tage. Danach gilt eine 90-Tage-Sperre für neue Streiks. Warnung und Streik stehen
in der Aufmerksamkeitsliste und halten einen Zeitsprung an. Flugpläne behalten
ihre Route und Position; ein datierter Abflugstopp im bestehenden Störungscursor
verschiebt den nächsten Abflug. Ein schon gestarteter Flug behält sein
unverändertes Abflugergebnis. Frachtflugzeuge benötigen kein Kabinenpersonal.

KI-Airlines verwenden denselben Personalzustand und dieselben Regeln. Am
Zahltag merken sie bei Finanznot 95 % vor, sonst 100 %. Unter 60 Moral stellen
sie auch in Finanznot wieder Marktgehalt ein. So erzeugt die Sparpolitik allein
keine Kündigungs- oder Streikspirale; ungedeckte Gehälter und Restrukturierung
können trotzdem zu Arbeitskampf führen. Migration v39 → v40 initialisiert jede
Airline mit Marktgehalt und neutraler Moral, ohne rückwirkende Buchungen.
