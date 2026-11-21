extends SceneTree
## A check booked during a flight (LEAD-029): Routes and Overview keep the
## expected landing, show that maintenance follows, and hide the superseded departure.

var main


func _initialize() -> void:
	call_deferred("_run")


func _ops() -> Dictionary:
	main.routes_panel._operations_signature = ""
	main.routes_panel.refresh_operations()
	var row: TreeItem = main.routes_panel.operations_tree.get_root().get_first_child()
	var op: Dictionary = main.simulation.get_operations()[0]
	return {"op": op, "cells": [row.get_text(2), row.get_text(3), row.get_text(4)]}


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
	main.name_input.text = "Pending UI Air"
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
	if not _check(sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.schedule_route(1, 1, 120).is_empty(), "Shuttle scheduled"):
		return
	sim.start()
	sim.set_speed(16)
	sim.advance(7300.0 / 16.0)
	var before := _ops()
	if not _check(before["op"]["state_kind"] == "airborne" and before["op"]["next_departure"] != "—", "Airborne row shows the next departure"):
		return
	var landing: String = before["op"]["expected_landing"]
	if not _check(sim.request_maintenance(1).is_empty(), "Order a check in flight"):
		return
	var ops := _ops()
	var end: String = ops["op"]["maintenance_estimate_end"]
	if not _check(ops["op"]["maintenance_pending"] and ops["op"]["next_departure"] == "—" and ops["op"]["expected_landing"] == landing and ops["cells"][0] == main.tr("OPS_STATE_MAINT_PENDING") and ops["cells"][1] == main.tr("OPS_NEXT_MAINT_PENDING") and "voraussichtlich" in ops["op"]["status_text"] and ops["cells"][2] == landing and not end.is_empty(), "Routes keeps the landing and shows maintenance instead of a departure"):
		return
	var overview := _overview_text()
	if not _check("Wartung nach Landung" in overview and "voraussichtlich" in overview and end in overview and sim.get_map_flights()[0]["arrival"] == landing, "Overview shows the pending check and keeps the landing"):
		return
	main.routes_panel.size = Vector2(680, 520)
	main._refresh_commitments()

	var path := OS.get_temp_dir().path_join("airline-maintenance-pending-%d.json" % Time.get_ticks_usec())
	main.save_path = path
	main._save_legacy_game()
	main._load_legacy_game()
	DirAccess.remove_absolute(path)
	sim = main.simulation
	ops = _ops()
	if not _check(ops["op"]["maintenance_pending"] and ops["op"]["expected_landing"] == landing and ops["op"]["next_departure"] == "—" and "Wartung nach Landung" in _overview_text(), "Pending check survives save and load"):
		return

	if not _check(sim.cancel_maintenance(1).is_empty(), "Cancel the pending check"):
		return
	ops = _ops()
	if not _check(not ops["op"]["maintenance_pending"] and ops["op"]["state_kind"] == "airborne" and ops["op"]["next_departure"] != "—" and ops["op"]["expected_landing"] == landing, "Cancelling restores the normal airborne view"):
		return
	if not _check("Wartung nach Landung" not in _overview_text(), "Overview drops the pending note"):
		return

	sim.request_maintenance(1)
	sim.advance(8000.0 / 16.0)
	ops = _ops()
	var plane: Dictionary = sim.get_fleet()[0]
	if not _check(plane["status"] == "maintenance" and not ops["op"]["maintenance_pending"] and ops["op"]["state_kind"] in ["maintenance", "technician_wait"] and ops["op"]["next_departure"] == "—", "Landing moves to the grounded states"):
		return
	print("PASS: maintenance pending after landing")
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
