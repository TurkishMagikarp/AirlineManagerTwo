extends SceneTree
## First-route schedule recovery through real controls: crew block -> Staff -> back, short interval, retry, two legs.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	TranslationServer.set_locale("de")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	main.name_input.text = "Recovery Air"
	await _click(main.found_button)
	var sim: RefCounted = main.simulation
	if not _check(sim.get_airline_info()["airport_icao"] == "EDDF", "Standard founding at EDDF"):
		return
	await _click(main.navigation[2])
	var panel = main.routes_panel
	var empty_button: Button = panel.results_empty_button
	if not _check(empty_button.visible and empty_button.text == "Flugzeug beschaffen", "Without aircraft, empty results link to procurement"):
		return
	await _click(empty_button)
	if not _check(main.current_page == 1, "Results procurement action opens Fleet"):
		return
	for frame in range(5):
		await process_frame
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	await _click(main.buy_button)
	if not _check(sim.get_fleet().size() == 1 and sim.get_fleet()[0]["model_id"] == "atr72_600", "Bought the ATR through Fleet"):
		return
	await _click(main.navigation[2])
	if not _check(panel.origin.get_selected_metadata() == "EDDF" and panel.destination.selected == -1, "Route planner opens at the airline home and asks for a destination"):
		return
	if not _check(empty_button.text == "Route planen" and empty_button.visible, "Empty results offer route planning once an aircraft is available, got " + empty_button.text):
		return
	await _click(empty_button)
	if not _check(panel.destination.has_focus(), "Route planning action focuses destination selection"):
		return
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == "EDDM":
			panel.destination.select(i)
			panel.destination.item_selected.emit(i)
	await _click(panel.create_button)
	for _i in range(5):
		await process_frame
	if not _check(main.simulation.get_routes().size() == 1, "Route created: " + panel.status.text + panel.preview.text):
		return
	var route: Dictionary = sim.get_routes()[0]
	var route_id: int = route["id"]
	if not _check(route["origin_icao"] == "EDDF" and route["destination_icao"] == "EDDM" and panel._route().get("id", 0) == route_id and panel.ticket_input.get_line_edit().has_focus(), "Creation selects EDDF–EDDM and reveals its editor"):
		return
	root.size = Vector2i(680, 520)
	for frame in range(4):
		await process_frame
	var row: TreeItem = panel.route_tree.get_selected()
	row.deselect(0)
	panel.route_tree.item_selected.emit()
	if not _check(empty_button.visible and empty_button.text == "Route auswählen", "With no selection, empty results direct the player to the register"):
		return
	await _click(empty_button)
	if not _check(panel.route_tree.has_focus(), "Route selection action focuses the register"):
		return
	row.select(0)
	panel.route_tree.item_selected.emit()
	await _click(panel.assign_button)
	if not _check(sim.get_routes()[0]["aircraft_ids"].is_empty() and empty_button.visible and empty_button.text == "Flugzeug zuweisen", "An unassigned route offers its available assignment control"):
		return
	await _click(empty_button)
	if not _check(panel.assignment.has_focus(), "Assignment action focuses the aircraft picker"):
		return
	await _click(panel.assign_button)
	if not _check(sim.get_routes()[0]["aircraft_ids"] == route["aircraft_ids"], "The same aircraft can be assigned again"):
		return
	panel.ticket_input.value = 100
	await _click(panel.ticket_button)
	if not _check(main.simulation.get_routes()[0]["ticket_eur"] == 100, "Fare set through UI"):
		return
	panel.delay_input.value = 3
	panel.interval_input.value = 1
	panel.refresh_operations()
	if not _check(panel.schedule_button.disabled and panel.crew_button.visible and panel.crew_button.is_visible_in_tree(), "Crew-blocked route disables activation and offers Staff"):
		return
	if not _check(panel.maintenance_label.visible and panel.maintenance_label.text == route["staff_status"] and "ATR 72" in route["staff_status"], "Routes displays the simulation’s qualified crew shortage"):
		return
	if not _check("Personal" in empty_button.text, "Empty results lead to Staff when crew-blocked"):
		return
	await _click(empty_button)
	for _i in range(10):
		await process_frame
	if not _check(main.current_page == 5 and root.gui_get_focus_owner() == main.staff_panel.hire_missing_button, "Empty-results action opens Staff focused on hiring"):
		return
	await _click(main.navigation[2])
	if not _check(panel.crew_button.is_visible_in_tree() and panel.crew_button.focus_mode == Control.FOCUS_ALL, "Schedule crew action is keyboard-reachable after returning"):
		return
	panel.crew_button.grab_focus()
	await _key(KEY_ENTER)
	for _i in range(10):
		await process_frame
	if not _check(main.current_page == 5 and root.gui_get_focus_owner() == main.staff_panel.hire_missing_button, "Crew action opens Staff focused on hiring"):
		return
	var scroll := main.staff_panel.get_parent().get_parent() as ScrollContainer
	var staff = main.staff_panel
	if not _check(scroll.get_global_rect().encloses(staff.hire_missing_button.get_global_rect()) and scroll.get_global_rect().encloses(staff.shortage_label.get_global_rect()), "Staff shortage and hire button are in view at 680x520"):
		return
	if not _check(staff.aircraft_tree.get_selected().get_metadata(0) == route["aircraft_id"], "Staff highlights the route’s aircraft"):
		return
	if not _check(sim.get_routes()[0]["staff_status"] != "" and not sim.get_routes()[0]["has_plan"], "Navigation leaves the route unstaffed and inactive"):
		return
	await _click(main.staff_panel.hire_missing_button)
	await _click(main.navigation[2])
	if not _check(panel._route().get("id", 0) == route_id and panel.delay_input.value == 3 and panel.interval_input.value == 1, "Same route and entered values after return"):
		return
	if not _check(not panel.schedule_button.disabled and not panel.crew_button.visible and not main.simulation.get_routes()[0]["has_plan"], "Activation enabled, still inactive"):
		return
	if not _check("Flugplan" in empty_button.text and "einrichten" in empty_button.text, "Eligible empty state reveals schedule editor"):
		return
	await _click(empty_button)
	for _i in range(2):
		await process_frame
	if not _check(root.gui_get_focus_owner() == panel.delay_input.get_line_edit(), "Empty-results action focuses schedule editor"):
		return
	await _click(panel.schedule_button)
	for _i in range(4):
		await process_frame
	var field: Label = panel.interval_input.get_meta("field_help")
	var route_scroll := panel.get_parent().get_parent() as ScrollContainer
	if not _check(not main.simulation.get_routes()[0]["has_plan"] and field.text != panel.interval_input.get_meta("field_help_text") and "mindestens" in field.text and field.is_visible_in_tree(), "Short interval rejected with minimum beside field"):
		return
	var view := route_scroll.get_global_rect()
	if not _check(view.encloses(field.get_global_rect()) and view.encloses(panel.schedule_button.get_global_rect()) and view.encloses(panel.interval_input.get_global_rect()), "Field, validation and retry visible at 680x520"):
		return
	if not _check(root.gui_get_focus_owner() == panel.interval_input.get_line_edit(), "Interval keeps keyboard focus"):
		return
	await _click(main.navigation[0])
	if not _check(not main.simulation.get_routes()[0]["has_plan"], "Navigation leaves schedule inactive"):
		return
	await _click(main.navigation[2])
	panel.interval_input.value = 120
	await _click(panel.schedule_button)
	for _i in range(3):
		await process_frame
	if not _check(main.simulation.get_routes()[0]["has_plan"] and field.text == panel.interval_input.get_meta("field_help_text"), "Corrected interval activates plan and clears error"):
		return
	if not _check(not empty_button.visible and "nach der ersten Landung" in panel.results_empty_text.text, "Active empty state explains results arrive after landing"):
		return
	for jump in range(3):
		if sim.get_flight_results().size() >= 2:
			break
		await _click(main.jump_buttons[1])
		while main.jump_in_progress:
			await process_frame
	var results: Array = sim.get_flight_results()
	if not _check(results.size() >= 2 and results.any(func(f): return f["route_id"] == route_id and f["origin"] == "EDDF" and f["destination"] == "EDDM") and results.any(func(f): return f["route_id"] == route_id and f["origin"] == "EDDM" and f["destination"] == "EDDF"), "Outbound and return legs completed after using the clock controls"):
		return
	if not _check(panel.results_tree.visible and not panel.results_empty.visible and panel.results_tree.get_root().get_child_count() >= 2, "Routes displays the completed flights"):
		return
	print("PASS: first-route schedule recovery through Staff and interval retry")
	quit(0)


func _click(button: Button) -> void:
	var main = root.get_child(0)
	for i in range(main.fleet_sections.size()):
		if main.fleet_sections[i].is_ancestor_of(button):
			main.fleet_tabs.current_tab = i
	await process_frame
	var parent := button.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(button)
		parent = parent.get_parent()
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame


func _key(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
