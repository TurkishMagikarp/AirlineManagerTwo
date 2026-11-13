# Mehrere Flugzeuge pro Route

Eine Route kann mehrere eigene Flugzeuge aufnehmen. Der eingestellte Abstand gilt
weiterhin **pro Flugzeug**. Zwei Flugzeuge bei 240 Minuten ergeben zusammen einen
Abflug alle 120 Minuten; jedes Flugzeug pendelt weiter zwischen Start und Ziel.
Der Routen-Tab zeigt die zugewiesenen Flugzeuge und bietet Hinzufügen/Entfernen.

Rust prüft für jedes Flugzeug Reichweite, Standort, Wartungszustand, Besatzung und
Flugzeit plus Bodenzeit. Beim Hinzufügen zu einer aktiven Route werden die Phasen
gleichmäßig auf das Intervall verteilt (bei nicht teilbaren Sekunden mit höchstens
einer Sekunde Unterschied). Laufende Flüge behalten ihre Abflugangebote und enden
inklusive Bodenzeit, bevor ihre zukünftige Phase angepasst wird. Während dieses
Übergangs kann der Abstand vorübergehend ungleichmäßig sein. Ein Wartungsrückkehrer
nimmt seine Phase wieder auf; andere Flugzeuge behalten ihre Pläne.

Entfernen in der Luft merkt die Freigabe nach der Bodenzeit vor. Die bereits
begonnene Landung wird genau einmal abgerechnet. Verkauf und Leasingrückgabe
verwenden die bisherigen Verfügbarkeitsregeln; sie lösen nur die betreffende
Zuweisung. Entfernen, Verkauf und Wartung verschieben die übrigen Flugzeuge nicht.
Damit können nach einer Entfernung Lücken in der Rotation bleiben. Ein gemeinsam
neu aktivierter Flugplan verteilt die verbleibenden Flugzeuge wieder gleichmäßig.

Intern bleibt jeder Flugzeugdienst ein eigener `Route`-Datensatz mit stabiler
Dienstkennung. `parent_route_id` verbindet zusätzliche Dienste mit der sichtbaren
Route. Dadurch bleiben Betrieb, Personal, Wartung, KI und Angebots-/Spillverteilung
auf dem bestehenden Simulationspfad. Die Nachfrage sieht alle aktiven Dienste als
Angebote; kein zusätzlicher Passagiermarkt wird erzeugt. Das Archiv behält die
Dienstkennungen und eine dauerhafte Zuordnung zur sichtbaren Route, auch nach deren
Löschung. Anzeige, Summen und Flugseiten fassen sie wieder zusammen.

Spielstandversion 15 speichert eine sichtbare Route mit `aircraft_ids` und den
zusätzlichen Dienstzuständen unter `rotations`. Der Hauptdienst behält seinen
bisherigen Plan am Routenknoten. Seine Zuweisung ergibt sich aus der Flugzeugliste
abzüglich der zusätzlichen Dienste. Die Validierung prüft beide Darstellungen.
Versionen bis 14 werden beim Laden explizit von `aircraft_id` auf eine Liste mit
null oder einem Flugzeug migriert; das gilt auch für KI-Airlines. Historische
Flugangebote und Finanzbuchungen werden dabei nicht neu berechnet.

KI-Airlines vergleichen bei ihrer bisherigen Expansionsprüfung neue Märkte mit
zusätzlichen Maschinen auf starken Bestandsrouten. `config/competition.json` setzt
die Obergrenze auf zwei Flugzeuge je Route und die Auslastungsschwelle auf 85 %.
Auch nach der Erweiterung muss die erwartete Auslastung die Schwelle erreichen;
der zusätzliche Wochengewinn berücksichtigt Nachfrageverlust der eigenen Dienste,
Wartung, Personal, Leasing, Kaufpreis und die bestehende Liquiditätsreserve.
Zwei schwache Wochenprüfungen (Verlust oder geringe Auslastung) geben zunächst nur
eine Maschine über die gemeinsame Rust-Entfernung frei. Wartung und ausstehende
Freigaben pausieren die Gruppenprüfung. Die letzte Maschine kann später ebenfalls
freigegeben werden. Es gibt keinen neuen Spielstandzustand oder KI-Flugbetrieb.

Noch keine individuellen Tages-/Wochentage oder Überführungen.
Godot-Oberfläche, START.bat und Windows-Neustarttests müssen auf einem passenden
Rechner geprüft werden; Rust-Tests belegen keine visuelle Darstellung.
