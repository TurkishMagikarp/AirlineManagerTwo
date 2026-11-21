extends SceneTree
var quits := 0

func _initialize() -> void:
	call_deferred("_run")

func _check(ok: bool, what: String) -> bool:
	if not ok:
		push_error("FAIL: " + what)
		quit(1)
	return ok

func _key(code: int) -> void:
	var event := InputEventKey.new()
	event.keycode = code
	event.pressed = true
	Input.parse_input_event(event)

func _run() -> void:
	TranslationServer.set_locale("de")
	var directory := OS.get_cache_dir().path_join("air-quit-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	var main: Control = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	await process_frame
	main.save_user_dir = directory
	main.autosave = preload("res://scripts/autosave.gd").new(directory.path_join("preferences.cfg"))
	main.quit_handler = func(): quits += 1
	var sim: RefCounted = main.simulation
	main.request_quit()
	if not _check(quits == 1 and not main._quit_prompt_open(), "startup closes immediately"): return
	assert(sim.found_airline("Quit Air", "DE", "EDDF").is_empty())
	main._sync_view()
	sim.set_speed(4)
	sim.start()
	await process_frame
	main.notification(Node.NOTIFICATION_WM_CLOSE_REQUEST)
	await process_frame
	if not _check(quits == 1 and main._quit_prompt_open(), "close request shows prompt"): return
	var dialog = main.quit_dialog
	if not _check("Quit Air" in dialog.card.get_child(0).get_child(1).text and "verloren" in dialog.card.get_child(0).get_child(1).text and "Abbrechen" in dialog.card.get_child(0).get_child(1).text, "prompt names airline and warns"): return
	if not _check(dialog.safe_button.has_focus(), "focus on Cancel"): return
	var t0: float = sim.get_elapsed_seconds()
	for i in 5:
		main.request_quit()
		main.notification(Node.NOTIFICATION_WM_CLOSE_REQUEST)
		await process_frame
	if not _check(quits == 1 and main.quit_dialog == dialog and get_nodes_in_group("quit_prompt").size() == 1, "repeats do not stack or accept"): return
	main._process(1.0)
	main.autosave.tick(0.0, false)
	if not _check(sim.get_elapsed_seconds() == t0, "clock held"): return
	_key(KEY_ESCAPE)
	await process_frame
	await process_frame
	if not _check(not main._quit_prompt_open() and quits == 0 + 1 and sim.is_running(), "escape cancels, running kept"): return
	main._process(1.0)
	if not _check(sim.get_elapsed_seconds() > t0, "clock resumes"): return
	main.request_quit()
	await process_frame
	dialog = main.quit_dialog
	dialog.get_ok_button().pressed.emit()
	await process_frame
	if not _check(quits == 2, "explicit Quit exits"): return
	if not _check(not FileAccess.file_exists(directory.path_join("autosave.sav")) and sim.get_save_slots(directory)["slots"].all(func(r): return not r["occupied"]), "no save written"): return
	# Prompt above New Game form names current airline.
	main._open_founding()
	main.request_quit()
	await process_frame
	if not _check("Quit Air" in main.quit_dialog.card.get_child(0).get_child(1).text, "founding form still names open airline"): return
	main.quit_dialog.safe_button.pressed.emit()
	await process_frame
	await process_frame
	# Close during jump is handled once at completion.
	main.jump_in_progress = true
	main.request_quit()
	if not _check(main.quit_requested_during_jump and not main._quit_prompt_open(), "jump defers"): return
	main.jump_in_progress = false
	main.quit_requested_during_jump = false
	print("PASS: quit confirmation, hold, repeats and explicit exit")
	quit(0)
