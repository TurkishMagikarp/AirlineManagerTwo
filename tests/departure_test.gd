extends SceneTree
## Real extension: frozen departure results survive a fresh process and supply changes.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var sim = ClassDB.instantiate("SimulationClock")
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-departure-test.json")
	if "--restore-departure" in OS.get_cmdline_user_args():
		if not _check(sim.load_game(path).is_empty(), "Load airborne departure in fresh process"):
			return
	else:
		sim.found_airline("Departure Air", "DE", "EDDF")
		for id in [1, 2]:
			sim.buy_aircraft("atr72_600")
			sim.create_route("EDDF", "EDDM", id)
		sim.schedule_route(1, 1, 120)
		sim.schedule_route(2, 30, 120)
		sim.start()
		sim.advance(60.0)
		if not _check(sim.get_active_flights().size() == 1 and sim.get_financial_summary()["flights"] == 0, "Departure fixed before settlement"):
			return
		if not _check(sim.stop_schedule(2).is_empty(), "Change supply during active flight"):
			return
		if not _check(sim.save_game(path).is_empty(), "Save immutable departure snapshot"):
			return
	var frozen: Dictionary = sim.get_active_flights()[0]
	sim.start()
	sim.advance(10.0)
	if not _check(sim.get_active_flights()[0] == frozen, "Supply change does not rewrite flight"):
		return
	sim.advance(3000.0)
	if not _check(sim.get_flight_results()[0] == frozen and sim.get_financial_summary()["flights"] == 1, "Landing settles exact snapshot once"):
		return
	sim.advance(0.0)
	if not _check(sim.get_financial_summary()["flights"] == 1, "No double settlement"):
		return
	sim.advance(5000.0)
	var back: Dictionary = sim.get_active_flights()[0]
	if not _check(back["origin"] == frozen["destination"] and back["destination"] == frozen["origin"] and back["passengers"] > frozen["passengers"], "Return direction uses updated supply only at next departure"):
		return
	print("PASS: departure snapshot, supply change, exact settlement, return direction and save/restart")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
