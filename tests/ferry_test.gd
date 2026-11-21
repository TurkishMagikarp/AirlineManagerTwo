extends SceneTree
var main

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	assert(sim.found_airline("Ferry UI", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("a320_200").is_empty())
	assert(sim.hire_pilots("A320 family", 4).is_empty())
	main._sync_view()
	main._show_page(1)
	assert(not main.ferry_button.disabled)
	main.ferry_button.pressed.emit()
	await process_frame
	assert(main.ferry_dialog.visible)
	for i in range(main.ferry_destination.item_count):
		if main.ferry_destination.get_item_metadata(i) == "NZAA":
			main.ferry_destination.select(i)
			main.ferry_destination.item_selected.emit(i)
	assert(main.ferry_dialog.get_ok_button().disabled)
	assert("Reichweite" in main.ferry_preview.text)
	for i in range(main.ferry_destination.item_count):
		if main.ferry_destination.get_item_metadata(i) == "EDDM":
			main.ferry_destination.select(i)
			main.ferry_destination.item_selected.emit(i)
	assert(not main.ferry_dialog.get_ok_button().disabled)
	var quote: Dictionary = sim.preview_ferry(1, "EDDM")
	assert(main.UI.money(quote["cost_eur"]) in main.ferry_preview.text)
	main.ferry_dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_fleet()[0]["ferry_state"] == "Überführung")
	assert(main.ferry_button.disabled)
	assert(main.disposal_button.disabled and main.cabin_button.disabled)
	assert(is_instance_valid(main.ferry_toast))
	assert(sim.get_map_flights().size() == 1)
	var path := "user://ferry-ui-test.json"
	assert(sim.save_game(path).is_empty())
	assert(sim.load_game(path).is_empty())
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(quote["flight_seconds"]))
	sim.pause()
	main._refresh_fleet()
	assert(sim.get_fleet()[0]["location_icao"] == "EDDM")
	assert(sim.get_ferry_archive(0).size() == 1)
	assert(sim.get_ferry_archive(0)[0]["costs_eur"] == quote["cost_eur"])
	assert(sim.get_financial_summary()["ferry_eur"] == quote["cost_eur"])
	DirAccess.remove_absolute(ProjectSettings.globalize_path(path))
	main.queue_free()
	await process_frame
	await _other_errors_have_no_staff_shortcut()
	await _fleet_pilot_shortage()
	await _route_reposition()
	await _existing_route_reposition()
	await _destination_blockers_first()
	print("PASS: ferry dialog range rejection, route-planner repositioning, Rust quote, dispatch, chip, toast, map, restart, landing and archive")
	quit(0)


func _press_enter() -> void:
	for pressed in [true, false]:
		var key := InputEventKey.new()
		key.keycode = KEY_ENTER
		key.physical_keycode = KEY_ENTER
		key.pressed = pressed
		Input.parse_input_event(key)
		await process_frame


func _pick(select: OptionButton, icao: String) -> void:
	for i in range(select.item_count):
		if select.get_item_metadata(i) == icao:
			select.select(i)
			select.item_selected.emit(i)
			return
	assert(false, "Missing " + icao)


