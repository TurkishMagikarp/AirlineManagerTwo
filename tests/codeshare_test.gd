extends SceneTree
func _initialize() -> void:
	call_deferred("_run")
func _run() -> void:
	TranslationServer.set_locale("de")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Codeshare UI", "DE", "EDDF").is_empty())
	assert(sim.lease_aircraft("a320_200").is_empty())
	assert(sim.create_route("EDDF", "LTFM", 1).is_empty())
	assert(sim.set_ticket_price(1, 120).is_empty())
	sim.hire_missing_staff()
	assert(sim.schedule_route(1, 1, 240).is_empty())
	assert(sim.lease_aircraft("a320_200").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 2).is_empty())
	assert(sim.set_ticket_price(2, 50).is_empty())
	sim.hire_missing_staff()
	assert(sim.schedule_route(2, 1, 240).is_empty())
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel.codeshare_panel
	assert(panel.empty.visible)
	var index := 0
	for row: Dictionary in panel.rows:
		if row["name"] == "Regional Wings": break
		index += 1
	panel.partner.select(index)
	panel.partner.item_selected.emit(index)
	panel.propose.pressed.emit()
	assert(panel.rows[index]["active"])
	assert(not panel.empty.visible)
	var route: Dictionary = sim.get_routes()[0]
	assert(sim.get_codeshares().any(func(c: Dictionary): return c["active"]))
	sim.start()
	assert(sim.advance(3.0 * 86400.0))
	sim.pause()
	panel.refresh()
	assert(panel.rows[index]["passengers"] >= 0)
	assert(panel.rows[index]["revenue"] >= 0)
	panel.end.pressed.emit()
	await process_frame
	assert(panel.dialog.visible)
	panel.dialog.danger_button.pressed.emit()
	await process_frame
	assert(not panel.rows[index]["active"])
	assert(panel.rows[index]["passengers"] >= 0)
	assert(sim.get_routes()[0]["connections"].all(func(c: Dictionary): return c["partner"].is_empty()))
	# Refusal travels through the same real propose button.
	assert(sim.stop_schedule(1).is_empty())
	assert(sim.stop_schedule(2).is_empty())
	panel.refresh()
	panel.propose.pressed.emit()
	assert(panel.result.text == tr("CODESHARE_NO_HUB"))
	main.queue_free()
	await process_frame
	print("PASS: codeshare proposal, state, counters, visible confirmed cancellation and refusal through real UI")
	quit(0)
