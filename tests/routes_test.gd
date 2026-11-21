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
	if not _check(_destination_index(panel, "EDDF") < 0 and panel.destination.item_count > 1, "Origin is excluded from the destination picker"):
		return
	if not _check(panel.destination.get_item_metadata(0) != "" and "km" in panel.destination.get_item_text(0) and "Gebühren" in panel.destination.get_item_text(0) and "(" + panel.destination.get_item_metadata(0) + ")" in panel.destination.get_item_text(0), "Destination rows show code, distance, fees and ICAO"):
		return
	var ranks := _ranks(panel)
	var near: int = ranks["EDDM"]
	var far: int = ranks["LTFM"]
	if not _check(near < far and not "außer Reichweite" in panel.destination.get_item_text(near) and "außer Reichweite" in panel.destination.get_item_text(far), "EDDM is in range and ahead of out-of-range LTFM, which is labelled"):
		return
	var last := -1.0
	var seen_far := false
	for i in range(panel.destination.item_count):
		var option: Dictionary = panel.destination_options[panel.destination.get_item_metadata(i)]
		if not option["in_range"] and not seen_far:
			seen_far = true
			last = -1.0
		if not _check(option["in_range"] != seen_far and option["distance_km"] >= last, "In-range destinations first, nearest first within each group"):
			return
		last = option["distance_km"]
	_choose_airport(panel.destination, "LTFM")
	if not _check(panel.create_button.disabled and "Reichweite überschritten" in panel.preview.text, "Out-of-range selection keeps Create disabled with the explanation"):
		return
	panel.aircraft.select(-1)
	panel.aircraft.item_selected.emit(-1)
	if not _check(panel.destination.get_selected_metadata() == "LTFM" and not "außer Reichweite" in panel.destination.get_item_text(panel.destination.selected) and panel.destination.get_item_metadata(0) != "", "Without an aircraft airports are browsable without range labels and the selection is kept"):
		return
	panel.aircraft.select(0)
	panel.aircraft.item_selected.emit(0)
	panel.destination.select(-1)
	panel._preview()
	if not _check(panel.destination.selected == -1 and "kein Zielflughafen" not in panel.preview.text and "in Reichweite" in panel.preview.text, "In-range hint appears with no destination and nothing is picked automatically"):
		return
	_choose_airport(panel.origin, "HKJK")
	if not _check(panel.destination.selected == -1 and "HKJK" in panel.preview.text and "ATR" in panel.preview.text and "kein Zielflughafen" in panel.preview.text and "größerer Reichweite" in panel.preview.text and "Flotte" in panel.preview.text and panel.create_button.disabled, "Nairobi with an ATR explains that nothing is in range and how to proceed"):
		return
	if not _check(_destination_index(panel, "HKJK") < 0 and panel.destination.get_item_metadata(0) == "FAOR" and "außer Reichweite" in panel.destination.get_item_text(0), "Nairobi list excludes itself, lists FAOR first as out of range"):
		return
	_choose_airport(panel.destination, "FAOR")
	_choose_airport(panel.origin, "EDDF")
	if not _check(panel.destination.get_selected_metadata() == "FAOR" and _destination_index(panel, "EDDF") < 0 and _destination_index(panel, "HKJK") >= 0 and panel.create_button.disabled, "Origin change keeps the destination by ICAO and recomputes the list"):
		return
	_choose_airport(panel.destination, "EDDM")
	_choose_airport(panel.destination, "LTFM")
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
	await _click(main.new_game_dialog.danger_button)
	await process_frame
	await _click(main.navigation[2])
	if not _check(main.simulation.get_routes().is_empty() and panel.empty_label.visible and panel.create_button.disabled, "New game clears routes and stale assignments"):
		return
	if not _check(main.simulation.buy_aircraft("atr72_600").is_empty() and main.simulation.lease_aircraft("a320_200").is_empty(), "Fresh company gets an ATR and a leased A320"):
		return
	await _click(main.navigation[2])
	var atr_index := -1
	var a320_index := -1
	for i in range(panel.aircraft.item_count):
		if "ATR" in panel.aircraft.get_item_text(i):
			atr_index = i
		if "A320" in panel.aircraft.get_item_text(i):
			a320_index = i
	if not _check(atr_index >= 0 and a320_index >= 0, "Fresh company owns an ATR and an A320 for range comparison"):
		return
	_choose_airport(panel.origin, "HKJK")
	_choose(panel.aircraft, atr_index)
	if not _check(panel.destination.selected == -1 and "kein Zielflughafen" in panel.preview.text, "Nairobi with ATR has no destination in range"):
		return
	_choose(panel.aircraft, a320_index)
	var faor: int = _destination_index(panel, "FAOR")
	if not _check(panel.destination.selected == -1 and faor == 0 and not "außer Reichweite" in panel.destination.get_item_text(faor) and not "kein Zielflughafen" in panel.preview.text, "Switching to an A320 puts FAOR in range at the top and picks nothing"):
		return
	_choose_airport(panel.destination, "FAOR")
	_choose(panel.aircraft, atr_index)
	if not _check(panel.destination.get_selected_metadata() == "FAOR" and panel.create_button.disabled and "Reichweite überschritten" in panel.preview.text, "Destination kept by ICAO when its aircraft range class changes"):
		return
	_choose_airport(panel.origin, "EDDF")
	_choose(panel.aircraft, atr_index)
	panel.destination.select(-1)
	panel._preview()
	panel.destination.grab_focus()
	for pressed in [true, false]:
		var key := InputEventKey.new()
		key.keycode = KEY_ENTER
		key.physical_keycode = KEY_ENTER
		key.pressed = pressed
		Input.parse_input_event(key)
		await process_frame
	await process_frame
	if not _check(panel.destination.get_popup().visible and panel.destination.get_popup().item_count == panel.destination.item_count, "Keyboard opens the destination picker at minimum size"):
		return
	panel.destination.get_popup().hide()
	panel.get_parent().get_parent().ensure_control_visible(panel.preview)
	await process_frame
	if not _check(root.get_visible_rect().intersects(panel.preview.get_global_rect()) and panel.preview.text.length() > 0 and panel.destination.focus_mode != Control.FOCUS_NONE, "Destination picker and range explanation reachable at minimum size"):
		return
	_choose_airport(panel.destination, "EDDM")
	await _click(main.new_game_button)
	main.simulation.start()
	var held: float = main.simulation.get_elapsed_seconds()
	await _click(main.cancel_founding_button)
	if not _check(main.current_page == 2 and panel.visible and panel.destination.get_selected_metadata() == "EDDM" and main.new_game_button.has_focus() and main.simulation.is_running() and main.simulation.get_elapsed_seconds() == held, "Cancelling New Game returns to Routes with the draft, focus on New Game and clock held"):
		return
	main.simulation.pause()
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
	await _click(main.new_game_dialog.danger_button)
	await process_frame
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


func _destination_index(panel, code: String) -> int:
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == code:
			return i
	return -1


func _ranks(panel) -> Dictionary:
	var ranks := {}
	for i in range(panel.destination.item_count):
		ranks[panel.destination.get_item_metadata(i)] = i
	return ranks
