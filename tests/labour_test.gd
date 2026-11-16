extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var sim: RefCounted = ClassDB.instantiate("SimulationClock")
	assert(sim.found_airline("Labour UI", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.hire_missing_staff().is_empty())
	var overview: Dictionary = sim.get_staff_overview()
	assert(overview["roles"][0]["morale"] == 75)
	var quote: Dictionary = sim.preview_staff_pay("pilot", 85)
	assert(quote["cost"] == 22100 and quote["delta"] == -3900)
	assert(not sim.set_staff_pay("pilot", 84).is_empty())
	assert(sim.set_staff_pay("pilot", 85).is_empty())
	assert(sim.get_staff_overview()["roles"][0]["pending_percent"] == 85)
	assert(sim.get_staff_overview()["roles"][0]["pay_percent"] == 100)
	var panel = load("res://scripts/staff_panel.gd").new()
	panel.simulation = sim
	root.add_child(panel)
	panel.refresh(true)
	assert(panel.pay_cards["pilot"]["slider"].value == 85)
	panel.pay_cards["pilot"]["slider"].value = 120
	panel._confirm_pay("pilot")
	panel.pay_dialog.get_ok_button().pressed.emit()
	assert(sim.get_staff_overview()["roles"][0]["pending_percent"] == 120)
	panel.queue_free()
	await process_frame
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var world: RefCounted = main.simulation
	assert(world.found_airline("Warning navigation", "DE", "EDDF").is_empty())
	assert(world.buy_aircraft("atr72_600").is_empty())
	assert(world.hire_missing_staff().is_empty())
	assert(world.set_staff_pay("pilot", 85).is_empty())
	world.start()
	world.advance(210.0 * 86400.0)
	world.pause()
	main._sync_view()
	var found_warning := false
	for entry: Dictionary in world.get_attention_snapshot():
		if entry["kind"] == "strike_warning":
			main._open_attention(entry)
			await process_frame
			assert(root.gui_get_focus_owner() == main.staff_panel.pay_cards["pilot"]["slider"])
			found_warning = true
	assert(found_warning)
	main.queue_free()
	await process_frame
	print("PASS: Staff pay preview, validation, pending period, confirmation and dated warning navigation")
	quit(0)
