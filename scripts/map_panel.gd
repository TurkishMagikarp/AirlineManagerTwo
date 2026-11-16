extends VBoxContainer
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Read-only view: all flight coordinates and progress come from Rust.
const MapCanvas = preload("res://scripts/map_canvas.gd")
const UI = preload("res://scripts/ui/components.gd")
var simulation
var canvas: Control
var details: Label
var summary: Label
var selected_id: int = 0
var last_flights: Array = []
var last_selected_id := -1
var last_selected_route := -1
var card: PanelContainer
var card_body: VBoxContainer
var scale_label: Label
var route_table: Tree
var route_rows: Array = []
var last_card_key := ""
var competitors: Array = []
var last_selected_competitor := -1
var competitor_layer: CheckBox
var geometry_rebuilds := 0

func _ready() -> void:
	summary = UI.label("", ThemeTokens.SMALL)
	add_child(summary)
	canvas = MapCanvas.new()
	canvas.custom_minimum_size = Vector2(0, 400)
	canvas.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	canvas.selected.connect(func(id): selected_id = id; refresh_flights())
	canvas.camera_changed.connect(_position_overlays)
	add_child(canvas)
	var layers := UI.map_float(canvas, ThemeTokens.SPACE_3)
	layers.position = Vector2.ONE * ThemeTokens.SPACE_4
	var layer_body := VBoxContainer.new()
	layer_body.add_theme_constant_override("separation", ThemeTokens.SPACE_2)
	layers.add_child(layer_body)
	layer_body.add_child(UI.label(tr("MAP_LAYERS"), ThemeTokens.LABEL, UI.MUTED))
	for caption in ["MAP_OWN_ROUTES", "MAP_AIRPORTS", "MAP_AIRCRAFT"]:
		var check := UI.map_layer(tr(caption), caption == "MAP_OWN_ROUTES")
		check.button_pressed = true
		check.toggled.connect(func(value):
			if caption == "MAP_OWN_ROUTES": canvas.show_routes = value
			elif caption == "MAP_AIRPORTS": canvas.show_airports = value
			else: canvas.show_flights = value
			canvas.queue_redraw())
		layer_body.add_child(check)
	competitor_layer = UI.map_layer(tr("MAP_COMPETITORS"), false)
	competitor_layer.toggled.connect(func(value):
		canvas.show_competitors = value
		if not value: canvas.selected_competitor = -1
		last_selected_id = -2147483648
		refresh_flights()
		canvas.queue_redraw())
	layer_body.add_child(competitor_layer)
	var zoom_panel := UI.map_float(canvas, 0)
	zoom_panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_RIGHT)
	zoom_panel.offset_left = -52
	zoom_panel.offset_right = -16
	zoom_panel.offset_top = 16
	zoom_panel.offset_bottom = 16
	var zoom_body := VBoxContainer.new()
	zoom_body.add_theme_constant_override("separation", 0)
	zoom_panel.add_child(zoom_body)
	for caption in ["+", "−", "◎"]:
		var button := Button.new()
		button.text = caption
		button.theme_type_variation = "MapZoomButton"
		button.custom_minimum_size = Vector2.ONE * ThemeTokens.CONTROL_HEIGHT
		button.tooltip_text = tr("MAP_CENTER_BASE") if caption == "◎" else tr("MAP_ZOOM_IN") if caption == "+" else tr("MAP_ZOOM_OUT")
		button.pressed.connect(func():
			if caption == "◎": canvas.center_base()
			else: canvas.zoom_at(canvas.size / 2, 1.3 if caption == "+" else 1.0 / 1.3))
		zoom_body.add_child(button)
	scale_label = UI.map_scale(canvas)
	card = UI.map_float(canvas, ThemeTokens.SPACE_4)
	card.custom_minimum_size.x = 300
	card.size.x = 300
	card.resized.connect(func(): _position_overlays.call_deferred())
	card_body = VBoxContainer.new()
	card_body.add_theme_constant_override("separation", ThemeTokens.SPACE_3)
	card.add_child(card_body)
	card.hide()
	details = UI.label("")
	details.hide() # Preserve the complete accessible detail text used by map consumers.
	add_child(details)
	route_table = Tree.new()
	route_table.columns = 3
	route_table.hide_root = true
	route_table.column_titles_visible = true
	route_table.custom_minimum_size.y = 140
	for i in range(3): route_table.set_column_title(i, [tr("MAP_ROUTE"), tr("MAP_DISTANCE"), tr("ROUTE_PRICE")][i])
	route_table.item_selected.connect(_select_row)
	add_child(route_table)
	var hint := UI.label(tr("MAP_HINT"), ThemeTokens.CODE, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(hint)
	refresh()

func refresh() -> void:
	if not is_instance_valid(canvas): return
	canvas.airports = simulation.get_map_airports()
	canvas.routes = simulation.get_map_routes()
	canvas.base_icao = simulation.get_airline_info().get("airport_icao", "")
	var by_airport := {}
	for event in simulation.get_world_events():
		for icao in event["airports"]:
			if not by_airport.has(icao): by_airport[icao] = []
			by_airport[icao].append(event)
	if by_airport != canvas.airport_events:
		canvas.airport_events = by_airport
		canvas.queue_redraw()
	last_selected_id = -2147483648
	route_rows = simulation.get_routes()
	route_table.clear()
	var root := route_table.create_item()
	for route in route_rows:
		var row := route_table.create_item(root)
		row.set_metadata(0, route["id"])
		for i in range(3): UI.cell_text(row, i, ["%s–%s" % [route["origin_icao"], route["destination_icao"]], "%.0f km" % route["distance_km"], (tr("FREIGHT_RATE_VALUE") % UI.decimal(route["cargo_rate_cents"] / 100.0, 2)) if route.get("freighter_only", false) else UI.money(route["ticket_eur"])][i])
	refresh_flights()

func refresh_flights() -> void:
	if not is_instance_valid(canvas): return
	var public_routes: Array = simulation.get_competitor_routes()
	if public_routes != competitors:
		canvas.airports = simulation.get_map_airports()
		competitors = public_routes
		canvas.competitor_paths.clear()
		for path in simulation.get_map_competitor_routes():
			canvas.competitor_paths.append(canvas.unwrap_path(path))
		geometry_rebuilds += 1
		canvas.selected_competitor = -1
		last_selected_id = -2147483648
	canvas.flights = simulation.get_map_flights()
	canvas.selected_id = selected_id
	if canvas.flights == last_flights and selected_id == last_selected_id and canvas.selected_route == last_selected_route and canvas.selected_competitor == last_selected_competitor: return
	last_flights = canvas.flights
	last_selected_id = selected_id
	summary.text = tr("MAP_SUMMARY") % [canvas.flights.size(), canvas.routes.size()]
	details.text = tr("MAP_NO_FLIGHT")
	for child in card_body.get_children():
		card_body.remove_child(child)
		child.queue_free()
	card.visible = selected_id != 0 or canvas.selected_route != 0 or canvas.selected_competitor >= 0
	if selected_id <= 0 and canvas.selected_route == 0: route_table.deselect_all()
	if selected_id < 0 and -selected_id - 1 < canvas.airports.size():
		var airport: Dictionary = canvas.airports[-selected_id - 1]
		details.text = "%s · %s\n%s" % [airport["icao"], airport["name"], airport["slots"]]
		_card_header(airport["icao"], tr("MAP_BASE") if airport["icao"] == canvas.base_icao else "", "neutral")
		_add_name(airport["name"], ThemeTokens.MAP_CARD_NAME)
		_add_name(airport["slots"])
		for event in canvas.airport_events.get(airport["icao"], []):
			var line := HBoxContainer.new()
			card_body.add_child(line)
			var chip := UI.status_chip(canvas.event_kind_name(event), "caution" if event["kind"] == "slump" else "go" if event["kind"] == "boom" else "info")
			chip.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
			line.add_child(chip)
			var text := UI.label(canvas.event_line(event).split(" · ", true, 1)[1], ThemeTokens.SMALL, UI.MUTED)
			text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			text.custom_minimum_size.x = 150
			line.add_child(text)
		for service in airport["airlines"]:
			_add_name(tr("MAP_AIRPORT_SERVICE") % [service["airline"], service["departures_per_day"]])
		_action(tr("MAP_CREATE_ROUTE"), 0, airport["icao"])
		_action(tr("MAP_SHOW_ROUTES"), 0)
	var route_id: int = canvas.selected_route
	for flight in canvas.flights:
		if flight["aircraft_id"] == selected_id:
			route_id = flight["route_id"]
			details.text = tr("MAP_FLIGHT_DETAILS") % [flight["flight_number"], flight["model"], flight["identifier"], flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]]
			if route_id == 0:
				_card_header("%s–%s" % [flight["origin"], flight["destination"]], "Überführung", "info")
				_add_name("%s · %s" % [flight["model"], flight["identifier"]])
				card_body.add_child(UI.route_leg(flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]))
	for route in route_rows:
		if route["id"] != route_id: continue
		canvas.selected_route = route_id
		var airborne := false
		for flight in canvas.flights:
			if flight["route_id"] == route_id: airborne = true
		_card_header("%s–%s" % [route["origin_icao"], route["destination_icao"]], tr("MAP_IN_FLIGHT") if airborne else tr("MAP_ACTIVE") if route["has_plan"] else tr("MAP_PLANNED"), "info" if airborne else "neutral")
		var cities: Array[String] = []
		for code in [route["origin_icao"], route["destination_icao"]]:
			for airport in canvas.airports:
				if airport["icao"] == code: cities.append(airport["name"])
		_add_name(" – ".join(cities), ThemeTokens.MAP_CARD_NAME)
		for flight in canvas.flights:
			if flight["route_id"] != route_id: continue
			_add_name("%s · %s · %s" % [flight["flight_number"], flight["model"], flight["identifier"]])
			var track := UI.LegTrack.new()
			track.percent = flight["progress"] * 100
			track.custom_minimum_size.y = 12
			card_body.add_child(track)
			_fact(tr("MAP_ARRIVAL"), "%s · %.1f %%" % [flight["arrival"], flight["progress"] * 100])
			break
		_fact(tr("MAP_DISTANCE"), "%.0f km" % route["distance_km"])
		if not route.get("freighter_only", false):
			_fact(tr("MAP_TICKET_PRICE"), UI.money(route["ticket_eur"]))
		else:
			var chip := UI.status_chip(tr("FREIGHTER"), "info")
			chip.tooltip_text = tr("FREIGHTER_TOOLTIP")
			card_body.add_child(chip)
			_fact(tr("FREIGHT_EXPECTED"), "%d kg · %.1f %%" % [route["estimated_cargo_kg"], route["freight_load_percent"]])
			_fact(tr("FREIGHT_REVENUE"), UI.money(route["actual_cargo_revenue_eur"]))
		for offer in competitors:
			if _same_pair(route, offer): _add_name(tr("MAP_CONTESTED") % _offer_values(offer))
		_action(tr("MAP_EDIT_SCHEDULE"), route_id)
		var item := route_table.get_root().get_first_child()
		while item != null:
			if item.get_metadata(0) == route_id: item.select(0)
			item = item.get_next()
	if canvas.selected_competitor >= 0 and canvas.selected_competitor < competitors.size():
		var offer: Dictionary = competitors[canvas.selected_competitor]
		_card_header("%s–%s" % [offer["origin_icao"], offer["destination_icao"]], tr("MAP_COMPETITOR"), "neutral")
		_add_name(offer["airline"], ThemeTokens.MAP_CARD_NAME)
		_add_name(offer["model"])
		_add_name(tr("MAP_FREQUENCY") % offer["departures_per_day_per_direction"])
		_fact(tr("MAP_ECONOMY"), UI.money(offer["ticket_eur"]))
		_fact(tr("MAP_BUSINESS"), UI.money(offer["business_ticket_eur"]))
		details.text = "%s · %s · %s" % [offer["airline"], offer["model"], tr("MAP_CONTESTED") % _offer_values(offer)]
	card.reset_size()
	var card_key := "%d/%d/%d" % [selected_id, canvas.selected_route, canvas.selected_competitor]
	if card.visible and card_key != last_card_key: _reveal_selected.call_deferred()
	last_card_key = card_key
	last_selected_competitor = canvas.selected_competitor
	last_selected_route = canvas.selected_route
	_position_overlays.call_deferred()
	canvas.queue_redraw()

