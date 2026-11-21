extends SceneTree

const UI := preload("res://scripts/ui/components.gd")
var failures := 0

func _initialize() -> void:
	call_deferred("_run")

func _check(ok: bool, what: String) -> void:
	if not ok:
		failures += 1
		push_error("FAIL: " + what)

func _flight_ok(f: Dictionary, label: String) -> void:
	_check(f["ticket_revenue_eur"] + f["cargo_revenue_eur"] + f["ancillary_revenue_eur"] == f["revenue_eur"], label + " receipts sum to revenue")
	_check(f["revenue_eur"] - f["costs_eur"] == f["profit_eur"], label + " revenue - costs = result")
	var cost_sum: int = f["fuel_eur"] + f["carbon_eur"] + f["airport_eur"] + f["crew_eur"] + f["overhead_eur"] + f["service_eur"] + f["care_eur"] + f["loyalty_cost_eur"]
	_check(cost_sum == f["costs_eur"], label + " cost parts sum to costs")

func _tooltip_has(item: TreeItem, column: int, parts: Array, label: String) -> void:
	var text := item.get_tooltip_text(column)
	for part in parts:
		_check(text.contains("%s: %s" % [part[0], part[1]]), "%s tooltip shows %s" % [label, part[0]])

func _compare(main: Control, sim: RefCounted, expect_cargo: bool, expect_ancillary: bool, expect_loyalty: bool, label: String) -> void:
	main._sync_view()
	main._show_page(2)
	await process_frame
	var panel: Control = main.routes_panel
	var results: Array = sim.get_flight_results()
	_check(not results.is_empty(), label + " has landings")
	var item: TreeItem = panel.results_tree.get_root().get_first_child()
	var seen_cargo := false
	var seen_ancillary := false
	var seen_loyalty := false
	for r in results:
		_flight_ok(r, label + " result")
		seen_cargo = seen_cargo or r["cargo_revenue_eur"] > 0
		seen_ancillary = seen_ancillary or r["ancillary_revenue_eur"] > 0
		seen_loyalty = seen_loyalty or r["loyalty_cost_eur"] > 0
		_check(item.get_text(3) == UI.money(r["revenue_eur"]), label + " Routes revenue cell")
		_check(item.get_text(4) == UI.money(r["costs_eur"]), label + " Routes cost cell")
		_check(item.get_text(5) == UI.money(r["profit_eur"]), label + " Routes result cell")
		_tooltip_has(item, 3, UI.revenue_parts(r), label + " Routes revenue")
		_tooltip_has(item, 4, UI.cost_parts(r), label + " Routes cost")
		item = item.get_next()
	_check(seen_cargo == expect_cargo, label + " cargo presence")
	_check(seen_ancillary == expect_ancillary, label + " ancillary presence")
	_check(seen_loyalty == expect_loyalty, label + " loyalty presence")
	main._show_page(4)
	await process_frame
	var fin: Control = main.finances_panel
	var route: Dictionary = sim.get_route_finances()[0]
	_flight_ok(route, label + " route")
	var route_row: TreeItem = fin.route_tree.get_root().get_first_child()
	_check(route_row.get_text(2) == UI.money(route["revenue_eur"]) and route_row.get_text(3) == UI.money(route["costs_eur"]) and route_row.get_text(4) == UI.money(route["profit_eur"]), label + " Finances route row")
	_tooltip_has(route_row, 2, UI.revenue_parts(route), label + " route revenue")
	_tooltip_has(route_row, 3, UI.cost_parts(route), label + " route cost")
	route_row.select(0)
	fin.route_tree.item_selected.emit()
	var page: Array = sim.get_route_flight_page(route["route_id"], 0)
	var flight_row: TreeItem = fin.flights_tree.get_root().get_first_child()
	var revenue := 0
	var costs := 0
	for f in page:
		_flight_ok(f, label + " archive")
		revenue += f["revenue_eur"]
		costs += f["costs_eur"]
		_check(flight_row.get_text(5) == UI.money(f["revenue_eur"]) and flight_row.get_text(6) == UI.money(f["costs_eur"]) and flight_row.get_text(7) == UI.money(f["profit_eur"]), label + " archive row")
		_tooltip_has(flight_row, 5, UI.revenue_parts(f), label + " archive revenue")
		_tooltip_has(flight_row, 6, UI.cost_parts(f), label + " archive cost")
		flight_row = flight_row.get_next()
	_check(page.size() == route["flights"], label + " page holds all landings")
	_check(revenue == route["revenue_eur"] and costs == route["costs_eur"], label + " flights add up to route total")
	var latest: Dictionary = page[0]
	for r in results:
		if r["landing"] == latest["landing"] and r["route_id"] == route["route_id"]:
			_check(r["revenue_eur"] == latest["revenue_eur"] and r["costs_eur"] == latest["costs_eur"] and r["profit_eur"] == latest["profit_eur"], label + " screens agree on the same landing")
	var company: Dictionary = sim.get_financial_summary()
	_check(company["company_revenue_eur"] == company["ticket_revenue_eur"] + company["cargo_revenue_eur"] + company["ancillary_revenue_eur"] + company["charter_revenue_eur"], label + " company receipts sum to company revenue")
	_check(company["company_revenue_eur"] - company["company_costs_eur"] == company["profit_eur"], label + " company revenue - costs = result")
	_check(fin.metrics["revenue"].text == UI.money(company["company_revenue_eur"]) and fin.metrics["costs"].text == UI.money(company["company_costs_eur"]) and fin.metrics["profit"].text == UI.signed_money(company["profit_eur"]), label + " Finances company totals")
	_check(main.finance_breakdown.text.contains(UI.money(company["company_revenue_eur"])) and main.finance_breakdown.text.contains(UI.money(company["company_costs_eur"])) and main.finance_breakdown.text.contains(UI.signed_money(company["profit_eur"])), label + " Overview company totals")
	_check(fin.flights_tree.get_column_title(2) == "Ticketpreis je Pax", label + " ticket price label")
	_check(fin.detail.text.contains("zentrale Gehälter und Wartungschecks"), label + " central costs explained")

