extends SceneTree
## Real DLL + route panel; run again with --restore-competition in a fresh process.

func _initialize() -> void:
	call_deferred("_run")


func _snapshot(sim: RefCounted) -> String:
	return JSON.stringify([sim.get_elapsed_seconds(), sim.get_airline_info(), sim.get_financial_summary(), sim.get_flight_results(), sim.get_routes(), sim.get_route_competition(1)])


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-competition-test.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--competition-save="):
			path = arg.trim_prefix("--competition-save=")
	if "--restore-competition" in OS.get_cmdline_user_args():
		if not _check(sim.load_game(path).is_empty(), "Load all airlines in fresh process"):
			return
		if not _check(_snapshot(sim) == FileAccess.get_file_as_string(path + ".expected"), "All offers, money, flights and exact time restored"):
			return
	else:
		if not _check(sim.found_airline("Competition Air", "DE", "EDDF").is_empty(), "Found player and competitors"):
			return
		sim.buy_aircraft("atr72_600")
		sim.create_route("EDDF", "EDDM", 1)
		sim.set_ticket_price(1, 90)
		sim.schedule_route(1, 1, 120)
		var equal: Array = sim.get_route_competition(1)
		if not _check(equal.size() == 2 and equal[1]["airline"] == "Regional Wings" and equal[1]["active"], "Real rival offer on player's route"):
			return
		sim.start()
		sim.set_speed(16)
		sim.advance(100.25 / 16.0)
		var frozen: Array = sim.get_active_flights()
		if not _check(sim.set_ticket_price(1, 50).is_empty() and sim.get_active_flights() == frozen, "Active price change preserves departure snapshot"):
			return
		var cheap: Array = sim.get_route_competition(1)
		if not _check(cheap[0]["passengers_per_day"] > equal[0]["passengers_per_day"] and cheap[1]["passengers_per_day"] < equal[1]["passengers_per_day"], "Cheap fare wins passengers from rival"):
			return
		if not _check(is_equal_approx(cheap[0]["share_percent"] + cheap[1]["share_percent"], 100.0), "Passenger shares reconcile"):
			return
		sim.pause()
		if not _check(sim.save_game(path).is_empty(), "Save in-flight multi-airline game"):
			return
		var expected := FileAccess.open(path + ".expected", FileAccess.WRITE)
		expected.store_string(_snapshot(sim))
		expected.close()
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel
	panel.refresh()
	panel.route_tree.get_root().get_child(0).select(0)
	panel.route_tree.item_selected.emit()
	if not _check("Regional Wings" in panel.competition_label.text and "Competition Air" in panel.competition_label.text and "Pax/Tag" in panel.competition_label.text, "Route UI shows real competitor and estimated split"):
		return
	var slow: RefCounted = ClassDB.instantiate("SimulationClock")
	if not _check(slow.load_game(path).is_empty(), "Load comparison simulation"):
		return
	sim.start()
	slow.start()
	if not _check(sim.advance(21.0 * 86400.0 / 16.0), "Fast forward across weekly decisions"):
		return
	for i in range(504):
		if not _check(slow.advance(3600.0 / 16.0), "Hourly comparison step"):
			return
	if not _check(_snapshot(sim) == _snapshot(slow), "Fast forward and hourly steps match after AI decisions"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	var scroll := panel.get_parent().get_parent() as ScrollContainer
	scroll.ensure_control_visible(panel.competition_label)
	await process_frame
	if not _check(main.size.x <= 680 and panel.competition_label.size.x <= 680, "Competition panel wraps in small window"):
		return
	main.queue_free()
	await process_frame
	print("PASS: multi-airline market, price competition, immutable flights, route UI, save/restart and fast forward")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
