extends SceneTree
## Real keyboard/mouse budget input plus separate-process v19/v20 restart phases.
const DAY := 86400.0
var main

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/marketing-ui.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--marketing-mode="):
			mode = arg.trim_prefix("--marketing-mode=")
		if arg.begins_with("--marketing-save="):
			path = arg.trim_prefix("--marketing-save=")
	if not _check(mode in ["ui", "write-v19", "read-v19", "write-v20", "read-v20"], "Explicit mode"):
		return
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var legacy := mode.ends_with("v19")
	if mode.begins_with("read-"):
		if not _check(sim.load_game(path).is_empty(), "Load in fresh process"):
			return
		if not _check(_snapshot() == FileAccess.open(path + ".initial", FileAccess.READ).get_var(), "Exact state restored"):
			return
	else:
		if not _check(sim.found_airline("Marketing UI", "DE", "EDDF").is_empty(), "Found airline"):
			return
		main._sync_view()
		await _click(main.navigation[4])
		if not await _budget(73333):
			return
		if not _check(sim.get_marketing()["awareness"] == 0, "No instant awareness"):
			return
		_advance(12.345)
		if not _check(sim.save_game(path).is_empty(), "Save mid-month"):
			return
		if legacy:
			# This legacy fixture predates marketing: no historical costs or ledger changes.
			# Regenerate from a zero-spend world before introducing marketing.
			if not _check(sim.found_airline("Marketing UI", "DE", "EDDF").is_empty(), "Reset true legacy scenario"):
				return
			_advance(3.345)
			sim.save_game(path)
			var contents := FileAccess.get_file_as_string(path).replace('"version":20', '"version":19')
			contents = RegEx.create_from_string('"marketing":\\{[^}]*\\},|,"marketing":\\{[^}]*\\}').sub(contents, "", true)
			contents = contents.replace('"marketing_costs":[],', '').replace(',"marketing_costs":[]', '')
			var file := FileAccess.open(path, FileAccess.WRITE)
			file.store_string(contents)
			file.close()
			if not _check(sim.load_game(path).is_empty() and sim.get_marketing()["budget_eur"] == 0 and sim.get_marketing()["awareness"] == 0, "v19 neutral migration"):
				return
		FileAccess.open(path + ".initial", FileAccess.WRITE).store_var(_snapshot())
	if legacy and not await _budget(30000):
		return
	_advance(20.125)
	if not await _budget(0):
		return
	var before: float = sim.get_marketing()["awareness"]
	_advance(60)
	if not _check(sim.get_marketing()["awareness"] < before and sim.get_marketing()["trend"] < 0, "Gradual decay"):
		return
	if mode.begins_with("write-"):
		FileAccess.open(path + ".expected", FileAccess.WRITE).store_var(_snapshot())
	elif mode.begins_with("read-"):
		if not _check(_snapshot() == FileAccess.open(path + ".expected", FileAccess.READ).get_var(), "Restart continuation identical"):
			return
	if not _verify_ui() or not await _layouts():
		return
	print("PASS: marketing " + mode)
	quit(0)

func _snapshot() -> Array:
	var sim = main.simulation
	return [sim.get_marketing(), sim.get_financial_summary(), sim.get_finance_series(0), sim.get_marketing_history(), sim.get_competitors()]

func _advance(days: float) -> void:
	main.simulation.start()
	main.simulation.set_speed(16)
	_check(main.simulation.advance(days * DAY / 16.0), "Advance")
	main.simulation.pause()

func _budget(amount: int) -> bool:
	main._sync_view()
	await _click(main.navigation[4])
	var panel = main.finances_panel
	panel.refresh(true)
	var edit: LineEdit = panel.marketing_budget.get_line_edit()
	await _reveal(edit)
	await _mouse(edit.get_global_rect().get_center())
	edit.select_all()
	for character in str(amount):
		var event := InputEventKey.new()
		event.pressed = true
		event.unicode = character.unicode_at(0)
		event.keycode = character.unicode_at(0)
		Input.parse_input_event(event)
		await process_frame
		event.pressed = false
		Input.parse_input_event(event)
		await process_frame
	await _click(panel.marketing_apply)
	return _check(main.simulation.get_marketing()["budget_eur"] == amount and panel.marketing_result.text == "Marketingbudget übernommen.", "Real input commits budget")

func _verify_ui() -> bool:
	var panel = main.finances_panel
	panel.refresh(true)
	var marketing: Dictionary = main.simulation.get_marketing()
	var summary: Dictionary = main.simulation.get_financial_summary()
	if not _check(("Bekanntheit: %.2f / 100" % marketing["awareness"]) in panel.marketing_status.text and ("%+.4f Punkte" % marketing["trend"]) in panel.marketing_status.text, "Displayed awareness and trend equal Rust"):
		return false
	var rows: Array = main.simulation.get_marketing_history()
	if not _check(panel.marketing_history.get_root().get_child_count() == rows.size(), "Archive rows rendered"):
		return false
	var total := 0
	for i in range(rows.size()):
		total += int(rows[i]["amount_eur"])
		if not _check(panel.marketing_history.get_root().get_child(i).get_text(1) == main.UI.money(rows[i]["amount_eur"]), "Archive cost rendered"):
			return false
	var series_cost := 0
	for sample in main.simulation.get_finance_series(0):
		series_cost += int(sample["costs"])
	return _check(total > 0 and total == summary["marketing_eur"] and series_cost == summary["costs_eur"] and summary["costs_eur"] == total, "Idle-company cost series, archive and summary reconcile")

func _layouts() -> bool:
	var panel = main.finances_panel
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		for control in [panel.marketing_budget, panel.marketing_apply, panel.marketing_status, panel.marketing_history]:
			await _reveal(control)
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Marketing reachable at %s" % dimensions):
				return false
	return true

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
	await _mouse(button.get_global_rect().get_center())

func _mouse(position: Vector2) -> void:
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