func _passenger(main: Control, fees: String, loyalty: String) -> RefCounted:
	var sim: RefCounted = main.simulation
	_check(sim.found_airline("Reconcile", "DE", "EDDF").is_empty(), "found")
	_check(sim.buy_aircraft("atr72_600").is_empty(), "buy")
	sim.hire_missing_staff()
	_check(sim.create_route("EDDF", "EDDM", 1).is_empty(), "route")
	_check(sim.schedule_route(1, 1, 120).is_empty(), "schedule")
	_check(sim.set_ancillary_policy(fees).is_empty(), "fees")
	_check(sim.set_loyalty_level(loyalty).is_empty(), "loyalty")
	sim.start()
	sim.advance(86400.0)
	sim.pause()
	return sim

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim := _passenger(main, "standard", "basic")
	await _compare(main, sim, true, true, true, "passenger")
	var path := OS.get_environment("TMPDIR") + "/flight_reconciliation.save"
	if OS.get_environment("TMPDIR").is_empty(): path = "user://flight_reconciliation.save"
	var before: Array = sim.get_flight_results()
	_check(sim.save_game(path).is_empty(), "save")
	_check(sim.load_game(path).is_empty(), "load")
	_check(sim.get_flight_results() == before, "recorded amounts intact after reload")
	await _compare(main, sim, true, true, true, "reloaded")
	DirAccess.remove_absolute(path)
	root.size = Vector2i(680, 520)
	main._show_page(2)
	await process_frame
	var panel: Control = main.routes_panel
	panel.get_parent().get_parent().ensure_control_visible(panel.results_tree)
	await process_frame
	_check(panel.results_tree.get_column_width(3) > 0 and panel.results_tree.get_column_title(3) == "Gesamtumsatz", "Routes revenue column present at 680x520")
	main.queue_free()
	await process_frame

	var main2 = load("res://scenes/main.tscn").instantiate()
	root.add_child(main2)
	main2.set_process(false)
	var sim2 := _passenger(main2, "included", "none")
	await _compare(main2, sim2, true, false, false, "no-fees")
	main2.queue_free()
	await process_frame

	var main3 = load("res://scenes/main.tscn").instantiate()
	root.add_child(main3)
	main3.set_process(false)
	await preload("res://tests/freighter_fixture.gd").setup(main3, self)
	var sim3: RefCounted = main3.simulation
	for f in sim3.get_route_flight_page(1, 0):
		_check(f["ticket_revenue_eur"] == 0 and f["cargo_revenue_eur"] > 0, "freighter receipts")
	await _compare(main3, sim3, true, false, false, "freighter")
	main3.queue_free()
	await process_frame
	if failures > 0:
		quit(1)
		return
	print("PASS: revenue, costs and results reconcile in Routes and Finances, after reload, without fees and for a freighter")
	quit(0)
