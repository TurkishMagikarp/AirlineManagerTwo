extends "res://tests/insurance_test.gd"
## Real mouse input follows Rust snapshot targets; run with Godot on a supported host.

func _run() -> void:
	root.gui_embed_subwindows = true
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if not _check(sim.found_airline("Attention", "DE", "EDDF").is_empty() and sim.buy_aircraft("atr72_600").is_empty(), "Create attention fixture"):
		return
	if not _check(sim.create_route("EDDF", "EDDM", 1).is_empty(), "Assign route without crew"):
		return
	main._sync_view()
	await _click_kind("crew")
	if not _check(main.current_page == 2 and main.routes_panel._route().get("id", 0) == 1, "Crew entry selects its route"):
		return
	await _click_kind("idle")
	if not _check(main.current_page == 1 and main.selected_aircraft == 1, "Idle entry selects its aircraft"):
		return
	if not _check(sim.hire_staff("pilot", 1).is_empty(), "Hire trainee"):
		return
	var employee: Dictionary = sim.get_staff()[0]
	var other_rating := ""
	for rating in sim.get_staff_overview()["ratings"]:
		if rating["rating"] != employee["rating"]:
			other_rating = rating["rating"]
			break
	if not _check(sim.train_pilot(employee["id"], other_rating).is_empty(), "Start conversion"):
		return
	main._refresh_attention()
	await _click_kind("training")
	if not _check(main.current_page == 5 and main.staff_panel.selected_id == employee["id"], "Training entry selects its employee"):
		return
	await _click(main.attention_button)
	if not _check(not main.attention_scroll.visible, "Attention list collapses"):
		return
	await _click(main.attention_button)
	if not _check(main.attention_scroll.visible, "Attention list expands"):
		return
	print("PASS: attention navigation and collapse")
	quit(0)

func _click_kind(kind: String) -> void:
	var entries: Array = main.simulation.get_attention_snapshot()
	for i in range(entries.size()):
		if entries[i]["kind"] == kind:
			await _click(main.attention_rows.get_child(i))
			return
	_check(false, "Missing attention entry: " + kind)
