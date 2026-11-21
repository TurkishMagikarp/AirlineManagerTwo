extends VBoxContainer
## Presentation only: Rust owns routes, availability, distances and validation.

signal changed
signal procurement_requested
signal staff_requested(aircraft_id: int)
signal reposition_requested(aircraft_id: int, destination: String, opener: Control)
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
const UI = preload("res://scripts/ui/components.gd")

var simulation: RefCounted
var origin: OptionButton
var destination: OptionButton
var aircraft: OptionButton
var preview: Label
var create_button: Button
var procurement_button: Button
var reposition_button: Button
var empty_action: Button
var status: Label
var season_label: Label
var route_tree: Tree
var empty_label: Control
var assignment: OptionButton
var assign_button: Button
var assign_ferry_button: Button
var delete_button: Button
var wind_down_button: Button
var release_label: Label
var delay_input: SpinBox
var interval_input: SpinBox
var schedule_button: Button
var stop_button: Button
var maintenance_label: Label
var crew_button: Button
var schedule_route_id := 0
var operations_label: Label
var operations_tree: Tree
var operations_empty: PanelContainer
var _operations_signature := ""
var service_select: OptionButton
var service_cost: Label
var business_input: SpinBox
var business_button: Button
var cargo_input: SpinBox
var cargo_button: Button
var cargo_status: Label
var ticket_input: SpinBox
var ticket_button: Button
var results_tree: Tree
var results_empty_text: Label
var results_empty_button: Button
var results_action: Callable
var codeshare_panel: VBoxContainer
var alliance_panel: VBoxContainer
var competition_label: Label
var competition_tree: Tree
var competition_empty: Control
var connections_label: Label
var connections_empty: Control
var connections_chip: Control
var fare_box: VBoxContainer
var fare_value: Label
var fare_support: Label
var fare_business: Label
var fare_apply: Button
var fare_switch: CheckButton
var fare_last: Label
var fare_key: Array = []
var fare_cached: Dictionary = {}
var results_empty: Control
var editor_panel: Control
var destination_key: Array = []
var destination_options: Dictionary = {}


