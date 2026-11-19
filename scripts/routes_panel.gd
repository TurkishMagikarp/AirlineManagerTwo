extends VBoxContainer
## Presentation only: Rust owns routes, availability, distances and validation.

signal changed
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
const UI = preload("res://scripts/ui/components.gd")

var simulation: RefCounted
var origin: OptionButton
var destination: OptionButton
var aircraft: OptionButton
var preview: Label
var create_button: Button
var status: Label
var season_label: Label
var route_tree: Tree
var empty_label: Control
var assignment: OptionButton
var assign_button: Button
var delete_button: Button
var delay_input: SpinBox
var interval_input: SpinBox
var schedule_button: Button
var stop_button: Button
var maintenance_label: Label
var operations_label: Label
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
		for select in [origin, destination]:
			select.add_item("%s · Gebühren %d %% · %s (%s)" % [airport["iata"], airport["charge_percent"], String(airport["name"]).left(32) + ("…" if String(airport["name"]).length() > 32 else ""), airport["icao"]])
			select.set_item_metadata(select.item_count - 1, airport["icao"])
			select.set_item_tooltip(select.item_count - 1, "%s · Gebührenstufe %d %% des Modell-/Passagiertarifs an diesem Endpunkt; Start und Ziel werden addiert." % [airport["name"], airport["charge_percent"]])
	for i in range(origin.item_count):
		if origin.get_item_metadata(i) == "EDDF": origin.select(i)
		if destination.get_item_metadata(i) == "EDDM": destination.select(i)
	aircraft = _select("Verfügbares Flugzeug")
	preview = _label("")
	add_child(preview)
	create_button = Button.new()
	create_button.text = "Route erstellen"
	create_button.theme_type_variation = "PrimaryButton"
	create_button.pressed.connect(_create)
	add_child(create_button)
	for select in [origin, destination, aircraft]:
		select.item_selected.connect(func(_index: int): _preview())
	status = _label("")
	add_child(status)
	empty_label = UI.empty_state("Noch keine Routen erstellt.", hint.text, create_button.text, func():
		get_parent().get_parent().ensure_control_visible(origin)
		origin.grab_focus()
	)
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
	delete_button = Button.new()
	delete_button.text = "Route löschen"
	delete_button.theme_type_variation = "DangerButton"
	delete_button.pressed.connect(_delete)
	actions.add_child(delete_button)
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
	schedule_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.schedule_route(route["id"], int(delay_input.value), int(interval_input.value)), "Flugplan aktiviert.", interval_input)
	)
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
	operations_label = _label("")
	add_child(operations_label)
	_arrange_workspace()
	for control in [origin, destination, aircraft, assignment, service_select]:
		UI.field_from_label(control)
	for pair in [[ticket_input, "EUR"], [business_input, "EUR"], [cargo_input, "EUR/kg"], [delay_input, "min"], [interval_input, "min"]]:
		UI.field_from_label(pair[0], pair[1], ticket_button.tooltip_text if pair[0] == ticket_input else "")
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


