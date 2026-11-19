extends SceneTree
## Presentation fields and decisions must never mutate the simulation on preview/cancel.

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Kit Regression", "DE", "EDDF").is_empty())
	var expected := {
		"a320_200": ["Narrowbody", 830, 4, 6], "a320neo": ["Narrowbody", 830, 4, 6],
		"b737_800": ["Narrowbody", 840, 4, 7], "b737_max8": ["Narrowbody", 840, 4, 7],
		"atr72_600": ["Turboprop", 510, 4, 3], "atr72_evo": ["Turboprop", 510, 4, 3],
		"a330_300": ["Widebody", 871, 6, 12], "a330_900": ["Widebody", 871, 6, 12],
		"a350_900": ["Widebody", 903, 6, 13], "b777_300er": ["Widebody", 905, 8, 16],
		"atr72_600f": ["Turboprop", 510, 4, 0], "b737_800bcf": ["Narrowbody", 840, 4, 0],
		"b777f": ["Widebody", 905, 8, 0],
	}
	for model in sim.get_aircraft_models():
		var actual := [model["class"], model["cruise_kmh"], model["pilots"], model["cabin_crew"]]
		if actual != expected.get(model["id"], []):
			push_error("Unexpected catalog fields for %s: %s" % [model["id"], actual])
			quit(1)
			return
	assert(sim.buy_aircraft("atr72_600").is_empty())
	main._sync_view()
	main._show_page(1)
	assert(main.fleet_sections[0].is_visible_in_tree())
	assert(not main.fleet_sections[1].is_visible_in_tree())
	var before: Array = sim.get_fleet().duplicate(true)
	var cash: int = sim.get_airline_info()["capital_eur"]
	main._confirm_disposal()
	await process_frame
	var dialog = root.get_child(root.get_child_count() - 1).get_child(0)
	assert(dialog.get_cancel_button().has_focus())
	dialog.get_cancel_button().pressed.emit()
	await process_frame
	assert(sim.get_fleet() == before and sim.get_airline_info()["capital_eur"] == cash)
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		main._show_page(0)
		for frame in range(5): await process_frame
		assert(main.metric_grid.columns == (4 if dimensions.x == 1440 else 2))
		assert(main.overview_register.get_root().get_child_count() == 1)
		assert(main.overview_operations.get_parent().get_parent().get_parent().get_parent().columns == (1 if dimensions.x < 1100 else 2))
		main._show_page(1)
		for i in range(3):
			main.fleet_tabs.current_tab = i
			await process_frame
			assert(main.fleet_sections[i].is_visible_in_tree())
	assert(sim.get_fleet() == before and sim.get_airline_info()["capital_eur"] == cash)
	main._confirm_disposal()
	await process_frame
	dialog = root.get_child(root.get_child_count() - 1).get_child(0)
	dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_fleet().is_empty())
	assert(sim.get_airline_info()["capital_eur"] > cash)
	main.queue_free()
	await process_frame
	print("PASS: catalog display fields, responsive kit composition, tab navigation and cancel/confirm sale")
	quit(0)
