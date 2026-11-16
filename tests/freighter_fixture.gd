extends RefCounted
## Drive the real Fleet purchase controls and route creation/schedule form.
static func setup(main: Control, tree: SceneTree) -> void:
	var sim: RefCounted = main.simulation
	if sim.get_airline_info().is_empty(): assert(sim.found_airline("Freighter UI", "DE", "EDDF").is_empty())
	main._sync_view()
	main._show_page(1)
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600f":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	main.buy_button.pressed.emit()
	await tree.process_frame
	assert(sim.get_fleet().size() == 1)
	assert(sim.sell_aircraft(1).is_empty())
	main._sync_view()
	main.buy_button.pressed.emit()
	await tree.process_frame
	assert(sim.get_fleet().size() == 1)
	main._sync_view()
	main._show_page(2)
	var panel: Control = main.routes_panel
	for select in [panel.origin, panel.destination]:
		for i in range(select.item_count):
			if select.get_item_metadata(i) == ("EDDF" if select == panel.origin else "EDDM"):
				select.select(i)
				select.item_selected.emit(i)
	panel.aircraft.select(0)
	panel.aircraft.item_selected.emit(0)
	assert(not panel.create_button.disabled)
	panel.create_button.pressed.emit()
	await tree.process_frame
	var row: TreeItem = panel.route_tree.get_root().get_first_child()
	row.select(0)
	panel.route_tree.item_selected.emit()
	assert(sim.hire_missing_staff().is_empty())
	panel.delay_input.value = 1
	panel.interval_input.value = 1440
	panel.schedule_button.pressed.emit()
	await tree.process_frame
	assert(sim.get_routes()[0]["has_plan"])
	sim.set_speed(16)
	sim.start()
	for day in range(20): sim.advance(86400.0 / 16.0)
	sim.pause()
	main._sync_view()
	row = panel.route_tree.get_root().get_first_child()
	row.select(0)
	panel.route_tree.item_selected.emit()
	var route: Dictionary = sim.get_routes()[0]
	assert(route["freighter_only"] and route["actual_cargo_kg"] > 0 and route["actual_cargo_revenue_eur"] > 0)
	assert(panel.ticket_button.disabled and panel.business_button.disabled and panel.service_select.disabled)
	assert(not panel.fare_box.visible)
	assert(sim.get_financial_summary()["ticket_revenue_eur"] == 0)
