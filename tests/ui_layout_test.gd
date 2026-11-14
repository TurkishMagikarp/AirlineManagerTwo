extends SceneTree
## Real renderer captures are opt-in; all data comes from the unchanged Rust API.

var main: Control
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Meridian Air", "DE", "EDDF").is_empty())
	for i in range(3):
		assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.set_ticket_price(1, 71).is_empty())
	sim.hire_missing_staff()
	assert(sim.schedule_route(1, 1, 120).is_empty())
	sim.set_speed(16)
	sim.start()
	sim.advance(3650.0)
	sim.pause()
	main._sync_view()
	var unchanged: Dictionary = sim.get_financial_summary().duplicate(true)
	for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
		root.size = dimensions
		for page in range(main.PAGES.size()):
			main._show_page(page)
			if page == 2:
				main.routes_panel.route_tree.get_root().get_first_child().select(0)
				main.routes_panel._selection_changed()
			var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
			scroll.scroll_vertical = 0
			for frame in range(5):
				await process_frame
			for control in [main.navigation[0], main.navigation[2], main.new_game_button, main.start_button, main.save_button, main.load_button, main.time_label, main.save_status]:
				if not root.get_visible_rect().encloses(control.get_global_rect()):
					push_error("Control outside viewport at %s: %s" % [dimensions, control])
					quit(1)
					return
			if main.size.x > dimensions.x:
				push_error("Horizontal layout overflow at %s" % dimensions)
				quit(1)
				return
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				var filename := "%s-%dx%d.png" % [["overview", "fleet", "routes", "map", "finances", "staff"][page], dimensions.x, dimensions.y]
				assert(root.get_texture().get_image().save_png(capture_dir.path_join(filename)) == OK)
			if not capture_dir.is_empty():
				var panel: Control = [main.finance_panel, main.fleet_panel, main.routes_panel, main.map_panel, main.finances_panel, main.staff_panel][page]
				scroll.scroll_vertical = int(panel.position.y)
				for frame in range(5):
					await process_frame
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-content-%dx%d.png" % [["overview", "fleet", "routes", "map", "finances", "staff"][page], dimensions.x, dimensions.y])) == OK)
			if page == 2:
				# Verify deep controls stay reachable through the page scroll.
				scroll.ensure_control_visible(main.routes_panel.schedule_button)
				for frame in range(3):
					await process_frame
				assert(root.get_visible_rect().encloses(main.routes_panel.schedule_button.get_global_rect()))
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-controls-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				scroll.ensure_control_visible(main.routes_panel.results_tree)
				for frame in range(3):
					await process_frame
				assert(root.get_visible_rect().encloses(main.routes_panel.results_tree.get_global_rect()))
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-results-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		main._open_founding()
		for frame in range(5):
			await process_frame
		assert(root.get_visible_rect().encloses(main.founding.get_global_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("founding-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		main._sync_view()
	assert(sim.get_financial_summary() == unchanged, "Navigation/layout must never change finances")
	main.queue_free()
	await process_frame
	print("PASS: populated overview/fleet/routes/map/finances/staff and founding at 1440x1000, 1100x720, 680x520; deep controls reachable; no financial changes")
	quit(0)
