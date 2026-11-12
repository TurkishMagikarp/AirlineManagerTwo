extends SceneTree
## UI workflow plus optional separate-process in-flight restore. Never touches user saves.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var path := ""
	var restore := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--flight-save="):
			path = arg.trim_prefix("--flight-save=")
		if arg == "--restore-flight":
			restore = true
	if restore:
		if not _check(main.simulation.load_game(path).is_empty(), "Load in-flight game in fresh process"):
			return
		if not _check(main.simulation.get_fleet()[0]["status"] == "in_flight" and main.simulation.get_elapsed_seconds() == 60.0, "Exact in-flight state restored"):
			return
		main._sync_view()
		var map_flights: Array = main.simulation.get_map_flights()
		if not _check(map_flights.size() == 1 and map_flights[0]["progress"] == 0.0 and map_flights[0]["origin"] == "EDDF", "Map position reconstructed in fresh process"):
			return
	else:
		main.name_input.text = "Flight UI Air"
		await _click(main.found_button)
		await _click(main.navigation[1])
		for i in range(main.model_select.item_count):
			if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
				main.model_select.select(i)
				main.model_select.item_selected.emit(i)
		await _click(main.buy_button)
		await _click(main.navigation[2])
		await _click(main.routes_panel.create_button)
		if not _check(main.simulation.get_routes().size() == 1, "Route created before schedule"):
			return
		var row: TreeItem = main.routes_panel.route_tree.get_root().get_child(0)
		row.select(0)
		main.routes_panel.route_tree.item_selected.emit()
		main.routes_panel.ticket_input.value = 100
		await _click(main.routes_panel.ticket_button)
		if not _check(main.simulation.get_routes()[0]["ticket_eur"] == 100, "Ticket price set through actual UI"):
			return
		main.routes_panel.delay_input.value = 1
		root.size = Vector2i(680, 520)
		main.routes_panel.interval_input.value = 1
		await _click(main.routes_panel.schedule_button)
		if not _check("mindestens" in main.routes_panel.status.text, "Impossible interval rejected via UI"):
			return
		main.routes_panel.interval_input.value = 120
		await _click(main.routes_panel.schedule_button)
		if not _check(main.simulation.get_routes()[0]["has_plan"], "Flight plan created with real UI"):
			return
		main.simulation.advance(9999.0)
		if not _check(main.simulation.get_elapsed_seconds() == 0, "Pause blocks departure"):
			return
		await _click(main.start_button)
		main.simulation.advance(60.0)
		if not path.is_empty():
			if not _check(main.simulation.save_game(path).is_empty(), "Save airborne game"):
				return
			print("PASS: UI flight plan and airborne save for separate-process restore")
			quit(0)
			return
	var operation: Dictionary = main.simulation.get_operations()[0]
	main._show_page(2)
	main._process(0.0)
	if not _check("Unterwegs" in main.routes_panel.operations_label.text and "Nächster Abflug" in main.routes_panel.operations_label.text, "Current flight and next departure displayed"):
		return
	await _click(main.pause_button)
	main.simulation.advance(10000.0)
	if not _check(main.simulation.get_elapsed_seconds() == 60.0, "Airborne pause freezes time"):
		return
	await _click(main.speed_buttons[2])
	await _click(main.start_button)
	main.simulation.advance(operation["flight_seconds"] / 16.0)
	var financial: Dictionary = main.simulation.get_financial_summary()
	var results: Array = main.simulation.get_flight_results()
	if not _check(financial["flights"] == 1 and results.size() == 1 and results[0]["ticket_eur"] == 100, "First landing settles saved ticket price exactly once"):
		return
	if not _check(results[0]["passengers"] > 0 and results[0]["passengers"] < results[0]["seats"] and financial["profit_eur"] > 0, "Partial flight earns profit"):
		return
	main._process(0.0)
	if not _check(main.routes_panel.results_tree.get_root().get_first_child().get_text(5) == main.UI.money(main.simulation.get_flight_results()[0]["profit_eur"]), "Flight financial result displayed"):
		return
	if not _check(main.simulation.get_fleet()[0]["status"] == "ground" and main.simulation.get_fleet()[0]["location_icao"] == "EDDM", "16x lands exactly and starts ground time"):
		return
	main.simulation.advance(operation["ground_seconds"] / 16.0)
	if not _check(main.simulation.get_fleet()[0]["status"] == "parked", "Ground time ends exactly"):
		return
	main.simulation.advance((operation["next_departure_seconds"] - main.simulation.get_elapsed_seconds()) / 16.0)
	if not _check(main.simulation.get_fleet()[0]["status"] == "in_flight", "Return flight departs"):
		return
	main.simulation.advance(7200.0 * 100 / 16.0)
	if not _check(main.simulation.get_operations()[0]["completed_flights"] == 101, "Fast forward counts every landing once"):
		return
	if not _check(main.simulation.get_financial_summary()["flights"] == 101 and main.simulation.get_flight_results().size() == 100, "16x settles all flights and bounds history"):
		return
	await _click(main.navigation[0])
	if not _check(main.metrics["profit"].is_visible_in_tree() and main.metrics["balance"].text == main.UI.money(main.simulation.get_airline_info()["capital_eur"]) and main.metrics["profit"].text == main.UI.money(main.simulation.get_financial_summary()["profit_eur"]), "Company financial overview displayed"):
		return
	await _click(main.navigation[1])
	if not _check("Unterwegs" in main.fleet_tree.get_root().get_child(0).get_text(3), "Fleet displays live flight status"):
		return
	main.queue_free()
	await process_frame
	print("PASS: flight UI, departure, landing, ground, pause, 16x, large jump and return")
	quit(0)


func _click(button: Button) -> void:
	await process_frame
	var parent := button.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(button)
		parent = parent.get_parent()
	await process_frame
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
