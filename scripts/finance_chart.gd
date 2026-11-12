extends Control
## Bounded presentation of Rust samples. No economic calculations here.
const UI = preload("res://scripts/ui/components.gd")
var samples: Array = []
var field := "balance"
var percent := false

func _ready() -> void:
	custom_minimum_size.y = 180
	size_flags_horizontal = Control.SIZE_EXPAND_FILL
	mouse_filter = Control.MOUSE_FILTER_PASS
	resized.connect(queue_redraw)

func set_series(rows: Array, key: String, is_percent: bool = false) -> void:
	samples = rows
	field = key
	percent = is_percent
	queue_redraw()

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and not samples.is_empty():
		var i := clampi(roundi(event.position.x / maxf(size.x, 1) * (samples.size() - 1)), 0, samples.size() - 1)
		var value: float = samples[i][field]
		tooltip_text = "%s\n%s" % [samples[i]["time"], "Keine Landungen" if percent and value < 0 else "%.1f %%" % value if percent else UI.money(samples[i][field])]

func _draw() -> void:
	if samples.is_empty():
		return
	var low := 0.0
	var high := 100.0 if percent else 1.0
	for row in samples:
		if percent and row[field] < 0:
			continue
		low = minf(low, row[field])
		high = maxf(high, row[field])
	var area := Rect2(8, 26, maxf(size.x - 16, 1), maxf(size.y - 52, 1))
	var font := get_theme_default_font()
	draw_string(font, Vector2(8, 17), "%.1f %%" % high if percent else UI.money(int(high)), HORIZONTAL_ALIGNMENT_LEFT, -1, 12, UI.MUTED)
	draw_string(font, Vector2(8, size.y - 4), "%.1f %%" % low if percent else UI.money(int(low)), HORIZONTAL_ALIGNMENT_LEFT, -1, 12, UI.MUTED)
	var zero_y := area.end.y - (0 - low) / (high - low) * area.size.y
	draw_line(Vector2(area.position.x, zero_y), Vector2(area.end.x, zero_y), UI.MUTED * Color(1, 1, 1, 0.4))
	var previous := Vector2.ZERO
	var connected := false
	for i in range(samples.size()):
		var value: float = samples[i][field]
		if percent and value < 0:
			connected = false
			continue
		var point := Vector2(area.position.x + float(i) / maxi(samples.size() - 1, 1) * area.size.x, area.end.y - (value - low) / (high - low) * area.size.y)
		if connected:
			draw_line(previous, Vector2(point.x, previous.y), UI.ACCENT, 2, true)
			draw_line(Vector2(point.x, previous.y), point, UI.ACCENT, 2, true)
		draw_circle(point, 2.5, UI.ACCENT)
		previous = point
		connected = true
