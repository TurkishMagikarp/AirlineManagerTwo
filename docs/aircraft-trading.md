# Flugzeugverkauf und Leasing

Alle Regeln liegen in `rust/src/trading.rs`; die Oberfläche zeigt Rust-Angebote
und Fehlermeldungen. Parameter stehen in `config/aircraft_trading.json`.

- Verkauf: aktueller Listenpreis × Altersfaktor × Zustandsfaktor, abgerundet auf
  ganze EUR. Der Altersfaktor startet bei 80 %, sinkt um 5 Prozentpunkte je
  365 Spieltage (anteilig in ganzen Basispunkten) und bleibt mindestens 20 %.
  Der bestehende Wartungszustand skaliert den Wert zwischen 50 % und 100 %.
- Verkauf/Rückgabe: nur geparkt oder in Bodenzeit, ohne beauftragten oder aktiven
  Check. Der Flugplan wird entfernt und die Route bleibt ohne Flugzeug erhalten.
  Verkaufte/zurückgegebene Flugzeuge verlassen das Spiel; Kennungen werden nicht
  wiederverwendet. Leasingflugzeuge können nicht verkauft werden.
- Leasing: 5 % Kaution, kein Kaufpreis; 1 % Listenpreis alle 30 Spieltage, erstmals
  nach einer vollen Periode. Kaution, Rate und Periodenlänge werden im Vertrag
  festgeschrieben. Raten laufen auch bei Stillstand und Wartung weiter. Bei
  fehlender Liquidität wird der volle Betrag als negativer Kontostand gebucht;
  die bestehende Finanznot stoppt neue Abflüge, laufende Flüge enden regulär.
- Rückgabe: Kaution zurück, abzüglich einer Zustandsgebühr von bis zu 5 % des
  aktuellen Listenpreises, proportional zum Verschleiß. Die Gebühr ist auf die
  hinterlegte Kaution begrenzt; keine zusätzliche Rückgabeschuld.
- Finanzhistorie: Verkauf, Kaution, Rate und Rückgabe erscheinen als separate
  Geldbewegungen; nur Raten zählen als laufende Kosten. Verkäufe und Kautionen
  verändern den Kontostand, nicht den operativen Flugumsatz. Das Panel zeigt die
  letzten 100 Flugzeugbuchungen; im Spielstand bleibt das vollständige Journal.

Spielstandversion 13 ergänzt Erwerbszeitpunkt und Leasingvertrag. Bei Versionen
1–12 gilt für vorhandene Flugzeuge der Gründungszeitpunkt (Spielsekunde 0).
Die Migration erzeugt keine Leasingverträge, Kosten oder Einnahmen. Neue Käufe
und Leasingverträge verwenden den tatsächlichen Erwerbszeitpunkt.

KI-Airlines nutzen dieselben Transaktionen. Bei knappem Kaufbudget ist Leasing
möglich, sofern die erwartete Wochenmarge nach Wartung, Personal und Leasingrate
positiv genug ist und vier Wochen Bruttokosten als Reserve bleiben. Bestehende
Leasingraten fließen in die Reserve ein. In Finanznot werden ungenutzte geparkte
Maschinen verkauft/zurückgegeben und überzähliges Personal abgebaut, bevor die
bestehende Kreditregel greift. Keine Subventionen oder gebrauchten Märkte.