func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var hint := _label("Luftlinie · vereinfachte Referenzreichweite, ohne Wetter oder Reserven.\nFlugplan: Pendelverkehr mit abwechselndem Hin- und Rückflug. Keine automatische Überführung.")
	add_child(hint)
	origin = _select("Startflughafen")
	destination = _select("Zielflughafen")
	var airports: Array = []
	for country in simulation.get_countries():
		airports.append_array(simulation.get_airports(country["code"]))
	airports.sort_custom(func(a: Dictionary, b: Dictionary): return a["iata"].naturalnocasecmp_to(b["iata"]) < 0)
	for airport in airports:
		origin.add_item("%s · Gebühren %d %% · %s (%s)" % [airport["iata"], airport["charge_percent"], _short_name(airport["name"]), airport["icao"]])
		origin.set_item_metadata(origin.item_count - 1, airport["icao"])
		origin.set_item_tooltip(origin.item_count - 1, _fee_tip(airport))
	origin.select(-1)
	destination.select(-1)
	aircraft = _select("Verfügbares Flugzeug")
	preview = _label("")
	add_child(preview)
	create_button = Button.new()
	create_button.text = "Route erstellen"
	create_button.theme_type_variation = "PrimaryButton"
	create_button.pressed.connect(_create)
	add_child(create_button)
	procurement_button = Button.new()
	procurement_button.text = "Flugzeug beschaffen"
	procurement_button.tooltip_text = "Öffnet Flotte › Beschaffung. Start- und Zielflughafen bleiben hier erhalten."
	procurement_button.visible = false
	procurement_button.pressed.connect(func(): procurement_requested.emit())
	add_child(procurement_button)
	reposition_button = Button.new()
	reposition_button.visible = false
	reposition_button.pressed.connect(func(): reposition_requested.emit(_selected_id(aircraft), origin.get_selected_metadata(), reposition_button))
	add_child(reposition_button)
	for select in [origin, destination, aircraft]:
		select.item_selected.connect(func(_index: int): _preview())
	status = _label("")
	add_child(status)
	empty_label = UI.empty_state("Noch keine Routen erstellt.", hint.text, create_button.text, func():
		if aircraft.item_count == 0:
			procurement_requested.emit()
			return
		get_parent().get_parent().ensure_control_visible(origin)
		origin.grab_focus()
	)
	empty_action = empty_label.get_child(0).get_child(2)
	add_child(empty_label)
	route_tree = UI.table(["Route", "Strecke", "Entfernung", "Flugzeug", tr("TRANSFER_MARKET_HEADER"), "Auslastung", tr("FARE_AUTOPILOT")], [65, 140, 110, 170, 320, 200, 145], 180)
	for i in range(6):
		route_tree.set_column_expand(i, i == 1 or i == 3)
	route_tree.item_selected.connect(_selection_changed)
	add_child(route_tree)
	connections_chip = UI.status_chip(tr("TRANSFER_FED"), "info")
	add_child(connections_chip)
	connections_label = _label("")
	add_child(connections_label)
	connections_empty = UI.empty_state(tr("TRANSFER_EMPTY"), tr("TRANSFER_HINT"), tr("TRANSFER_SELECT"), func(): route_tree.grab_focus())
	add_child(connections_empty)
	season_label = _label("")
	season_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	season_label.tooltip_text = "Skaliert den Basismarkt vor Preisreaktion und Sitzverteilung. 1,00 = Jahresmittel."
	add_child(season_label)
	assignment = _select("Ausgewählte Route: Flugzeugzuweisung")
	var actions := HFlowContainer.new()
	add_child(actions)
	assign_button = Button.new()
	assign_button.text = "Flugzeug hinzufügen"
	assign_button.pressed.connect(_assign)
	actions.add_child(assign_button)
	assign_ferry_button = Button.new()
	assign_ferry_button.visible = false
	assign_ferry_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			reposition_requested.emit(_selected_id(assignment), route["origin_icao"], assign_ferry_button))
	actions.add_child(assign_ferry_button)
	wind_down_button = Button.new()
	wind_down_button.text = tr("ROUTE_WIND_DOWN")
	wind_down_button.tooltip_text = tr("ROUTE_WIND_DOWN_TIP")
	wind_down_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.wind_down_route(route["id"]), tr("ROUTE_WIND_DOWN_DONE"))
	)
	actions.add_child(wind_down_button)
	delete_button = Button.new()
	delete_button.text = "Route löschen"
	delete_button.theme_type_variation = "DangerButton"
	delete_button.pressed.connect(_delete)
	actions.add_child(delete_button)
	release_label = _label("")
	release_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	release_label.visible = false
	add_child(release_label)
	assignment.item_selected.connect(func(_index: int): _assignment_preview())
	add_child(_label("Economy-Preis der ausgewählten Route · EUR pro Passagier und Einzelflug (vorläufig)"))
	ticket_input = SpinBox.new()
	ticket_input.get_line_edit().theme_type_variation = "NumericInput"
	ticket_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	ticket_input.min_value = 0
	ticket_input.max_value = 100000
	ticket_input.step = 1
	add_child(ticket_input)
	ticket_button = Button.new()
	ticket_button.text = "Ticketpreis übernehmen"
	ticket_button.tooltip_text = "Gilt ab dem nächsten Abflug; laufende Flüge behalten ihren Preis."
	ticket_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.set_ticket_price(route["id"], int(ticket_input.value)), "Ticketpreis geändert.", ticket_input)
	)
	add_child(ticket_button)
	fare_box = VBoxContainer.new()
	add_child(fare_box)
	fare_support = _label("")
	fare_value = UI.metric(fare_box, tr("FARE_TITLE"), fare_support)
	fare_business = _label("")
	fare_box.add_child(fare_business)
	fare_apply = Button.new()
	fare_apply.text = tr("FARE_APPLY")
	fare_apply.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			var error: String = simulation.set_ticket_price(route["id"], fare_cached["fare"])
			if error.is_empty() and fare_cached.get("business", -1) >= 0:
				error = simulation.set_business_ticket_price(route["id"], fare_cached["business"])
			_feedback(error, tr("FARE_APPLIED"))
	)
	fare_box.add_child(fare_apply)
	fare_switch = CheckButton.new()
	fare_switch.toggled.connect(func(enabled: bool):
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.set_pricing_autopilot(route["id"], enabled), "")
	)
	fare_box.add_child(fare_switch)
	fare_last = _label("")
	fare_box.add_child(fare_last)
	add_child(_label("Business-Preis · EUR pro Passagier und Einzelflug"))
	business_input = SpinBox.new()
	business_input.get_line_edit().theme_type_variation = "NumericInput"
	business_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	business_input.min_value = 0
	business_input.max_value = 100000
	business_input.step = 1
	add_child(business_input)
	business_button = Button.new()
	business_button.text = "Business-Preis übernehmen"
	business_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.set_business_ticket_price(route["id"], int(business_input.value)), "Business-Preis geändert.", business_input)
	)
	add_child(business_button)
	add_child(_label(tr("FREIGHT_RATE")))
	cargo_input = SpinBox.new()
	cargo_input.get_line_edit().theme_type_variation = "NumericInput"
	cargo_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	cargo_input.min_value = 0
	cargo_input.step = 0.01
	add_child(cargo_input)
	cargo_button = Button.new()
	cargo_button.text = "Frachtpreis übernehmen"
	cargo_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			cargo_input.apply()
			_feedback(simulation.set_cargo_rate(route["id"], roundi(cargo_input.value * 100.0)), "Frachtpreis geändert.", cargo_input)
	)
	add_child(cargo_button)
	cargo_status = _label("")
	add_child(cargo_status)
	service_select = _select("Bordservice · gilt ab dem nächsten Abflug")
	for level in ["Basic", "Standard", "Premium"]:
		service_select.add_item(level)
		service_select.set_item_metadata(service_select.item_count - 1, level.to_lower())
	service_select.item_selected.connect(func(index: int):
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.set_service_level(route["id"], service_select.get_item_metadata(index)), "Bordservice geändert.")
	)
	service_cost = _label("")
	add_child(service_cost)
	var rules := _label("Wirtschaftsregeln & Berechnung: Details im Tooltip")
	rules.tooltip_text = simulation.get_economy_rules()
	add_child(rules)
	add_child(_label("Flugplan für ausgewählte Route · erster Abflug in Spielminuten"))
	delay_input = SpinBox.new()
	delay_input.get_line_edit().theme_type_variation = "NumericInput"
	delay_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	delay_input.min_value = 1
	delay_input.max_value = 10080
	delay_input.value = 5
	add_child(delay_input)
	add_child(_label("Abstand pro Flugzeug in Spielminuten (mindestens Flug + Bodenzeit); mehrere Flugzeuge starten versetzt"))
	interval_input = SpinBox.new()
	interval_input.get_line_edit().theme_type_variation = "NumericInput"
	interval_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	interval_input.min_value = 1
	interval_input.max_value = 10080
	interval_input.value = 120
	add_child(interval_input)
	schedule_button = Button.new()
	schedule_button.text = "Flugplan aktivieren"
	schedule_button.pressed.connect(_schedule)
	add_child(schedule_button)
	stop_button = Button.new()
	stop_button.text = "Flugplan beenden"
	stop_button.tooltip_text = "Nur nach Ende der Bodenzeit möglich."
	stop_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.stop_schedule(route["id"]), "Flugplan beendet.")
	)
	add_child(stop_button)
	maintenance_label = _label("")
	maintenance_label.add_theme_color_override("font_color", UI.NEGATIVE)
	maintenance_label.visible = false
	add_child(maintenance_label)
	crew_button = Button.new()
	crew_button.text = tr("ROUTE_OPEN_STAFF")
	crew_button.theme_type_variation = "PrimaryButton"
	crew_button.hide()
	crew_button.pressed.connect(_open_staff_for_route)
	add_child(crew_button)
	operations_label = _label("")
	add_child(operations_label)
	_arrange_workspace()
	for control in [origin, destination, aircraft, assignment, service_select]:
		UI.field_from_label(control)
	for pair in [[ticket_input, "EUR"], [business_input, "EUR"], [cargo_input, "EUR/kg"], [delay_input, "min"], [interval_input, "min"]]:
		UI.field_from_label(pair[0], pair[1], ticket_button.tooltip_text if pair[0] == ticket_input else "")
	schedule_button.reparent(interval_input.get_parent())
	codeshare_panel = preload("res://scripts/codeshare_panel.gd").new()
	codeshare_panel.simulation = simulation
	add_child(codeshare_panel)
	alliance_panel = preload("res://scripts/alliance_panel.gd").new()
	alliance_panel.simulation = simulation
	add_child(alliance_panel)
	competition_label = _label("")
	competition_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	competition_tree = UI.table(["Airline", "Flugzeug", "Economy", "Business", "Abflüge/Tag je Richtung"], [150, 150, 100, 100, 180], 140)
	competition_tree.set_column_expand(0, true)
	competition_tree.set_column_expand(1, true)
	competition_empty = UI.empty_state("Keine Route ausgewählt", "Wähle eine Route im Register, um die öffentlichen Flugpläne der Konkurrenz auf dieser Strecke zu vergleichen.", "Route auswählen", func(): route_tree.grab_focus())
	var competition_panel := UI.panel(self, "05  /  WETTBEWERB · ÖFFENTLICHE FLUGPLÄNE")
	competition_panel.add_child(competition_label)
	competition_panel.add_child(competition_tree)
	competition_panel.add_child(competition_empty)
	refresh()


