extends SceneTree
## Real Finances mouse/keyboard input; write/read modes require separate processes.
const InsuranceFixture = preload("res://tests/insurance_fixture.gd")
const DAY := 86400.0
var main
var probe_path := ""
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/fuel-ui.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--fuel-mode="):
			mode = arg.trim_prefix("--fuel-mode=")
		if arg.begins_with("--fuel-save="):
			path = arg.trim_prefix("--fuel-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v20", "read-v20", "write-v22", "read-v22"], "Explicit mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var legacy := mode.ends_with("v20")
	if mode == "ui":
		for share in range(3):
			for term in range(3):
				if not _found() or not await _sign(share, term):
					return
		if not await _blocked_offers(path):
			return
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load through UI in fresh process"):
			return
		if not _check(_snapshot() == FileAccess.open(path + ".initial", FileAccess.READ).get_var(), "Full state restored byte-identically, including player/AI contracts and history"):
			return
	else:
		if not _found():
			return
		if not legacy and not await _sign(2, 0):
			return
		if not _check(sim.buy_aircraft("atr72_600").is_empty() and sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.set_ticket_price(1, 110).is_empty() and sim.hire_missing_staff().is_empty() and sim.schedule_route(1, 1, 120).is_empty(), "Set up profitable departures"):
			return
		_advance(3.345 if legacy else 21.345)
		if not _check(sim.save_game(path).is_empty(), "Save mid-contract with completed and airborne quotes"):
			return
		if legacy:
			var original := FileAccess.get_file_as_string(path)
			if not _verify_owners(false):
				return
			# Pre-contract world: all effects are zero. Strip additive fields with
			# text edits to preserve Rust u64/nanosecond encodings exactly.
			var fixture := _strip_fields(original).replace('"version":24', '"version":20')
			if not _check(JSON.parse_string(fixture)["version"] == 20 and not '"fuel_contract' in fixture, "True v20 fixture for every owner and historical quote"):
				return
			_write(path, fixture)
			if not _check(sim.load_game(path).is_empty(), "Load v20 migration reference"):
				return
			if not _check(InsuranceFixture.without_policy(_snapshot()[0]) == InsuranceFixture.without_policy(original), "v20 migration preserves prior state and history alongside neutral insurance migration"):
				return
		_store(path + ".initial", _snapshot())
	if not _verify_owners(not legacy) or not _verify_ui():
		return
	if not legacy:
		if not _billing() or not await _layouts("active"):
			return
	else:
		if not await _sign(1, 0):
			return
	# Continue beyond the player's 90-day expiry and through further AI reviews.
	_advance(100.125)
	if not _check(not sim.get_fuel_contract()["active"] and sim.get_fuel_contract()["can_start"], "Expired contract offers renewal") or not _verify_ui():
		return
	if mode.begins_with("write-"):
		_store(path + ".expected", _snapshot())
	elif mode.begins_with("read-"):
		if not _check(_snapshot() == FileAccess.open(path + ".expected", FileAccess.READ).get_var(), "Restart continuation equals uninterrupted world, active terms, AI, cumulative effect and frozen quotes"):
			return
		if not _check(sim.save_game(path).is_empty() and JSON.parse_string(FileAccess.get_file_as_string(path))["version"] == 24, "Resave as v24"):
			return
	if not await _layouts("expired"):
		return
	print("PASS: fuel contracts " + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _found() -> bool:
	return _check(main.simulation.found_airline("Fuel UI", "DE", "EDDF").is_empty(), "Found fresh scenario")

func _snapshot() -> Array:
	var sim = main.simulation
	_check(sim.save_game(probe_path).is_empty(), "Serialize whole world")
	return [FileAccess.get_file_as_string(probe_path), sim.get_fuel_contract(), sim.get_financial_summary(), sim.get_finance_series(0), sim.get_competitors(), sim.get_active_flights()]

func _verify_owners(active: bool) -> bool:
	var state: Dictionary = JSON.parse_string(_snapshot()[0])["state"]
	var owners: Array = [state]
	for ai in state["competitors"]:
		owners.append(ai["game"])
	if not _check(owners.size() == 3, "Player and both AI companies covered"):
		return false
	for owner in owners:
		if not _check((owner["fuel_contract"] != null) == active, "Every owner has expected contract state"):
			return false
		if active:
			var contract: Dictionary = owner["fuel_contract"]
			if not _check(contract["ends_at"] > state["clock"]["elapsed_seconds"] and contract["locked_cents"] > 0 and owner["finance"]["totals"]["fuel_contract_effect"] != 0, "Active player/AI terms and nonzero cumulative difference"):
				return false
		elif not _check(owner["finance"]["totals"]["fuel_contract_effect"] == 0, "Legacy world has no historical contract billing"):
			return false
	return true

func _sign(share_index: int, term_index: int) -> bool:
	main._sync_view()
	await _click(main.navigation[4])
	var panel = main.finances_panel
	panel.refresh(true)
	var before: Dictionary = main.simulation.get_fuel_contract()
	await _pick(panel.fuel_share, share_index)
	await _pick(panel.fuel_term, term_index)
	await _reveal(panel.fuel_start)
	panel.fuel_start.grab_focus()
	await _key(KEY_SPACE, root)
	var after: Dictionary = main.simulation.get_fuel_contract()
	var locked := int(ceil(before["market_cents"] * (10000 + before["premium_bps"]) / 10000.0))
	if not _check(after["active"] and after["share"] == [25, 50, 75][share_index] and after["locked_cents"] == locked and after["remaining_seconds"] == [3, 6, 12][term_index] * int(before["month_days"]) * int(DAY) and panel.fuel_result.text == "Treibstoffvertrag abgeschlossen.", "Mouse selection and keyboard signing: share=%s term=%s selected=%s/%s before=%s after=%s result=%s" % [share_index, term_index, panel.fuel_share.selected, panel.fuel_term.selected, before, after, panel.fuel_result.text]):
		return false
	var snapshot := _snapshot()
	await _click(panel.fuel_start)
	if not _check(panel.fuel_start.disabled and _snapshot() == snapshot and not main.simulation.start_fuel_contract(25, 3).is_empty() and _snapshot() == snapshot, "Second contract blocked by UI and Rust without mutation"):
		return false
	return _verify_ui()

func _blocked_offers(path: String) -> bool:
	if not _found():
		return false
	var original: String = _snapshot()[0]
	var capital: int = main.simulation.get_airline_info()["capital_eur"]
	# A test-only negative endowment keeps the opening ledger consistent; the
	# real Rust clock then moves this distressed company into restructuring.
	# Airline fields need not be adjacent in the serializer.
	var position := original.find('"name":"Fuel UI"')
	var balance_position := original.find('"capital_eur":', position)
	var fixture := original.left(balance_position) + original.substr(balance_position).replace('"capital_eur":%d' % capital, '"capital_eur":-1')
	position = fixture.rfind('"opening_balance":')
	fixture = fixture.left(position) + fixture.substr(position).replace('"opening_balance":%d' % capital, '"opening_balance":-1')
	_write(path, fixture)
	if not _check(main.simulation.load_game(path).is_empty(), "Load distressed endowment fixture"):
		return false
	for stage in ["Finanznot", "Restrukturierung"]:
		main._sync_view()
		await _click(main.navigation[4])
		main.finances_panel.refresh(true)
		var snapshot := _snapshot()
		await _click(main.finances_panel.fuel_start)
		if not _check(stage in main.simulation.get_financing()["status"] and not main.simulation.get_fuel_contract()["can_start"] and main.finances_panel.fuel_start.disabled and _snapshot() == snapshot and not main.simulation.start_fuel_contract(25, 3).is_empty() and _snapshot() == snapshot, stage + " offer blocked in UI and Rust"):
			return false
		if stage == "Finanznot":
			_advance(31)
	return true

func _billing() -> bool:
	# Compare each retained settled quote against the same physical fuel at
	# departure market price. Weekly prices are read from Rust, not reimplemented.
	var state: Dictionary = JSON.parse_string(_snapshot()[0])["state"]
	var contract: Dictionary = state["fuel_contract"]
	var quotes: Array = state["finance"]["recent"]
	var rates: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://config/economy.json"))["models"]["atr72_600"]
	var reference: RefCounted = ClassDB.instantiate("SimulationClock")
	var positive := false
	var negative := false
	for flight in quotes:
		var departure: int = int(flight["landing"]) - int(main.simulation.get_operations()[0]["flight_seconds"])
		reference.found_airline("Market reference", "DE", "EDDF")
		reference.start()
		reference.advance(float(departure))
		var market: int = reference.get_fuel_contract()["market_cents"]
		var quantity := (int(flight["landing"]) - departure) * int(rates["fuel_kg_per_hour"]) + int(rates["fuel_cycle_kg"]) * 3600
		var full_market := int(ceil(quantity * market / 360000.0))
		var blend: int = market * (100 - int(contract["share_percent"])) + int(contract["locked_cents"]) * int(contract["share_percent"])
		var billed := int(ceil(quantity * blend / 36000000.0))
		var effect: int = flight["totals"]["fuel_contract_effect"]
		if not _check(flight["totals"]["fuel"] == billed and effect == billed - full_market, "Settled fuel and signed effect equal blended bill minus market bill"):
			return false
		positive = positive or effect > 0
		negative = negative or effect < 0
	return _check(quotes.size() > 3 and positive and negative and main.simulation.get_fuel_contract()["effect_eur"] == state["finance"]["totals"]["fuel_contract_effect"], "Multiple departures include premium costs and savings; cumulative difference exposed unchanged")

func _verify_ui() -> bool:
	var fuel: Dictionary = main.simulation.get_fuel_contract()
	main.finances_panel.refresh(true)
	var text: String = main.finances_panel.fuel_status.text
	var expected := "Markt: %d ct/kg · Aufschlag: %.2f %% · Monat: %d Tage · Vertragseffekt seit Gründung: %s" % [fuel["market_cents"], fuel["premium_bps"] / 100.0, fuel["month_days"], main.UI.money(fuel["effect_eur"])]
	return _check(expected in text and (("Aktiv: %d %% zu %d ct/kg · Rest: %d Sekunden" % [fuel["share"], fuel["locked_cents"], fuel["remaining_seconds"]]) in text if fuel["active"] else "Kein aktiver Vertrag." in text), "Displayed market, premium, cumulative signed effect, active terms and expiry match Rust")

func _layouts(stage: String) -> bool:
	main._sync_view()
	await _click(main.navigation[4])
	var panel = main.finances_panel
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		for control in [panel.fuel_share, panel.fuel_term, panel.fuel_start, panel.fuel_status]:
			await _reveal(control)
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Fuel control reachable at %s: %s" % [dimensions, control.get_global_rect()]):
				return false
		# Exercise selectors at every size, including the smallest viewport.
		await _pick(panel.fuel_share, 1)
		await _pick(panel.fuel_term, 2)
		if not _check(panel.fuel_share.get_selected_id() == 50 and panel.fuel_term.get_selected_id() == 12, "Options usable at every viewport size"):
			return false
		if not capture_dir.is_empty():
			await _reveal(panel.fuel_status)
			await RenderingServer.frame_post_draw
			if not _check(root.get_texture().get_image().save_png(capture_dir.path_join("fuel-%s-%dx%d.png" % [stage, dimensions.x, dimensions.y])) == OK, "Capture rendered Finances"):
				return false
	return true

func _advance(days: float) -> void:
	main.simulation.start()
	main.simulation.set_speed(16)
	_check(main.simulation.advance(days * DAY / 16.0), "Advance Rust clock")
	main.simulation.pause()

func _strip_fields(text: String) -> String:
	text = RegEx.create_from_string('"fuel_contract":(null|\\{[^}]*\\}),|,"fuel_contract":(null|\\{[^}]*\\})').sub(text, "", true)
	return RegEx.create_from_string('"fuel_contract_effect":-?\\d+,|,"fuel_contract_effect":-?\\d+').sub(text, "", true)

func _write(path: String, text: String) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_string(text)
	file.close()

func _store(path: String, value: Array) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_var(value)
	file.close()

func _pick(option: OptionButton, index: int) -> void:
	await _click(option)
	var popup := option.get_popup()
	_check(popup.visible, "Mouse opens options")
	var position := Vector2(popup.position) + Vector2(popup.size.x / 2.0, popup.size.y * (index + 0.5) / 3.0)
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
