extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main: Control = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Macro UI", "DE", "EDDF").is_empty())
	assert(sim.get_world_economy().percent == 100.0)
	assert(sim.get_world_economy().phase == "normal")
	main._sync_view()
	for language in ["de", "en"]:
		TranslationServer.set_locale(language)
		main._refresh_finance()
		assert(main.metrics.economy.text == ("100,0 %" if language == "de" else "100.0%"))
		assert(main.metrics.economy.get_parent().get_child(0).text == tr("ECONOMY_TITLE"))
		assert(main.metrics.economy.get_parent().get_parent().tooltip_text == tr("ECONOMY_TOOLTIP"))
		assert(main.economy_support.get_child_count() == 1)
	sim.start()
	assert(sim.advance(364.0 * 86400.0))
	sim.pause()
	main._notify_economy()
	assert(is_instance_valid(main.economy_toast))
	assert(sim.take_economy_notifications().is_empty())
	var path := ProjectSettings.globalize_path("user://world-economy-test.json")
	assert(sim.save_game(path).is_empty())
	var index: Dictionary = sim.get_world_economy()
	assert(sim.load_game(path).is_empty())
	assert(sim.get_world_economy() == index)
	assert(sim.take_economy_notifications().is_empty())
	DirAccess.remove_absolute(path)
	main.queue_free()
	await process_frame
	print("PASS: economy API, translated metric/chip/tooltip, phase toast and saved delivery")
	quit(0)
