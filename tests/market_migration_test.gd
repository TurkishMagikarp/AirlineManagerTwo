extends SceneTree
## Legacy single-aircraft routes and pre-fuel saves through the real extension/UI.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	var directory := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode")
	var restore := "--restore-migration" in OS.get_cmdline_user_args()
	if not restore:
		sim.found_airline("Migration Air", "DE", "EDDF")
		sim.buy_aircraft("atr72_600")
		sim.create_route("EDDF", "EDDM", 1)
		sim.start()
		sim.advance(86400.0)
		sim.pause()
		var source := directory.path_join("migration-source.json")
		if not _check(sim.save_game(source).is_empty(), "Save migration source"):
			return
		var current := FileAccess.get_file_as_string(source)
		var fuel := RegEx.create_from_string('"fuel_market":\\{[^}]*\\},?')
		var cabins := RegEx.create_from_string('"business_(seats|ticket_eur|passengers|revenue)":\\d+,|,"business_(seats|ticket_eur|passengers|revenue)":\\d+')
		var service := RegEx.create_from_string('"service_level":"[a-z]+",|,"service_level":"[a-z]+"|"service":-?\\d+,|,"service":-?\\d+')
		var assignment := RegEx.create_from_string('"aircraft_ids":\\[([0-9]+)\\]')
		for version in [14, 15]:
			# Text substitutions preserve Rust u64/nanosecond integer encodings.
			var legacy := fuel.sub(current, "", true).replace('"version":18', '"version":%d' % version)
			legacy = cabins.sub(legacy, "", true)
			legacy = service.sub(legacy, "", true)
			if not _check(JSON.parse_string(legacy)["version"] == version, "Fixture uses actual legacy version"):
				return
			if version == 14:
				legacy = assignment.sub(legacy, '"aircraft_id":$1', true)
				legacy = legacy.replace('"aircraft_ids":[]', '"aircraft_id":null').replace('"rotations":[],', '')
			var file := FileAccess.open(directory.path_join("migration-v%d.json" % version), FileAccess.WRITE)
			file.store_string(legacy)
			file.close()
	for version in [14, 15]:
		var path := directory.path_join("migration-v%d.json" % version)
		var error: String = sim.load_game(path)
		if not _check(error.is_empty(), "Load v%d in Godot: %s" % [version, error]):
			return
		main._sync_view()
		main._show_page(2)
		var routes: Array = sim.get_routes()
		if not _check(routes.size() == 1 and routes[0]["aircraft_ids"] == [1] and sim.get_fleet()[0]["route_id"] == 1 and "AM-000001" in main.routes_panel.route_tree.get_root().get_child(0).get_text(3), "Legacy assignment renders in Routes"):
			return
		if not _check(sim.get_elapsed_seconds() == 86400.0 and sim.get_financial_summary()["fuel_cents_per_kg"] == 80, "Legacy time preserved and fuel starts at base price"):
			return
		if not _check(sim.save_game(path).is_empty() and JSON.parse_string(FileAccess.get_file_as_string(path))["version"] == 18, "Migrated save uses current version"):
			return
	# Exercise a changed global price, beyond the zero-delta initial display.
	sim.start()
	sim.advance(31.0 * 86400.0)
	sim.pause()
	main._sync_view()
	main._show_page(4)
	var summary: Dictionary = sim.get_financial_summary()
	var note: Label = main.finances_panel.note
	if not _check(summary["fuel_month_delta_cents"] != 0 and ("Treibstoff weltweit: %.2f EUR/kg" % (summary["fuel_cents_per_kg"] / 100.0)) in note.text and ("%d Cent/kg gegenüber vor 30 Tagen" % summary["fuel_month_delta_cents"]) in note.text, "Changed fuel price and nonzero monthly delta reach Finances"):
		return
	main.queue_free()
	await process_frame
	print("PASS: v14 route groups, v15 fuel migration, current saves and route UI (fresh-process restore=%s)" % restore)
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
