extends "res://tests/airport_charges_test.gd"
## Mouse/keyboard Personal controls and byte-preserving legacy/current restarts.
var trainee := 0

func _run() -> void:
	var mode := "ui"
	var path := ProjectSettings.globalize_path("user://pilot-ratings.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--pilot-mode="):
			mode = arg.trim_prefix("--pilot-mode=")
		if arg.begins_with("--pilot-save="):
			path = arg.trim_prefix("--pilot-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v24", "read-v24", "write-v25", "read-v25"], "Explicit pilot mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if mode == "ui":
		if not await _pilot_controls():
			return
	elif mode.begins_with("write-"):
		if not _prepare(sim, "Pilot Restart"):
			return
		# Both AI owners receive real API-built mixed-fleet states, including a
		# suspended plan in a staffed hangar and a second running service.
		var world: String = _snapshot()[0]
		for index in range(2):
			var other: RefCounted = ClassDB.instantiate("SimulationClock")
			if not _prepare(other, ["Regional Wings", "Continental Sky"][index]):
				return
			_check(other.save_game(probe_path + ".ai").is_empty(), "Serialize AI fixture")
			var owner: String = FileAccess.get_file_as_string(probe_path + ".ai")
			owner = _json_value(owner, '"state":')
			owner = _replace_value(owner, '"competitors":', "[]")
			# SaveState wraps fleet/routes and uses a display clock; embedded Game
			# uses their raw arrays/IDs and the runtime Clock representation.
			owner = _replace_value(owner, '"clock":', '{"elapsed":{"secs":16000,"nanos":0},"running":true,"speed_index":0}')
			owner = _replace_value(owner, '"fleet":', _json_value(owner, '"aircraft":') + ',"next_aircraft_id":3')
			owner = _replace_value(owner, '"routes":', _json_value(_json_value(owner, '"routes":'), '"routes":') + ',"next_route_id":3')
			var at := 0
			for skip in range(index + 1):
				at = world.find('"game":', at) + 7
			var old := _json_value(world.substr(at), "")
			world = world.left(at) + owner + world.substr(at + old.length())
		_write(path, world)
		var load_error: String = sim.load_game(path)
		if not _check(load_error.is_empty(), "Load all-owner API-built maintenance fixture: " + load_error):
			return
		if mode.ends_with("v24"):
			# v24 has no ratings or training. Strip only the new employee fields;
			# do not round u64 seeds or financial remainders through Godot JSON.
			var before: String = _snapshot()[0]
			var old := _without_ratings(before).replace('"version":25', '"version":24')
			if not _check(not '"rating"' in old and not '"training"' in old and '"version":24' in old, "Legacy fixture contains no post-v24 employee fields"):
				return
			_write(path, old)
			if not _check(sim.load_game(path).is_empty() and _without_ratings(_snapshot()[0]) == _without_ratings(before), "v24 migration preserves all non-rating world bytes"):
				return
		else:
			if not _begin_training():
				return
			_advance(3600)
			_check(sim.save_game(path).is_empty(), "Save v25 pilot mid-training")
		_store(path + ".initial", _snapshot())
	else:
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(path + ".initial"), "Fresh reader matches uninterrupted full world and APIs immediately"):
			return
	if mode != "ui":
		var state := _state()
		for owner in _owners(state):
			if not _check(_fleet(owner).size() == 2 and _fleet(owner)[0]["status"] == "maintenance" and _fleet(owner)[0]["maintenance"]["resume"] != null and _routes(owner)[1]["plan"] != null, "Every owner has mixed fleet, maintenance with reserved crew and a running plan"):
				return
		if mode.ends_with("v24"):
			# Legacy training cannot predate the feature. Start it after migration
			# in writer and reader; the v25 pair additionally crosses mid-training.
			if not _begin_training():
				return
		else:
			for employee in sim.get_staff():
				if employee["status"] == "training":
					trainee = employee["id"]
			if not _check(trainee > 0, "Mid-training pilot survives v25 restart"):
				return
		var payroll: int = sim.get_financial_summary()["payroll_eur"]
		main._sync_view()
		await _click(main.navigation[5])
		if not _pilot_view():
			return
		# Stop one second before the exact deadline, then cross it separately.
		var ends := 0
		for employee in _state()["staff"]["employees"]:
			if employee["id"] == trainee:
				ends = employee["training"]["ends_at"]
		var now: int = _state()["clock"]["elapsed_seconds"]
		_advance(ends - now - 1)
		if not _check(_employee()["status"] == "training", "Pilot unavailable until exact Rust deadline"):
			return
		_advance(1)
		if not _check(_employee()["status"] == "available" and _employee()["rating"] == "737", "Conversion completes exactly at deadline"):
			return
		_advance(now + 100 * 86400 - ends)
		if not _check(sim.get_financial_summary()["payroll_eur"] > payroll, "100-day continuation books wages and training personnel expense") or not _pilot_view():
			return
		for owner in _owners(_state()):
			if not _check(not owner["history"]["payroll"].is_empty(), "Player and AI wages booked"):
				return
		if mode.begins_with("write-"):
			_store(path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(path + ".expected"), "100-day reader equals uninterrupted world, wages, personnel expense and all APIs"):
			return
	print("PASS: pilot ratings " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _prepare(sim: RefCounted, title: String) -> bool:
	for error in [sim.found_airline(title, "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.lease_aircraft("a320_200"), sim.create_route("EDDF", "EDDM", 1), sim.create_route("EDDF", "EDDM", 2), sim.hire_missing_staff(), sim.schedule_route(1, 1, 120), sim.schedule_route(2, 2, 120)]:
		if not _check(error.is_empty(), "Prepare mixed fleet: " + error):
			return false
	sim.start()
	sim.advance(8000)
	sim.pause()
	if not _check(sim.request_maintenance(1).is_empty(), "Request worn aircraft maintenance"):
		return false
	sim.start()
	sim.advance(8000)
	sim.pause()
	return true

func _begin_training() -> bool:
	var sim = main.simulation
	if not _check(sim.hire_pilots("ATR 72", 1).is_empty(), "Hire spare trainee"):
		return false
	trainee = sim.get_staff()[-1]["id"]
	var before: int = sim.get_financial_summary()["payroll_eur"]
	return _check(sim.train_pilot(trainee, "737").is_empty() and sim.get_financial_summary()["payroll_eur"] - before == sim.get_staff_overview()["conversion_cost_eur"], "Conversion cost is booked exactly once as personnel expense")

func _employee() -> Dictionary:
	for employee in main.simulation.get_staff():
		if employee["id"] == trainee:
			return employee
	return {}

func _pilot_controls() -> bool:
	var sim = main.simulation
	if not _check(sim.found_airline("Pilot Input", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty(), "Found UI fixture"):
		return false
	main._sync_view()
	await _click(main.navigation[5])
	var panel = main.staff_panel
	await _choose_rating(panel.rating_select, _rating_index("ATR 72"))
	await _reveal(panel.count_spin)
	await _mouse(panel.count_spin.get_line_edit().get_global_rect().get_center())
	await _key(KEY_HOME)
	await _key(KEY_DELETE)
	var digit := InputEventKey.new()
	digit.keycode = KEY_2
	digit.unicode = 50
	digit.pressed = true
	root.push_input(digit)
	await process_frame
	await _key(KEY_ENTER)
	await _click(panel.hire_button)
	trainee = sim.get_staff()[-1]["id"]
	if not _check(sim.get_staff().size() == 2 and _employee()["rating"] == "ATR 72", "Mouse hires pilot for mouse-selected type with keyboard-entered count"):
		return false
	# Add a type with no rated pilots, so the shortage contains the exact Rust
	# rating requirement while ATR pilots remain available.
	if not _check(sim.lease_aircraft("a320_200").is_empty(), "Add missing-rating aircraft"):
		return false
	main._sync_view()
	await _click(main.navigation[5])
	if not _pilot_view():
		return false
	await _choose_rating(panel.rating_select, _rating_index("737"))
	await _select_trainee()
	await _click(panel.train_button)
	if not _check(_employee()["status"] == "training" and "Umschulung begonnen" in panel.dismiss_status.text, "Mouse starts conversion") or not _pilot_view():
		return false
	var before := _snapshot()
	for control in [panel.dismiss_button, panel.train_button]:
		if not _check(control.disabled, "Training prevents dismissal and re-conversion"):
			return false
		await _click(control)
	if not _check(_snapshot() == before, "Mouse clicks on disabled actions preserve complete world"):
		return false
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		await _reveal(panel.shortage_label)
		if not _check(main.size.x <= dimensions.x and panel.size.x <= dimensions.x - 40, "Personal page fits %s" % dimensions):
			return false
		await _capture("shortage", dimensions)
		await _reveal(panel.ratings_tree)
		await _capture("ratings", dimensions)
		panel.ratings_tree.scroll_to_item(panel.ratings_tree.get_root().get_child(2))
		await process_frame
		await _capture("ratings-last", dimensions)
		panel.ratings_tree.scroll_to_item(panel.ratings_tree.get_root().get_child(0))
		await _select_trainee()
		await _capture("employee", dimensions)
		# Narrow tables deliberately scroll horizontally. Capture the deadline
		# column as well as the identity columns, without mistaking truncation
		# for an unreadable or missing API value.
		await _wheel(panel.staff_tree, MOUSE_BUTTON_WHEEL_RIGHT)
		await process_frame
		await _capture("deadline", dimensions)
		await _wheel(panel.staff_tree, MOUSE_BUTTON_WHEEL_LEFT)
		await _reveal(panel.train_button)
		if not _check(panel.is_visible_in_tree() and main.current_page == 5 and root.get_visible_rect().encloses(panel.train_button.get_global_rect()), "Training action reachable at %s" % dimensions):
			return false
		await _capture("training", dimensions)
	return true

func _wheel(tree: Tree, direction: MouseButton) -> void:
	# Center the gesture on the visible employee row (a tall table may extend
	# above/below the page viewport). Ten horizontal notches reach either end.
	var row := tree.get_selected()
	var point := tree.global_position + tree.get_item_area_rect(row, 0).get_center()
	point.x = tree.get_global_rect().get_center().x
	for notch in range(10):
		for pressed in [true, false]:
			var event := InputEventMouseButton.new()
			event.button_index = direction
			event.position = point
			event.pressed = pressed
			Input.parse_input_event(event)
			await process_frame

func _choose_rating(choice: OptionButton, index: int) -> void:
	await _click(choice)
	var popup := choice.get_popup()
	await _mouse(Vector2(popup.position) + Vector2(popup.size.x / 2.0, popup.size.y * (index + 0.5) / choice.item_count))
	_check(choice.selected == index, "Mouse selects pilot type expected=%d actual=%d popup=%s size=%s choice=%s" % [index, choice.selected, popup.position, popup.size, choice.get_global_rect()])

func _rating_index(rating: String) -> int:
	var choice = main.staff_panel.rating_select
	for i in range(choice.item_count):
		if choice.get_item_text(i) == rating:
			return i
	return -1

func _select_trainee() -> void:
	var tree: Tree = main.staff_panel.staff_tree
	await _reveal(tree)
	var row := tree.get_root().get_first_child()
	while row != null:
		if row.get_metadata(0)["id"] == trainee:
			tree.scroll_to_item(row)
			await process_frame
			var parent := tree.get_parent()
			while parent != null:
				if parent is ScrollContainer:
					var point := tree.global_position + tree.get_item_area_rect(row, 0).get_center()
					parent.scroll_vertical += int(point.y - parent.global_position.y - parent.size.y / 2)
				parent = parent.get_parent()
			for frame in range(5):
				await process_frame
			var point := tree.global_position + tree.get_item_area_rect(row, 0).get_center()
			if not _check(tree.is_visible_in_tree() and root.get_visible_rect().has_point(point) and point.y > main.navigation[5].get_global_rect().end.y, "Employee click stays inside visible content"):
				return
			await _mouse(point)
			_check(main.current_page == 5 and main.staff_panel.selected_id == trainee, "Mouse selects trainee on Personal page")
			return
		row = row.get_next()
	_check(false, "Trainee exists in employee table")

func _pilot_view() -> bool:
	var panel = main.staff_panel
	panel.refresh(true)
	var row: TreeItem = panel.ratings_tree.get_root().get_first_child()
	for rating in main.simulation.get_staff_overview()["ratings"]:
		for i in range(4):
			if not _check(row.get_text(i) == str(rating[["rating", "rated", "busy", "available"][i]]), "Per-type counts match Rust API"):
				return false
		row = row.get_next()
	for plane in main.simulation.get_staff_overview()["aircraft"]:
		if plane["state"] == "short" and not _check(plane["problem"] in panel.shortage_label.text, "Exact missing-rating shortage matches Rust API"):
			return false
	row = panel.staff_tree.get_root().get_first_child()
	for employee in main.simulation.get_staff():
		if not _check(row.get_text(1) == "%s · %s" % [employee["role_name"], employee["rating"]] and row.get_text(3) == employee["status_text"], "Employee rating and exact training deadline match Rust API"):
			return false
		row = row.get_next()
	return true

func _snapshot() -> Array:
	return super._snapshot() + [main.simulation.get_staff(), main.simulation.get_staff_overview()]

func _fleet(owner: Dictionary) -> Array:
	return owner["fleet"]["aircraft"] if owner["fleet"] is Dictionary else owner["fleet"]

func _without_ratings(world: String) -> String:
	world = RegEx.create_from_string(',?"rating":(?:"[^"]*"|null)').sub(world, "", true)
	return RegEx.create_from_string(',?"training":null').sub(world, "", true)

# Extract a JSON object/array without decoding/re-encoding integer seeds or
# fractional accounting journals. Used only to assemble independent API fixtures.
func _json_value(text: String, key: String) -> String:
	var start := 0 if key.is_empty() else text.find(key) + key.length()
	var depth := 0
	var quoted := false
	var escaped := false
	for i in range(start, text.length()):
		var c := text[i]
		if quoted:
			if escaped:
				escaped = false
			elif c == "\\":
				escaped = true
			elif c == '"':
				quoted = false
		elif c == '"':
			quoted = true
		elif c in ["{", "["]:
			depth += 1
		elif c in ["}", "]"]:
			depth -= 1
			if depth == 0:
				return text.substr(start, i + 1 - start)
	return ""

func _replace_value(text: String, key: String, value: String) -> String:
	var start := text.find(key) + key.length()
	var old := _json_value(text, key)
	return text.left(start) + value + text.substr(start + old.length())

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("pilot-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real Xvfb render")
