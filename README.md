# Airline Manager — Neustart

## Starten

Unter Windows `START.bat` ausführen. Voraussetzung: Godot 4.6+ (getestet mit
4.7.2), Rust über rustup und Visual Studio C++ Build Tools inklusive Windows SDK.
Der Launcher baut die Rust-Bibliothek, importiert die GDExtension und startet Godot.
Godot wird über `AIRLINE_GODOT`, den PATH oder den bisherigen lokalen Pfad
`%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe` gesucht.
Bei Buildfehlern wird das Spiel nicht mit einer veralteten Bibliothek gestartet.
Ein fehlgeschlagener Godot-Erstimport wird einmal wiederholt (hier trat sporadisch
ein Editor-Absturz beim Import-Ende auf); bei erneutem Fehler wird abgebrochen.
Der erste Rust-Build benötigt Internet für die Cargo-Abhängigkeiten; das Spiel läuft offline.

Alternativ `cargo build --locked --manifest-path rust/Cargo.toml`, danach
`project.godot` im Editor öffnen und **F5** starten. Vor einem erneuten Rust-Build
Spiel und Editor schließen (Windows sperrt geladene DLLs). Für Release-Exporte
zuvor mit `cargo build --release --locked --manifest-path rust/Cargo.toml` bauen.
macOS/Linux sind in der Extension konfiguriert, aber noch nicht getestet;
die Bibliothek muss auf der jeweiligen Zielplattform gebaut werden.

## Oberfläche

Übersicht, Flotte, Routen und Finanzen verwenden ein gemeinsames Navy-Operations-Theme mit
Kennzahlen-Karten, Statusfarben und Ergebnistabellen. Neue untergeordnete Controls
erben das Theme automatisch. Bausteine, Layoutregeln und reproduzierbare echte
Godot-Screenshots: [UI-Design](docs/ui-design.md).

## Kleiner Simulationskern

Rust (`rust/src/game.rs`, `rust/src/clock.rs`) besitzt Airline und Spielzeit,
Pause und Geschwindigkeit.
`rust/src/lib.rs` ist nur der godot-rust/GDExtension-Adapter. Godot reicht die
Frame-Dauer und Bedienaktionen weiter und stellt den Rust-Zustand dar. Es gibt
keine zweite Uhr oder Zeitberechnung in GDScript und keine Beispieldaten.

Jede Gründung beginnt pausiert bei **Tag 1 · 00:00:00** (verstrichene Spielzeit,
kein erfundenes Kalenderdatum). **Start / Pause**, **1× / 4× / 16×** steuern die Uhr;
1× entspricht einer Spielsekunde pro realer Sekunde. Diese Stufen sind vorläufige
technische Testwerte, keine endgültige Balancing-Entscheidung. Navigation bleibt
per Maus oder Tab / Umschalt+Tab und Enter / Leertaste bedienbar.

Noch keine Offline-Fortschreibung oder Bedienung zum Springen zu
Ereignissen/Zeitpunkten. Spätere Spielregeln und
Berechnungen gehören in den Rust-Kern, nicht in Godot.

Produktregeln und offene Fragen: [Game Design](docs/game-design/README.md).

## Eigene Airline gründen

Beim Start erscheint der Gründungsbildschirm. Namen eingeben, Heimatland und
einen dort gelegenen Flughafen auswählen, **Airline gründen** drücken. Danach
zeigt die bisherige Übersicht Name, Heimatland, Heimatflughafen und Startkapital.
Alternativ **Spielstand laden** drücken. Namen werden in Rust getrimmt und auf
1–80 Zeichen ohne Steuerzeichen geprüft; auch Land und Flughafen werden dort
validiert. Fehlgeschlagene Gründungen lassen einen bestehenden Zustand unverändert.

**Neues Spiel** öffnet die Gründung erneut. Bis zur Bestätigung bleibt der alte
Zustand erhalten, **Abbrechen** kehrt zurück. Während des Formulars läuft die Zeit
nicht weiter. Erst erfolgreiches Gründen ersetzt die laufende Airline und setzt
die Uhr zurück. Die gespeicherte Datei wird erst durch **Speichern** überschrieben.

Startkapital: `config/new_game.json`, Feld `starting_capital_eur` (nichtnegative,
ganze Euro). Der mitgelieferte Wert **10.000.000 EUR** und die vorläufige
Darstellungswährung EUR sind technische Testkonfiguration, keine beschlossenen
Wirtschafts- oder Länderregeln. Datei ändern, Spiel/Editor schließen und über
`START.bat` neu bauen. Die Einstellung gilt nur für neue Gründungen; geladene
Airlines behalten ihr gespeichertes Guthaben. Flugzeugkäufe werden davon abgezogen;
Flüge verbuchen Ticketeinnahmen und Betriebskosten bei Landung.

