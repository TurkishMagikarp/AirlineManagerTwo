# SegmentedControl

Picks exactly one of two to five short options that take effect at once: game speed (Pause, 1×, 4×, 16×) and chart range. The consumer provides the options and the selected one.

- Labels are set in `code` style and stay under six characters. Longer choices belong in a Select.
- The selected segment is an `accent` fill with `on-accent` text; the tray is `surface-sunken`.
- Never use it for navigation between views; that is Tabs.

Godot: an `HBoxContainer` of toggle `Button`s in one `ButtonGroup`, variation `SegmentButton`, inside a `PanelContainer` tray.
