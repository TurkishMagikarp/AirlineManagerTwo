extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	TranslationServer.set_locale("en")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Fare test", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel
	panel.route_tree.get_root().get_first_child().select(0)
	panel._selection_changed()
	assert(panel.fare_cached.is_empty())
	assert(panel.fare_apply.disabled)
	assert(not panel.fare_switch.button_pressed)
	sim.hire_missing_staff()
	assert(sim.schedule_route(1, 1, 120).is_empty())
	panel.refresh()
	assert(not panel.fare_cached.is_empty())
	assert(not panel.fare_apply.disabled)
	var snapshot: Dictionary = panel.fare_cached.duplicate(true)
	panel.refresh_operations()
	assert(snapshot == panel.fare_cached)
	panel.fare_apply.pressed.emit()
	assert(sim.get_routes()[0]["ticket_eur"] == snapshot["fare"])
	panel.fare_switch.button_pressed = true
	assert(sim.get_routes()[0]["pricing_autopilot"])
	assert(panel.route_tree.get_root().get_first_child().get_text(6) == "On")
	assert(sim.set_ticket_price(1, 137).is_empty())
	panel.refresh_operations()
	assert(panel.fare_key[1] == 137)
	panel.fare_switch.button_pressed = false
	assert(not sim.get_routes()[0]["pricing_autopilot"])
	main.queue_free()
	await process_frame
	print("PASS: fare advisor empty state, cached refresh, apply, manual fare and autopilot switch/chip")
	quit(0)
