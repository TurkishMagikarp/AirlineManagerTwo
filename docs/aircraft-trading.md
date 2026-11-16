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

## Herstellerbestellungen (Spielstandversion 39)

Die zusätzliche Beschaffungsart in **Flotte → Beschaffung** reserviert eine oder
mehrere fabrikneue Maschinen. Sofortkauf, Leasing und Gebrauchtmarkt bleiben
verfügbar. Regeln stehen in `rust/src/orders.rs`, alle vorläufigen Werte in
`config/manufacturer_orders.json`.

- Der gemeinsame Katalogpreis wird je Stück um 5 % (ab einer), 7,5 % (ab drei)
  beziehungsweise 10 % (ab fünf Maschinen) reduziert und auf ganze EUR
  abgerundet. Die heutige Schwierigkeit verändert den Katalogpreis nicht.
  20 % des rabattierten Stückpreises werden als Anzahlung abgerundet; die
  Restzahlung ist exakt Stückpreis minus Anzahlung.
- ATR-Passagiermaschinen kommen nach 30 Tagen, schmale Passagiermaschinen nach
  60, Großraumflugzeuge nach 120–150 Tagen; Frachter nach 60–150 Tagen.
  Lieferungen erfolgen am ersten Mitternachtstermin nach der Mindestvorlaufzeit, weitere
  Maschinen derselben Bestellung im Abstand von sieben Tagen. Maximal zwölf
  Positionen dürfen offen sein. Das Angebot zeigt sämtliche Termine vorab.
- Bei Lieferung wird die Restzahlung abgebucht. Die Maschine ist neu, geparkt
  und an der Heimatbasis; Personal, Versicherung, Wartung und Zulassung eines
  Flugplans folgen denselben Regeln wie beim Sofortkauf. Ein voller Slotbestand
  verhindert den Erwerb nicht, aber weiterhin die Aufnahme neuer Flüge.
- Reicht das Guthaben nicht, erscheint eine Meldung in der Aufmerksamkeitsliste
  und ein Zeitsprung hält an. Jeden folgenden Mitternachtstermin wird erneut
  geprüft, einschließlich des siebten Tags. Bleibt die Zahlung aus, entfällt
  die Position ohne Erstattung. Auch manuelle Stornierung, Insolvenz und
  Restrukturierung verlieren die Anzahlung. Keine Weiterverkäufe von Positionen.
- Anzahlung und Restzahlung erscheinen als Flugzeuginvestitionen im bestehenden
  Kaufjournal; operative Gewinne werden dadurch nicht verändert. Ein Storno
  erzeugt keinen zweiten Aufwand und keine Rückzahlung.
- Gesunde KI-Airlines ersetzen den ansonsten voll finanzierbaren Neukauf der
  dritten Maschine durch eine einzelne Herstellerbestellung.
  Die normale Betriebsreserve muss zusätzlich zum vollen Kaufpreis reichen;
  während eine Position offen ist, erfolgt kein weiterer Flottenausbau. Nach
  Lieferung bewertet die normale Wochenprüfung den Einsatz der geparkten
  Maschine. Keine Sonderpreise und keine Subventionen.
- Versionen 1–38 laden für alle Eigentümer ohne offene Positionen. Guthaben,
  bestehende Maschinen und historische Zahlungen bleiben erhalten. Offene
  Positionen speichern Preis, Anzahlung, Termin und Zahlungsrückstand.
