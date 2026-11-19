extends SceneTree
## Route profitability CSV export against real Rust data; compares parsed rows with get_route_finances().
const RouteCsv = preload("res://scripts/route_finance_csv.gd")

func _initialize() -> void:
	call_deferred("_run")

func _parse(text: String) -> Array:
	var lines := text.split("\r\n")
	if lines[-1] != "":
		return []
	lines.remove_at(lines.size() - 1)
	var rows: Array = []
	for line in lines:
		rows.append(line.split(","))
	return rows

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	sim.found_airline("CSV Air", "DE", "EDDF")
	var panel = main.finances_panel
	var dir := OS.get_temp_dir().path_join("airline-route-csv-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(dir)
	var path := dir.path_join("routes.csv")
	var empty_report: Array = sim.get_route_finances()
	if not _check(empty_report.is_empty() and panel.export_route_csv(path).is_empty(), "Empty report exports"):
		return
	if not _check(FileAccess.get_file_as_string(path) == "route_id,origin,destination,flights,revenue_eur,costs_eur,profit_eur\r\n", "Empty report is header only"):
		return
	for id in [1, 2]:
		sim.buy_aircraft("atr72_600")
		sim.create_route("EDDF", "EDDM", id)
		sim.hire_missing_staff()
		sim.schedule_route(id, 1, 120)
	sim.start()
	sim.advance(3.0 * 86400.0)
	sim.pause()
	var target: int = sim.get_routes()[-1]["id"]
	for op in sim.get_operations():
		if op["route_id"] == target:
			sim.remove_route_aircraft(target, op["aircraft_id"])
	var deleted: String = sim.delete_route(target)
	for step in range(200):
		if deleted.is_empty():
			break
		sim.advance(600.0)
		deleted = sim.delete_route(target)
	sim.pause()
	if not _check(deleted.is_empty(), "Delete second route: " + deleted):
		return
	var rows: Array = sim.get_route_finances()
	var inactive := 0
	for r in rows:
		inactive += 0 if r["active"] else 1
	if not _check(rows.size() == 2 and inactive == 1 and rows[0]["flights"] > 0, "Fixture has active and historical inactive route (%s)" % [rows]):
		return
	var before: Dictionary = sim.get_financial_summary()
	var before_routes: Array = sim.get_routes()
	if not _check(panel.export_route_csv(path).is_empty(), "Export succeeds"):
		return
	var parsed := _parse(FileAccess.get_file_as_string(path))
	if not _check(parsed.size() == rows.size() + 1 and parsed[0] == PackedStringArray(RouteCsv.HEADER), "Header and one row per route"):
		return
	for i in range(rows.size()):
		var r: Dictionary = rows[i]
		var cells: PackedStringArray = parsed[i + 1]
		if not _check(cells == PackedStringArray([str(r["route_id"]), r["origin"], r["destination"], str(r["flights"]), str(r["revenue_eur"]), str(r["costs_eur"]), str(r["profit_eur"])]), "Row %d matches Rust: %s" % [i, cells]):
			return
		if not _check(cells[4].is_valid_int() and cells[5].is_valid_int() and cells[6].is_valid_int(), "Plain numeric EUR values"):
			return
	if not _check(panel.export_status.visible and panel.export_status.text.contains(path), "Success feedback shown"):
		return
	if not _check(sim.get_financial_summary() == before and sim.get_routes() == before_routes and sim.get_route_finances() == rows, "Export leaves game state unchanged"):
		return
	var bad := dir.path_join("missing/sub/routes.csv")
	var error: String = panel.export_route_csv(bad)
	if not _check(not error.is_empty() and panel.export_status.text.begins_with("Export fehlgeschlagen") and not FileAccess.file_exists(bad), "Unwritable destination reports error"):
		return
	if not _check(sim.get_financial_summary() == before, "Failed export leaves state unchanged"):
		return
	var cancelled := dir.path_join("cancelled.csv")
	panel.export_dialog.canceled.emit()
	if not _check(not FileAccess.file_exists(cancelled) and FileAccess.get_file_as_string(path) == RouteCsv.build(rows), "Cancel writes nothing"):
		return
	if not _check(panel.export_button.focus_mode == Control.FOCUS_ALL and panel.export_button.text != "", "Export button keyboard focusable"):
		return
	DirAccess.remove_absolute(path)
	DirAccess.remove_absolute(dir)
	main.queue_free()
	await process_frame
	print("PASS: route profitability CSV export")
	quit(0)

func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
