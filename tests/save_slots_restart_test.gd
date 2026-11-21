extends SceneTree
## Named manual saves and autosave across real process restarts (driven by run_save_restart_test.ps1).
##
## Modes (one Godot process each, shared isolated profile directory):
##   write      found + route through the UI, save "A" and "B" (flight airborne) through Speichern,
##              enable autosave in the dialog, trigger one due write, record expectations, confirm quit
##   read-a | read-b | read-auto
##              fresh process: load that entry through the founding screen's "Spielstand laden"
## Direct Linux run:
##   Godot --headless --path . --script res://tests/save_slots_restart_test.gd -- --save-test-mode=write --save-test-dir=/tmp/x
##   (then the same with read-a, read-b, read-auto)

const NAME_A := "Frühflug 日本語 A"
const NAME_B := "Im Anflug B"
const SLOT_A := 0
const SLOT_B := 1
const AUTOSAVE_ID := 1000
const EXPECTED_FILE := "expected.bin"

var main
var sim: RefCounted
var directory := ""


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
	motion.position = button.get_global_rect().get_center()
	motion.global_position = motion.position
	Input.parse_input_event(motion)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = button.get_global_rect().get_center()
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame


func _start_main() -> void:
	TranslationServer.set_locale("de")
	main = load("res://scenes/main.tscn").instantiate()
	# Profile directory (saves and preferences.cfg) must be set before the scene builds its autosave settings.
	main.save_user_dir = directory
	root.size = Vector2i(1440, 1000)
	root.add_child(main)
	main.set_process(false)
	sim = main.simulation
	await process_frame
	await process_frame


## Everything the player can see about the company at one instant (before any further tick).
func _snapshot() -> Dictionary:
	return {
		"airline": sim.get_airline_info(),
		"elapsed": sim.get_elapsed_seconds(),
		"running": sim.is_running(),
		"speed": sim.get_speed(),
		"time": sim.get_display_time(),
		"fleet": sim.get_fleet(),
		"routes": sim.get_routes(),
		"operations": sim.get_operations(),
		"finance": sim.get_financial_summary(),
		"results": sim.get_flight_results(),
	}


func _outcome() -> Dictionary:
	return {
		"elapsed": sim.get_elapsed_seconds(),
		"finance": sim.get_financial_summary(),
		"archive": sim.get_route_flight_page(1, 0),
		"results": sim.get_flight_results(),
		"fleet": sim.get_fleet(),
		"operations": sim.get_operations(),
	}


## Controlled ticks: Godot-side continuation at 1x so that every call advances whole game seconds.
func _continue(landing_seconds: int) -> Dictionary:
	sim.start()
	sim.set_speed(1)
	if not _check(sim.advance(float(landing_seconds + 60) - sim.get_elapsed_seconds()), "Controlled advance to the landing"):
		return {}
	var landed := _outcome()
	if not _check(sim.advance(86400.0), "Controlled advance one day past the landing"):
		return {}
	return {"landed": landed, "later": _outcome()}


## Revenue of the flights booked since `before` (newest archive entries), summed from the archive.
func _new_flight_revenue(before: Dictionary, after: Dictionary) -> int:
	var count: int = after["finance"]["flights"] - before["finance"]["flights"]
	var archive: Array = after["archive"]
	var total := 0
	for i in range(mini(count, archive.size())):
		total += archive[i]["revenue_eur"]
	return total


func _landings(archive: Array, landing: String) -> int:
	return archive.filter(func(f): return f["landing"] == landing and f["route_id"] == 1).size()


func _select_destination(icao: String) -> bool:
	var panel = main.routes_panel
	for i in range(panel.destination.item_count):
		if panel.destination.get_item_metadata(i) == icao:
			panel.destination.select(i)
			panel.destination.item_selected.emit(i)
			return true
	return false


func _buy_atr() -> bool:
	await _click(main.navigation[1])
	for i in range(main.model_select.item_count):
		if main.model_select.get_item_metadata(i)["id"] == "atr72_600":
			main.model_select.select(i)
			main.model_select.item_selected.emit(i)
	var before: int = sim.get_fleet().size()
	await _click(main.buy_button)
	return _check(sim.get_fleet().size() == before + 1, "ATR 72 bought through Fleet: " + main.fleet_status.text)


