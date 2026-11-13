extends SceneTree
## Real Routes selector and Rust-backed costs. Run: Godot --headless --path . --script res://tests/service_test.gd
var main

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var panel = main.routes_panel
	assert(panel.service_select.disabled)
	assert(sim.found_airline("Service UI", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("a320_200").is_empty())
	assert(sim.set_cabin_layout(1, 12).is_empty())
	assert(sim.create_route("EDDF", "LTFM", 1).is_empty())
	assert(sim.hire_missing_staff().is_empty())
	main._sync_view()
	main.navigation[2].pressed.emit()
	panel.route_tree.get_root().get_first_child().select(0)
	panel._selection_changed()
	assert(not panel.service_select.disabled and panel.service_select.selected == 1)
	assert(panel.service_select.item_count == 3)
	assert(sim.schedule_route(1, 1, 360).is_empty())
	for i in [2, 0, 1, 2]:
		panel.service_select.select(i)
		panel.service_select.item_selected.emit(i)
		var route: Dictionary = sim.get_routes()[0]
		assert(route["service_level"] == ["basic", "standard", "premium"][i])
		assert(panel.service_select.selected == i)
		assert(panel.UI.money(route["estimated_service_eur"]) in panel.service_cost.text)
		assert(route["estimated_service_eur"] > 0 if i == 2 else route["estimated_service_eur"] < 0 if i == 0 else route["estimated_service_eur"] == 0)
	sim.start()
	assert(sim.advance(60.0))
	sim.pause()
	var frozen: Array = sim.get_active_flights()
	panel.service_select.select(0)
	panel.service_select.item_selected.emit(0)
	assert(sim.get_active_flights() == frozen)
	assert(sim.get_routes()[0]["service_level"] == "basic")
	sim.start()
	assert(sim.advance(86400.0))
	sim.pause()
	panel.refresh_operations()
	assert(panel.UI.money(sim.get_routes()[0]["actual_service_eur"]) in panel.service_cost.text)
	main.finances_panel.refresh(true)
	assert("Bordservice" in main.finances_panel.note.text)
	main.queue_free()
	await process_frame
	print("PASS: Service selector, per-route costs, live refresh and frozen departure results")
	quit(0)
