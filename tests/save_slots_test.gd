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
	dialog.load_slot(0, true)
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
	dialog.load_slot(dialog.AUTOSAVE_ID, false)
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