Acht echte Flughäfen in Deutschland, Türkei und USA stehen zur Verfügung.
Der [Katalog](data/README.md) liegt getrennt in `data/airports.json` und verwendet
stabile ICAO-/Ländercodes als Grundlage für eine spätere weltweite Datenquelle.

## Flotte und Flugzeugkauf

Im **Flotte**-Tab ein Modell wählen: Preis, Sitzplätze und Referenzreichweite
werden vor dem Kauf angezeigt. **Flugzeug kaufen** kauft sofort ein Exemplar.
Rust prüft die Airline und das Guthaben, vergibt die Kennung und zieht genau den
angezeigten Preis ab. Bei Fehlern bleiben Flotte, Guthaben und Kennungszähler
unverändert; die Meldung wird im Tab angezeigt. Die Übersicht zeigt nun das
verbleibende **Guthaben**, nicht mehr unverändert das ursprüngliche Startkapital.

Gekaufte Flugzeuge gehören der Spieler-Airline, stehen zunächst **geparkt** am
Heimatflughafen und erscheinen einzeln mit Kennung, Typ, ICAO-Standort und Status.
Der Flughafenname und der tatsächlich bezahlte Kaufpreis stehen in den Tooltips.
Die laufenden Kennungen `AM-000001`, `AM-000002`, … sind spielinterne IDs, keine
nationalen Luftfahrzeugregistrierungen. Mehrere Exemplare desselben Modells sind
möglich. Bei kleineren Fenstern ist der Tab scrollbar; in breiten Fenstern stehen
Kaufbereich und eigene Flotte nebeneinander.

Modellangaben: `data/aircraft_models.json` ([Referenzen und Einschränkungen](data/README.md)).
Vorläufige Kaufpreise: `config/aircraft_prices.json`, aktuell A320-200 **8.000.000**,
737-800 **7.000.000**, ATR 72-600 **3.000.000 EUR**. Preise ändern, Spiel/Editor
schließen und mit `START.bat` neu bauen. Historisch bezahlte Kaufpreise werden
pro Flugzeug gespeichert und bei späteren Preisänderungen nicht neu berechnet.
Nach Käufen weiterhin ausdrücklich **Speichern** drücken.

Noch kein Verkauf, Leasing, Bestell-/Lieferprozess, Wartung oder
separates Buchhaltungsjournal. Sofortige Verfügbarkeit und Testpreise sind
vorläufige Vereinfachungen, keine endgültigen Wirtschaftsregeln.

## Routenplanung

Im **Routen**-Tab Start und Ziel aus den acht vorhandenen Flughäfen sowie ein
freies eigenes Flugzeug wählen. Die Vorschau zeigt die Luftlinienentfernung und
die Reichweitenprüfung aus Rust. Gleiche oder unbekannte Flughäfen, fremde bzw.
nicht vorhandene Flugzeuge und Strecken oberhalb der Modellreichweite werden
abgelehnt, ohne Zustand oder Kennungszähler zu ändern.

**Route erstellen** reserviert das Flugzeug exklusiv. Die Liste zeigt Kennung,
Strecke, Entfernung und Flugzeug. Eine Zeile auswählen, dann unten ein freies
Flugzeug wählen und **Zuweisung übernehmen** drücken. **Keine Zuweisung** gibt das
Flugzeug frei und behält die Route; **Route löschen** entfernt die Route und gibt
ihre Zuweisung frei. Die Flotte zeigt reservierte Flugzeuge mit ihrer Routennummer.
Mehrere Verbindungen auf derselben Strecke sind mit verschiedenen Flugzeugen möglich.

Ohne aktivierten Flugplan bleiben Routen reine Planungen und der Standort unverändert.
Noch keine Überführungen; Nachfrage und Wirtschaft siehe unten.
Entfernung: Großkreis mit Haversine und mittlerem Erdradius
6371,0088 km; Reichweite: vereinfachter Modellwert ohne Wetter oder Reserven.
Rust prüft den ungerundeten Wert; die UI zeigt eine Nachkommastelle.
Änderungen wie gewohnt ausdrücklich **Speichern**.

## Erster Flugbetrieb

