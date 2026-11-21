extends SceneTree
## Alpha acceptance "Run and change the airline over two years": found a standard company through the UI, buy an ATR 72,
## hire crew, open EDDF–EDDM, fly with pause/speed/jumps, open a second line with a newly bought aircraft, change a fare,
## reach the start of Day 731 and show another landing of the same company booked afterwards.

const DAY := 86400.0
const YEAR1_END := 365.0 * DAY
const TARGET := 730.0 * DAY
const MAX_JUMPS := 80

var main
var sim: RefCounted
var panel
var log_lines: Array[String] = []


func _initialize() -> void:
	call_deferred("_run")


func _fail(message: String) -> void:
	push_error("FAIL: " + message)
	quit(1)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_fail(message)
	return condition


func _click(button: Button) -> void:
	for i in range(main.fleet_sections.size()):
		if main.fleet_sections[i].is_ancestor_of(button):
			main.fleet_tabs.current_tab = i
	await process_frame
	var parent := button.get_parent()
	while parent != null:
		if parent is ScrollContainer:
			parent.ensure_control_visible(button)
		parent = parent.get_parent()
	for _i in range(3):
		await process_frame
	var motion := InputEventMouseMotion.new()
	for attempt in range(2):
		if attempt == 1:
			# Centre the control in its scroll area so overlays at the window edge do not cover it.
			parent = button.get_parent()
			while parent != null:
				if parent is ScrollContainer:
					parent.scroll_vertical += int(button.get_global_rect().get_center().y - parent.get_global_rect().get_center().y)
				parent = parent.get_parent()
			for _i in range(3):
				await process_frame
		motion.position = button.get_global_rect().get_center()
		motion.global_position = motion.position
		Input.parse_input_event(motion)
		await process_frame
		if root.gui_get_hovered_control() == button:
			break
	if root.gui_get_hovered_control() != button:
		_fail("Button %s (%s) not reachable at %s; hovered %s" % [button.name, button.text, str(button.get_global_rect()), str(root.gui_get_hovered_control())])
		return
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame


func _select_destination(icao: String) -> bool:
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == icao:
			panel.destination.select(i)
			panel.destination.item_selected.emit(i)
			return true
	return false


func _buy_atr() -> bool:
	await _click(main.navigation[1])
	for frame in range(3):
		await process_frame
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	var before: int = sim.get_fleet().size()
	await _click(main.buy_button)
	return _check(sim.get_fleet().size() == before + 1, "ATR 72 bought through Fleet (fleet %d, day %s, cash %s): %s | %s" % [before, sim.get_display_time(), str(sim.get_airline_info().get("capital_eur")), main.fleet_status.text, main.buy_button.tooltip_text + " disabled=" + str(main.buy_button.disabled) + " vis=" + str(main.buy_button.is_visible_in_tree()) + " sel=" + str(main.model_select.selected) + " rect=" + str(main.buy_button.get_global_rect()) + " page=" + str(main.current_page) + " cash=" + str(sim.get_financial_summary()["profit_eur"])])


func _hire_crew() -> bool:
	await _click(main.navigation[5])
	for frame in range(3):
		await process_frame
	await _click(main.staff_panel.hire_missing_button)
	await process_frame
	await _click(main.navigation[2])
	return true


