# SettingsRow

One setting: its name, one sentence about what it does, and one control. The consumer provides all three.

- Name in `body-strong`, description in `small` and `ink-muted`, control right-aligned.
- The description states the effect or when it applies ("Gilt sofort, ohne Neustart."), not the name again.
- Rows stack inside a Panel, separated by `line`. One control per row; a key binding shows its KeyCap and a small "Ändern" Button.
- Every setting applies at once. There is no Apply button on the settings screen.

Godot: an `HBoxContainer` with a text `VBoxContainer` (expand) and the control.
