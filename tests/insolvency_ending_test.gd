extends SceneTree
## Main scene + real GDExtension: insolvency is a persistent ending and New Game founds a separate world.

const DAY := 86400.0


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var directory := OS.get_cache_dir().path_join("air-insolvency-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	await process_frame
	main.set_process(false)
	main.save_user_dir = directory
	main.autosave = preload("res://scripts/autosave.gd").new(directory.path_join("preferences.cfg"))
	var sim: RefCounted = main.simulation
	if not _check(sim.found_airline("Gesund Air", "DE", "EDDF").is_empty(), "Found healthy company"):
		return
	main._sync_view()
	sim.start()
	sim.advance(5.0)
	await _save_slot(main, "Gesund", 0)
	if not _check(not main.terminal_notice.visible and not main.start_button.disabled or sim.is_running(), "Healthy company has no ending notice"):
		return

	# Recovery before the deadline lifts the restrictions.
	_make_distressed(sim, directory)
	sim.start()
	sim.advance(31.0 * DAY)
	var info: Dictionary = sim.get_financing()
	if not _check(info["distressed"] and info["deadline"] != "—", "Deadline starts"):
		return
	main._refresh_time()
	if not _check(not main.terminal_notice.visible, "No notice before the deadline"):
		return
	if not _check(sim.borrow_money(500000).is_empty(), "Eligible rescue accepted"):
		return
	info = sim.get_financing()
	if not _check(not info["distressed"] and info["deadline"] == "—", "Rescue lifts distress and the deadline"):
		return
	sim.advance(100.0 * DAY)
	main._refresh_time()
	if not _check(not sim.get_airline_info()["insolvent"] and not main.terminal_notice.visible and not main.terminal_state, "Rescue keeps the company playable"):
		return

	# Ordinary advance into insolvency.
	_make_distressed(sim, directory)
	sim.start()
	sim.advance(31.0 * DAY)
	var elapsed_before: float = sim.get_elapsed_seconds()
	sim.advance(70.0 * DAY)
	main._refresh_time()
	main._show_page(0)
	if not _check(sim.get_airline_info()["insolvent"] and main.terminal_notice.visible, "Ordinary advance ends management with a notice"):
		return
	var deadline_clock: float = sim.get_elapsed_seconds()
	if not _check(deadline_clock > elapsed_before and deadline_clock < elapsed_before + 70.0 * DAY, "Clock stops at the deadline"):
		return
	await _check_terminal(main, deadline_clock)
	var terminal_airline: Dictionary = sim.get_airline_info()
	await _save_slot(main, "Insolvent", 1)

	# Load cancel returns to the same company.
	await _click(main.terminal_load_button)
	if not _check(is_instance_valid(main.save_slots_dialog), "Notice opens the slot dialog"):
		return
	main.save_slots_dialog.close_dialog()
	await process_frame
	await process_frame
	if not _check(main.terminal_notice.visible and sim.get_airline_info() == terminal_airline, "Load cancel keeps the ended company"):
		return

	# New Game: cancel, invalid and valid submissions.
	await _click(main.terminal_new_game_button)
	if not _check(main.founding.visible, "New Game opens founding"):
		return
	await _click(main.cancel_founding_button)
	if not _check(main.overview.visible and main.terminal_notice.visible and sim.get_airline_info() == terminal_airline, "Cancel preserves the terminal company"):
		return
	await _click(main.terminal_new_game_button)
	main.name_input.text = "   "
	await _click(main.found_button)
	if not _check(main.founding.visible and sim.get_airline_info() == terminal_airline and not main.founding_status.text.is_empty(), "Invalid founding preserves the terminal company"):
		return
	main.name_input.text = "Neu Air"
	await _click(main.found_button)
	var fresh: Dictionary = sim.get_airline_info()
	if not _check(fresh["name"] == "Neu Air" and not fresh["insolvent"] and not main.terminal_notice.visible and sim.get_elapsed_seconds() == 0.0 and not sim.is_running() and not main.start_button.disabled and not main.jump_buttons[0].disabled, "Valid founding starts a separate paused Day 1 world"):
		return
	for index in range(2):
		main._open_save_slots(false)
		await process_frame
		main.save_slots_dialog.load_slot(index, false)
		await process_frame
		await process_frame
		var loaded: Dictionary = sim.get_airline_info()
		if index == 0:
			if not _check(loaded["name"] == "Gesund Air" and not main.terminal_notice.visible and sim.get_elapsed_seconds() == 5.0 and not main.jump_buttons[0].disabled, "Healthy slot restores normal controls at its clock"):
				return
		elif not _check(loaded["name"] == "Distressed Air" and main.terminal_notice.visible and sim.get_elapsed_seconds() == deadline_clock, "Terminal slot reloads with the notice"):
			return
	await _check_terminal(main, deadline_clock)

	# Jump into insolvency.
	_make_distressed(sim, directory)
	sim.start()
	sim.advance(31.0 * DAY)
	sim.pause()
	main._refresh_time()
	for _i in range(4):
		if sim.get_airline_info()["insolvent"]: break
		await main._jump(2592000)
	main._refresh_time()
	if not _check(sim.get_airline_info()["insolvent"] and main.terminal_notice.visible and main.jump_buttons[3].disabled, "Jump into insolvency ends management"):
		return

	root.gui_embed_subwindows = true
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for page in [0, 4]:
			main._show_page(page)
			for frame in range(5): await process_frame
			for control in [main.terminal_notice, main.terminal_load_button, main.terminal_new_game_button]:
				if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Notice fits %s page %d" % [dimensions, page]):
					return
			for argument in OS.get_cmdline_user_args():
				if argument.begins_with("--capture-dir="):
					await RenderingServer.frame_post_draw
					root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join("insolvency-%s-%dx%d.png" % [["overview", "", "", "", "finances"][page], dimensions.x, dimensions.y]))
	main.terminal_load_button.grab_focus()
	if not _check(main.terminal_load_button.has_focus() and main.terminal_new_game_button.focus_mode == Control.FOCUS_ALL and main.terminal_load_button.get_parent() == main.terminal_new_game_button.get_parent(), "Notice actions are keyboard reachable"):
		return
	main.terminal_new_game_button.grab_focus()
	await process_frame
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-dir="):
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join("insolvency-focus-680x520.png"))
	print("PASS: insolvency ending, terminal reload, New Game separate world")
	main.queue_free()
	await process_frame
	quit(0)


