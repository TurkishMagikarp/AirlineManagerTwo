extends Control
## Equirectangular projection; camera state only, never a flight clock.
signal selected(aircraft_id: int)
var airports: Array = []
var routes: Array = []
var flights: Array = []
var selected_id: int = 0
var zoom: float = 1.0
var offset := Vector2.ZERO
var land: Array[PackedVector2Array] = []
var dragging := false
var drag_distance: float = 0.0

func _ready() -> void:
	clip_contents = true
	mouse_filter = Control.MOUSE_FILTER_STOP
	mouse_default_cursor_shape = Control.CURSOR_DRAG
	var geo: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://data/world_land.geojson"))
	for feature in geo["features"]:
		var geometry: Dictionary = feature["geometry"]
		var polygons: Array = geometry["coordinates"] if geometry["type"] == "MultiPolygon" else [geometry["coordinates"]]
		for polygon in polygons:
			var ring := PackedVector2Array()
			for point in polygon[0]: ring.append(Vector2(point[0], point[1]))
			if not Geometry2D.triangulate_polygon(ring).is_empty(): land.append(ring)
	resized.connect(queue_redraw)

func scale_factor() -> float:
	return minf(size.x / 360.0, size.y / 180.0) * zoom

func project(point: Vector2) -> Vector2:
	return size / 2 + offset + Vector2(point.x, -point.y) * scale_factor()

func reset_view() -> void:
	zoom = 1.0
	offset = Vector2.ZERO
	queue_redraw()

func zoom_at(cursor: Vector2, factor: float) -> void:
	var old := zoom
	zoom = clampf(zoom * factor, 1.0, 12.0)
	offset = cursor - size / 2 - (cursor - size / 2 - offset) * (zoom / old)
	_constrain()
	queue_redraw()

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
	draw_rect(Rect2(Vector2.ZERO, size), Color("0b1725"))
	for lon in range(-180, 181, 30):
		for copy in range(-1, 2):
			var x := lon + copy * 360
			draw_line(project(Vector2(x, -90)), project(Vector2(x, 90)), Color("182b3e"))
	for lat in range(-60, 61, 30):
		draw_line(Vector2(0, project(Vector2(0, lat)).y), Vector2(size.x, project(Vector2(0, lat)).y), Color("182b3e"))
	for ring in land:
		for copy in range(-1, 2):
			var polygon := PackedVector2Array()
			for point in ring: polygon.append(project(point + Vector2(copy * 360, 0)))
			draw_colored_polygon(polygon, Color("223b4b"))
			draw_polyline(polygon, Color("355466"), 1, true)
	for route in routes:
		var path := unwrap_path(route["points"])
		for copy in range(-1, 2):
			var line := PackedVector2Array()
			for point in path: line.append(project(point + Vector2(copy * 360, 0)))
			draw_polyline(line, Color("54bcc9") if route["active"] else Color("66788c"), 1.5, true)
	var font := ThemeDB.fallback_font
	var label_boxes: Array[Rect2] = []
	for airport in airports:
		for copy in range(-1, 2):
			var p := project(airport["position"] + Vector2(copy * 360, 0))
			draw_circle(p, 3, Color("e2eaf4"))
			if zoom >= 2:
				var label_position := p + Vector2(6, -6)
				for box in label_boxes:
					if box.intersects(Rect2(label_position - Vector2(0, 12), Vector2(40, 16))): label_position.y += 18
				label_boxes.append(Rect2(label_position - Vector2(0, 12), Vector2(40, 16)))
				draw_string(font, label_position, airport["icao"], HORIZONTAL_ALIGNMENT_LEFT, -1, 12, Color("b0becf"))
	for flight in flights:
		for copy in range(-1, 2):
			var p := project(flight["position"] + Vector2(copy * 360, 0))
			var direction: Vector2 = flight["ahead"] - flight["position"]
			direction.x = wrapf(direction.x, -180, 180)
			var angle := Vector2(direction.x, -direction.y).angle()
			if flight["aircraft_id"] == selected_id: draw_circle(p, 15, Color("54bcc9"), false, 2, true)
			var icon := PackedVector2Array()
			for vertex in [Vector2(11, 0), Vector2(-7, -8), Vector2(-3, 0), Vector2(-7, 8)]: icon.append(p + vertex.rotated(angle))
			draw_colored_polygon(icon, Color("ffd58a"))

func hit_test(point: Vector2) -> int:
	var closest := 18.0
	var id := 0
	for flight in flights:
		for copy in range(-1, 2):
			var distance := point.distance_to(project(flight["position"] + Vector2(copy * 360, 0)))
			if distance < closest:
				closest = distance
				id = flight["aircraft_id"]
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
		queue_redraw()
		accept_event()