const OPS_KINDS := {
	"airborne": ["OPS_STATE_AIRBORNE", "info"], "ground": ["OPS_STATE_GROUND", "neutral"],
	"scheduled": ["OPS_STATE_SCHEDULED", "neutral"], "weather_delay": ["OPS_STATE_WEATHER", "caution"],
	"maintenance": ["OPS_STATE_MAINTENANCE", "caution"], "technician_wait": ["OPS_STATE_TECH_WAIT", "stop"], "maintenance_pending": ["OPS_STATE_MAINT_PENDING", "caution"], "release": ["OPS_STATE_RELEASE", "caution"], "crew_blocked": ["OPS_STATE_CREW_BLOCKED", "stop"],
	"no_plan": ["OPS_STATE_NO_PLAN", "neutral"],
}


func _refresh_operations_table(metrics: Dictionary) -> void:
	var rows: Array = simulation.get_operations()
	operations_tree.visible = not rows.is_empty()
	operations_empty.visible = rows.is_empty()
	var signature := str([rows, metrics.values().map(func(r): return [r["id"], r["punctuality_percent"], r["punctuality_flights"]]), TranslationServer.get_locale()])
	if signature == _operations_signature:
		return
	_operations_signature = signature
	operations_tree.clear()
	var root := operations_tree.create_item()
	for operation: Dictionary in rows:
		var row := operations_tree.create_item(root)
		var route: Dictionary = metrics.get(operation["route_id"], {})
		UI.cell_text(row, 0, operation["identifier"])
		UI.cell_text(row, 1, "%s → %s" % [operation["origin"], operation["destination"]])
		var kind: Array = OPS_KINDS[operation["state_kind"]]
		UI.cell_status(row, 2, tr(kind[0]), kind[1])
		var blocked: bool = operation["crew_blocked"] and operation["state_kind"] in ["crew_blocked", "airborne"] and not operation["maintenance_pending"]
		var next_text: String = operation["next_departure"]
		if operation["technician_wait"]:
			next_text = tr("OPS_NEXT_TECH_WAIT")
		elif operation["maintenance_pending"]:
			next_text = tr("OPS_NEXT_MAINT_PENDING")
		elif operation["state_kind"] == "maintenance":
			next_text = tr("OPS_NEXT_RELEASE") % operation["maintenance_release"]
		UI.cell_text(row, 3, tr("OPS_NEXT_CREW_BLOCKED") if blocked else next_text)
		UI.cell_text(row, 4, operation["expected_landing"])
		var delay_minutes := ceili(operation["delay_seconds"] / 60.0)
		UI.cell_text(row, 5, ("%d min" % delay_minutes) if delay_minutes > 0 else "—")
		var punctuality := "—" if route.get("punctuality_percent", -1.0) < 0 else "%.1f %%" % route["punctuality_percent"]
		var tip := "%s\n%s" % [operation["status_text"], tr("OPS_ROUTE_TIP") % [operation["route_id"], punctuality, route.get("punctuality_flights", 0)]]
		for column in range(6):
			row.set_tooltip_text(column, tip)
		row.set_metadata(0, operation["route_id"])


func _arrange_workspace() -> void:
	var original := get_children()
	var grid := VBoxContainer.new()
	grid.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	add_child(grid)
	var network := UI.panel(grid, "01  /  ROUTENREGISTER & LIVE-BETRIEB")
	var ops_panel := UI.panel(grid, tr("OPS_PANEL_TITLE"))
	var planner := UI.panel(grid, "02  /  VERBINDUNG PLANEN")
	var editor := UI.panel(self, "03  /  AUSGEWÄHLTE ROUTE · ZUWEISUNG & FLUGPLAN")
	editor_panel = editor.get_parent()
	var editor_grid := GridContainer.new()
	editor_grid.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	editor_grid.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	editor.add_child(editor_grid)
	editor_grid.resized.connect(func(): editor_grid.columns = 3 if editor_grid.size.x >= 1100 else 1)
	var editor_columns: Array[VBoxContainer] = []
	for i in range(3):
		var column := VBoxContainer.new()
		column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		editor_grid.add_child(column)
		editor_columns.append(column)
	var assignment_title: Node = original[original.find(assignment) - 1]
	var ticket_title: Node = original[original.find(ticket_input) - 1]
	var schedule_title: Node = original[original.find(delay_input) - 1]
	var result_panel := UI.panel(self, "04  /  FLUGERGEBNISSE · LETZTE 100 LANDUNGEN")
	var target := planner
	for node in original:
		if node == empty_label:
			target = network
		if node == assignment_title:
			target = editor_columns[0]
		if node == ticket_title:
			target = editor_columns[1]
		if node == schedule_title:
			target = editor_columns[2]
		if node == operations_label:
			target = ops_panel
		node.reparent(target)
	operations_tree = UI.table([tr("OPS_COL_AIRCRAFT"), tr("OPS_COL_DIRECTION"), tr("OPS_COL_STATE"), tr("OPS_COL_NEXT"), tr("OPS_COL_LANDING"), tr("OPS_COL_DELAY")], [110, 120, 130, 150, 150, 90], 160)
	ops_panel.add_child(operations_tree)
	operations_empty = UI.empty_state(tr("OPS_EMPTY_TITLE"), tr("OPS_EMPTY_TEXT"), tr("OPS_EMPTY_ACTION"), func():
		get_parent().get_parent().ensure_control_visible(origin)
		origin.grab_focus()
	)
	ops_panel.add_child(operations_empty)
	results_tree = UI.table(["Landung / Route", "Strecke", "Pax / Auslastung", "Gesamtumsatz", "Flugkosten", "Ergebnis", "Fracht kg / EUR (in Umsatz)"], [170, 145, 135, 120, 110, 110, 260], 240)
	result_panel.add_child(results_tree)
	results_empty = UI.empty_state(tr("ROUTE_RESULTS_EMPTY"), tr("ROUTE_RESULTS_START"), tr("OPS_EMPTY_ACTION"), func():
		if results_action.is_valid():
			results_action.call())
	var empty_column := results_empty.get_child(0)
	results_empty_text = empty_column.get_child(1)
	results_empty_button = empty_column.get_child(2)
	result_panel.add_child(results_empty)


