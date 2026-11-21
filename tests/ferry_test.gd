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
	await _route_reposition()
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
	m.ferry_dialog.safe_button.pressed.emit()
	for _i in range(3):
		await process_frame
	assert(panel.reposition_button.has_focus())
	assert(sim.get_fleet()[0]["ferry_state"] == "" and panel.origin.get_selected_metadata() == "EDDM" and panel.destination.get_selected_metadata() == "EDDF")
	assert(sim.get_routes().is_empty() and panel.create_button.disabled)
	assert(sim.hire_pilots("ATR 72", 4).is_empty() and sim.hire_staff("cabin_crew", 3).is_empty())
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
