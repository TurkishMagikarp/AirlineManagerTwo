# EmptyState

Fills a panel or table that has nothing to show yet. The consumer provides a short title, one or two sentences and the action that creates the first entry.

- Dashed `line-strong` frame, centred, padding `space-7`.
- The title says what is missing ("Noch keine Routen"); the text says what the thing is for and what happens next.
- Exactly one primary Button. If the player cannot act yet, say what is needed instead.

Godot: a `VBoxContainer` shown in place of the `Tree` when it has no rows.
