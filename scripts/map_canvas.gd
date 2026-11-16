extends Control
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Equirectangular projection; camera state only, never a flight clock.
signal selected(aircraft_id: int)
signal camera_changed
var base_icao := ""
var show_competitors := false
var competitor_paths: Array = []
var selected_competitor := -1
var show_routes := true
var show_airports := true
var show_flights := true
var selected_route := 0
var selection_position := Vector2.ZERO
var airports: Array = []
## ICAO -> announced or active events (boom, slump, weather) touching that airport.
var airport_events: Dictionary = {}
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
	camera_changed.emit()
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
	# Competitors are painted first, so own routes always remain on top.
	for path in competitor_paths if show_competitors else []:
		for copy in range(-1, 2):
			var phase := 0.0
			for i in range(1, path.size()):
				var a := project(path[i - 1] + Vector2(copy * 360, 0))
				var b := project(path[i] + Vector2(copy * 360, 0))
				var length := a.distance_to(b)
				var cursor := 0.0
				while cursor < length:
					var step := minf(maxf(0.001, 6.0 - fmod(phase, 6.0)), length - cursor)
					if fmod(phase, 12.0) < 6.0:
						draw_line(a.lerp(b, cursor / length), a.lerp(b, (cursor + step) / length), ThemeTokens.INK_FAINT, 2, true)
					cursor += step
					phase += step
	for route in routes if show_routes else []:
		var path := unwrap_path(route["points"])
		for copy in range(-1, 2):
			var line := PackedVector2Array()
			for point in path: line.append(project(point + Vector2(copy * 360, 0)))
			draw_polyline(line, ThemeTokens.ACCENT_TEXT, 3 if route["id"] == selected_route else 2, true)
	var font := ThemeTokens.font_for(ThemeTokens.CODE)
	var label_boxes: Array[Rect2] = []
	for airport in airports if show_airports else []:
		for copy in range(-1, 2):
			var p := project(airport["position"] + Vector2(copy * 360, 0))
			var base: bool = airport["icao"] == base_icao
			if base:
				draw_rect(Rect2(p - Vector2(7, 7), Vector2(14, 14)), ThemeTokens.ACCENT)
				draw_rect(Rect2(p - Vector2(7, 7), Vector2(14, 14)), ThemeTokens.ON_ACCENT, false, 1.5)
			else:
				draw_circle(p, 5, ThemeTokens.MAP_LAND)
				draw_circle(p, 5, ThemeTokens.INK, false, 2, true)
			if airport_events.has(airport["icao"]):
				var marker := event_marker(airport["icao"])
				draw_arc(p, 10.0 if base else 9.0, 0, TAU, 32, marker[0] if marker[1] else Color(marker[0], 0.6), 2.0 if marker[1] else 1.5, true)
			if (zoom >= 2 or base) and Rect2(Vector2.ZERO, size).has_point(p):
				var label_position := p + (Vector2(-54, 25) if base else Vector2(12, -6))
				label_position.x = clampf(label_position.x, 8, maxf(8, size.x - 50))
				label_position.y = clampf(label_position.y, 16, size.y - 8)
				var label_box := Rect2(label_position - Vector2(4, 16), Vector2(54, 22))
				for attempt in range(airports.size() + 1):
					var overlaps := false
					for box in label_boxes:
						if box.intersects(label_box): overlaps = true; break
					if not overlaps: break
					label_position.y += 22
					if label_position.y > size.y - 8: label_position.y = 16
					label_box.position = label_position - Vector2(4, 16)
				label_boxes.append(label_box)
				if base:
					draw_style_box(ThemeTokens.box(ThemeTokens.SURFACE_RAISED, ThemeTokens.LINE_STRONG, ThemeTokens.RADIUS_SM), Rect2(label_position - Vector2(8, 15), Vector2(50, 22)))
				draw_string(font, label_position, airport["icao"], HORIZONTAL_ALIGNMENT_LEFT, -1, ThemeTokens.CODE["size"], ThemeTokens.INK)
	for flight in flights if show_flights else []:
		for copy in range(-1, 2):
			var p := project(flight["position"] + Vector2(copy * 360, 0))
			var direction: Vector2 = flight["ahead"] - flight["position"]
			direction.x = wrapf(direction.x, -180, 180)
			var angle := Vector2(direction.x, -direction.y).angle()
			if flight["aircraft_id"] == selected_id: draw_circle(p, 15, ThemeTokens.ACCENT_TEXT, false, 2, true)
			var icon := PackedVector2Array()
			for vertex in [Vector2(11, 0), Vector2(-7, -8), Vector2(-3, 0), Vector2(-7, 8)]: icon.append(p + vertex.rotated(angle))
			draw_colored_polygon(icon, ThemeTokens.INK)

