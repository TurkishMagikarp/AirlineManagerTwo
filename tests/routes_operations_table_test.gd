extends SceneTree
## Routes operations table: one row per aircraft, timed rows first, dash without next departure, reload-stable.

var main: Control

func _initialize() -> void:
	call_deferred("_run")

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition

func _rows() -> Array:
	var result: Array = []
	var row: TreeItem = main.routes_panel.operations_tree.get_root().get_first_child()
	while row != null:
		result.append([row.get_text(0), row.get_text(1), row.get_text(2), row.get_text(3), row.get_text(4), row.get_text(5)])
		row = row.get_next()
	return result

func _refresh() -> void:
	main.routes_panel._operations_signature = ""
	main.routes_panel.refresh_operations()

func _run() -> void:
	TranslationServer.set_locale("de")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	for error in [sim.found_airline("Ops Table Air", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.hire_missing_staff(), sim.schedule_route(1, 1, 240), sim.add_route_aircraft(1, 2)]:
		if not _check(error.is_empty(), "Setup: " + error):
			return
	_refresh()
	if not _check(main.routes_panel.operations_empty.visible == false and _rows().size() == 2, "One row per assigned aircraft"):
		return
	var operations: Array = sim.get_operations()
	var rows := _rows()
	if not _check(rows[0][0] == operations[0]["identifier"] and rows[0][1] == "EDDF → EDDM" and rows[0][3] == operations[0]["next_departure"] and rows[0][4] == "—", "Scheduled row shows direction, next departure and no landing"):
		return
	sim.start()
	sim.advance(100.0)
	_refresh()
	rows = _rows()
	operations = sim.get_operations()
	if not _check(rows[1][2] == main.tr("OPS_STATE_AIRBORNE") and rows[1][4] == operations[1]["expected_landing"] and rows[1][4] != "—" and rows[1][3] == operations[1]["next_departure"] and rows[0][2] == main.tr("OPS_STATE_SCHEDULED"), "Airborne row shows expected landing separately from next departure"):
		return
	if not _check(operations[0]["next_departure_seconds"] <= operations[1]["next_departure_seconds"], "Timed rows sorted by next departure"):
		return
	var released: int = operations[1]["aircraft_id"]
	if not _check(sim.remove_route_aircraft(1, released).is_empty(), "Release request accepted"):
		return
	_refresh()
	rows = _rows()
	if not _check(rows.size() == 2 and rows[1][2] == main.tr("OPS_STATE_RELEASE") and rows[1][3] == "—" and rows[1][4] != "—" and rows[0][3] != "—", "Pending release shows a dash and sorts last"):
		return
	var path := ProjectSettings.globalize_path("user://ops_table_test.json")
	if not _check(sim.save_game(path).is_empty(), "Save"):
		return
	var before := _rows()
	if not _check(sim.load_game(path).is_empty(), "Reload"):
		return
	_refresh()
	if not _check(_rows() == before, "Table identical after reload: %s vs %s" % [_rows(), before]):
		return
	DirAccess.remove_absolute(path)
	if not await _crew_blocked_scenario():
		return
	print("PASS: routes operations table")
	quit(0)

func _crew_blocked_scenario() -> bool:
	main.queue_free()
	await process_frame
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	for error in [sim.found_airline("Crew Board Air", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.create_route("EDDF", "EDDM", 2), sim.hire_missing_staff(), sim.set_ticket_price(1, 100), sim.set_ticket_price(2, 100), sim.schedule_route(1, 1, 120), sim.schedule_route(2, 1, 120), sim.set_staff_pay("pilot", 85)]:
		if not _check(error.is_empty(), "Crew setup: " + error):
			return false
	sim.start()
	for day in range(300):
		sim.advance(86400.0)
		if sim.get_staff_overview()["roles"][0]["resigned"] > 0:
			break
	sim.pause()
	var path := ProjectSettings.globalize_path("user://ops_table_crew_test.json")
	if not _check(sim.save_game(path).is_empty() and sim.load_game(path).is_empty(), "Save and reload during shortage"):
		return false
	root.size = Vector2i(680, 520)
	_refresh()
	var operations: Array = sim.get_operations()
	var blocked: Array = operations.filter(func(o): return o["crew_blocked"])
	var staffed: Array = operations.filter(func(o): return not o["crew_blocked"])
	if not _check(blocked.size() == 1 and staffed.size() == 1, "One blocked and one staffed aircraft"):
		return false
	if not _check("Piloten (ATR 72)" in blocked[0]["status_text"] and blocked[0]["crew_missing"] != "" and blocked[0]["next_departure"] == "—" and blocked[0]["delay_seconds"] == 0, "Blocked detail names role and rating, no time or delay"):
		return false
	var rows := _rows()
	var blocked_row := -1
	for i in range(rows.size()):
		if rows[i][0] == blocked[0]["identifier"]:
			blocked_row = i
	if not _check(blocked_row >= 0 and rows[blocked_row][2] == main.tr("OPS_STATE_CREW_BLOCKED") and rows[blocked_row][3] == main.tr("OPS_NEXT_CREW_BLOCKED") and rows[blocked_row][5] == "—", "Blocked row shows state, no time, no delay"):
		return false
	if not _check(rows[1 - blocked_row][3] == staffed[0]["next_departure"] and rows[1 - blocked_row][3] != "—", "Staffed row keeps its real time"):
		return false
	for entry: Dictionary in sim.get_attention_snapshot():
		if not _check(entry["kind"] != "delay", "No generic delay warning from the hold"):
			return false
	if not _check(sim.hire_missing_staff().is_empty(), "Hire missing crew"):
		return false
	_refresh()
	for operation: Dictionary in sim.get_operations():
		if not _check(not operation["crew_blocked"] and operation["next_departure_seconds"] >= 0, "Real departure after hiring"):
			return false
	var flights: int = sim.get_financial_summary()["flights"]
	sim.start()
	sim.advance(3.0 * 86400.0)
	sim.pause()
	if not _check(sim.get_financial_summary()["flights"] > flights, "Plan completes another leg"):
		return false
	DirAccess.remove_absolute(path)
	return true
