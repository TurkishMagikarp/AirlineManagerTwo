extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	TranslationServer.set_locale("de")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Transfer test", "DE", "EDDF").is_empty())
	for id in [1, 2]:
		assert(sim.lease_aircraft("a320_200").is_empty())
		assert(sim.create_route("EDDF", "EDDM" if id == 1 else "LTFM", id).is_empty())
		assert(sim.set_ticket_price(id, 90 if id == 1 else 120).is_empty())
		sim.hire_missing_staff()
		assert(sim.schedule_route(id, 1, 240).is_empty())
	var route: Dictionary = sim.get_routes()[0]
	assert(route["expected_daily_connecting"] > 0)
	assert(route["connections"].size() > 0)
	assert(route["expected_daily_local"] + route["expected_daily_connecting"] == route["expected_daily_passengers"])
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel
	panel.route_tree.get_root().get_first_child().select(0)
	panel._selection_changed()
	assert(panel.connections_label.visible and not panel.connections_empty.visible)
	sim.start()
	assert(sim.advance(86400.0))
	sim.pause()
	var totals: Dictionary = sim.get_financial_summary()
	var connecting := 0
	for result in sim.get_flight_results():
		assert(result["passengers"] == result["local_passengers"] + result["connecting_passengers"])
		assert(result["connecting_revenue_eur"] <= result["ticket_revenue_eur"])
		connecting += result["connecting_passengers"]
	assert(connecting > 0)
	assert(totals["connecting_passengers"] == connecting)
	assert(sim.stop_schedule(2).is_empty())
	panel.refresh()
	assert(panel.connections_empty.visible and not panel.connections_chip.visible)
	main.queue_free()
	await process_frame
	print("PASS: Rust transfer API, route counts, feeding paths, frozen results and finance totals")
	quit(0)
