# Tooltip

Explains a control or breaks a figure down on hover or keyboard focus. The consumer provides either one short sentence or a caption with label and value pairs.

- `surface-raised`, `line-strong` border, `radius-md`, `shadow-pop`. Text in `small` style.
- A breakdown lists parts in `ink-muted` with values right-aligned in mono, and closes with a total row above a `line` rule.
- A tooltip never holds something the player must click, and never the only copy of information needed to decide.

Godot: `TooltipPanel` and `TooltipLabel` theme types; breakdowns via `_make_custom_tooltip`.
