extends SceneTree
## Real cargo controls; restart writers and readers must run in separate processes.
const InsuranceFixture = preload("res://tests/insurance_fixture.gd")
var main
var probe_path := ""
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/cargo-ui.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--cargo-mode="):
			mode = arg.trim_prefix("--cargo-mode=")
		if arg.begins_with("--cargo-save="):
			path = arg.trim_prefix("--cargo-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v21", "read-v21", "write-v22", "read-v22"], "Explicit mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var legacy := mode.ends_with("v21")
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(path + ".initial"), "Fresh process restores full world and API byte-identically"):
			return
	else:
		if not _check(sim.found_airline("Cargo UI", "DE", "EDDF").is_empty() and sim.buy_aircraft("a320_200").is_empty() and sim.create_route("EDDF", "LTFM", 1).is_empty() and sim.set_ticket_price(1, 210).is_empty() and sim.hire_missing_staff().is_empty() and sim.schedule_route(1, 1, 360).is_empty(), "Set up player service with normal AI competitors"):
			return
		main._sync_view()
		if mode == "ui":
			if not await _controls():
				return
		else:
			if legacy:
				# Zero-demand prices for every owner create a genuinely cargo-free
				# old world before stripping fields. Do not round-trip JSON doubles.
				var initial: String = _snapshot()[0]
				_write(path, initial.replace('"cargo_rate_cents":150', '"cargo_rate_cents":10000'))
				if not _check(sim.load_game(path).is_empty(), "Load neutral legacy fixture"):
					return
			_advance(21 * 86400 + 120)
			if not _check(sim.save_game(path).is_empty(), "Save completed cargo results and airborne flight"):
				return
			if legacy:
				var original := FileAccess.get_file_as_string(path)
				var fixture := _strip(original).replace('"version":27', '"version":21')
				if not _check(JSON.parse_string(fixture)["version"] == 21 and not '"cargo_' in fixture, "True v21 fixture omits every cargo field"):
					return
				_write(path, fixture)
				if not _check(sim.load_game(path).is_empty() and InsuranceFixture.without_policy(_snapshot()[0]) == InsuranceFixture.without_policy(original.replace('"cargo_rate_cents":10000', '"cargo_rate_cents":150')), "Migration adds only default rates and zero cargo; money, history and quotes unchanged"):
					return
			_store(path + ".initial", _snapshot())
	if not _owners(legacy, mode != "ui") or not await _views():
		return
	if mode != "ui":
		_advance(100 * 86400 + 123)
		if not await _views():
			return
		if mode.begins_with("write-"):
			_store(path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(path + ".expected"), "Long continuation equals uninterrupted player/AI world, history, charts and quotes"):
			return
		if not _check(sim.save_game(probe_path).is_empty() and JSON.parse_string(FileAccess.get_file_as_string(probe_path))["version"] == 27, "Writes current v27"):
			return
	if not await _layouts():
		return
	print("PASS: cargo " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _controls() -> bool:
	await _click(main.navigation[2])
	var panel = main.routes_panel
	await _select(panel.route_tree)
	var maximum: int = main.simulation.get_routes()[0]["cargo_max_rate_cents"]
	for entry in [["-1", 0], ["999999", maximum], ["2.10", 210]]:
		if not await _rate(entry[0], entry[1]):
			return false
	_advance(60)
	var airborne: Dictionary = _state()["routes"]["routes"][0]["plan"]["airborne"].duplicate(true)
	if not _check(airborne["totals"]["cargo_kg"] > 0 and airborne["totals"]["cargo_revenue"] > 0, "Departure carries paid cargo"):
		return false
	if not await _rate(str(maximum / 100.0), maximum):
		return false
	if not _check(_state()["routes"]["routes"][0]["plan"]["airborne"] == airborne, "Changing rate airborne preserves full frozen quote"):
		return false
	_advance(float(airborne["landing"]) - 60)
	var landed: Dictionary = main.simulation.get_route_flight_page(1, 0)[0]
	if not _check(landed["cargo_kg"] == airborne["totals"]["cargo_kg"] and landed["cargo_revenue_eur"] == airborne["totals"]["cargo_revenue"], "First landing books frozen kg and revenue"):
		return false
	_advance(21600)
	landed = main.simulation.get_route_flight_page(1, 0)[0]
	return _check(landed["cargo_kg"] == 0 and landed["cargo_revenue_eur"] == 0 and main.simulation.get_route_finances()[0]["flights"] == 2, "Next landing uses new zero-demand maximum rate")

func _rate(text: String, cents: int) -> bool:
	var panel = main.routes_panel
	await _reveal(panel.cargo_input)
	var edit: LineEdit = panel.cargo_input.get_line_edit()
	await _mouse(edit.get_global_rect().get_center())
	edit.select_all()
	# Text events pass through the real focused LineEdit, followed by keyboard
	# submission and a real mouse click on the apply control.
	for character in text:
		var event := InputEventKey.new()
		event.unicode = character.unicode_at(0)
		event.pressed = true
		root.push_input(event)
		await process_frame
	await _key(KEY_ENTER, root)
	await _click(panel.cargo_button)
	return _check(main.simulation.get_routes()[0]["cargo_rate_cents"] == cents and panel.cargo_input.value == cents / 100.0 and panel.status.text.begins_with("Frachtpreis geändert."), "Keyboard edit/clamp and mouse apply: %s => rate=%s input=%s status=%s edit=%s" % [text, main.simulation.get_routes()[0]["cargo_rate_cents"], panel.cargo_input.value, panel.status.text, edit.text])

func _owners(legacy: bool, require_ai: bool) -> bool:
	var state := _state()
	var owners: Array = [state]
	for company in state["competitors"]:
		owners.append(company["game"])
	if not _check(owners.size() == 3, "Player and both AI owners covered"):
		return false
	if require_ai:
		var airborne: Variant = state["routes"]["routes"][0]["plan"]["airborne"]
		if not _check(airborne != null and (legacy or airborne["totals"].get("cargo_revenue", 0) > 0), "Restart includes airborne player cargo quote"):
			return false
	for owner in owners:
		if legacy:
			for route in (owner["routes"]["routes"] if owner["routes"] is Dictionary else owner["routes"]):
				if not _check(route["cargo_rate_cents"] == 150, "Every migrated route uses default"):
					return false
			if not _zero_cargo(owner):
				return false
		elif (require_ai or owner == state) and not _check(owner["finance"]["totals"].get("cargo_kg", 0) > 0 and owner["finance"]["totals"].get("cargo_revenue", 0) > 0, "Player and AI settle paid cargo"):
			return false
	return true

func _zero_cargo(value: Variant) -> bool:
	if value is Dictionary:
		for key in value:
			if key in ["cargo_kg", "cargo_revenue"] and not _check(value[key] == 0, "No retroactive cargo in any totals, archive or airborne quote"):
				return false
			if not _zero_cargo(value[key]):
				return false
	elif value is Array:
		for item in value:
			if not _zero_cargo(item):
				return false
	return true

func _views() -> bool:
	main._sync_view()
	await _click(main.navigation[2])
	var panel = main.routes_panel
	await _select(panel.route_tree)
	panel.refresh_operations()
	var route: Dictionary = main.simulation.get_routes()[0]
	if not _check(panel.cargo_input.value == route["cargo_rate_cents"] / 100.0 and ("%d kg/Tag · bisher befördert: %d kg · %s Umsatz" % [route["cargo_daily_demand_kg"], route["actual_cargo_kg"], main.UI.money(route["actual_cargo_revenue_eur"])]) in panel.cargo_status.text, "Route price, demand, settled kg/revenue match Rust"):
		return false
	var results: Array = main.simulation.get_flight_results()
	for i in range(results.size()):
		var row: TreeItem = panel.results_tree.get_root().get_child(i)
		if not _check(row.get_text(3) == main.UI.money(results[i]["ticket_revenue_eur"]) and row.get_text(6) == _cargo_text(results[i]), "Route flight ticket/cargo cells match Rust"):
			return false
	await _click(main.navigation[4])
	panel = main.finances_panel
	panel.refresh(true)
	var summary: Dictionary = main.simulation.get_financial_summary()
	if not _check(panel.metrics["revenue"].text == main.UI.money(summary["ticket_revenue_eur"]) and panel.metrics["cargo"].text == main.UI.money(summary["cargo_revenue_eur"]) and summary["ticket_revenue_eur"] + summary["cargo_revenue_eur"] == summary["revenue_eur"], "Separate company ticket/cargo totals reconcile"):
		return false
	await _select(panel.route_tree)
	var rows: Array = main.simulation.get_route_finances()
	for i in range(rows.size()):
		var row: TreeItem = panel.route_tree.get_root().get_child(i)
		if not _check(row.get_text(2) == main.UI.money(rows[i]["ticket_revenue_eur"]) and row.get_text(6) == _cargo_text(rows[i]), "Finance route ticket/cargo match Rust"):
			return false
	rows = main.simulation.get_route_flight_page(1, 0)
	for i in range(rows.size()):
		var row: TreeItem = panel.flights_tree.get_root().get_child(i)
		if not _check(row.get_text(5) == main.UI.money(rows[i]["ticket_revenue_eur"]) and row.get_text(8) == _cargo_text(rows[i]), "Finance flight ticket/cargo match Rust"):
			return false
	await _reveal(panel.chart_select)
	await _click(panel.chart_select)
	var popup: PopupMenu = panel.chart_select.get_popup()
	await _mouse(Vector2(popup.position) + Vector2(popup.size.x / 2.0, popup.size.y * 1.5 / 5.0))
	var revenue := 0
	for sample in panel.chart.samples:
		revenue += sample["revenue"]
	return _check(panel.chart.field == "revenue" and panel.chart.samples == main.simulation.get_finance_series(0) and revenue == summary["revenue_eur"], "Real chart selection includes both ticket and cargo income: field=%s selected=%s sum=%s expected=%s" % [panel.chart.field, panel.chart_select.selected, revenue, summary["revenue_eur"]])

func _layouts() -> bool:
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		await _click(main.navigation[2])
		await _select(main.routes_panel.route_tree)
		for control in [main.routes_panel.cargo_input, main.routes_panel.cargo_button, main.routes_panel.cargo_status, main.routes_panel.results_tree]:
			await _reveal(control)
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Route controls reachable at %s" % dimensions):
				return false
		await _reveal(main.routes_panel.cargo_input)
		await _align_top(main.routes_panel.cargo_input)
		await _capture("rate", dimensions)
		await _click(main.routes_panel.cargo_button)
		if not _check(main.routes_panel.status.text.begins_with("Frachtpreis geändert."), "Cargo apply reachable with mouse at every size"):
			return false
		await _reveal(main.routes_panel.cargo_status)
		await _capture("route", dimensions)
		if not await _table_end(main.routes_panel.results_tree, 6):
			return false
		await _capture("route-results", dimensions)
		await _click(main.navigation[4])
		for control in [main.finances_panel.metrics["cargo"], main.finances_panel.chart_select, main.finances_panel.chart, main.finances_panel.route_tree, main.finances_panel.flights_tree]:
			await _reveal(control)
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Finance controls reachable at %s" % dimensions):
				return false
		if not await _table_end(main.finances_panel.route_tree, 6):
			return false
		await _capture("finances-route", dimensions)
		if not await _table_end(main.finances_panel.flights_tree, 8):
			return false
		await _capture("finances", dimensions)
	return true

func _table_end(tree: Tree, column: int) -> bool:
	await _reveal(tree)
	# Tables have their own horizontal scrollbar. Exercise its track with real
	# mouse input, then align the table header in the enclosing page viewport.
	for child in tree.get_children(true):
		if child is HScrollBar and child.visible:
			for step in range(5):
				await _mouse(child.global_position + Vector2(child.size.x - 24, child.size.y / 2.0))
	await _align_top(tree)
	var row := tree.get_root().get_first_child()
	var cell := tree.get_item_area_rect(row, column)
	var visible := Rect2(Vector2.ZERO, tree.size)
	return _check(visible.encloses(cell), "Cargo table column reachable using horizontal mouse scroll: %s / %s scroll=%s" % [cell, visible, tree.get_scroll()])

func _align_top(control: Control) -> void:
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.scroll_vertical += int(control.global_position.y - parent.global_position.y)
		parent = parent.get_parent()
	await process_frame

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("cargo-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real rendered capture")

func _cargo_text(row: Dictionary) -> String:
	return "%d kg / %s" % [row["cargo_kg"], main.UI.money(row["cargo_revenue_eur"])]

func _advance(seconds: float) -> void:
	main.simulation.start()
	_check(main.simulation.advance(seconds), "Advance Rust clock")
	main.simulation.pause()

func _snapshot() -> Array:
	var sim = main.simulation
	_check(sim.save_game(probe_path).is_empty(), "Serialize full world")
	return [FileAccess.get_file_as_string(probe_path), sim.get_routes(), sim.get_financial_summary(), sim.get_route_finances(), sim.get_route_flight_page(1, 0), sim.get_finance_series(0), sim.get_competitors(), sim.get_active_flights()]

func _state() -> Dictionary:
	return JSON.parse_string(_snapshot()[0])["state"]

func _strip(text: String) -> String:
	return RegEx.create_from_string('"cargo_(rate_cents|kg|revenue)":-?\\d+,|,"cargo_(rate_cents|kg|revenue)":-?\\d+').sub(text, "", true)

func _write(path: String, text: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(text)
	file.close()

func _store(path: String, value: Array) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_var(value)
	file.close()

func _read(path: String) -> Array:
	return FileAccess.open(path, FileAccess.READ).get_var()

func _select(tree: Tree) -> void:
	await _reveal(tree)
	var row := tree.get_root().get_first_child()
	await _mouse(tree.global_position + tree.get_item_area_rect(row, 0).get_center())
	_check(tree.get_selected() == row, "Select route using mouse")

func _mouse(position: Vector2) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _key(code: Key, window: Window) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		window.push_input(event)
		await process_frame

func _reveal(control: Control) -> void:
	await process_frame
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(control)
		parent = parent.get_parent()
	await process_frame

func _click(button: Button) -> void:
	await _reveal(button)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
