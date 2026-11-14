extends SceneTree
## Real cabin/fare controls; optional write/read modes require separate processes.

var main

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--cabin-mode="):
			mode = arg.trim_prefix("--cabin-mode=")
		if arg.begins_with("--cabin-save="):
			path = arg.trim_prefix("--cabin-save=")
	if not _check(mode in ["ui", "write-v16", "read-v16", "write-v17", "read-v17"] and (mode == "ui" or not path.is_empty()), "Valid mode and explicit isolated restart path"):
		return
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var panel = main.routes_panel
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load via UI in a fresh process"):
			return
	else:
		main.name_input.text = "Cabin UI Air"
		await _click(main.found_button)
		await _click(main.navigation[1])
		for i in range(main.model_select.item_count):
			if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
				main.model_select.select(i)
				main.model_select.item_selected.emit(i)
		await _click(main.buy_button)
		var plane: Dictionary = sim.get_fleet()[0]
		if not _check(not main.cabin_button.disabled and main.cabin_input.editable and main.UI.money(plane["cabin_fee_eur"]) in main.cabin_button.text, "Parked aircraft offers refit and Rust fee"):
			return
		if not mode.ends_with("v16"):
			var balance: int = sim.get_airline_info()["capital_eur"]
			main.cabin_input.value = 8
			await _click(main.cabin_button)
			plane = sim.get_fleet()[0]
			if not _check(plane["business_seats"] == 8 and plane["economy_seats"] == 56 and sim.get_airline_info()["capital_eur"] == balance - plane["cabin_fee_eur"] and main.fleet_status.text == "Kabine geändert." and "Y 56 / J 8" in main.fleet_tree.get_root().get_child(0).get_text(1), "Refit changes displayed layout and debits exactly the quoted fee"):
				return
			await _click(main.cabin_button)
			if not _check(sim.get_airline_info()["capital_eur"] == balance - plane["cabin_fee_eur"], "Unchanged layout is free"):
				return
		await _click(main.navigation[2])
		await _click(panel.create_button)
		_select_route()
		panel.ticket_input.value = 73
		await _click(panel.ticket_button)
		if not mode.ends_with("v16"):
			panel.business_input.value = 181
			await _click(panel.business_button)
		if not _check(sim.get_routes()[0]["ticket_eur"] == 73 and (mode.ends_with("v16") or sim.get_routes()[0]["business_ticket_eur"] == 181), "Independent fare buttons update Rust route"):
			return
		if not mode.ends_with("v16"):
			sim.hire_missing_staff()
			panel.refresh()
			_select_route()
			panel.delay_input.value = 1
			await _click(panel.schedule_button)
			sim.start()
			sim.advance(60.0)
			sim.pause()
			await _click(main.navigation[1])
			if not _check(sim.get_fleet()[0]["status"] == "in_flight" and main.cabin_button.disabled and not main.cabin_input.editable, "Airborne refit controls are blocked"):
				return
			var before: Array = sim.get_fleet()
			if not _check(not sim.set_cabin_layout(1, 9).is_empty() and sim.get_fleet() == before, "Rust also rejects airborne refit without mutation"):
				return
			sim.start()
			sim.advance(3000.0)
			sim.pause()
			main._refresh_fleet()
			if not _check(sim.get_fleet()[0]["status"] == "ground" and not main.cabin_button.disabled, "Landed aircraft offers refit again"):
				return
	if mode.ends_with("v16"):
		if not _check(sim.get_fleet()[0]["business_seats"] == 0 and sim.get_fleet()[0]["economy_seats"] == 72 and sim.get_routes()[0]["ticket_eur"] == 73, "v16 restart preserves economy fare and all-economy layout"):
			return
		await _click(main.navigation[1])
		if not _check(main.cabin_input.value == 0 and "Y 72 / J 0" in main.fleet_tree.get_root().get_child(0).get_text(1), "Migrated cabin displayed as all-economy"):
			return
		await _click(main.navigation[2])
		_select_route()
		if not _check(panel.ticket_input.value == 73, "Migrated route displays saved economy fare"):
			return
	else:
		if not _check(sim.get_fleet()[0]["business_seats"] == 8 and sim.get_fleet()[0]["economy_seats"] == 56 and sim.get_routes()[0]["ticket_eur"] == 73 and sim.get_routes()[0]["business_ticket_eur"] == 181, "v17 keeps mixed cabin and both fares"):
			return
		await _click(main.navigation[2])
		_select_route()
		var result: Dictionary = sim.get_flight_results()[0]
		var row: TreeItem = panel.results_tree.get_root().get_child(0)
		if not _check(result["business_passengers"] > 0 and result["economy_passengers"] > 0 and row.get_text(2) == "Y %d/%d · J %d/%d" % [result["economy_passengers"], result["economy_seats"], result["business_passengers"], result["business_seats"]] and row.get_tooltip_text(3) == "Economy: %s · Business: %s" % [main.UI.money(result["economy_revenue_eur"]), main.UI.money(result["business_revenue_eur"])], "Results show both class loads and revenue"):
			return
		if not _check(result["economy_revenue_eur"] == result["economy_passengers"] * 73 and result["business_revenue_eur"] == result["business_passengers"] * 181 and result["revenue_eur"] == result["economy_revenue_eur"] + result["business_revenue_eur"] + result["cargo_revenue_eur"], "Per-class receipts match the separate fares"):
			return
		if not _check(panel.ticket_input.value == 73 and panel.business_input.value == 181, "Selected route displays both saved fares"):
			return
	if mode.begins_with("write-"):
		main.save_path = path
		await _click(main.save_button)
		if not _check(main.save_status.text == "Spielstand gespeichert.", "Save restart fixture via UI"):
			return
		if mode.ends_with("v17") or mode.ends_with("v16"):
			var version := 16 if mode.ends_with("v16") else 17
			var contents := FileAccess.get_file_as_string(path).replace('"version":28', '"version":%d' % version)
			var service := RegEx.create_from_string('"service_level":"[a-z]+",|,"service_level":"[a-z]+"|"service":-?\\d+,|,"service":-?\\d+')
			var file := FileAccess.open(path, FileAccess.WRITE)
			file.store_string(service.sub(contents, "", true))
			file.close()
		if mode.ends_with("v16"):
			# Strip only new integer fields; do not round u64 seeds through JSON floats.
			var contents := FileAccess.get_file_as_string(path)
			var fields := RegEx.new()
			fields.compile('"business_(seats|ticket_eur|passengers|revenue)":\\d+,|,"business_(seats|ticket_eur|passengers|revenue)":\\d+')
			contents = fields.sub(contents, "", true)
			var fixture: Dictionary = JSON.parse_string(contents)
			if not _check(fixture["version"] == 16 and not "business_seats" in contents and not "business_ticket_eur" in contents, "Fixture has v16 schema without cabin fields"):
				return
			var file := FileAccess.open(path, FileAccess.WRITE)
			file.store_string(contents)
			file.close()
	if mode == "ui":
		await _click(main.navigation[1])
		await _click(main.maintenance_button)
		if not _check(sim.get_fleet()[0]["status"] == "maintenance" and main.cabin_button.disabled and not main.cabin_input.editable, "Maintenance blocks cabin controls"):
			return
		var before: Array = sim.get_fleet()
		if not _check(not sim.set_cabin_layout(1, 9).is_empty() and sim.get_fleet() == before, "Rust rejects hangar refit without mutation"):
			return
		for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
			root.size = dimensions
			for control in [main.cabin_input, main.cabin_button]:
				await _reveal(control)
				if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Fleet cabin control reachable at %s" % dimensions):
					return
			await _click(main.navigation[2])
			_select_route()
			for control in [panel.ticket_input, panel.ticket_button, panel.business_input, panel.business_button, panel.results_tree]:
				await _reveal(control)
				if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Class fare/results control reachable at %s" % dimensions):
					return
			await _click(main.navigation[1])
	print("PASS: cabin UI/refit fee, ground/air/hangar restrictions, separate fares, per-class results; mode=" + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _select_route() -> void:
	main.routes_panel.route_tree.get_root().get_child(0).select(0)
	main.routes_panel.route_tree.item_selected.emit()

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
