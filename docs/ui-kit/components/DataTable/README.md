# DataTable

Rows of comparable records: fleet register, routes, flight results, payroll. The consumer provides column titles, widths and rows.

- Header strip is `surface-sunken` with `column-head` titles. Rows are 40px with a `line` divider.
- Text columns are left-aligned in `body`. Every numeric column is right-aligned in `data` style with the unit in each cell.
- Codes (registration, ICAO pair, flight number) use `code` style.
- State is a StatusChip, not coloured text. Money is `go` or `stop` only for results, always with a sign.
- The selected row is `accent-wash`; hover is `surface-raised`. The table scrolls inside its frame; the page never scrolls sideways.

Godot: `UI.table(titles, widths, height)` returns a `Tree`; chips are drawn with `set_custom_draw` or a cell icon plus text.
