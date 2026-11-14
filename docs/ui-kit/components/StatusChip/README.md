# StatusChip

Names the state of an aircraft, route, flight or staff member in one or two words. The consumer provides the word and one of five kinds.

- **go** (circle): running as planned. **info** (diamond): in progress, airborne. **neutral** (dash): idle states such as Bodenzeit and Geparkt. **caution** (triangle): needs attention soon. **stop** (square): blocked.
- The glyph differs per kind and the state is always written out, so the chip reads without colour.
- Text is `code` style at 11px, uppercase. Never put a number alone in a chip.

Godot: a `PanelContainer` with an `HBoxContainer` of a small drawn glyph and a `Label`; five theme variations `ChipGo`, `ChipInfo`, `ChipNeutral`, `ChipCaution`, `ChipStop`.
