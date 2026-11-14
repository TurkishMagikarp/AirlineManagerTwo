extends SceneTree
## Real input events and read-only Rust snapshots; no personal save file.
var failed := false

func _initialize() -> void:
	call_deferred("_run")

func check(condition: bool, message: String) -> void:
	if not condition:
		failed = true
		push_error("FAIL: " + message)

func click_at(point: Vector2) -> void:
	var motion := InputEventMouseMotion.new()
	motion.position = point
	Input.parse_input_event(motion)
	await process_frame
	for pressed in [true, false]:
		var event := InputEventMouseButton.new()
		event.button_index = MOUSE_BUTTON_LEFT
		event.position = point
		event.pressed = pressed
		Input.parse_input_event(event)
		await process_frame

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	root.size = Vector2i(1100, 720)
	var sim = main.simulation
	check(sim.found_airline("Map Test Air", "DE", "EDDF").is_empty(), "Found airline")
	check(sim.buy_aircraft("atr72_600").is_empty(), "Buy real aircraft")
	check(sim.create_route("EDDF", "EDDM", 1).is_empty(), "Create real route")
	sim.hire_missing_staff()
	check(sim.schedule_route(1, 1, 120).is_empty(), "Schedule real flight")
	main._sync_view()
	await process_frame
	await click_at(main.navigation[3].get_global_rect().get_center())
	check(main.current_page == 3, "Actual Karte navigation button")
	var panel = main.map_panel
	var canvas = panel.canvas
	check(not canvas.show_competitors and not panel.competitor_layer.button_pressed, "Competitor layer defaults off")
	var offers: Array = sim.get_competitor_routes()
	check(offers.size() == 2, "Real AI public schedules")
	var expected_keys := ["airline", "business_ticket_eur", "departures_per_day_per_direction", "destination_icao", "model", "origin_icao", "ticket_eur"]
	for offer in offers:
		var keys: Array = offer.keys()
		keys.sort()
		check(keys == expected_keys, "Public dictionary allowlist excludes confidential fields")
	await click_at(panel.competitor_layer.get_global_rect().get_center())
	check(canvas.show_competitors, "Layer checkbox enables competitors")
	main._show_page(2)
	main._show_page(3)
	check(canvas.show_competitors, "Layer persists across navigation")
	var rebuilds: int = panel.geometry_rebuilds
	for _i in range(5): panel.refresh_flights()
	check(panel.geometry_rebuilds == rebuilds, "Unchanged schedules reuse competitor geometry")

	check(canvas.flights.is_empty(), "No invented aircraft before departure")
	sim.start()
	sim.advance(60 + sim.get_operations()[0]["flight_seconds"] * 0.4)
	sim.pause()
	main._show_page(3)
	main._refresh_time()
	panel.refresh()
	await process_frame
	await process_frame
	await process_frame
	var flight: Dictionary = canvas.flights[0]
	var point: Vector2 = canvas.project(flight["position"])
	await click_at(canvas.global_position + point)
	check(panel.selected_id == 1 and flight["flight_number"] in panel.details.text, "Aircraft click shows flight number")
	for text in [flight["model"], "EDDF", "EDDM", "40.0 %", flight["arrival"]]:
		check(text in panel.details.text, "Details: " + text)
	var before: Array = sim.get_map_flights()
	sim.advance(100)
	panel.refresh_flights()
	check(sim.get_map_flights() == before, "Paused position frozen")
	await process_frame
	await process_frame
	var static_draws: int = canvas.background.draw_count
	for _i in range(5):
		panel.refresh_flights()
		canvas.queue_redraw()
		await process_frame
	check(canvas.background.draw_count == static_draws, "Flight redraws reuse the retained basemap")
	point = canvas.project(flight["position"])
	var wheel := InputEventMouseButton.new()
	wheel.button_index = MOUSE_BUTTON_WHEEL_UP
	wheel.pressed = true
	wheel.position = canvas.global_position + point
	Input.parse_input_event(wheel)
	await process_frame
	var wheel_release := InputEventMouseButton.new()
	wheel_release.button_index = MOUSE_BUTTON_WHEEL_UP
	wheel_release.position = wheel.position
	Input.parse_input_event(wheel_release)
	await process_frame
	check(canvas.zoom > 1, "Mouse wheel zoom")
	check(canvas.background.draw_count > static_draws, "Zoom invalidates the basemap")
	check(canvas.project(flight["position"]).distance_to(point) < 0.1, "Zoom anchored to cursor")
	var start: Vector2 = canvas.global_position + canvas.size / 2
	var press := InputEventMouseButton.new()
	press.button_index = MOUSE_BUTTON_LEFT
	press.pressed = true
	press.position = start
	Input.parse_input_event(press)
	await process_frame
	var old_offset: Vector2 = canvas.offset
	var motion := InputEventMouseMotion.new()
	motion.position = start + Vector2(50, 0)
	motion.relative = Vector2(50, 0)
	motion.button_mask = MOUSE_BUTTON_MASK_LEFT
	Input.parse_input_event(motion)
	await process_frame
	var release := InputEventMouseButton.new()
	release.button_index = MOUSE_BUTTON_LEFT
	release.position = motion.position
	Input.parse_input_event(release)
	await process_frame
	check(canvas.offset.x > old_offset.x + 49, "Mouse drag pans")
	check(panel.selected_id == 1, "Dragging does not deselect aircraft")
	await click_at(canvas.global_position + canvas.project(flight["position"]))
	check(panel.selected_id == 1, "Aircraft hit testing follows zoom and camera")
	var seam: PackedVector2Array = canvas.unwrap_path(PackedVector2Array([Vector2(170, 0), Vector2(-170, 0)]))
	check(seam[1].x == 190, "Dateline route stays short")
	var test_dir := OS.get_environment("TEMP").path_join("opencode")
	DirAccess.make_dir_recursive_absolute(test_dir)
	var save_path := test_dir.path_join("airline-map-ui-%d.json" % OS.get_process_id())
	main.save_path = save_path
	await click_at(main.save_button.get_global_rect().get_center())
	check(FileAccess.file_exists(save_path), "Save button writes isolated file: " + main.save_status.text)
	sim.start()
	sim.advance(10)
	await click_at(main.load_button.get_global_rect().get_center())
	check(sim.get_map_flights() == before, "Save/load restores exact in-flight map snapshot")
	check(not sim.is_running(), "Load restores pause")
	DirAccess.remove_absolute(save_path)
	for speed in [1, 4, 16]:
		sim.set_speed(speed)
		sim.start()
		var prior: float = sim.get_map_flights()[0]["progress"]
		sim.advance(0.25)
		var actual: float = sim.get_map_flights()[0]["progress"]
		check(absf(actual - prior - 0.25 * speed / sim.get_operations()[0]["flight_seconds"]) < 0.000001, "Position at %dx" % speed)
	sim.pause()
	main._show_page(3)
	main._refresh_time()
	panel.refresh()
	await process_frame
	canvas.reset_view()
	# A useful Europe view for the real renderer screenshot.
	canvas.zoom_at(canvas.project(Vector2(12, 49)), 4.0)
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture="):
			await RenderingServer.frame_post_draw
			check(root.get_texture().get_image().save_png(arg.trim_prefix("--capture=")) == OK, "Real viewport screenshot")
	root.size = Vector2i(680, 520)
	await process_frame
	await process_frame
	check(canvas.size.x <= 640, "Map fits narrow window")
	check(main.navigation[3].get_global_rect().end.x <= 680, "Navigation fits narrow window")
	check(main.save_button.get_global_rect().end.x <= 680, "Save remains reachable")
	canvas.reset_view()
	# Route geometry uses the same seam-aware projection as rendering.
	var route_point: Vector2 = canvas.project(canvas.routes[0]["points"][90])
	canvas.show_airports = false
	canvas.show_flights = false
	check(canvas.hit_test(route_point) == 0 and canvas.selected_route == 1, "Own route arc can be selected")
	panel.selected_id = 0
	panel.refresh_flights()
	await process_frame
	await process_frame
	check(panel.card.visible, "Route selection opens card")
	check(Rect2(Vector2.ZERO, canvas.size).encloses(panel.card.get_rect()), "Route card clamped at narrow width: %s / %s" % [panel.card.get_rect(), canvas.size])
	var own_card := ""
	for child in panel.card_body.get_children():
		if child is Label: own_card += child.text
	check("Regional Wings" in own_card and "90 EUR" in own_card, "Own route names public competitor offer")
	canvas.show_routes = false
	var competitor_point: Vector2 = canvas.project(canvas.competitor_paths[1][90])
	canvas.hit_test(competitor_point)
	check(canvas.selected_competitor == 1, "Competitor arc hit testing")
	panel.selected_id = 0
	panel.refresh_flights()
	await process_frame
	await process_frame
	check(panel.card.visible and "Continental Sky" in panel.details.text, "Read-only competitor card")
	for child in panel.card_body.get_children(): check(not child is Button, "Competitor card has no edit actions")
	check(Rect2(Vector2.ZERO, canvas.size).encloses(panel.card.get_rect()), "Competitor card fits narrow map")
	TranslationServer.set_locale("en")
	check(panel.tr("MAP_COMPETITORS") == "Competitor routes", "English map catalog")
	check(panel.tr("MAP_FREQUENCY") == "%.1f departures/day each way", "English frequency catalog")
	TranslationServer.set_locale("de")
	panel.competitor_layer.button_pressed = false
	check(canvas.selected_competitor == -1 and not panel.card.visible, "Hiding layer clears competitor selection")
	canvas.show_routes = true
	canvas.selected_route = 1
	panel.refresh_flights()
	var escape := InputEventKey.new()
	escape.keycode = KEY_ESCAPE
	escape.pressed = true
	Input.parse_input_event(escape)
	await process_frame
	check(not panel.card.visible and canvas.selected_route == 0, "Escape closes selection card")
	canvas.show_routes = false
	canvas.hit_test(route_point)
	check(canvas.selected_route == 0, "Hidden routes cannot be selected")
	canvas.show_airports = true
	canvas.show_flights = true
	canvas.show_routes = true
	canvas.center_base()
	var base: Vector2
	for airport in canvas.airports:
		if airport["icao"] == "EDDF": base = canvas.project(airport["position"])
	check(absf(base.x - canvas.size.x / 2) < 0.1, "Base centering uses real airline airport")
	for i in range(canvas.airports.size()):
		if canvas.airports[i]["icao"] == "EDDM": panel.selected_id = -i - 1
	panel.refresh_flights()
	await process_frame
	await process_frame
	var airport_card := ""
	for child in panel.card_body.get_children():
		if child is Label: airport_card += child.text
	check("Regional Wings" in airport_card and "Map Test Air" in airport_card, "Airport card lists serving airlines with departures")
	var create_action: Button = panel.card_body.get_child(panel.card_body.get_child_count() - 2)
	create_action.pressed.emit()
	await process_frame
	check(main.current_page == 2, "Airport action opens route creation")
	var destination: OptionButton = main.routes_panel.destination
	check(destination.get_item_metadata(destination.selected) == "EDDM", "Airport action preselects real destination")
	main._show_page(3)
	panel.selected_id = 0
	canvas.selected_route = 1
	panel.refresh_flights()
	await process_frame
	await process_frame
	var schedule_action: Button = panel.card_body.get_child(panel.card_body.get_child_count() - 1)
	schedule_action.pressed.emit()
	await process_frame
	check(main.current_page == 2 and main.routes_panel.route_tree.get_selected().get_metadata(0)["id"] == 1, "Route action opens selected schedule")


	main.queue_free()
	await process_frame
	if not failed: print("PASS: map positions, pause, 1/4/16x, click, zoom, pan, dateline, save/load and narrow layout")
	quit(1 if failed else 0)