Im **Routen**-Tab eine Route auswählen, den ersten Abflug **in Spielminuten** und
den **Abstand zwischen Abflügen** eingeben, dann **Flugplan aktivieren** drücken.
Das zugewiesene Flugzeug muss am Startflughafen geparkt sein. Der Flugplan pendelt
automatisch zwischen Start und Ziel; der Abstand gilt für jeden einzelnen Hin-
oder Rückflug, nicht erst für den gesamten Umlauf. Erster Abflug frühestens in
einer Minute; zu knappe Abstände werden ohne Zustandsänderung abgelehnt.

Rust berechnet `ceil(Entfernung / Reisegeschwindigkeit × 3600)` Spielsekunden:
A320 830 km/h, 737-800 840 km/h, ATR 72-600 510 km/h. Nach jeder Landung sind
45 Minuten (Jets) bzw. 30 Minuten (ATR) Bodenzeit vorgeschrieben. Das sind bewusst
vereinfachte Referenzgeschwindigkeiten und vorläufige Bodenzeiten; noch keine
Steig-/Sinkflug-, Roll-, Wetter- oder Verspätungsmodelle.

Die Flotte zeigt **Geparkt**, **Unterwegs** oder **Gelandet · Bodenzeit**. Der
Routen-Tab zeigt aktuelle Flugrichtung, Landetermin/Bodenzeitende, gezählte Landungen
und nach Zeit sortierte nächste Abflüge. Während eines Flugs bezeichnet der
gespeicherte Standort den letzten Abflugort, nicht einen aktuellen Bodenstandort.
Flugpläne lassen sich nur im geparkten Zustand beenden, nicht während Flug oder
Bodenzeit. Vor Zuweisungsänderung oder Löschen zuerst den Flugplan beenden.
Ein neuer Flugplan startet einen neuen Landungszähler; Finanzsummen und Flugergebnisse bleiben erhalten.

Die vollständige Berechnung liegt in `rust/src/operations.rs`. Der Zustand wird
aus ganzzahligen Ereignisgrenzen abgeleitet: Auch über beliebig viele Umläufe
werden sämtliche Landungen gezählt, ohne frameabhängige Abflugprüfungen oder
eine Schleife über jeden Flug. Pause friert den Betrieb ein; 1×/4×/16× nutzen
dieselbe Rust-Uhr. Der neue Tab **Karte** zeigt Flughäfen, Großkreisrouten und
anklickbare Live-Flugzeuge auf einer offline verfügbaren 2D-Weltkarte.
Mausrad zoomt, Ziehen verschiebt; Details und Tests: [Flugkarte](docs/map.md).

## Vereinfachte Wirtschaftssimulation

Im **Routen**-Tab eine Route auswählen, **Ticketpreis** (0–100.000 ganze EUR pro
Passagier und Einzelflug) einstellen und **Ticketpreis übernehmen** drücken.
Neue und migrierte Routen starten mit **100 EUR**. Der Preis gilt für beide
Pendelrichtungen und kann auch bei aktivem Flugplan geändert werden. Bereits
gestartete Flüge behalten ihr gespeichertes Abflugergebnis und ihren alten Preis.

Alle Regeln, Rundungen und Geldbewegungen liegen in `rust/src/economy.rs` und
`rust/src/operations.rs`, Godot stellt nur dar. Kein Zufall, keine Frameabhängigkeit.
Vorläufige, leicht anpassbare Werte: `config/economy.json`; nach Änderungen neu
bauen. Dieselbe Ausgangslage **mit derselben Konfiguration** liefert dasselbe Ergebnis.

Nachfrage: Ein sitzplatzunabhängiger Tagesmarkt je Flughafenpaar und Richtung
berücksichtigt Größe/Bedeutung beider Flughäfen, Entfernung und Geschäfts-/Urlaubsreisen.
Alle aktiven Pendelflugpläne derselben Verbindung teilen ihn nach Preisattraktivität und Frequenz,
auch bei umgekehrter Routenrichtung. Hohe Preise schrecken beide Gruppen ab,
Urlauber früher. Gleichmäßiger Tagesdurchschnitt, kein hartes Kalendertageskontingent.
Die Routenliste zeigt geschätzte Nachfrage/Tag/Richtung und tatsächliche archivierte
Auslastung. [Formeln, Ausgangsdaten und Grenzen](docs/demand.md).

Kosten und Preise:

