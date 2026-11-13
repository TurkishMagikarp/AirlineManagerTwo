extends SceneTree
## Staff rules through the real adapter: shortages block flights and checks,
## salaries reach the finances, step size does not matter. Optional save/restore
## in separate processes (`--staff-save=PATH`, `--restore-staff`).

const DAY := 86400.0

func _initialize() -> void:
	call_deferred("_run")


func _fresh(name: String) -> RefCounted:
	var sim: RefCounted = ClassDB.instantiate("SimulationClock")
	if not sim.found_airline(name, "DE", "EDDF").is_empty():
		return null
	sim.buy_aircraft("atr72_600")
	sim.create_route("EDDF", "EDDM", 1)
	return sim


func _run() -> void:
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-staff-test.json")
	var restore := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--staff-save="):
			path = arg.trim_prefix("--staff-save=")
		if arg == "--restore-staff":
			restore = true
	if restore:
		await _restore(path)
		return

	var sim := _fresh("Staff UI Air")
	if not _check(sim != null, "Found company"):
		return
	# Shortage: no staff, so no plan and no flights.
	var overview: Dictionary = sim.get_staff_overview()
	if not _check(overview["employees"] == 0 and overview["missing_total"] > 0, "New airline shows missing staff"):
		return
	var error: String = sim.schedule_route(1, 1, 120)
	if not _check("Besatzung fehlt" in error and "4 Piloten" in error, "Schedule refused without crew: " + error):
		return
	var route: Dictionary = sim.get_routes()[0]
	if not _check(not route["can_schedule"] and not String(route["staff_status"]).is_empty(), "Route reports the staffing shortage"):
		return
	sim.set_speed(16)
	sim.start()
	sim.advance(600.0)
	sim.pause()
	if not _check(sim.get_active_flights().is_empty() and sim.get_financial_summary()["flights"] == 0, "No flight departs without crew"):
		return
	# Hiring: invalid requests change nothing, a partial crew is still short.
	if not _check(not sim.hire_staff("pilot", 0).is_empty() and not sim.hire_staff("captain", 1).is_empty(), "Invalid hires rejected"):
		return
	if not _check(sim.hire_staff("pilot", 4).is_empty() and sim.hire_staff("cabin_crew", 2).is_empty(), "Hire pilots and part of the cabin crew"):
		return
	if not _check("1 Flugbegleiter" in sim.schedule_route(1, 1, 120), "Still short of one cabin crew member"):
		return
	if not _check(sim.hire_missing_staff().is_empty() and sim.get_staff_overview()["missing_total"] == 0, "Hire exactly what is missing"):
		return
	if not _check(sim.schedule_route(1, 1, 120).is_empty(), "Staffed aircraft can be scheduled"):
		return
	var pilot_id := 0
	for employee in sim.get_staff():
		if employee["role"] == "pilot":
			if not employee["can_dismiss"]:
				pilot_id = employee["id"]
	if not _check(not sim.dismiss_staff(pilot_id).is_empty(), "Assigned pilot cannot be dismissed"):
		return
	var plane: Dictionary = sim.get_staff_overview()["aircraft"][0]
	if not _check(plane["state"] == "crewed", "Aircraft shows its assigned crew"):
		return

	# Time acceleration: payroll and balances must not depend on the step size.
	var coarse := _fresh("Step Air")
	var fine := _fresh("Step Air")
	for other in [coarse, fine]:
		other.hire_missing_staff()
		other.schedule_route(1, 1, 120)
		other.set_speed(16)
		other.start()
	coarse.advance(DAY * 70.0 / 16.0)
	for i in range(700):
		fine.advance(DAY / 10.0 / 16.0)
	var a: Dictionary = coarse.get_airline_info()
	var b: Dictionary = fine.get_airline_info()
	var summary: Dictionary = coarse.get_financial_summary()
	if not _check(a["capital_eur"] == b["capital_eur"] and summary == fine.get_financial_summary(), "Same state for different step sizes"):
		return
	if not _check(summary["payroll_eur"] == 2 * coarse.get_staff_overview()["monthly_payroll_eur"], "Two payrolls in the finances after 70 days"):
		return
	var dismissed: String = coarse.dismiss_staff(coarse.get_staff()[-1]["id"])
	if not _check(not dismissed.is_empty() or coarse.get_staff_overview()["employees"] > 0, "Dismissal obeys assignment"):
		return

	# Maintenance without technicians: the check waits in the hangar.
	var hangar := _fresh("Hangar Air")
	hangar.hire_staff("pilot", 4)
	hangar.hire_staff("cabin_crew", 3)
	hangar.schedule_route(1, 1, 120)
	hangar.set_speed(16)
	hangar.start()
	hangar.advance(7300.0 / 16.0)
	if not _check(hangar.request_maintenance(1).is_empty(), "Order a check"):
		return
	hangar.advance(8000.0 / 16.0)
	var row: Dictionary = hangar.get_fleet()[0]
	if not _check(row["status"] == "maintenance" and row["maintenance_waiting_staff"], "Check waits for technicians"):
		return
	if not _check("Techniker" in hangar.get_routes()[0]["maintenance_status"], "Route shows the waiting check"):
		return
	hangar.advance(3.0 * DAY / 16.0)
	if not _check(hangar.get_fleet()[0]["maintenance_waiting_staff"], "Waiting does not consume hangar time"):
		return
	if not _check(hangar.hire_staff("technician", 3).is_empty() and not hangar.get_fleet()[0]["maintenance_waiting_staff"], "Hiring technicians starts the check"):
		return
	hangar.pause()
	if not _check(hangar.save_game(path).is_empty(), "Save mid-maintenance with staff"):
		return
	print("PASS: staff shortages, hiring, salaries, step-size equality and technician waiting")
	quit(0)


func _restore(path: String) -> void:
	var sim: RefCounted = ClassDB.instantiate("SimulationClock")
	if not _check(sim.load_game(path).is_empty(), "Load staff save in fresh process"):
		return
	var before: Dictionary = sim.get_staff_overview()
	if not _check(before["employees"] == 10 and sim.get_fleet()[0]["status"] == "maintenance", "Staff and check restored"):
		return
	sim.set_speed(16)
	sim.start()
	sim.advance(3.0 * DAY / 16.0)
	if not _check(sim.get_fleet()[0]["status"] == "parked" and sim.get_routes()[0]["has_plan"], "Check ends and the plan resumes with the same crew"):
		return
	sim.advance(40.0 * DAY / 16.0)
	if not _check(sim.get_financial_summary()["payroll_eur"] > 0, "Salaries continue after restart"):
		return
	print("PASS: staff state survives restart and continues")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
