extends "res://tests/pilot_ratings_test.gd"
## Reputation presentation, real operational recovery and lossless v27/v29 restarts.
var reputation_path := ""

func _run() -> void:
	var mode := "ui"
	reputation_path = ProjectSettings.globalize_path("user://reputation-test.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--reputation-mode="):
			mode = arg.trim_prefix("--reputation-mode=")
		if arg.begins_with("--reputation-save="):
			reputation_path = arg.trim_prefix("--reputation-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v27", "read-v27", "write-v29", "read-v29"], "Explicit reputation mode"):
		return
	probe_path = reputation_path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if mode.begins_with("read-"):
		main.save_path = reputation_path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(reputation_path + ".initial"), "Fresh reader matches full player/AI world bytes and API snapshots immediately"):
			return
	else:
		if not _reputation_fixture():
			return
		if mode.ends_with("v27"):
			# A real v27 save has no reputation in any owner. Compare with an
			# independently loaded neutral v29 baseline, retaining every other
			# byte (especially u64 seeds and fractional accounting journals).
			var world: String = _snapshot()[0]
			var pattern := RegEx.create_from_string('"reputation":\\{[^{}]*\\}')
			var neutral := pattern.sub(world, '"reputation":{"flights":0,"late":0,"score":0,"trend":0}', true)
			_write(probe_path, neutral)
			if not _check(sim.load_game(probe_path).is_empty(), "Load neutral v29 comparison world"):
				return
			var expected := _snapshot()
			var old := RegEx.create_from_string(',"reputation":\\{[^{}]*\\}').sub(world, "", true).replace('"version":29', '"version":27')
			if not _check(not '"reputation"' in old and '"version":27' in old, "Legacy fixture omits reputation for player and both AI"):
				return
			_write(reputation_path, old)
			if not _check(sim.load_game(reputation_path).is_empty() and _snapshot() == expected, "v27 migration changes only neutral reputation and version; all world bytes and APIs match v29 baseline"):
				return
		else:
			if not _check(sim.save_game(reputation_path).is_empty(), "Save non-neutral v29 world with pending daily observations"):
				return
		_store(reputation_path + ".initial", _snapshot())
	for owner in _owners(_state()):
		if not _check(_routes(owner)[0]["plan"]["airborne"] != null, "Player and both AI have frozen airborne departure quotes at restart"):
			return
		var reputation: Dictionary = owner["reputation"]
		if not _check(reputation["score"] == (0 if mode.ends_with("v27") else -800000) and reputation["flights"] == (0 if mode.ends_with("v27") else 1000), "Every owner retains expected neutral migration or nonzero score and pending counters"):
			return
	main._sync_view()
	main.attention_expanded = true
	await _click(main.navigation[1])
	if not await _reputation_views("initial", not mode.ends_with("v27")):
		return
	var before := _snapshot()
	# Presentation must not mutate the world; page changes render API state.
	await _click(main.navigation[2])
	if not _route_punctuality() or not _check(_snapshot() == before, "Rendering reputation and route punctuality preserves every owner"):
		return
	await _click(main.navigation[1])
	if mode == "ui":
		# First midnight consumes the seeded bad day plus real landings. The
		# next two days contain only observations from actual Rust operations.
		_advance(86400 - 21700)
		main._process(0)
		if not _check(sim.get_financial_summary()["reputation_trend"] == "sinkend", "Bad day's observations lower reputation at midnight") or not await _reputation_views("falling", true):
			return
		var bad: int = _state()["reputation"]["score"]
		var flights: int = sim.get_financial_summary()["punctuality_flights"]
		_advance(2 * 86400)
		main._process(0)
		var summary: Dictionary = sim.get_financial_summary()
		if not _check(summary["punctuality_flights"] > flights and summary["punctuality_percent"] > 90 and _state()["reputation"]["score"] > bad and summary["reputation_trend"] == "steigend", "Actual on-time landings recover reputation without replacing fixture state") or not await _reputation_views("recovering", false):
			return
	else:
		_advance(7 * 86400)
		main._process(0)
		if not await _reputation_views("seven-days", false):
			return
		await _click(main.navigation[2])
		if not _route_punctuality():
			return
		for owner in _owners(_state()):
			if not _check(owner["finance"]["totals"]["flights"] > 0 and owner["reputation"]["score"] != 0, "Player and both AI settle flights and update reputation during continuation"):
				return
		if mode.begins_with("write-"):
			_store(reputation_path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(reputation_path + ".expected"), "Seven-day reader equals uninterrupted serialized player/AI worlds and API snapshots"):
			return
	print("PASS: reputation " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _reputation_fixture() -> bool:
	var sim = main.simulation
	for error in [sim.found_airline("Punctual Audit", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.hire_missing_staff(), sim.schedule_route(1, 1, 120)]:
		if not _check(error.is_empty(), "Prepare reputation fixture: " + error):
			return false
	_advance(21700)
	var world: String = _snapshot()[0]
	var pattern := RegEx.create_from_string('"reputation":\\{[^{}]*\\}')
	if not _check(pattern.search_all(world).size() == 3, "Reputation exists for player and both AI owners"):
		return false
	# Only reputation is a fixture input. All routes, airborne offers, finances,
	# seeds and AI schedules remain untouched; subsequent recovery is simulated.
	world = pattern.sub(world, '"reputation":{"flights":1000,"late":1000,"score":-800000,"trend":-1000}', true)
	_write(probe_path, world)
	return _check(sim.load_game(probe_path).is_empty(), "Load clearly poor reputation with a pending late day for every owner")

func _reputation_views(label: String, warning: bool) -> bool:
	var summary: Dictionary = main.simulation.get_financial_summary()
	var expected := "Pünktlichkeitsruf: %+.1f %% (%s)" % [summary["reputation_percent"], summary["reputation_trend"]]
	if not _check(expected in main.description.text, "Live fleet/airline description renders exact Rust reputation percent and trend: " + expected):
		return false
	var entries: Array = main.simulation.get_attention_snapshot()
	var found := 0
	for i in range(entries.size()):
		if not _check(main.attention_rows.get_child(i).text == entries[i]["text"], "Attention row renders exact Rust text"):
			return false
		if entries[i]["kind"] == "reputation":
			found += 1
			if not _check(entries[i]["page"] == "fleet" and main.attention_rows.get_child(i).is_visible_in_tree(), "Reputation warning targets fleet and is visible"):
				return false
	if not _check(found == (1 if warning else 0), "Reputation warning follows Rust falling/recovery condition"):
		return false
	await _reveal(main.description)
	await _capture("reputation-" + label, Vector2i(root.size))
	return true

func _route_punctuality() -> bool:
	var panel = main.routes_panel
	var row = panel.route_tree.get_root().get_first_child()
	for route in main.simulation.get_routes():
		var otp := "—" if route["punctuality_percent"] < 0 else "%.1f %%" % route["punctuality_percent"]
		if not _check(row.get_tooltip_text(0) == "Pünktlichkeit: %s · %d erfasste Flüge" % [otp, route["punctuality_flights"]] and "Route #%d · Pünktlichkeit: %s · %d erfasste Flüge" % [route["id"], otp, route["punctuality_flights"]] in panel.operations_label.text, "Route tooltip and operations render exact Rust punctuality values"):
			return false
		row = row.get_next()
	return true

func _snapshot() -> Array:
	return super._snapshot() + [main.simulation.get_attention_snapshot(), main.simulation.get_map_airports(), main.simulation.get_flight_results()]