func _update_results_empty(route: Dictionary, has_routes: bool) -> void:
	var description: String
	var action := ""
	results_action = Callable()
	if not has_routes:
		if procurement_button.visible:
			description = tr("ROUTE_RESULTS_PROCUREMENT")
			action = tr("ROUTE_RESULTS_BUY")
			results_action = func(): procurement_requested.emit()
		else:
			description = tr("ROUTE_RESULTS_START")
			action = tr("OPS_EMPTY_ACTION")
			results_action = _focus_results_target.bind(destination if origin.selected >= 0 else origin)
	elif route.is_empty():
		description = tr("ROUTE_RESULTS_SELECT")
		action = tr("ROUTE_RESULTS_PICK")
		results_action = _focus_results_target.bind(route_tree)
	elif route["has_plan"]:
		description = tr("ROUTE_RESULTS_ACTIVE")
	elif route["aircraft_ids"].is_empty():
		if assignment.item_count > 0:
			description = tr("ROUTE_RESULTS_ASSIGN")
			action = tr("ROUTE_RESULTS_ASSIGN_ACTION")
			results_action = _focus_results_target.bind(assignment)
		else:
			description = tr("ROUTE_RESULTS_NO_AIRCRAFT")
	elif not route["maintenance_status"].is_empty():
		description = route["maintenance_status"]
	elif not route["staff_status"].is_empty():
		description = route["staff_status"]
		action = tr("ROUTE_OPEN_STAFF")
		results_action = _open_staff_for_route
	elif route["can_schedule"]:
		description = tr("ROUTE_RESULTS_SCHEDULE")
		action = tr("ROUTE_RESULTS_EDIT")
		results_action = _focus_results_target.bind(delay_input.get_line_edit())
	else:
		description = tr("ROUTE_RESULTS_BLOCKED")
	results_empty_text.text = description
	results_empty_button.text = action
	results_empty_button.visible = results_action.is_valid()


func _focus_results_target(target: Control) -> void:
	target.grab_focus()
	if target == delay_input.get_line_edit():
		await get_tree().process_frame
		if target.has_focus():
			var scroll := get_parent().get_parent() as ScrollContainer
			scroll.ensure_control_visible(delay_input.get_parent())


func _open_staff_for_route() -> void:
	var aircraft_ids: Array = _route().get("aircraft_ids", [])
	for plane in simulation.get_staff_overview()["aircraft"]:
		if plane["id"] in aircraft_ids and plane["state"] == "short":
			staff_requested.emit(plane["id"])
			return


func _label(text: String) -> Label:
	var label := UI.label(text, ThemeTokens.SMALL, UI.MUTED)
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return label


func _select(title: String) -> OptionButton:
	add_child(_label(title))
	var select := OptionButton.new()
	select.fit_to_longest_item = false
	select.clip_text = true
	select.get_popup().max_size = Vector2i(600, 280)
	select.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_child(select)
	return select


func _selected_id(select: OptionButton) -> int:
	return int(select.get_selected_metadata()) if select.selected >= 0 else 0


func _route() -> Dictionary:
	var row := route_tree.get_selected()
	return row.get_metadata(0) if row != null else {}


func refresh() -> void:
	var selected_route: int = _route().get("id", 0)
	var selected_plane := _selected_id(aircraft)
	var fleet: Array = simulation.get_fleet()
	aircraft.clear()
	for plane in fleet:
		if plane["route_id"] == 0:
			aircraft.add_item("%s · %s · %d km Reichweite" % [plane["identifier"], plane["model_name"], plane["range_km"]])
			aircraft.set_item_metadata(aircraft.item_count - 1, plane["id"])
			if plane["id"] == selected_plane:
				aircraft.select(aircraft.item_count - 1)
	aircraft.disabled = aircraft.item_count == 0
	empty_action.text = "Flugzeug beschaffen" if aircraft.item_count == 0 else create_button.text
	var routes: Array = simulation.get_routes()
	empty_label.visible = routes.is_empty()
	create_button.theme_type_variation = "Button" if routes.is_empty() else "PrimaryButton"
	route_tree.visible = not routes.is_empty()
	route_tree.clear()
	var root_item := route_tree.create_item()
	for route in routes:
		var row := route_tree.create_item(root_item)
		row.set_metadata(0, route)
		UI.cell_status(row, 6, tr("FARE_ON") if route.get("pricing_autopilot", false) else "—", "info" if route.get("pricing_autopilot", false) else "neutral")
		UI.cell_text(row, 0, "#%d" % route["id"])
		UI.cell_text(row, 1, "%s–%s" % [route["origin_icao"], route["destination_icao"]])
		UI.cell_text(row, 2, "%.1f km" % route["distance_km"])
		var assigned_names: PackedStringArray = []
		for plane in fleet:
			if plane["id"] in route["aircraft_ids"]:
				assigned_names.append("%s · %s" % [plane["identifier"], plane["model_name"]])
		UI.cell_text(row, 3, ", ".join(assigned_names) if not assigned_names.is_empty() else "Nicht zugewiesen")
		if not String(route["maintenance_status"]).is_empty():
			UI.cell_text(row, 3, row.get_text(3) + " · Wartung")
		for i in range(6):
			row.set_tooltip_text(i, row.get_text(i))
		if route["id"] == selected_route:
			row.select(0)
	_selection_changed()
	refresh_operations()


