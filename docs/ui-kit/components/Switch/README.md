# Switch

Turns one setting on or off, effective at once. The consumer provides the state; the row it sits in provides the label.

- On is an `accent` track with an `on-accent` thumb at the right; off is `surface-sunken` with an `ink-muted` thumb at the left.
- The state is always written beside it ("Ein" / "Aus", "On" / "Off") in `code` style, so it reads without colour.
- Use a Switch for settings that apply immediately. A choice that waits for a confirm button is a Checkbox.

Godot: `CheckButton` with custom `checked` and `unchecked` icons, plus a `Label` for the state word.