func _same_pair(a: Dictionary, b: Dictionary) -> bool:
	return (a["origin_icao"] == b["origin_icao"] and a["destination_icao"] == b["destination_icao"]) or (a["origin_icao"] == b["destination_icao"] and a["destination_icao"] == b["origin_icao"])

func _offer_values(offer: Dictionary) -> Array:
	return [offer["airline"], UI.money(offer["ticket_eur"]), UI.money(offer["business_ticket_eur"]), offer["departures_per_day_per_direction"]]

func _card_header(title: String, status: String, kind: String) -> void:
	var row := HBoxContainer.new()
	card_body.add_child(row)
	var heading := UI.label(title, ThemeTokens.FIGURE)
	heading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(heading)
	if not status.is_empty():
		var chip := UI.status_chip(status, kind)
		chip.size_flags_vertical = Control.SIZE_SHRINK_BEGIN
		row.add_child(chip)

func _add_name(text: String, style: Dictionary = ThemeTokens.SMALL) -> void:
	var label := UI.label(text, style, UI.MUTED)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.custom_minimum_size.x = 268
	card_body.add_child(label)

func _fact(caption: String, value: String) -> void:
	var row := HBoxContainer.new()
	card_body.add_child(row)
	var label := UI.label(caption, ThemeTokens.SMALL, UI.MUTED)
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(label)
	row.add_child(UI.label(value, ThemeTokens.DATA))

