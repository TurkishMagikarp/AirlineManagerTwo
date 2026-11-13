# Live-Flugkarte

Im Tab **Karte** zoomt das Mausrad am Cursor, die linke Maustaste verschiebt.
**− / + / Welt** zoomen beziehungsweise setzen die Ansicht zurück. Anklicken
eines Flugzeugs zeigt Flugnummer, Typ, Kennung, Start, Ziel, Fortschritt und
Ankunft in Spielzeit. Nur tatsächlich fliegende Flugzeuge erscheinen; bei
Landung verschwindet das Symbol. Alle Flughäfen aus dem bestehenden Rust-Katalog
und alle eigenen Routen bleiben sichtbar. Ab Zoomstufe 2 erscheinen ICAO-Labels.

## Datenfluss

`rust/src/map.rs` leitet Großkreispositionen aus `operations::movement`, Flugzeit
und der bestehenden Rust-Uhr ab, einschließlich Sekundenbruchteilen. Kein neuer
gespeicherter Flugzustand, keine zweite Uhr und keine Godot-Tweens.
Der GDExtension-Adapter liefert Flughäfen, abgetastete Großkreisrouten und aktuelle
Flüge. Godot fragt Flüge pro sichtbarem laufendem Frame ab; Routengeometrie beim Öffnen.
Pausierte/unveränderte Flugdaten lösen keinen neuen Zeichenauftrag aus.
`scripts/map_background.gd` hält einmal triangulierte Land-Meshes und zeichnet
nur sichtbare Weltkopien. Der Hintergrund wird nur bei Kamera-/Größenänderung
neu aufgebaut, nicht beim Neuzeichnen der Flugzeuge. Benchmark und Messwerte:
[Qualitätscheck](../README.md#qualitätscheck--langzeittest-13112026).

Flugnummern sind technische Kennungen `AM-<Route>-<Abflugsekunde>`; noch keine
frei wählbaren kommerziellen Flugnummern. Equirektanguläre Projektion:
Großkreislinien werden vor dem Zeichnen entlang des kürzeren Längengradübergangs
entrollt und an den Weltkanten wiederholt. Symbole und Hit-Tests verwenden
dieselbe Projektion und dieselben Weltkopien.

Pause, 1×/4×/16× und Laden verwenden unverändert die autoritative Rust-Uhr.
Die Karte benötigt keine zusätzlichen Spielstandfelder (aktuell Version 9).
Kamera und Auswahl werden nicht gespeichert.

## Kartendaten / Lizenz

`data/world_land.geojson`: Natural Earth **1:110m Land**, unverändert bezogen von
https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_110m_land.geojson
am 12.11.2026. Natural Earth ist Public Domain:
https://www.naturalearthdata.com/about/terms-of-use/
Die Datei liegt lokal im Repository; im Spiel gibt es keine Netzwerkaufrufe.
Nur Landflächen, keine Ländergrenzen, Ortsdaten oder Online-Tiles.

## Prüfungen

```text
cargo test --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --script res://tests/map_test.gd
Godot --path . --script res://tests/map_test.gd -- --capture=docs/screenshots/map.png
```

Rust prüft Großkreis, Datumsgrenze, Endpunkte, antipodale Robustheit, exakte
Abflug-/Landegrenzen, Rückflug, Pause, alle Geschwindigkeiten und identische
Positionen nach Speichern/Laden. Godot prüft echte Mausereignisse für Navigation,
Flugzeugauswahl (auch nach Zoom/Pan), cursorzentrierten Zoom, Verschieben,
Flugdetails, Speichern/Laden, 1×/4×/16× und das kleine Fenster (680×520).
Der bestehende Flug-Neustarttest prüft die Kartenposition im frischen Prozess.
Persönliche Spielstände bleiben unberührt.

Verifiziert unter Windows/Godot 4.7.2: 39 Rust-Tests, neun Godot-Testskripte,
alle drei Neustarttests sowie `START.bat --headless --quit-after 10`.
Echter Renderer-Screenshot: [Karte](screenshots/map.png).

Bewusst einfach: acht vorhandene Flughäfen, keine Bodenflugzeuge, kein Wetter,
keine 3D-Darstellung, keine Flugnummernverwaltung und keine gespeicherte Kamera.
Kleine Fenster scrollen den Karteninhalt; Zeitsteuerung bleibt erreichbar.
