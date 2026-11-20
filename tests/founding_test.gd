extends SceneTree


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var directory := OS.get_cache_dir().path_join("air-founding-slots-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main = scene.instantiate()
	root.add_child(main)
	await process_frame
	main.set_process(false)
	main.save_user_dir = directory
	main.autosave = preload("res://scripts/autosave.gd").new(directory.path_join("preferences.cfg"))
	if not _check(main.founding.visible and not main.overview.visible and not main.simulation.has_airline() and main.name_input.has_focus(), "New game begins at founding form"):
		return
	if not _check("pausiert" in main.founding_status.text and "benannten Speicherplatz" in main.founding_status.text and not "ersetzt" in main.founding_status.text, "First launch message: paused start and named slot"):
		return
	main.name_input.text = "   "
	await _click(main.found_button)
	if not _check(not main.simulation.has_airline() and main.founding.visible and "Namen" in main.founding_status.text, "Empty name rejected by Rust"):
		return
	for index in range(main.country_select.item_count):
		main.country_select.select(index)
		main.country_select.item_selected.emit(index)
		var code: String = main.country_select.get_selected_metadata()
		var airports: Array = main.simulation.get_airports(code)
		if not _check(main.airport_select.item_count == airports.size() and airports.size() > 0, "Country filters available airports"):
			return
		airports.sort_custom(func(a: Dictionary, b: Dictionary): return a["iata"].naturalnocasecmp_to(b["iata"]) < 0)
		for airport_index in range(airports.size()):
			if not _check(main.airport_select.get_item_metadata(airport_index) == airports[airport_index]["icao"], "Stable airport identifiers come from Rust"):
				return
	for i in range(main.country_select.item_count):
		if main.country_select.get_item_metadata(i) == "TR":
			main.country_select.select(i)
			main.country_select.item_selected.emit(i)
	for i in range(main.airport_select.item_count):
		if main.airport_select.get_item_metadata(i) == "LTFJ": main.airport_select.select(i)
	main.name_input.text = "  Bosporus 日本 Air  "
	await _click(main.found_button)
	var airline: Dictionary = main.simulation.get_airline_info()
	if not _check(main.overview.visible and not main.founding.visible and airline["name"] == "Bosporus 日本 Air" and airline["airport_icao"] == "LTFJ", "Actual founding button opens overview with canonical company"):
		return
	if not _check(main.heading.text == airline["name"] and "LTFJ" in main.description.text and main.metrics["balance"].is_visible_in_tree() and main.metrics["balance"].text == main.UI.money(airline["capital_eur"]) and "%d EUR" % airline["capital_eur"] == main.simulation.get_starting_capital(), "Overview displays Rust company and configured capital"):
		return
	if not _check(not main.simulation.is_running() and main.simulation.get_elapsed_seconds() == 0.0 and main.simulation.get_speed() == 1, "Founding starts a fresh paused clock"):
		return
	var error: String = main.simulation.found_airline("Wrong country", "DE", "LTFM")
	if not _check(not error.is_empty() and main.simulation.get_airline_info() == airline, "Cross-country airport rejected without state loss"):
		return
	main.simulation.start()
	main.simulation.advance(12.5)
	main.save_button.pressed.emit()
	await process_frame
	main.save_slots_dialog.name_field.text = "Erste Airline"
	main.save_slots_dialog.request_save()
	main.save_slots_dialog.close_dialog()
	await process_frame
	await _click(main.new_game_button)
	if not _check(main.founding.visible and main.simulation.get_airline_info() == airline, "Opening new game does not destroy existing company"):
		return
	if not _check("ersetzt die laufende Airline" in main.founding_status.text and "ungespeichertem Fortschritt" in main.founding_status.text and "unter Laden" in main.founding_status.text, "New Game message: replaces running company, slots stay under Load"):
		return
	await _click(main.cancel_founding_button)
	if not _check(main.overview.visible and main.simulation.get_elapsed_seconds() == 12.5 and main.simulation.is_running(), "Cancel keeps company, time and running status"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Second Air"
	await _click(main.found_button)
	if not _check(main.heading.text == "Second Air" and main.simulation.get_elapsed_seconds() == 0.0 and not main.simulation.is_running(), "Confirmed new game replaces state and resets time"):
		return
	main.simulation.start()
	main.simulation.advance(3.0)
	main.save_button.pressed.emit()
	await process_frame
	main.save_slots_dialog.name_field.text = "Zweite Airline"
	main.save_slots_dialog.request_save()
	var slots: Array = main.simulation.get_save_slots(directory)["slots"]
	if not _check(slots[0]["occupied"] and slots[1]["occupied"] and not main.simulation.get_save_slots(directory)["autosave"]["occupied"], "Second company saved in a free slot, first slot and autosave untouched"):
		return
	main.save_slots_dialog.close_dialog()
	await process_frame
	for case in [[0, "Bosporus 日本 Air", 12.5], [1, "Second Air", 3.0]]:
		main._open_save_slots(false)
		await process_frame
		main.save_slots_dialog.load_slot(case[0], false)
		await process_frame
		if not _check(main.simulation.get_airline_info()["name"] == case[1] and main.simulation.get_elapsed_seconds() == case[2], "Slot %d loads its own company and saved clock" % case[0]):
			return
	root.size = Vector2i(680, 520)
	await process_frame
	await process_frame
	if not _check(root.get_visible_rect().encloses(main.new_game_button.get_global_rect()), "New game button fits minimum window"):
		return
	await _click(main.new_game_button)
	await process_frame
	for control in [main.found_button, main.founding_load_button, main.cancel_founding_button, main.founding_status]:
		if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Founding actions fit minimum window"):
			return
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-dir="):
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join("founding-minimum.png"))
	print("PASS: founding UI, Rust validation, real airport choices, configured capital, overview and cancellable new game")
	main.queue_free()
	await process_frame
	quit(0)


func _click(button: Button) -> void:
	# Let container layout settle after switching between the two screens.
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
