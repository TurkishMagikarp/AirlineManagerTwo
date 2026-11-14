extends "res://tests/insurance_test.gd"
## Actual input, viewport renders and live main-loop refresh; rules stay in Rust.
var trainee := 0
var fixture := ""

func _run() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	fixture = ProjectSettings.globalize_path("user://attention-fixture.json")
	probe_path = fixture + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	for error in [sim.found_airline("Attention UI", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.lease_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.create_route("EDDF", "EDDM", 2), sim.hire_pilots("ATR 72", 2)]:
		if not _check(error.is_empty(), "Create multi-target fixture: " + error):
			return
	trainee = sim.get_staff()[-1]["id"]
	if not _check(sim.train_pilot(trainee, "737").is_empty() and sim.save_game(fixture).is_empty(), "Save active training with idle fleet and crew shortages"):
		return
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5):
			await process_frame
		if not _restore() or not await _navigation(dimensions):
			return
	for speed in [1, 4, 16]:
		if not _restore() or not await _live_updates(speed):
			return
	print("PASS: attention real input, layouts, stable rows, live 1x/4x/16x and save/load")
	main.queue_free()
	await process_frame
	quit(0)

func _restore() -> bool:
	if not _check(main.simulation.load_game(fixture).is_empty(), "Restore attention fixture"):
		return false
	main.attention_expanded = true
	main._sync_view()
	return _rows_match()

func _rows_match() -> bool:
	var entries: Array = main.simulation.get_attention_snapshot()
	if not _check(main.attention_rows.get_child_count() == entries.size(), "Exactly one rendered row per Rust entry"):
		return false
	for i in range(entries.size()):
		if not _check(main.attention_rows.get_child(i).text == entries[i]["text"], "Exact ordered Rust row text"):
			return false
	return _check(main.attention_scroll.visible == (main.attention_expanded and not entries.is_empty()), "Visibility follows current conditions and collapse state")

func _entry_button(kind: String, id: int) -> Button:
	var entries: Array = main.simulation.get_attention_snapshot()
	for i in range(entries.size()):
		var entry: Dictionary = entries[i]
		var target: int = entry["employee_id"] if kind == "training" else entry["route_id"] if kind == "crew" else entry["aircraft_id"]
		if entry["kind"] == kind and target == id:
			return main.attention_rows.get_child(i)
	return null

func _navigation(dimensions: Vector2i) -> bool:
	# Select target 2 after target 1: a default selection cannot satisfy this.
	for id in [1, 2]:
		await _click(_entry_button("crew", id))
		if not _check(main.current_page == 2 and main.routes_panel._route().get("id", 0) == id, "Crew mouse click selects route %d at %s" % [id, dimensions]):
			return false
		await _click(_entry_button("idle", id))
		if not _check(main.current_page == 1 and main.selected_aircraft == id and main.fleet_tree.get_selected().get_metadata(0) == id, "Idle mouse click selects aircraft %d at %s" % [id, dimensions]):
			return false
	await _click(_entry_button("training", trainee))
	if not _check(main.current_page == 5 and main.staff_panel.selected_id == trainee, "Training mouse click selects exact employee"):
		return false
	await _reveal(main.attention_button)
	await _capture("expanded", dimensions)
	# Unchanged snapshots must preserve node identity, focus and scroll position.
	main.attention_button.grab_focus()
	for frame in range(3):
		await process_frame
	var list_rect: Rect2 = main.attention_scroll.get_global_rect()
	var nodes: Array = main.attention_rows.get_children()
	var focused: Control = root.gui_get_focus_owner()
	var scroll: int = main.attention_scroll.scroll_vertical
	for i in range(10):
		main._process(0.1)
		await process_frame
	if not _check(main.attention_rows.get_children() == nodes and root.gui_get_focus_owner() == focused and main.attention_scroll.scroll_vertical == scroll and main.attention_scroll.get_global_rect() == list_rect, "Stable rows do not rebuild, move scroll or steal focus on unchanged frames"):
		return false
	await _click(main.attention_button)
	if not _check(not main.attention_scroll.visible, "Mouse collapse"):
		return false
	await _capture("collapsed", dimensions)
	await _click(main.attention_button)
	if not _rows_match():
		return false
	# Page controls below the list must receive input, also when it is expanded.
	await _click(main.navigation[1])
	var before: int = main.simulation.get_fleet().size()
	await _click(main.lease_button)
	if not _check(main.current_page == 1 and main.simulation.get_fleet().size() == before + 1, "Expanded attention list does not intercept underlying fleet action"):
		return false
	main._process(0.0)
	return _rows_match()

