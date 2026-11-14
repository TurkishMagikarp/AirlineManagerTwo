# AttentionList

The prioritised list of things the player should deal with, above every screen. The consumer provides the entries from `SimulationClock.get_attention_snapshot()` in the order Rust returns them.

- Each row: a StatusChip for the kind, one sentence with the lead phrase in `body-strong`, and where a click leads ("Flotte öffnen →").
- The sentence names the object (registration, route, person) and the fact. It does not advise.
- The header shows the count and collapses the list. The list scrolls inside itself after four rows.
- With no entries the header reads "Keine Hinweise" and the count is hidden.

Godot: a `VBoxContainer` of row `Button`s (variation `AttentionRow`) in a `ScrollContainer` with `follow_focus`.
