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
		await _click(main.start_button)
		await _click(main.speed_buttons[2 if running else 1])
		clock.advance(86400.125 / speed)
		if not running:
			await _click(main.pause_button)
		await _click(main.save_button)
		if not _check(main.save_status.text == "Spielstand gespeichert.", "Save button writes via Rust"):
			return
	else:
		await _click(main.load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load button restores via Rust after restart"):
			return
	if not _check(clock.get_elapsed_seconds() == 86400.125 and clock.is_running() == running and clock.get_speed() == speed, "Exact saved time, running flag and speed"):
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