- Referenzpreis: `60 + floor(4 × km / 100)` EUR.
- Verkaufte Tickets: Minimum aus gemeinsam verteiltem Marktanteil und Sitzplätzen. Ab dem doppelten
  Referenzpreis fliegt das Flugzeug leer, bei 0 EUR sind Tickets kostenlos.
- Einnahmen: Passagiere × Ticketpreis.
- Treibstoff: Reisezeitverbrauch plus fester Start-/Landeanteil, **0,80 EUR/kg**.
- Flughäfen/Abfertigung beider Endpunkte zusammen: **1.400 EUR (Jets) / 600 EUR
  (ATR) + 25 EUR/Passagier**.
- Personal: (Flugzeit + 30 Minuten) × modellabhängiger Stundensatz.
- Wartung/Verwaltung: einfache Zyklus-/Stundenpauschale, kein Wartungssystem.
  Stundenkosten je Kategorie auf volle EUR aufgerundet.

| Modell | Treibstoff kg/h | Personal EUR/h |
| --- | ---: | ---: |
| A320-200 | 2.500 | 650 |
| 737-800 | 2.600 | 650 |
| ATR 72-600 | 750 | 350 |

Kostenreferenzen und Testmatrix: [Wirtschaft](docs/economy.md).
Die Stellwerte der Nachfrage stehen in `config/demand.json` und `data/airports.json`.

Bei jeder **Landung** werden Einnahmen und sämtliche Kosten genau einmal verbucht,
auch für leere Flüge. Unterwegs noch keine Abbuchung, keine Treibstofflagerhaltung.
Negative Kontostände sind erlaubt; Käufe benötigen weiterhin ausreichend Guthaben.
Noch keine Kredite, Zinsen, Insolvenz, Wartungsereignisse, Steuerregeln oder
laufenden Kosten geparkter Flugzeuge; Wartung wird nur pauschal berücksichtigt.

Der Routen-Tab zeigt die **letzten 100 Landungen** einzeln mit Richtung, Zeitpunkt,
Preis, Passagieren, Auslastung, Einnahmen, Kostenaufschlüsselung und Gewinn/Verlust.
Die Unternehmensübersicht zeigt den aktuellen Kontostand sowie kumulierte Flüge,
Passagiere, Auslastung, Einnahmen, Betriebskosten und Betriebsgewinn/-verlust.
Flugzeugkäufe werden separat ausgewiesen. Finanzsummen zählen über die begrenzte
Ergebnisliste hinaus weiter, auch nach Beenden oder Löschen einer Route.
Große Zeitsprünge werden gebündelt abgerechnet, ohne Schleife über alle Flüge.
Pause, 1×/4×/16×, Speichern/Laden und erneute Flugpläne verursachen keine Doppelbuchungen.

## Erste KI-Fluggesellschaften

Zwei fiktive Konkurrenten starten mit eigenem Startkapital, gekauften Flugzeugen,
Flugplänen, Ticketpreisen und separaten Konten/Finanzarchiven:

- **Regional Wings**: Frankfurt–München, ATR 72-600, zunächst 90 EUR.
- **Continental Sky**: Frankfurt–Istanbul IST, A320-200, zunächst 150 EUR.

Beide pendeln zunächst alle sechs Stunden. Spieler und KI nutzen dieselben
Rust-Funktionen für Kauf, Streckenprüfung, Flugbewegung, Nachfrage, Kosten und
Abrechnung. Es gibt keine zusätzlichen Passagiere oder Betriebssubventionen für KI.
Alle Angebote teilen denselben Markt pro Flughafenpaar und Richtung; günstigere
Angebote können Passagiere von Konkurrenten gewinnen, Frequenz beeinflusst den
Tagesanteil und Sitze begrenzen die tatsächlich beförderten Passagiere.

Im **Routen**-Tab eine eigene Route auswählen: Der Wettbewerbsbereich zeigt alle
passenden Angebote, auch umgekehrte Strecken, mit Airline, Flugzeug/Sitzen, Preis,
Abflügen/Tag und geschätzten Pax/Tag/Richtung. Prozentwerte beziehen sich auf die
geschätzten **beförderten** Passagiere, nicht auf den gesamten Basismarkt.
Inaktive Angebote werden als solche angezeigt. Die letzte KI-Entscheidung steht dabei.
Karte, eigene Flotte und Finanzansichten bleiben spielerbezogen. Der Wettbewerbsbereich
zeigt auch ohne ausgewählte eigene Route alle KI-Strecken, Flotten, Standorte,
Kaufzeitpunkte, Guthaben und die letzte Entscheidung.

