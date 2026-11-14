# Flugzeugverkauf und Leasing

Alle Regeln liegen in `rust/src/trading.rs`; die Oberfläche zeigt Rust-Angebote
und Fehlermeldungen. Parameter stehen in `config/aircraft_trading.json`.

- Verkauf: aktueller Listenpreis × Altersfaktor × Zustandsfaktor, abgerundet auf
  ganze EUR. Der Altersfaktor startet bei 80 %, sinkt um 5 Prozentpunkte je
  365 Spieltage (anteilig in ganzen Basispunkten) und bleibt mindestens 20 %.
  Der bestehende Wartungszustand skaliert den Wert zwischen 50 % und 100 %.
- Verkauf/Rückgabe: nur geparkt oder in Bodenzeit, ohne beauftragten oder aktiven
  Check. Der Flugplan wird entfernt und die Route bleibt ohne Flugzeug erhalten.
  Verkaufte/zurückgegebene Flugzeuge erscheinen im gemeinsamen Gebrauchtmarkt.
  Rückgaben gehören dort der Leasinggesellschaft. Leasingflugzeuge können nicht verkauft werden.
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
bestehende Kreditregel greift. Keine Subventionen.


## Gemeinsamer Gebrauchtmarkt (Spielstandversion 30)

- Spieler und KI kaufen aus derselben Liste. Ein Kauf entfernt das Angebot sofort;
  die wöchentlichen KI-Prüfungen nutzen die bestehende feste Reihenfolge.
- Preis: `hull_value` × (10000 + `dealer_markup_bps`) / 10000, ganze EUR.
  Der Aufschlag beträgt 5 %. Alter und Wartungszustand werden bei jeder Abfrage
  neu bewertet; geparkte Angebote sammeln keine zusätzlichen Flugstunden/Zyklen.
- Erhalten bleiben ursprüngliches Erwerbsdatum (`acquired_at`), Sitzkonfiguration,
  gesamte Flugstunden/Zyklen und alle Check-Zähler. `purchased_at` speichert den
  Kaufzeitpunkt des neuen Eigentümers. Die lokale Flottenkennung wird neu vergeben.
  Der Kauf nutzt dieselben Liquiditätsprüfungen und das Kaufjournal wie ein Neukauf;
  Lieferung erfolgt sofort an die Heimatbasis. Fällige Wartung bleibt fällig.
- `market_expiry_days` (180 Tage) entfernt Angebote ohne Geldbuchung.
  `market_size_cap` (128) verschrottet beim Einstellen das älteste überschüssige
  Angebot; Angebotskennungen werden nicht wiederverwendet.
- KI-Wachstum behält den bisherigen Neukauf-/Leasing-Budgetentscheid bei.
  Gebrauchtangebote werden gegen diese Alternative desselben Modells anhand
  der Wochenmarge verglichen, einschließlich des auf 104 Wochen verteilten
  Unterschieds der Anschaffungskosten nach Abzug des verbleibenden Rumpfwerts
  (dieselbe Alters-/Zustandsbewertung). Laufende Wartung,
  bereits aufgelaufene Check-Kosten, Personal, Leasingraten und vier Wochen
  Betriebsreserve zählen mit. Bereits wartungsfällige Angebote sowie abweichende
  Business-Kabinen werden von der KI derzeit nicht zur Expansion gekauft.
- Versionen 1–29 starten mit leerem Markt, ohne Flotten- oder Geldänderung.
  Bei vorhandenen Flugzeugen bedeutet ein fehlendes `purchased_at`, dass kein
  späterer Eigentümerwechsel erfasst wurde; `acquired_at` bleibt unverändert.
- Das Flottenpanel zeigt Rust-Angebote mit Modell, Alter, Zustand, Preis,
  Flugstunden, Zyklen und Verkäufer sowie einer Kaufen-Schaltfläche.
