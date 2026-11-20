extends SceneTree
## First-route schedule recovery through real controls: crew block -> Staff -> back, short interval, retry, two legs.

var capture_dir := ""
var size := Vector2i(680, 520)


func _shot(name: String) -> void:
	if capture_dir.is_empty():
		return
	for _i in range(5):
		await process_frame
	await RenderingServer.frame_post_draw
	root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [name, size.x, size.y]))


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
		if arg.begins_with("--size="):
			var parts := arg.trim_prefix("--size=").split("x")
			size = Vector2i(int(parts[0]), int(parts[1]))
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	main.name_input.text = "Recovery Air"
	await _click(main.found_button)
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	await _click(main.buy_button)
	await _click(main.navigation[2])
	var panel = main.routes_panel
	var empty_button: Button = panel.results_empty.find_children("*", "Button", true, false)[0]
	if not _check(empty_button.text == "Route erstellen" and empty_button.visible, "Empty results offer route creation without routes"):
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
	root.size = size
	var route_id: int = main.simulation.get_routes()[0]["id"]
	panel.ticket_input.value = 100
	await _click(panel.ticket_button)
	if not _check(main.simulation.get_routes()[0]["ticket_eur"] == 100, "Fare set through UI"):
		return
	panel.delay_input.value = 3
	panel.interval_input.value = 1
	panel.refresh_operations()
	if not _check(panel.schedule_button.disabled and panel.crew_button.visible and panel.crew_button.is_visible_in_tree(), "Crew-blocked route disables activation and offers Staff"):
		return
	if not _check(panel.maintenance_label.visible and not panel.maintenance_label.text.is_empty(), "Crew shortage notice shown"):
		return
	if not _check("Personal" in empty_button.text, "Empty results lead to Staff when crew-blocked"):
		return
	(panel.get_parent().get_parent() as ScrollContainer).ensure_control_visible(panel.results_empty)
	await _shot("recovery-results-crew-blocked")
	await _click(empty_button)
	for _i in range(10):
		await process_frame
	if not _check(main.current_page == 5 and root.gui_get_focus_owner() == main.staff_panel.hire_missing_button, "Empty-results action opens Staff focused on hiring"):
		return
	await _click(main.navigation[2])
	if not _check(panel.crew_button.is_visible_in_tree(), "Crew action visible after returning"):
		return
	(panel.get_parent().get_parent() as ScrollContainer).ensure_control_visible(panel.crew_button)
	await _shot("recovery-crew-blocked")
	await _click(panel.crew_button)
	for _i in range(10):
		await process_frame
	if not _check(main.current_page == 5 and root.gui_get_focus_owner() == main.staff_panel.hire_missing_button, "Crew action opens Staff focused on hiring"):
		return
	var scroll := main.staff_panel.get_parent().get_parent() as ScrollContainer
	await _shot("recovery-staff")
	if not _check(scroll.get_global_rect().encloses(main.staff_panel.hire_missing_button.get_global_rect()), "Hire button in view at 680x520"):
		return
	if not _check(main.simulation.get_routes()[0]["staff_status"] != "", "Navigation alone does not hire"):
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
	var field: Control = panel.interval_input.get_meta("field_help")
	var route_scroll := panel.get_parent().get_parent() as ScrollContainer
	if not _check(not main.simulation.get_routes()[0]["has_plan"] and field.text != panel.interval_input.get_meta("field_help_text") and "mindestens" in field.text and field.is_visible_in_tree(), "Short interval rejected with minimum beside field"):
		return
	await _shot("recovery-invalid-interval")
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
	if not _check(not empty_button.visible and "nach der ersten Landung" in panel.results_empty.find_children("*", "Label", true, false)[1].text, "Active empty state explains results arrive after landing"):
		return
	await _shot("recovery-activated")
	await _click(main.start_button)
	var guard := 0
	while main.simulation.get_flight_results().size() < 2 and guard < 100:
		main.simulation.advance(600.0)
		guard += 1
	var results: Array = main.simulation.get_flight_results()
	if not _check(results.size() >= 2 and results[0]["origin"] != results[1]["origin"], "Outbound and return legs completed"):
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


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
