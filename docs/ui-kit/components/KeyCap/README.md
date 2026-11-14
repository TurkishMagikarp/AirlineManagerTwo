# KeyCap

Shows a keyboard key or a combination. The consumer provides the key names from the input map, in the active language ("Leertaste" / "Space", "Strg" / "Ctrl").

- `surface-raised` cap with a `line-strong` border, heavier at the bottom; text in `code` style.
- Combinations join caps with a plus sign in `ink-muted`.
- Use it in settings, in tooltips after the action name, and nowhere else.

Godot: a `Label` in a `PanelContainer` with the `KeyCap` variation; key names from `InputEvent.as_text()`.
