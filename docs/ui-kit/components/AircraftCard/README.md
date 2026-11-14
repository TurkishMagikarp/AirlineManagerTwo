# AircraftCard

One aircraft type on offer. The consumer provides the type name, its class, four facts and the price.

- Type name in the display face at 20px, class as a mono caption.
- Facts are always the same four in the same order: Sitze, Reichweite, Reisegeschwindigkeit, Besatzung.
- The price sits above the Button, in mono, with the unit smaller.
- If the player cannot afford it, the Button is disabled and one `stop` sentence says why.
- Cards share a row and wrap; compare more than four types in a DataTable instead.

Godot: a `PanelContainer` per type in an `HFlowContainer`.