Alle sieben Spieltage prüft jede KI drei Preise (bisheriger Preis und ±10 EUR)
mit derselben Gewinnberechnung. Zwei aufeinanderfolgende Verlustwochen **je Route**
führen zur Schließung, sobald die Maschine geparkt ist. Flugzeug, Konto und
Finanzarchiv bleiben erhalten; Gewinne anderer Strecken verdecken Verluste nicht.

Alle vier Wochen sucht die KI höchstens eine neue Verbindung: freie geparkte
Maschinen zuerst, sonst ein regulär gekauftes Flugzeug am Heimatflughafen.
Start ist immer der tatsächliche Standort, Reichweite und Flug-/Bodenzeiten gelten
wie beim Spieler. Keine doppelte eigene Verbindung im selben Markt. Kandidaten
werden mit allen konkurrierenden Angeboten bei 30–150 % des Referenzpreises geprüft.
Voraussetzungen: mindestens 5.000 EUR erwarteter Wochengewinn, Kaufamortisation
innerhalb von 104 Wochen und nach dem Kauf mindestens 1 Mio. EUR bzw. vier Wochen
Bruttobetriebskosten Reserve (der höhere Wert gilt). Maximal sechs Flugzeuge je KI.
Nach einer Schließung beginnt die Expansionswartezeit erneut. Alte gespeicherte
stillgelegte Flugpläne behalten ihre bisherige Wiedereinstiegsprüfung.
Diese konservativen Grenzen sind vorläufiges Balancing, keine optimale Strategie.

Vereinfachung: deterministischer Tagesdurchschnitt ohne Einzelbuchungen,
Kapazitätsüberhänge werden nicht auf andere Angebote umverteilt. Wöchentliche
Entscheidungsgrenzen teilen große Zeitsprünge; dazwischen werden Flüge weiter
gebündelt abgerechnet. Keine Schleife pro Flug. Ausstehende Abflugergebnisse
bleiben bei Preis-/Angebotsänderungen unverändert.

## Finanzen und Routencontrolling

Der Tab **Finanzen** zeigt Kontostand, Ticketeinnahmen, Betriebskosten und das
signierte Betriebsergebnis. Käufe sind Investitionsausgaben, keine Betriebskosten.
Einfache Diagramme zeigen Kontostand sowie Einnahmen, Kosten und Ergebnis je
Zeitabschnitt. In der Routenübersicht sind profitable und verlustbringende Routen
farblich markiert. Eine Route anklicken: Auslastungsverlauf und alle archivierten
Landungen mit Ticketpreis, Passagieren, Umsatz, Kosten und Ergebnis erscheinen.
Die Kostenarten stehen im Tooltip jeder Flugzeile. Auch gelöschte Routen bleiben
im Finanzarchiv erhalten. Routen ohne Landungen zeigen ausdrücklich „Keine Flüge“.

Rust speichert ab jetzt jede Landung verlustfrei in `rust/src/history.rs`: Folgen
mit gleichem Preis, Kosten, Kapazität und Flugabstand werden zu einem Block
zusammengefasst. Das sind die tatsächlich bei Landung verbuchten Ergebnisse,
keine neue Wirtschaftslogik und keine Nachberechnung mit heutigen Preisen.
Flugzeugkäufe erhalten Zeitstempel; Kontoverläufe ergeben sich aus Anfangssaldo,
verbuchten Flugergebnissen und Käufen. Bei neuen Airlines beginnt das Archiv
bei Gründung. Bei Spielständen der Versionen 1–6 beginnt es erst beim Laden;
der Beginn wird angezeigt. Bestehende Gesamtsummen, Kontostand und die bisherige
Liste der letzten 100 Landungen bleiben erhalten, fehlende ältere Details werden
nicht erfunden. Gesamtkennzahlen und neues Routenarchiv können deshalb bei alten
Spielständen unterschiedliche Erfassungszeiträume haben.

Die Flugtabelle fordert nur **50 Zeilen pro Seite** aus Rust an; Blättern erzeugt
nicht alle bisherigen Flüge. Diagramme verwenden maximal **64 Zeitpunkte** mit
exakten Kontoständen an diesen Zeitpunkten bzw. summierten Ergebnissen seit dem
vorherigen Zeitpunkt. Dazwischenliegende Kontobewegungen können im Diagramm
ausgelassen werden, bleiben im Archiv erhalten. Auslastung je Zeitabschnitt ist
nach angebotenen Sitzen gewichtet; Abschnitte ohne Landungen bleiben Lücken.
Die sichtbare Finanzseite prüft höchstens zweimal pro realer Sekunde auf neue
Buchungen und baut unveränderte Tabellen nicht neu auf. Versteckte Routen- und
Finanzansichten aktualisieren keine Ergebnistabellen. Die Weltkarte ist unverändert.
Bei kleinen Fenstern bricht die Navigation in zwei Zeilen um.

