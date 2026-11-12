extends SceneTree
## Real Rust extension, route UI and isolated save; supports a fresh-process restore.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-demand-test.json")
	var restore := "--restore-demand" in OS.get_cmdline_user_args()
	if restore:
		if not _check(sim.load_game(path).is_empty(), "Restore shared market in fresh process"):
			return
	else:
		sim.found_airline("Shared Demand", "DE", "EDDF")
		for id in [1, 2]:
			sim.buy_aircraft("atr72_600")
			sim.create_route("EDDF", "EDDM", id)
			sim.set_ticket_price(id, 100)
			sim.schedule_route(id, 1, 120)
		var routes: Array = sim.get_routes()
		if not _check(routes[0]["daily_demand"] == routes[1]["daily_demand"] and routes[0]["estimated_passengers"] == routes[1]["estimated_passengers"], "Same daily market, shared equally"):
			return
		sim.set_speed(16)
		sim.start()
		sim.advance(86400.0 / 16.0)
		sim.pause()
		var totals: Dictionary = sim.get_financial_summary()
		if not _check(totals["flights"] == 24 and totals["passengers"] <= 2 * routes[0]["daily_demand"], "Twenty-four flights do not multiply daily demand"):
			return
		if not _check(sim.save_game(path).is_empty(), "Save shared market and actual results"):
			return
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel
	panel.refresh()
	var route: Dictionary = sim.get_routes()[0]
	var row: TreeItem = panel.route_tree.get_root().get_child(0)
	if not _check(row.get_text(4) == "%d Pax" % route["daily_demand"] and row.get_text(5) == "%.1f %%" % route["actual_load_percent"] and route["actual_flights"] == 12, "Route UI shows Rust daily estimate and real weighted load"):
		return
	var historical: Array = sim.get_flight_results()
	if not _check(sim.stop_schedule(1).is_empty(), "Stop parked plan"):
		return
	panel.refresh()
	row = panel.route_tree.get_root().get_child(0)
	row.select(0)
	panel.route_tree.item_selected.emit()
	panel.ticket_input.value = 100000
	panel.ticket_button.pressed.emit()
	if not _check(sim.get_routes()[0]["daily_demand"] == 0 and sim.get_flight_results() == historical, "Very high UI ticket price removes future demand, not past passengers"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	if not _check(panel.route_tree.columns == 6 and main.size.x <= 680, "Demand table keeps small-window layout"):
		return
	sim.schedule_route(1, 1, 120)
	sim.start()
	sim.advance(7200.0 / 16.0)
	if not _check(sim.get_financial_summary()["flights"] == 26 and sim.get_routes()[0]["daily_demand"] == 0, "Restart continuation settles each flight once"):
		return
	main.queue_free()
	await process_frame
	print("PASS: shared daily market, actual route load, high prices, UI, save/restart and continuation")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