func refresh_operations() -> void:
	_preview()
	_assignment_preview(false)
	# Refresh live metrics without rebuilding selection or resetting ticket inputs.
	var metrics: Dictionary = {}
	for route in simulation.get_routes():
		metrics[route["id"]] = route
	season_label.visible = not metrics.is_empty()
	if not metrics.is_empty():
		var season: Dictionary = metrics.values()[0]
		season_label.text = tr("DEMAND_SEASON_FACTORS") % [UI.decimal(season["season_business_factor"], 2), UI.decimal(season["season_leisure_factor"], 2)]
		season_label.text += "\n" + tr("DEMAND_ECONOMY_FACTORS") % [UI.decimal(season["economy_business_factor"], 2), UI.decimal(season["economy_leisure_factor"], 2), UI.decimal(season["economy_cargo_factor"], 2)]
		var focus: Dictionary = metrics.get(_route().get("id", 0), season)
		var ev := [focus["event_business_factor"], focus["event_leisure_factor"], focus["event_cargo_factor"]]
		if ev != [1.0, 1.0, 1.0]:
			season_label.text += "\n" + tr("DEMAND_EVENT_FACTORS") % [UI.decimal(ev[0], 2), UI.decimal(ev[1], 2), UI.decimal(ev[2], 2)]
			if not focus["event_places"].is_empty():
				season_label.text += " · " + tr("DEMAND_EVENT_NAMES") % ", ".join(focus["event_places"])
	var tree_root := route_tree.get_root()
	if tree_root != null:
		for row in tree_root.get_children():
			var route: Dictionary = metrics.get(row.get_metadata(0)["id"], {})
			if route.is_empty():
				continue
			if route.get("aircraft_ids", []) != row.get_metadata(0).get("aircraft_ids", []):
				refresh.call_deferred()
				return
			row.set_metadata(0, route)
			UI.cell_status(row, 6, tr("FARE_ON") if route.get("pricing_autopilot", false) else "—", "info" if route.get("pricing_autopilot", false) else "neutral")
			var reason: String = route["maintenance_status"] if not String(route["maintenance_status"]).is_empty() else route["staff_status"]
			var names: PackedStringArray = []
			for plane in simulation.get_fleet():
				if plane["id"] in route["aircraft_ids"]: names.append("%s · %s" % [plane["identifier"], plane["model_name"]])
			var assigned := ", ".join(names) if not names.is_empty() else "Nicht zugewiesen"
			var state := reason if not reason.is_empty() else "Flugplan aktiviert." if route["has_plan"] else "Reichweite ausreichend"
			UI.cell_status(row, 3, assigned + (" · " + state if not names.is_empty() else ""), "stop" if not reason.is_empty() or names.is_empty() else "go" if route["has_plan"] else "neutral")
			UI.cell_text(row, 4, (tr("FREIGHT_ROUTE_MARKET") % [route["cargo_daily_demand_kg"], UI.money(route["actual_cargo_revenue_eur"])]) if route.get("freighter_only", false) else "%d / %d / %d" % [route["base_daily_market"], route["expected_daily_local"], route["expected_daily_connecting"]])
			row.set_tooltip_text(0, "Pünktlichkeit: %s · %d erfasste Flüge" % ["—" if route["punctuality_percent"] < 0 else "%.1f %%" % route["punctuality_percent"], route["punctuality_flights"]])
			row.set_tooltip_text(4, (row.get_text(4) + "\n" + tr("FREIGHT_LOAD_TOOLTIP")) if route.get("freighter_only", false) else "Pro Tag und Richtung: lokaler Basismarkt / erwartete lokale Passagiere / Umsteiger (aus ihren eigenen O&D-Märkten).\nOhne eigenen Flugplan: keine Beförderung, kein Sitzmangel.\nDurchschnitt pro Abflug: %d Pax" % route["estimated_passengers"])
			for connection in route["connections"]:
				row.set_tooltip_text(4, row.get_tooltip_text(4) + "\n%s – %s – %s · %.1f Pax/Tag" % [connection["origin"], connection["hub"], connection["destination"], connection["daily"]])
			UI.cell_text(row, 5, ("%d kg · %.1f %%" % [route["estimated_cargo_kg"], route["freight_load_percent"]]) if route.get("freighter_only", false) else "%.1f %%" % route["actual_load_percent"] if route["actual_flights"] > 0 else "Keine Flüge")
			row.set_tooltip_text(5, tr("FREIGHT_LOAD_TOOLTIP") if route.get("freighter_only", false) else "Tatsächlich beförderte Passagiere / angebotene Sitze aller archivierten Landungen dieser Route. Bei alten Spielständen ab Archivbeginn.")
			if not String(route["maintenance_status"]).is_empty():
				row.set_tooltip_text(3, row.get_text(3) + "\n" + route["maintenance_status"])
			elif not String(route["staff_status"]).is_empty():
				row.set_tooltip_text(3, row.get_text(3) + "\n" + route["staff_status"])
	var selected: Dictionary = metrics.get(_route().get("id", 0), {})
	_update_fare(selected)
	_update_connections(selected)
	_update_schedule_controls(selected)
	_update_service_cost(selected)
	var lines: PackedStringArray = []
	for route in metrics.values():
		lines.append("Route #%d · Pünktlichkeit: %s · %d erfasste Flüge" % [route["id"], "—" if route["punctuality_percent"] < 0 else "%.1f %%" % route["punctuality_percent"], route["punctuality_flights"]])
	operations_label.text = "\n".join(lines)
	operations_label.visible = not lines.is_empty()
	_refresh_operations_table(metrics)
	_refresh_competition()
	results_tree.clear()
	var root_item := results_tree.create_item()
	var results: Array = simulation.get_flight_results()
	results_tree.visible = not results.is_empty()
	results_empty.visible = results.is_empty()
	if results_empty.visible:
		_update_results_empty(selected, not metrics.is_empty())
	results_tree.set_meta("breakdowns", {})
	for result in results:
		var row := results_tree.create_item(root_item)
		var values := ["%s / #%d" % [result["landing"], result["route_id"]], "%s–%s" % [result["origin"], result["destination"]], "%d/%d · %.1f %%" % [result["passengers"], result["seats"], result["load_percent"]], UI.money(result["revenue_eur"]), UI.money(result["costs_eur"]), UI.money(result["profit_eur"])]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
		UI.set_breakdown(results_tree, row, 3, UI.revenue_parts(result))
		UI.set_breakdown(results_tree, row, 4, UI.cost_parts(result))
		UI.cell_text(row, 2, "Y %d/%d · J %d/%d" % [result["economy_passengers"], result["economy_seats"], result["business_passengers"], result["business_seats"]])
		row.set_tooltip_text(2, tr("TRANSFER_RESULT") % [result["local_passengers"], result["connecting_passengers"]] + "\n" + tr("TRANSFER_REVENUE") % UI.money(result["connecting_revenue_eur"]) + "\n" + "Economy: %s · Business: %s" % [UI.money(result["economy_revenue_eur"]), UI.money(result["business_revenue_eur"])])
		UI.cell_text(row, 6, "%d kg / %s" % [result["cargo_kg"], UI.money(result["cargo_revenue_eur"])])
		UI.cell_result(row, 5, result["profit_eur"])


func reset_for_world() -> void:
	var home: String = simulation.get_airline_info().get("airport_icao", "")
	origin.select(-1)
	for i in range(origin.item_count):
		if origin.get_item_metadata(i) == home:
			origin.select(i)
	destination.select(-1)
	destination_key = []
	aircraft.clear()
	route_tree.deselect_all()
	refresh()
	status.text = ""


func _short_name(name: String, limit := 32) -> String:
	return name.left(limit) + ("…" if name.length() > limit else "")


func _fee_tip(airport: Dictionary) -> String:
	return "%s · Gebührenstufe %d %% des Modell-/Passagiertarifs an diesem Endpunkt; Start und Ziel werden addiert." % [airport["name"], airport["charge_percent"]]


func sync_destinations() -> void:
	var origin_icao: String = origin.get_selected_metadata() if origin.selected >= 0 else ""
	var plane := _selected_id(aircraft)
	var key := [origin_icao, plane]
	if key == destination_key:
		return
	destination_key = key
	var kept: String = destination.get_selected_metadata() if destination.selected >= 0 else ""
	destination.clear()
	destination_options.clear()
	var options: Array = []
	if origin_icao.is_empty():
		for country in simulation.get_countries():
			options.append_array(simulation.get_airports(country["code"]))
		options.sort_custom(func(a: Dictionary, b: Dictionary): return a["iata"].naturalnocasecmp_to(b["iata"]) < 0)
		for airport in options:
			airport["ranged"] = false
			airport["in_range"] = true
	else:
		options = simulation.get_route_destinations(origin_icao, plane)
	for option in options:
		var text := "%s (%s) · " % [option["iata"], option["icao"]]
		if not origin_icao.is_empty():
			text += "%s km · " % UI.decimal(option["distance_km"], 0)
		if option["ranged"] and not option["in_range"]:
			text += "außer Reichweite · "
		text += "Gebühren %d %% · %s" % [option["charge_percent"], _short_name(option["name"])]
		destination.add_item(text)
		destination.set_item_metadata(destination.item_count - 1, option["icao"])
		destination.set_item_tooltip(destination.item_count - 1, _fee_tip(option) + (" Außerhalb der Referenzreichweite des gewählten Flugzeugs." if option["ranged"] and not option["in_range"] else ""))
		destination_options[option["icao"]] = option
		if option["icao"] == kept:
			destination.select(destination.item_count - 1)
	if kept.is_empty() or not destination_options.has(kept):
		destination.select(-1)


