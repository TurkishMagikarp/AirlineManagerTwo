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
