# Dialog

Interrupts for a decision that cannot be undone: selling an aircraft, deleting a route, overwriting a save. The consumer provides a question as title, one or two sentences of consequence and two button labels.

- The title is the question with the object's name ("D-ANFC verkaufen?"). The body states what happens and what it costs.
- Buttons are verbs: the safe choice left as a default Button, the destructive one right as the filled `stop` Button. Focus starts on the safe choice.
- `scrim` behind, `surface-raised` panel, `radius-lg`, `shadow-pop`, padding `space-6`, width 440px.
- Do not use a dialog for success messages or for anything that can be undone.

Godot: `ConfirmationDialog` with `embedded_panel` styled, or a `PanelContainer` over a full-rect `ColorRect` scrim.
