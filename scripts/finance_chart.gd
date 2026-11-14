extends Control
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Bounded presentation of Rust samples. No economic calculations here.
const UI = preload("res://scripts/ui/components.gd")
var samples: Array = []
var field := "balance"
var comparison := ""
var percent := false
var plot := Rect2()

func _ready() -> void:
	custom_minimum_size.y = ThemeTokens.CHART_HEIGHT
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	mouse_filter = Control.MOUSE_FILTER_PASS
	resized.connect(queue_redraw)

func set_series(rows: Array, key: String, is_percent: bool = false) -> void:
	samples = rows
	field = key
	percent = is_percent
	comparison = ""
	queue_redraw()

func set_comparison(key: String) -> void:
	comparison = key
	queue_redraw()

func _value(value: Variant, signed: bool = false) -> String:
	return "%.1f %%" % value if percent else UI.signed_money(value) if signed else UI.money(value)

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and not samples.is_empty():
		var i := clampi(roundi((event.position.x - plot.position.x) / maxf(plot.size.x, 1) * (samples.size() - 1)), 0, samples.size() - 1)
		var value: float = samples[i][field]
		tooltip_text = "%s\n%s" % [samples[i]["time"], "Keine Landungen" if percent and value < 0 else _value(samples[i][field], field == "profit")]
		if not comparison.is_empty():
			tooltip_text += "\n%s" % _value(samples[i][comparison])

func _point(index: int, key: String, low: float, high: float) -> Vector2:
	return Vector2(plot.position.x + (1.0 if samples.size() == 1 else float(index) / (samples.size() - 1)) * plot.size.x, plot.end.y - (float(samples[index][key]) - low) / (high - low) * plot.size.y)

