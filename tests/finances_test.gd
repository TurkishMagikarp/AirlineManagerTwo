extends SceneTree
## Real UI + Rust archive, including a million flights and a fresh-process restore.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-finance-archive-test.json")
	var restore := false
	var capture := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--finance-save="):
			path = arg.trim_prefix("--finance-save=")
		if arg == "--restore-finance":
			restore = true
		if arg.begins_with("--capture-dir="):
			capture = arg.trim_prefix("--capture-dir=")
	if restore:
		if not _check(sim.load_game(path).is_empty(), "Load archive in fresh process"):
			return
	else:
		sim.found_airline("Ledger Air", "DE", "EDDF")
		for id in [1, 2]:
			sim.buy_aircraft("atr72_600")
			sim.create_route("EDDF", "EDDM", id)
			sim.set_ticket_price(id, 71 if id == 1 else 134)
			sim.hire_missing_staff()
			sim.schedule_route(id, 1, 120)
		sim.start()
		# Test-only endowment: the archive stress scenario intentionally runs a
		# loss-making route for centuries. Without funding it must now suspend.
		if not _check(sim.save_game(path).is_empty(), "Save funded archive fixture"):
			return
		# Keep Rust integer encodings intact (Godot's JSON parser uses doubles).
		var text := FileAccess.get_file_as_string(path)
		var fixture: Dictionary = JSON.parse_string(text)
		var balance := int(fixture["state"]["airline"]["capital_eur"])
		var opening := int(fixture["state"]["history"]["opening_balance"])
		var position := text.find('"capital_eur":', text.find('"name":"Ledger Air"'))
		text = text.left(position) + text.substr(position).replace('"capital_eur":%d' % balance, '"capital_eur":%d' % (balance + 100_000_000_000))
		position = text.rfind('"opening_balance":')
		text = text.left(position) + text.substr(position).replace('"opening_balance":%d' % opening, '"opening_balance":%d' % (opening + 100_000_000_000))
		var file := FileAccess.open(path, FileAccess.WRITE)
		file.store_string(text)
		file.close()
		var error: String = sim.load_game(path)
		if not _check(error.is_empty(), "Load funded archive fixture: " + error):
			return
		sim.advance(60.0 + 7200.0 * 1_000_000)
		sim.pause()
		if not _check(sim.save_game(path).is_empty(), "Save million-flight archive"):
			return
	var routes: Array = sim.get_route_finances()
	# ~228 years of shuttles, interrupted only by mandatory maintenance checks.
	var count: int = routes[0]["flights"]
	if not _check(routes.size() == 2 and count > 900_000 and routes[0]["profit_eur"] > 0 and routes[1]["profit_eur"] < 0, "Real profitable and loss-making routes"):
		return
	var checks := 0
	for plane in sim.get_fleet():
		checks += plane["checks"] + (1 if plane["maintenance_state"] != "none" else 0)
	var info: Dictionary = sim.get_finance_history_info()
	# Checks, weekly fuel, seasonal passenger rounding and competitor service
	# changes legitimately start blocks. The archive still compresses millions
	# of flights without discarding any departure quote.
	if not _check(checks > 2000 and info["batch_count"] >= checks and info["batch_count"] < 100_000, "Archive grows only with real interruptions: %d checks, %d blocks" % [checks, info["batch_count"]]):
		return
	var series: Array = sim.get_finance_series(0)
	if not _check(series.size() <= 64 and series[-1]["balance"] == sim.get_airline_info()["capital_eur"], "Bounded series reconciles to account"):
		return
	var revenue := 0
	var costs := 0
	for sample in series:
		revenue += sample["revenue"]
		costs += sample["costs"]
	if not _check(revenue == sim.get_financial_summary()["revenue_eur"] and costs == sim.get_financial_summary()["costs_eur"], "Chart periods reconcile to real totals"):
		return
	var started := Time.get_ticks_usec()
	for i in range(100):
		sim.get_route_flight_page(1, count - 50 if i % 2 == 0 else 0)
	print("Archive: 100 newest/oldest page queries across a million flights: %.2f ms" % ((Time.get_ticks_usec() - started) / 1000.0))
	main._sync_view()
	main.navigation[4].pressed.emit()
	var panel = main.finances_panel
	panel.set_process(false)
	panel.route_tree.get_root().get_first_child().select(0)
	panel._select_route()
	if not _check(panel.is_visible_in_tree() and panel.flight_count == count and panel.flights_tree.get_root().get_child_count() == 50, "Clicked route renders only 50 flights"):
		return
	panel.next_button.pressed.emit()
	if not _check(panel.offset == 50 and panel.flights_tree.get_root().get_first_child().get_text(0) == sim.get_route_flight_page(1, 50)[0]["landing"], "Older flight page from Rust"):
		return
	panel.offset = count - 50
	panel._flights()
	if not _check(panel.next_button.disabled and panel.flights_tree.get_root().get_child_count() == 50, "Oldest recorded flights remain accessible"):
		return
	var unchanged_root: TreeItem = panel.flights_tree.get_root()
	panel.refresh()
	if not _check(panel.flights_tree.get_root() == unchanged_root, "Unchanged financial UI is not rebuilt"):
		return
	var before: Dictionary = sim.get_financial_summary()
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		var scroll: ScrollContainer = panel.get_parent().get_parent()
		scroll.scroll_vertical = 0
		for frame in range(5):
			await process_frame
		if not _check(main.size.x <= dimensions.x and root.get_visible_rect().encloses(main.navigation[4].get_global_rect()) and root.get_visible_rect().encloses(main.new_game_button.get_global_rect()), "Finance navigation fits " + str(dimensions)):
			return
		if not capture.is_empty():
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(capture.path_join("finances-%dx%d.png" % [dimensions.x, dimensions.y]))
		scroll.ensure_control_visible(panel.next_button)
		for frame in range(3):
			await process_frame
		if not _check(root.get_visible_rect().encloses(panel.next_button.get_global_rect()), "Flight paging reachable"):
			return
		if not capture.is_empty():
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(capture.path_join("finance-flights-%dx%d.png" % [dimensions.x, dimensions.y]))
	if not _check(sim.get_financial_summary() == before, "Navigation does not book money"):
		return
	sim.start()
	sim.advance(7200.0)
	var continued: Array = sim.get_route_finances()
	if not _check(continued[0]["flights"] - count in [0, 1] and continued[0]["flights"] + continued[1]["flights"] == sim.get_financial_summary()["flights"], "Loaded archive continues exactly once (or waits in the hangar)"):
		return
	main.queue_free()
	await process_frame
	print("PASS: finance tab, ~two million flights with maintenance interruptions, compact archive, bounded charts/pages, layout, save/load and continuation")
	quit(0)

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
