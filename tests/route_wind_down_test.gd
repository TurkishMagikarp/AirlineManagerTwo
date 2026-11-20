extends SceneTree
## Wind down a flying route from the Routes panel: pending releases, save/load, delete, Finances retention.

var main: Control


func _initialize() -> void:
	call_deferred("_run")


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition


func _reveal(control: Control) -> void:
	await process_frame
	var parent := control.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(control)
		parent = parent.get_parent()
	await process_frame


func _run() -> void:
	TranslationServer.set_locale("de")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	for error in [sim.found_airline("Wind Down Air", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.hire_missing_staff(), sim.schedule_route(1, 1, 240), sim.add_route_aircraft(1, 2)]:
		if not _check(error.is_empty(), "Setup: " + error):
			return
	main.navigation[2].pressed.emit()
	var panel = main.routes_panel
	panel.refresh()
	panel.focus_route(1)
	sim.start()
	sim.advance(100.0)
	panel.refresh_operations()
	var route: Dictionary = panel._route()
	if not _check(not panel.wind_down_button.disabled and panel.delete_button.disabled and route["can_wind_down"] and not route["can_delete"], "Active route offers wind down and blocks delete with an explanation"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	await _reveal(panel.wind_down_button)
	if not _check(panel.wind_down_button.focus_mode == Control.FOCUS_ALL and root.get_visible_rect().encloses(panel.wind_down_button.get_global_rect()), "Wind down reachable and focusable at minimum size"):
		return
	panel.wind_down_button.grab_focus()
	panel.wind_down_button.pressed.emit()
	panel.wind_down_button.pressed.emit()
	await process_frame
	route = panel._route()
	if not _check(route["release_pending"].size() >= 1 and panel.release_label.visible and panel.delete_button.disabled and route["delete_available_at"] > 0 and route["delete_available_at_text"] in panel.release_label.text, "Pending release and delete time shown"):
		return
	var pending: Array = route["release_pending"]
	var path := ProjectSettings.globalize_path("user://wind_down_test.json")
	if not _check(sim.save_game(path).is_empty() and sim.load_game(path).is_empty(), "Save and reload while release is pending"):
		return
	panel.refresh()
	panel.focus_route(1)
	if not _check(panel._route()["release_pending"] == pending and panel.delete_button.disabled and panel.release_label.visible, "Pending release survives Save/Load"):
		return
	var ready: int = panel._route()["delete_available_at"]
	sim.advance(float(ready) - 100.0 + 1.0)
	panel.refresh_operations()
	await process_frame
	await process_frame
	route = sim.get_routes()[0]
	if not _check(route["release_pending"].is_empty() and route["can_delete"] and not panel.delete_button.disabled and not panel.release_label.visible and not route["has_plan"], "Delete becomes available after all releases"):
		return
	var flights: int = route["actual_flights"]
	if not _check(flights >= 1, "Completed flights recorded"):
		return
	panel.delete_button.pressed.emit()
	await process_frame
	if not _check(sim.get_routes().is_empty(), "Route deleted"):
		return
	var kept := 0
	for r in sim.get_route_finances():
		if r["route_id"] == 1 and not r["active"]:
			kept = r["flights"]
	if not _check(kept >= flights, "Closed route keeps its completed flights in Finances"):
		return
	DirAccess.remove_absolute(path)
	print("PASS: route wind down")
	quit(0)