## [colour, active] of the strongest event at an airport: slump, then weather, then boom.
func event_marker(icao: String) -> Array:
	var best := 0
	var active := false
	for event in airport_events.get(icao, []):
		var rank: int = {"slump": 3, "weather": 2, "boom": 1}.get(event["kind"], 1)
		if rank > best: best = rank
		active = active or event["status"] == "active"
	return [[ThemeTokens.GO, ThemeTokens.SKY, ThemeTokens.CAUTION][best - 1], active]

func event_kind_name(event: Dictionary) -> String:
	if event["kind"] == "weather": return tr("EVENTS_WEATHER_" + str(event["weather_kind"]).to_upper())
	return tr("EVENTS_" + str(event["kind"]).to_upper())

func event_line(event: Dictionary) -> String:
	var status: String = tr("MAP_EVENT_ACTIVE") if event["status"] == "active" else tr("MAP_EVENT_UPCOMING") % (event["starts_at"] / 86400 + 1)
	var line: String = tr("MAP_EVENT_LINE") % [event_kind_name(event), status, event["ends_at"] / 86400]
	if event["kind"] == "weather": line += " · " + tr("MAP_EVENT_DELAY") % ceili(event["delay_seconds"] / 60.0)
	return line

func _get_tooltip(at_position: Vector2) -> String:
	var closest := 14.0
	var found := -1
	for i in range(airports.size() if show_airports else 0):
		if not airport_events.has(airports[i]["icao"]): continue
		for copy in range(-1, 2):
			var distance := at_position.distance_to(project(airports[i]["position"] + Vector2(copy * 360, 0)))
			if distance < closest:
				closest = distance
				found = i
	if found < 0: return ""
	var lines: Array[String] = ["%s · %s" % [airports[found]["icao"], airports[found]["name"]]]
	for event in airport_events[airports[found]["icao"]]: lines.append(event_line(event))
	return "\n".join(lines)

func hit_test(point: Vector2) -> int:
	var closest := 18.0
	var id := 0
	for flight in flights if show_flights else []:
		for copy in range(-1, 2):
			var distance := point.distance_to(project(flight["position"] + Vector2(copy * 360, 0)))
			if distance < closest:
				closest = distance
				id = flight["aircraft_id"]
	if id == 0:
		for i in range(airports.size() if show_airports else 0):
			for copy in range(-1, 2):
				var distance := point.distance_to(project(airports[i]["position"] + Vector2(copy * 360, 0)))
				if distance < closest:
					closest = distance
					id = -i - 1
	selected_route = 0
	selected_competitor = -1
	if id == 0 and show_routes:
		for route in routes:
			var path := unwrap_path(route["points"])
			for copy in range(-1, 2):
				for i in range(1, path.size()):
					var a := project(path[i - 1] + Vector2(copy * 360, 0))
					var b := project(path[i] + Vector2(copy * 360, 0))
					var distance := point.distance_to(Geometry2D.get_closest_point_to_segment(point, a, b))
					if distance < minf(closest, 8):
						closest = distance
						selected_route = route["id"]
	if id == 0 and selected_route == 0 and show_competitors:
		for index in range(competitor_paths.size()):
			var path: PackedVector2Array = competitor_paths[index]
			for copy in range(-1, 2):
				for i in range(1, path.size()):
					var a := project(path[i - 1] + Vector2(copy * 360, 0))
					var b := project(path[i] + Vector2(copy * 360, 0))
					var distance := point.distance_to(Geometry2D.get_closest_point_to_segment(point, a, b))
					if distance < minf(closest, 8):
						closest = distance
						selected_competitor = index
	selection_position = point
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

func center_base() -> void:
	for airport in airports:
		if airport["icao"] == base_icao:
			zoom = 6.0
			offset = -Vector2(airport["position"].x, -airport["position"].y) * scale_factor()
			_constrain()
			_camera_changed()
			return
