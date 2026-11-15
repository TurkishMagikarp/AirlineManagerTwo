# Weltwirtschaft

Rust besitzt einen weltweiten Konjunkturindex in Basispunkten (10.000 = 100 %).
`config/world_economy.json` enthält Band, wöchentliche Kadenz, Schockgröße,
Rückkehr zum Mittelwert, Segmentreaktionen und Phasenschwellen. Der Start ist
exakt neutral. Ein SplitMix64-Schock ist über Seed und Periodennummer adressiert;
die ganzzahlige Rekurrenz läuft stets in derselben Reihenfolge. Abgeleitete
Periodenwerte werden pro Thread für höchstens vier Seeds zwischengespeichert.
Spielgeschwindigkeit, Framegröße und Abfragefolge ändern den Verlauf nicht.

Standard: 85–112 %, wöchentlich höchstens ±1,4 Prozentpunkte Zufall plus
1/104 der Abweichung zurück zum Normalniveau. Die Rückkehr-Halbwertszeit liegt
bei etwa 72 Wochen; mehrjährige Auf- und Abschwünge entstehen ohne Ereignisse.
Geschäftsnachfrage reagiert mit 1,4, Urlaub mit 0,6 und Bauchfracht mit 1,0:
`Faktor = 1 + (Index / 10.000 - 1) × Reaktion`.
Bei 85 % Index sind das 79 %, 91 % und 85 % Nachfrage. Reaktion 0 ist exakt
neutral. Kosten, Treibstoffmarkt, Gehälter und Flugzeugwerte ändern sich nicht.

Unveränderliche Angebots-Snapshots tragen denselben Markt für alle Eigentümer.
Abflugangebote verwenden die tatsächliche Abflugzeit; Jahresplanung blendet nur
die Saison aus und behält die aktuelle Konjunktur. Umsteiger stammen aus dem
skalierten Ursprungsmarkt. Ihre Tages-Caches unterscheiden zusätzlich die
Konjunkturperiode und sind auf 512 Einträge begrenzt. Eine alte, untertägige
Startzeit wird an ihrer genauen Periodengrenze berücksichtigt.

Save v34 speichert Seed, Startzeit und die zuletzt zugestellte Phasenperiode.
Ältere Saves starten bei ihrer Ladezeit exakt auf 100 %, mit einem festen aus
dem Treibstoff-Seed abgeleiteten Seed. Alle KI-Airlines erhalten denselben
Zeitursprung. Bestehende Flugangebote, Historie und Geldbeträge bleiben erhalten.
Die erste Konjunkturänderung kommt erst eine volle Periode später.

Overview zeigt MetricCard, StatusChip und Tooltip auf Deutsch und Englisch;
die Routenansicht nennt die drei Nachfragefaktoren. Phasenwechsel werden einmal
als Toast zugestellt. Übersprungene Phasen werden zusammengefasst; sie erzeugen
keine Aufmerksamkeit und unterbrechen keinen Zeitsprung. Die gespeicherte
Zustellposition verhindert Wiederholungen nach erneutem Laden.

Wenn die exakte lokale Nachfrage im Tagesverlauf eine zwischengespeicherte
Umsteigequote über die Sitzgrenze schieben würde, berechnet Rust das gesamte
Umsteigernetz für diesen Abflugzeitpunkt neu. So bleiben lokale Priorität,
Umsteigerbeine, Erlösaufteilung und Kabinengrenzen konsistent. Normale Abflüge
nutzen weiter den Tages-Cache; die Konjunktur wird weiterhin nur pro Periode
abgeleitet. Ein siebenjähriger Archiv-/Restart-Test deckt den zuvor überbuchten
KI-Flug am Tag 2232 ab.
