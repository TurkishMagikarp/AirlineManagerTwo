# Codeshares (erste Fassung)

Im Routenbildschirm steht **Codeshare / Partnernetz** direkt vor der
Wettbewerbsübersicht. Auswahl, Vorschlag und bestätigte Kündigung gehen an Rust.
Die Tabelle zeigt die eigenen gelandeten Codeshare-Passagiere und den eigenen
Ticketumsatz je Partner, über alle bisherigen Verträge hinweg. Werte bleiben nach
Kündigung erhalten. Verbindungen einer ausgewählten Route nennen den Partner
sowie die erwartete Tagesrate je Richtung; dies ist keine Buchungsgarantie.

## Vertrag und Annahme

Nur der Spieler schlägt Verträge mit vorhandenen KI-Airlines vor. Beide Seiten
brauchen aktive Flugpläne an mindestens einem gemeinsamen Flughafen. Keine Seite
darf insolvent, in Restrukturierung oder finanziell bedrängt sein. Die minimale
Liquidität und das maximale Partnerkontingent stehen in `config/codeshare.json`
(vorläufig: 0 EUR und zwei Partner). Eine KI hat in dieser Fassung höchstens den
Spieler als Partner; es gibt keine KI–KI-Verträge. Rust liefert den Ablehnungsgrund.

Beginn und Kündigung wirken sofort für neue Abflüge, ohne Gebühr und ohne
Kündigungsfrist. Bei Restrukturierung oder Ende einer Seite wird der Vertrag an
der Ereignisgrenze beendet. Bereits beim Abflug fixierte Ergebnisse werden nicht
neu berechnet; sie werden bei Landung unverändert verbucht.

## Gemeinsame Nachfrage

Die bestehende einstufige Umsteigerverteilung wird um Pfade mit je einem Flug
beider Vertragspartner ergänzt. Ein gemeinsamer Eigentümer funktioniert weiterhin
unverändert. Ohne Vertrag sind gemischte Pfade ausgeschlossen, auch bei gleichem
Hub. Es gibt keine Verkettung von Verträgen und keine Verbindung über drei Airlines.

Lokale Passagiere haben Vorrang. Geometrie, Umweggrenze, Mindeststreckenlänge,
Transferabschlag, Economy-Kapazität, gemeinsame Kapazitätskürzung und Ratenquantum
entsprechen `docs/demand.md`. Der gemeinsame Ticketpreis ist die Summe beider
Streckenpreise. Der Anteil des ersten Beins wird nach Entfernung abgerundet; das
zweite erhält den Rest. Beide Airlines bekommen ihre eigenen Passagiere und
Umsatzanteile mit derselben Formel, ohne Bonus oder zusätzliche Zahlung.

Der Reputationsfaktor eines gemischten Pfads ist **das Minimum der beiden
Airline-Faktoren**, einmal für die gesamte Reise. Der Faktor einer einzelnen
Airline wird nicht zweimal multipliziert. Volle lokale Kabinen oder ein zu
kleiner Rundungsrest können trotz aktivem Vertrag null Umsteiger ergeben.

## Speicherung und Caches

Save-Version 38 speichert aktive Verträge samt Beginn und je Partner die
Abflugbelege innerhalb der bestehenden Finanz-/Flughistorie. Alte Spielstände
starten ohne Verträge und mit leeren Belegen; Geld, Passagiere und laufende Flüge
bleiben unverändert. Ein Beleg enthält ausschließlich den eigenen Beinanteil.

Verträge sind Bestandteil der abgeleiteten Angebots- und Topologieschlüssel.
Beginn, Kündigung und automatische Beendigung erhöhen die kommerzielle Revision;
vorbereitete Abflugmärkte und Tages-/Jahresallokationen werden dadurch erneuert.
Reputationsänderungen behalten die bestehende separate Cache-Revision.
Die Caches werden nicht gespeichert. Belege für bereits gestartete Flüge leben
unabhängig davon im unveränderlichen Abflugergebnis.

Nicht enthalten: Allianzen, Interline ohne Vertrag, gemeinsame Treueprogramme,
zeitlich abgestimmte Anschlüsse, Gebühren und reale Vertragsverhandlungen.
