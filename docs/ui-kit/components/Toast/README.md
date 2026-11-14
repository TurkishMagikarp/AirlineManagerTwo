# Toast

Confirms that something the player triggered has happened, or reports that it failed. The consumer provides a StatusChip, one sentence and at most one action.

- It appears bottom right, above the content, with `shadow-pop`.
- The sentence names what happened to what: "Spielstand „Nordwind Regional“ gesichert."
- A success toast is replaced by the next one or closed by the player. A failure stays until closed and offers the fix as its action.
- Events of the simulation (a flight landed, maintenance due) are never toasts; they belong in the AttentionList.

Godot: a `PanelContainer` in a top-level `CanvasLayer`.
