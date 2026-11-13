extends VBoxContainer
## Presentation only: Rust owns routes, availability, distances and validation.

signal changed
const UI = preload("res://scripts/ui/components.gd")

var simulation: RefCounted
var origin: OptionButton
var destination: OptionButton
var aircraft: OptionButton
var preview: Label
var create_button: Button
var status: Label
var route_tree: Tree
var empty_label: Label
var assignment: OptionButton
var assign_button: Button
var delete_button: Button
var delay_input: SpinBox
var interval_input: SpinBox
var schedule_button: Button
var stop_button: Button
var maintenance_label: Label
var operations_label: Label
var ticket_input: SpinBox
var ticket_button: Button
var results_tree: Tree
var competition_label: Label


func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var hint := _label("Luftlinie · vereinfachte Referenzreichweite, ohne Wetter oder Reserven.\nFlugplan: Pendelverkehr mit abwechselndem Hin- und Rückflug. Keine automatische Überführung.")
	add_child(hint)
	origin = _select("Startflughafen")
	destination = _select("Zielflughafen")
	for country in simulation.get_countries():
		for airport in simulation.get_airports(country["code"]):
			for select in [origin, destination]:
				select.add_item("%s · %s (%s)" % [airport["iata"], airport["name"], airport["icao"]])
				select.set_item_metadata(select.item_count - 1, airport["icao"])
	destination.select(1)
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
	empty_label = _label("Noch keine Routen erstellt.")
	add_child(empty_label)
	route_tree = Tree.new()
	route_tree.columns = 6
	route_tree.hide_root = true
	route_tree.select_mode = Tree.SELECT_ROW
	route_tree.column_titles_visible = true
	route_tree.custom_minimum_size.y = 180
	for i in range(6):
		route_tree.set_column_title(i, ["Route", "Strecke", "Entfernung", "Flugzeug", "Nachfrage/Tag →", "Auslastung"][i])
		route_tree.set_column_expand(i, i == 1 or i == 3)
		route_tree.set_column_custom_minimum_width(i, [65, 140, 110, 170, 125, 100][i])
	route_tree.item_selected.connect(_selection_changed)
	add_child(route_tree)
	assignment = _select("Ausgewählte Route: Flugzeugzuweisung")
	var actions := HBoxContainer.new()
	add_child(actions)
	assign_button = Button.new()
	assign_button.text = "Zuweisung übernehmen"
	assign_button.pressed.connect(_assign)
	actions.add_child(assign_button)
	delete_button = Button.new()
	delete_button.text = "Route löschen"
	delete_button.theme_type_variation = "DangerButton"
	delete_button.pressed.connect(_delete)
	actions.add_child(delete_button)
	assignment.item_selected.connect(func(_index: int): _assignment_preview())
	add_child(_label("Ticketpreis der ausgewählten Route · EUR pro Passagier und Einzelflug (vorläufig)"))
	ticket_input = SpinBox.new()
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
			_feedback(simulation.set_ticket_price(route["id"], int(ticket_input.value)), "Ticketpreis geändert.")
	)
	add_child(ticket_button)
	var rules := _label("Wirtschaftsregeln & Berechnung: Details im Tooltip")
	rules.tooltip_text = simulation.get_economy_rules()
	add_child(rules)
	add_child(_label("Flugplan für ausgewählte Route · erster Abflug in Spielminuten"))
	delay_input = SpinBox.new()
	delay_input.min_value = 1
	delay_input.max_value = 10080
	delay_input.value = 5
	add_child(delay_input)
	add_child(_label("Abstand zwischen Abflügen in Spielminuten (mindestens Flug + Bodenzeit)"))
	interval_input = SpinBox.new()
	interval_input.min_value = 1
	interval_input.max_value = 10080
	interval_input.value = 120
	add_child(interval_input)
	schedule_button = Button.new()
	schedule_button.text = "Flugplan aktivieren"
	schedule_button.pressed.connect(func():
		var route := _route()
		if not route.is_empty():
			_feedback(simulation.schedule_route(route["id"], int(delay_input.value), int(interval_input.value)), "Flugplan aktiviert.")
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
	competition_label = _label("Route auswählen, um den gemeinsamen Passagiermarkt zu sehen.")
	var competition_panel := UI.panel(self, "05  /  WETTBEWERB · GEMEINSAMER STRECKENMARKT")
	competition_panel.add_child(competition_label)
	refresh()


func _arrange_workspace() -> void:
	var original := get_children()
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 2 if grid.size.x >= 1050 else 1)
	var network := UI.panel(grid, "01  /  ROUTENREGISTER & LIVE-BETRIEB")
	var planner := UI.panel(grid, "02  /  VERBINDUNG PLANEN")
	var editor := UI.panel(self, "03  /  AUSGEWÄHLTE ROUTE · ZUWEISUNG & FLUGPLAN")
	var editor_grid := GridContainer.new()
	editor_grid.add_theme_constant_override("h_separation", 18)
	editor_grid.add_theme_constant_override("v_separation", 18)
	editor.add_child(editor_grid)
	editor_grid.resized.connect(func(): editor_grid.columns = 3 if editor_grid.size.x >= 1000 else 1)
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
	results_tree = UI.table(["Landung / Route", "Strecke", "Pax / Auslastung", "Einnahmen", "Kosten", "Ergebnis"], [170, 145, 135, 110, 110, 110], 240)
	result_panel.add_child(results_tree)


