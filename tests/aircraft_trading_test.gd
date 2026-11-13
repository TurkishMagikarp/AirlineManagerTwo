extends SceneTree
## Disposal eligibility comes from Rust for both owned and leased aircraft.

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	for leased in [false, true]:
		var main = load("res://scenes/main.tscn").instantiate()
		root.add_child(main)
		main.set_process(false)
		var sim: RefCounted = main.simulation
		assert(sim.found_airline("Trading UI", "DE", "EDDF").is_empty())
		assert((sim.lease_aircraft("atr72_600") if leased else sim.buy_aircraft("atr72_600")).is_empty())
		assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
		sim.hire_missing_staff()
		assert(sim.schedule_route(1, 1, 120).is_empty())
		sim.start()
		assert(sim.advance(60.0 + 7200.0 * 2 + 100.0))
		main._sync_view()
		main._show_page(1)
		if not _blocked(main, leased, "in_flight"):
			return
		assert(sim.request_maintenance(1).is_empty())
		assert(sim.advance(3600.0))
		main._refresh_fleet()
		if not _blocked(main, leased, "maintenance"):
			return
		main.queue_free()
		await process_frame
	print("PASS: Fleet sale and lease return blocked airborne and in maintenance")
	quit(0)

func _blocked(main, leased: bool, status: String) -> bool:
	var sim: RefCounted = main.simulation
	var before: Array = sim.get_fleet()
	var cash: int = sim.get_airline_info()["capital_eur"]
	var plane: Dictionary = before[0]
	var error: String = sim.return_aircraft(1) if leased else sim.sell_aircraft(1)
	var ok: bool = plane["status"] == status and plane["leased"] == leased and not plane["disposal_error"].is_empty() and main.disposal_button.disabled and plane["disposal_error"] in main.disposal_details.text and main.disposal_button.text == ("Leasing zurückgeben" if leased else "Flugzeug verkaufen") and not error.is_empty() and sim.get_fleet() == before and sim.get_airline_info()["capital_eur"] == cash
	if not ok:
		push_error("FAIL: disposal blocked for leased=%s, status=%s" % [leased, status])
		quit(1)
	return ok