func _in_range_count() -> int:
	var in_range := 0
	for option in destination_options.values():
		if option["in_range"]:
			in_range += 1
	return in_range


func _update_procurement_shortcut() -> void:
	var range_blocked := origin.selected >= 0 and _selected_id(aircraft) != 0 and _in_range_count() == 0
	procurement_button.visible = aircraft.item_count == 0 or range_blocked


func _range_hint() -> String:
	var plane := _selected_id(aircraft)
	if origin.selected < 0 or plane == 0:
		return ""
	var in_range := _in_range_count()
	var plane_name := aircraft.get_item_text(aircraft.selected)
	var origin_icao: String = origin.get_selected_metadata()
	if in_range == 0:
		return "Von %s aus liegt mit %s kein Zielflughafen des Katalogs in Reichweite. Wähle ein Flugzeug mit größerer Reichweite oder sieh im Flotte-Tab unter Beschaffung nach." % [origin_icao, plane_name]
	return "%d Zielflughäfen in Reichweite von %s stehen zuerst, nächster zuerst; weiter entfernte sind als außer Reichweite markiert." % [in_range, origin_icao]


func _preview() -> void:
	sync_destinations()
	if origin.selected < 0 or destination.selected < 0:
		preview.text = "Wähle Start- und Zielflughafen, um Entfernung und Reichweite zu prüfen." if origin.selected < 0 else "Zielflughafen wählen: Startflughafen ist %s." % origin.get_selected_metadata()
		var hint := _range_hint()
		if not hint.is_empty():
			preview.text += "\n" + hint
		UI.field_error(aircraft, "")
		create_button.disabled = true
		if aircraft.item_count == 0:
			preview.text += "\nKein freies Flugzeug. Im Flotte-Tab kaufen oder eine Zuweisung lösen."
		_update_procurement_shortcut()
		reposition_button.visible = false
		return
	var result: Dictionary = simulation.preview_route(origin.get_selected_metadata(), destination.get_selected_metadata(), _selected_id(aircraft), 0)
	preview.text = "Entfernung: %.1f km · %s" % [result["distance_km"], "Reichweite ausreichend" if result["error"].is_empty() else result["error"]]
	preview.text += "\n%s · %s" % [result["origin_slots"], result["destination_slots"]]
	if result["has_cost_estimate"]:
		preview.text += "\nPlanung: %d Pax · Kosten %s, davon Flughäfen Start + Ziel %s je Abflug.\nJahresmittel, Standardpreise, 1 Abflug/Tag/Richtung; Personal und Wartungschecks separat." % [result["estimated_passengers"], UI.money(result["estimated_costs_eur"]), UI.money(result["estimated_airport_eur"])]
	UI.field_error(aircraft, result["error"])
	create_button.disabled = not result["error"].is_empty()
	reposition_button.visible = result["needs_reposition"]
	if result["needs_reposition"]:
		var plane_name := aircraft.get_item_text(aircraft.selected).get_slice(" · ", 0)
		reposition_button.text = "%s von %s nach %s überführen" % [plane_name, result["aircraft_location"], origin.get_selected_metadata()]
		reposition_button.tooltip_text = "Öffnet die Überführungsbestätigung mit Kosten und Dauer. Danach und nach der Bodenzeit kann die Route aus diesem Entwurf erstellt werden."
		preview.text += "\nAblauf: Überführung bestätigen, landen, Bodenzeit abwarten, dann Route erstellen. Oder Startflughafen bzw. Flugzeug ändern."
	if aircraft.item_count == 0:
		preview.text += "\nKein freies Flugzeug. Im Flotte-Tab kaufen oder eine Zuweisung lösen."
	_update_procurement_shortcut()


func _selection_changed() -> void:
	fare_key.clear()
	var route := _route()
	status.text = ""
	if ticket_button != null:
		ticket_button.disabled = route.is_empty() or route.get("freighter_only", false)
		ticket_input.editable = not ticket_button.disabled
		ticket_input.value = route.get("ticket_eur", 100)
		cargo_button.disabled = route.is_empty()
		cargo_input.editable = not route.is_empty()
		cargo_input.max_value = route.get("cargo_max_rate_cents", 0) / 100.0
		cargo_input.value = route.get("cargo_rate_cents", 0) / 100.0
		business_input.value = route.get("business_ticket_eur", 200)
		business_button.disabled = ticket_button.disabled
		business_input.editable = not ticket_button.disabled
		service_select.disabled = ticket_button.disabled
		for i in range(service_select.item_count):
			if service_select.get_item_metadata(i) == route.get("service_level", "standard"):
				service_select.select(i)
		_update_service_cost(route)
	if schedule_button != null:
		var route_id: int = route.get("id", 0)
		if route_id != schedule_route_id:
			schedule_route_id = route_id
			UI.field_error(interval_input, "")
		_update_schedule_controls(route)
	var selected_assignment := _selected_id(assignment)
	assignment.clear()
	for plane in simulation.get_fleet():
		if plane["route_id"] == 0 or plane["route_id"] == route.get("id", -1):
			var assigned: bool = plane["id"] in route.get("aircraft_ids", [])
			assignment.add_item("%s · %s · %s" % [plane["identifier"], plane["model_name"], "Entfernen" if assigned else "Hinzufügen"])
			assignment.set_item_metadata(assignment.item_count - 1, plane["id"])
			if plane["id"] == selected_assignment:
				assignment.select(assignment.item_count - 1)
	assignment.disabled = route.is_empty()
	_update_wind_down(route)
	_update_fare(route)
	_update_connections(route)
	_assignment_preview()
	_refresh_competition()
	if results_empty.visible:
		_update_results_empty(route, route_tree.visible)


func _update_connections(route: Dictionary) -> void:
	var connections: Array = route.get("connections", [])
	connections_chip.visible = route.get("transfer_fed", false)
	connections_empty.visible = connections.is_empty() and not route.get("freighter_only", false)
	connections_label.visible = not connections.is_empty()
	var lines: PackedStringArray = [tr("TRANSFER_COUNTS") % [route.get("origin_icao", ""), route.get("destination_icao", ""), route.get("expected_daily_local", 0), route.get("expected_daily_connecting", 0)], tr("TRANSFER_FEEDS")]
	for connection in connections:
		var partner: String = connection.get("partner", "")
		lines.append((tr("TRANSFER_PATH") % [connection["origin"], connection["hub"], connection["destination"], connection["daily"]]) + (" · " + tr("CODESHARE_VIA") % partner if not partner.is_empty() else ""))
	connections_label.text = "\n".join(lines)


