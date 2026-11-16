# Zusatzgebühren

Die airlineweite Richtlinie steht unter **Finanzen → Zusatzgebühren**. Sie gilt
für alle eigenen Strecken und neu angelegten Routen. Rust liest die Raten aus
`config/ancillaries.json`:

| Richtlinie | Economy je Passagier | Business je Passagier | Marktgewicht |
|---|---:|---:|---:|
| Inklusive | 0 EUR | 0 EUR | 100 % |
| Standardgebühren | 8 EUR | 4 EUR | 98 % |
| Aggressive Gebühren | 20 EUR | 10 EUR | 94 % |

Gepäck und Sitzplatzwahl sind zusammengefasst. Einnahmen sind ganzzahlige EUR:
`(Passagiere − Business) × Economygebühr + Business × Businessgebühr`.
Die endgültigen, bereits gerundeten Passagierzahlen des bestehenden Verteilers
werden verwendet; es gibt keinen zweiten Nachfragepass und keine weitere Rundung.
Der Attraktivitätsfaktor multipliziert das Auswahlgewicht vor Sitzüberlauf.
Bei Umsteigeverbindungen wirken die Faktoren beider Abschnitte auf das
Reisegewicht. Eine Senkung des Gewichts ist keine garantierte proportionale
Senkung der Passagierzahl.

Jeder Betreiber berechnet seine eigene Gebühr je geflogenem Abschnitt,
auch für Codeshare-Umsteiger. Die Codeshare-Ticketaufteilung bleibt unverändert.
Frachter und Überführungen haben keine Passagiergebühren.

Abflug fixiert Betrag und Ergebnis; die Landung bucht die Einnahmen einmal.
Richtlinienwechsel und spätere Konfigurationsänderungen ändern keine bereits
abgeflogenen Flüge, auch nach Laden. Zusatzumsätze erscheinen getrennt in
Finanzen, Streckenbericht, Flugdetails und Finanzarchiv. Sie zählen zum
Betriebsumsatz für Steuern, Meilensteine und die Gewinnzahlen der Preisberatung.
Die Preisberatung verändert weiterhin ausschließlich Ticketpreise.

KI-Airlines prüfen wöchentlich nach ihren Liquiditätsmaßnahmen: negatives
Guthaben führt zu Standardgebühren, sonst Inklusive. Auch im weiterhin
notleidenden Fall vor dem frühen Review-Ende wird die Richtlinie geprüft.
Die Regel ist deterministisch, verwendet keine Zufallszahlen und führt
keinen zusätzlichen Optimierungslauf aus. Aggressive Gebühren wählt nur der Spieler.

Save v41 speichert Richtlinie und unveränderliche Zusatzerlöse. v40 und ältere
Saves starten für alle Betreiber mit Inklusive und null Zusatzumsatz in allen
historischen und laufenden Ergebnissen. Es erfolgt keine Nachbuchung. Die
neutrale Richtlinie lässt die bisherige Gleitkomma-Reihenfolge unverändert.