func _open_route() -> bool:
	var panel = main.routes_panel
	await _click(main.navigation[2])
	panel.refresh()
	if not _check(_select_destination("EDDM"), "EDDM offered as destination"):
		return false
	await _click(panel.create_button)
	for _i in range(5):
		await process_frame
	if not _check(sim.get_routes().size() == 1, "Route EDDF–EDDM created: " + panel.status.text):
		return false
	panel.ticket_input.value = 100
	await _click(panel.ticket_button)
	await _click(main.navigation[5])
	for _i in range(3):
		await process_frame
	await _click(main.staff_panel.hire_missing_button)
	await _click(main.navigation[2])
	panel.refresh()
	panel.focus_route(1)
	panel.delay_input.value = 3
	panel.interval_input.value = 120
	await _click(panel.schedule_button)
	for _i in range(3):
		await process_frame
	return _check(sim.get_routes()[0]["has_plan"], "Route scheduled")


## Save through the real dialog into the first free manual slot under this name.
func _save_named(slot: int, title: String) -> bool:
	await _click(main.save_button)
	var dialog = main.save_slots_dialog
	if not _check(is_instance_valid(dialog) and dialog.saving, "Speichern opens the slot dialog"):
		return false
	dialog.select_slot(slot, "")
	dialog.name_field.text = title
	if not _check(not dialog.save_action.disabled and dialog.save_action.text == "Hier speichern", "Free slot offers Hier speichern"):
		return false
	await _click(dialog.save_action)
	if not _check(dialog.status.text == "Spielstand gespeichert." and main.save_status.text == "Spielstand gespeichert.", "Saved " + title):
		return false
	await _click(dialog.find_children("*", "Button", true, false).filter(func(b): return b.text == "Schließen")[0])
	await process_frame
	return _check(not is_instance_valid(main.save_slots_dialog), "Dialog closed after saving " + title)


func _slot_rows() -> Dictionary:
	var listing: Dictionary = sim.get_save_slots(directory)
	return {"a": listing["slots"][SLOT_A], "b": listing["slots"][SLOT_B], "auto": listing["autosave"]}


