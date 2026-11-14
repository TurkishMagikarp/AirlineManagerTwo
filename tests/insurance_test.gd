extends SceneTree
## Real Finances input and lossless separate-process restart probes.
var main
var probe_path := ""
var capture_dir := ""
var initial_world := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := ProjectSettings.globalize_path("user://insurance-test.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--insurance-mode="):
			mode = arg.trim_prefix("--insurance-mode=")
		if arg.begins_with("--insurance-save="):
			path = arg.trim_prefix("--insurance-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v22", "read-v22", "write-v23", "read-v23"], "Explicit mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var legacy := mode.ends_with("v22")
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen." and _snapshot() == _read(path + ".initial"), "Fresh process restores full serialized world and API exactly"):
			return
	else:
		if not _check(sim.found_airline("Insurance UI", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty() and sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.hire_missing_staff().is_empty() and sim.schedule_route(1, 1, 120).is_empty(), "Set up player and both normal AI airlines"):
			return
		# Seed 1288 produces a first-landing incident for Insurance UI / aircraft 1.
		# Preserve JSON integers and doubles byte-for-byte; never JSON.stringify a world.
		var initial: String = _snapshot()[0]
		_write(path, RegEx.create_from_string('"seed":\\d+').sub(initial, '"seed":1288', true))
		var seed_error: String = sim.load_game(path)
		if not _check(seed_error.is_empty(), "Load deterministic incident seed: " + seed_error):
			return
		main._sync_view()
		initial_world = _snapshot()[0]
		if mode == "ui":
			if not await _controls():
				return
		elif legacy:
			# A real v22 world has neither policies nor insurance journals. Disable
			# every owner before departure so migration changes only policy defaults.
			var neutral: String = _snapshot()[0]
			neutral = RegEx.create_from_string('"coverage":"basic"').sub(neutral, '"coverage":"none"', true)
			neutral = RegEx.create_from_string('"annual_premium":\\d+').sub(neutral, '"annual_premium":0', true)
			_write(path, neutral)
			if not _check(sim.load_game(path).is_empty(), "Neutral old-world fixture"):
				return
			_advance(120)
			var original: String = _snapshot()[0]
			var old := _without_insurance(original).replace('"version":27', '"version":22')
			_write(path, old)
			if not _check(not '"insurance"' in old and not '"insurance_costs"' in old and sim.load_game(path).is_empty() and _without_insurance(_snapshot()[0]) == _without_insurance(original), "v22 migration preserves all money, history, fleet and frozen departure offers byte-for-byte"):
				return
		else:
			if not await _coverage(1, true):
				return
			_advance(120)
			var landing: int = _state()["routes"]["routes"][0]["plan"]["airborne"]["landing"]
			_advance(landing - 120)
			if not _check(sim.save_game(path).is_empty(), "Save mid-repair paid claim"):
				return
		if mode != "ui":
			_store(path + ".initial", _snapshot())
	if mode != "ui":
		var state := _state()
		if legacy:
			for owner in [state, state["competitors"][0]["game"], state["competitors"][1]["game"]]:
				if not _check(owner["insurance"]["coverage"] == "none" and owner["insurance"]["recent"].is_empty(), "v22 migrates every owner to no coverage / no claims"):
					return
			if not _check(state["routes"]["routes"][0]["plan"]["airborne"] != null, "v22 includes a frozen running departure"):
				return
		elif not _check(state["fleet"]["aircraft"][0]["maintenance"] != null and state["insurance"]["recent"][0]["insurer"] > 0 and state["competitors"][0]["game"]["insurance"]["coverage"] == "basic", "v23 contains mid-repair aircraft, paid claim and basic-covered AI"):
			return
		if not await _views():
			return
		_advance(100 * 86400 + 123)
		if not await _views():
			return
		if mode.begins_with("write-"):
			_store(path + ".expected", _snapshot())
		elif not _check(_snapshot() == _read(path + ".expected"), "Full player/AI world and API equal uninterrupted run after another 100 days"):
			return
	else:
		if not await _layouts():
			return
	print("PASS: insurance " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _controls() -> bool:
	await _click(main.navigation[4])
	if not _check(main.simulation.get_insurance()["coverage"] == "none", "Initial no coverage"):
		return false
	for index in [1, 2, 0]:
		if not await _coverage(index, index != 2) or not await _views():
			return false
	# First landing without cover yields a real uninsured repair.
	_advance(120)
	var landing: int = _state()["routes"]["routes"][0]["plan"]["airborne"]["landing"]
	_advance(landing - 120)
	if not _check(main.simulation.get_insurance()["uninsured"] > 0, "Real uninsured incident"):
		return false
	if not await _views():
		return false
	_write(probe_path, initial_world)
	if not _check(main.simulation.load_game(probe_path).is_empty(), "Restore deterministic pre-departure world"):
		return false
	main._sync_view()
	if not await _coverage(1, true):
		return false
	_advance(landing)
	if not _check(main.simulation.get_insurance()["deductible"] > 0 and main.simulation.get_insurance()["claims"][0]["insurer"] > 0, "Real covered incident with deductible and paid insurer claim") or not await _views():
		return false
	_advance(21617)
	# The paid claim remains recorded while its repair is pending, with
	# fractional premium and surcharge. Only policy fields are fixture inputs.
	var text: String = _snapshot()[0]
	text = text.replace('"loading_bps":0', '"loading_bps":1500')
	_write(probe_path, text)
	if not _check(main.simulation.load_game(probe_path).is_empty(), "Load nonzero surcharge fixture"):
		return false
	main._sync_view()
	# Settle to this instant first, so switching off must preserve the exact remainder.
	if not _check(main.simulation.set_insurance("basic").is_empty(), "Settle fractional premium"):
		return false
	if not _check(main.simulation.get_insurance()["premium"] > 0, "Nonzero premium booked while repair is pending"):
		return false
	var visit: Dictionary = _state()["fleet"]["aircraft"][0]["maintenance"]
	var before: Dictionary = _state()["insurance"]
	if not _check(before["remainder"] > 0 and before["loading_bps"] > 0 and not before["recent"].is_empty(), "Nonvacuous claims/remainder/surcharge fixture"):
		return false
	if not await _coverage(0, false):
		return false
	var after: Dictionary = _state()["insurance"]
	for field in ["recent", "remainder", "loading_bps", "year_claims"]:
		if not _check(after[field] == before[field], "Disabling preserves " + field):
			return false
	if not _check(_state()["fleet"]["aircraft"][0]["maintenance"] == visit, "Disabling preserves pending repair obligation"):
		return false
	var api: Dictionary = main.simulation.get_insurance()
	return _check(not main.simulation.set_insurance("invalid").is_empty() and main.simulation.get_insurance() == api, "Invalid policy is transactional") and await _views()

func _coverage(index: int, mouse: bool) -> bool:
	await _click(main.navigation[4])
	var choice: OptionButton = main.finances_panel.insurance_choice
	await _click(choice)
	var popup := choice.get_popup()
	if mouse:
		await _mouse(Vector2(popup.position) + Vector2(popup.size.x / 2.0, popup.size.y * (index + 0.5) / 3.0))
	else:
		# Mouse-opened popups can have no keyboard focus (-1). Navigate
		# from the actual focused item, including that initial empty position.
		var current: int = popup.get_focused_item()
		for step in range(absi(index - current)):
			await _key(KEY_DOWN if index > current else KEY_UP)
		await _key(KEY_ENTER)
	return _check(main.simulation.get_insurance()["coverage"] == ["none", "basic", "full"][index] and main.finances_panel.insurance_result.text == "Deckung geändert.", "Real policy selection index=%s mouse=%s api=%s result=%s popup=%s" % [index, mouse, main.simulation.get_insurance()["coverage"], main.finances_panel.insurance_result.text, popup.visible])

func _views() -> bool:
	main._sync_view()
	await _click(main.navigation[4])
	var panel = main.finances_panel
	panel.refresh(true)
	var api: Dictionary = main.simulation.get_insurance()
	if not _check(panel.insurance_choice.selected == ["none", "basic", "full"].find(api["coverage"]), "Coverage matches API"):
		return false
	for quote in [["Basis", "basic"], ["Voll", "full"]]:
		var prefix: String = quote[1]
		var expected := "%s: %s/Jahr · Selbstbehalt %.2f %% · Limit %.2f %% des Zeitwerts" % [quote[0], main.UI.money(api[prefix + "_annual"]), api[prefix + "_deductible_bps"] / 100.0, api[prefix + "_limit_bps"] / 100.0]
		if not _check(expected in panel.insurance_status.text, "Exact API quote / deductible / limit: " + prefix):
			return false
	if not _check(("Schadenaufschlag %.2f %%" % (api["loading_bps"] / 100.0)) in panel.insurance_status.text, "Exact surcharge from API"):
		return false
	for pair in [["Prämien", "premium"], ["Selbstbehalte", "deductible"], ["Unversicherte Reparaturen", "uninsured"], ["Aktuelle Jahresprämie", "annual"]]:
		if not _check((pair[0] + ": " + main.UI.money(api[pair[1]])) in panel.insurance_status.text, "Exact API money: " + pair[0]):
			return false
	var lines: PackedStringArray = panel.insurance_status.text.split("\n")
	if not _check(lines.size() == 7 + api["claims"].size(), "Incident list has exactly API row count"):
		return false
	for i in range(api["claims"].size()):
		var c: Dictionary = api["claims"][i]
		var expected := "%s · AM-%06d · %s · Reparatur %s · Eigenanteil %s · Versicherer %s" % [c["time"], c["aircraft"], c["kind"], main.UI.money(c["repair"]), main.UI.money(c["paid"]), main.UI.money(c["insurer"])]
		if not _check(lines[7 + i] == expected, "Incident row exactly matches Rust API"):
			return false
	return true

func _layouts() -> bool:
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		if not await _coverage(1, true) or not await _coverage(2, false) or not await _coverage(0, true) or not await _views():
			return false
		var panel = main.finances_panel
		await _reveal(panel.insurance_choice)
		if not _check(root.get_visible_rect().encloses(panel.insurance_choice.get_global_rect()), "Selector reachable at %s" % dimensions):
			return false
		await _capture("coverage", dimensions)
		# The incident list is rendered text. Scroll its last row into the page
		# viewport and click it through the real input path at every window size.
		var parent = panel.insurance_status.get_parent()
		while parent != null:
			if parent is ScrollContainer:
				parent.scroll_vertical += int(panel.insurance_status.get_global_rect().end.y - parent.get_global_rect().end.y + 24)
			parent = parent.get_parent()
		await process_frame
		var point: Vector2 = panel.insurance_status.get_global_rect().end - Vector2(40, 12)
		if not _check(root.get_visible_rect().has_point(point), "Last incident row reachable at %s" % dimensions):
			return false
		await _mouse(point)
		await _capture("incidents", dimensions)
	return true

func _capture(label: String, dimensions: Vector2i) -> void:
	if not capture_dir.is_empty():
		await RenderingServer.frame_post_draw
		_check(root.get_texture().get_image().save_png(capture_dir.path_join("insurance-%s-%dx%d.png" % [label, dimensions.x, dimensions.y])) == OK, "Real render capture")

func _without_insurance(text: String) -> String:
	text = RegEx.create_from_string('"insurance":\\{(?:[^{}]|\\{[^{}]*\\})*\\},?').sub(text, "", true)
	return RegEx.create_from_string(',?"insurance_costs":\\[[^\\[\\]]*\\]').sub(text, "", true)

func _advance(seconds: float) -> void:
	main.simulation.start()
	_check(main.simulation.advance(seconds), "Advance Rust clock")
	main.simulation.pause()

func _snapshot() -> Array:
	var sim = main.simulation
	_check(sim.save_game(probe_path).is_empty(), "Serialize full world")
	return [FileAccess.get_file_as_string(probe_path), sim.get_insurance(), sim.get_airline_info(), sim.get_fleet(), sim.get_routes(), sim.get_operations(), sim.get_financial_summary(), sim.get_route_finances(), sim.get_route_flight_page(1, 0), sim.get_finance_series(0), sim.get_competitors(), sim.get_active_flights()]

func _state() -> Dictionary:
	return JSON.parse_string(_snapshot()[0])["state"]

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

func _mouse(position: Vector2) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _key(code: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = code
		event.physical_keycode = code
		event.pressed = pressed
		root.push_input(event)
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
