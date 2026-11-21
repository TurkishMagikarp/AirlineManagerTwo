extends SceneTree
var main: Control
var directory: String
func _initialize() -> void:
	call_deferred("_run")
func _run() -> void:
	TranslationServer.set_locale("de")
	directory = OS.get_cache_dir().path_join("air-save-slots-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	main.save_user_dir = directory
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Slots Air", "DE", "EDDF").is_empty())
	main._sync_view()
	main.save_button.pressed.emit()
	await process_frame
	var dialog: Control = main.save_slots_dialog
	assert(dialog.rows.all(func(r): return not r["occupied"]))
	dialog.name_field.text = "Vor dem Sprung 日本語"
	dialog.request_save()
	assert(dialog.status.text == "Spielstand gespeichert.")
	var before: Dictionary = sim.get_financial_summary().duplicate(true)
	assert(sim.buy_aircraft("atr72_600").is_empty())
	dialog.name_field.text = "Nach dem Kauf"
	dialog.request_save()
	assert(is_instance_valid(dialog.decision))
	assert(dialog.decision.get_cancel_button().text == "Abbrechen")
	assert(sim.get_save_slots(directory)["slots"][0]["backup"].is_empty())
	dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	assert(not sim.get_save_slots(directory)["slots"][0]["backup"].is_empty())
	dialog.select_slot(1, "")
	dialog.name_field.text = "Zweiter Platz"
	dialog.request_save()
	assert(sim.get_save_slots(directory)["slots"][1]["occupied"])
	# Cancel deletion preserves both generations.
	dialog.request_delete(0, "Nach dem Kauf")
	dialog.decision.get_cancel_button().pressed.emit()
	await process_frame
	assert(sim.get_save_slots(directory)["slots"][0]["occupied"])
	# Loading over an open airline asks first; cancel keeps the company and the selection.
	var later: Dictionary = sim.get_financial_summary().duplicate(true)
	dialog.select_slot(0, "Nach dem Kauf")
	dialog.request_load(0, true)
	assert(is_instance_valid(dialog.decision))
	assert(dialog.decision.safe_button.text == "Abbrechen")
	var texts: Array = dialog.decision.find_children("*", "Label", true, false).map(func(l): return l.text)
	assert(texts.any(func(t): return "Sicherung laden?" == t) and texts.any(func(t): return "Vor dem Sprung" in t and "Tag 1" in t and "Nicht gespeicherter Fortschritt" in t))
	dialog.decision.get_cancel_button().pressed.emit()
	await process_frame
	await process_frame
	assert(sim.get_financial_summary() == later and not sim.get_fleet().is_empty())
	assert(dialog.selected == 0 and is_instance_valid(main.save_slots_dialog))
	var focus_owner: Control = root.gui_get_focus_owner()
	assert(focus_owner != null and focus_owner.is_visible_in_tree() and dialog.is_ancestor_of(focus_owner))
	dialog.request_load(0, false)
	assert(is_instance_valid(dialog.decision))
	dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_financial_summary() == later)
	main._load_game()
	await process_frame
	dialog = main.save_slots_dialog
	dialog.request_load(0, true)
	dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_financial_summary() == before)
	assert(sim.get_fleet().is_empty())
	assert(main.save_status.text == "Sicherung geladen.")
	# Legacy APIs and every new slot API reject actions during a worker jump.
	assert(sim.begin_jump(86400))
	assert(not sim.save_slot(directory, 2, "During jump", false).is_empty())
	assert(not sim.load_slot(directory, 0, true).is_empty())
	assert(not sim.delete_save_slot(directory, 0).is_empty())
	assert(not sim.get_save_slots(directory)["error"].is_empty())
	assert(not sim.save_game(directory.path_join("blocked.json")).is_empty())
	assert(not sim.load_game(directory.path_join("blocked.json")).is_empty())
	main.jump_in_progress = true
	main._load_game()
	assert(not is_instance_valid(main.save_slots_dialog))
	main.jump_in_progress = false
	while sim.poll_jump().is_empty(): await process_frame
	main._load_game()
	await process_frame
	dialog = main.save_slots_dialog
	dialog.request_delete(1, "Zweiter Platz")
	dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	assert(not sim.get_save_slots(directory)["slots"][1]["occupied"])
	# Tab traversal stays inside the modal.
	var focus: Control = root.gui_get_focus_owner()
	if focus == null: dialog._focus_default()
	for i in range(25):
		var key := InputEventKey.new()
		key.keycode = KEY_TAB
		key.pressed = true
		Input.parse_input_event(key)
		await process_frame
		focus = root.gui_get_focus_owner()
		assert(focus != null and dialog.is_ancestor_of(focus))
	dialog.close_dialog()
	await process_frame
	# Dense list at the minimum window size: focus stays inside the scroll viewport.
	root.size = Vector2i(680, 520)
	var manual_count: int = sim.get_save_slots(directory)["slots"].size()
	for id in range(manual_count):
		assert(sim.save_slot(directory, id, "Dicht %d" % id, id == 0).is_empty())
	assert(sim.save_slot(directory, manual_count - 2, "Dicht neu", true).is_empty())
	main._load_game()
	await process_frame
	await process_frame
	dialog = main.save_slots_dialog
	var scroll: ScrollContainer = dialog.list_box.get_parent()
	var late: int = manual_count - 2
	var first_focus: Control = root.gui_get_focus_owner()
	var seen_late := false
	var chosen: int = dialog.selected
	for i in range(80):
		await _tab(false)
		focus = root.gui_get_focus_owner()
		assert(focus != null and dialog.is_ancestor_of(focus))
		if dialog.list_box.is_ancestor_of(focus):
			var view := scroll.get_global_rect()
			var box := focus.get_global_rect()
			assert(box.position.y >= view.position.y - 1 and box.end.y <= view.end.y + 1)
			if focus.get_parent().get_parent().get_meta("slot_id", -1) == manual_count - 1: seen_late = true
	assert(seen_late and dialog.selected == chosen)
	for i in range(80):
		await _tab(true)
		focus = root.gui_get_focus_owner()
		assert(focus != null and dialog.is_ancestor_of(focus))
		if dialog.list_box.is_ancestor_of(focus):
			var view := scroll.get_global_rect()
			var box := focus.get_global_rect()
			assert(box.position.y >= view.position.y - 1 and box.end.y <= view.end.y + 1)
	assert(dialog.selected == chosen)
	# Select a late slot with a backup by keyboard, reach both load actions, cancel keeps focus on the slot.
	dialog.select_slot(late, "")
	await process_frame
	await process_frame
	var reached := {}
	for i in range(80):
		await _tab(false)
		focus = root.gui_get_focus_owner()
		if focus is Button and focus.text in ["Laden", "Sicherung laden"]:
			reached[focus.text] = true
			var view := scroll.get_global_rect()
			var box := focus.get_global_rect()
			assert(box.position.y >= view.position.y - 1 and box.end.y <= view.end.y + 1)
			if reached.size() == 2: break
	assert(reached.size() == 2)
	focus.pressed.emit()
	assert(is_instance_valid(dialog.decision))
	dialog.decision.get_cancel_button().pressed.emit()
	await process_frame
	await process_frame
	focus = root.gui_get_focus_owner()
	assert(dialog.selected == late and focus != null and focus.get_parent().get_parent().get_meta("slot_id", -1) == late)
	var view_after := scroll.get_global_rect()
	assert(focus.get_global_rect().end.y <= view_after.end.y + 1 and focus.get_global_rect().position.y >= view_after.position.y - 1)
	dialog.close_dialog()
	await process_frame
	# Save mode with one free slot: the free-slot action is reachable and tab alone writes nothing.
	assert(sim.delete_save_slot(directory, manual_count - 1).is_empty())
	main.save_button.pressed.emit()
	await process_frame
	await process_frame
	dialog = main.save_slots_dialog
	scroll = dialog.list_box.get_parent()
	var wrote_before: Dictionary = sim.get_save_slots(directory)
	var free_reached := false
	for i in range(80):
		await _tab(false)
		focus = root.gui_get_focus_owner()
		if focus is Button and "Freier Speicherplatz" in focus.text:
			free_reached = true
			var view := scroll.get_global_rect()
			var box := focus.get_global_rect()
			assert(box.position.y >= view.position.y - 1 and box.end.y <= view.end.y + 1)
			break
	assert(free_reached and sim.get_save_slots(directory) == wrote_before)
	focus.pressed.emit()
	await process_frame
	dialog.name_field.text = "Per Tastatur"
	dialog.save_action.pressed.emit()
	await process_frame
	assert(sim.get_save_slots(directory)["slots"][manual_count - 1]["occupied"])
	dialog.close_dialog()
	await process_frame
	for id in range(manual_count): assert(sim.delete_save_slot(directory, id).is_empty())
	# Autosave: off by default, persisted preference, deferred during jumps, explicit reload only.
	var prefs := directory.path_join("preferences.cfg")
	var Autosave := load("res://scripts/autosave.gd")
	main.autosave = Autosave.new(prefs)
	assert(not main.autosave.enabled and not main.autosave.tick(1.0e6, true))
	main._autosave_tick(1.0e6)
	assert(not sim.get_save_slots(directory)["autosave"]["occupied"])
	main._load_game()
	await process_frame
	var choice: OptionButton = main.save_slots_dialog.autosave_choice
	assert(choice.selected == 0)
	choice.select(choice.get_item_index(5))
	choice.item_selected.emit(choice.selected)
	main.save_slots_dialog.close_dialog()
	await process_frame
	var reloaded = Autosave.new(prefs)
	assert(reloaded.enabled and reloaded.interval_minutes == 5)
	main._autosave_tick(299.0)
	assert(not sim.get_save_slots(directory)["autosave"]["occupied"])
	main.jump_in_progress = true
	main._autosave_tick(2.0)
	assert(not sim.get_save_slots(directory)["autosave"]["occupied"])
	main.jump_in_progress = false
	var settled: Dictionary = sim.get_financial_summary().duplicate(true)
	main._autosave_tick(0.0)
	var entry: Dictionary = sim.get_save_slots(directory)["autosave"]
	assert(entry["occupied"] and entry["error"].is_empty() and entry["header"]["name"] == "Autosave")
	assert(not sim.get_save_slots(directory)["slots"][2]["occupied"])
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.get_financial_summary() != settled)
	main._load_game()
	await process_frame
	dialog = main.save_slots_dialog
	assert(not dialog.autosave.is_empty())
	assert(dialog.list_box.get_children().any(func(card): return card.get_meta("slot_id", -1) == dialog.AUTOSAVE_ID))
	var current: Dictionary = sim.get_financial_summary().duplicate(true)
	dialog.select_slot(dialog.AUTOSAVE_ID, "")
	dialog.request_load(dialog.AUTOSAVE_ID, false)
	assert(is_instance_valid(dialog.decision))
	dialog.decision.get_cancel_button().pressed.emit()
	await process_frame
	await process_frame
	assert(sim.get_financial_summary() == current and dialog.selected == dialog.AUTOSAVE_ID)
	dialog.request_load(dialog.AUTOSAVE_ID, false)
	dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_financial_summary() == settled)
	assert(sim.begin_jump(86400))
	assert(not sim.autosave(directory).is_empty())
	assert(not sim.load_autosave(directory, false).is_empty())
	while sim.poll_jump().is_empty(): await process_frame
	DirAccess.remove_absolute(prefs)
	main.autosave.configure(false, 10)
	DirAccess.remove_absolute(prefs)
	var autosave_dir := directory.path_join("saves/autosave")
	for generation in DirAccess.get_directories_at(autosave_dir):
		for file in DirAccess.get_files_at(autosave_dir.path_join(generation)): DirAccess.remove_absolute(autosave_dir.path_join(generation).path_join(file))
		DirAccess.remove_absolute(autosave_dir.path_join(generation))
	for file in DirAccess.get_files_at(autosave_dir): DirAccess.remove_absolute(autosave_dir.path_join(file))
	DirAccess.remove_absolute(autosave_dir)
	for id in range(sim.get_save_slots(directory)["slots"].size()):
		if sim.get_save_slots(directory)["slots"][id]["occupied"]: assert(sim.delete_save_slot(directory, id).is_empty())
	DirAccess.remove_absolute(directory.path_join("saves/legacy-imported"))
	DirAccess.remove_absolute(directory.path_join("saves"))
	DirAccess.remove_absolute(directory)
	main.queue_free()
	await process_frame
	print("PASS: save slots dialog, confirmations, backup load, focus and jump guards")
	quit(0)
func _tab(backwards: bool) -> void:
	var key := InputEventKey.new()
	key.keycode = KEY_TAB
	key.shift_pressed = backwards
	key.pressed = true
	Input.parse_input_event(key)
	await process_frame
	await process_frame
