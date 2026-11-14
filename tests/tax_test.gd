extends SceneTree
## Real Finances input; restart modes run in separate processes via the PS wrapper.
const DAY := 86400.0
var main
var capture_dir := ""
var probe_path := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/tax-ui.json")
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--tax-mode="):
			mode = arg.trim_prefix("--tax-mode=")
		if arg.begins_with("--tax-save="):
			path = arg.trim_prefix("--tax-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v18", "read-v18", "write-v22", "read-v22"], "Explicit tax test mode"):
		return
	probe_path = path + ".probe"
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var legacy := mode.ends_with("v18")
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load through UI in fresh process"):
			return
		var initial: Dictionary = FileAccess.open(path + ".initial", FileAccess.READ).get_var()
		if not _check(_snapshot() == initial, "Exact summary, assessments, series and balance restored"):
			return
	else:
		if not _check(sim.found_airline("Tax UI", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty() and sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.set_ticket_price(1, 110).is_empty() and sim.hire_missing_staff().is_empty(), "Tax scenario setup"):
			return
		main._sync_view()
		await _click(main.navigation[4])
		if not await _verify_ui() or not _check(sim.get_financial_summary()["tax_year"] == 0 and sim.get_tax_history().is_empty(), "Jahr 0 before first close"):
			return
		if not legacy:
			# Idle staff creates a genuine loss without fabricating ledger entries.
			_advance(365)
			var loss: Dictionary = sim.get_tax_history()[0]
			if not _check(loss["year"] == 1 and loss["profit_eur"] < 0 and loss["tax_eur"] == 0 and loss["loss_eur"] == -loss["profit_eur"], "Zero-tax loss year creates carryforward") or not await _verify_ui():
				return
			if mode == "ui" and not await _layouts("loss"):
				return
		if not _check(sim.schedule_route(1, 1, 120).is_empty(), "Start profitable service"):
			return
		_advance(180)
		if not _check(sim.save_game(path).is_empty(), "Write mid-year save"):
			return
		if legacy:
			# No JSON reserialization: preserve Rust u64/nanosecond values exactly.
			var contents := FileAccess.get_file_as_string(path).replace('"version":30', '"version":18')
			contents = RegEx.create_from_string('"tax":\\{[^}]*\\},|,"tax":\\{[^}]*\\}').sub(contents, "", true)
			contents = contents.replace('"taxes":[],', '').replace(',"taxes":[]', '')
			if not _check(JSON.parse_string(contents)["version"] == 18 and not '"tax"' in contents and not '"taxes"' in contents, "True v18 fixture without tax state, including AI"):
				return
			var file := FileAccess.open(path, FileAccess.WRITE)
			file.store_string(contents)
			file.close()
			var error: String = sim.load_game(path)
			if not _check(error.is_empty(), "Load migration reference: " + error):
				return
		_store(path + ".initial", _snapshot())
	if legacy:
		if not _check(sim.get_tax_history().is_empty() and sim.get_financial_summary()["tax_eur"] == 0 and sim.get_financial_summary()["tax_loss_eur"] == 0 and sim.get_financial_summary()["profit_eur"] > 0, "v18 keeps past profit without carryforward or back-taxation"):
			return
	var before: Dictionary = sim.get_financial_summary()
	_advance(185)
	var summary: Dictionary = sim.get_financial_summary()
	var assessment: Dictionary = sim.get_tax_history()[0]
	if legacy:
		var post_load: int = summary["profit_eur"] + summary["tax_eur"] - before["profit_eur"]
		if not _check(assessment["year"] == 1 and assessment["profit_eur"] == post_load and assessment["tax_eur"] > 0 and assessment["tax_eur"] == int((post_load * int(summary["tax_rate_bps"]) + 5000) / 10000), "Migration taxes only post-load profit at next year-end"):
			return
	else:
		var previous: Dictionary = sim.get_tax_history()[1]
		if not _check(assessment["year"] == 2 and assessment["profit_eur"] > previous["loss_eur"] and assessment["loss_eur"] == 0 and assessment["tax_eur"] > 0 and assessment["tax_eur"] == int(((int(assessment["profit_eur"]) - int(previous["loss_eur"])) * int(assessment["rate_bps"]) + 5000) / 10000), "Profitable year consumes carryforward before tax"):
			return
	if not await _verify_ui():
		return
	if mode.begins_with("write-"):
		_store(path + ".expected", _snapshot())
	elif mode.begins_with("read-"):
		var expected: Dictionary = FileAccess.open(path + ".expected", FileAccess.READ).get_var()
		if not _check(_snapshot() == expected, "Fresh-process continuation equals uninterrupted reference, including AI"):
			return
		if not _check(sim.save_game(path).is_empty() and JSON.parse_string(FileAccess.get_file_as_string(path))["version"] == 30, "Resave as v30"):
			return
	elif not await _layouts("profit"):
		return
	print("PASS: tax note/table/chart, loss carryforward, migration and restart; mode=" + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _advance(days: int) -> void:
	main.simulation.start()
	main.simulation.advance(days * DAY)
	main.simulation.pause()

func _snapshot() -> Dictionary:
	var sim = main.simulation
	_check(sim.save_game(probe_path).is_empty(), "Serialize exact state for restart comparison")
	return {"save": FileAccess.get_file_as_string(probe_path), "summary": sim.get_financial_summary(), "taxes": sim.get_tax_history(), "series": sim.get_finance_series(0), "airline": sim.get_airline_info(), "competitors": sim.get_competitors()}

func _store(path: String, value: Dictionary) -> void:
	var file := FileAccess.open(path, FileAccess.WRITE)
	file.store_var(value)
	file.close()

func _verify_ui() -> bool:
	main._sync_view()
	await _click(main.navigation[4])
	var panel = main.finances_panel
	panel.refresh(true)
	var summary: Dictionary = main.simulation.get_financial_summary()
	var rates: Dictionary = JSON.parse_string(FileAccess.get_file_as_string("res://config/tax.json"))
	if not _check(summary["tax_rate_bps"] == rates["country_rates_bps"]["DE"], "German home rate equals configuration"):
		return false
	var expected := "Ertragsteuer · letzter Abschluss Jahr %d: %s · Verlustvortrag: %s · Heimatsteuersatz: %.2f %% · Steuern insgesamt: %s." % [summary["tax_year"], main.UI.money(summary["tax_year_eur"]), main.UI.money(summary["tax_loss_eur"]), summary["tax_rate_bps"] / 100.0, main.UI.money(summary["tax_eur"])]
	if not _check(expected in panel.note.text, "Tax note equals Rust summary: " + panel.note.text):
		return false
	var rows: Array = main.simulation.get_tax_history()
	if not _check(panel.tax_history.get_root().get_child_count() == rows.size(), "One UI row per assessment, including zero-tax years"):
		return false
	var tax_sum := 0
	for i in range(rows.size()):
		var a: Dictionary = rows[i]
		tax_sum += int(a["tax_eur"])
		var values := [str(a["year"]), main.UI.money(a["profit_eur"]), "%.2f %%" % (a["rate_bps"] / 100.0), main.UI.money(a["tax_eur"]), main.UI.money(a["loss_eur"])]
		for column in range(values.size()):
			if not _check(panel.tax_history.get_root().get_child(i).get_text(column) == values[column], "Assessment table cell equals Rust"):
				return false
	if not await _select_tax():
		return false
	var chart_tax := 0
	for sample in panel.chart.samples:
		chart_tax += int(sample["tax"])
	return _check(panel.chart.field == "tax" and panel.chart.samples == main.simulation.get_finance_series(0) and chart_tax == tax_sum and tax_sum == summary["tax_eur"], "Selected tax chart reconciles with assessment archive and summary")

func _select_tax() -> bool:
	var select: OptionButton = main.finances_panel.chart_select
	await _click(select)
	var popup := select.get_popup()
	# The kit shadow moves the actual menu panel inside the popup window.
	var menu_rect: Rect2 = popup.get_child(0, true).get_global_rect()
	if not _check(popup.visible, "Chart selector opens through mouse input"):
		return false
	var position := Vector2(popup.position) + menu_rect.position + Vector2(menu_rect.size.x / 2.0, menu_rect.size.y * 4.5 / 5.0)
	await _mouse(position)
	return _check(select.selected == 4, "Mouse selects Ertragsteuer chart series")

func _layouts(stage: String) -> bool:
	var panel = main.finances_panel
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		for control in [panel.note, panel.tax_history, panel.chart_select, panel.chart]:
			await _reveal(control)
			if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Tax control reachable by scroll at %s" % dimensions):
				return false
		if not await _select_tax():
			return false
		if not capture_dir.is_empty():
			for entry in [[panel.note, "note"], [panel.tax_history, "table"], [panel.chart_select, "selector"], [panel.chart, "chart"]]:
				await _reveal(entry[0])
				await RenderingServer.frame_post_draw
				if not _check(root.get_texture().get_image().save_png(capture_dir.path_join("tax-%s-%s-%dx%d.png" % [stage, entry[1], dimensions.x, dimensions.y])) == OK, "Capture tax UI"):
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