func _arrange_workspace() -> void:
	var original := get_children()
	var grid := VBoxContainer.new()
	grid.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	add_child(grid)
	var network := UI.panel(grid, "01  /  ROUTENREGISTER & LIVE-BETRIEB")
	var planner := UI.panel(grid, "02  /  VERBINDUNG PLANEN")
	var editor := UI.panel(self, "03  /  AUSGEWÄHLTE ROUTE · ZUWEISUNG & FLUGPLAN")
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
			target = network
		node.reparent(target)
	results_tree = UI.table(["Landung / Route", "Strecke", "Pax / Auslastung", "Tickets", "Kosten", "Ergebnis", "Fracht kg / EUR"], [170, 145, 135, 110, 110, 110, 260], 240)
	result_panel.add_child(results_tree)
	results_empty = UI.empty_state("Noch keine Landungen", "Flugplan: Pendelverkehr mit abwechselndem Hin- und Rückflug. Keine automatische Überführung.", schedule_button.text, func():
		var next_control: Control = origin if simulation.get_routes().is_empty() else route_tree if _route().is_empty() else schedule_button
		get_parent().get_parent().ensure_control_visible(next_control)
		next_control.grab_focus()
	)
	result_panel.add_child(results_empty)


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
	_update_fare(metrics.get(_route().get("id", 0), {}))
	_update_connections(metrics.get(_route().get("id", 0), {}))
	_update_schedule_controls(metrics.get(_route().get("id", 0), {}))
	_update_service_cost(metrics.get(_route().get("id", 0), {}))
	var lines: PackedStringArray = ["Flugbetrieb · nächste Abflüge (sortiert)"]
	for route in metrics.values():
		lines.append("Route #%d · Pünktlichkeit: %s · %d erfasste Flüge" % [route["id"], "—" if route["punctuality_percent"] < 0 else "%.1f %%" % route["punctuality_percent"], route["punctuality_flights"]])
	for operation in simulation.get_operations():
		lines.append("%s · Route #%d · Flug %d min · Boden %d min\n%s\nNächster Abflug: %s" % [operation["identifier"], operation["route_id"], ceili(operation["flight_seconds"] / 60.0), operation["ground_seconds"] / 60, operation["status_text"], operation["next_departure"]])
	operations_label.text = "\n\n".join(lines)
	_refresh_competition()
	results_tree.clear()
	var root_item := results_tree.create_item()
	var results: Array = simulation.get_flight_results()
	results_tree.visible = not results.is_empty()
	results_empty.visible = results.is_empty()
	results_empty.get_child(0).get_child(2).text = create_button.text if metrics.is_empty() else "Flugplan aktivieren"
	results_tree.set_meta("breakdowns", {})
	for result in results:
		var row := results_tree.create_item(root_item)
		var values := ["%s / #%d" % [result["landing"], result["route_id"]], "%s–%s" % [result["origin"], result["destination"]], "%d/%d · %.1f %%" % [result["passengers"], result["seats"], result["load_percent"]], UI.money(result["ticket_revenue_eur"]), UI.money(result["costs_eur"]), UI.money(result["profit_eur"])]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
			row.set_tooltip_text(i, "Ticket: %s\nTreibstoff: %s\nFlughäfen: %s\nPersonal: %s\nWartung / Verwaltung: %s\nBordservice (± Standard): %s" % [UI.money(result["ticket_eur"]), UI.money(result["fuel_eur"]), UI.money(result["airport_eur"]), UI.money(result["crew_eur"]), UI.money(result["overhead_eur"]), UI.money(result["service_eur"])])
			row.set_tooltip_text(i, row.get_tooltip_text(i) + "\nBetreuung / Entschädigung: %s" % UI.money(result["care_eur"]))
		row.set_tooltip_text(4, row.get_tooltip_text(4) + "\n" + tr("ANCILLARY_ROUTE_REVENUE") % UI.money(result["ancillary_revenue_eur"]))
		var tooltip := row.get_tooltip_text(4)
		var parts: Array = [[tr("ANCILLARY_REVENUE"), UI.money(result["ancillary_revenue_eur"])]]
		for pair in [["Treibstoff", "fuel_eur"], ["Flughäfen", "airport_eur"], ["Personal", "crew_eur"], ["Wartung / Verwaltung", "overhead_eur"], ["Bordservice (± Standard)", "service_eur"], ["Betreuung / Entschädigung", "care_eur"], ["Kosten", "costs_eur"]]:
			parts.append([pair[0], UI.money(result[pair[1]])])
		var breakdowns: Dictionary = results_tree.get_meta("breakdowns")
		breakdowns[tooltip] = parts
		UI.cell_text(row, 2, "Y %d/%d · J %d/%d" % [result["economy_passengers"], result["economy_seats"], result["business_passengers"], result["business_seats"]])
		row.set_tooltip_text(2, tr("TRANSFER_RESULT") % [result["local_passengers"], result["connecting_passengers"]])
		row.set_tooltip_text(3, tr("TRANSFER_REVENUE") % UI.money(result["connecting_revenue_eur"]) + "\n" + "Economy: %s · Business: %s" % [UI.money(result["economy_revenue_eur"]), UI.money(result["business_revenue_eur"])])
		UI.cell_text(row, 6, "%d kg / %s" % [result["cargo_kg"], UI.money(result["cargo_revenue_eur"])])
		UI.cell_result(row, 5, result["profit_eur"])


