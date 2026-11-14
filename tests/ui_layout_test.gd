extends SceneTree
## Real renderer captures are opt-in; all data comes from the unchanged Rust API.

const UI = preload("res://scripts/ui/components.gd")
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
	if not capture_dir.is_empty():
		for dimensions in [Vector2i(1440, 1000), Vector2i(1100, 720), Vector2i(680, 520)]:
			root.size = dimensions
			main._sync_view()
			main._show_page(1)
			var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
			scroll.scroll_vertical = int(main.fleet_panel.position.y)
			for frame in range(5): await process_frame
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("empty-fleet-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
			var empty_action: Control = main.empty_fleet.get_child(0).get_child(2)
			scroll.ensure_control_visible(empty_action)
			for frame in range(5): await process_frame
			assert(root.get_visible_rect().encloses(empty_action.get_global_rect()))
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("empty-fleet-action-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
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
	assert(sim.sell_aircraft(2).is_empty())
	assert(sim.lease_aircraft("atr72_600").is_empty())
	assert(sim.return_aircraft(4).is_empty())
	assert(sim.hire_pilots("ATR 72", 1).is_empty())
	assert(sim.train_pilot(sim.get_staff()[-1]["id"], "737").is_empty())
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
					push_error("Control outside viewport at %s page %d: %s %s %s" % [dimensions, page, control, control.get_global_rect(), control.text])
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
			if page == 1:
				if not capture_dir.is_empty():
					for tab in [1, 2]:
						main.fleet_tabs.current_tab = tab
						scroll.scroll_vertical = int(main.fleet_panel.position.y)
						for frame in range(5): await process_frame
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-tab-%d-%dx%d.png" % [tab, dimensions.x, dimensions.y])) == OK)
						if tab == 1:
							for i in range(main.aircraft_cards.size()):
								for action in ["buy", "lease"]:
									var button: Control = main.aircraft_cards[i][action]
									scroll.ensure_control_visible(button)
									for frame in range(5): await process_frame
									assert(root.get_visible_rect().encloses(button.get_global_rect()))
									await RenderingServer.frame_post_draw
									assert(root.get_texture().get_image().save_png(capture_dir.path_join("aircraft-%d-%s-%dx%d.png" % [i, action, dimensions.x, dimensions.y])) == OK)

				if not capture_dir.is_empty():
					main._confirm_maintenance()
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(main.decision_dialog.get_ok_button().get_global_rect()))
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("maintenance-dialog-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
					main.decision_dialog.get_cancel_button().pressed.emit()
					await process_frame
				main.fleet_tabs.current_tab = 1
				await process_frame
				# Capture the market heading, then prove every nested buy control is reachable.
				var market_scroll: ScrollContainer = main.used_rows.get_parent()
				market_scroll.scroll_vertical = 0
				var market_body: Control = market_scroll.get_parent()
				var market_shell: Control = market_body.get_meta("panel_shell")
				scroll.scroll_vertical = int(market_shell.global_position.y - main.fleet_panel.global_position.y + main.fleet_panel.position.y)
				for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("used-market-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				for card in main.used_rows.get_children():
					if card is HSeparator: continue
					var buy: Control = card.get_child(card.get_child_count() - 1).get_child(1)
					market_scroll.ensure_control_visible(buy)
					for frame in range(3): await process_frame
					scroll.ensure_control_visible(buy)
					for frame in range(3): await process_frame
					assert(root.get_visible_rect().encloses(buy.get_global_rect()))
					if not capture_dir.is_empty():
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("used-market-buy-%d-%dx%d.png" % [card.get_index(), dimensions.x, dimensions.y])) == OK)
			if page == 5:
				for target in [main.staff_panel.hire_button, main.staff_panel.staff_tree, main.staff_panel.training_rows, main.staff_panel.train_button]:
					scroll.ensure_control_visible(target)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(target.get_global_rect()))
					if not capture_dir.is_empty():
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("staff-detail-%d-%dx%d.png" % [target.get_index(), dimensions.x, dimensions.y])) == OK)
			if page == 2:
				# Verify deep controls stay reachable through the page scroll.
				scroll.ensure_control_visible(main.routes_panel.schedule_button)
				for frame in range(3):
					await process_frame
				assert(root.get_visible_rect().encloses(main.routes_panel.schedule_button.get_global_rect()))
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-controls-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
					var interval: SpinBox = main.routes_panel.interval_input
					var previous := interval.value
					interval.value = 1
					var error: String = sim.schedule_route(1, 1, 1)
					assert(not error.is_empty())
					UI.field_error(interval, error)
					scroll.ensure_control_visible(interval.get_parent())
					for frame in range(5): await process_frame
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-field-error-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
					interval.value = previous
				scroll.ensure_control_visible(main.routes_panel.results_tree)
				for frame in range(3):
					await process_frame
				assert(root.get_visible_rect().encloses(main.routes_panel.results_tree.get_global_rect()))
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-results-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
					var table: Tree = main.routes_panel.results_tree
					var text := table.get_root().get_first_child().get_tooltip_text(4)
					var tooltip: Control = table._make_custom_tooltip(text)
					var layer := CanvasLayer.new()
					root.add_child(layer)
					layer.add_child(tooltip)
					tooltip.position = Vector2(main.rail.size.x + 24, main.time_label.get_parent().get_parent().size.y + 24)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(tooltip.get_global_rect()))
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-cost-tooltip-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
					layer.queue_free()
					await process_frame
		main._open_founding()
		for frame in range(5):
			await process_frame
		assert(root.get_visible_rect().encloses(main.founding.get_global_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("founding-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		main._sync_view()
		if not capture_dir.is_empty(): await _capture_components(dimensions, sim)
	assert(sim.get_financial_summary() == unchanged, "Navigation/layout must never change finances")
	main.queue_free()
	await process_frame
	print("PASS: populated overview/fleet/routes/map/finances/staff and founding at 1440x1000, 1100x720, 680x520; deep controls reachable; no financial changes")
	quit(0)

## Extra helper specimens use only simulation figures, outside the game UI.
func _capture_components(dimensions: Vector2i, sim: RefCounted) -> void:
	var layer := CanvasLayer.new()
	layer.layer = 50
	root.add_child(layer)
	var container := PanelContainer.new()
	container.theme = main.theme
	container.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	layer.add_child(container)
	var content := VBoxContainer.new()
	container.add_child(content)
	var body := UI.panel(content, "LAUFENDER BETRIEB", UI.status_chip("Planmäßig", "go"))
	var metric := UI.metric(body, "KONTOSTAND", UI.progress(sim.get_financial_summary()["load_percent"]))
	metric.text = UI.money(sim.get_airline_info()["capital_eur"])
	var chips := HFlowContainer.new()
	body.add_child(chips)
	for pair in [["Planmäßig", "go"], ["Im Flug", "info"], ["Geparkt", "neutral"], ["Wartung fällig", "caution"], ["Keine Besatzung", "stop"]]:
		chips.add_child(UI.status_chip(pair[0], pair[1]))
	for flight in sim.get_map_flights():
		body.add_child(UI.route_leg(flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]))
		break
	var table := UI.table(["Kennung", "Status", "Kontostand"], [115, 200, 160], 110)
	body.add_child(table)
	var row := table.create_item(table.create_item())
	UI.cell_text(row, 0, sim.get_fleet()[0]["identifier"])
	UI.cell_status(row, 1, "Besatzung verfügbar", "go")
	UI.cell_text(row, 2, metric.text)
	for frame in range(8): await process_frame
	await RenderingServer.frame_post_draw
	assert(root.get_texture().get_image().save_png(capture_dir.path_join("kit-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	var dialog := UI.dialog(container, "%s verkaufen?" % sim.get_fleet()[0]["identifier"], "Das Flugzeug wird abgegeben und die Gutschrift gebucht. Der Flugplan kann dieses Flugzeug anschließend nicht mehr verwenden.", "Abbrechen", "Flugzeug verkaufen", func(): pass)
	dialog.popup_centered()
	for frame in range(8): await process_frame
	await RenderingServer.frame_post_draw
	assert(root.get_texture().get_image().save_png(capture_dir.path_join("kit-dialog-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	dialog.queue_free()
	layer.queue_free()
	await process_frame
