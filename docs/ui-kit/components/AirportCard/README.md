# AirportCard

Details of the airport selected on the map. The consumer provides the ICAO code, the city, a status and up to six facts.

- The code is the title, in mono at 24px; the city name sits below in `ink-muted`.
- Facts are label and value pairs, values right-aligned in mono with units.
- One primary action ("Route planen") and at most one secondary.
- The card opens beside the marker, never over it, and closes with Esc or a click on the map.

Godot: a `PanelContainer` positioned from the marker's screen position, clamped to the map rect.
