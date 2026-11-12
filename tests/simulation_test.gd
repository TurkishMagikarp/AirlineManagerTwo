extends SceneTree
## Exercises the real Rust extension and the controls in the actual main scene.


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main = scene.instantiate()
	root.add_child(main)
	await process_frame
	main.set_process(false)
	main.name_input.text = "Simulation Air"
	main.found_button.pressed.emit()
	var clock = main.simulation
	if not _check(clock.get_elapsed_seconds() == 0.0 and not clock.is_running(), "Initial state is paused at zero"):
		return
	if not _check(main.time_label.text == "Tag 1 · 00:00:00" and main.pause_button.disabled, "Initial Rust state is visible"):
		return
	clock.advance(10.0)
	if not _check(clock.get_elapsed_seconds() == 0.0, "Paused clock does not advance"):
		return
	await _click(main.start_button)
	if not _check(clock.is_running() and main.start_button.disabled, "Start button reaches Rust"):
		return
	var expected := 0.0
	for index in range(3):
		await _click(main.speed_buttons[index])
		var speed: int = [1, 4, 16][index]
		if not _check(clock.get_speed() == speed, "Speed button reaches Rust: %d" % speed):
			return
		clock.advance(0.5)
		expected += 0.5 * speed
		main._refresh_time()
		if not _check(is_equal_approx(clock.get_elapsed_seconds(), expected), "Rust applies multiplier"):
			return
		for other in range(3):
			if not _check(main.speed_buttons[other].button_pressed == (other == index), "Exclusive speed selection"):
				return
	await _click(main.pause_button)
	clock.advance(50.0)
	if not _check(clock.get_elapsed_seconds() == expected and not clock.is_running(), "Pause button stops time without reset"):
		return
	await _click(main.speed_buttons[0])
	if not _check(not clock.is_running() and clock.get_speed() == 1, "Changing speed does not unpause"):
		return
	if not _check(not clock.set_speed(-1) and not clock.set_speed(2) and not clock.advance(-1.0), "FFI rejects invalid commands"):
		return
	await _click(main.start_button)
	clock.advance(0.5)
	expected += 0.5
	if not _check(clock.get_elapsed_seconds() == expected, "Resume continues existing time"):
		return
	# Now exercise the actual frame -> GDScript -> Rust -> label path.
	main.set_process(true)
	await create_timer(0.2).timeout
	await _click(main.pause_button)
	if not _check(clock.get_elapsed_seconds() > expected, "Real Godot frames advance Rust time"):
		return
	var paused_time: float = clock.get_elapsed_seconds()
	await create_timer(0.1).timeout
	if not _check(clock.get_elapsed_seconds() == paused_time, "Real frames respect pause"):
		return
	if not _check(main.time_label.text == clock.get_display_time() and main.state_label.text == "Pausiert · 1×", "UI reflects Rust snapshot"):
		return
	await _click(main.navigation[1])
	if not _check(main.current_page == 1 and clock.get_elapsed_seconds() == paused_time, "Navigation preserves simulation"):
		return
	var other_clock := SimulationClock.new()
	other_clock.start()
	other_clock.advance(86400.0)
	if not _check(other_clock.get_display_time() == "Tag 2 · 00:00:00" and clock.get_elapsed_seconds() == paused_time, "Day rollover and independent instances"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	await process_frame
	for control in [main.time_label, main.start_button, main.pause_button, main.speed_buttons[2]]:
		if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Time controls fit minimum window"):
			return
	print("PASS: Rust extension, real mouse controls, 1x/4x/16x, pause/resume, frame updates, time display, rollover and navigation")
	main.queue_free()
	await process_frame
	quit(0)


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
