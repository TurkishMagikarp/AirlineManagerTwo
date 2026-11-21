extends SceneTree
var main: Control
var sim: RefCounted
func _initialize() -> void:
	call_deferred("_run")
func _frames(count: int) -> void:
	for i in range(count):
		main._process(0.5)
		await process_frame
func _run() -> void:
	TranslationServer.set_locale("de")
	var directory := OS.get_cache_dir().path_join("air-time-hold-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	main.save_user_dir = directory
	sim = main.simulation
	assert(sim.found_airline("Hold Air", "DE", "EDDF").is_empty())
	main._sync_view()
	sim.start()
	assert(sim.set_speed(16))
	var t0: float = sim.get_elapsed_seconds()
	await _frames(2)
	var t1: float = sim.get_elapsed_seconds()
	assert(t1 > t0)
	for button in [main.save_button, main.load_button]:
		button.pressed.emit()
		await process_frame
		var dialog: Control = main.save_slots_dialog
		assert(dialog.find_child("TimeHoldNotice", true, false) != null)
		var held: float = sim.get_elapsed_seconds()
		await _frames(3)
		assert(sim.get_elapsed_seconds() == held)
		if button == main.save_button:
			dialog.name_field.text = "Held"
			dialog.request_save()
			await _frames(2)
			assert(sim.get_elapsed_seconds() == held)
			dialog.name_field.text = "Held"
			dialog.request_save()
			assert(is_instance_valid(dialog.decision))
			await _frames(2)
			assert(sim.get_elapsed_seconds() == held)
			dialog.decision.get_cancel_button().pressed.emit()
			await _frames(2)
			assert(is_instance_valid(main.save_slots_dialog) and sim.get_elapsed_seconds() == held)
		dialog.close_dialog()
		await process_frame
		assert(not is_instance_valid(main.save_slots_dialog))
		assert(sim.get_elapsed_seconds() == held and sim.is_running() and sim.get_speed() == 16)
		await _frames(1)
		var after: float = sim.get_elapsed_seconds()
		assert(after > held and after - held < 0.5 * 16 * 2)
	# Saved slot keeps the clock from when Save opened, with running flag and speed.
	main.save_button.pressed.emit()
	await process_frame
	var saved_at: float = sim.get_elapsed_seconds()
	await _frames(3)
	main.save_slots_dialog.select_slot(1, "")
	main.save_slots_dialog.name_field.text = "Spaeter"
	main.save_slots_dialog.request_save()
	main.save_slots_dialog.close_dialog()
	await process_frame
	# Paused world stays paused and held.
	sim.pause()
	main.load_button.pressed.emit()
	await process_frame
	var paused_at: float = sim.get_elapsed_seconds()
	await _frames(3)
	assert(sim.get_elapsed_seconds() == paused_at)
	# Loading a running slot into a paused company uses the slot's settings.
	main.save_slots_dialog.request_load(1, false)
	assert(is_instance_valid(main.save_slots_dialog.decision))
	main.save_slots_dialog.decision.get_cancel_button().pressed.emit()
	await _frames(2)
	assert(not sim.is_running() and sim.get_elapsed_seconds() == paused_at and is_instance_valid(main.save_slots_dialog))
	main.save_slots_dialog.request_load(1, false)
	main.save_slots_dialog.decision.get_ok_button().pressed.emit()
	await process_frame
	await process_frame
	assert(sim.is_running() and sim.get_speed() == 16 and sim.get_elapsed_seconds() == saved_at)
	# Failed load keeps time held.
	main.load_button.pressed.emit()
	await process_frame
	var failed_at: float = sim.get_elapsed_seconds()
	main.save_slots_dialog.load_slot(7, false)
	await _frames(3)
	assert(is_instance_valid(main.save_slots_dialog) and sim.get_elapsed_seconds() == failed_at)
	main.save_slots_dialog.close_dialog()
	await process_frame
	# Loading a paused slot from a running company.
	sim.pause()
	main.save_button.pressed.emit()
	await process_frame
	main.save_slots_dialog.select_slot(2, "")
	main.save_slots_dialog.name_field.text = "Pausiert"
	main.save_slots_dialog.request_save()
	main.save_slots_dialog.close_dialog()
	await process_frame
	sim.start()
	main.load_button.pressed.emit()
	await process_frame
	main.save_slots_dialog.request_load(2, false)
	main.save_slots_dialog.decision.get_ok_button().pressed.emit()
	await _frames(2)
	assert(not sim.is_running())
	var paused_slot_time: float = sim.get_elapsed_seconds()
	await _frames(2)
	assert(sim.get_elapsed_seconds() == paused_slot_time)
	for id in range(3):
		sim.delete_save_slot(directory, id)
	main.queue_free()
	await process_frame
	print("PASS: save/load dialogs hold simulation time")
	quit(0)
