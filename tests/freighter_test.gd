extends SceneTree
func _initialize() -> void:
	call_deferred("_run")
func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	await preload("res://tests/freighter_fixture.gd").setup(main, self)
	var sim: RefCounted = main.simulation
	var summary: Dictionary = sim.get_financial_summary()
	assert(summary["cargo_revenue_eur"] > 0 and summary["ticket_revenue_eur"] == 0)
	var flights: Array = sim.get_route_flight_page(1, 0)
	assert(not flights.is_empty())
	for flight in flights:
		assert(flight["cargo_revenue_eur"] > 0 and flight["passengers"] == 0 and flight["ticket_revenue_eur"] == 0)
	main._show_page(4)
	main.finances_panel.route_tree.get_root().get_first_child().select(0)
	main.finances_panel.route_tree.item_selected.emit()
	assert(main.finances_panel.flights_tree.get_root().get_first_child().get_text(2) == "—")
	root.size = Vector2i(680, 520)
	main._show_page(2)
	await process_frame
	var panel: Control = main.routes_panel
	panel.get_parent().get_parent().ensure_control_visible(panel.cargo_button)
	await process_frame
	assert(root.get_visible_rect().encloses(panel.cargo_button.get_global_rect()))
	main.queue_free()
	await process_frame
	print("PASS: real freighter purchase, used offer, route form, freight results and passenger controls at 680x520")
	quit(0)