func _open_route(destination: String, aircraft_index: int, ticket: int) -> int:
	await _click(main.navigation[2])
	panel.refresh()
	for i in range(panel.origin.item_count):
		if panel.origin.get_item_metadata(i) == "EDDF":
			panel.origin.select(i)
			panel.origin.item_selected.emit(i)
	var offered: Array = []
	for i in range(panel.destination.item_count):
		offered.append(panel.destination.get_item_metadata(i))
	if not _check(_select_destination(destination), "Destination %s offered among %s" % [destination, str(offered)]):
		return 0
	if aircraft_index >= 0 and panel.aircraft.item_count > aircraft_index:
		panel.aircraft.select(aircraft_index)
		panel.aircraft.item_selected.emit(aircraft_index)
	var before: int = sim.get_routes().size()
	await _click(panel.create_button)
	for _i in range(5):
		await process_frame
	if not _check(sim.get_routes().size() == before + 1, "Route EDDF–%s created: %s %s disabled=%s tip=%s aircraft=%d/%d vis=%s" % [destination, panel.status.text, panel.preview.text, panel.create_button.disabled, panel.create_button.tooltip_text, panel.aircraft.selected, panel.aircraft.item_count, panel.create_button.is_visible_in_tree()]):
		return 0
	var route: Dictionary = panel._route()
	panel.ticket_input.value = ticket
	await _click(panel.ticket_button)
	await _hire_crew()
	panel.refresh()
	panel.focus_route(route["id"])
	panel.delay_input.value = 3
	panel.interval_input.value = 120
	await _click(panel.schedule_button)
	for _i in range(3):
		await process_frame
	var fresh: Dictionary = sim.get_routes().filter(func(r): return r["id"] == route["id"])[0]
	if not _check(fresh["has_plan"] and fresh["ticket_eur"] == ticket and not fresh["aircraft_ids"].is_empty(), "Route %d scheduled with fare %d" % [route["id"], ticket]):
		return 0
	return route["id"]


## One jump through the clock bar. Returns the stop (attention) kind or "".
func _jump(index: int) -> String:
	var before: float = sim.get_elapsed_seconds()
	await _click(main.jump_buttons[index])
	var guard := 0
	while main.jump_in_progress:
		guard += 1
		if guard > 20000:
			_fail("Jump %d never finished at %s" % [index, sim.get_display_time()])
			return "error"
		await process_frame
	await process_frame
	if sim.get_elapsed_seconds() <= before:
		var rows: Array = sim.get_attention_snapshot()
		if rows.is_empty():
			_fail("No progress after jump %d at %s: %s" % [index, sim.get_display_time(), main.jump_status.text])
			return "error"
	return await _handle_stop()


## Deal with an Attention stop through the offered actions.
func _handle_stop() -> String:
	var kind := ""
	var attention: Dictionary = main.jump_attention_target
	if main.jump_attention_button.visible and not attention.is_empty():
		kind = str(attention.get("kind", "attention"))
		log_lines.append("stop at %s: %s" % [sim.get_display_time(), main.jump_status.text])
		await _click(main.jump_attention_button)
		for _i in range(3):
			await process_frame
		if main.current_page == 5 or kind.contains("crew") or kind.contains("staff"):
			await _click(main.staff_panel.hire_missing_button)
	return kind


func _jump_until(seconds: float) -> bool:
	for step in range(MAX_JUMPS):
		var remaining: float = seconds - sim.get_elapsed_seconds()
		if remaining <= 0.0:
			return true
		var index := 3 if remaining >= 30.0 * DAY else (2 if remaining >= 7.0 * DAY else 1)
		var result: String = await _jump(index)
		if result == "error":
			return false
	return _check(false, "Target %.0f not reached within %d jumps, at %s" % [seconds, MAX_JUMPS, sim.get_display_time()])


func _day(landing: String) -> int:
	return int(landing.split(" ")[1])


func _player_landings() -> Array:
	var all: Array = []
	for route in sim.get_routes() + []:
		var offset := 0
		while true:
			var page: Array = sim.get_route_flight_page(route["id"], offset)
			all.append_array(page)
			if page.size() < 50:
				break
			offset += page.size()
	return all


