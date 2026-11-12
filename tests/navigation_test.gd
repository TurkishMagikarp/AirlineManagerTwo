extends SceneTree
## Run: Godot --headless --path . --script res://tests/navigation_test.gd


func _initialize() -> void:
	call_deferred("_run")


func _run() -> void:
	var scene: PackedScene = load("res://scenes/main.tscn")
	var main = scene.instantiate()
	root.add_child(main)
	main.name_input.text = "Navigation Air"
	main.found_button.pressed.emit()
	await process_frame
	if not _check(main.navigation.size() == 3, "Three navigation buttons"):
		return
	if not _check(main.navigation[0].has_focus(), "Initial keyboard focus"):
		return
	for index in range(3):
		if index > 0:
			var tab := InputEventKey.new()
			tab.keycode = KEY_TAB
			tab.pressed = true
			Input.parse_input_event(tab)
			await process_frame
			tab = InputEventKey.new()
			tab.keycode = KEY_TAB
			Input.parse_input_event(tab)
			await process_frame
		if not _check(main.navigation[index].has_focus(), "Tab focus order"):
			return
		var press := InputEventKey.new()
		press.keycode = KEY_ENTER if index % 2 == 0 else KEY_SPACE
		press.pressed = true
		Input.parse_input_event(press)
		await process_frame
		var release := InputEventKey.new()
		release.keycode = press.keycode
		release.pressed = false
		Input.parse_input_event(release)
		await process_frame
		if not _check(main.current_page == index, "Keyboard activation page %d" % index):
			return
		if not _check(main.page_title.text == main.PAGES[index]["title"], "Visible page title"):
			return
		for other in range(3):
			if not _check(main.navigation[other].button_pressed == (other == index), "Exclusive selected state"):
				return
		if not await _capture("page-%d.png" % index):
			return
	var mouse := InputEventMouseButton.new()
	mouse.button_index = MOUSE_BUTTON_LEFT
	mouse.position = main.navigation[0].get_global_rect().get_center()
	mouse.pressed = true
	Input.parse_input_event(mouse)
	await process_frame
	var mouse_release := InputEventMouseButton.new()
	mouse_release.button_index = MOUSE_BUTTON_LEFT
	mouse_release.position = mouse.position
	Input.parse_input_event(mouse_release)
	await process_frame
	if not _check(main.current_page == 0, "Button signal returns to overview"):
		return
	main._show_page(-1)
	if not _check(main.current_page == 0, "Invalid page rejected"):
		return
	root.size = Vector2i(680, 520)
	await process_frame
	if not await _capture("minimum-680x520.png"):
		return
	print("PASS: mouse navigation, Tab focus, Enter/Space activation, selection and invalid index")
	main.queue_free()
	await process_frame
	quit(0)


func _capture(filename: String) -> bool:
	# Optional real viewport captures; use only with a graphical renderer.
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--capture-dir="):
			await RenderingServer.frame_post_draw
			var directory := argument.trim_prefix("--capture-dir=")
			return _check(root.get_texture().get_image().save_png(directory.path_join(filename)) == OK, "Capture " + filename)
	return true


func _check(condition: bool, message: String) -> bool:
	if not condition:
		push_error("FAIL: " + message)
		quit(1)
	return condition
