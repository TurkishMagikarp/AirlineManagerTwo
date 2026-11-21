extends SceneTree
## Attention crew warning -> Staff shortage + hire action -> service resumes (pilot and cabin shortages, reload while blocked).
var main: Node
var failed := false

func _initialize() -> void:
	call_deferred("_run")

func _check(ok: bool, what: String) -> bool:
	if not ok:
		failed = true
		print("FAIL: " + what)
		quit(1)
	return ok

func _frames(n: int = 4) -> void:
	for i in range(n):
		await process_frame

func _fixture(role: String, path: String) -> bool:
	var sim: RefCounted = main.simulation
	for error in [sim.found_airline("Crew Restore", "DE", "EDDF"), sim.buy_aircraft("atr72_600"), sim.buy_aircraft("atr72_600"), sim.create_route("EDDF", "EDDM", 1), sim.create_route("EDDF", "EDDM", 2), sim.hire_missing_staff(), sim.set_ticket_price(1, 100), sim.set_ticket_price(2, 100), sim.schedule_route(1, 1, 120), sim.schedule_route(2, 1, 120), sim.set_staff_pay(role, 85)]:
		if not _check(error.is_empty(), "fixture: " + error):
			return false
	sim.start()
	for day in range(300):
		sim.advance(86400.0)
		if sim.get_staff_overview()["roles"][0 if role == "pilot" else 1]["resigned"] > 0:
			break
	sim.pause()
	return _check(sim.save_game(path).is_empty(), "save blocked fixture")

func _press(key: Key) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = key
		event.physical_keycode = key
		event.pressed = pressed
		Input.parse_input_event(event)
		await _frames(2)

func _crew_entry() -> Dictionary:
	for entry: Dictionary in main.simulation.get_attention_snapshot():
		if entry["kind"] == "crew":
			return entry
	return {}

func _crew_button() -> Button:
	var entries: Array = main.simulation.get_attention_snapshot()
	for i in range(entries.size()):
		if entries[i]["kind"] == "crew" and entries[i]["aircraft_id"] == _crew_entry().get("aircraft_id", -1):
			return main.attention_rows.get_child(i)
	return null

func _scenario(role: String, rating_text: String) -> bool:
	var path := ProjectSettings.globalize_path("user://crew-restore-%s.json" % role)
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	if not await _fixture(role, path):
		return false
	var sim: RefCounted = main.simulation
	# Reload while blocked: a fresh simulation loads the saved state.
	if not _check(sim.load_game(path).is_empty(), "reload blocked save"):
		return false
	root.size = Vector2i(680, 520)
	main.attention_expanded = true
	main._sync_view()
	await _frames(6)
	var entry := _crew_entry()
	if not _check(not entry.is_empty() and entry["page"] == "staff" and entry["aircraft_id"] > 0, "crew warning targets staff and an aircraft"):
		return false
	var employees: int = sim.get_staff_overview()["employees"]
	var flights: int = sim.get_financial_summary()["flights"]
	var plans := str(sim.get_routes().map(func(r): return r["has_plan"]))
	var button := _crew_button()
	if not _check(button != null, "crew warning row rendered"):
		return false
	button.grab_focus()
	await _frames(2)
	await _press(KEY_ENTER)
	await _frames(8)
	var panel = main.staff_panel
	var scroll: ScrollContainer = panel.get_parent().get_parent()
	var hire: Button = panel.hire_missing_button
	if not _check(main.current_page == 5 and root.gui_get_focus_owner() == hire and not hire.disabled, "keyboard activation focuses the enabled hire action"):
		return false
	if not _check(scroll.get_global_rect().encloses(hire.get_global_rect()), "hire action inside the visible Staff viewport %s %s %s" % [scroll.get_global_rect(), hire.get_global_rect(), panel.shortage_label.get_global_rect()]):
		return false
	var identifier: String = ""
	for plane in sim.get_staff_overview()["aircraft"]:
		if plane["id"] == entry["aircraft_id"]:
			identifier = plane["identifier"]
	var text: String = panel.shortage_label.text
	if not _check(identifier in text and rating_text in text, "shortage names aircraft and missing crew: " + text):
		return false
	var other_staffed := false
	for plane in sim.get_staff_overview()["aircraft"]:
		if plane["id"] != entry["aircraft_id"] and plane["state"] == "crewed":
			other_staffed = true
	if not _check(other_staffed or role == "cabin_crew", "other aircraft stays labelled crewed"):
		return false
	if not _check(sim.get_staff_overview()["employees"] == employees and str(sim.get_routes().map(func(r): return r["has_plan"])) == plans and sim.get_financial_summary()["flights"] == flights, "opening the warning changes nothing"):
		return false
	await _press(KEY_ENTER)
	await _frames(4)
	if not _check(_crew_entry().is_empty() or _crew_entry()["aircraft_id"] != entry["aircraft_id"], "warning clears after hiring"):
		return false
	for plane in sim.get_staff_overview()["aircraft"]:
		if not _check(plane["state"] == "crewed", "all aircraft crewed after hire"):
			return false
	sim.start()
	sim.advance(3.0 * 86400.0)
	sim.pause()
	if not _check(sim.get_financial_summary()["flights"] > flights, "schedule completes a flight after recovery"):
		return false
	main.queue_free()
	await _frames(2)
	return true

func _run() -> void:
	root.gui_embed_subwindows = true
	if not await _scenario("pilot", "1 Piloten (ATR 72)"):
		return
	if not await _scenario("cabin_crew", "Flugbegleiter"):
		return
	print("PASS: crew warning opens Staff shortage and hire action; hiring resumes service")
	quit(0)
