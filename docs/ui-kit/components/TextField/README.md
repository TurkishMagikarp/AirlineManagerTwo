# TextField

One line of typed input with a label above and help below. The consumer provides the label, the value, an optional unit and the help or error sentence.

- Numbers are right-aligned in `data` style with the unit ("EUR", "%") as a fixed suffix inside the well, so the player never types it.
- The well is `surface-sunken` with a `line-strong` border. On error the border turns `stop` and the help line is replaced by a sentence that says what to enter.
- Help text states the useful range or consequence, not the field's name again.

Godot: `LineEdit` (or `SpinBox` for numbers) in a `VBoxContainer` with two `Label`s; error state swaps the `normal` stylebox via the `FieldError` variation.
