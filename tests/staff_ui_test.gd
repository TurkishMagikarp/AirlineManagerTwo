extends SceneTree
## Runs the real Staff tab in a native window (or headlessly for CI).
## Optional renderer captures: -- --capture-dir=PATH. Never opens a player save.

var main: Control
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _settle() -> void:
	for frame in range(8):
		await process_frame

func _capture(name: String) -> void:
	if capture_dir.is_empty():
		return
	await RenderingServer.frame_post_draw
	assert(root.get_texture().get_image().save_png(capture_dir.path_join(name + ".png")) == OK)

func _run() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Staff Inspection Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	main._sync_view()
	main.navigation[5].pressed.emit()
	var panel: VBoxContainer = main.staff_panel
	var scroll: ScrollContainer = panel.get_parent().get_parent()
	assert(panel.is_visible_in_tree())
	assert("fehlen" in panel.shortage_label.text)
	assert(not panel.hire_missing_button.disabled)
	await _settle()
	await _capture("staff-shortages")
	panel.hire_missing_button.pressed.emit()
	assert(sim.get_staff_overview()["employees"] == 10)
	assert(panel.hire_missing_button.disabled)
	assert("Kein Personalmangel" in panel.shortage_label.text)
	# Hiring via the visible input also commits unsubmitted SpinBox text.
	panel.role_select.select(0)
	panel.count_spin.get_line_edit().text = "2"
	panel.hire_button.pressed.emit()
	assert(sim.get_staff_overview()["employees"] == 12)
	assert(sim.schedule_route(1, 1, 120).is_empty())
	panel.refresh(true)
	var row: TreeItem = panel.staff_tree.get_root().get_first_child()
	row.select(0)
	panel._selection_changed()
	assert(panel.dismiss_button.disabled, "Reserved crew cannot be dismissed in the UI")
	while row.get_next() != null:
		row = row.get_next()
	row.select(0)
	panel._selection_changed()
	var selected: int = panel.selected_id
	panel.refresh(true)
	assert(panel.selected_id == selected and not panel.dismiss_button.disabled)
	# Changing the employee data rebuilds the table without losing selection.
	assert(sim.hire_staff("technician", 1).is_empty())
	panel.refresh(true)
	assert(panel.selected_id == selected and not panel.dismiss_button.disabled)
	panel.dismiss_button.pressed.emit()
	assert(sim.get_staff_overview()["employees"] == 12)
	assert("entlassen" in panel.dismiss_status.text)
	assert(panel.dismiss_button.disabled)
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(800, 600), Vector2i(680, 520)]:
		root.size = dimensions
		scroll.scroll_vertical = 0
		await _settle()
		assert(main.size.x <= dimensions.x, "No horizontal page overflow")
		assert(panel.size.x <= dimensions.x - 40, "Staff panel fits the page")
		await _capture("staff-%dx%d" % [dimensions.x, dimensions.y])
		for control in [panel.hire_missing_button, panel.hire_button, panel.dismiss_button]:
			scroll.ensure_control_visible(control)
			await _settle()
			assert(root.get_visible_rect().encloses(control.get_global_rect()), "Staff actions remain reachable")
			if control == panel.hire_button:
				await _capture("staff-hire-%dx%d" % [dimensions.x, dimensions.y])
		await _capture("staff-dismiss-%dx%d" % [dimensions.x, dimensions.y])
	main.queue_free()
	await process_frame
	print("PASS: Staff tab hiring, shortages, selection, dismissal and 1440/1100/800/680 layouts")
	quit(0)