func _write() -> void:
	var DAY := 86400.0
	await _start_main()
	if not _check(not sim.has_airline() and main.founding.visible, "Fresh profile starts at the founding screen"):
		return
	if not _check(not FileAccess.file_exists(directory.path_join("preferences.cfg")) and not main.autosave.enabled, "Isolated profile has autosave off"):
		return
	main.name_input.text = "Neustart Air"
	await _click(main.found_button)
	var info: Dictionary = sim.get_airline_info()
	if not _check(info["airport_icao"] == "EDDF" and sim.get_elapsed_seconds() == 0.0 and not sim.is_running(), "Founded at EDDF, paused Day 1"):
		return
	if not await _buy_atr():
		return
	if not await _open_route():
		return
	await _click(main.start_button)
	await _click(main.speed_buttons[1])
	if not _check(sim.is_running() and sim.get_speed() == 4, "Running at 4x"):
		return
	# Save A shortly before the first departure: running 4x, one aircraft, nothing airborne.
	var departure: int = sim.get_operations()[0]["next_departure_seconds"]
	if not _check(departure > 0, "First departure scheduled (%d)" % departure):
		return
	assert(sim.advance((float(departure) / 2.0 - sim.get_elapsed_seconds()) / 4.0))
	if not _check(sim.get_operations()[0]["state_kind"] != "airborne" and sim.get_elapsed_seconds() > 0.0, "Nothing airborne at save A"):
		return
	if not await _save_named(SLOT_A, NAME_A):
		return
	var snap_a := _snapshot()
	# Different state for B: second aircraft, flight airborne, clock paused at 16x.
	if not await _buy_atr():
		return
	assert(sim.advance((float(departure) + 300.0 - sim.get_elapsed_seconds()) / 4.0))
	var flight: Dictionary = sim.get_operations()[0]
	if not _check(flight["state_kind"] == "airborne" and flight["expected_landing_seconds"] > sim.get_elapsed_seconds(), "Flight airborne with a later expected landing"):
		return
	await _click(main.speed_buttons[2])
	await _click(main.pause_button)
	if not _check(not sim.is_running() and sim.get_speed() == 16, "Paused at 16x for save B"):
		return
	if not await _save_named(SLOT_B, NAME_B):
		return
	var snap_b := _snapshot()
	if not _check(snap_a["elapsed"] < snap_b["elapsed"] and snap_a["fleet"].size() == 1 and snap_b["fleet"].size() == 2 and snap_a["finance"] != snap_b["finance"] and snap_a["running"] and not snap_b["running"], "Saves A and B are distinguishable"):
		return
	var rows_manual := _slot_rows()
	# Uninterrupted continuation from B: the pending flight lands once and is booked once.
	var landing: int = flight["expected_landing_seconds"]
	var landing_text: String = flight["expected_landing"]
	if not _check(_landings(snap_b["results"], landing_text) == 0, "Landing not booked yet at B"):
		return
	var reference := _continue(landing)
	if reference.is_empty():
		return
	var landed: Dictionary = reference["landed"]
	if not _check(_landings(landed["archive"], landing_text) == 1 and landed["finance"]["flights"] == snap_b["finance"]["flights"] + 1, "Reference lands the identified flight once"):
		return
	# Back to a UI-driven state, then autosave through the existing dialog and a due write.
	await _click(main.load_button)
	var dialog = main.save_slots_dialog
	var choice: OptionButton = dialog.autosave_choice
	choice.select(choice.get_item_index(1))
	choice.item_selected.emit(choice.selected)
	await _click(dialog.find_children("*", "Button", true, false).filter(func(b): return b.text == "Schließen")[0])
	await process_frame
	if not _check(main.autosave.enabled and main.autosave.interval_minutes == 1 and FileAccess.file_exists(directory.path_join("preferences.cfg")), "Autosave enabled in the isolated profile"):
		return
	main._autosave_tick(59.0)
	if not _check(not sim.get_save_slots(directory)["autosave"]["occupied"], "Autosave not due after 59 s"):
		return
	main._autosave_tick(1.0)
	var snap_auto := _snapshot()
	var rows := _slot_rows()
	if not _check(rows["auto"]["occupied"] and rows["auto"]["error"].is_empty() and rows["auto"]["header"]["name"] == "Autosave", "Autosave has its own entry"):
		return
	if not _check(rows["a"] == rows_manual["a"] and rows["b"] == rows_manual["b"] and rows["a"]["header"]["name"] == NAME_A and rows["b"]["header"]["name"] == NAME_B, "Manual slots keep content and metadata"):
		return
	if not _check(snap_auto["elapsed"] > snap_b["elapsed"] + DAY and snap_auto["running"] and snap_auto["speed"] == 1, "Autosave state differs from both manual saves"):
		return
	var file := FileAccess.open(directory.path_join(EXPECTED_FILE), FileAccess.WRITE)
	# Binary variant storage keeps doubles exact (text would round them).
	file.store_var({"a": snap_a, "b": snap_b, "auto": snap_auto, "reference": reference, "landing": landing_text, "landing_seconds": landing, "rows": rows})
	file.close()
	# Normal quit confirmation.
	var quits := [0]
	main.quit_handler = func(): quits[0] += 1
	main.request_quit()
	await process_frame
	if not _check(main._quit_prompt_open() and "Neustart Air" in main.quit_dialog.card.get_child(0).get_child(1).text, "Quit confirmation names the airline"):
		return
	main.quit_dialog.get_ok_button().pressed.emit()
	await process_frame
	if not _check(quits[0] == 1, "Quit confirmed"):
		return
	print("PASS: write (process %d): saves A %s and B %s, autosave, quit" % [OS.get_process_id(), snap_a["time"], snap_b["time"]])
	quit(0)


