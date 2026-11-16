extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	TranslationServer.set_locale("en")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	await process_frame
	for key in ["capital", "competition", "crises"]:
		assert(main._difficulty_level(key) == "standard")
	assert(main.difficulty_preset[1].button_pressed)
	main.difficulty_preset[0].pressed.emit()
	assert(main._difficulty_level("capital") == "high")
	assert(main._difficulty_level("competition") == "low")
	assert(main._difficulty_level("crises") == "low")
	# A manual selection overrides one axis without reapplying the preset.
	main.difficulty_choices["crises"][2].button_pressed = true
	main.difficulty_choices["crises"][2].pressed.emit()
	assert(main._difficulty_level("capital") == "high")
	assert(main._difficulty_level("competition") == "low")
	assert(main._difficulty_level("crises") == "high")
	main.name_input.text = "Custom world"
	main.found_button.pressed.emit()
	await process_frame
	var sim = main.simulation
	var info: Dictionary = sim.get_airline_info()
	assert(info.difficulty_capital == "high" and info.difficulty_competition == "low" and info.difficulty_crises == "high")
	assert(info.capital_eur == 20000000 and sim.get_competitors().size() == 1)
	assert(main.overview.visible and "High" in main.difficulty_caption.text and "Low" in main.difficulty_caption.text)
	var before: Dictionary = info.duplicate(true)
	assert(not sim.found_airline_with_levels("Bad", "DE", "EDDF", "wrong", "standard", "standard").is_empty())
	assert(sim.get_airline_info() == before)
	var path := ProjectSettings.globalize_path("user://difficulty-test.json")
	assert(sim.save_game(path).is_empty())
	assert(JSON.parse_string(FileAccess.get_file_as_string(path))["version"] == 40)
	assert(sim.load_game(path).is_empty())
	assert(sim.get_airline_info() == before)
	main._sync_view()
	assert("High" in main.difficulty_caption.text)
	DirAccess.remove_absolute(path)
	main._open_founding()
	for key in ["capital", "competition", "crises"]:
		assert(main._difficulty_level(key) == "standard")
	main._sync_view()
	assert(sim.get_airline_info() == before)
	main.queue_free()
	await process_frame
	print("PASS: standard defaults, preset with manual override, nonstandard form founding, Rust validation, overview and v40 round trip")
	quit(0)