func _action(caption: String, route_id: int, airport_icao: String = "") -> void:
	var button := Button.new()
	button.text = caption
	button.theme_type_variation = "PanelActionButton" if airport_icao.is_empty() else "PrimaryButton"
	button.pressed.connect(func():
		var main := get_tree().current_scene
		if main == null: main = get_tree().root.get_child(get_tree().root.get_child_count() - 1)
		main._show_page(2)
		var routes = main.routes_panel
		var scroll: ScrollContainer = routes.get_parent().get_parent()
		if not airport_icao.is_empty():
			for select in [routes.origin, routes.destination]:
				var code: String = canvas.base_icao if select == routes.origin else airport_icao
				for index in range(select.item_count):
					if select.get_item_metadata(index) == code: select.select(index)
			routes._preview()
			scroll.ensure_control_visible.call_deferred(routes.origin)
			routes.origin.grab_focus()
		elif route_id == 0:
			scroll.ensure_control_visible.call_deferred(routes.route_tree)
			routes.route_tree.grab_focus()
		var item: TreeItem = main.routes_panel.route_tree.get_root().get_first_child()
		while item != null:
			var route: Dictionary = item.get_metadata(0)
			if route.get("id", 0) == route_id:
				item.select(0)
				main.routes_panel._selection_changed()
				scroll.ensure_control_visible.call_deferred(routes.delay_input)
				routes.delay_input.get_line_edit().grab_focus()
				break
			item = item.get_next())
	card_body.add_child(button)

