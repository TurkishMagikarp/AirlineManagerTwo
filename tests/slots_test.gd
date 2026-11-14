extends "res://tests/insurance_test.gd"
## Slot data flows from Rust into the existing planning and map panels.
func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if not _check(sim.found_airline("Slots UI", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty(), "Found and buy"):
		return
	main._sync_view()
	var panel = main.routes_panel
	if not _check(panel.preview.text.contains("Slots belegt") and panel.preview.text.contains("frei/Tag"), "Planning displays Rust slot usage"):
		return
	if not _check(sim.create_route("EDDF", "EDDM", 1).is_empty() and sim.hire_missing_staff().is_empty() and sim.schedule_route(1, 1, 120).is_empty(), "Schedule service"):
		return
	var airports: Array = sim.get_map_airports()
	for airport in airports:
		if airport["icao"] == "EDDF":
			if not _check(airport["slots"].contains("Slots belegt") and airport["slots"].contains("frei/Tag"), "Map has live slot detail"):
				return
		if airport["icao"] == "EDFH":
			if not _check(airport["slots"].contains("unbegrenzt"), "Secondary airport unlimited"):
				return
	print("PASS: slots API and planning presentation")
	main.queue_free()
	await process_frame
	quit(0)
