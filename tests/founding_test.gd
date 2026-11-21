extends SceneTree


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	TranslationServer.set_locale("de")
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
	var before_cash: int = airline["capital_eur"]
	main.simulation.advance(1.0)
	var live: Dictionary = main.simulation.get_airline_info()
	var live_time: float = main.simulation.get_elapsed_seconds()
	main.name_input.text = "   "
	await _click(main.found_button)
	if not _check(main.founding.visible and "Namen" in main.founding_status.text and not is_instance_valid(main.new_game_dialog) and main.simulation.get_airline_info() == live, "Invalid New Game reports in place without confirmation"):
		return
	main.name_input.text = "Second Air"
	await _click(main.found_button)
	await process_frame
	var dialog = main.new_game_dialog
	if not _check(is_instance_valid(dialog) and dialog.visible and main.simulation.get_airline_info() == live, "Valid New Game asks before replacing"):
		return
	var dialog_text := ""
	for node in dialog.find_children("*", "Label", true, false): dialog_text += node.text + "\n"
	if not _check("Bosporus 日本 Air" in dialog_text and "Second Air" in dialog_text and ("Unsaved progress" in dialog_text or "Nicht gespeicherter Fortschritt" in dialog_text) and ("Named saves" in dialog_text or "Spielstände bleiben" in dialog_text), "Confirmation names both airlines and unsaved progress"):
		return
	if not _check(dialog.safe_button.has_focus(), "Initial focus on Cancel"):
		return
	var held: float = main.simulation.get_elapsed_seconds()
	main._process(5.0)
	if not _check(main.simulation.get_elapsed_seconds() == held, "Clock held while the confirmation is open"):
		return
	var event := InputEventKey.new()
	event.keycode = KEY_ESCAPE
	event.pressed = true
	Input.parse_input_event(event)
	await process_frame
	await process_frame
	await process_frame
	if not _check(not is_instance_valid(main.new_game_dialog) and main.simulation.get_airline_info() == live and main.name_input.text == "Second Air" and main.found_button.has_focus() and main.founding.visible, "Escape dismisses, keeps the draft and refocuses Found"):
		return
	await _click(main.cancel_founding_button)
	if not _check(main.overview.visible and main.simulation.get_airline_info() == live and main.simulation.get_elapsed_seconds() == live_time and main.simulation.is_running() and live["capital_eur"] == before_cash, "Cancel keeps company, time and running status"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Second Air"
	root.size = Vector2i(680, 520)
	await process_frame
	await _click(main.found_button)
	await process_frame
	await process_frame
	dialog = main.new_game_dialog
	if not _check(is_instance_valid(dialog) and dialog.safe_button.has_focus() and root.get_visible_rect().encloses(dialog.card.get_global_rect()) and root.get_visible_rect().encloses(dialog.safe_button.get_global_rect()) and root.get_visible_rect().encloses(dialog.danger_button.get_global_rect()), "Confirmation fits 680x520 with initial focus on Cancel"):
		return
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-dir="):
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join("new-game-confirm-680x520.png"))
			root.size = Vector2i(1440, 1000)
			await process_frame
			await process_frame
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join("new-game-confirm-1440x1000.png"))
			root.size = Vector2i(680, 520)
			await process_frame
	await _click(dialog.safe_button)
	await process_frame
	await process_frame
	if not _check(main.founding.visible and main.found_button.has_focus() and main.simulation.get_airline_info() == live, "Cancel button dismisses and restores focus"):
		return
	root.size = Vector2i(1152, 648)
	await _click(main.found_button)
	await process_frame
	await _click(main.new_game_dialog.danger_button)
	await process_frame
	if not _check(main.heading.text == "Second Air" and main.simulation.get_elapsed_seconds() == 0.0 and not main.simulation.is_running(), "Explicit confirmation replaces state and resets time"):
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
	main.queue_free()
	await process_frame
	if not await _english_founding():
		return
	print("PASS: founding UI, Rust validation, real airport choices, configured capital, overview and cancellable new game")
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