func _live_updates(speed: int) -> bool:
	var sim = main.simulation
	await _click(main.speed_buttons[[1, 4, 16].find(speed)])
	await _click(main.start_button)
	# Run actual frame callbacks briefly as well as deterministic boundary steps.
	var live_nodes: Array = main.attention_rows.get_children()
	var live_scroll: int = main.attention_scroll.scroll_vertical
	var live_rect: Rect2 = main.attention_scroll.get_global_rect()
	main.set_process(true)
	for i in range(4):
		await process_frame
	if not _check(main.attention_rows.get_children() == live_nodes and main.attention_scroll.scroll_vertical == live_scroll and main.attention_scroll.get_global_rect() == live_rect, "Running unchanged snapshot preserves rows and scroll at %dx" % speed):
		return false
	await _click(_entry_button("idle", 2))
	main.set_process(false)
	if not _check(main.current_page == 1 and main.selected_aircraft == 2, "Real row input also works during automatic %dx frames" % speed):
		return false
	if not _check(sim.is_running() and sim.get_speed() == speed, "Real start/speed controls reach live %dx" % speed):
		return false
	if not _check(sim.hire_missing_staff().is_empty(), "Resolve all crew shortages"):
		return false
	main._process(0.0)
	if not _check(_entry_button("crew", 1) == null and _entry_button("crew", 2) == null, "Crew rows clear on next running frame"):
		return false
	if not _check(sim.schedule_route(1, 1, 120).is_empty(), "Assign idle aircraft"):
		return false
	main._process(0.0)
	if not _check(_entry_button("idle", 1) == null and _entry_button("idle", 2) != null, "Only assigned aircraft clears while another idle entry stays"):
		return false
	await _click(main.attention_button)
	if not _check(not main.attention_scroll.visible, "Collapse while running"):
		return false
	main.save_path = fixture + ".running"
	await _click(main.save_button)
	var expected: String = _snapshot()[0]
	var entries: Array = sim.get_attention_snapshot()
	main._process(1.0)
	await _click(main.load_button)
	if not _check(main.save_status.text == "Spielstand geladen." and _snapshot()[0] == expected and sim.get_attention_snapshot() == entries and not main.attention_scroll.visible, "Real save/load restores exact player/AI world, rows and collapsed view at %dx" % speed):
		return false
	var ends := 0
	for employee in _state()["staff"]["employees"]:
		if employee["id"] == trainee:
			ends = employee["training"]["ends_at"]
	var remaining: float = ends - sim.get_elapsed_seconds()
	main._process((remaining - 1.0) / speed)
	if not _check(_entry_button("training", trainee) != null, "Training entry stays until deadline"):
		return false
	main._process(1.0 / speed)
	if not _check(_entry_button("training", trainee) == null and not main.attention_scroll.visible, "Training entry clears exactly at deadline without expanding collapsed list"):
		return false
	await _click(main.attention_button)
	if not _rows_match():
		return false
	await _click(_entry_button("idle", 2))
	if not _check(main.current_page == 1 and main.selected_aircraft == 2, "Remaining row target is correct after removals and reload"):
		return false
	# Empty list removes its viewport; empty header cannot reopen it.
	if not _check(sim.schedule_route(2, 2, 120).is_empty(), "Assign last idle aircraft"):
		return false
	main._process(0.0)
	if not _check(sim.get_attention_snapshot().is_empty() and main.attention_button.disabled and not main.attention_scroll.visible, "No stale rows or empty scroll viewport"):
		return false
	await _click(main.attention_button)
	await _click(main.pause_button)
	return _rows_match()

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("attention-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real attention render")

func _click(button: Button) -> void:
	if not _check(button != null, "Click target exists"):
		return
	await _reveal(button)
	# Nested list/page scrolls settle independently after resizing/focus changes.
	for frame in range(5):
		await process_frame
	var visible_rect := button.get_global_rect()
	var parent := button.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			visible_rect = visible_rect.intersection(parent.get_global_rect())
		parent = parent.get_parent()
	visible_rect = visible_rect.intersection(root.get_visible_rect())
	if not _check(visible_rect.has_area() and button.is_visible_in_tree(), "Mouse target intersects every clipping viewport: " + button.text):
		return
	await _mouse(visible_rect.get_center())

func _reveal(control: Control) -> void:
	await process_frame
	# Center a clickable point, not the whole nested scroll container, from
	# inside out. An entire table/list need not fit in the narrow page viewport.
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.scroll_vertical += int(control.get_global_rect().get_center().y - parent.get_global_rect().get_center().y)
			for frame in range(3):
				await process_frame
		parent = parent.get_parent()
