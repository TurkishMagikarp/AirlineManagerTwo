# ClockBar

The fixed top bar: game date and time, speed, cash and save or load. It stays outside the scrolling content. The consumer provides the clock snapshot from Rust; nothing here keeps its own time.

- Date and time are a split-flap readout: one `flap` cell per character, `flap-ink` digits in `figure` weight, a `flap-seam` hinge line. The board stays dark in both themes.
- Format: weekday and date in the active language (`SA 14.11.2026` in German, `SAT 14 NOV 2026` in English), then `HH:MM` in 24-hour time in both.
- Speed is a SegmentedControl; cash is `ink` and turns `stop` when negative.
- Cells change without animation. A flip animation would have to be driven by the simulation clock, and the UI has no clock of its own.

Godot: `HBoxContainer` in a `PanelContainer`; each flap cell is a `Label` in a `PanelContainer` with the `FlapCell` variation.
