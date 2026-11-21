extends SceneTree
## Wear grounding during a jump: the jump stops, Attention opens the aircraft's Fleet maintenance details, and jumping again resumes service.

var main


func _initialize() -> void:
	call_deferred("_run")


func _forced_rows() -> Array:
	return main.simulation.get_attention_snapshot().filter(func(e: Dictionary): return e["kind"] == "forced_check")


func _jump_once(button_index: int) -> void:
	main.jump_buttons[button_index].pressed.emit()
	while main.jump_in_progress:
		await process_frame
	await process_frame


func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	assert(sim.found_airline("Forced Check Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.hire_missing_staff().is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.schedule_route(1, 1, 120).is_empty())
	main._sync_view()
	var stopped := false
	for i in range(12):
		await _jump_once(3)
		if not _forced_rows().is_empty():
			stopped = true
			break
	assert(stopped)
	var rows := _forced_rows()
	assert(rows.size() == 1)
	var plane_id: int = rows[0]["aircraft_id"]
	var fleet: Array = sim.get_fleet()
	var plane: Dictionary = fleet.filter(func(p: Dictionary): return p["id"] == plane_id)[0]
	assert(plane["status"] == "maintenance" and plane["maintenance_unscheduled"])
	assert(plane["identifier"] in main.jump_status.text and "Pflichtwartung" in main.jump_status.text)
	var operation: Dictionary = sim.get_operations().filter(func(o: Dictionary): return o["aircraft_id"] == plane_id)[0]
	assert(operation["state_kind"] == ("technician_wait" if plane["maintenance_waiting_staff"] else "maintenance") and operation["next_departure"] == "—" and "Pflichtwartung" in operation["status_text"])
	assert(plane["maintenance_waiting_staff"] or operation["maintenance_release"] == plane["maintenance_end"])
	var elapsed: float = sim.get_elapsed_seconds()
	# Collapsed Attention, another aircraft selected, 680x520: the clock-bar action opens the grounded aircraft.
	root.size = Vector2i(680, 520)
	await process_frame
	main.attention_expanded = false
	main.attention_scroll.hide()
	var spare: int = fleet.filter(func(p: Dictionary): return p["id"] != plane_id)[0]["id"]
	main._show_page(1)
	main._select_fleet_aircraft(spare)
	main._show_page(0)
	assert(main.jump_attention_button.visible and main.jump_attention_button.text == "Hinweis öffnen")
	assert(plane["identifier"] in main.jump_status.text)
	main.jump_attention_button.grab_focus()
	assert(main.jump_attention_button.has_focus())
	assert(root.get_visible_rect().encloses(main.jump_attention_button.get_global_rect()))
	var accept := InputEventAction.new()
	accept.action = "ui_accept"
	accept.pressed = true
	Input.parse_input_event(accept)
	main.jump_attention_button.pressed.emit()
	for i in range(4): await process_frame
	assert(main.current_page == 1 and main.selected_aircraft == plane_id)
	assert(plane["identifier"] in main.maintenance_details.text)
	# Wrong-target safety: a stale identity is never replaced by another warning.
	var kept: Dictionary = main.jump_attention_target.duplicate()
	main.jump_attention_target = kept.duplicate()
	main.jump_attention_target["aircraft_id"] = 9999
	main._refresh_jump_attention()
	assert(not main.jump_attention_button.visible and main.jump_attention_target.is_empty())
	main.jump_attention_target = kept
	main._refresh_jump_attention()
	assert(main.jump_attention_button.visible)
	main._sync_view()
	assert(not main.jump_attention_button.visible and main.jump_attention_target.is_empty())
	main.jump_attention_target = kept
	main._show_page(0)
	# Select the other aircraft, then reach the grounded one through Attention.
	var other: int = fleet.filter(func(p: Dictionary): return p["id"] != plane_id)[0]["id"]
	main._show_page(1)
	main._select_fleet_aircraft(other)
	assert(main.selected_aircraft == other)
	main._show_page(0)
	var button: Button
	for i in range(main.attention_rows.get_child_count()):
		if plane["identifier"] in main.attention_rows.get_child(i).text:
			button = main.attention_rows.get_child(i)
	assert(button != null)
	button.grab_focus()
	var press := InputEventKey.new()
	press.keycode = KEY_ENTER
	press.pressed = true
	Input.parse_input_event(press)
	button.pressed.emit()
	await process_frame
	assert(main.current_page == 1 and main.selected_aircraft == plane_id)
	assert(plane["identifier"] in main.maintenance_details.text and "Ungeplante Wartung" in main.maintenance_details.text)
	# The player can jump again with the same check present; service resumes.
	await _jump_once(0)
	assert(not main.jump_attention_button.visible and main.jump_attention_target.is_empty())
	assert(sim.get_elapsed_seconds() > elapsed)
	await _jump_once(2)
	assert(_forced_rows().is_empty() or _forced_rows()[0]["aircraft_id"] != plane_id)
	plane = sim.get_fleet().filter(func(p: Dictionary): return p["id"] == plane_id)[0]
	assert(plane["status"] != "maintenance")
	main.queue_free()
	await process_frame
	print("PASS: wear grounding stops the jump, Attention opens Fleet maintenance details, jumping again resumes")
	quit(0)
