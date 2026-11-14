extends "res://tests/pilot_ratings_test.gd"
## Maintenance-base presentation and byte-preserving v28/v29 process restarts.
var base_path := ""

func _run() -> void:
	var mode := "ui"
	base_path = ProjectSettings.globalize_path("user://maintenance-bases.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--bases-mode="):
			mode = arg.trim_prefix("--bases-mode=")
		if arg.begins_with("--bases-save="):
			base_path = arg.trim_prefix("--bases-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v28", "read-v28", "write-v29", "read-v29"], "Explicit maintenance-base mode"):
		return
	probe_path = base_path + ".probe"
	root.gui_embed_subwindows = true
	if not capture_dir.is_empty():
		root.size = Vector2i(1100, 1000)
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if mode.begins_with("read-"):
		main.save_path = base_path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(base_path + ".initial"), "Fresh reader matches full serialized player/AI worlds and API snapshots immediately"):
			return
	else:
		if not await _base_fixture():
			return
		if mode.ends_with("v28"):
			# v28 has no explicit bases. Remove only those arrays, retaining u64
			# seeds, fractional journals and already frozen outstation visits.
			if not _check(sim.set_maintenance_base("EDLP", false).is_empty(), "Remove optional base for independent v29 comparison baseline"):
				return
			var expected := _snapshot()
			var world: String = expected[0]
			var pattern := RegEx.create_from_string('"maintenance_bases":\\[[^\\]]*\\],')
			if not _check(pattern.search_all(world).size() == 3, "Player and both AI have maintenance-base arrays"):
				return
			var old := pattern.sub(world, "", true).replace('"version":29', '"version":28')
			if not _check(not '"maintenance_bases"' in old and '"version":28' in old, "Legacy fixture omits new field for every owner"):
				return
			_write(base_path, old)
			if not _check(sim.load_game(base_path).is_empty() and _snapshot() == expected, "v28 migration only restores empty explicit bases and version; all other world bytes and APIs match v29 baseline"):
				return
		elif not _check(sim.save_game(base_path).is_empty(), "Save v29 with running home and outstation checks and a fee-paying base"):
			return
		_store(base_path + ".initial", _snapshot())
	if not _check(sim.get_fleet()[1]["maintenance_state"] == "active" and sim.get_fleet()[1]["maintenance_location_kind"] == "Außenstation (Aufschlag)", "Restart fixture retains running outstation check"):
		return
	main._sync_view()
	await _click(main.navigation[1])
	if not await _base_view("EDDF") or not await _plane_view(1) or not await _plane_view(2):
		return
	if mode == "ui":
		if not await _base_actions():
			return
	else:
		_advance(7 * 86400)
		main._sync_view()
		await _click(main.navigation[1])
		for owner in _owners(_state()):
			if not _check(_fleet(owner)[0]["wear"]["checks"] > 0 and _fleet(owner)[1]["wear"]["checks"] > 0 and owner["finance"]["totals"]["flights"] > 0, "Player and both AI complete checks and resume flights over seven days"):
				return
		if not await _base_view("EDLP") or not await _plane_view(2):
			return
		if mode.begins_with("write-"):
			_store(base_path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(base_path + ".expected"), "Seven-day reader equals uninterrupted full player/AI worlds and API snapshots"):
			return
	print("PASS: maintenance bases " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _prepare_bases(sim: RefCounted, title: String) -> bool:
	for error in [sim.found_airline(title, "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.lease_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.create_route("EDDF", "EDDM", 2), sim.hire_missing_staff(), sim.schedule_route(1, 1, 120), sim.schedule_route(2, 2, 120)]:
		if not _check(error.is_empty(), "Prepare maintenance-base fixture: " + error):
			return false
	# The default technician pool covers one check; this fixture runs two at
	# once and must not confuse a staffing wait with base repricing.
	if not _check(sim.hire_staff("technician", 3).is_empty(), "Staff both simultaneous checks"):
		return false
	sim.start()
	sim.advance(8000)
	sim.pause()
	return true

func _base_fixture() -> bool:
	var sim = main.simulation
	if not _prepare_bases(sim, "Maintenance Bases"):
		return false
	main._sync_view()
	await _click(main.navigation[1])
	if not await _base_view("EDDF") or not await _plane_view(1):
		return false
	if not _check(sim.get_fleet()[0]["maintenance_location"] == "EDDF" and sim.get_fleet()[0]["maintenance_location_kind"] == "Wartungsbasis", "Home estimate before ordering"):
		return false
	await _click(main.maintenance_button)
	_advance(8000)
	main._process(0)
	main._refresh_fleet()
	if not await _plane_view(2):
		return false
	if not _check(sim.get_fleet()[1]["maintenance_location"] == "EDDM" and sim.get_fleet()[1]["maintenance_location_kind"] == "Außenstation (Aufschlag)", "Outstation estimate before ordering"):
		return false
	# The airport selector is the base list; the estimate below remains visible.
	if not await _base_view("EDDM"):
		return false
	await _reveal(main.base_action)
	await _capture("base-list-outstation-estimate", Vector2i(root.size))
	if not capture_dir.is_empty():
		await _click(main.base_airports)
		await _capture("base-list-expanded", Vector2i(root.size))
		await _key(KEY_ESCAPE)
	await _click(main.maintenance_button)
	_advance(8000)
	main._refresh_fleet()
	if not _check(sim.get_fleet()[0]["maintenance_state"] == "active" and sim.get_fleet()[1]["maintenance_state"] == "active" and not sim.get_fleet()[0]["maintenance_waiting_staff"] and not sim.get_fleet()[1]["maintenance_waiting_staff"], "Home and outstation checks are both running and staffed"):
		return false
	# A fee-paying base survives the v29 restart; v28 uses the empty baseline.
	if not await _base_view("EDLP"):
		return false
	await _click(main.base_action)
	if not _check(_base_row("EDLP")["is_base"], "Mouse opens second airport base") or not await _base_view("EDLP"):
		return false
	# Assemble both AI from independent API-built owner states. Preserve JSON
	# bytes instead of rounding seeds through Godot's JSON number representation.
	var world: String = _snapshot()[0]
	for index in range(2):
		var other: RefCounted = ClassDB.instantiate("SimulationClock")
		if not _prepare_bases(other, ["Regional Wings", "Continental Sky"][index]):
			return false
		if not _check(other.request_maintenance(1).is_empty(), "AI home request"):
			return false
		other.start()
		other.advance(8000)
		other.pause()
		if not _check(other.request_maintenance(2).is_empty(), "AI outstation request"):
			return false
		other.start()
		other.advance(8000)
		other.pause()
		_check(other.save_game(probe_path + ".ai").is_empty(), "Serialize AI fixture")
		var owner := _json_value(FileAccess.get_file_as_string(probe_path + ".ai"), '"state":')
		owner = _replace_value(owner, '"competitors":', "[]")
		owner = _replace_value(owner, '"clock":', '{"elapsed":{"secs":24000,"nanos":0},"running":false,"speed_index":0}')
		owner = _replace_value(owner, '"fleet":', _json_value(owner, '"aircraft":') + ',"next_aircraft_id":3')
		owner = _replace_value(owner, '"routes":', _json_value(_json_value(owner, '"routes":'), '"routes":') + ',"next_route_id":3')
		var at := 0
		for skip in range(index + 1):
			at = world.find('"game":', at) + 7
		var old := _json_value(world.substr(at), "")
		world = world.left(at) + owner + world.substr(at + old.length())
	_write(probe_path, world)
	return _check(sim.load_game(probe_path).is_empty(), "Load all-owner running-check fixture")

func _base_actions() -> bool:
	var sim = main.simulation
	var before: Array = sim.get_fleet()
	if not await _base_view("EDLP"):
		return false
	await _click(main.base_action)
	if not _check(not _base_row("EDLP")["is_base"], "Close unused second base while checks run") or not await _plane_view(1):
		return false
	if not await _base_view("EDDM"):
		return false
	await _click(main.base_action)
	if not _check(_base_row("EDDM")["is_base"], "Open base at running outstation check") or not await _base_view("EDDM") or not await _plane_view(2):
		return false
	for i in range(2):
		var after: Dictionary = sim.get_fleet()[i]
		for key in ["maintenance_cost_eur", "maintenance_start", "maintenance_end", "maintenance_end_seconds", "maintenance_hours"]:
			if not _check(after[key] == before[i][key], "Base changes preserve active check " + key):
				return false
	var reason: String = _base_row("EDDM")["action_error"]
	var frozen := _snapshot()
	if not _check(not reason.is_empty() and main.base_action.disabled, "Rust running-check reason disables close"):
		return false
	await _click(main.base_action)
	if not _check(sim.set_maintenance_base("EDDM", false) == reason and _snapshot() == frozen, "Closing running-check base refused with exact Rust reason and no mutation"):
		return false
	# Exercise the handler too, guarding against stale enabled controls.
	main._change_maintenance_base()
	if not _check(main.maintenance_status.text == reason and _snapshot() == frozen, "Action feedback shows exact Rust refusal"):
		return false
	var end: int = sim.get_fleet()[1]["maintenance_end_seconds"]
	_advance(end - sim.get_elapsed_seconds())
	main._refresh_fleet()
	if not await _base_view("EDDM"):
		return false
	await _click(main.base_action)
	return _check(not _base_row("EDDM")["is_base"] and sim.get_fleet()[1]["checks"] == 1, "Closing works after check completes: base=%s plane=%s feedback=%s" % [_base_row("EDDM"), sim.get_fleet()[1], main.maintenance_status.text])

func _base_row(icao: String) -> Dictionary:
	for row in main.simulation.get_maintenance_airports():
		if row["icao"] == icao:
			return row
	return {}

func _base_view(icao: String) -> bool:
	main._refresh_maintenance()
	var rows: Array = main.simulation.get_maintenance_airports()
	var index := -1
	for i in range(rows.size()):
		var row: Dictionary = rows[i]
		var expected := "%s · %s%s" % [row["icao"], row["name"], " · Heimatbasis" if row["is_home"] else (" · Wartungsbasis" if row["is_base"] else "")]
		if not _check(main.base_airports.get_item_text(i) == expected, "Base list equals exact Rust status and airport"):
			return false
		if row["icao"] == icao:
			index = i
	if not _check(index >= 0, "Base airport present"):
		return false
	await _choose_key(main.base_airports, index)
	var row: Dictionary = rows[index]
	var details := "Heimatbasis · dauerhaft, gebührenfrei" if row["is_home"] else "Einrichtung %s · alle 30 Spieltage %s%s" % [main.UI.money(row["setup_eur"]), main.UI.money(row["fee_eur"]), " · nächste Gebühr " + row["next_fee"] if row["is_base"] else ""]
	if not String(row["action_error"]).is_empty():
		details += "\n" + row["action_error"]
	var action := "Basis schließen (keine Erstattung)" if row["is_base"] else "Basis eröffnen · %s" % main.UI.money(row["setup_eur"])
	return _check(main.base_details.text == details and main.base_action.text == action and main.base_action.disabled == not String(row["action_error"]).is_empty(), "Exact Rust setup cost, recurring fee, deadline, reason and enabled state")

func _plane_view(id: int) -> bool:
	var item: TreeItem = main.fleet_tree.get_root().get_child(id - 1)
	item.select(0)
	main.fleet_tree.item_selected.emit()
	var plane: Dictionary = main.simulation.get_fleet()[id - 1]
	var text: String = main.maintenance_details.text
	if not _check("Hier / nächster Landeort: %s · %s" % [plane["maintenance_location"], plane["maintenance_location_kind"]] in text, "Exact Rust maintenance location and base/outstation kind"):
		return false
	var expected := "Wartung jetzt: ca. %s · %.0f h Dauer" % [main.UI.money(plane["maintenance_cost_eur"]), plane["maintenance_hours"]]
	if plane["maintenance_state"] == "active":
		expected = "%s läuft: %s – %s · %s" % ["Ungeplante Wartung" if plane["maintenance_unscheduled"] else "Wartung", plane["maintenance_start"], plane["maintenance_end"], main.UI.money(plane["maintenance_cost_eur"])]
	return _check(expected in text, "Displayed check price, duration or frozen start/end equals Rust API")

func _snapshot() -> Array:
	return super._snapshot() + [main.simulation.get_maintenance_airports(), main.simulation.get_staff(), main.simulation.get_staff_overview(), main.simulation.get_attention_snapshot()]

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("maintenance-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real Xvfb maintenance render")
