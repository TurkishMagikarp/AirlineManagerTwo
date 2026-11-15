extends SceneTree
var main

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	assert(sim.found_airline("Ferry UI", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("a320_200").is_empty())
	assert(sim.hire_pilots("A320 family", 4).is_empty())
	main._sync_view()
	main._show_page(1)
	assert(not main.ferry_button.disabled)
	main.ferry_button.pressed.emit()
	await process_frame
	assert(main.ferry_dialog.visible)
	for i in range(main.ferry_destination.item_count):
		if main.ferry_destination.get_item_metadata(i) == "NZAA":
			main.ferry_destination.select(i)
			main.ferry_destination.item_selected.emit(i)
	assert(main.ferry_dialog.get_ok_button().disabled)
	assert("Reichweite" in main.ferry_preview.text)
	for i in range(main.ferry_destination.item_count):
		if main.ferry_destination.get_item_metadata(i) == "EDDM":
			main.ferry_destination.select(i)
			main.ferry_destination.item_selected.emit(i)
	assert(not main.ferry_dialog.get_ok_button().disabled)
	var quote: Dictionary = sim.preview_ferry(1, "EDDM")
	assert(main.UI.money(quote["cost_eur"]) in main.ferry_preview.text)
	main.ferry_dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_fleet()[0]["ferry_state"] == "Überführung")
	assert(main.ferry_button.disabled)
	assert(main.disposal_button.disabled and main.cabin_button.disabled)
	assert(is_instance_valid(main.ferry_toast))
	assert(sim.get_map_flights().size() == 1)
	var path := "user://ferry-ui-test.json"
	assert(sim.save_game(path).is_empty())
	assert(sim.load_game(path).is_empty())
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(quote["flight_seconds"]))
	sim.pause()
	main._refresh_fleet()
	assert(sim.get_fleet()[0]["location_icao"] == "EDDM")
	assert(sim.get_ferry_archive(0).size() == 1)
	assert(sim.get_ferry_archive(0)[0]["costs_eur"] == quote["cost_eur"])
	assert(sim.get_financial_summary()["ferry_eur"] == quote["cost_eur"])
	DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	main.queue_free()
	await process_frame
	print("PASS: ferry dialog range rejection, Rust quote, dispatch, chip, toast, map, restart, landing and archive")
	quit(0)