func _check_terminal(main, deadline_clock: float) -> void:
	var sim: RefCounted = main.simulation
	for page in range(main.PAGES.size()):
		main._show_page(page)
		main._refresh_time()
		if not _check(main.terminal_notice.is_visible_in_tree() and sim.get_airline_info()["name"] in main.terminal_body.text, "Notice on page %d" % page):
			return
	if not _check(main.start_button.disabled and main.jump_buttons.all(func(b): return b.disabled) and main.speed_buttons.all(func(b): return b.disabled), "Start, speed and jumps unavailable"):
		return
	main._refresh_commitments_jump()
	if not _check(main.commitments_jump_button.disabled, "Commitment jump unavailable"):
		return
	sim.start()
	sim.advance(10.0 * DAY)
	if not _check(sim.get_elapsed_seconds() == deadline_clock and not sim.is_running(), "Clock stays at the deadline"):
		return
	main._show_page(4)
	if not _check(main.finances_panel.visible and main.save_button.is_visible_in_tree() and not main.save_button.disabled, "Finances and saving stay available"):
		return


func _save_slot(main, title: String, id: int) -> void:
	main._open_save_slots(true)
	await process_frame
	main.save_slots_dialog.selected = id
	main.save_slots_dialog.name_field.text = title
	main.save_slots_dialog.request_save()
	main.save_slots_dialog.close_dialog()
	await process_frame
	await process_frame


func _make_distressed(sim: RefCounted, directory: String) -> void:
	var path := directory.path_join("distress.json")
	var name := "Distressed Air"
	var seed: RefCounted = ClassDB.instantiate("SimulationClock")
	assert(seed.found_airline(name, "DE", "EDDF").is_empty())
	assert(seed.save_game(path).is_empty())
	var text := FileAccess.get_file_as_string(path)
	var fixture: Dictionary = JSON.parse_string(text)
	var cash: int = fixture["state"]["airline"]["capital_eur"]
	var opening: int = fixture["state"]["history"]["opening_balance"]
	var position := text.find('"capital_eur":', text.find('"name":"%s"' % name))
	text = text.left(position) + text.substr(position).replace('"capital_eur":%d' % cash, '"capital_eur":-1')
	position = text.rfind('"opening_balance":')
	text = text.left(position) + text.substr(position).replace('"opening_balance":%d' % opening, '"opening_balance":%d' % (opening - cash - 1))
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(text)
	file.close()
	assert(sim.load_game(path).is_empty())
	DirAccess.remove_absolute(path)


func _click(button: Button) -> void:
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
