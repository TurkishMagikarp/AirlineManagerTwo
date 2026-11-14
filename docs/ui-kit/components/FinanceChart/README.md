# FinanceChart

A line chart of money over time. The consumer provides one or two series, the period labels and the unit.

- Revenue is a solid `go` line; costs a dashed `stop` line, so the two differ in pattern as well as hue. The gap between them is filled with `go-wash`.
- Grid lines are `line`, horizontal only. Axis ticks are `code` style in `ink-muted`. The y axis starts at zero.
- Only the last point of each series gets a marker and a printed value.
- At most two series per chart. A third question gets its own chart.

Godot: `scripts/finance_chart.gd` draws with `_draw`; colours come from the theme, not from literals.
