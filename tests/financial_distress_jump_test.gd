extends SceneTree
## A jump that enters financial distress or restructuring stops for a finance Attention entry that opens the lending controls.

var main


func _initialize() -> void:
	call_deferred("_run")


func _jump_longest() -> void:
	main._jump(main.simulation.get_jump_options()[-1]["seconds"])
	while main.jump_in_progress:
		await process_frame
	await process_frame


func _finance_rows() -> Array:
	return main.simulation.get_attention_snapshot().filter(func(e: Dictionary): return e["kind"] in ["financial_distress", "restructuring"])


func _expected_focus() -> Control:
	var panel = main.finances_panel
	for button: Button in [panel.loan_reschedule, panel.loan_repay, panel.loan_borrow]:
		if not button.disabled:
			return button
	return panel.loan_status


func _snapshot() -> Array:
	var sim = main.simulation
	return [sim.get_elapsed_seconds(), sim.get_airline_info()["capital_eur"], sim.get_financing()["debt_eur"]]


func _press_enter(button: Button) -> void:
	button.grab_focus()
	assert(button.has_focus())
	var press := InputEventKey.new()
	press.keycode = KEY_ENTER
	press.pressed = true
	Input.parse_input_event(press)
	button.pressed.emit()
	for i in range(6): await process_frame


func _assert_lending_focus() -> void:
	var panel = main.finances_panel
	assert(main.current_page == 4)
	for i in range(10): await process_frame
	var owner := root.gui_get_focus_owner()
	assert(owner == _expected_focus())
	assert(panel.loan_status.get_parent().is_ancestor_of(owner))
	assert(root.get_visible_rect().encloses(owner.get_global_rect()))
	assert(panel.loan_reschedule.disabled == not main.simulation.get_financing()["can_reschedule"])


func _run() -> void:
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	assert(sim.found_airline("Distress Air", "DE", "EDDF").is_empty())
	for i in range(3): assert(sim.buy_aircraft("atr72_600").is_empty())
	for i in range(6): assert(sim.lease_aircraft("atr72_600").is_empty())
	main._sync_view()
	root.size = Vector2i(680, 520)
	main._show_page(0)
	main.attention_expanded = false
	for frame in range(4): await process_frame

	for expected_kind in ["financial_distress", "restructuring"]:
		main._show_page(0)
		for i in range(8):
			await _jump_longest()
			if main.jump_attention_target.get("kind", "") == expected_kind:
				break
		assert(main.jump_attention_target.get("kind", "") == expected_kind)
		assert(main.jump_attention_button.visible)
		assert(not main.attention_scroll.visible)
		var rows := _finance_rows()
		assert(rows.size() == 1 and rows[0]["kind"] == expected_kind)
		var financing: Dictionary = sim.get_financing()
		if expected_kind == "restructuring":
			assert(financing["deadline"] != "—" and financing["deadline"] in rows[0]["text"])
		else:
			assert(financing["deadline"] == "—" and "Restrukturierung ab" in rows[0]["text"])
		var before := _snapshot()
		await _press_enter(main.jump_attention_button)
		await _assert_lending_focus()
		assert(_snapshot() == before)
		main._show_page(0)
		main.attention_expanded = true
		main.attention_signature = ""
		main._refresh_attention()
		var button: Button
		for i in range(main.attention_rows.get_child_count()):
			var row: Button = main.attention_rows.get_child(i)
			if ("Restrukturierung bis" if expected_kind == "restructuring" else "Finanznot") in row.text:
				button = row
		assert(button != null)
		await _press_enter(button)
		await _assert_lending_focus()
		assert(_snapshot() == before)
		main.attention_expanded = false

	main._show_page(0)
	for plane in sim.get_fleet():
		if not sim.get_financing()["distressed"]:
			break
		sim.sell_aircraft(plane["id"])
	main.attention_signature = ""
	main._refresh_attention()
	assert(not sim.get_financing()["distressed"])
	assert(_finance_rows().is_empty())
	assert(not main.jump_attention_button.visible)
	print("PASS")
	quit(0)