func _update_service_cost(route: Dictionary) -> void:
	cargo_status.text = "" if route.is_empty() else "Frachtmarkt je Richtung: %d kg/Tag · bisher befördert: %d kg · %s Umsatz. Passagiere und Gepäck haben Vorrang; hohe Preise verringern die Nachfrage." % [route["cargo_daily_demand_kg"], route["actual_cargo_kg"], UI.money(route["actual_cargo_revenue_eur"])]
	if route.get("freighter_only", false):
		cargo_status.text = tr("FREIGHT_RESULTS") % [route["cargo_daily_demand_kg"], route["estimated_cargo_kg"], route["freight_load_percent"], route["actual_cargo_kg"], UI.money(route["actual_cargo_revenue_eur"])]
	for control in [ticket_input, ticket_button, business_input, business_button, service_select]:
		control.tooltip_text = tr("FREIGHTER_TOOLTIP") if route.get("freighter_only", false) else ""
	service_cost.text = "" if route.is_empty() or route.get("freighter_only", false) else "Flughafengebühren Start + Ziel: %s je Abflug (Ø aktive Flugzeuge; ohne Plan 0).\nBordservice gegenüber Standard: Y %s / J %s je Passagier · erwartet %s je Abflug (Ø aktive Flugzeuge) · bisher %s.\nStandard ist in den Grundkosten enthalten; Basic spart, Premium kostet zusätzlich. Laufende Flüge bleiben unverändert." % [UI.money(route["estimated_airport_eur"]), UI.money(route["service_economy_eur"]), UI.money(route["service_business_eur"]), UI.money(route["estimated_service_eur"]), UI.money(route["actual_service_eur"])]
	if not route.is_empty() and not route.get("freighter_only", false):
		service_cost.text += "\n" + tr("ANCILLARY_ROUTE_REVENUE") % UI.money(route["actual_ancillary_revenue_eur"])


# Maintenance starts and ends while time runs, so this follows live route data.
func _update_schedule_controls(route: Dictionary) -> void:
	if schedule_button == null:
		return
	var hangar: String = route.get("maintenance_status", "")
	var crew: String = route.get("staff_status", "")
	schedule_button.disabled = route.is_empty() or not route.get("can_schedule", false)
	schedule_button.text = "Flugplan gesperrt · Wartung" if not hangar.is_empty() and not route.get("has_plan", false) else "Flugplan gesperrt · Besatzung fehlt" if not crew.is_empty() else "Flugplan aktivieren"
	stop_button.disabled = route.is_empty() or not route.get("has_plan", false)
	_update_wind_down(route)
	var notice := hangar
	if not crew.is_empty():
		notice = crew if notice.is_empty() else notice + "\n" + crew
	maintenance_label.text = notice
	maintenance_label.visible = not notice.is_empty()
	crew_button.visible = not crew.is_empty() and not route.get("has_plan", false)


func _update_wind_down(route: Dictionary) -> void:
	var pending: Array = route.get("release_pending", [])
	wind_down_button.disabled = route.is_empty() or not route.get("can_wind_down", false)
	delete_button.disabled = route.is_empty() or not route.get("can_delete", true)
	var lines: PackedStringArray = []
	for release: Dictionary in pending:
		lines.append(tr("ROUTE_RELEASE_LINE") % [release["identifier"], release["at"]])
	if not pending.is_empty():
		lines.append(tr("ROUTE_DELETE_AT") % route["delete_available_at_text"])
	elif route.get("has_plan", false):
		lines.append(tr("ROUTE_DELETE_BLOCKED"))
	release_label.text = "\n".join(lines)
	release_label.visible = not lines.is_empty()
	delete_button.tooltip_text = "" if not delete_button.disabled else tr("ROUTE_DELETE_BLOCKED")


static func rival_services(routes: Array, a: String, b: String) -> Array:
	var result: Array = []
	for service in routes:
		var o: String = service["origin_icao"]
		var d: String = service["destination_icao"]
		if (o == a and d == b) or (o == b and d == a):
			result.append(service)
	result.sort_custom(func(x, y): return [x["airline"], x["model"]] < [y["airline"], y["model"]])
	return result


func _set_competition_empty(title: String, text: String) -> void:
	competition_tree.visible = false
	competition_empty.visible = true
	var labels := competition_empty.find_children("*", "Label", true, false)
	labels[0].text = title
	labels[1].text = text


func _refresh_competition() -> void:
	if codeshare_panel != null: codeshare_panel.refresh()
	if alliance_panel != null: alliance_panel.refresh()
	if competition_label == null:
		return
	var route := _route()
	competition_tree.clear()
	if route.is_empty():
		competition_label.visible = false
		_set_competition_empty("Keine Route ausgewählt", "Wähle eine Route im Register, um die öffentlichen Flugpläne der Konkurrenz auf dieser Strecke zu vergleichen.")
		return
	var live_route: Dictionary = route
	for current in simulation.get_routes():
		if current["id"] == route["id"]:
			live_route = current
	competition_label.visible = true
	competition_label.text = "%s ↔ %s · Schätzung pro Tag und Richtung: gemeinsamer Basismarkt ca. %d Pax vor Preisreaktion. Keine garantierten Buchungen." % [route["origin_icao"], route["destination_icao"], live_route["base_daily_market"]]
	var services := rival_services(simulation.get_competitor_routes(), route["origin_icao"], route["destination_icao"])
	if services.is_empty():
		_set_competition_empty("Kein Wettbewerber auf dieser Strecke", "Keine andere Airline veröffentlicht einen Flugplan zwischen %s und %s (in beide Richtungen geprüft)." % [route["origin_icao"], route["destination_icao"]])
		return
	competition_empty.visible = false
	competition_tree.visible = true
	var root := competition_tree.create_item()
	for service in services:
		var row := competition_tree.create_item(root)
		var reverse: bool = service["origin_icao"] != route["origin_icao"]
		UI.cell_text(row, 0, service["airline"])
		UI.cell_text(row, 1, service["model"])
		UI.cell_text(row, 2, "%d EUR" % service["ticket_eur"])
		UI.cell_text(row, 3, "%d EUR" % service["business_ticket_eur"] if service["business_ticket_eur"] > 0 else "—")
		UI.cell_text(row, 4, "%.1f ×" % service["departures_per_day_per_direction"])
		if reverse:
			row.set_tooltip_text(0, "Veröffentlicht als %s → %s" % [service["origin_icao"], service["destination_icao"]])