## Speichern und Laden

**Speichern** ersetzt den einen lokalen Spielstand; **Laden** stellt ihn wieder
her. Nach einem Neustart **Spielstand laden** drücken: Airline einschließlich
Name, Land, Heimatflughafen und verbleibendem Guthaben, die komplette Flotte
einschließlich bezahlter Preise und Kennungszähler, Routen mit Flugzeugzuweisungen
und Routenkennungszähler, Pendelflugpläne samt Landungszählern und aktuellem
Flug-/Bodenzustand sowie Spielzeit (nanosekundengenau),
Pausenstatus und Geschwindigkeit werden wiederhergestellt. Ein laufender
Spielstand läuft anschließend weiter, ein pausierter bleibt pausiert. Die Zeit
während des geschlossenen Spiels wird nicht angerechnet. Es gibt bewusst kein
automatisches Speichern oder Laden.

Datei: `user://savegame.json`, von Godot auf den beschreibbaren Benutzerordner
aufgelöst (unabhängig von Installationsordner und Arbeitsverzeichnis):

- Windows: `%APPDATA%\Godot\app_userdata\<Projektname>\savegame.json`
- macOS: `~/Library/Application Support/Godot/app_userdata/<Projektname>/savegame.json`

Der aktuelle Projektname ist `Airline Manager — UI-Basis`. Der tatsächliche Pfad
steht auch im Tooltip der Speichermeldung. macOS ist hier nicht praktisch getestet.

Rust (`rust/src/save.rs`) übernimmt Dateizugriff, JSON-Format und Validierung.
Das Format hat Version **9** und einen `state`-Bereich mit `clock`, `airline`,
`fleet`, `routes` (einschließlich Ticketpreis und optionalem Flugplan) und `finance`
(Finanzsummen und letzte 100 Flugergebnisse) sowie `history` (Beginn, Anfangssaldo,
vorherige Finanzsummen, verlustfreie Flugblöcke und zeitgestempelte Käufe).
Zusätzlich enthält `competitors` die vollständigen KI-Unternehmenszustände mit
Flotte, Routen, Konten, Abflugergebnissen, Archiv und nächster Entscheidungszeit.
Archivzeiten, Summen und Flugdaten werden beim Laden und Speichern validiert.
Additive KI-Felder speichern Strecken-Verlustprüfungen und Expansionswartezeit;
bisherige Version-9-Spielstände bleiben durch Standardwerte lesbar.
Versionen **1–8** bleiben ladbar. Alte Airline-Spielstände erhalten die beiden
KI-Unternehmen erst zum Ladezeitpunkt, ohne rückwirkende Flüge oder Änderungen
am Spielerkonto; bereits gespeicherte Abflugergebnisse bleiben erhalten.
Laufende Flüge enthalten ihr beim Abflug fixiertes Ergebnis (Passagiere, Einnahmen,
Sitze/Auslastung und Kosten). Änderungen anderer Flugpläne wirken erst auf künftige
Abflüge. Versionen **1–7** erhalten für bereits laufende Flüge einmal beim Laden
eine Schätzung aus dem gespeicherten Angebot; vergangene Ergebnisse bleiben erhalten.
Version **6** übernimmt unverändert ihre Finanzsummen und eröffnet das neue Archiv
am Ladezeitpunkt. Additives Feld `overhead`: bei alten
Version-6-Ergebnissen null. Historische Ergebnisse werden nicht
mit neuen Konfigurationswerten nachberechnet; künftige Abflüge nutzen die aktuelle
Build-Konfiguration. Version **5** behält Flugbetrieb und Kontostand und erhält
Standardticketpreise sowie leere Finanzsummen, ohne vergangene Flüge nachträglich
abzurechnen. Ein bereits laufender Flug wird erst bei seiner nächsten Landung abgerechnet.
Version **4** behält
Routen und Zuweisungen ohne Flugpläne. Widersprüchliche Flugzustände, Zeitgrenzen
oder Landungszähler werden abgelehnt. Version **3** behält Airline und Flotte und erhält eine
leere Routenliste. Ungültige Routen, fehlende Referenzen, Reichweitenüberschreitungen
und doppelte Flugzeugbelegungen werden beim Laden vollständig abgelehnt. Spielstände
der Version **2** behalten ihre Airline und ihr Guthaben und erhalten eine leere
Flotte, ohne zusätzliche Kosten oder erfundene Flugzeuge. Alte Zeit-Spielstände
der Version **1** bleiben ebenfalls lesbar, enthalten jedoch noch keine Airline. Ihre Uhr
wird ohne erfundene Unternehmensdaten geladen; der Gründungsbildschirm erklärt,
dass eine anschließende Gründung ein neues Spiel beginnt. Inkompatible Änderungen
brauchen eine neue Version und gegebenenfalls eine Migration; unbekannte Versionen
werden jetzt abgelehnt. Keine Serialisierung von Godot-Objekten.