func _run() -> void:
	TranslationServer.set_locale("de")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	sim = main.simulation
	panel = main.routes_panel
	root.size = Vector2i(1440, 1000)
	await process_frame
	main.name_input.text = "Zweijahr Air"
	await _click(main.found_button)
	var info: Dictionary = sim.get_airline_info()
	if not _check(info["airport_icao"] == "EDDF" and sim.get_elapsed_seconds() == 0.0 and not sim.is_running(), "Founded at EDDF, paused Day 1"):
		return
	var start_cash: int = info["capital_eur"]
	if not await _buy_atr():
		return
	var route_a: int = await _open_route("EDDM", 0, 100)
	if route_a == 0:
		return
	# Start, speed change, pause, jumps.
	await _click(main.start_button)
	if not _check(sim.is_running(), "Start runs the clock"):
		return
	await _click(main.speed_buttons[1])
	if not _check(sim.get_speed() == 4, "Speed 4x selected"):
		return
	for _i in range(30):
		main._process(0.5)
		await process_frame
	await _click(main.pause_button)
	var paused_at: float = sim.get_elapsed_seconds()
	await process_frame
	if not _check(not sim.is_running() and sim.get_elapsed_seconds() == paused_at and paused_at > 0.0, "Pause stops the clock after running"):
		return
	if not await _jump_until(30.0 * DAY):
		return
	var landed_first: int = sim.get_route_flight_page(route_a, 0).size()
	if not _check(landed_first >= 10, "First route flew during month 1 (%d landings)" % landed_first):
		return
	# Second line with a newly bought aircraft.
	if not await _buy_atr():
		return
	var route_b: int = await _open_route("LSZH", 1, 110)
	if route_b == 0:
		return
	var routes_before: Array = sim.get_routes()
	if not _check(routes_before.size() == 2 and routes_before[0]["aircraft_ids"] != routes_before[1]["aircraft_ids"], "Second line flies with the new aircraft"):
		return
	if not await _jump_until(YEAR1_END):
		return
	var year1: Dictionary = sim.get_financial_summary()
	var year1_flights: int = year1["flights"]
	if not _check(year1["company_revenue_eur"] > 0 and year1["company_costs_eur"] > 0 and year1_flights > 0, "Year 1 booked revenue and costs"):
		return
	# Change the fare on the first route through the UI.
	await _click(main.navigation[2])
	panel.refresh()
	panel.focus_route(route_a)
	panel.ticket_input.value = 120
	await _click(panel.ticket_button)
	var changed: Dictionary = sim.get_routes().filter(func(r): return r["id"] == route_a)[0]
	if not _check(changed["ticket_eur"] == 120, "Fare change applied on first route"):
		return
	if not await _jump_until(TARGET):
		return
	var elapsed: float = sim.get_elapsed_seconds()
	if not _check(elapsed >= TARGET, "Reached Day 731 (elapsed %.0f)" % elapsed):
		return
	var year2: Dictionary = sim.get_financial_summary()
	var landings: Array = _player_landings()
	var in_y1 := 0
	var in_y2 := 0
	for flight in landings:
		var day := _day(flight["landing"])
		if day <= 365:
			in_y1 += 1
		elif day <= 730:
			in_y2 += 1
	if not _check(in_y1 > 0 and in_y2 > 0, "Dated landings in both years (%d / %d)" % [in_y1, in_y2]):
		return
	var rev2: int = year2["company_revenue_eur"] - year1["company_revenue_eur"]
	var cost2: int = year2["company_costs_eur"] - year1["company_costs_eur"]
	if not _check(rev2 > 0 and cost2 > 0, "Year 2 booked revenue %d and costs %d" % [rev2, cost2]):
		return
	# Finances and Routes show the totals.
	await _click(main.navigation[4])
	for _i in range(3):
		await process_frame
	await _click(main.navigation[2])
	var finance: Array = sim.get_route_finances()
	var flown := 0
	for r in finance:
		flown += r["flights"]
	if not _check(flown > year1_flights and flown == year2["flights"], "Route finances reconcile with company flights"):
		return
	# Keep playing the same company: one more landing and its booking.
	var total_before: int = _player_landings().size()
	var revenue_before: int = year2["company_revenue_eur"]
	if not await _jump_until(sim.get_elapsed_seconds() + DAY):
		return
	var after: Dictionary = sim.get_financial_summary()
	var landings_after: Array = _player_landings()
	if not _check(landings_after.size() > total_before and after["flights"] > year2["flights"] and after["company_revenue_eur"] > revenue_before, "Another flight of the same company landed and was booked"):
		return
	print("INFO start_cash=%d y1_flights=%d y1_rev=%d y1_cost=%d y2_rev=%d y2_cost=%d landings=%d/%d day=%d" % [start_cash, year1_flights, year1["company_revenue_eur"], year1["company_costs_eur"], rev2, cost2, in_y1, in_y2, int(sim.get_elapsed_seconds() / DAY) + 1])
	for line in log_lines:
		print("INFO " + line)
	print("PASS: two-year passenger airline through the UI")
	quit(0)
