# Bauchfracht

Passagierflugzeuge befördern zusätzlich Fracht, ohne Frachter oder Verträge.
`data/aircraft_models.json` enthält Bauchfrachtkapazität und vereinfachtes
Gesamtnutzlastlimit in kg. Passagiere einschließlich Gepäck verbrauchen zuerst
Nutzlast; Fracht nutzt das Minimum aus verbleibender Nutzlast und Frachtraum.
Eine volle Economy-Kabine lässt bei den ersten drei Modellen keine Nutzlast übrig.

Rust liest die Regeln aus `config/cargo.json`. Der Standardpreis beträgt
1,50 EUR/kg. Ein ganzjähriger Basismarkt von 0,5 kg je potenziellem Passagier
verwendet dieselbe Flughafen-, Größen- und Entfernungsformel wie Passagiere.
Dieser Tagesmarkt gilt je Richtung und wird zwischen allen aktiven Diensten
aller Airlines des Flughafenpaars geteilt, auch bei umgekehrter Routendefinition.
Die günstigste verfügbare Rate bestimmt die marktweite Preisbereitschaft.
Frequenz und lineare Preisbereitschaft gewichten die Zuteilung; freie Kapazität
kann den Überhang anderer Dienste aufnehmen. Hohe Preise erhalten keine Nachfrage;
bei 6 EUR/kg fällt die Nachfrage auf null. Nicht bediente Nachfrage verfällt.
Seltene Flüge sammeln keine Nachfrage mehrerer Tage. Wartung und verkaufte
Flugzeuge erzeugen keine Abflüge oder Frachtumsätze.

Preise werden in Cent/kg gespeichert, Mengen auf ganze kg abgerundet und
Umsätze je Flug auf ganze EUR abgerundet. Die gemeinsame Umsatzsumme enthält
Tickets und Fracht; `cargo_revenue` führt Fracht zusätzlich separat, sodass
Ticketumsatz = Gesamtumsatz minus Frachtumsatz bleibt. Beide zählen zum Gewinn
und zur Ertragsteuer. Historie und Finanzen buchen erst bei Landung. Abflugwerte
bleiben einschließlich Fracht unverändert, wenn sich Preise unterwegs ändern.
KI-Airlines verwenden dieselben Regeln und behalten den Standardpreis. Die
Frachtparameter sind auf einen kleinen Zusatz zum Ticketgeschäft ausgelegt.

Speicherversion 22: Fehlende Frachtpreise alter Routen erhalten den Standardwert;
fehlende Frachtwerte alter Abflugangebote und Historieneinträge bleiben null.
Es gibt keine rückwirkenden Einnahmen. Der bestehende Ereignistakt und die
versionierten Abflugangebote sichern identische Ergebnisse bei 1×/4×/16×,
verschiedenen Schrittgrößen und Neustart.

Die Route zeigt Preis und bisherige kg/Umsätze. Finanzen zeigt Ticket- und
Frachtumsatz getrennt sowie Fracht je Route und Flug. Diagramm-Umsätze enthalten
beide Erlösarten. GDScript rendert Rust-Werte und reicht Eingaben weiter.

## Verifikation

Rust-Tests: Nutzlast bei voller Kabine, Konkurrenz und Überhang, Preisgrenzen,
Preiswechsel im Flug, Wartung/Verkauf, Migration und zwei Jahre KI-Betrieb mit
Geschwindigkeits-, Schrittgrößen-, Steuer- und Neustartvergleich. Ein gemeinsamer
Passagier-Allokationslauf liefert sämtliche Nutzlastwerte; ein Vergleichstest
prüft die Gleichheit mit einzelnen Angeboten bei gleichen lokalen IDs,
verschiedenen Modellen, Kabinen, Preisen, Frequenzen und Jahreszeiten.
Der gemeinsame Angebots-Arbeitsbereich wird pro unverändertem Angebotsstand
vorbereitet: Flughafen-/Preisfaktoren, Kabinenreihenfolge, Fracht-Basismenge,
Frequenzen und Modellreferenzen werden wiederverwendet.
Jeder Abflug berechnet weiterhin saisonale Passagierlasten, freie Nutzlast und
Überhang neu. Der Allokationspuffer wird ohne neue Speicherallokation befüllt;
der Cache wird weder gespeichert noch über Firmenereignisse hinweg verwendet.
Ein unabhängiger Vergleich mit dem ursprünglichen Passagier-Allokator prüft
identische Ergebnisse für zwei saisonale Jahre einschließlich Premiumkabinen,
Preisen, Bekanntheit und seltenen Flügen. Ein weiterer Regressionstest vergleicht
wiederverwendete und frische Angebote bei
wechselnden Jahreszeiten, vollen/leeren Kabinen und umgekehrten Richtungen.

Godot/GUI, Screenshots, START.bat und Windows-Neustarttests sind auf dem
Linux-Server nicht ausführbar. Offene manuelle Prüfung: Route auswählen,
Frachtpreis ändern, während des Fluges erneut ändern, unveränderte Landung
prüfen, nächste Landung und getrennte Finanzzeilen vergleichen; Layout und
Lesbarkeit bei kleinen Fenstern prüfen und Screenshots anfertigen.
