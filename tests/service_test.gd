extends SceneTree
## Real input plus optional isolated, separate-process v17/v18 restart phases.
var main
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var mode := "ui"
	var path := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--service-mode="):
			mode = arg.trim_prefix("--service-mode=")
		if arg.begins_with("--service-save="):
			path = arg.trim_prefix("--service-save=")
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	if not _check(mode in ["ui", "write-v17", "read-v17", "write-v18", "read-v18"] and (mode == "ui" or not path.is_empty()), "Explicit restart mode and isolated save path"):
		return
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	var panel = main.routes_panel
	var legacy := mode.ends_with("v17")
	var frozen: Array
	if mode.begins_with("read-"):
		main.save_path = path
		await _click(main.founding_load_button)
		if not _check(main.save_status.text == "Spielstand geladen.", "Load with real button in a fresh process"):
			return
		frozen = FileAccess.open(path + ".flight", FileAccess.READ).get_var()
	else:
		if not _check(panel.service_select.disabled, "No route disables service selector"):
			return
		sim.found_airline("Service UI", "DE", "EDDF")
		sim.buy_aircraft("a320_200")
		sim.set_cabin_layout(1, 12)
		sim.create_route("EDDF", "LTFM", 1)
		sim.borrow_money(8000000)
		if not _check(sim.buy_aircraft("atr72_600").is_empty() and sim.create_route("EDDF", "EDDM", 2).is_empty(), "Second independent route"):
			return
		sim.hire_missing_staff()
		main._sync_view()
		await _click(main.navigation[2])
		_select_route()
		if not _check(not panel.service_select.disabled and panel.service_select.selected == 1 and panel.service_select.item_count == 3, "Standard default and three choices"):
			return
		panel.delay_input.value = 1
		panel.interval_input.value = 360
		await _click(panel.schedule_button)
		if not _check(sim.get_routes()[0]["has_plan"], "Real schedule button activates route"):
			return
		if not legacy:
			for i in [2, 0, 1, 2]:
				if not await _service(i):
					return
				var route: Dictionary = sim.get_routes()[0]
				if not _check(route["service_level"] == ["basic", "standard", "premium"][i] and panel.service_select.selected == i and main.UI.money(route["estimated_service_eur"]) in panel.service_cost.text and sim.get_routes()[1]["service_level"] == "standard" and (route["estimated_service_eur"] > 0 if i == 2 else route["estimated_service_eur"] < 0 if i == 0 else route["estimated_service_eur"] == 0), "Selector i=%s selected=%s route=%s second=%s cost=%s estimate=%s" % [i, panel.service_select.selected, route["service_level"], sim.get_routes()[1]["service_level"], panel.service_cost.text, route["estimated_service_eur"]]):
					return
		sim.start()
		sim.advance(60.0)
		sim.pause()
		frozen = sim.get_active_flights()
		if not _check(frozen.size() == 1 and (frozen[0]["service_eur"] == 0 if legacy else frozen[0]["service_eur"] > 0), "Departure snapshots: flights=%s operations=%s" % [frozen, sim.get_operations()]):
			return
		if not legacy:
			for i in [0, 1, 2, 0]:
				if not await _service(i):
					return
				if not _check(sim.get_active_flights() == frozen, "Every service choice preserves departed quote"):
					return
		if mode.begins_with("write-"):
			main.save_path = path
			await _click(main.navigation[0])
			await _click(main.save_button)
			if not _check(main.save_status.text == "Spielstand gespeichert.", "Save airborne flight through real button"):
				return
			var file := FileAccess.open(path + ".flight", FileAccess.WRITE)
			file.store_var(frozen)
			file.close()
			# Both restart fixtures use their named historical format.
			var tax_free := FileAccess.get_file_as_string(path)
			tax_free = RegEx.create_from_string('"tax":\\{[^}]*\\},|,"tax":\\{[^}]*\\}').sub(tax_free, "", true)
			tax_free = tax_free.replace('"taxes":[],', '').replace(',"taxes":[]', '')
			file = FileAccess.open(path, FileAccess.WRITE)
			file.store_string(tax_free.replace('"version":25', '"version":18'))
			file.close()
			if legacy:
				# Preserve u64 seeds and nanoseconds: never reserialize the save as GDScript JSON.
				var contents := FileAccess.get_file_as_string(path).replace('"version":18', '"version":17')
				var fields := RegEx.create_from_string('"service_level":"[a-z]+",|,"service_level":"[a-z]+"|"service":-?\\d+,|,"service":-?\\d+')
				contents = fields.sub(contents, "", true)
				if not _check(JSON.parse_string(contents)["version"] == 17 and not '"service"' in contents and not '"service_level"' in contents, "True v17 fixture without service fields"):
					return
				file = FileAccess.open(path, FileAccess.WRITE)
				file.store_string(contents)
				file.close()
			var fixture_text := FileAccess.get_file_as_string(path)
			if not _check(JSON.parse_string(fixture_text)["version"] == (17 if legacy else 18) and not '"tax"' in fixture_text and not '"taxes"' in fixture_text, "Named historical service format without tax fields"):
				return
			print("PASS: service fixture " + mode)
			quit(0)
			return
	await _click(main.navigation[2])
	_select_route()
	if not _check(sim.get_active_flights() == frozen and sim.get_routes()[0]["service_level"] == ("standard" if legacy else "basic") and sim.get_routes()[1]["service_level"] == "standard" and panel.service_select.selected == (1 if legacy else 0), "Airborne quote frozen; independent route settings survive restart"):
		return
	sim.start()
	sim.advance(5.0 * 3600.0)
	sim.pause()
	if not _check(sim.get_flight_results().size() == 1 and sim.get_flight_results()[0] == frozen[0], "First landing retains complete departed quote"):
		return
	sim.start()
	sim.advance(86400.0)
	sim.pause()
	var results: Array = sim.get_route_flight_page(1, 0)
	if not _check(results.size() > 1 and (results[0]["service_eur"] == 0 if legacy else results[0]["service_eur"] < 0), "Next departures use selected Standard/Basic service"):
		return
	main._sync_view()
	panel.refresh_operations()
	if not _check(main.UI.money(sim.get_routes()[0]["actual_service_eur"]) in panel.service_cost.text, "Routes actual cost refreshes from archive"):
		return
	for index in range(panel.results_tree.get_root().get_child_count()):
		var result: Dictionary = sim.get_flight_results()[index]
		if not _check(("Bordservice (± Standard): " + main.UI.money(result["service_eur"])) in panel.results_tree.get_root().get_child(index).get_tooltip_text(4), "Routes flight cost tooltip matches archived flight"):
			return
	await _click(main.navigation[4])
	var finances = main.finances_panel
	finances.refresh(true)
	finances.route_tree.get_root().get_child(0).select(0)
	finances.route_tree.item_selected.emit()
	var service_total := 0
	for index in range(results.size()):
		service_total += int(results[index]["service_eur"])
		if not _check(("Bordservice (± Standard): " + main.UI.money(results[index]["service_eur"])) in finances.flights_tree.get_root().get_child(index).get_tooltip_text(6), "Finances tooltip matches archived flight"):
			return
	if not _check(service_total == sim.get_financial_summary()["service_eur"] and ("Bordservice (Kosten / Ersparnis gegenüber Standard, in Gesamtkosten enthalten): %s." % main.UI.money(service_total)) in finances.note.text and main.UI.money(service_total) in finances.detail.text, "Finances service total and route detail equal archive sum"):
		return
	if mode == "ui":
		for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
			root.size = dimensions
			await _click(main.navigation[2])
			_select_route()
			for control in [panel.service_select, panel.service_cost, panel.schedule_button, panel.results_tree]:
				await _reveal(control)
				if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Routes service control reachable at %s" % dimensions):
					return
			if not await _service(1) or not await _service(0):
				return
			if not _check(sim.get_routes()[0]["service_level"] == "basic", "Selector usable at %s" % dimensions):
				return
			if not await _capture(panel.service_select, "routes", dimensions) or not await _capture(panel.service_cost, "service-cost", dimensions):
				return
			await _click(main.navigation[4])
			for control in [finances.note, finances.flights_tree]:
				await _reveal(control)
				if not _check(root.get_visible_rect().encloses(control.get_global_rect()), "Finances service line/archive reachable at %s" % dimensions):
					return
			if not await _capture(finances.note, "finances", dimensions):
				return
	if mode == "read-v17":
		await _click(main.navigation[0])
		await _click(main.save_button)
		if not _check(main.save_status.text == "Spielstand gespeichert." and JSON.parse_string(FileAccess.get_file_as_string(path))["version"] == 25, "Migrated v17 resaves as v25"):
			return
	print("PASS: real service input, frozen costs, archive/Finances agreement and layout; mode=" + mode)
	main.queue_free()
	await process_frame
	quit(0)

func _select_route() -> void:
	main.routes_panel.route_tree.get_root().get_child(0).select(0)
	main.routes_panel.route_tree.item_selected.emit()

func _service(index: int) -> bool:
	await _click(main.routes_panel.service_select)
	if not _check(main.routes_panel.service_select.get_popup().visible, "Service popup opens with mouse input"):
		return false
	var popup: PopupMenu = main.routes_panel.service_select.get_popup()
	# Equal-height, single-line items; click their centers inside the popup window.
	var position := Vector2(popup.position) + Vector2(popup.size.x / 2.0, popup.size.y * (index + 0.5) / 3.0)
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = position
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

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
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _capture(control: Control, panel: String, dimensions: Vector2i) -> bool:
	if capture_dir.is_empty():
		return true
	await _reveal(control)
	await RenderingServer.frame_post_draw
	return _check(root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [panel, dimensions.x, dimensions.y])) == OK, "Screenshot write")

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
