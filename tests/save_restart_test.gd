extends SceneTree
## Run write/read modes in separate Godot processes (see run_save_restart_test.ps1).


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var mode := ""
	var path := ""
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--save-test-mode="):
			mode = argument.trim_prefix("--save-test-mode=")
		if argument.begins_with("--save-test-path="):
			path = argument.trim_prefix("--save-test-path=")
	if not _check(mode in ["write-running", "read-running", "write-paused", "read-paused"] and not path.is_empty(), "Explicit test mode and isolated path required"):
		return
	var running := mode.ends_with("-running")
	var speed := 16 if running else 4
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main = scene.instantiate()
	# Freeze presentation ticks so assertions compare the exact saved instant.
	main.save_path = path
	root.add_child(main)
	main.set_process(false)
	await process_frame
	var clock = main.simulation
	if not _check(clock.get_elapsed_seconds() == 0.0 and not clock.is_running() and clock.get_speed() == 1, "Fresh process begins with a fresh clock"):
		return
	if mode.begins_with("write-"):
		main.name_input.text = "Neustart 日本 Air"
		main.country_select.select(1)
		main.country_select.item_selected.emit(1)
		main.airport_select.select(1)
		await _click(main.found_button)
		await _click(main.start_button)
		await _click(main.speed_buttons[2 if running else 1])
		clock.advance(86400.125 / speed)
		if not running:
			await _click(main.pause_button)
		await _click(main.navigation[1])
		_select_atr(main)
		await _click(main.buy_button)
		await _click(main.buy_button)
		await _click(main.navigation[2])
		await _click(main.routes_panel.create_button)
		await _click(main.navigation[0])
		await _click(main.save_button)
		if not _check(main.save_status.text == "Spielstand gespeichert.", "Save button writes via Rust"):
			return
	else:
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load button restores via Rust after restart"):
			return
	if not _check(clock.get_elapsed_seconds() == 86400.125 and clock.is_running() == running and clock.get_speed() == speed, "Exact saved time, running flag and speed"):
		return
	var airline: Dictionary = clock.get_airline_info()
	if not _check(airline.get("name") == "Neustart 日本 Air" and airline.get("country_code") == "TR" and airline.get("airport_icao") == "LTFJ" and airline.get("capital_eur", -1) >= 0, "Airline survives complete process restart"):
		return
	if not _check(main.overview.visible and not main.founding.visible and main.heading.text == airline["name"] and "LTFJ" in main.description.text and main.metrics["balance"].text == main.UI.money(airline["capital_eur"]), "Overview shows founded or loaded airline"):
		return
	var fleet: Array = clock.get_fleet()
	var routes: Array = clock.get_routes()
	if not _check(routes.size() == 1 and routes[0]["origin_icao"] == "EDDF" and routes[0]["destination_icao"] == "EDDM" and routes[0]["aircraft_id"] == 1 and fleet[0]["route_id"] == 1, "Route and exclusive assignment survive complete restart"):
		return
	if mode.begins_with("read-"):
		if not _check(not clock.create_route("LTFM", "LTFJ", 1).is_empty() and clock.get_routes() == routes, "Duplicate assignment rejected after restart"):
			return
	var atr_price := 0
	for model in clock.get_aircraft_models():
		if model["id"] == "atr72_600":
			atr_price = model["price_eur"]
	var start_capital := int(clock.get_starting_capital().trim_suffix(" EUR"))
	if not _check(fleet.size() == 2 and airline["capital_eur"] == start_capital - 2 * atr_price, "Fleet and debited balance survive complete restart"):
		return
	for index in range(2):
		if not _check(fleet[index]["identifier"] == "AM-%06d" % (index + 1) and fleet[index]["model_id"] == "atr72_600" and fleet[index]["location_icao"] == "LTFJ" and fleet[index]["status"] == "parked" and fleet[index]["purchase_price_eur"] == atr_price, "Aircraft identity, model, location, status and paid price restored"):
			return
	if mode.begins_with("read-"):
		if not _check(main.time_label.text == "Tag 2 · 00:00:00" and main.start_button.disabled == running, "Loaded UI matches Rust"):
			return
		if not _check(main.speed_buttons[2 if running else 1].button_pressed, "Loaded speed is selected"):
			return
		clock.advance(1.0)
		var expected: float = 86400.125 + (speed if running else 0)
		if not _check(clock.get_elapsed_seconds() == expected, "Restored state continues at correct speed or stays paused"):
			return
		await _click(main.navigation[1])
		if not _check(main.fleet_tree.get_root().get_child_count() == 2, "Loaded aircraft appear in fleet table"):
			return
		_select_atr(main)
		await _click(main.buy_button)
		fleet = clock.get_fleet()
		airline = clock.get_airline_info()
		if not _check(fleet.size() == 3 and fleet[2]["identifier"] == "AM-000003" and airline["capital_eur"] == start_capital - 3 * atr_price, "Buying after restart continues IDs and debits saved balance"):
			return
		await _click(main.buy_button)
		if not _check("Nicht genügend Guthaben" in main.fleet_status.text and clock.get_fleet() == fleet and clock.get_airline_info() == airline, "Insufficient funds after restart does not alter state"):
			return
		await _click(main.navigation[0])
		# Refresh time-dependent demand fields after the deliberate clock advance.
		routes = clock.get_routes()
		# All failures must preserve the current live state and the good on-disk save.
		var good_bytes := FileAccess.get_file_as_bytes(path)
		main.save_path = path + ".corrupt"
		for content in ["", "{", "not a save", "{\"version\":1}"]:
			var file := FileAccess.open(main.save_path, FileAccess.WRITE)
			file.store_string(content)
			file.close()
			await _click(main.load_button)
			if not _check(main.save_status.text != "Spielstand geladen." and clock.get_elapsed_seconds() == expected and clock.is_running() == running and clock.get_speed() == speed, "Corrupt load preserves live state"):
				return
		var invalid_company: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(path))
		# Preserve seed/u64 encodings so this fails specifically on the assignment.
		var invalid_routes := FileAccess.get_file_as_string(path).replace('"aircraft_ids":[1]', '"aircraft_ids":[999]')
		var route_file := FileAccess.open(main.save_path, FileAccess.WRITE)
		route_file.store_string(invalid_routes)
		route_file.close()
		await _click(main.load_button)
		if not _check(main.save_status.text != "Spielstand geladen." and clock.get_routes() == routes and clock.get_fleet() == fleet, "Invalid route load preserves live routes and aircraft"):
			return
		invalid_company["state"]["airline"]["country_code"] = "DE"
		var invalid_file := FileAccess.open(main.save_path, FileAccess.WRITE)
		invalid_file.store_string(JSON.stringify(invalid_company))
		invalid_file.close()
		await _click(main.load_button)
		if not _check(main.save_status.text != "Spielstand geladen." and clock.get_airline_info() == airline and clock.get_elapsed_seconds() == expected and clock.get_speed() == speed and clock.is_running() == running, "Invalid airline data cannot replace company or clock"):
			return
		var invalid_fleet: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(path))
		invalid_fleet["state"]["fleet"]["aircraft"][0]["id"] = 2
		invalid_file = FileAccess.open(main.save_path, FileAccess.WRITE)
		invalid_file.store_string(JSON.stringify(invalid_fleet))
		invalid_file.close()
		await _click(main.load_button)
		if not _check(main.save_status.text != "Spielstand geladen." and clock.get_fleet() == fleet and clock.get_airline_info() == airline and clock.get_elapsed_seconds() == expected, "Corrupt fleet cannot erase aircraft or money"):
			return
		main.save_path = path + ".missing"
		await _click(main.load_button)
		if not _check(main.save_status.text != "Spielstand geladen." and clock.get_elapsed_seconds() == expected, "Missing save is safe"):
			return
		main.save_path = path.path_join("blocked.json")
		await _click(main.save_button)
		if not _check(main.save_status.text != "Spielstand gespeichert." and FileAccess.get_file_as_bytes(path) == good_bytes, "Write failure leaves existing save intact"):
			return
		root.size = Vector2i(680, 520)
		await process_frame
		await process_frame
		for control in [main.save_button, main.load_button, main.save_status]:
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Save controls and error fit minimum window"):
				return
	print("PASS: %s (process %d)" % [mode, OS.get_process_id()])
	main.queue_free()
	await process_frame
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


func _select_atr(main) -> void:
	for index in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(index)["id"] == "atr72_600":
			main.model_select.select(index)
			main.model_select.item_selected.emit(index)
			return


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
