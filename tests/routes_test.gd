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
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			_choose(main.model_select, i)
	await _click(main.buy_button)
	await _click(main.buy_button)
	await _click(main.navigation[2])
	_choose(panel.destination, 0)
	if not _check(panel.create_button.disabled and "verschieden" in panel.preview.text, "Same airport rejected in preview"):
		return
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == "LTFM":
			_choose(panel.destination, i)
	if not _check(panel.create_button.disabled and "Reichweite überschritten" in panel.preview.text, "Long route rejected in preview"):
		return
	_choose(panel.destination, 1)
	if not _check(not panel.create_button.disabled and "Reichweite ausreichend" in panel.preview.text and "km" in panel.preview.text, "Valid route displays distance and range check"):
		return
	await _click(panel.create_button)
	if not _check(main.simulation.get_routes().size() == 1 and panel.aircraft.item_count == 1 and panel.route_tree.get_root().get_child_count() == 1, "Created route listed and aircraft removed from available choices"):
		return
	var before: Array = main.simulation.get_routes()
	for args in [["XXXX", "EDDM", 2], ["EDDF", "EDDF", 2], ["EDDF", "KJFK", 2], ["LTFM", "LTFJ", 1]]:
		if not _check(not main.simulation.create_route(args[0], args[1], args[2]).is_empty() and main.simulation.get_routes() == before, "Rust rejects invalid routes and duplicate assignments without mutation"):
			return
	var row: TreeItem = panel.route_tree.get_root().get_child(0)
	row.select(0)
	panel.route_tree.item_selected.emit()
	_choose(panel.assignment, 2)
	await _click(panel.assign_button)
	if not _check(main.simulation.get_routes()[0]["aircraft_id"] == 2 and panel.aircraft.get_item_metadata(0) == 1, "Reassignment releases previous aircraft"):
		return
	_choose(panel.assignment, 0)
	await _click(panel.assign_button)
	if not _check(main.simulation.get_routes()[0]["aircraft_id"] == 0 and panel.aircraft.item_count == 2, "Unassigning releases aircraft but keeps route"):
		return
	_choose(panel.assignment, 1)
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
	if not _check(main.simulation.get_routes()[0]["id"] == 2, "Deleted IDs are not reused"):
		return
	await _click(main.navigation[1])
	var assigned_id: int = main.simulation.get_routes()[0]["aircraft_id"]
	if not _check("Route #2" in main.fleet_tree.get_root().get_child(assigned_id - 1).get_text(3), "Fleet displays reserved route"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Fresh Routes"
	await _click(main.found_button)
	await _click(main.navigation[2])
	if not _check(main.simulation.get_routes().is_empty() and panel.empty_label.visible and panel.create_button.disabled, "New game clears routes and stale assignments"):
		return
	print("PASS: route UI, distances, range, exclusive assignments, reassign, unassign, delete, small window and new game")
	main.queue_free()
	await process_frame
	quit(0)


func _choose(select: OptionButton, index: int) -> void:
	select.select(index)
	select.item_selected.emit(index)


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