Eine vollständig geschriebene und synchronisierte temporäre Datei im selben
Ordner ersetzt erst zum Schluss den bisherigen Spielstand. Fehler beim Laden
(fehlende, defekte, zu große oder ungültige Dateien) erscheinen als Meldung und
verändern die laufende Simulation nicht. Noch keine mehreren Speicherplätze,
Backups oder automatische Wiederherstellung. Ungültige Airline- und Flottendaten werden
ebenso abgelehnt wie defekte Zeitdaten, ohne den laufenden Zustand zu ersetzen.
Das bisherige Dateilimit beträgt 1 MiB; auch Speichern prüft es, damit keine
zu große, anschließend nicht mehr ladbare Datei einen guten Spielstand ersetzt.
Das Limit gilt weiterhin auch mit Archiv: Sehr viele Preis-/Flugplanwechsel können
es erreichen; gleichbleibender Flugbetrieb braucht dagegen auch für Millionen
Flüge nur wenige Archivblöcke. Noch keine Auslagerung des Archivs in eine Datenbank.

## Tests

Nach dem Rust-Build und dem ersten Godot-Import aus diesem Projektordner:

```text
cargo test --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --script res://tests/navigation_test.gd
Godot --headless --path . --script res://tests/simulation_test.gd
Godot --headless --path . --script res://tests/founding_test.gd
Godot --headless --path . --script res://tests/fleet_test.gd
Godot --headless --path . --script res://tests/routes_test.gd
Godot --headless --path . --script res://tests/operations_test.gd
Godot --headless --path . --script res://tests/economy_test.gd
Godot --headless --path . --script res://tests/demand_test.gd
Godot --headless --path . --script res://tests/demand_test.gd -- --restore-demand
Godot --headless --path . --script res://tests/competition_test.gd
Godot --headless --path . --script res://tests/finances_test.gd
Godot --headless --path . --script res://tests/map_test.gd
powershell -ExecutionPolicy Bypass -File tests/run_save_restart_test.ps1
powershell -ExecutionPolicy Bypass -File tests/run_flight_restart_test.ps1
powershell -ExecutionPolicy Bypass -File tests/run_economy_restart_test.ps1
powershell -ExecutionPolicy Bypass -File tests/run_finances_restart_test.ps1
powershell -ExecutionPolicy Bypass -File tests/run_competition_restart_test.ps1
```

`Godot` durch den eigenen Executable-Pfad ersetzen. Der Verbindungstest lädt die
echte DLL, klickt die tatsächlichen UI-Buttons und prüft auch den Frame-Betrieb.
Die Godot-Tests können ohne `--headless` mit echtem Fenster ausgeführt werden.
`START.bat --headless --quit-after 10` prüft den gesamten Startpfad.

