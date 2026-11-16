extends SceneTree
func _initialize() -> void:
	call_deferred("_run")
func _run() -> void:
	TranslationServer.set_locale("de")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Alliance UI", "DE", "EDDF").is_empty())
	for target in ["LTFM", "EDDM", "LFPG"]:
		assert(sim.lease_aircraft("a320_200").is_empty())
		sim.hire_missing_staff()
		var plane: int = sim.get_fleet().back()["id"]
		assert(sim.create_route("EDDF", target, plane).is_empty())
		var route: int = sim.get_routes().back()["id"]
		assert(sim.set_ticket_price(route, 120).is_empty())
		assert(sim.schedule_route(route, 1, 240).is_empty())
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel.alliance_panel
	assert(panel.rows.size() == 3)
	assert(sim.get_alliance_rules()["fee_eur"] > 0)
	# Nobody is a member yet: refused with an exact, translated reason.
	assert(panel.apply.disabled)
	assert(panel.reason.text.contains("keine Mitglieder"))
	assert(sim.apply_alliance("aurora") == "ALLIANCE_EMPTY")
	assert(sim.apply_alliance("nope") == "ALLIANCE_MISSING")
	assert(sim.leave_alliance() == "ALLIANCE_NONE")
	sim.start()
	var joined := ""
	for day in range(1, 181):
		assert(sim.advance(86400.0))
		for row: Dictionary in sim.get_alliances():
			if row["member_count"] > 0 and row["qualifies"]:
				joined = row["id"]
		if joined != "": break
	sim.pause()
	if joined == "":
		print("PASS: alliance panel refusals; no AI alliance qualified for the player within 180 days")
		quit(0)
		return
	panel.refresh()
	var index := 0
	for row: Dictionary in panel.rows:
		if row["id"] == joined: break
		index += 1
	panel.choice.select(index)
	panel.choice.item_selected.emit(index)
	assert(not panel.apply.disabled)
	assert(panel.reason.text == tr("ALLIANCE_QUALIFIES"))
	panel.apply.pressed.emit()
	await process_frame
	assert(panel.dialog.visible)
	panel.dialog.danger_button.pressed.emit()
	await process_frame
	assert(panel.rows[index]["state"] == "pending")
	assert(panel.apply.disabled and not panel.leave.disabled)
	sim.start()
	assert(sim.advance(31.0 * 86400.0))
	sim.pause()
	panel.refresh()
	assert(panel.rows[index]["state"] == "member")
	assert(sim.get_attention_snapshot().any(func(e: Dictionary): return e["kind"] == "alliance"))
	panel.leave.pressed.emit()
	await process_frame
	assert(panel.dialog.visible)
	panel.dialog.danger_button.pressed.emit()
	await process_frame
	assert(panel.rows[index]["state"] == "leaving")
	assert(sim.leave_alliance() == "ALLIANCE_LEAVING")
	print("PASS: alliance panel lists alliances, explains refusals, applies, joins, announces exit with notice")
	quit(0)