func _preview() -> void:
	var result: Dictionary = simulation.preview_route(origin.get_selected_metadata(), destination.get_selected_metadata(), _selected_id(aircraft), 0)
	preview.text = "Entfernung: %.1f km · %s" % [result["distance_km"], "Reichweite ausreichend" if result["error"].is_empty() else result["error"]]
	preview.text += "\n%s · %s" % [result["origin_slots"], result["destination_slots"]]
	if result["has_cost_estimate"]:
		preview.text += "\nPlanung: %d Pax · Kosten %s, davon Flughäfen Start + Ziel %s je Abflug.\nJahresmittel, Standardpreise, 1 Abflug/Tag/Richtung; Personal und Wartungschecks separat." % [result["estimated_passengers"], UI.money(result["estimated_costs_eur"]), UI.money(result["estimated_airport_eur"])]
	UI.field_error(aircraft, result["error"])
	create_button.disabled = not result["error"].is_empty()
	if aircraft.item_count == 0:
		preview.text += "\nKein freies Flugzeug. Im Flotte-Tab kaufen oder eine Zuweisung lösen."


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
		_update_schedule_controls(route)
	assignment.clear()
	for plane in simulation.get_fleet():
		if plane["route_id"] == 0 or plane["route_id"] == route.get("id", -1):
			var assigned: bool = plane["id"] in route.get("aircraft_ids", [])
			assignment.add_item("%s · %s · %s" % [plane["identifier"], plane["model_name"], "Entfernen" if assigned else "Hinzufügen"])
			assignment.set_item_metadata(assignment.item_count - 1, plane["id"])
	assignment.disabled = route.is_empty()
	delete_button.disabled = route.is_empty()
	_update_fare(route)
	_update_connections(route)
	_assignment_preview()
	_refresh_competition()


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
	var notice := hangar
	if not crew.is_empty():
		notice = crew if notice.is_empty() else notice + "\n" + crew
	maintenance_label.text = notice
	maintenance_label.visible = not notice.is_empty()


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


func _assignment_preview() -> void:
	var route := _route()
	assign_button.disabled = route.is_empty() or assignment.item_count == 0
	assign_button.tooltip_text = ""
	var removing: bool = _selected_id(assignment) in route.get("aircraft_ids", [])
	assign_button.text = "Flugzeug entfernen" if removing else "Flugzeug hinzufügen"
	if removing:
		assign_button.tooltip_text = "Laufender Flug und Bodenzeit werden beendet; andere Flugzeuge fliegen weiter."
		return
	if not route.is_empty() and _selected_id(assignment) != 0:
		var result: Dictionary = simulation.preview_route(route["origin_icao"], route["destination_icao"], _selected_id(assignment), route["id"])
		assign_button.disabled = not result["error"].is_empty()
		assign_button.tooltip_text = result["error"]
		status.text = result["error"]


func _feedback(error: String, success: String, field: Control = null) -> void:
	refresh()
	if field != null: UI.field_error(field, error)
	status.text = success + " Bitte speichern." if error.is_empty() else error
	if error.is_empty():
		changed.emit()
	await get_tree().process_frame
	var scroll := get_parent().get_parent() as ScrollContainer
	scroll.ensure_control_visible(status)


func _create() -> void:
	_feedback(simulation.create_route(origin.get_selected_metadata(), destination.get_selected_metadata(), _selected_id(aircraft)), "Route erstellt.")


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
