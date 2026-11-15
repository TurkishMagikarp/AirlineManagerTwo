extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main: Control = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Milestone test", "DE", "EDDF").is_empty())
	assert(sim.get_milestones().size() == 15)
	assert(sim.get_milestones().all(func(e: Dictionary): return e.reached < 0))
	assert(sim.buy_aircraft("atr72_600").is_empty())
	var live: Dictionary = sim.get_milestones().filter(func(e: Dictionary): return e.id == "fleet_1")[0]
	assert(live.current == 1 and live.reached == -1)
	sim.start()
	assert(sim.advance(86400.0))
	sim.pause()
	var entries: Array = sim.get_milestones()
	var first: Dictionary = entries.filter(func(e: Dictionary): return e.id == "fleet_1")[0]
	assert(first.reached == 86400 and first.current == 1)
	main._sync_view()
	main.milestones_panel.notify_reached()
	assert(is_instance_valid(main.milestones_panel.notification))
	assert(sim.take_milestone_notifications().is_empty())
	var path := ProjectSettings.globalize_path("user://milestones-test.json")
	assert(sim.save_game(path).is_empty())
	assert(sim.load_game(path).is_empty())
	assert(sim.get_milestones() == entries)
	assert(sim.take_milestone_notifications().is_empty())
	DirAccess.remove_absolute(path)
	root.gui_embed_subwindows = true
	main.milestones_panel.open_list()
	await process_frame
	assert(sim.found_airline("Fresh goals", "DE", "EDDF").is_empty())
	main._sync_view()
	assert(not is_instance_valid(main.milestones_panel.notification))
	assert(sim.get_milestones().all(func(e: Dictionary): return e.reached < 0))
	main.queue_free()
	await process_frame
	print("PASS: milestone API, Overview, dialog, toast delivery, restart and reset")
	quit(0)