func _select_row() -> void:
	var row := route_table.get_selected()
	if row == null: return
	var id: int = row.get_metadata(0)
	if canvas.selected_route == id: return
	selected_id = 0
	canvas.selected_route = id
	canvas.selected_competitor = -1
	for route in canvas.routes:
		if route["id"] == id: canvas.selection_position = canvas.project(route["points"][64])
	refresh_flights()

func _position_overlays() -> void:
	if not is_instance_valid(scale_label): return
	canvas.custom_minimum_size.y = minf(400, maxf(352, get_viewport_rect().size.y - 168))
	scale_label.position = Vector2(16, canvas.size.y - 48)
	# Projection is equirectangular: label the equatorial distance of the 96px bar.
	scale_label.text = "%.0f km" % (96.0 / canvas.scale_factor() * 111.195)
	if not card.visible: return
	var anchor: Vector2 = canvas.selection_position
	if selected_id < 0: anchor = canvas.project(canvas.airports[-selected_id - 1]["position"])
	for flight in canvas.flights:
		if flight["aircraft_id"] == selected_id: anchor = canvas.project(flight["position"])
	var x := anchor.x + 24
	if x + card.size.x > canvas.size.x - 68: x = anchor.x - card.size.x - 24
	card.position = Vector2(clampf(x, 16, maxf(16, canvas.size.x - card.size.x - 68)), clampf(anchor.y, 16, maxf(16, canvas.size.y - card.size.y - 16)))

func _unhandled_key_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		selected_id = 0
		canvas.selected_route = 0
		canvas.selected_competitor = -1
		refresh_flights()
		get_viewport().set_input_as_handled()

func _reveal_selected() -> void:
	if not card.visible: return
	var anchor: Vector2 = canvas.selection_position
	if selected_id < 0: anchor = canvas.project(canvas.airports[-selected_id - 1]["position"])
	for flight in canvas.flights:
		if flight["aircraft_id"] == selected_id: anchor = canvas.project(flight["position"])
	if canvas.size.x < card.size.x * 2 + 120:
		# Leave the 36px zoom group and its 16px gutters beside the card.
		var shift := canvas.size.x - card.size.x - 92 - anchor.x
		canvas.offset.x += shift
		canvas.selection_position.x += shift
		canvas._constrain()
		canvas._camera_changed()