func _read(mode: String) -> void:
	var reader := FileAccess.open(directory.path_join(EXPECTED_FILE), FileAccess.READ)
	var expected: Dictionary = reader.get_var()
	reader.close()
	var key := mode.trim_prefix("read-")
	var slot: int = {"a": SLOT_A, "b": SLOT_B, "auto": AUTOSAVE_ID}[key]
	var want: Dictionary = expected[key]
	await _start_main()
	if not _check(not sim.has_airline() and main.founding.visible and sim.get_elapsed_seconds() == 0.0, "Fresh process starts at the founding screen"):
		return
	if not _check(main.autosave.enabled and main.autosave.interval_minutes == 1, "Autosave preference restored from the profile"):
		return
	# Manual slots and autosave exactly as the writing process left them.
	var rows := _slot_rows()
	if not _check(rows == expected["rows"], "Slot headers unchanged by restart"):
		return
	await _click(main.founding_load_button)
	var dialog = main.save_slots_dialog
	if not _check(is_instance_valid(dialog) and not dialog.saving, "Spielstand laden opens from the founding screen"):
		return
	var texts: Array = dialog.list_box.find_children("*", "Label", true, false).map(func(l): return l.text)
	for part in [NAME_A, NAME_B, "Neustart Air · EDDF"]:
		if not _check(texts.any(func(t): return part in t) or dialog.list_box.find_children("*", "Button", true, false).any(func(b): return part in b.text), "Dialog lists " + part):
			return
	if not _check(dialog.list_box.get_children().any(func(c): return c.get_meta("slot_id", -1) == AUTOSAVE_ID), "Dialog lists the autosave separately"):
		return
	dialog.select_slot(slot, "")
	await process_frame
	var load_button: Button = null
	for card in dialog.list_box.get_children():
		if card.get_meta("slot_id", -1) == slot:
			load_button = card.find_children("*", "Button", true, false).filter(func(b): return b.text == "Laden")[0]
	if not _check(load_button != null, "Load action of the chosen entry"):
		return
	await _click(load_button)
	await process_frame
	if not _check(main.save_status.text == "Spielstand geladen." and not is_instance_valid(main.save_slots_dialog), "Entry loaded: " + main.save_status.text):
		return
	# Compare before any further simulation tick.
	var got := _snapshot()
	for field in want:
		if got[field] != want[field] and want[field] is Array:
			for i in range(want[field].size()):
				if got[field].size() <= i or got[field][i] != want[field][i]:
					print("DIFF %s[%d]: %s vs %s" % [field, i, str(got[field][i]) if got[field].size() > i else "-", str(want[field][i])])
		if not _check(got[field] == want[field], "Restored %s equals the saved moment" % field):
			return
	var info: Dictionary = got["airline"]
	if not _check(main.overview.visible and not main.founding.visible and main.heading.text == "Neustart Air" and info["airport_icao"] == "EDDF" and main.metrics["balance"].text == main.UI.money(info["capital_eur"]), "Overview shows the loaded company, base and balance"):
		return
	var control_ok: bool = main.start_button.disabled == want["running"]
	if not _check(control_ok and main.state_label.text.ends_with("%d×" % want["speed"]), "Clock controls show the saved running state and speed"):
		return
	if key == "b":
		await _continue_checks(expected)
	if not _check(not main.jump_in_progress, "No stray jump"):
		return
	print("PASS: %s (process %d): %s restored exactly at %s" % [mode, OS.get_process_id(), key, want["time"]])
	quit(0)


## Resume the airborne-flight save: same landing, same booking as the uninterrupted run, once.
func _continue_checks(expected: Dictionary) -> void:
	var landing: String = expected["landing"]
	var op: Dictionary = sim.get_operations()[0]
	if not _check(op["state_kind"] == "airborne" and op["expected_landing"] == landing and _landings(sim.get_route_flight_page(1, 0), landing) == 0, "Pending flight identified after restore"):
		return
	var reference: Dictionary = expected["reference"]
	var again := _continue(expected["landing_seconds"])
	if again.is_empty():
		return
	for stage in ["landed", "later"]:
		var got: Dictionary = again[stage]
		var want: Dictionary = reference[stage]
		for field in want:
			if not _check(got[field] == want[field], "%s: %s equals the uninterrupted run" % [stage, field]):
				return
		if not _check(_landings(got["archive"], landing) == 1 and _landings(got["results"], landing) == 1, "%s: identified flight appears exactly once" % stage):
			return
	var before: Dictionary = expected["b"]
	var booked: Dictionary = again["landed"]
	var new_flights: int = booked["finance"]["flights"] - before["finance"]["flights"]
	if not _check(new_flights == 1, "Landing booked once in company flights (%d new)" % new_flights):
		return
	var landed_result: Dictionary = booked["archive"].filter(func(f): return f["landing"] == landing)[0]
	var delta: int = booked["finance"]["company_revenue_eur"] - before["finance"]["company_revenue_eur"]
	if not _check(delta == landed_result["revenue_eur"] and delta == _new_flight_revenue(before, booked), "Company revenue rose by exactly the landing's revenue (%d vs %d)" % [delta, landed_result["revenue_eur"]]):
		return
	var later: Dictionary = again["later"]
	if not _check(later["finance"]["company_revenue_eur"] - before["finance"]["company_revenue_eur"] == _new_flight_revenue(before, later), "Revenue after a further day equals the archive sum"):
		return


func _run() -> void:
	var mode := ""
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--save-test-mode="):
			mode = argument.trim_prefix("--save-test-mode=")
		if argument.begins_with("--save-test-dir="):
			directory = argument.trim_prefix("--save-test-dir=")
	if not _check(mode in ["write", "read-a", "read-b", "read-auto"] and not directory.is_empty(), "Explicit test mode and isolated profile directory required"):
		return
	DirAccess.make_dir_recursive_absolute(directory)
	if mode == "write":
		await _write()
	else:
		await _read(mode)