Der Neustarttest startet vier vollständig getrennte Godot-Prozesse: jeweils
Speichern und Wiederherstellen eines laufenden und eines pausierten Spielstands.
Er gründet eine Airline über die UI, klickt die echten Buttons und prüft nach
vollständigem Neustart Airline, Flotte, Routen, exklusive Zuweisungen, Guthaben, Kennungen, exakte Zeit und
Geschwindigkeit. Er prüft auch einen weiteren Kauf nach dem Laden, unzureichendes
Guthaben und defekte Spielstände. Rust-Tests decken zusätzlich Käufe aller Modelle,
exaktes Restguthaben null, alte Spielstände und fehlerhafte Flottendaten ab.
Routentests prüfen zusätzlich ungültige Strecken, Reichweitenüberschreitungen,
doppelte Zuweisungen, Wechsel/Freigabe/Löschen und die Bedienung im kleinen Fenster.
Die Testdateien liegen isoliert im Temp-Ordner, nicht im persönlichen Spielstand.
Wirtschaftstests prüfen verschiedene Ticketpreise, volle/teilweise/leere Flüge,
Sitzplatzgrenzen, positive und negative Ergebnisse, exakte Kontobewegungen,
negative Kontostände, ungültige Preise, Finanzüberläufe ohne Teilbuchungen,
gleichzeitige Routen, 16× mit kleinen/großen Schritten und eine Million Landungen.
Zusätzlich: Finanzmigration aus Version 5 ohne Nachberechnung, begrenzte Historie,
Routenlöschung/Neugründung sowie Speichern/Laden und Fortsetzung in getrennten Prozessen.
Der neue Finanzarchivtest prüft zusätzlich zwei Millionen echte Landungen auf zwei
Routen, profitable und verlustbringende Ergebnisse, nur zwei Archivblöcke,
Diagramm-/Kontenabgleich, 50-Zeilen-Seiten bis zum ältesten Flug, unveränderte
Tabellen ohne neue Buchungen und Layouts bis 680×520. Der separate Neustarttest
lädt dieses Archiv in einem neuen Godot-Prozess und setzt es ohne Doppelbuchung
fort. Rust prüft außerdem Preis-/Auslastungswechsel, identische Archive bei
kleinen/großen 16×-Schritten, Migration aus Version 6 und ungültige Archivdaten.
Vor der KI-Erweiterung verifiziert: 54 Rust-Tests, Clippy ohne Warnungen, Nachfrage-UI samt separatem
Ladeprozess, bestehende Godot-Tests und alle vier
Prozess-Neustarttests sowie `START.bat --headless --quit-after 10` erfolgreich.
Reale Grafikprüfung bei 1440×1000, 1100×720 und 680×520 erfolgreich. Gemessen im
Debug-Build auf diesem Rechner: 100 Abfragen neuester/ältester Flugseiten aus einer
Million Flüge ca. 59–72 ms insgesamt (kein Benchmark der Weltkarte).
Optional `-Godot "C:\Pfad\Godot.exe"` angeben. Auf macOS kann dasselbe GDScript
in separaten Prozessen mit `--save-test-mode=write-running` / `read-running` /
`write-paused` / `read-paused` und `--save-test-path=/absoluter/testpfad.json`
nach dem `--`-Trenner aufgerufen werden.

KI-Erweiterung verifiziert: **61 Rust-Tests**, Clippy ohne Warnungen, alle zehn
bisherigen Godot-Tests, der neue Wettbewerbs-UI-Test und alle fünf
Prozess-Neustarttests. `START.bat --headless --quit-after 10` funktioniert weiter.
Die neue Testmatrix prüft gemeinsame Nachfrage bei gemischten Preisen und
Frequenzen, Sitzgrenzen, Preiswettbewerb, getrennte Konten, wöchentliche
Preisentscheidungen/Stilllegung, unveränderte laufende Flüge, Altspielstand-Migration,
beschädigte KI-Daten, transaktionale Fehler sowie identische Ergebnisse bei
großen/kleinen Schritten und 1×/4×/16×. Der Millionentest mit zwei Spielerrouten
bleibt kompakt; tatsächliche KI-Marktänderungen erzeugen zusätzliche Archivblöcke.

KI-Wiedereinstieg verifiziert: **62 Rust-Tests** (inkl. Wiederaufnahme nach
Stilllegung und Warten bei unrentablem Markt), Clippy ohne Warnungen, alle
Godot-Tests und alle fünf Prozess-Neustarttests.

KI-Expansion verifiziert: **66 Rust-Tests**, Clippy ohne Warnungen, elf Godot-Tests
und alle fünf Prozess-Neustarttests. Neu geprüft: echte Käufe und Kontenabgleich,
freie Maschinen statt Neukauf, begrenztes Budget, streckenbezogene Schließungen
mit erhaltenem Archiv, gemeinsamer Passagiermarkt, Flottenlimit nach fünf Jahren,
kompakter speicherbarer Langzeitstand und alte Version-9-KI-Felder. Der Godot-Test
prüft zusätzlich Expansion über ein Jahr und das Laden vergrößerter Flotten.
`START.bat --headless --quit-after 10` weiterhin erfolgreich. Noch keine Überführungen,
Flugzeugverkäufe, Kredite, Insolvenz oder globale Netz-/Frequenzoptimierung;
KI-Entscheidungen bleiben bewusst eine kleine deterministische Heuristik.