func _label(text: String) -> Label:
	var label := Label.new()
	label.text = text
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return label


func _select(title: String) -> OptionButton:
	add_child(_label(title))
	var select := OptionButton.new()
	select.fit_to_longest_item = false
	select.clip_text = true
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
	route_tree.visible = not routes.is_empty()
	route_tree.clear()
	var root_item := route_tree.create_item()
	for route in routes:
		var row := route_tree.create_item(root_item)
		row.set_metadata(0, route)
		row.set_text(0, "#%d" % route["id"])
		row.set_text(1, "%s → %s" % [route["origin_icao"], route["destination_icao"]])
		row.set_text(2, "%.1f km" % route["distance_km"])
		row.set_text(3, "Nicht zugewiesen")
		for plane in fleet:
			if plane["id"] == route["aircraft_id"]:
				row.set_text(3, "%s · %s" % [plane["identifier"], plane["model_name"]])
		if not String(route["maintenance_status"]).is_empty():
			row.set_text(3, row.get_text(3) + " · Wartung")
		for i in range(6):
			row.set_tooltip_text(i, row.get_text(i))
		if route["id"] == selected_route:
			row.select(0)
	_selection_changed()
	_preview()
	refresh_operations()


func refresh_operations() -> void:
	# Refresh live metrics without rebuilding selection or resetting ticket inputs.
	var metrics: Dictionary = {}
	for route in simulation.get_routes():
		metrics[route["id"]] = route
	var tree_root := route_tree.get_root()
	if tree_root != null:
		for row in tree_root.get_children():
			var route: Dictionary = metrics.get(row.get_metadata(0)["id"], {})
			if route.is_empty():
				continue
			row.set_text(4, "%d Pax" % route["daily_demand"])
			row.set_tooltip_text(4, "Nachfrage bei eigenem Preis ohne Preiswettbewerb, NICHT pro Flug oder Route.\nGeschäft: ca. %d · Urlaub/Privat: ca. %d\nGeteilter Anteil pro Abflug vor Sitzlimit: %d Pax%s\nDie gemeinsame Marktaufteilung steht im Wettbewerbsbereich." % [route["daily_business"], route["daily_leisure"], route["estimated_passengers"], "" if route["has_plan"] else " (ohne eigenen Flugplan)"])
			row.set_text(5, "%.1f %%" % route["actual_load_percent"] if route["actual_flights"] > 0 else "Keine Flüge")
			row.set_tooltip_text(5, "Tatsächlich beförderte Passagiere / angebotene Sitze aller archivierten Landungen dieser Route. Bei alten Spielständen ab Archivbeginn.")
			if not String(route["maintenance_status"]).is_empty():
				row.set_tooltip_text(3, row.get_text(3) + "\n" + route["maintenance_status"])
			elif not String(route["staff_status"]).is_empty():
				row.set_tooltip_text(3, row.get_text(3) + "\n" + route["staff_status"])
	_update_schedule_controls(metrics.get(_route().get("id", 0), {}))
	var lines: PackedStringArray = ["Flugbetrieb · nächste Abflüge (sortiert)"]
	for operation in simulation.get_operations():
		lines.append("%s · Route #%d · Flug %d min · Boden %d min\n%s\nNächster Abflug: %s" % [operation["identifier"], operation["route_id"], ceili(operation["flight_seconds"] / 60.0), operation["ground_seconds"] / 60, operation["status_text"], operation["next_departure"]])
	operations_label.text = "\n\n".join(lines)
	_refresh_competition()
	results_tree.clear()
	var root_item := results_tree.create_item()
	var results: Array = simulation.get_flight_results()
	if results.is_empty():
		var empty := results_tree.create_item(root_item)
		empty.set_text(0, "Noch keine Landungen")
		empty.set_selectable(0, false)
	for result in results:
		var row := results_tree.create_item(root_item)
		var values := ["%s / #%d" % [result["landing"], result["route_id"]], "%s → %s" % [result["origin"], result["destination"]], "%d/%d · %.1f %%" % [result["passengers"], result["seats"], result["load_percent"]], UI.money(result["revenue_eur"]), UI.money(result["costs_eur"]), UI.money(result["profit_eur"])]
		for i in range(values.size()):
			row.set_text(i, values[i])
			row.set_tooltip_text(i, "Ticket: %s\nTreibstoff: %s\nFlughäfen: %s\nPersonal: %s\nWartung / Verwaltung: %s" % [UI.money(result["ticket_eur"]), UI.money(result["fuel_eur"]), UI.money(result["airport_eur"]), UI.money(result["crew_eur"]), UI.money(result["overhead_eur"])])
		row.set_custom_color(5, UI.POSITIVE if result["profit_eur"] >= 0 else UI.NEGATIVE)


