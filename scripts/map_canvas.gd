extends Control
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Equirectangular projection; camera state only, never a flight clock.
signal selected(aircraft_id: int)
var airports: Array = []
var routes: Array = []
var flights: Array = []
var selected_id: int = 0
var zoom: float = 1.0
var offset := Vector2.ZERO
var background: Control
var dragging := false
var drag_distance: float = 0.0

func _ready() -> void:
	clip_contents = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	mouse_default_cursor_shape = Control.CURSOR_DRAG
	background = preload("res://scripts/map_background.gd").new()
	background.show_behind_parent = true
	add_child(background)
	resized.connect(_camera_changed)

func _camera_changed() -> void:
	queue_redraw()
	if is_instance_valid(background): background.queue_redraw()

func scale_factor() -> float:
	return minf(size.x / 360.0, size.y / 180.0) * zoom

func project(point: Vector2) -> Vector2:
	return size / 2 + offset + Vector2(point.x, -point.y) * scale_factor()

func reset_view() -> void:
	zoom = 1.0
	offset = Vector2.ZERO
	_camera_changed()

func zoom_at(cursor: Vector2, factor: float) -> void:
	var old := zoom
	zoom = clampf(zoom * factor, 1.0, 12.0)
	offset = cursor - size / 2 - (cursor - size / 2 - offset) * (zoom / old)
	_constrain()
	_camera_changed()

func _constrain() -> void:
	var world := 360.0 * scale_factor()
	offset.x = wrapf(offset.x, -world / 2, world / 2)
	var limit := maxf(0.0, 90 * scale_factor() - size.y / 2)
	offset.y = clampf(offset.y, -limit, limit)

## Unwrap each segment before projection, then draw copies at the world edges.
func unwrap_path(points: PackedVector2Array) -> PackedVector2Array:
	var result := PackedVector2Array()
	for point in points:
		if not result.is_empty(): point.x = result[-1].x + wrapf(point.x - result[-1].x, -180, 180)
		result.append(point)
	return result

func _draw() -> void:
	for route in routes:
		var path := unwrap_path(route["points"])
		for copy in range(-1, 2):
			var line := PackedVector2Array()
			for point in path: line.append(project(point + Vector2(copy * 360, 0)))
			draw_polyline(line, ThemeTokens.ACCENT_TEXT if route["active"] else ThemeTokens.INK_FAINT, 1.5, true)
	var font := ThemeTokens.font_for(ThemeTokens.CODE)
	var label_boxes: Array[Rect2] = []
	for airport in airports:
		for copy in range(-1, 2):
			var p := project(airport["position"] + Vector2(copy * 360, 0))
			draw_circle(p, 3, ThemeTokens.INK)
			if zoom >= 2:
				var label_position := p + Vector2(6, -6)
				for box in label_boxes:
					if box.intersects(Rect2(label_position - Vector2(0, 12), Vector2(40, 16))): label_position.y += 18
				label_boxes.append(Rect2(label_position - Vector2(0, 12), Vector2(40, 16)))
				draw_string(font, label_position, airport["icao"], HORIZONTAL_ALIGNMENT_LEFT, -1, ThemeTokens.CODE["size"], ThemeTokens.INK_MUTED)
	for flight in flights:
		for copy in range(-1, 2):
			var p := project(flight["position"] + Vector2(copy * 360, 0))
			var direction: Vector2 = flight["ahead"] - flight["position"]
			direction.x = wrapf(direction.x, -180, 180)
			var angle := Vector2(direction.x, -direction.y).angle()
			if flight["aircraft_id"] == selected_id: draw_circle(p, 15, ThemeTokens.ACCENT_TEXT, false, 2, true)
			var icon := PackedVector2Array()
			for vertex in [Vector2(11, 0), Vector2(-7, -8), Vector2(-3, 0), Vector2(-7, 8)]: icon.append(p + vertex.rotated(angle))
			draw_colored_polygon(icon, ThemeTokens.SKY)

func hit_test(point: Vector2) -> int:
	var closest := 18.0
	var id := 0
	for flight in flights:
		for copy in range(-1, 2):
			var distance := point.distance_to(project(flight["position"] + Vector2(copy * 360, 0)))
			if distance < closest:
				closest = distance
				id = flight["aircraft_id"]
	if id == 0:
		for i in range(airports.size()):
			for copy in range(-1, 2):
				var distance := point.distance_to(project(airports[i]["position"] + Vector2(copy * 360, 0)))
				if distance < closest:
					closest = distance
					id = -i - 1
	return id

func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.pressed and event.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			zoom_at(event.position, 1.2 if event.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0 / 1.2)
			accept_event()
		if event.button_index == MOUSE_BUTTON_LEFT:
			dragging = event.pressed
			if event.pressed:
				drag_distance = 0
			elif drag_distance < 5:
				selected.emit(hit_test(event.position))
			accept_event()
	if event is InputEventMouseMotion and dragging:
		drag_distance += event.relative.length()
		offset += event.relative
		_constrain()
		_camera_changed()
		accept_event()
