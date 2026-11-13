extends SceneTree
## Deterministic adapter + existing UI; optional save/restore in separate processes.

func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var simulation: RefCounted = main.simulation
	var path := OS.get_environment("LOCALAPPDATA").path_join("Temp/opencode/airline-economy-test.json")
	var restore := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--economy-save="):
			path = arg.trim_prefix("--economy-save=")
		if arg == "--restore-economy":
			restore = true
	if restore:
		if not _check(simulation.load_game(path).is_empty(), "Load financial save in fresh process"):
			return
	else:
		if not _check(simulation.found_airline("Money Air", "DE", "EDDF").is_empty(), "Found company"):
			return
		simulation.buy_aircraft("atr72_600")
		simulation.create_route("EDDF", "EDDM", 1)
		simulation.set_ticket_price(1, 0)
		simulation.hire_missing_staff()
		simulation.schedule_route(1, 1, 120)
		simulation.set_speed(16)
		simulation.start()
		var flight: int = simulation.get_operations()[0]["flight_seconds"]
		simulation.advance((60.0 + flight) / 16.0)
		var full: Dictionary = simulation.get_flight_results()[0]
		if not _check(full["passengers"] == full["seats"] and full["load_percent"] == 100.0 and full["revenue_eur"] == 0 and full["profit_eur"] < 0, "Full free flight loses money"):
			return
		simulation.advance((7200.0 - flight) / 16.0)
		simulation.advance((flight + 1800.0) / 16.0)
		if not _check(simulation.stop_schedule(1).is_empty(), "Stop after returning home"):
			return
		simulation.set_ticket_price(1, 100000)
		simulation.hire_missing_staff()
		simulation.schedule_route(1, 1, 120)
		simulation.advance((60.0 + flight) / 16.0)
		simulation.pause()
		if not _check(simulation.save_game(path).is_empty(), "Save prices, results, totals and money"):
			return
		if not _check(simulation.load_game(path).is_empty(), "Reload without extra settlement"):
			return
	var results: Array = simulation.get_flight_results()
	var totals: Dictionary = simulation.get_financial_summary()
	if not _check(results.size() == 3 and totals["flights"] == 3 and simulation.get_routes()[0]["ticket_eur"] == 100000, "History, aggregates and price restored"):
		return
	var revenue := 0
	var costs := 0
	for result in results:
		if not _check(result["overhead_eur"] > 0 and result["costs_eur"] == result["fuel_eur"] + result["airport_eur"] + result["crew_eur"] + result["overhead_eur"], "All cost components including operating allowance reconcile"):
			return
		revenue += result["revenue_eur"]
		costs += result["costs_eur"]
	if not _check(results[0]["passengers"] == 0 and results[0]["load_percent"] == 0.0 and results[0]["costs_eur"] > 0, "Empty flight still incurs operating costs"):
		return
	if not _check(totals["revenue_eur"] == revenue and totals["costs_eur"] == costs and simulation.get_airline_info()["capital_eur"] == 7000000 + revenue - costs, "Every money movement reconciles"):
		return
	main._sync_view()
	main._show_page(0)
	if not _check(main.metrics["profit"].is_visible_in_tree() and main.metrics["profit"].text == main.UI.money(totals["profit_eur"]), "Restored finances visible in company overview"):
		return
	if not _check(main.finance_breakdown.is_visible_in_tree() and "Wartung / Verwaltung" in main.finance_breakdown.text and main.UI.money(totals["overhead_eur"]) in main.finance_breakdown.text, "Operating allowance is transparently displayed"):
		return
	main._show_page(4)
	var fuel_note: Label = main.finances_panel.note
	if not _check(fuel_note.is_visible_in_tree() and ("Treibstoff weltweit: %.2f EUR/kg" % (totals["fuel_cents_per_kg"] / 100.0)) in fuel_note.text and ("%d Cent/kg gegenüber vor 30 Tagen" % totals["fuel_month_delta_cents"]) in fuel_note.text, "Fuel display forwards the Rust global price and monthly change"):
		return
	simulation.start()
	simulation.advance(7200.0 / 16.0)
	if not _check(simulation.get_financial_summary()["flights"] == 4, "Loaded 16x plan continues without duplicate charges"):
		return
	main.queue_free()
	await process_frame
	print("PASS: full/empty flights, money reconciliation, financial save/load and 16x continuation")
	quit(0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
