# Regionale Weltereignisse

Konfiguration: `config/world_events.json` (alle Zahlen), Regeln: `rust/src/world_events.rs`.

## Arten
- **Flughafen-Boom**: ein Flughafen, 5–14 Tage, Nachfrage von und nach dort +15…35 %, 21 Tage vorher angekündigt.
- **Länder-Flaute**: alle Flughäfen eines Landes, 3–8 Wochen, Nachfrage −10…25 %, ohne Vorwarnung.
- Eigene Reaktion je Segment (`reactions`: Geschäft, Urlaub, Fracht). Reaktion 0 oder `enabled: false` ist exakt neutral.

## Ableitung
Ereignisse sind eine reine Funktion aus Spielstand-Seed, Art, Bahn (`lane`) und absolutem Zeitfenster (`slot`); es wird kein Zufallszustand gespeichert. Ein Zeitfenster ist länger als das längste Ereignis, daher überlappen Ereignisse einer Bahn nie; gleichzeitig aktiv sind höchstens `lanes` je Art. Ziele werden nach Flughafengröße gewichtet (`size_weight_power`). Start und Ende liegen auf Tagesgrenzen, das Ergebnis hängt weder von Spielgeschwindigkeit, Schrittweite, Zeitsprüngen noch Laden ab. Je Tag wird die Liste einmal berechnet und zwischengespeichert; pro Flug wird nichts abgeleitet.

## Markt
Der Ereignisfaktor multipliziert den gemeinsamen Markt wie Konjunktur und Saison (lokal, Umstieg, Fracht) in den vorhandenen Angebots-Snapshots, je Strecke einmal je betroffenem Ereignis. Kosten, Treibstoff, Slots und Flugzeugwerte bleiben unverändert. KI-Airlines erhalten denselben Markt; ihre Jahresplanung ignoriert Ereignisse wie die Saison.

## Speicherstand
v42 → v43: `Cycle.events_from` (Ladezeitpunkt). Alte Spielstände erhalten keine rückwirkenden Ereignisse; das erste kann frühestens nach dem Laden angekündigt werden. Angebote, Historie und Geld bleiben unverändert.

## Oberfläche
- Übersicht: Block „Ereignisse“ (Ort, Art, Zeitraum, Wirkung, Status).
- Streckenansicht: Ereignisfaktor neben Saison und Konjunktur.
- Aufmerksamkeitsliste: Ein Eintrag erscheint, sobald ein angekündigtes oder aktives Ereignis einen Flughafen des Spielers (Heimat oder Streckenende) betrifft. Er ist abgeleitet, nicht gespeichert, und erscheint deshalb nach dem Laden nicht erneut; ein Zeitsprung stoppt nur, wenn er neu hinzukommt.
- Kartenmarkierungen: Flughäfen mit angekündigtem oder aktivem Ereignis (Boom, Flaute, Wetter) tragen einen Ring (Flaute `caution`, Wetter `info`, Boom `go`; gedünnt, solange nur angekündigt). Der Hover-Tooltip nennt Art und Ende, die Flughafenkarte listet die Ereignisse mit Status-Chips.

# Flughafenwetter

Konfiguration: `config/weather.json`, Regeln: `rust/src/weather.rs`. `enabled: false` ist exakt neutral.

- Arten (Schnee, Sturm, Nebel) mit Breitengrad-Band, Jahreszeit (Tag im Jahr, südliche Halbkugel um `southern_shift_days` verschoben), Chance je Zeitfenster, Dauer und Abflugverspätung (`min/max_delay_seconds`).
- Reine Funktion aus Welt-Seed, Flughafen, Art und Zeitfenster wie die Weltereignisse; nichts Zufälliges im Spielstand. 3 Tage Vorwarnung (`warning_days`); Zeitfenster `slot_days` ≥ längste Dauer, je Art keine Überlappung. Mehrere gleichzeitige Arten am selben Flughafen: die stärkste Verspätung gilt.
- Wirkung: Abflüge, deren planmäßige Abflugzeit auf einen Wettertag des Abflughafens fällt, erhalten die Verspätung zusätzlich zur technischen Verspätung (`disruptions::Cursor::next_timed`); Umlauf, Pünktlichkeit, Ruf und Betreuungskosten reagieren wie bei technischen Verspätungen. Keine Ausfälle, keine Umleitungen. KI und Spieler: gleiche Flughäfen, Tage und Verspätungen.
- Pro Tag wird die Vorschau-Liste einmal berechnet (Cache); pro Flug nur eine O(1)-Abfrage (direkt adressierter Tages-Cache).
- Spielstand v43 → v44: `Cycle.weather_from` (Ladezeitpunkt) und `Cursor.weather` (Welt-Seed + Beginn). Alte Spielstände erhalten kein rückwirkendes Wetter.
- Oberfläche: Übersichtsblock „Ereignisse“ (Art „Schnee/Sturm/Nebel“, Wirkung „Abflug bis +N min“), Aufmerksamkeitseintrag „Wetter“ für Heimat- und Streckenflughäfen, „(Wetter)“ als Ursache hinter der Verspätung (Flotte, Einsatzliste), Kartenringe wie oben.
