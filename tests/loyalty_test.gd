extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Reward audit", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	sim.hire_missing_staff()
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.schedule_route(1, 1, 120).is_empty())
	assert(sim.get_loyalty()["level"] == "none")
	main.finances_panel.loyalty_buttons[1].pressed.emit()
	assert(sim.get_loyalty()["level"] == "basic")
	assert(sim.get_financial_summary()["loyalty_cost_eur"] == 0)
	sim.start()
	sim.advance(60.0)
	assert(sim.set_loyalty_level("none").is_empty())
	sim.advance(86400.0)
	sim.pause()
	main._sync_view()
	main._show_page(4)
	main.finances_panel.refresh(true)
	var summary: Dictionary = sim.get_financial_summary()
	assert(summary["loyalty_cost_eur"] > 0)
	assert(sim.get_loyalty()["members"] > 0)
	assert(sim.get_loyalty()["bonus_percent"] > 0)
	assert(main.finances_panel.loyalty_metrics["cost"].text == main.finances_panel.UI.money(summary["loyalty_cost_eur"]))
	assert(main.finances_panel.loyalty_buttons[0].button_pressed)
	var total := 0
	for period in sim.get_finance_series(0):
		total += period["loyalty_cost"]
	assert(total == summary["loyalty_cost_eur"])
	var routes: Array = sim.get_route_finances()
	assert(routes[0]["loyalty_cost_eur"] == summary["loyalty_cost_eur"])
	main.finances_panel.route_tree.get_root().get_first_child().select(0)
	main.finances_panel._select_route()
	assert(main.finances_panel.detail.text.contains(main.finances_panel.UI.money(summary["loyalty_cost_eur"])))
	var before: Dictionary = sim.get_loyalty()
	assert(not sim.set_loyalty_level("invalid").is_empty())
	assert(sim.get_loyalty() == before)
	main.queue_free()
	await process_frame
	print("PASS: loyalty segments, Rust rates/members/bonus, frozen costs and finance history/route report")
	quit(0)
