extends SceneTree
## Real route controls against the loaded Rust extension; isolated disk saves.


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	main.name_input.text = "Route UI Air"
	await _click(main.found_button)
	await _click(main.navigation[2])
	var panel = main.routes_panel
	if not _check(panel.empty_label.visible and panel.create_button.disabled, "No aircraft cannot create routes"):
		return
	if not _check(panel.origin.get_selected_metadata() == "EDDF" and panel.destination.selected == -1 and "Ziel" in panel.preview.text and not "km" in panel.preview.text, "Default founding at EDDF starts at the home airport with a destination prompt"):
		return
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			_choose(main.model_select, i)
	await _click(main.buy_button)
	await _click(main.buy_button)
	await _click(main.navigation[2])
	if not _check(panel.destination.selected == -1 and panel.create_button.disabled and "Ziel" in panel.preview.text and not "km" in panel.preview.text, "No destination: Create disabled, no distance claims even with aircraft"):
		return
	_choose_airport(panel.destination, "EDDF")
	if not _check(panel.create_button.disabled and "verschieden" in panel.preview.text, "Same airport rejected in preview"):
		return
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == "LTFM":
			_choose(panel.destination, i)
	if not _check(panel.create_button.disabled and "Reichweite überschritten" in panel.preview.text, "Long route rejected in preview"):
		return
	_choose_airport(panel.destination, "EDDM")
	if not _check(not panel.create_button.disabled and "Reichweite ausreichend" in panel.preview.text and "km" in panel.preview.text, "Valid route displays distance and range check"):
		return
	var estimate: Dictionary = main.simulation.preview_route("EDDF", "EDDM", panel.aircraft.get_selected_metadata(), 0)
	if not _check(estimate["has_cost_estimate"] and estimate["estimated_airport_eur"] > 0 and "Start + Ziel" in panel.preview.text and "Gebühren 60 %" in panel.origin.get_item_text(panel.origin.selected), "Rust airport estimate and endpoint levels appear in planning"):
		return
	await _click(main.navigation[0])
	await _click(main.navigation[2])
	if not _check(panel.origin.get_selected_metadata() == "EDDF" and panel.destination.get_selected_metadata() == "EDDM", "Manual endpoints survive tab switches"):
		return
	await _click(panel.create_button)
	for _i in range(3):
		await process_frame
	if not _check(panel.route_tree.get_selected() != null and panel.route_tree.get_selected().get_metadata(0)["id"] == main.simulation.get_routes()[0]["id"] and panel.ticket_input.get_line_edit().has_focus() and not main.simulation.get_routes()[0]["has_plan"], "Created route is selected, unscheduled, and focus lands in its editor"):
		return
	if not _check(panel.ticket_input.get_line_edit().get_global_rect().intersects(root.get_visible_rect()) , "Fare editor in view after creation"):
		return
	if not _check(main.simulation.get_routes().size() == 1 and panel.aircraft.item_count == 1 and panel.route_tree.get_root().get_child_count() == 1, "Created route listed and aircraft removed from available choices"):
		return
	await _click(panel.create_button)
	for _i in range(3):
		await process_frame
	var routes_now: Array = main.simulation.get_routes()
	if not _check(routes_now.size() == 2 and panel.route_tree.get_selected().get_metadata(0)["id"] == routes_now[1]["id"], "Parallel route on same endpoints selects the new route though another was selected"):
		return
	await _click(panel.delete_button)
	var before: Array = main.simulation.get_routes()
	for args in [["XXXX", "EDDM", 2], ["EDDF", "EDDF", 2], ["EDDF", "KJFK", 2], ["LTFM", "LTFJ", 1]]:
		if not _check(not main.simulation.create_route(args[0], args[1], args[2]).is_empty() and main.simulation.get_routes() == before, "Rust rejects invalid routes and duplicate assignments without mutation"):
			return
	var row: TreeItem = panel.route_tree.get_root().get_child(0)
	row.select(0)
	panel.route_tree.item_selected.emit()
	_choose(panel.assignment, 1)
	await _click(panel.assign_button)
	row = panel.route_tree.get_root().get_child(0)
	if not _check(main.simulation.get_routes()[0]["aircraft_ids"] == [1, 2] and panel.aircraft.item_count == 0 and "AM-000001" in row.get_text(3) and "AM-000002" in row.get_text(3), "Adding a second aircraft keeps both assignments and displays both in one route row"):
		return
	_choose(panel.assignment, 0)
	await _click(panel.assign_button)
	if not _check(main.simulation.get_routes()[0]["aircraft_ids"] == [2] and panel.aircraft.get_item_metadata(0) == 1, "Removing one aircraft keeps the other rotation assigned"):
		return
	_choose(panel.assignment, 1)
	await _click(panel.assign_button)
	if not _check(main.simulation.get_routes()[0]["aircraft_ids"].is_empty() and panel.aircraft.item_count == 2, "Removing the last aircraft keeps the empty route"):
		return
	_choose(panel.assignment, 0)
	await _click(panel.assign_button)
	root.size = Vector2i(680, 520)
	await process_frame
	await _reveal(panel.delete_button)
	if not _check(root.get_visible_rect().encloses(panel.delete_button.get_global_rect()), "Route actions reachable at minimum size"):
		return
	await _click(panel.delete_button)
	if not _check(main.simulation.get_routes().is_empty() and panel.empty_label.visible and panel.aircraft.item_count == 2, "Deleting releases assignment and updates empty list"):
		return
	await _click(panel.create_button)
	if not _check(main.simulation.get_routes()[0]["id"] > 2, "Deleted route and internal rotation IDs are not reused"):
		return
	await _click(main.navigation[1])
	var assigned_id: int = main.simulation.get_routes()[0]["aircraft_id"]
	if not _check("Route #%d" % main.simulation.get_routes()[0]["id"] in main.fleet_tree.get_root().get_child(assigned_id - 1).get_text(3), "Fleet displays reserved route"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Fresh Routes"
	await _click(main.found_button)
	await _click(main.navigation[2])
	if not _check(main.simulation.get_routes().is_empty() and panel.empty_label.visible and panel.create_button.disabled, "New game clears routes and stale assignments"):
		return
	_choose_airport(panel.destination, "EDDM")
	await _click(main.new_game_button)
	await _click(main.cancel_founding_button)
	await _click(main.navigation[2])
	if not _check(panel.destination.get_selected_metadata() == "EDDM", "Cancelling New Game keeps the manual draft"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Istanbul Routes"
	for i in range(main.country_select.item_count):
		if main.country_select.get_item_metadata(i) == "TR":
			main.country_select.select(i)
			main.country_select.item_selected.emit(i)
	for i in range(main.airport_select.item_count):
		if main.airport_select.get_item_metadata(i) == "LTFJ":
			main.airport_select.select(i)
	await _click(main.found_button)
	await _click(main.navigation[2])
	if not _check(panel.origin.get_selected_metadata() == "LTFJ" and panel.destination.selected == -1 and panel.create_button.disabled and panel.route_tree.get_selected() == null, "Founding at LTFJ starts the planner at LTFJ and drops the old draft"):
		return
	var save_path := ProjectSettings.globalize_path("user://routes_ui_world.json")
	_choose_airport(panel.destination, "LTFM")
	if not _check(main.simulation.save_game(save_path).is_empty(), "Save LTFJ world"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Frankfurt Again"
	for i in range(main.country_select.item_count):
		if main.country_select.get_item_metadata(i) == "DE":
			main.country_select.select(i)
			main.country_select.item_selected.emit(i)
	for i in range(main.airport_select.item_count):
		if main.airport_select.get_item_metadata(i) == "EDDF":
			main.airport_select.select(i)
	await _click(main.found_button)
	main.save_path = save_path
	main._load_legacy_game()
	await _click(main.navigation[2])
	if not _check(panel.origin.get_selected_metadata() == "LTFJ" and panel.destination.selected == -1 and main.simulation.get_routes().is_empty(), "Loading a world resets the planner to that world's base without changing routes"):
		return
	print("PASS: route UI, distances, range, multiple aircraft, add/remove assignments, delete, small window and new game")
	main.queue_free()
	await process_frame
	quit(0)


func _choose(select: OptionButton, index: int) -> void:
	select.select(index)
	select.item_selected.emit(index)


func _reveal(control: Control) -> void:
	var main = root.get_child(0)
	for i in range(main.fleet_sections.size()):
		if main.fleet_sections[i].is_ancestor_of(control):
			main.fleet_tabs.current_tab = i
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


func _choose_airport(select: OptionButton, code: String) -> void:
	for i in range(select.item_count):
		if select.get_item_metadata(i) == code:
			_choose(select, i)
			return
	assert(false, "Missing airport: " + code)
