# MetricCard

One key figure with its caption. The consumer provides the caption, the value, its unit and at most one supporting line: a change against the previous period or a ProgressBar.

- Caption in `label` style, value in `figure` style with the unit smaller and `ink-muted`.
- The value turns `stop` only when the figure itself is bad (negative cash, a loss). It never turns `go`; a good figure is the normal state.
- A change line carries an arrow and a sign, so direction does not depend on colour.
- Cards share a row and wrap from four to two columns.

Godot: `UI.metric(parent, title)` returns the value `Label`.
