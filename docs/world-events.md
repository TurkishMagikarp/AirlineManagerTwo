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
- Kartenmarkierungen: nicht umgesetzt.
