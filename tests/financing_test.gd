extends SceneTree
## Real GDExtension + Finance panel smoke, deterministic payments and disk restart.

const DAY := 86400.0
const FinancePanel = preload("res://scripts/finances_panel.gd")

func _initialize() -> void:
	call_deferred("_run")

func _check(ok: bool, message: String) -> bool:
	if not ok:
		push_error(message)
		quit(1)
	return ok

func _fresh() -> RefCounted:
	var sim: RefCounted = ClassDB.instantiate("SimulationClock")
	sim.found_airline("Finance test", "DE", "EDDF")
	return sim

func _run() -> void:
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-financing-test.json")
	var keep_save := false
	var restore := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--financing-save="):
			path = arg.trim_prefix("--financing-save=")
			keep_save = true
		if arg == "--restore-financing":
			restore = true
	var sim := _fresh()
	if restore:
		if not _check(sim.load_game(path).is_empty(), "Restore facility in a separate process"):
			return
		var info: Dictionary = sim.get_financing()
		if not _check(info["debt_eur"] == 1000000 and info["overdue_eur"] == 0, "Restored principal"):
			return
		sim.start()
		sim.set_speed(16)
		sim.advance(30.0 * DAY / 16.0)
		info = sim.get_financing()
		if not _check(info["debt_eur"] == 950000 and info["interest_eur"] == 4932, "Restored payment charged once"):
			return
		print("financing restart: PASS")
		quit(0)
		return
	var opening: int = sim.get_financing()["cash_eur"]
	if not _check(not sim.borrow_money(0).is_empty(), "Zero loan rejected"):
		return
	if not _check(sim.borrow_money(1200000).is_empty(), "Borrow via adapter"):
		return
	if not _check(sim.get_financing()["cash_eur"] == opening + 1200000, "Loan changes cash"):
		return
	if not _check(not sim.borrow_money(1).is_empty(), "No second simultaneous facility"):
		return
	var panel := FinancePanel.new()
	panel.simulation = sim
	root.add_child(panel)
	panel.refresh(true)
	if not _check("1.200.000" in panel.loan_status.text or panel.loan_status.text.contains("1,200,000") or panel.loan_status.text.contains("1200000"), "Debt visible: " + panel.loan_status.text):
		return
	panel.loan_amount.value = 200000
	panel._loan_action(false)
	if not _check(sim.get_financing()["debt_eur"] == 1000000, "Finance action repays"):
		return
	var save_error: String = sim.save_game(path)
	if not _check(save_error.is_empty(), "Save facility: " + save_error):
		return
	var loaded := _fresh()
	if not _check(loaded.load_game(path).is_empty(), "Load facility"):
		return
	if not _check(loaded.get_financing() == sim.get_financing(), "Facility and rules survive load"):
		return
	var expected: Dictionary = {}
	for speed in [1, 4, 16]:
		var candidate := _fresh()
		candidate.load_game(path)
		candidate.set_speed(speed)
		candidate.start()
		if not _check(candidate.advance(30.0 * DAY / speed), "Advance loan period"):
			return
		var info: Dictionary = candidate.get_financing()
		if not _check(info["debt_eur"] == 950000 and info["interest_eur"] > 0, "Interest and installment"):
			return
		var summary: Dictionary = candidate.get_financial_summary()
		var series: Array = candidate.get_finance_series(0)
		var chart_costs := 0
		for sample in series:
			chart_costs += int(sample["costs"])
		if not _check(summary["costs_eur"] == info["interest_eur"] and summary["profit_eur"] == -info["interest_eur"] and chart_costs == info["interest_eur"] and series[-1]["balance"] == info["cash_eur"], "Interest reaches profit/charts; principal is not an expense"):
			return
		if expected.is_empty():
			expected = info
		elif not _check(info == expected, "1x/4x/16x identical financing"):
			return
	# Isolated old-save-style fixture: fund the historical opening consistently,
	# then let Rust advance real distress/grace events and expose the deadline.
	var distress_path := path + ".distress"
	var distressed := _fresh()
	assert(distressed.borrow_money(1000000).is_empty())
	assert(distressed.save_game(distress_path).is_empty())
	var text := FileAccess.get_file_as_string(distress_path)
	var fixture: Dictionary = JSON.parse_string(text)
	var cash: int = fixture["state"]["airline"]["capital_eur"]
	var opening_balance: int = fixture["state"]["history"]["opening_balance"]
	var position := text.find('"capital_eur":', text.find('"name":"Finance test"'))
	text = text.left(position) + text.substr(position).replace('"capital_eur":%d' % cash, '"capital_eur":-1')
	position = text.rfind('"opening_balance":')
	text = text.left(position) + text.substr(position).replace('"opening_balance":%d' % opening_balance, '"opening_balance":%d' % (opening_balance - cash - 1))
	var file := FileAccess.open(distress_path, FileAccess.WRITE)
	file.store_string(text)
	file.close()
	assert(distressed.load_game(distress_path).is_empty())
	distressed.start()
	assert(distressed.advance(31.0 * DAY))
	var resolution: Dictionary = distressed.get_financing()
	panel.simulation = distressed
	panel.refresh(true)
	if not _check(resolution["deadline"] != "—" and resolution["can_reschedule"] and "Restrukturierungsfrist: " + resolution["deadline"] in panel.loan_status.text and not panel.loan_reschedule.disabled, "Finances displays the real Rust restructuring deadline and rescue action"):
		return
	DirAccess.remove_absolute(distress_path)
	panel.queue_free()
	if not keep_save:
		DirAccess.remove_absolute(path)
	print("financing_test: PASS")
	quit(0)
