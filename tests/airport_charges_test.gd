extends "res://tests/insurance_test.gd"
## Airport UI and lossless player/AI continuation in separate Godot processes.
## Reuses the insurance regression's input helpers and byte-preserving API snapshot.

func _run() -> void:
	var mode := "ui"
	var path := ProjectSettings.globalize_path("user://airport-charges.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--airport-mode="):
			mode = arg.trim_prefix("--airport-mode=")
		if arg.begins_with("--airport-save="):
			path = arg.trim_prefix("--airport-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v23", "read-v23", "write-v25", "read-v25"], "Explicit airport mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(path + ".initial"), "Fresh reader exactly restores serialized player/AI world and API"):
			return
	else:
		if not _check(sim.found_airline("Airport Charges", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty(), "Found airline and purchase"):
			return
		main._sync_view()
		await _click(main.navigation[2])
		var panel = main.routes_panel
		# Open the actual destination popup and navigate by keyboard; no signals
		# or direct selector mutations stand in for route UI input.
		await _choose_key(panel.destination, 2)
		await _choose_key(panel.destination, 1)
		if not _planning_view():
			return
		if mode == "ui":
			for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
				root.size = dimensions
				await _reveal(panel.create_button)
				if not _check(root.get_visible_rect().encloses(panel.create_button.get_global_rect()), "Planning action reachable at %s" % dimensions) or not _planning_view():
					return
				await _capture("planning", dimensions)
		root.size = Vector2i(1100, 720)
		await _click(panel.create_button)
		if not _check(sim.get_routes().size() == 1 and sim.hire_missing_staff().is_empty(), "Mouse creates route; staff available"):
			return
		main._sync_view()
		await _click(main.navigation[2])
		await _reveal(panel.route_tree)
		await _mouse(panel.route_tree.global_position + panel.route_tree.get_item_area_rect(panel.route_tree.get_root().get_child(0), 0).get_center())
		await _click(panel.schedule_button)
		if not _check(sim.get_routes()[0]["has_plan"], "Mouse activates selected route schedule: %s selected=%s disabled=%s rect=%s" % [panel.status.text, panel.route_tree.get_selected(), panel.schedule_button.disabled, panel.schedule_button.get_global_rect()]):
			return
		_advance(21940)
		if mode == "ui":
			for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
				root.size = dimensions
				if not await _active_view():
					return
				await _reveal(panel.service_cost)
				await _capture("active", dimensions)
		else:
			var original: String = _snapshot()[0]
			if mode.ends_with("v23"):
				# v23 charged the model/passenger tariff once. Rewrite only each
				# frozen airport amount, retaining every other JSON byte (u64 seeds
				# and fractional journals must never pass through JSON.stringify).
				var pattern := RegEx.create_from_string('"airborne":\\{.*?"totals":\\{[^}]*\\}')
				var config: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://config/economy.json"))
				var matches := pattern.search_all(original)
				if not _check(matches.size() == 3, "Airborne player and both AI owners"):
					return
				for i in range(matches.size() - 1, -1, -1):
					var part: String = matches[i].get_string()
					var totals: Dictionary = JSON.parse_string(part.substr(part.find('"totals":') + 9))
					# Sorted JSON puts competitors before the player's routes.
					var owner: Dictionary = _state()["competitors"][i]["game"] if i < 2 else _state()
					var fleet: Array = owner["fleet"] if i < 2 else owner["fleet"]["aircraft"]
					var model: String = fleet[0]["model_id"]
					var tariff: int = config["models"][model]["airport_base_eur"] + config["airport_eur_per_passenger"] * totals["passengers"]
					if not _check(tariff != totals["airport"], "Legacy quote differs from endpoint tariff"):
						return
					var changed := RegEx.create_from_string('"airport":\\d+').sub(part, '"airport":%d' % tariff)
					original = original.left(matches[i].get_start()) + changed + original.substr(matches[i].get_end())
				original = original.replace('"version":27', '"version":23')
			_write(path, original)
			if not _check(sim.load_game(path).is_empty() and _snapshot()[0] == original.replace('"version":23', '"version":27'), "v23/v27 identity load preserves every world byte and frozen tariff"):
				return
			_store(path + ".initial", _snapshot())
	if mode != "ui":
		if not await _active_view():
			return
		var before := _state()
		var now: int = before["clock"]["elapsed_seconds"]
		var landing := now
		for owner in _owners(before):
			var quote: Dictionary = _routes(owner)[0]["plan"]["airborne"]
			landing = maxi(landing, quote["landing"])
		_advance(landing - now)
		var after := _owners(_state())
		var initial := _owners(before)
		for i in range(initial.size()):
			var quote: Dictionary = _routes(initial[i])[0]["plan"]["airborne"]
			var found := false
			for result in after[i]["finance"]["recent"]:
				if result["landing"] == quote["landing"]:
					found = result["totals"] == quote["totals"]
			if not _check(found, "Owner %d first landing books every frozen departure total" % i):
				return
		# Inspect each owner's next quote after its legacy flight has landed.
		for i in range(initial.size()):
			var state := _state()
			var owner: Dictionary = _owners(state)[i]
			var plan: Dictionary = _routes(owner)[0]["plan"]
			if plan["airborne"] == null:
				var next: int = plan["first_departure"] + plan["completed_flights"] * plan["interval_seconds"]
				_advance(next - state["clock"]["elapsed_seconds"])
				owner = _owners(_state())[i]
			if not _new_tariff(owner):
				return
		_advance(100 * 86400 + now - _state()["clock"]["elapsed_seconds"])
		if not await _active_view():
			return
		if mode.begins_with("write-"):
			_store(path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(path + ".expected"), "100-day reader equals uninterrupted player/AI world and APIs"):
			return
	print("PASS: airport charges " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _planning_view() -> bool:
	var panel = main.routes_panel
	for country in main.simulation.get_countries():
		for airport in main.simulation.get_airports(country["code"]):
			for choice in [panel.origin, panel.destination]:
				for i in range(choice.item_count):
					if choice.get_item_metadata(i) == airport["icao"]:
						if not _check(("Gebühren %d %%" % airport["charge_percent"]) in choice.get_item_text(i), "Airport level equals Rust API"):
							return false
	var q: Dictionary = main.simulation.preview_route(panel.origin.get_selected_metadata(), panel.destination.get_selected_metadata(), panel.aircraft.get_selected_metadata(), 0)
	var expected := "Planung: %d Pax · Kosten %s, davon Flughäfen Start + Ziel %s je Abflug." % [q["estimated_passengers"], main.UI.money(q["estimated_costs_eur"]), main.UI.money(q["estimated_airport_eur"])]
	return _check(q["has_cost_estimate"] and q["estimated_airport_eur"] > 0 and expected in panel.preview.text, "Exact planning passengers/total/airport cost equals Rust API")

func _active_view() -> bool:
	main._sync_view()
	await _click(main.navigation[2])
	var panel = main.routes_panel
	await _reveal(panel.route_tree)
	await _mouse(panel.route_tree.global_position + panel.route_tree.get_item_area_rect(panel.route_tree.get_root().get_child(0), 0).get_center())
	var route: Dictionary = main.simulation.get_routes()[0]
	var expected := "Flughafengebühren Start + Ziel: %s je Abflug" % main.UI.money(route["estimated_airport_eur"])
	return _check(route["estimated_airport_eur"] > 0 and expected in panel.service_cost.text, "Exact active service airport estimate equals Rust API")

func _choose_key(choice: OptionButton, index: int) -> void:
	await _click(choice)
	var current := choice.get_popup().get_focused_item()
	for step in range(absi(index - current)):
		await _key(KEY_DOWN if index > current else KEY_UP)
	await _key(KEY_ENTER)
	_check(choice.selected == index, "Keyboard selects actual airport popup")

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("airport-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real Xvfb render")

# Container reflow after resizing/rebuilding the route grid takes several frames.
func _reveal(control: Control) -> void:
	for frame in range(5):
		await process_frame
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(control)
		parent = parent.get_parent()
	for frame in range(5):
		await process_frame

func _owners(state: Dictionary) -> Array:
	return [state, state["competitors"][0]["game"], state["competitors"][1]["game"]]

func _routes(owner: Dictionary) -> Array:
	return owner["routes"] if owner["routes"] is Array else owner["routes"]["routes"]

func _new_tariff(owner: Dictionary) -> bool:
	var quote: Dictionary = _routes(owner)[0]["plan"]["airborne"]
	var fleet: Array = owner["fleet"] if owner["fleet"] is Array else owner["fleet"]["aircraft"]
	var config: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://config/economy.json"))
	var tariff: int = config["models"][fleet[0]["model_id"]]["airport_base_eur"] + config["airport_eur_per_passenger"] * quote["totals"]["passengers"]
	var expected := 0
	for country in main.simulation.get_countries():
		for airport in main.simulation.get_airports(country["code"]):
			if airport["icao"] in [quote["origin"], quote["destination"]]:
				expected += ceili(tariff * airport["charge_percent"] / 100.0)
	return _check(expected > 0 and quote["totals"]["airport"] == expected, "Next player/AI departure uses new endpoint tariffs")