func _draw() -> void:
	if samples.is_empty():
		return
	var keys := [field] if comparison.is_empty() else ["revenue", "costs"]
	var low := 0.0
	var high := 100.0 if percent else 0.0
	var last_indices := {}
	for key in keys: last_indices[key] = -1
	for i in range(samples.size()):
		var row: Dictionary = samples[i]
		for key in keys:
			if percent and row[key] < 0: continue
			last_indices[key] = i
			# Keep signed cash/results visible; nonnegative series start at zero.
			low = minf(low, row[key])
			high = maxf(high, row[key])
	if low == 0:
		high = maxf(high, 4.0)
	# Build the signed axis outwards from zero; never lose the zero reference.
	var ticks: Array[float] = [0.0]
	if high > 0:
		var divisions := 4 if low == 0 else 1 if high < 2 else 2
		for tick in range(1, divisions + 1): ticks.append(high * tick / divisions)
	if low < 0:
		var divisions := 4 if high == 0 else 1 if low > -2 else 2
		for tick in range(1, divisions + 1): ticks.append(low * tick / divisions)
	var axis := ThemeTokens.CHART_AXIS
	var value_style := ThemeTokens.CHART_VALUE
	var font := ThemeTokens.font_for(axis)
	var value_font := ThemeTokens.font_for(value_style)
	var left := 0.0
	var right := 0.0
	for tick in ticks:
		left = maxf(left, font.get_string_size(_value(tick), HORIZONTAL_ALIGNMENT_LEFT, -1, axis["size"]).x)
	for key in keys:
		if last_indices[key] >= 0:
			right = maxf(right, value_font.get_string_size(_value(samples[last_indices[key]][key], key == "profit"), HORIZONTAL_ALIGNMENT_LEFT, -1, value_style["size"]).x)
	plot = Rect2(left + ThemeTokens.SPACE_3, ThemeTokens.SPACE_4, maxf(size.x - left - right - ThemeTokens.SPACE_3 * 3, 1), size.y - ThemeTokens.SPACE_4 - ThemeTokens.SPACE_6)
	for tick in ticks:
		var y := plot.end.y - plot.size.y * (tick - low) / (high - low)
		draw_line(Vector2(plot.position.x, y), Vector2(plot.end.x, y), ThemeTokens.LINE)
		draw_string(font, Vector2(0, y + ThemeTokens.SPACE_1), _value(tick), HORIZONTAL_ALIGNMENT_RIGHT, left, axis["size"], ThemeTokens.INK_MUTED)
	# Label only as many real period timestamps as fit. All remain in tooltips.
	var first_time: String = samples[0]["time"]
	var last_time: String = samples[-1]["time"]
	var last_width := font.get_string_size(last_time, HORIZONTAL_ALIGNMENT_LEFT, -1, axis["size"]).x
	var first_width := font.get_string_size(first_time, HORIZONTAL_ALIGNMENT_LEFT, -1, axis["size"]).x
	if first_width + last_width + ThemeTokens.SPACE_4 <= size.x:
		draw_string(font, Vector2(0, size.y - ThemeTokens.SPACE_1), first_time, HORIZONTAL_ALIGNMENT_LEFT, -1, axis["size"], ThemeTokens.INK_MUTED)
	draw_string(font, Vector2(size.x - last_width, size.y - ThemeTokens.SPACE_1), last_time, HORIZONTAL_ALIGNMENT_LEFT, -1, axis["size"], ThemeTokens.INK_MUTED)
	if keys.size() == 2:
		for i in range(1, samples.size()):
			var a := _point(i - 1, "revenue", low, high)
			var b := _point(i, "revenue", low, high)
			var c := _point(i, "costs", low, high)
			var d := _point(i - 1, "costs", low, high)
			if (a.y - d.y) * (b.y - c.y) < 0:
				var crossing := a.lerp(b, (d.y - a.y) / ((b.y - a.y) - (c.y - d.y)))
				draw_colored_polygon(PackedVector2Array([a, crossing, d]), ThemeTokens.GO_WASH)
				draw_colored_polygon(PackedVector2Array([b, crossing, c]), ThemeTokens.GO_WASH)
			elif a != d or b != c:
				var polygon := PackedVector2Array([a, b, c] if a == d else [a, b, d] if b == c else [a, b, c, d])
				draw_colored_polygon(polygon, ThemeTokens.GO_WASH)
	var label_y := -INF
	for key in keys:
		var ink: Color = ThemeTokens.STOP if key in ["costs", "tax"] else ThemeTokens.GO if key == "revenue" else ThemeTokens.SKY
		var connected := false
		var previous := Vector2.ZERO
		var dash_position := 0.0
		for i in range(samples.size()):
			if percent and samples[i][key] < 0:
				connected = false
				continue
			var point := _point(i, key, low, high)
			if connected:
				if key in ["costs", "tax"]:
					dash_position = _dashed(previous, point, ink, dash_position)
				else:
					draw_line(previous, point, ink, ThemeTokens.CHART_LINE_WIDTH, true)
			previous = point
			connected = true
		# Missing landings are gaps, not points: mark the last real sample.
		var last_index: int = last_indices[key]
		if last_index >= 0:
			previous = _point(last_index, key, low, high)
			draw_circle(previous, ThemeTokens.CHART_MARKER_RADIUS, ink, true, -1, true)
			draw_circle(previous, ThemeTokens.CHART_MARKER_RADIUS, ThemeTokens.SURFACE, false, ThemeTokens.CHART_LINE_WIDTH, true)
			var y := clampf(previous.y + ThemeTokens.SPACE_1, value_style["height"], plot.end.y)
			if absf(y - label_y) < value_style["height"]:
				y = y + value_style["height"] if y + value_style["height"] <= plot.end.y else y - value_style["height"]
			label_y = y
			draw_string(value_font, Vector2(previous.x + ThemeTokens.SPACE_3, y), _value(samples[last_index][key], key == "profit"), HORIZONTAL_ALIGNMENT_LEFT, -1, value_style["size"], ThemeTokens.INK)

## Carry the CSS 6px/4px dash phase across segments, even with dense samples.
func _dashed(start: Vector2, end: Vector2, ink: Color, phase: float) -> float:
	var length := start.distance_to(end)
	var position := 0.0
	var cycle := ThemeTokens.CHART_DASH + ThemeTokens.CHART_GAP
	while position < length:
		var on := phase < ThemeTokens.CHART_DASH
		var step := minf((ThemeTokens.CHART_DASH if on else cycle) - phase, length - position)
		if on:
			draw_line(start.lerp(end, position / length), start.lerp(end, (position + step) / length), ink, ThemeTokens.CHART_LINE_WIDTH, true)
		position += step
		phase = fmod(phase + step, cycle)
	return phase