func _preview() -> void:
	var result: Dictionary = simulation.preview_route(origin.get_selected_metadata(), destination.get_selected_metadata(), _selected_id(aircraft), 0)
	preview.text = "Entfernung: %.1f km · %s" % [result["distance_km"], "Reichweite ausreichend" if result["error"].is_empty() else result["error"]]
	create_button.disabled = not result["error"].is_empty()
	if aircraft.item_count == 0:
		preview.text += "\nKein freies Flugzeug. Im Flotte-Tab kaufen oder eine Zuweisung lösen."


func _selection_changed() -> void:
	var route := _route()
	status.text = ""
	if ticket_button != null:
		ticket_button.disabled = route.is_empty()
		ticket_input.editable = not ticket_button.disabled
		ticket_input.value = route.get("ticket_eur", 100)
	if schedule_button != null:
		_update_schedule_controls(route)
	assignment.clear()
	assignment.add_item("Keine Zuweisung (Flugzeug freigeben)")
	assignment.set_item_metadata(0, 0)
	for plane in simulation.get_fleet():
		if plane["route_id"] == 0 or plane["route_id"] == route.get("id", -1):
			assignment.add_item("%s · %s · %d km Reichweite" % [plane["identifier"], plane["model_name"], plane["range_km"]])
			assignment.set_item_metadata(assignment.item_count - 1, plane["id"])
			if plane["id"] == route.get("aircraft_id", 0):
				assignment.select(assignment.item_count - 1)
	assignment.disabled = route.is_empty()
	delete_button.disabled = route.is_empty()
	_assignment_preview()
	_refresh_competition()


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


func _refresh_competition() -> void:
	if competition_label == null:
		return
	var route := _route()
	var companies: PackedStringArray = ["KONKURRENZÜBERSICHT · alle Strecken und Flotten"]
	for company in simulation.get_competitors():
		companies.append("%s · Guthaben %d EUR · %d Flugzeuge\n%s\nFlotte:\n%s\nKäufe: %s\nEntscheidung: %s" % [company["airline"], company["balance"], company["fleet_size"], company["routes"], company["fleet"], company["purchases"], company["last_action"]])
	if route.is_empty():
		competition_label.text = "\n\n".join(companies)
		return
	var live_route: Dictionary = route
	for current in simulation.get_routes():
		if current["id"] == route["id"]:
			live_route = current
	var lines: PackedStringArray = ["%s ↔ %s · Schätzung pro Tag und Richtung\nGemeinsamer Basismarkt: ca. %d Pax/Tag vor Preisreaktion" % [route["origin_icao"], route["destination_icao"], live_route["base_daily_market"]]]
	for offer in simulation.get_route_competition(route["id"]):
		lines.append("%s · %s · %d Sitze · %d EUR\n%.1f Abflüge/Tag je Richtung → ca. %.0f Pax/Tag · %.1f %% der beförderten Pax%s\n%s" % [offer["airline"], offer["model"], offer["seats"], offer["ticket_eur"], offer["departures_per_day_per_direction"], offer["passengers_per_day"], offer["share_percent"], "" if offer["active"] else " · inaktiv", offer["last_action"]])
	lines.append("Ein Markt für alle Airlines. Preis und Frequenz verteilen Nachfrage; Sitze begrenzen Buchungen. Nicht bediente Nachfrage verfällt. Schätzung, keine garantierten Buchungen. KI prüft wöchentlich Preise und Verluste.")
	competition_label.text = "\n\n".join(companies) + "\n\n" + "\n\n".join(lines)


func _assignment_preview() -> void:
	var route := _route()
	assign_button.disabled = route.is_empty()
	assign_button.tooltip_text = ""
	if not route.is_empty() and _selected_id(assignment) != 0:
		var result: Dictionary = simulation.preview_route(route["origin_icao"], route["destination_icao"], _selected_id(assignment), route["id"])
		assign_button.disabled = not result["error"].is_empty()
		assign_button.tooltip_text = result["error"]
		status.text = result["error"]


func _feedback(error: String, success: String) -> void:
	refresh()
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
		_feedback(simulation.assign_route(route["id"], _selected_id(assignment)), "Zuweisung geändert.")


func _delete() -> void:
	var route := _route()
	if not route.is_empty():
		_feedback(simulation.delete_route(route["id"]), "Route gelöscht; Flugzeug freigegeben.")