func _assignment_preview(report := true) -> void:
	var route := _route()
	assign_button.disabled = route.is_empty() or assignment.item_count == 0
	assign_button.tooltip_text = ""
	assign_ferry_button.visible = false
	var removing: bool = _selected_id(assignment) in route.get("aircraft_ids", [])
	assign_button.text = "Flugzeug entfernen" if removing else "Flugzeug hinzufügen"
	if removing:
		assign_button.tooltip_text = "Laufender Flug und Bodenzeit werden beendet; andere Flugzeuge fliegen weiter."
		return
	if not route.is_empty() and _selected_id(assignment) != 0:
		var result: Dictionary = simulation.preview_route(route["origin_icao"], route["destination_icao"], _selected_id(assignment), route["id"])
		assign_button.disabled = not result["error"].is_empty()
		assign_button.tooltip_text = result["error"]
		if result["needs_reposition"]:
			var plane_name := assignment.get_item_text(assignment.selected).get_slice(" · ", 0)
			assign_ferry_button.text = "%s von %s nach %s überführen" % [plane_name, result["aircraft_location"], route["origin_icao"]]
			assign_ferry_button.tooltip_text = "Öffnet die Überführungsbestätigung mit Kosten und Dauer. Nach Landung und Bodenzeit kann das Flugzeug hier hinzugefügt werden."
			assign_ferry_button.visible = true
		if report or not result["error"].is_empty():
			status.text = result["error"]


func _schedule() -> void:
	var route := _route()
	if route.is_empty() or schedule_button.disabled:
		return
	var error: String = simulation.schedule_route(route["id"], int(delay_input.value), int(interval_input.value))
	refresh()
	UI.field_error(interval_input, error)
	if error.is_empty():
		status.text = "Flugplan aktiviert. Bitte speichern."
		changed.emit()
	# The inline message can change the height of the schedule editor.
	for frame in range(2):
		await get_tree().process_frame
	if not is_visible_in_tree() or _route().get("id", 0) != route["id"]:
		return
	if error.is_empty():
		stop_button.grab_focus()
	else:
		var input := interval_input.get_line_edit()
		input.grab_focus()
		await get_tree().process_frame
		if input.has_focus():
			var scroll := get_parent().get_parent() as ScrollContainer
			scroll.ensure_control_visible(schedule_button.get_parent())


func _feedback(error: String, success: String, field: Control = null, select_id := 0) -> void:
	refresh()
	if select_id != 0:
		var row := route_tree.get_root().get_first_child()
		while row != null and row.get_metadata(0)["id"] != select_id:
			row = row.get_next()
		if row != null:
			row.select(0)
			_selection_changed()
	if field != null: UI.field_error(field, error)
	status.text = success + " Bitte speichern." if error.is_empty() else error
	if error.is_empty():
		changed.emit()
	await get_tree().process_frame
	var scroll := get_parent().get_parent() as ScrollContainer
	if select_id != 0 and error.is_empty():
		await get_tree().process_frame
	if select_id != 0 and error.is_empty() and ticket_input.get_line_edit().is_visible_in_tree():
		scroll.scroll_vertical += int(editor_panel.global_position.y - scroll.global_position.y)
		await get_tree().process_frame
		scroll.ensure_control_visible(ticket_input)
		ticket_input.get_line_edit().grab_focus()
		return
	scroll.ensure_control_visible(status)


func _create() -> void:
	if origin.selected < 0 or destination.selected < 0:
		return
	var known: Array = []
	for route in simulation.get_routes():
		known.append(route["id"])
	var error: String = simulation.create_route(origin.get_selected_metadata(), destination.get_selected_metadata(), _selected_id(aircraft))
	var created := 0
	if error.is_empty():
		for route in simulation.get_routes():
			if not route["id"] in known:
				created = route["id"]
	_feedback(error, "Route erstellt.", null, created)


func _assign() -> void:
	var route := _route()
	if not route.is_empty():
		var plane := _selected_id(assignment)
		if plane in route.get("aircraft_ids", []):
			_feedback(simulation.remove_route_aircraft(route["id"], plane), "Flugzeug freigegeben bzw. Freigabe nach Bodenzeit vorgemerkt.")
		else:
			_feedback(simulation.add_route_aircraft(route["id"], plane), "Flugzeug zur Rotation hinzugefügt.")


func _delete() -> void:
	var route := _route()
	if not route.is_empty():
		_feedback(simulation.delete_route(route["id"]), "Route gelöscht; Flugzeug freigegeben.")


func focus_route(id: int) -> void:
	var row := route_tree.get_root().get_first_child()
	while row != null:
		if row.get_metadata(0)["id"] == id:
			row.select(0)
			_selection_changed()
			route_tree.ensure_cursor_is_visible()
			route_tree.grab_focus()
			return
		row = row.get_next()


func _update_fare(route: Dictionary) -> void:
	fare_box.visible = not route.get("freighter_only", false)
	if not fare_box.visible: return
	var key := [route.get("id", 0), route.get("ticket_eur", -1), route.get("business_ticket_eur", -1), route.get("has_plan", false), route.get("aircraft_ids", []), route.get("service_level", ""), route.get("cargo_rate_cents", 0)]
	if key != fare_key:
		if not route.is_empty():
			if not ticket_input.get_line_edit().has_focus(): ticket_input.value = route["ticket_eur"]
			if not business_input.get_line_edit().has_focus(): business_input.value = route["business_ticket_eur"]
		fare_key = key.duplicate(true)
		fare_cached = simulation.get_fare_suggestion(route.get("id", 0))
	fare_value.text = "—" if fare_cached.is_empty() else UI.money(fare_cached["fare"])
	fare_support.text = tr("FARE_NONE") if fare_cached.is_empty() else tr("FARE_PROFIT") % [UI.signed_money(fare_cached["current_profit"]), UI.signed_money(fare_cached["profit"])]
	fare_business.text = "" if fare_cached.get("business", -1) < 0 else tr("FARE_BUSINESS") % UI.money(fare_cached["business"])
	var explanation := tr("FARE_TOOLTIP")
	if not fare_cached.is_empty(): explanation += "\n" + tr("FARE_ASOF") % _fare_date(fare_cached["at"])
	fare_value.get_parent().get_parent().tooltip_text = explanation
	fare_apply.tooltip_text = explanation
	fare_apply.disabled = fare_cached.is_empty()
	var enabled: bool = route.get("pricing_autopilot", false)
	fare_switch.set_pressed_no_signal(enabled)
	fare_switch.disabled = route.is_empty()
	fare_switch.text = tr("FARE_AUTOPILOT") + " · " + tr("FARE_ON" if enabled else "FARE_OFF")
	fare_switch.tooltip_text = tr("FARE_AUTO_TOOLTIP")
	fare_last.text = "" if route.get("pricing_last", "").is_empty() else tr("FARE_LAST") % (route["pricing_last"] + " · " + _fare_date(route["pricing_changed_at"]))


func _fare_date(seconds: int) -> String:
	return tr("FARE_DATE") % [seconds / 86400 + 1, "%02d:%02d" % [seconds / 3600 % 24, seconds / 60 % 60]]
