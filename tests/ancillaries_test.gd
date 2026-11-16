extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Fee audit", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	sim.hire_missing_staff()
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.schedule_route(1, 1, 120).is_empty())
	assert(sim.get_ancillary_policy()["policy"] == "included")
	main.finances_panel.ancillary_buttons[1].pressed.emit()
	assert(sim.get_ancillary_policy()["policy"] == "standard")
	assert(sim.get_financial_summary()["ancillary_revenue_eur"] == 0)
	sim.start()
	sim.advance(60.0)
	assert(sim.set_ancillary_policy("aggressive").is_empty())
	sim.advance(86400.0)
	sim.pause()
	main._sync_view()
	main._show_page(4)
	var summary: Dictionary = sim.get_financial_summary()
	assert(summary["ancillary_revenue_eur"] > 0)
	assert(summary["revenue_eur"] == summary["ticket_revenue_eur"] + summary["cargo_revenue_eur"] + summary["ancillary_revenue_eur"])
	assert(main.finances_panel.metrics["ancillary"].text == main.finances_panel.UI.money(summary["ancillary_revenue_eur"]))
	assert(main.finances_panel.ancillary_buttons[2].button_pressed)
	var total := 0
	for period in sim.get_finance_series(0):
		total += period["ancillary"]
	assert(total == summary["ancillary_revenue_eur"])
	var routes: Array = sim.get_route_finances()
	assert(routes[0]["ancillary_revenue_eur"] == summary["ancillary_revenue_eur"])
	var before: Dictionary = sim.get_ancillary_policy()
	assert(not sim.set_ancillary_policy("invalid").is_empty())
	assert(sim.get_ancillary_policy() == before)
	main.queue_free()
	await process_frame
	print("PASS: airline-wide fee segments, Rust rates, frozen departure income and separate finance metric")
	quit(0)
