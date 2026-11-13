extends SceneTree
## Maintenance through the real Fleet tab: request in flight, finish the flight,
## hangar blocks departures, save/load mid-check, 16x resume. Optional
## separate-process restore (see run_maintenance_restart_test.ps1).

var main


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var path := ""
	var restore := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--maintenance-save="):
			path = arg.trim_prefix("--maintenance-save=")
		if arg == "--restore-maintenance":
			restore = true
	var generated := path.is_empty()
	if generated:
		path = OS.get_temp_dir().path_join("airline-maintenance-%d.json" % Time.get_ticks_usec())
	var sim = main.simulation
	if restore:
		if not _check(sim.load_game(path).is_empty(), "Load mid-maintenance game in a fresh process"):
			return
		main._sync_view()
		if not await _verify_hangar_and_resume(sim):
			return
		print("PASS: maintenance state restored in a separate process and resumed")
		quit(0)
		return
	main.name_input.text = "Hangar UI Air"
	await _click(main.found_button)
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	await _click(main.buy_button)
	var fresh: Dictionary = sim.get_fleet()[0]
	if not _check(fresh["condition_percent"] == 100.0 and fresh["cycles"] == 0 and not fresh["can_request_maintenance"] and "Frisch" in fresh["maintenance_hint"], "New aircraft is in perfect condition; nothing to maintain"):
		return
	var row: TreeItem = main.fleet_tree.get_root().get_child(0)
	if not _check(row.get_text(4) == "100 %" and "FH" in row.get_text(5) and main.maintenance_button.disabled, "Fleet table shows condition and upcoming maintenance"):
		return
	if not _check(sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.schedule_route(1, 1, 120).is_empty(), "Shuttle scheduled"):
		return
	await _click(main.start_button)
	await _click(main.speed_buttons[2])
	# Two complete round legs, then the third flight is airborne.
	sim.advance((60.0 + 7200.0 * 2 + 100.0) / 16.0)
	main._refresh_fleet()
	var plane: Dictionary = sim.get_fleet()[0]
	if not _check(plane["status"] == "in_flight" and plane["cycles"] == 2 and plane["condition_percent"] < 100.0 and plane["can_request_maintenance"], "Flying causes wear; airborne aircraft can be booked for maintenance"):
		return
	if not _check("Spätestens" in main.fleet_tree.get_root().get_child(0).get_text(5) and "ungeplant" in main.maintenance_details.text, "Upcoming mandatory limit and unscheduled cost are displayed"):
		return
	var balance: int = sim.get_airline_info()["capital_eur"]
	var cost: int = plane["maintenance_cost_eur"]
	await _click(main.maintenance_button)
	plane = sim.get_fleet()[0]
	if not _check(main.maintenance_status.text.begins_with("Wartung beauftragt") and plane["maintenance_state"] == "pending" and plane["status"] == "in_flight" and sim.get_airline_info()["capital_eur"] == balance, "Airborne request waits for landing, no debit yet"):
		return
	if not _check("Nach Landung" in main.fleet_tree.get_root().get_child(0).get_text(5) and main.maintenance_button.disabled, "Pending maintenance shown, no double booking"):
		return
	var flights: int = sim.get_financial_summary()["flights"]
	var profit: int = sim.get_financial_summary()["profit_eur"]
	# Finish the journey (one large 16x frame), then hangar.
	sim.advance(3600.0 / 16.0)
	main._refresh_fleet()
	plane = sim.get_fleet()[0]
	var summary: Dictionary = sim.get_financial_summary()
	if not _check(plane["status"] == "maintenance" and plane["location_icao"] == "EDDM" and summary["flights"] == flights + 1, "Flight in progress lands normally before maintenance"):
		return
	if not _check(sim.get_airline_info()["capital_eur"] == balance + (summary["profit_eur"] - profit) - cost and summary["maintenance_eur"] == cost, "Maintenance cost debited exactly once at start"):
		return
	if not _check(main.fleet_tree.get_root().get_child(0).get_text(3).begins_with("In Wartung") and "In Wartung bis" in sim.get_operations()[0]["status_text"], "Fleet and routes show the hangar state"):
		return
	if not _check(not sim.schedule_route(1, 1, 120).is_empty(), "Aircraft in maintenance cannot be scheduled"):
		return
	# Many frames inside the check: no departures at any frame size.
	for i in range(20):
		sim.advance(60.0 / 16.0)
	if not _check(sim.get_financial_summary()["flights"] == flights + 1 and sim.get_fleet()[0]["status"] == "maintenance", "No departures during maintenance"):
		return
	main.save_path = path
	await _click(main.save_button)
	if not _check(main.save_status.text == "Spielstand gespeichert.", "Save during maintenance"):
		return
	await _click(main.load_button)
	if not _check(main.save_status.text == "Spielstand geladen." and sim.get_fleet()[0]["maintenance_state"] == "active", "Load during maintenance"):
		return
	await _click(main.navigation[1])
	if not await _verify_hangar_and_resume(sim):
		return
	# An explicit path is kept for the separate-process restore run.
	if generated:
		DirAccess.remove_absolute(path)
	print("PASS: maintenance request in flight, landing first, hangar blocks departures, single debit, save/load, 16x resume")
	quit(0)


func _verify_hangar_and_resume(sim) -> bool:
	var plane: Dictionary = sim.get_fleet()[0]
	if not _check(plane["maintenance_state"] == "active" and plane["status"] == "maintenance" and plane["checks"] == 0, "Hangar state intact after load"):
		return false
	var flights: int = sim.get_financial_summary()["flights"]
	sim.start()
	sim.set_speed(16)
	# The check ends; the plan resumes one minute later from EDDM.
	sim.advance((plane["maintenance_end_seconds"] - sim.get_elapsed_seconds() + 30.0) / 16.0)
	main._refresh_fleet()
	plane = sim.get_fleet()[0]
	if not _check(plane["checks"] == 1 and plane["condition_percent"] == 100.0 and plane["maintenance_state"] == "none" and sim.get_routes()[0]["has_plan"], "Check completes, condition restored, plan resumes"):
		return false
	if not _check(sim.get_financial_summary()["flights"] == flights, "Resume starts at the next departure, no retroactive flights"):
		return false
	sim.advance(7200.0 / 16.0)
	if not _check(sim.get_financial_summary()["flights"] == flights + 1 and sim.get_fleet()[0]["cycles"] == 4, "Resumed flights accumulate new wear"):
		return false
	return _check(main.fleet_tree.get_root().get_child(0).get_text(4) != "100 %" or sim.get_fleet()[0]["condition_percent"] < 100.0, "Condition decreases again")


func _reveal(control: Control) -> void:
	await process_frame
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(control)
		parent = parent.get_parent()
	await process_frame


func _click(button: Button) -> void:
	await _reveal(button)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
