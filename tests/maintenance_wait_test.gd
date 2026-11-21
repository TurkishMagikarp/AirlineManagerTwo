extends SceneTree
## Fleet shows a check waiting for technicians (LEAD-009): register, details,
## Attention, the Staff action and the release time once staffed.

var main


func _initialize() -> void:
	call_deferred("_run")


func _ops() -> Dictionary:
	main.routes_panel._operations_signature = ""
	main.routes_panel.refresh_operations()
	var row: TreeItem = main.routes_panel.operations_tree.get_root().get_first_child()
	var op: Dictionary = main.simulation.get_operations()[0]
	return {"op": op, "cells": [row.get_text(2), row.get_text(3), row.get_tooltip_text(0)]}


func _overview_text() -> String:
	main._render_operations(main.overview_operations)
	var text := ""
	for child in main.overview_operations.get_children():
		if child is Label:
			text += child.text + "\n"
	return text


func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	main.name_input.text = "Wait UI Air"
	await _click(main.found_button)
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	await _click(main.buy_button)
	root.size = Vector2i(680, 520)
	sim.hire_pilots("ATR 72", 4)
	sim.hire_staff("cabin_crew", 3)
	var e1: String = sim.create_route("EDDF", "EDDM", 1)
	var e2: String = sim.schedule_route(1, 1, 120)
	if not _check(e1.is_empty() and e2.is_empty(), "Shuttle scheduled"):
		return
	sim.start()
	sim.set_speed(16)
	sim.advance(7300.0 / 16.0)
	if not _check(sim.request_maintenance(1).is_empty(), "Order a check"):
		return
	sim.advance(8000.0 / 16.0)
	main._refresh_fleet()
	var plane: Dictionary = sim.get_fleet()[0]
	if not _check(plane["maintenance_waiting_staff"] and plane["maintenance_technicians_available"] == 0, "Check waits for technicians"):
		return
	var estimate_end: String = plane["maintenance_end"]
	sim.advance(5.0 * 86400.0 / 16.0)
	main._refresh_fleet()
	plane = sim.get_fleet()[0]
	var row: TreeItem = main.fleet_tree.get_root().get_child(0)
	if not _check(row.get_text(5) == "Wartet auf Techniker" and estimate_end not in row.get_text(5), "Register shows the wait without a release time"):
		return
	var ops := _ops()
	var cells: Array = ops["cells"]
	if not _check(ops["op"]["technician_wait"] and cells[0] == main.tr("OPS_STATE_TECH_WAIT") and cells[1] == main.tr("OPS_NEXT_TECH_WAIT") and estimate_end not in cells[2] and "Techniker" in cells[2] and ops["op"]["next_departure"] == "—" and "Techniker" in ops["op"]["status_text"] and estimate_end not in ops["op"]["status_text"], "Routes row and tooltip show the technician wait without times"):
		return
	var overview := _overview_text()
	if not _check("wartet auf Techniker" in overview and estimate_end not in overview and "Nächster Abflug" not in overview, "Overview shows the wait without times"):
		return
	var details: String = main.maintenance_details.text
	if not _check("Techniker" in details and "Benötigt %d" % plane["maintenance_technicians_needed"] in details and "frei: 0" in details and plane["identifier"] in details and "bleibt am Boden" in details and estimate_end not in details and " – " not in details, "Details show the wait, counts and identity"):
		return
	if not _check(main.maintenance_staff_button.visible, "Staff action is visible"):
		return

	await _click(main.navigation[0])
	main._refresh_attention()
	var attention: Button
	for child in main.attention_rows.get_children():
		if child is Button and "Techniker" in child.text:
			attention = child
	if not _check(attention != null, "Attention row for the technician wait"):
		return
	attention.pressed.emit()
	await process_frame
	await process_frame
	if not _check(main.current_page == 1 and main.fleet_tabs.current_tab == 0 and main.selected_aircraft == 1, "Attention opens Fleet Register on the aircraft"):
		return

	main.maintenance_staff_button.grab_focus()
	for pressed in [true, false]:
		var key := InputEventKey.new()
		key.keycode = KEY_ENTER
		key.physical_keycode = KEY_ENTER
		key.pressed = pressed
		Input.parse_input_event(key)
		await process_frame
	for frame in range(6):
		await process_frame
	var staff = main.staff_panel
	if not _check(main.current_page == 5 and staff.role_keys[staff.role_select.selected] == "technician" and staff.role_select.has_focus(), "Staff action opens hiring with Technician selected and focused"):
		return

	var path := OS.get_temp_dir().path_join("airline-maintenance-wait-%d.json" % Time.get_ticks_usec())
	main.save_path = path
	main._save_legacy_game()
	main._load_legacy_game()
	DirAccess.remove_absolute(path)
	await _click(main.navigation[1])
	if not _check(sim.get_fleet()[0]["maintenance_waiting_staff"] and main.fleet_tree.get_root().get_child(0).get_text(5) == "Wartet auf Techniker" and main.maintenance_staff_button.visible, "Wait and action survive save and load"):
		return
	ops = _ops()
	if not _check(ops["op"]["technician_wait"] and estimate_end not in ops["op"]["status_text"], "Wait survives save and load in Routes"):
		return
	await _click(main.navigation[5])
	var needed: int = plane["maintenance_technicians_needed"]
	if not _check(needed >= 2, "Check needs several technicians"):
		return
	staff.count_spin.value = needed - 1
	await _click(staff.hire_button)
	await _click(main.navigation[1])
	plane = sim.get_fleet()[0]
	if not _check(plane["maintenance_waiting_staff"] and plane["maintenance_technicians_available"] == needed - 1 and main.maintenance_staff_button.visible, "Too few technicians keep the wait visible"):
		return
	if not _check(_ops()["op"]["technician_wait"], "Too few technicians keep the wait in Routes"):
		return
	await _click(main.navigation[5])
	staff.count_spin.value = 1
	await _click(staff.hire_button)
	await _click(main.navigation[1])
	plane = sim.get_fleet()[0]
	row = main.fleet_tree.get_root().get_child(0)
	if not _check(not plane["maintenance_waiting_staff"] and plane["maintenance_state"] == "active" and row.get_text(5).begins_with("Wartung bis") and plane["maintenance_end"] in row.get_text(5) and not main.maintenance_staff_button.visible and "läuft" in main.maintenance_details.text and plane["maintenance_end"] in main.maintenance_details.text, "Staffed check shows the release time and clears the action"):
		return
	ops = _ops()
	cells = ops["cells"]
	if not _check(not ops["op"]["technician_wait"] and ops["op"]["maintenance_release"] == plane["maintenance_end"] and plane["maintenance_end"] in ops["op"]["status_text"] and plane["maintenance_end"] in cells[1] and plane["maintenance_end"] in cells[2] and ops["op"]["next_departure"] == "—", "Routes shows the Fleet release time once staffed"):
		return
	overview = _overview_text()
	if not _check(plane["maintenance_end"] in overview and "Nächster Abflug" not in overview, "Overview shows the Fleet release time once staffed"):
		return
	var flights: int = sim.get_financial_summary()["flights"]
	sim.advance(3600.0 / 16.0)
	if not _check(sim.get_fleet()[0]["status"] == "maintenance" and sim.get_financial_summary()["flights"] == flights, "Aircraft stays grounded until completion"):
		return
	sim.advance((plane["maintenance_end_seconds"] - sim.get_elapsed_seconds() + 30.0) / 16.0)
	sim.advance(7300.0 / 16.0)
	if not _check(sim.get_fleet()[0]["checks"] == 1 and sim.get_financial_summary()["flights"] > flights, "Departures resume after completion"):
		return
	if not _check(_ops()["op"]["state_kind"] != "maintenance" and _ops()["op"]["next_departure"] != "—", "Real next departure returns after the check"):
		return
	print("PASS: Fleet shows the technician wait, the Staff action and the release time")
	quit(0)


func _click(button: Button) -> void:
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
