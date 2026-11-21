extends SceneTree

const UI := preload("res://scripts/ui/components.gd")
var failures := 0

func _initialize() -> void:
	call_deferred("_run")

func _check(ok: bool, what: String) -> void:
	if not ok:
		failures += 1
		push_error("FAIL: " + what)

func _agree(main: Control, sim: RefCounted, label: String) -> Dictionary:
	main._sync_view()
	main._show_page(4)
	main.finances_panel.refresh(true)
	var s: Dictionary = sim.get_financial_summary()
	var fin: Dictionary = main.finances_panel.metrics
	_check(s["company_revenue_eur"] == s["ticket_revenue_eur"] + s["cargo_revenue_eur"] + s["ancillary_revenue_eur"] + s["charter_revenue_eur"], label + " revenue components add up")
	_check(s["company_revenue_eur"] == s["revenue_eur"] + s["charter_revenue_eur"], label + " charter fee counted once")
	_check(s["company_costs_eur"] == s["costs_eur"] + s["charter_penalty_eur"], label + " charter penalty counted once")
	_check(s["company_revenue_eur"] - s["company_costs_eur"] == s["profit_eur"], label + " revenue - costs = result")
	_check(fin["revenue"].text == UI.money(s["company_revenue_eur"]) and fin["costs"].text == UI.money(s["company_costs_eur"]) and fin["profit"].text == UI.signed_money(s["profit_eur"]), label + " Finances totals")
	_check(fin["ticket"].text == UI.money(s["ticket_revenue_eur"]) and fin["cargo"].text == UI.money(s["cargo_revenue_eur"]) and fin["ancillary"].text == UI.money(s["ancillary_revenue_eur"]) and fin["charter"].text == UI.money(s["charter_revenue_eur"]) and fin["penalty"].text == UI.money(s["charter_penalty_eur"]), label + " Finances breakdown")
	var text: String = main.finance_breakdown.text
	for value in [s["company_revenue_eur"], s["company_costs_eur"], s["charter_revenue_eur"], s["charter_penalty_eur"]]:
		_check(text.contains(UI.money(value)), label + " Overview shows " + UI.money(value))
	_check(text.contains(UI.signed_money(s["profit_eur"])), label + " Overview result")
	return s

func _accept_charter(sim: RefCounted) -> int:
	for offer in sim.get_charter_offers():
		for option in sim.get_charter_aircraft_options(offer["id"]):
			if option["ok"] and sim.accept_charter(offer["id"], option["aircraft_id"]).is_empty():
				return offer["id"]
	return 0

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	_check(sim.found_airline("Results Air", "DE", "EDDF").is_empty(), "found")
	var s := _agree(main, sim, "fresh")
	_check(s["company_revenue_eur"] == 0 and s["company_costs_eur"] == 0 and s["profit_eur"] == 0, "fresh company is zero")
	_check(sim.lease_aircraft("a320_200").is_empty(), "lease")
	sim.hire_missing_staff()
	var fee_job := _accept_charter(sim)
	_check(fee_job > 0, "charter accepted")
	sim.start()
	for day in range(14):
		sim.advance(86400.0)
		if sim.get_charter_jobs().is_empty():
			break
	sim.pause()
	s = _agree(main, sim, "fee")
	_check(s["charter_revenue_eur"] > 0 and s["charter_penalty_eur"] == 0, "completed charter adds its fee to revenue")
	var penalty_job := _accept_charter(sim)
	if penalty_job > 0:
		_check(sim.cancel_charter(penalty_job).is_empty(), "charter cancelled")
		var after := _agree(main, sim, "penalty")
		_check(after["charter_penalty_eur"] > 0 and after["charter_revenue_eur"] == s["charter_revenue_eur"], "penalty is a cost, fee unchanged")
	var path := OS.get_environment("TMPDIR") + "/company_results.save"
	if OS.get_environment("TMPDIR").is_empty(): path = "user://company_results.save"
	var before: Dictionary = sim.get_financial_summary()
	_check(sim.save_game(path).is_empty() and sim.load_game(path).is_empty(), "save and reload")
	DirAccess.remove_absolute(path)
	s = _agree(main, sim, "reloaded")
	_check(s["company_revenue_eur"] == before["company_revenue_eur"] and s["company_costs_eur"] == before["company_costs_eur"] and s["profit_eur"] == before["profit_eur"], "totals survive reload")
	main.queue_free()
	await process_frame
	if failures == 0:
		print("PASS: company results reconcile in Overview and Finances")
	quit(1 if failures > 0 else 0)