func _route_reposition() -> void:
	var m = load("res://scenes/main.tscn").instantiate()
	root.add_child(m)
	m.set_process(false)
	root.size = Vector2i(680, 520)
	var sim = m.simulation
	assert(sim.found_airline("Reposition Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	m._sync_view()
	m._show_page(2)
	var panel = m.routes_panel
	panel.refresh()
	_pick(panel.origin, "EDDM")
	_pick(panel.destination, "EDDF")
	assert(panel.aircraft.item_count == 1)
	assert(panel.create_button.disabled and panel.reposition_button.visible)
	assert("EDDF" in panel.reposition_button.text and "EDDM" in panel.reposition_button.text)
	assert("EDDF" in panel.preview.text)
	_pick(panel.origin, "EDDF")
	_pick(panel.destination, "EDDM")
	assert(not panel.reposition_button.visible and not panel.create_button.disabled)
	_pick(panel.origin, "EDDM")
	_pick(panel.destination, "EDDF")
	assert(panel.reposition_button.visible)
	panel.get_parent().get_parent().ensure_control_visible(panel.reposition_button)
	await process_frame
	panel.reposition_button.grab_focus()
	await _press_enter()
	await process_frame
	assert(is_instance_valid(m.ferry_dialog) and m.ferry_dialog.visible)
	assert(m.ferry_aircraft == panel.aircraft.get_selected_metadata())
	assert(m.ferry_destination.get_selected_metadata() == "EDDM")
	assert(root.get_visible_rect().encloses(m.ferry_dialog.card.get_global_rect()))
	assert(m.ferry_dialog.get_ok_button().disabled and "ATR 72" in m.ferry_preview.text)
	assert(m.ferry_staff_button.visible and "4" in m.ferry_staff_button.text and "ATR 72" in m.ferry_staff_button.text)
	m.ferry_dialog.safe_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(panel.reposition_button.has_focus())
	assert(sim.get_fleet()[0]["ferry_state"] == "" and panel.origin.get_selected_metadata() == "EDDM" and panel.destination.get_selected_metadata() == "EDDF")
	assert(sim.get_routes().is_empty() and panel.create_button.disabled)
	var cash: int = sim.get_airline_info()["capital_eur"]
	var staff_before: int = sim.get_staff().size()
	panel.reposition_button.pressed.emit()
	await process_frame
	m.ferry_staff_button.grab_focus()
	await _press_enter()
	for _i in range(6):
		await process_frame
	assert(not is_instance_valid(m.ferry_dialog) or not m.ferry_dialog.is_inside_tree())
	assert(m.current_page == 5 and m.staff_panel.visible)
	assert(m.staff_panel.role_keys[m.staff_panel.role_select.selected] == "pilot")
	assert(m.staff_panel.rating_select.get_item_text(m.staff_panel.rating_select.selected) == "ATR 72")
	assert(int(m.staff_panel.count_spin.value) == 4)
	assert(m.staff_panel.count_spin.get_line_edit().has_focus() and m.staff_panel.count_spin.get_line_edit().is_visible_in_tree())
	assert(sim.get_airline_info()["capital_eur"] == cash and sim.get_staff().size() == staff_before)
	assert(sim.get_fleet()[0]["ferry_state"] == "" and sim.get_routes().is_empty())
	m.staff_panel.hire_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(sim.get_staff().size() == staff_before + 4)
	assert(sim.hire_staff("cabin_crew", 3).is_empty())
	m._show_page(2)
	assert(panel.origin.get_selected_metadata() == "EDDM" and panel.destination.get_selected_metadata() == "EDDF" and panel.reposition_button.visible)
	panel.reposition_button.pressed.emit()
	await process_frame
	assert(m.ferry_destination.get_selected_metadata() == "EDDM" and not m.ferry_dialog.get_ok_button().disabled)
	var quote: Dictionary = sim.preview_ferry(1, "EDDM")
	m.ferry_dialog.get_ok_button().pressed.emit()
	for _i in range(3):
		await process_frame
	assert(sim.get_routes().is_empty() and sim.get_fleet()[0]["ferry_state"] == "Überführung")
	assert(panel.create_button.disabled and not panel.reposition_button.visible and "Überführung" in panel.preview.text)
	assert(panel.origin.get_selected_metadata() == "EDDM" and panel.destination.get_selected_metadata() == "EDDF")
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(quote["flight_seconds"]))
	sim.pause()
	m._refresh_fleet()
	panel.refresh_operations()
	assert(sim.get_fleet()[0]["location_icao"] == "EDDM" and panel.create_button.disabled and "Bodenzeit" in panel.preview.text)
	sim.jump(86400.0)
	panel.refresh_operations()
	assert(not panel.create_button.disabled and not panel.reposition_button.visible)
	panel.create_button.pressed.emit()
	await process_frame
	var routes: Array = sim.get_routes()
	assert(routes.size() == 1 and routes[0]["origin_icao"] == "EDDM" and panel._route().get("id", 0) == routes[0]["id"])
	assert(sim.schedule_route(routes[0]["id"], 1, 120).is_empty())
	m.queue_free()
	await process_frame


func _existing_route_reposition() -> void:
	var m = load("res://scenes/main.tscn").instantiate()
	root.add_child(m)
	m.set_process(false)
	root.size = Vector2i(680, 520)
	var sim = m.simulation
	assert(sim.found_airline("Add Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty() and sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.hire_pilots("ATR 72", 4).is_empty() and sim.hire_staff("cabin_crew", 6).is_empty())
	var first: int = sim.get_fleet()[0]["id"]
	var second: int = sim.get_fleet()[1]["id"]
	var first_quote: Dictionary = sim.preview_ferry(first, "EDDM")
	assert(sim.send_ferry(first, "EDDM").is_empty())
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(first_quote["flight_seconds"]))
	sim.pause()
	sim.jump(86400.0)
	assert(sim.create_route("EDDM", "EDDF", first).is_empty())
	var route_id: int = sim.get_routes()[0]["id"]
	assert(not sim.add_route_aircraft(route_id, second).is_empty())
	assert(sim.get_routes()[0]["aircraft_ids"] == [first] and sim.get_fleet()[1]["route_id"] == 0)
	assert(sim.schedule_route(route_id, 1, 240).is_empty())
	assert(not sim.add_route_aircraft(route_id, second).is_empty())
	assert(sim.get_routes().size() == 1 and sim.get_fleet()[1]["route_id"] == 0)
	m._sync_view()
	m._show_page(2)
	var panel = m.routes_panel
	panel.refresh()
	panel.focus_route(route_id)
	for i in range(panel.assignment.item_count):
		if panel.assignment.get_item_metadata(i) == second:
			panel.assignment.select(i)
			panel.assignment.item_selected.emit(i)
	assert(panel.assign_button.disabled and panel.assign_ferry_button.visible)
	assert("EDDF" in panel.assign_ferry_button.text and "EDDM" in panel.assign_ferry_button.text)
	assert("EDDF" in panel.status.text and "EDDM" in panel.status.text)
	panel.get_parent().get_parent().ensure_control_visible(panel.assign_ferry_button)
	await process_frame
	panel.assign_ferry_button.grab_focus()
	await _press_enter()
	await process_frame
	assert(m.ferry_dialog.visible and m.ferry_aircraft == second and m.ferry_destination.get_selected_metadata() == "EDDM")
	assert(m.ferry_dialog.get_ok_button().disabled and m.ferry_staff_button.visible and "4" in m.ferry_staff_button.text)
	m.ferry_dialog.safe_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(panel.assign_ferry_button.has_focus() and sim.get_fleet()[1]["ferry_state"] == "")
	assert(panel._selected_id(panel.assignment) == second and panel._route().get("id", 0) == route_id)
	var cash: int = sim.get_airline_info()["capital_eur"]
	var staff_before: int = sim.get_staff().size()
	panel.assign_ferry_button.pressed.emit()
	await process_frame
	m.ferry_staff_button.grab_focus()
	await _press_enter()
	for _i in range(6):
		await process_frame
	assert(m.current_page == 5 and int(m.staff_panel.count_spin.value) == 4)
	assert(m.staff_panel.rating_select.get_item_text(m.staff_panel.rating_select.selected) == "ATR 72")
	assert(sim.get_airline_info()["capital_eur"] == cash and sim.get_staff().size() == staff_before)
	m.staff_panel.hire_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(sim.get_staff().size() == staff_before + 4)
	m._show_page(2)
	assert(panel._selected_id(panel.assignment) == second and panel._route().get("id", 0) == route_id and panel.assign_ferry_button.visible)
	panel.assign_ferry_button.pressed.emit()
	await process_frame
	assert(not m.ferry_dialog.get_ok_button().disabled and not m.ferry_staff_button.visible)
	var quote: Dictionary = sim.preview_ferry(second, "EDDM")
	m.ferry_dialog.get_ok_button().pressed.emit()
	for _i in range(3):
		await process_frame
	assert(panel._selected_id(panel.assignment) == second and panel._route().get("id", 0) == route_id)
	assert(panel.assign_button.disabled and "Überführung" in panel.status.text)
	assert(not sim.add_route_aircraft(route_id, second).is_empty())
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(quote["flight_seconds"]))
	sim.pause()
	m._refresh_fleet()
	panel.refresh_operations()
	assert(sim.get_fleet()[1]["location_icao"] == "EDDM" and panel.assign_button.disabled and "Bodenzeit" in panel.assign_button.tooltip_text)
	sim.jump(86400.0)
	panel.refresh_operations()
	assert(not panel.assign_button.disabled and not panel.assign_ferry_button.visible and panel._selected_id(panel.assignment) == second)
	panel.assign_button.pressed.emit()
	await process_frame
	assert(sim.get_routes()[0]["aircraft_ids"].has(second) or sim.get_routes().size() == 2)
	var flew := false
	for _i in range(60):
		sim.jump(600.0)
		for flight in sim.get_map_flights():
			if flight.get("aircraft_id", 0) == second:
				flew = true
	assert(flew and sim.get_fleet()[1]["route_id"] != 0)
	panel._assignment_preview()
	assert(panel.assign_button.text == "Flugzeug entfernen")
	panel.assign_button.pressed.emit()
	await process_frame
	m.queue_free()
	await process_frame


func _other_errors_have_no_staff_shortcut() -> void:
	var m = load("res://scenes/main.tscn").instantiate()
	root.add_child(m)
	m.set_process(false)
	var sim = m.simulation
	assert(sim.found_airline("Other Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty() and sim.hire_pilots("ATR 72", 4).is_empty())
	m._sync_view()
	m._show_page(1)
	m.ferry_button.pressed.emit()
	await process_frame
	_pick(m.ferry_destination, "NZAA")
	assert(m.ferry_dialog.get_ok_button().disabled and "Reichweite" in m.ferry_preview.text and not m.ferry_staff_button.visible)
	_pick(m.ferry_destination, "EDDM")
	assert(not m.ferry_dialog.get_ok_button().disabled and not m.ferry_staff_button.visible)
	m.ferry_dialog.get_ok_button().pressed.emit()
	for _i in range(3):
		await process_frame
	assert(sim.get_fleet()[0]["ferry_state"] == "Überführung")
	m._open_ferry(1, "EDDF")
	await process_frame
	assert(m.ferry_dialog.get_ok_button().disabled and not m.ferry_staff_button.visible)
	m.ferry_dialog.safe_button.pressed.emit()
	await process_frame
	m.queue_free()
	await process_frame


func _fleet_pilot_shortage() -> void:
	var m = load("res://scenes/main.tscn").instantiate()
	root.add_child(m)
	m.set_process(false)
	var sim = m.simulation
	assert(sim.found_airline("Fleet Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty() and sim.buy_aircraft("atr72_600").is_empty())
	var first: int = sim.get_fleet()[0]["id"]
	var second: int = sim.get_fleet()[1]["id"]
	m._sync_view()
	m._show_page(1)
	m.selected_aircraft = first
	m._refresh_fleet()
	assert(not m.ferry_button.disabled and sim.get_fleet()[0]["ferry_pilots_missing"] == 4)
	var cash: int = sim.get_airline_info()["capital_eur"]
	var staff_before: int = sim.get_staff().size()
	m.ferry_button.pressed.emit()
	await process_frame
	_pick(m.ferry_destination, "EDDM")
	assert(m.ferry_dialog.get_ok_button().disabled and m.ferry_staff_button.visible and "ATR 72" in m.ferry_preview.text and "4" in m.ferry_preview.text)
	m.ferry_staff_button.grab_focus()
	await _press_enter()
	for _i in range(8):
		await process_frame
	assert(m.current_page == 5 and m.staff_panel.role_keys[m.staff_panel.role_select.selected] == "pilot")
	assert(m.staff_panel.rating_select.get_item_text(m.staff_panel.rating_select.selected) == "ATR 72" and int(m.staff_panel.count_spin.value) == 4)
	assert("Flotte" in m.staff_panel.action_status.text and "Routen" not in m.staff_panel.action_status.text)
	assert(sim.get_airline_info()["capital_eur"] == cash and sim.get_staff().size() == staff_before and sim.get_fleet()[0]["ferry_state"] == "")
	m._show_page(1)
	assert(m.selected_aircraft == first)
	m.ferry_button.pressed.emit()
	await process_frame
	assert(m.ferry_destination.get_selected_metadata() == "EDDM" and m.ferry_staff_button.visible and m.ferry_dialog.get_ok_button().disabled)
	m.ferry_dialog.safe_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(m.ferry_button.has_focus())
	m.selected_aircraft = second
	m.ferry_button.pressed.emit()
	await process_frame
	assert(m.ferry_destination.selected == 0 and m.ferry_aircraft == second)
	assert(m.ferry_retained.get("aircraft_id", 0) == first)
	m.ferry_dialog.safe_button.pressed.emit()
	await process_frame
	m.selected_aircraft = first
	assert(sim.hire_pilots("ATR 72", 4).is_empty())
	m._refresh_fleet()
	m.ferry_button.pressed.emit()
	await process_frame
	assert(m.ferry_destination.get_selected_metadata() == "EDDM" and not m.ferry_staff_button.visible and not m.ferry_dialog.get_ok_button().disabled)
	m.ferry_dialog.get_ok_button().pressed.emit()
	for _i in range(3):
		await process_frame
	assert(sim.get_fleet()[0]["ferry_state"] == "Überführung" and m.ferry_retained.is_empty())
	m._refresh_fleet()
	assert(m.ferry_button.disabled)
	# A reused aircraft ID in a new world must not inherit a retained destination.
	m.ferry_retained = {"aircraft_id": 1, "destination": "EDDM"}
	m._sync_view()
	assert(m.ferry_retained.is_empty())
	m.queue_free()
	await process_frame


func _destination_blockers_first() -> void:
	var m = load("res://scenes/main.tscn").instantiate()
	root.add_child(m)
	m.set_process(false)
	root.size = Vector2i(680, 520)
	var sim = m.simulation
	assert(sim.found_airline("Blocker Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(not sim.preview_ferry(1, "XXXX")["error"].is_empty() and not sim.preview_ferry(1, "XXXX").has("pilots_missing"))
	assert("verschieden" in sim.preview_ferry(1, "EDDF")["error"] and not sim.preview_ferry(1, "EDDF").has("pilots_missing"))
	assert(not sim.send_ferry(1, "XXXX").is_empty() and not sim.send_ferry(1, "NZAA").is_empty())
	m._sync_view()
	m._show_page(1)
	m.selected_aircraft = 1
	m._refresh_fleet()
	m.ferry_button.pressed.emit()
	await process_frame
	var staff: int = sim.get_staff().size()
	for _round in range(2):
		_pick(m.ferry_destination, "NZAA")
		assert(m.ferry_dialog.get_ok_button().disabled and "Reichweite" in m.ferry_preview.text and not m.ferry_staff_button.visible)
		assert(not "Piloten" in m.ferry_preview.text)
		assert(m.ferry_destination.get_node(m.ferry_destination.focus_next) == m.ferry_dialog.danger_button)
		_pick(m.ferry_destination, "EDDM")
		assert(m.ferry_dialog.get_ok_button().disabled and m.ferry_staff_button.visible and "ATR 72" in m.ferry_preview.text and "4" in m.ferry_preview.text)
		assert(m.ferry_destination.get_node(m.ferry_destination.focus_next) == m.ferry_staff_button)
	_pick(m.ferry_destination, "NZAA")
	assert(not m.ferry_staff_button.visible)
	assert(sim.get_staff().size() == staff and sim.get_fleet()[0]["ferry_state"] == "")
	m.ferry_dialog.safe_button.pressed.emit()
	await process_frame
	m._show_page(2)
	var panel = m.routes_panel
	panel.refresh()
	_pick(panel.origin, "KORD")
	_pick(panel.destination, "KJFK")
	assert(panel.reposition_button.visible)
	panel.reposition_button.pressed.emit()
	await process_frame
	assert(m.ferry_destination.get_selected_metadata() == "KORD")
	assert(m.ferry_dialog.get_ok_button().disabled and "Reichweite" in m.ferry_preview.text and "Entwurf" in m.ferry_preview.text and not m.ferry_staff_button.visible)
	assert(root.get_visible_rect().encloses(m.ferry_dialog.card.get_global_rect()))
	m.ferry_dialog.safe_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(panel.reposition_button.has_focus() and panel.origin.get_selected_metadata() == "KORD" and panel.destination.get_selected_metadata() == "KJFK")
	m._open_ferry(1, "KORD", panel.assign_ferry_button)
	await process_frame
	assert(m.ferry_dialog.get_ok_button().disabled and "Reichweite" in m.ferry_preview.text and not m.ferry_staff_button.visible)
	m.ferry_dialog.safe_button.pressed.emit()
	await process_frame
	m.queue_free()
	await process_frame
