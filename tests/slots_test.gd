extends "res://tests/pilot_ratings_test.gd"
## Derived shared usage, grandfathered v26 schedules and lossless v27 restarts.
var slot_path := ""

func _run() -> void:
	var mode := "ui"
	slot_path = ProjectSettings.globalize_path("user://slots-test.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--slots-mode="):
			mode = arg.trim_prefix("--slots-mode=")
		if arg.begins_with("--slots-save="):
			slot_path = arg.trim_prefix("--slots-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v26", "read-v26", "write-v27", "read-v27"], "Explicit slots mode"):
		return
	probe_path = slot_path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	if mode.begins_with("read-"):
		main.save_path = slot_path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(slot_path + ".initial"), "Fresh reader restores every world byte and player/AI API"):
			return
	else:
		if not _slot_fixture():
			return
		var original: String = _snapshot()[0]
		var saved := original.replace('"version":30', '"version":26') if mode.ends_with("v26") else original
		_write(slot_path, saved)
		if not _check(main.simulation.load_game(slot_path).is_empty() and _snapshot()[0] == original, "v26/v27 identity load preserves all player/AI schedules, money and seeds"):
			return
		_store(slot_path + ".initial", _snapshot())
	if not await _slot_views():
		return
	if mode != "ui":
		# Run through flights, landings, maintenance and AI reviews in both
		# processes; compare serialized bytes as well as every inherited API.
		_advance(7 * 86400 + 123)
		if mode.begins_with("write-"):
			_store(slot_path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(slot_path + ".expected"), "Seven-day reader equals uninterrupted player/AI world and APIs"):
			return
	print("PASS: slots " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _slot_fixture() -> bool:
	var sim = main.simulation
	for error in [sim.found_airline("Slots Audit", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.hire_missing_staff(), sim.schedule_route(1, 1, 120), sim.buy_aircraft("atr72_600")]:
		if not _check(error.is_empty(), "Prepare slot fixture: " + error):
			return false
	# Old versions allowed unlimited schedules. Clone untouched future services,
	# retaining normal AI owners and every integer/financial byte verbatim.
	var world: String = _snapshot()[0]
	var player := _json_value(world, '"state":')
	var competitors := _json_value(player, '"competitors":')
	player = _replace_value(player, '"competitors":', "[]")
	var fleet := _json_value(player, '"fleet":')
	var planes := _json_value(fleet, '"aircraft":')
	var plane := _json_value(planes.substr(1), "")
	var routes := _json_value(player, '"routes":')
	var services := _json_value(routes, '"routes":')
	var service := _json_value(services.substr(1), "")
	for id in range(2, 22):
		planes = planes.left(-1) + "," + plane.replace('"id":1,', '"id":%d,' % (id + 1)) + "]"
		services = services.left(-1) + "," + service.replace('"id":1,', '"id":%d,' % id).replace('"aircraft_ids":[1]', '"aircraft_ids":[%d]' % (id + 1)) + "]"
	fleet = _replace_value(fleet, '"aircraft":', planes).replace('"next_aircraft_id":3', '"next_aircraft_id":23')
	routes = _replace_value(routes, '"routes":', services).replace('"next_route_id":2', '"next_route_id":22')
	var staff := _json_value(player, '"staff":')
	var original_employees := _json_value(staff, '"employees":')
	var employees: Array[String] = []
	for copy in range(21):
		var remaining := original_employees.substr(1)
		while remaining.begins_with("{"):
			var employee := _json_value(remaining, "")
			employees.append(RegEx.create_from_string('"id":\\d+').sub(employee, '"id":%d' % (employees.size() + 1)))
			remaining = remaining.substr(employee.length() + 1)
	staff = _replace_value(staff, '"employees":', "[" + ",".join(employees) + "]")
	staff = RegEx.create_from_string('"next_id":\\d+').sub(staff, '"next_id":%d' % (employees.size() + 1))
	player = _replace_value(player, '"staff":', staff)
	player = _replace_value(player, '"fleet":', fleet)
	player = _replace_value(player, '"routes":', routes)
	player = _replace_value(player, '"competitors":', competitors)
	_write(slot_path, _replace_value(world, '"state":', player))
	var error: String = sim.load_game(slot_path)
	return _check(error.is_empty(), "Load valid grandfathered over-capacity world: " + error) and _check(sim.hire_missing_staff().is_empty(), "Staff all grandfathered services")

func _slot_views() -> bool:
	var sim = main.simulation
	var state := _state()
	if not _check(_routes(state).size() == 21 and _fleet(state).size() == 22, "21 grandfathered schedules plus one spare aircraft"):
		return false
	# Independent reservation count from all three owners' saved timetables.
	var used := 0
	for owner in _owners(state):
		var own := 0
		for route in _routes(owner):
			if "EDDF" in [route["origin_icao"], route["destination_icao"]] and route["plan"] != null:
				own += ceili(86400.0 / (2 * route["plan"]["interval_seconds"]))
		if not _check(own > 0, "Player and each AI reserve FRA slots"):
			return false
		used += own
	var expected := "EDDF: %d Slots belegt · 0 von 120 frei/Tag" % used
	if not _check(used > 120, "Migration fixture exceeds hub capacity"):
		return false
	main._sync_view()
	await _click(main.navigation[2])
	var panel = main.routes_panel
	if not _check(expected in panel.preview.text and panel.create_button.disabled and "FRA is full: 0 of 120 daily slots free (1 required)" in panel.preview.text, "Full hub preview shows exact shared usage, airport, free/required slots and disables creation"):
		return false
	var before := _snapshot()
	if not _check(sim.create_route("EDDF", "EDDM", 2) == "FRA is full: 0 of 120 daily slots free (1 required)" and _snapshot() == before, "Full-hub rejection leaves every owner unchanged"):
		return false
	main.attention_expanded = true
	main._refresh_attention()
	var count := 0
	var entries: Array = sim.get_attention_snapshot()
	for i in range(entries.size()):
		if entries[i]["kind"] == "slots":
			count += 1
			if not _check(expected in entries[i]["text"] and main.attention_rows.get_child(i).text == entries[i]["text"], "Attention renders exact Rust slot entry"):
				return false
	if not _check(count == 1, "One near-full attention entry"):
		return false
	await _reveal(panel.preview)
	await _capture("slots-full-preview", Vector2i(root.size))
	await _click(main.navigation[3])
	var map = main.map_panel
	for i in range(map.canvas.airports.size()):
		var airport: Dictionary = map.canvas.airports[i]
		if airport["icao"] == "EDDF":
			map.selected_id = -i - 1
			map.refresh()
		if airport["icao"] == "EDFH" and not _check(airport["slots"] == "EDFH: Slots unbegrenzt", "Secondary airport stays unlimited"):
			return false
	if not _check(expected in map.details.text, "Selected map airport renders exact shared usage with saturated free count"):
		return false
	await _reveal(map.details)
	await _capture("slots-map-detail", Vector2i(root.size))
	for id in [21, 20]:
		if not _check(sim.stop_schedule(id).is_empty(), "Release six reservations at both endpoints"):
			return false
	var released := "EDDF: %d Slots belegt · %d von 120 frei/Tag" % [used - 12, 120 - used + 12]
	main._sync_view()
	map.refresh()
	if not _check(released in map.details.text, "Selected map detail refreshes when schedules release slots"):
		return false
	await _click(main.navigation[2])
	if not _check(released in panel.preview.text and not panel.create_button.disabled, "Live preview enables creation when the hub has room again"):
		return false
	var near_full: Array = sim.get_attention_snapshot().filter(func(e): return e["kind"] == "slots")
	if not _check(near_full.size() == 1 and released in near_full[0]["text"], "Below-capacity near-full hub retains one updated attention entry"):
		return false
	for id in range(1, 20):
		if not _check(sim.stop_schedule(id).is_empty(), "Release remaining player reservations"):
			return false
	if not _check(sim.get_attention_snapshot().filter(func(e): return e["kind"] == "slots").is_empty(), "Near-full attention clears after release"):
		return false
	# Continue restart comparisons from the original over-capacity timetable.
	_write(probe_path, before[0])
	return _check(sim.load_game(probe_path).is_empty() and _snapshot() == before, "Restore every player/AI byte after reversible UI probes")

func _snapshot() -> Array:
	return super._snapshot() + [main.simulation.get_map_airports(), main.simulation.get_attention_snapshot()]
