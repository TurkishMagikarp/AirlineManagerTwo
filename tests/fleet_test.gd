extends SceneTree
## Actual purchase buttons; no direct mutation of Rust funds or fleet.


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main = scene.instantiate()
	root.add_child(main)
	main.set_process(false)
	main.name_input.text = "Fleet UI Air"
	await _click(main.found_button)
	await _click(main.navigation[1])
	if not _check(main.fleet_panel.visible and main.empty_fleet.visible and main.simulation.get_fleet().is_empty(), "Founded airline starts without aircraft"):
		return
	var initial_balance: int = main.simulation.get_airline_info()["capital_eur"]
	var atr_index := -1
	var a320_index := -1
	for index in range(main.model_select.item_count):
		main.model_select.select(index)
		main.model_select.item_selected.emit(index)
		var model: Dictionary = main.model_select.get_item_metadata(index)
		if model["id"] == "atr72_600":
			atr_index = index
		if model["id"] == "a320_200":
			a320_index = index
		if not _check(main.UI.money(model["price_eur"]) in main.model_details.text and str(model["seats"]) in main.model_details.text and "%d km" % model["range_km"] in main.model_details.text, "Purchase shows Rust price, seat capacity and range"):
			return
	if not _check(atr_index >= 0 and a320_index >= 0 and main.model_select.item_count == 3, "Three real aircraft models available"):
		return
	main.model_select.select(atr_index)
	main.model_select.item_selected.emit(atr_index)
	var price: int = main.model_select.get_selected_metadata()["price_eur"]
	for count in range(1, 3):
		await _click(main.buy_button)
		var fleet: Array = main.simulation.get_fleet()
		if not _check(fleet.size() == count and main.simulation.get_airline_info()["capital_eur"] == initial_balance - price * count, "Successful purchase debits exactly once"):
			return
		var plane: Dictionary = fleet.back()
		if not _check(plane["identifier"] == "AM-%06d" % count and plane["model_id"] == "atr72_600" and plane["location_icao"] == "EDDF" and plane["status"] == "parked" and plane["purchase_price_eur"] == price, "Individual owned aircraft is parked at home airport"):
			return
		var row: TreeItem = main.fleet_tree.get_root().get_child(count - 1)
		if not _check(row.get_text(0) == plane["identifier"] and row.get_text(1) == "ATR 72-600 · " + plane["ownership"] and row.get_text(2) == "EDDF" and row.get_text(3) == "Geparkt", "Fleet table displays all purchased aircraft"):
			return
	var fleet_before: Array = main.simulation.get_fleet()
	var balance_before: int = main.simulation.get_airline_info()["capital_eur"]
	main.model_select.select(a320_index)
	main.model_select.item_selected.emit(a320_index)
	await _click(main.buy_button)
	if not _check("Nicht genügend Guthaben" in main.fleet_status.text and main.simulation.get_fleet() == fleet_before and main.simulation.get_airline_info()["capital_eur"] == balance_before, "Insufficient funds preserves both fleet and balance"):
		return
	if not await _capture("fleet.png"):
		return
	await _click(main.navigation[0])
	if not _check(main.metrics["balance"].is_visible_in_tree() and main.metrics["balance"].text == main.UI.money(balance_before), "Overview displays remaining balance, not original start capital"):
		return
	await _click(main.new_game_button)
	await _click(main.cancel_founding_button)
	if not _check(main.simulation.get_fleet() == fleet_before and main.simulation.get_airline_info()["capital_eur"] == balance_before, "Cancelling a new game preserves fleet and money"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	await _click(main.navigation[1])
	await _reveal(main.buy_button)
	if not _check(root.get_visible_rect().encloses(main.buy_button.get_global_rect()), "Purchase remains reachable in minimum window"):
		return
	if not await _capture("fleet-minimum.png"):
		return
	await _click(main.new_game_button)
	main.name_input.text = "Fresh Fleet Air"
	await _click(main.found_button)
	await _click(main.navigation[1])
	if not _check(main.simulation.get_fleet().is_empty() and main.empty_fleet.visible and main.simulation.get_airline_info()["capital_eur"] == initial_balance, "Confirmed new game clears old fleet and resets funds"):
		return
	print("PASS: real fleet UI purchases, model details, home location, unique IDs, insufficient funds, current balance and new-game lifecycle")
	main.queue_free()
	await process_frame
	quit(0)


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


func _capture(filename: String) -> bool:
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-dir="):
			await RenderingServer.frame_post_draw
			return _check(root.get_texture().get_image().save_png(argument.trim_prefix("--capture-dir=").path_join(filename)) == OK, "Capture " + filename)
	return true


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
