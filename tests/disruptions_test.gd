extends "res://tests/pilot_ratings_test.gd"
## Test inputs/timeline are pinned by disruptions_godot_fixture_* in Rust.
## Only Rust advances time or computes delays, quotes, care and punctuality.
const SEED := 33364
var disruption_path := ""

func _run() -> void:
	var mode := "ui"
	disruption_path = ProjectSettings.globalize_path("user://disruptions.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--disruption-mode="):
			mode = arg.trim_prefix("--disruption-mode=")
		if arg.begins_with("--disruption-save="):
			disruption_path = arg.trim_prefix("--disruption-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v25", "read-v25", "write-v26", "read-v26"], "Explicit disruption mode"):
		return
	probe_path = disruption_path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	if mode == "ui":
		if not _setup():
			return
		initial_world = _snapshot()[0]
		for speed in [1, 4, 16]:
			if not await _timeline(speed):
				return
		if not await _disruption_layouts():
			return
	elif not await _restart_family(mode):
		return
	print("PASS: disruptions " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _setup() -> bool:
	var sim = main.simulation
	for error in [sim.found_airline("Disruption Audit", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.hire_missing_staff(), sim.schedule_route(1, 1, 90)]:
		if not _check(error.is_empty(), "Create disruption fixture: " + error):
			return false
	# Change only the player cursor and its matching aircraft baseline. Real
	# production config, normal AI owners and all accounting bytes are retained.
	var world: String = _snapshot()[0]
	var player := _json_value(world, '"state":')
	var competitors := _json_value(player, '"competitors":')
	player = _replace_value(player, '"competitors":', "[]")
	var routes := _json_value(player, '"routes":')
	var plan := _json_value(routes, '"plan":')
	var cursor := _json_value(plan, '"disruption":')
	cursor = RegEx.create_from_string('"seed":\\d+').sub(cursor, '"seed":%d' % SEED)
	cursor = cursor.replace('"cycles":0', '"cycles":570').replace('"check_cycles":0', '"check_cycles":570')
	plan = _replace_value(plan, '"disruption":', cursor)
	player = _replace_value(player, '"routes":', _replace_value(routes, '"plan":', plan))
	var fleet := _json_value(player, '"fleet":')
	fleet = fleet.replace('"cycles":0', '"cycles":570').replace('"check_cycles":0', '"check_cycles":570')
	player = _replace_value(player, '"fleet":', fleet)
	player = _replace_value(player, '"competitors":', competitors)
	_write(disruption_path, _replace_value(world, '"state":', player))
	return _restore_disruption(disruption_path)

func _restore_disruption(path: String) -> bool:
	var error: String = main.simulation.load_game(path)
	if not _check(error.is_empty(), "Load fixture: " + error):
		return false
	main._sync_view()
	return true

func _frame_to(second: int, speed: int) -> void:
	# Controlled boundary frame through the production main-loop callback,
	# followed by actual engine frames. There is no test-side simulation clock.
	main.simulation.set_speed(speed)
	main.simulation.start()
	main._process((second - main.simulation.get_elapsed_seconds()) / speed)
	main.simulation.pause()
	await process_frame

func _timeline(speed: int) -> bool:
	_write(disruption_path, initial_world)
	if not _restore_disruption(disruption_path):
		return false
	await _click(main.speed_buttons[[1, 4, 16].find(speed)])
	await _click(main.start_button)
	main.set_process(true)
	for frame in range(4):
		await process_frame
	main.set_process(false)
	await _click(main.pause_button)
	if not _check(main.simulation.get_speed() == speed and main.simulation.get_elapsed_seconds() > 0 and main.simulation.get_elapsed_seconds() < 60, "Real automatic frames at %dx stay before scheduled departure" % speed):
		return false
	await _frame_to(59, speed)
	if not await _delay_views(0) or not await _punctuality_view("—", 0):
		return false
	await _frame_to(60, speed)
	if not await _delay_views(2932) or not await _punctuality_view("—", 0):
		return false
	await _frame_to(2993, speed)
	var quote: Dictionary = main.simulation.get_active_flights()[0]
	if not _check(quote["care_eur"] > 0 and main.simulation.get_financial_summary()["care_eur"] == 0, "Airborne care quote is frozen and unsettled"):
		return false
	var now: int = main.simulation.get_elapsed_seconds()
	var landing: int = _routes(_state())[0]["plan"]["airborne"]["landing"]
	await _frame_to(landing, speed)
	if not await _punctuality_view("0.0 %", 1) or not await _care_views(quote["care_eur"]):
		return false
	var before := _snapshot()
	for frame in range(5):
		main._process(0.0)
		await process_frame
	if not _check(_snapshot() == before, "Refresh frames do not rebook care"):
		return false
	if not _check(main.simulation.save_game(disruption_path + ".landed").is_empty(), "Save settled care") or not _restore_disruption(disruption_path + ".landed") or not _check(_snapshot() == before, "Reload preserves complete world and care APIs"):
		return false
	# Continue to the third leg: its own technical roll is zero, so its entire
	# delay is inherited from the first two legs. Rust pins the exact deadlines.
	await _frame_to(12000, speed)
	if not _check(_has_delay_attention(), "Large propagated backlog enters attention"):
		return false
	main.attention_expanded = true
	main._refresh_attention()
	var entries: Array = main.simulation.get_attention_snapshot()
	for i in range(entries.size()):
		if entries[i]["kind"] == "delay" and not _check(main.attention_rows.get_child(i).text == entries[i]["text"], "Backlog row matches Rust text"):
			return false
	await _frame_to(13525, speed)
	if not await _delay_views(2664):
		return false
	await _frame_to(59461, speed)
	if not _check(not _has_delay_attention() and main.simulation.get_fleet()[0]["delay_seconds"] == 0, "Spare schedule time clears backlog and attention"):
		return false
	# Several late and on-time landings give a nontrivial percentage, calculated
	# from persisted counters rather than copying the percentage under test.
	var totals: Dictionary = _state()["finance"]["totals"]
	var expected := "%.1f %%" % ((totals["otp"] - totals.get("late", 0)) * 100.0 / totals["otp"])
	return _check(now < landing, "Mid-delay snapshot really was airborne") and await _punctuality_view(expected, totals["otp"])

func _delay_views(delay: int) -> bool:
	await _click(main.navigation[1])
	var plane: Dictionary = main.simulation.get_fleet()[0]
	var row: TreeItem = main.fleet_tree.get_root().get_first_child()
	if not _check(plane["delay_seconds"] == delay and ("%d min verspätet" % ceili(delay / 60.0)) in row.get_text(3), "Fleet delay appears at scheduled departure"):
		return false
	await _click(main.navigation[2])
	main.routes_panel.refresh_operations()
	var operation: Dictionary = main.simulation.get_operations()[0]
	return _check(("%d min Verspätung" % ceili(delay / 60.0)) in operation["status_text"] and operation["status_text"] in main.routes_panel.operations_label.text, "Operations renders exact current Rust delay")

func _punctuality_view(expected: String, count: int) -> bool:
	await _click(main.navigation[1])
	var summary: Dictionary = main.simulation.get_financial_summary()
	if not _check(("Pünktlichkeitsruf: %+.1f %% (%s)" % [summary["reputation_percent"], summary["reputation_trend"]]) in main.description.text, "Rust reputation and trend rendered next to punctuality"):
		return false
	if not _check(("Pünktlichkeit: %s (%d Flüge)" % [expected, count]) in main.description.text, "Airline punctuality text and denominator"):
		return false
	await _click(main.navigation[2])
	main.routes_panel.refresh_operations()
	var row: TreeItem = main.routes_panel.route_tree.get_root().get_first_child()
	return _check(row.get_tooltip_text(0) == "Pünktlichkeit: %s · %d erfasste Flüge" % [expected, count] and ("Route #1 · Pünktlichkeit: %s · %d erfasste Flüge" % [expected, count]) in main.routes_panel.operations_label.text, "Route tooltip and operations punctuality")

func _care_views(care: int) -> bool:
	await _click(main.navigation[4])
	var panel = main.finances_panel
	panel.refresh(true)
	var row: TreeItem = panel.route_tree.get_root().get_first_child()
	if not _check(row.get_tooltip_text(3) == "Davon Betreuung / Entschädigung: %s" % main.UI.money(care) and main.simulation.get_route_finances()[0]["care_eur"] == care and main.simulation.get_financial_summary()["care_eur"] == care, "Route/company care is booked once"):
		return false
	await _reveal(panel.route_tree)
	await _mouse(panel.route_tree.global_position + panel.route_tree.get_item_area_rect(row, 0).get_center())
	var flight: TreeItem = panel.flights_tree.get_root().get_first_child()
	var expected := "Betreuung / Entschädigung: %s" % main.UI.money(care)
	return _check(flight != null and flight.get_tooltip_text(6).count(expected) == 1 and main.simulation.get_route_flight_page(1, 0)[0]["care_eur"] == care, "Flight cost breakdown includes care exactly once")

func _has_delay_attention() -> bool:
	for entry in main.simulation.get_attention_snapshot():
		if entry["kind"] == "delay":
			return true
	return false

func _disruption_layouts() -> bool:
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		_write(disruption_path, initial_world)
		if not _restore_disruption(disruption_path):
			return false
		await _frame_to(60, 1)
		if not await _delay_views(2932) or not await _punctuality_view("—", 0):
			return false
		for page in [1, 2]:
			await _click(main.navigation[page])
			await _reveal(main.description if page == 1 else main.routes_panel.operations_label)
			await _capture("unobserved-%d" % page, dimensions)
		await _frame_to(12000, 1)
		main.attention_expanded = true
		main._refresh_attention()
		for page in [1, 2, 4]:
			await _click(main.navigation[page])
			var panel: Control = main.fleet_tree if page == 1 else main.routes_panel.operations_label if page == 2 else main.finances_panel.route_tree
			await _reveal(panel)
			if not _check(main.size.x <= dimensions.x and panel.is_visible_in_tree() and panel.get_global_rect().intersects(root.get_visible_rect()), "Disruption panel reachable at %s" % dimensions):
				return false
			if page == 1:
				await _cell_capture(main.fleet_tree, 3, "page-1", dimensions)
			else:
				await _capture("page-%d" % page, dimensions)
		await _reveal(main.attention_button)
		await _capture("attention", dimensions)
		_write(disruption_path, initial_world)
		if not _restore_disruption(disruption_path):
			return false
		await _frame_to(2993, 1)
		var care: int = main.simulation.get_active_flights()[0]["care_eur"]
		await _frame_to(5104, 1)
		if not await _care_views(care):
			return false
		await _cell_capture(main.finances_panel.route_tree, 3, "route-care", dimensions)
		await _cell_capture(main.finances_panel.flights_tree, 0, "flight-care", dimensions)
		await _frame_to(59461, 1)
		await _click(main.navigation[1])
		await _reveal(main.description)
		await _capture("recovered-punctuality", dimensions)
	return true

func _cell_capture(tree: Tree, column: int, label: String, dimensions: Vector2i) -> void:
	await _reveal(tree)
	if not capture_dir.is_empty():
		var row := tree.get_root().get_first_child()
		# Scroll the real table to the cell before hovering its native tooltip.
		for child in tree.get_children(true):
			if child is HScrollBar:
				child.value += tree.get_item_area_rect(row, column).get_center().x - tree.size.x / 2.0
		await process_frame
		var cell := tree.get_item_area_rect(row, column)
		var motion := InputEventMouseMotion.new()
		motion.position = tree.global_position + cell.get_center()
		Input.parse_input_event(motion)
		await create_timer(0.8).timeout
		await _capture(label, dimensions)
		motion = InputEventMouseMotion.new()
		motion.position = Vector2(1, 1)
		Input.parse_input_event(motion)
		await process_frame

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("disruptions-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Original Godot viewport render")

func _restart_family(mode: String) -> bool:
	var sim = main.simulation
	if mode.begins_with("write-"):
		if not _setup():
			return false
		if mode.ends_with("v25"):
			# A legacy flight cannot already have technical delay/care. Preserve
			# its frozen quote and all other bytes; new rolls begin after landing.
			var neutral: String = _snapshot()[0]
			var player := _json_value(neutral, '"state":')
			var competitors := _json_value(player, '"competitors":')
			player = _replace_value(player, '"competitors":', "[]")
			var routes := _json_value(player, '"routes":')
			var cursor := _json_value(routes, '"disruption":')
			cursor = RegEx.create_from_string('"seed":\\d+').sub(cursor, '"seed":0')
			player = _replace_value(player, '"routes":', _replace_value(routes, '"disruption":', cursor))
			player = _replace_value(player, '"competitors":', competitors)
			_write(disruption_path, _replace_value(neutral, '"state":', player))
			if not _restore_disruption(disruption_path):
				return false
			_advance(120)
			var old := _without_disruptions(_snapshot()[0]).replace('"version":30', '"version":25')
			_write(disruption_path, old)
			if not _restore_disruption(disruption_path) or not _check(_without_disruptions(_snapshot()[0]) == old.replace('"version":25', '"version":30'), "v25 migration changes only disruption defaults for all owners"):
				return false
		else:
			_advance(2993)
			if not _check(sim.get_active_flights()[0]["care_eur"] > 0 and sim.get_fleet()[0]["delay_seconds"] == 2932 and sim.save_game(disruption_path).is_empty(), "v26 save includes delayed airborne aircraft and care quote"):
				return false
		_store(disruption_path + ".initial", _snapshot())
	else:
		main.save_path = disruption_path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(disruption_path + ".initial"), "Fresh process matches full world and API immediately"):
			return false
	var before := _state()
	if not _check(before["competitors"].size() == 2 and _routes(before)[0]["plan"]["airborne"] != null, "Restart family retains AI owners and an airborne quote"):
		return false
	if mode.ends_with("v25"):
		for owner in _owners(before):
			if not _check(owner["finance"]["totals"].get("otp", 0) == 0 and owner["finance"]["totals"].get("care", 0) == 0, "Legacy owners have no invented punctuality or care history"):
				return false
	var frozen: Dictionary = _routes(before)[0]["plan"]["airborne"]
	var now: int = before["clock"]["elapsed_seconds"]
	_advance(frozen["landing"] - now)
	if not _check(_state()["finance"]["recent"][0] == frozen, "First landing preserves every frozen departure total"):
		return false
	var settled := _snapshot()
	for frame in range(4):
		main._process(0.0)
		await process_frame
	if not _check(_snapshot() == settled, "Restart landing care is not rebooked on frames"):
		return false
	_advance(now + 100 * 86400 - sim.get_elapsed_seconds())
	if mode.begins_with("write-"):
		_store(disruption_path + ".expected", _snapshot())
	else:
		if not _check(_snapshot() == _read(disruption_path + ".expected"), "100-day full player/AI world and APIs match uninterrupted writer"):
			return false
	return true

func _without_disruptions(world: String) -> String:
	for name in ["reputation", "disruption", "otp", "late", "delay_seconds", "care"]:
		var key := '"%s":' % name
		while world.contains(key):
			var start := world.find(key)
			var value_start := start + key.length()
			var end := value_start
			if world[value_start] == '{':
				end += _json_value(world.substr(start), key).length()
			else:
				while end < world.length() and world[end] not in [',', '}']:
					end += 1
			if start > 0 and world[start - 1] == ',':
				start -= 1
			elif end < world.length() and world[end] == ',':
				end += 1
			world = world.left(start) + world.substr(end)
	return world

func _snapshot() -> Array:
	return super._snapshot() + [main.simulation.get_attention_snapshot(), main.simulation.get_flight_results()]