func _texts(node: Node) -> String:
	var text := ""
	for child in node.find_children("*", "Label", true, false): text += child.text + "\n"
	for child in node.find_children("*", "Button", true, false): text += child.text + "\n"
	return text


func _english_founding() -> bool:
	TranslationServer.set_locale("en")
	var directory := OS.get_cache_dir().path_join("air-founding-en-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	var main = (load("res://scenes/main.tscn") as PackedScene).instantiate()
	root.add_child(main)
	await process_frame
	main.set_process(false)
	main.save_user_dir = directory
	main.autosave = preload("res://scripts/autosave.gd").new(directory.path_join("preferences.cfg"))
	var form := _texts(main.founding)
	for expected in ["YOUR OWN AIRLINE", "A new beginning.", "Your airline's name", "Home country", "Home airport", "Found airline", "Load game", "Cancel", "starts paused on day 1"]:
		if not _check(expected in form, "English founding shows: " + expected):
			return false
	for german in ["Heimat", "Name deiner", "gründen", "Abbrechen", "pausiert", "Ein neuer"]:
		if not _check(not german in form, "English founding has no German: " + german):
			return false
	if not _check(main.name_input.placeholder_text == "Airline name (1–80 characters)", "English placeholder"):
		return false
	var names := []
	for i in range(main.country_select.item_count): names.append(main.country_select.get_item_text(i))
	if not _check("Germany" in names and not "Deutschland" in names and not "Türkei" in names, "Country menu in English"):
		return false
	for i in range(main.country_select.item_count):
		if main.country_select.get_item_metadata(i) == "TR":
			main.country_select.select(i)
			main.country_select.item_selected.emit(i)
	for i in range(main.airport_select.item_count):
		if main.airport_select.get_item_metadata(i) == "LTFJ": main.airport_select.select(i)
	main.difficulty_choices["capital"][0].pressed.emit()
	var level: String = main._difficulty_level("capital")
	for blank in ["", "   "]:
		main.name_input.text = blank
		await _click(main.found_button)
		if not _check(not main.simulation.has_airline() and main.founding.visible and "Please enter an airline name of 1 to 80 characters" in main.founding_status.text and main.name_input.has_focus(), "Blank name gives English correction hint and focus"):
			return false
	if not _check(main.country_select.get_selected_metadata() == "TR" and main.airport_select.get_selected_metadata() == "LTFJ" and main._difficulty_level("capital") == level, "Base and start conditions survive the error"):
		return false
	main.name_input.text = "English Air"
	await _click(main.found_button)
	var airline: Dictionary = main.simulation.get_airline_info()
	if not _check(main.overview.visible and airline["name"] == "English Air" and airline["airport_icao"] == "LTFJ" and not main.simulation.is_running() and main.simulation.get_elapsed_seconds() == 0.0 and main.metrics["balance"].text == main.UI.money(airline["capital_eur"]), "Corrected English founding opens the paused overview"):
		return false
	if not _check("founded" in main.save_status.text, "English founding feedback"):
		return false
	await _click(main.new_game_button)
	if not _check("replaces the running airline" in main.founding_status.text, "English New Game message"):
		return false
	main.name_input.text = "   "
	await _click(main.found_button)
	if not _check(main.founding.visible and "Please enter an airline name" in main.founding_status.text and not is_instance_valid(main.new_game_dialog), "English invalid New Game stays in form"):
		return false
	main.name_input.text = "Next Air"
	await _click(main.found_button)
	await process_frame
	var dialog = main.new_game_dialog
	var body := _texts(dialog)
	if not _check(is_instance_valid(dialog) and "Current airline: English Air" in body and "New airline: Next Air" in body and "Found a new airline?" in body and "Named saves stay available" in body and "Cancel" in body and "Found airline" in body, "English confirmation"):
		return false
	await _click(dialog.safe_button)
	await process_frame
	await process_frame
	if not _check(main.name_input.text == "Next Air" and main.simulation.get_airline_info() == airline, "Cancel keeps draft and company"):
		return false
	main.queue_free()
	await process_frame
	TranslationServer.set_locale("de")
	return true
