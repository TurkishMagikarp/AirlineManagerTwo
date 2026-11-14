# ProgressBar

Shows how far something has come. Two forms: a bar for a share of a whole, and a route leg for a flight between two airports. The consumer provides a value from 0 to 100 and, for the leg, both ICAO codes and the arrival time.

- Bar: 8px track in `surface-sunken`, fill `accent-text` by default; `go` or `stop` only when the value has been judged. Always print the number beside it.
- Leg: origin left, destination right, a 2px `line-strong` track, flown part in `accent-text`, the aircraft as a diamond. A flight that has not departed shows no diamond and `ink-muted` text.

Godot: `ProgressBar` with `background` and `fill` styleboxes; the leg is a small custom `Control` with `_draw`.
