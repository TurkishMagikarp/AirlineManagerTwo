extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.jump(3600)["advanced_seconds"] == 0)
	assert(sim.found_airline("Jump UI", "DE", "EDDF").is_empty())
	main._sync_view()
	assert(main.jump_buttons.size() == 4)
	assert(sim.get_jump_options().map(func(o): return o["seconds"]) == [3600, 86400, 604800, 2592000])
	for speed in [1, 4, 16]:
		for running in [false, true]:
			sim.set_speed(speed)
			if running: sim.start()
			else: sim.pause()
			var before: float = sim.get_elapsed_seconds()
			main.jump_buttons[0].grab_focus()
			main.jump_buttons[0].pressed.emit()
			assert(not sim.begin_jump(3600))
			assert(sim.advance(1000))
			assert(sim.get_elapsed_seconds() == before)
			while main.jump_in_progress:
				await process_frame
			assert(sim.get_elapsed_seconds() == before + 3600)
			assert(sim.is_running() == running)
			assert(sim.get_speed() == speed)
			assert(main.jump_status.visible)
			assert("abgeschlossen" in main.jump_status.text)
			assert(main.time_label.text == sim.get_display_time())
			assert(main.jump_buttons[0].has_focus())
			assert(main.jump_buttons[0].focus_mode == Control.FOCUS_ALL)
	main.queue_free()
	await process_frame
	print("PASS: configured jump buttons forward to Rust, refresh status, retain pause/speed")
	quit(0)
