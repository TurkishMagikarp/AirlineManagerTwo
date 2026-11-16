extends SceneTree
## Real renderer captures are opt-in; all data comes from the unchanged Rust API.

const UI = preload("res://scripts/ui/components.gd")
var main: Control
var capture_dir := ""

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	TranslationServer.set_locale("de") # Verify the longer German clock labels.
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-dir="):
			capture_dir = arg.trim_prefix("--capture-dir=")
	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	root.gui_embed_subwindows = true
	if not capture_dir.is_empty():
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			main._sync_view()
			for frame in range(5): await process_frame
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("founding-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
			main.founding_scroll.ensure_control_visible(main.founding_capital)
			for frame in range(5): await process_frame
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("founding-choices-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
			main.founding_scroll.scroll_vertical = 0
			for frame in range(5): await process_frame
			await _capture_picker(main.country_select, "founding-country", dimensions)
			await _capture_picker(main.airport_select, "founding-airport", dimensions)
	if OS.get_environment("AIRLINE_UI_CAPTURE_FERRY_ONLY") == "1":
		await _ferry_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: ferry dialog, airborne fleet, map and finance captures at 1440x1000 and 680x520")
		quit(0)
		return
	assert(sim.found_airline("Meridian Air", "DE", "EDDF").is_empty())
	if OS.get_environment("AIRLINE_UI_CAPTURE_ANCILLARIES_ONLY") == "1":
		await _ancillary_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: ancillary Finances and Routes at both sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_STAFF_ONLY") == "1":
		await _staff_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: Staff pay controls, all three groups and confirmation at both sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_ORDERS_ONLY") == "1":
		await _order_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: manufacturer order preview, open orders and confirmations at both sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_CODESHARE_ONLY") == "1":
		await _codeshare_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: codeshare empty, active, partner paths and cancellation captures at both sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_FREIGHTERS_ONLY") == "1":
		await _freighter_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: freighter Fleet, Routes, Finances and map captures at both sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_FARES_ONLY") == "1":
		await _fare_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: fare advisor, autopilot switch, route chip in both sizes and languages")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_ECONOMY_ONLY") == "1":
		await _economy_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: normal, boom and downturn economy cards in both languages and sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_MILESTONES_ONLY") == "1":
		await _milestone_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: empty and reached milestone Overview and dialogs in both languages and sizes")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_TRANSFERS_ONLY") == "1":
		await _transfer_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: local/connecting route counts, feeding paths and empty state at 1440x1000 and 680x520")
		quit(0)
		return
	if OS.get_environment("AIRLINE_UI_CAPTURE_SAVES_ONLY") == "1":
		await _save_slot_views(sim)
		main.queue_free()
		await process_frame
		print("PASS: empty, populated, damaged and long-name save dialogs at 1440x1000 and 680x520")
		quit(0)
		return
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
	if not capture_dir.is_empty() and OS.get_environment("AIRLINE_UI_CAPTURE_CLOCK_ONLY") == "1":
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			for page in range(main.PAGES.size()):
				main._show_page(page)
				for frame in range(5): await process_frame
				for control in main.jump_buttons + [main.time_label, main.start_button, main.save_button, main.load_button]:
					assert(root.get_visible_rect().encloses(control.get_global_rect()))
				await RenderingServer.frame_post_draw
				var name: String = ["overview", "fleet", "routes", "map", "finances", "staff"][page]
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [name, dimensions.x, dimensions.y])) == OK)
		await _jump_views()
		main.queue_free()
		await process_frame
		print("PASS: German clock bar on all six screens, jump stop and progress at 1440x1000 and 680x520")
		quit(0)
		return
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
			for control in main.jump_buttons + [main.navigation[0], main.navigation[2], main.new_game_button, main.start_button, main.save_button, main.load_button, main.time_label, main.save_status]:
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
				scroll.scroll_vertical += int(panel.get_global_rect().position.y - scroll.get_global_rect().position.y)
				if page == 3: scroll.ensure_control_visible(main.map_panel.canvas)
				for frame in range(5):
					await process_frame
				if page == 3:
					scroll.ensure_control_visible(main.map_panel.canvas)
					for frame in range(5): await process_frame
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-content-%dx%d.png" % [["overview", "fleet", "routes", "map", "finances", "staff"][page], dimensions.x, dimensions.y])) == OK)
			if page == 2 and not capture_dir.is_empty():
				scroll.ensure_control_visible(main.routes_panel.season_label)
				for frame in range(5): await process_frame
				scroll.ensure_control_visible(main.routes_panel.season_label)
				scroll.scroll_vertical += 32
				for frame in range(5): await process_frame
				assert(root.get_visible_rect().encloses(main.routes_panel.season_label.get_global_rect()))
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-demand-factors-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				scroll.ensure_control_visible(main.routes_panel.destination)
				for frame in range(5): await process_frame
				await _capture_picker(main.routes_panel.destination, "route-airport", dimensions)
			if page == 3:
				var map = main.map_panel
				map.canvas.reset_view()
				map.competitor_layer.button_pressed = true
				map.selected_id = 0
				map.canvas.selected_route = 0
				map.canvas.selected_competitor = -1
				map.refresh_flights()
				for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("map-competitors-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				map.canvas.center_base()
				for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("map-europe-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				for kind in ["airport", "route", "competitor"]:
					map.selected_id = 0
					map.canvas.selected_route = 0
					map.canvas.selected_competitor = -1
					if kind == "airport":
						for i in range(map.canvas.airports.size()):
							if map.canvas.airports[i]["icao"] == "EGLL": map.selected_id = -i - 1
					elif kind == "competitor":
						map.canvas.selected_competitor = 1 # Zero-based distant rival index.
						map.canvas.selection_position = map.canvas.size / 2
					else:
						map.canvas.selected_route = 1
						map.canvas.selection_position = map.canvas.size / 2
					map.refresh_flights()
					if kind == "competitor":
						var offer: Dictionary = sim.get_competitor_routes()[1]
						assert(offer["airline"] in map.details.text)
					for frame in range(5): await process_frame
					assert(Rect2(Vector2.ZERO, map.canvas.size).encloses(map.card.get_rect()))
					scroll.ensure_control_visible(map.canvas)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(map.card.get_global_rect()))
					if not capture_dir.is_empty():
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("map-%s-%dx%d.png" % [kind, dimensions.x, dimensions.y])) == OK)
				map.selected_id = 0
				map.canvas.selected_route = 0
				map.canvas.selected_competitor = -1
				map.refresh_flights()
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
								for action in ["header", "buy", "lease"]:
									var button: Control = main.aircraft_cards[i]["shell"].get_child(0).get_child(0) if action == "header" else main.aircraft_cards[i][action]
									if action == "header":
										scroll.scroll_vertical += int(button.global_position.y - scroll.global_position.y) - 16
									else:
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
			if page == 4:
				var finance = main.finances_panel
				finance.route_tree.get_root().get_first_child().select(0)
				finance._select_route()
				for index in range(finance.chart_select.item_count):
					finance.chart_select.select(index)
					finance._chart()
					for frame in range(5): await process_frame
					scroll.ensure_control_visible(finance.chart)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(finance.chart.get_global_rect()), "Chart %d at %s: %s" % [index, dimensions, finance.chart.get_global_rect()])
					if not capture_dir.is_empty():
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("finance-chart-%d-%dx%d.png" % [index, dimensions.x, dimensions.y])) == OK)
				finance.chart_select.select(0)
				finance._chart()
				var targets := [finance.insurance_choice, finance.fuel_start, finance.marketing_apply, finance.marketing_history, finance.tax_history, finance.loan_borrow, finance.loan_repay, finance.loan_reschedule, finance.aircraft_journal, finance.route_tree, finance.load_chart, finance.flights_tree, finance.next_button]
				for i in range(targets.size()):
					var target: Control = targets[i]
					scroll.ensure_control_visible(target)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(target.get_global_rect()))
					if not capture_dir.is_empty():
						await RenderingServer.frame_post_draw
						assert(root.get_texture().get_image().save_png(capture_dir.path_join("finance-detail-%d-%dx%d.png" % [i, dimensions.x, dimensions.y])) == OK)
						if target is Tree:
							for bar in target.get_children(true):
								if bar is HScrollBar and bar.visible:
									bar.value = bar.max_value
									for frame in range(5): await process_frame
									await RenderingServer.frame_post_draw
									assert(root.get_texture().get_image().save_png(capture_dir.path_join("finance-detail-right-%d-%dx%d.png" % [i, dimensions.x, dimensions.y])) == OK)
									bar.value = 0
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
					for bar in main.routes_panel.results_tree.get_children(true):
						if bar is HScrollBar and bar.visible:
							bar.value = bar.max_value
							for frame in range(5): await process_frame
							await RenderingServer.frame_post_draw
							assert(root.get_texture().get_image().save_png(capture_dir.path_join("route-results-right-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
							bar.value = 0
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
	if not await _widebody_views(sim):
		quit(1)
		return
	if not await _ai_cabin_views(sim):
		quit(1)
		return
	await _ferry_views(sim)
	await _jump_views()
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


func _capture_picker(select: OptionButton, prefix: String, dimensions: Vector2i) -> void:
	select.show_popup()
	var popup := select.get_popup()
	for frame in range(5): await process_frame
	assert(popup.size.x <= dimensions.x and popup.size.y <= 280)
	await RenderingServer.frame_post_draw
	assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [prefix, dimensions.x, dimensions.y])) == OK)
	popup.set_focused_item(popup.item_count - 1)
	for frame in range(5): await process_frame
	await RenderingServer.frame_post_draw
	assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-end-%dx%d.png" % [prefix, dimensions.x, dimensions.y])) == OK)
	popup.hide()
	for frame in range(3): await process_frame


## Ordinary deposits and staff rules, with fresh companies for Atlantic and Pacific views.
func _widebody_views(sim: RefCounted) -> bool:
	for region in [["DE", "EDDF", "KJFK", "RJTT"], ["JP", "RJTT", "KLAX", "KJFK"]]:
		if not _widebody_action(sim.found_airline("Widebody Air", region[0], region[1])): return false
		if not _widebody_action(sim.lease_aircraft("a330_300")): return false
		if not _widebody_action(sim.hire_missing_staff()): return false
		if not _widebody_action(sim.lease_aircraft("b777_300er")): return false
		if not _widebody_action(sim.hire_missing_staff()): return false
		if not _widebody_action(sim.set_cabin_layout(1, 60)): return false
		if not _widebody_action(sim.set_cabin_layout(2, 80)): return false
		for i in range(2):
			if not _widebody_action(sim.create_route(region[1], region[2 + i], i + 1)): return false
			if not _widebody_action(sim.set_ticket_price(i + 1, 450)): return false
			if not _widebody_action(sim.set_business_ticket_price(i + 1, 900)): return false
			if not _widebody_action(sim.schedule_route(i + 1, 1, 1440)): return false
		sim.set_speed(1)
		sim.start()
		sim.advance(5.0 * 3600.0)
		sim.pause()
		main._sync_view()
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			for page in [1, 3]:
				main._show_page(page)
				main.fleet_tabs.current_tab = 0
				var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
				if page == 1:
					scroll.scroll_vertical = int(main.fleet_panel.position.y)
				else:
					main.map_panel.canvas.reset_view()
					main.map_panel.canvas.selected_route = 1
					main.map_panel.canvas.selection_position = main.map_panel.canvas.size / 2
					main.map_panel.refresh_flights()
					for frame in range(5): await process_frame
					scroll.ensure_control_visible(main.map_panel.canvas)
				for frame in range(5): await process_frame
				if page == 1:
					scroll.ensure_control_visible(main.fleet_tree)
					for frame in range(5): await process_frame
					if not root.get_visible_rect().encloses(main.fleet_tree.get_global_rect()):
						push_error("Widebody fleet table outside viewport")
						return false
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					if root.get_texture().get_image().save_png(capture_dir.path_join("widebody-%s-%s-%dx%d.png" % [region[1], "fleet" if page == 1 else "map", dimensions.x, dimensions.y])) != OK:
						push_error("Widebody screenshot could not be saved")
						return false
	return true


func _widebody_action(error: String) -> bool:
	if not error.is_empty():
		push_error("Widebody fixture: " + error)
		return false
	return true


## Normal two-year rivals, including their organically acquired long-haul A330.
func _ai_cabin_views(sim: RefCounted) -> bool:
	if not _widebody_action(sim.found_airline("AI cabin audit", "DE", "EDDF")): return false
	sim.set_speed(1)
	sim.start()
	sim.advance(730.0 * 86400.0)
	sim.pause()
	var offers: Array = sim.get_competitor_routes()
	var selected := -1
	for i in range(offers.size()):
		if offers[i]["origin_icao"] == "SBGR" and offers[i]["destination_icao"] == "EGLL":
			assert("A330" in offers[i]["model"])
			selected = i
	assert(selected >= 0, "Normal AI expansion must expose its widebody route on the map")
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._sync_view()
		main._show_page(3)
		var map = main.map_panel
		map.canvas.reset_view()
		map.competitor_layer.button_pressed = true
		map.selected_id = 0
		map.canvas.selected_route = 0
		map.canvas.selected_competitor = selected
		map.canvas.selection_position = map.canvas.size / 2
		map.refresh_flights()
		var scroll: ScrollContainer = map.get_parent().get_parent()
		scroll.ensure_control_visible(map.canvas)
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(map.card)
		for frame in range(5): await process_frame
		assert(root.get_visible_rect().encloses(map.card.get_global_rect()))
		assert("A330" in map.details.text)
		assert(Rect2(Vector2.ZERO, map.canvas.size).encloses(map.card.get_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ai-cabin-map-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	return true


func _ferry_views(sim: RefCounted) -> void:
	assert(sim.found_airline("Ferry capture", "DE", "EDDF").is_empty())
	assert(sim.lease_aircraft("a330_300").is_empty())
	assert(sim.hire_pilots("A330", 6).is_empty())
	main._sync_view()
	main._show_page(1)
	main.fleet_tabs.current_tab = 0
	main._refresh_fleet()
	main._open_ferry()
	for i in range(main.ferry_destination.item_count):
		if main.ferry_destination.get_item_metadata(i) == "KJFK":
			main.ferry_destination.select(i)
			main.ferry_destination.item_selected.emit(i)
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		assert(root.get_visible_rect().encloses(main.ferry_dialog.card.get_global_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ferry-dialog-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	main.ferry_dialog.get_ok_button().pressed.emit()
	await process_frame
	main.ferry_toast.queue_free()
	sim.set_speed(1)
	sim.start()
	assert(sim.advance(3600.0))
	sim.pause()
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._sync_view()
		main._show_page(1)
		var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
		scroll.ensure_control_visible(main.ferry_button)
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(main.ferry_card.get_parent().get_parent().get_parent())
		for frame in range(5): await process_frame
		assert(root.get_visible_rect().encloses(main.ferry_card.get_parent().get_parent().get_parent().get_global_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ferry-airborne-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)

		main._show_page(3)
		main.map_panel.selected_id = 1
		main.map_panel.canvas.selection_position = main.map_panel.canvas.size / 2
		main.map_panel.refresh_flights()
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(main.map_panel.card)
		for frame in range(5): await process_frame
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ferry-map-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	sim.start()
	assert(sim.advance(86400.0))
	sim.pause()
	main._sync_view()
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._show_page(4)
		main.finances_panel.refresh(true)
		var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
		scroll.ensure_control_visible(main.finances_panel.ferry_history)
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(main.finances_panel.ferry_history)
		for frame in range(5): await process_frame
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ferry-finances-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)


func _jump_views() -> void:
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Jump capture", "DE", "EDDF").is_empty())
	for i in range(3): assert(sim.buy_aircraft("atr72_600").is_empty())
	for i in range(6): assert(sim.lease_aircraft("atr72_600").is_empty())
	main._sync_view()
	main._show_page(0)
	main._jump(main.simulation.get_jump_options()[-1]["seconds"])
	while main.jump_in_progress: await process_frame
	assert("Finanznot" in main.jump_status.text)
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		assert(root.get_visible_rect().encloses(main.jump_status.get_global_rect()))
		for button in main.jump_buttons:
			assert(root.get_visible_rect().encloses(button.get_global_rect()))
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("jump-status-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		var progress: Window = main._jump_progress()
		for frame in range(5): await process_frame
		assert(progress.exclusive)
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("jump-progress-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		progress.queue_free()
		await process_frame


func _save_slot_views(sim: RefCounted) -> void:
	var directory := OS.get_cache_dir().path_join("air-save-slot-shots-%d" % OS.get_process_id())
	DirAccess.make_dir_recursive_absolute(directory)
	main.save_user_dir = directory
	main._sync_view()
	for scenario in ["empty", "populated", "damaged", "long-name"]:
		if scenario == "populated":
			assert(sim.save_slot(directory, 0, "Vor dem Zeitsprung", false).is_empty())
			assert(sim.buy_aircraft("atr72_600").is_empty())
			assert(sim.save_slot(directory, 0, "Nach dem Flugzeugkauf", true).is_empty())
			assert(sim.save_slot(directory, 1, "Zweiter Speicherplatz", false).is_empty())
		if scenario == "damaged":
			var slot := directory.path_join("saves/slot-00")
			var manifest: Dictionary = JSON.parse_string(FileAccess.get_file_as_string(slot.path_join("manifest.json")))
			var file := FileAccess.open(slot.path_join(manifest["main"]).path_join("save.json"), FileAccess.WRITE)
			file.store_string("{")
			file.close()
		if scenario == "long-name":
			assert(sim.save_slot(directory, 1, "Vor dem nächsten großen Zeitsprung – Flottenplan und Rücklagen", true).is_empty())
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			main._sync_view()
			main._open_save_slots(scenario == "empty" or scenario == "long-name")
			await process_frame
			var dialog: Control = main.save_slots_dialog
			if scenario == "long-name": dialog.select_slot(1, "Vor dem nächsten großen Zeitsprung – Flottenplan und Rücklagen")
			for frame in range(5): await process_frame
			assert(root.get_visible_rect().encloses(dialog.content.get_global_rect()))
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("save-slots-%s-%dx%d.png" % [scenario, dimensions.x, dimensions.y])) == OK)
			if scenario == "populated":
				dialog.request_delete(0, "Nach dem Flugzeugkauf")
				for frame in range(3): await process_frame
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("save-slots-confirm-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
				dialog.decision.get_cancel_button().pressed.emit()
			dialog.close_dialog()
			await process_frame
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._sync_view()
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("overview-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
	for id in [0, 1]: assert(sim.delete_save_slot(directory, id).is_empty())
	DirAccess.remove_absolute(directory.path_join("saves/legacy-imported"))
	DirAccess.remove_absolute(directory.path_join("saves"))
	DirAccess.remove_absolute(directory)


func _transfer_views(sim: RefCounted) -> void:
	for id in [1, 2]:
		assert(sim.lease_aircraft("a320_200").is_empty())
		assert(sim.create_route("EDDF", "EDDM" if id == 1 else "LTFM", id).is_empty())
		assert(sim.set_ticket_price(id, 90 if id == 1 else 120).is_empty())
		sim.hire_missing_staff()
		assert(sim.schedule_route(id, 1, 240).is_empty())
	sim.start()
	assert(sim.advance(86400.0))
	sim.pause()
	main._sync_view()
	main._show_page(2)
	var panel = main.routes_panel
	panel.route_tree.get_root().get_first_child().select(0)
	panel._selection_changed()
	assert(sim.get_routes()[0]["expected_daily_connecting"] > 0)
	assert(not panel.connections_label.text.is_empty())
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		var scroll: ScrollContainer = panel.get_parent().get_parent()
		for target in [panel.route_tree, panel.connections_label, panel.results_tree, panel.competition_label]:
			scroll.ensure_control_visible(target)
			scroll.scroll_vertical += 24
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				var name := "routes-transfers" if target == panel.route_tree else "routes-feeds" if target == panel.connections_label else "routes-transfer-results" if target == panel.results_tree else "routes-transfer-competition"
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [name, dimensions.x, dimensions.y])) == OK)
	assert(sim.stop_schedule(2).is_empty())
	panel.refresh()
	assert(panel.connections_empty.visible)
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		var scroll: ScrollContainer = panel.get_parent().get_parent()
		scroll.ensure_control_visible(panel.connections_empty)
		for frame in range(5): await process_frame
		if not capture_dir.is_empty():
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("routes-no-transfers-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)

func _milestone_views(sim: RefCounted) -> void:
	for populated in [false, true]:
		if populated:
			assert(sim.buy_aircraft("atr72_600").is_empty())
			assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
			sim.hire_missing_staff()
			assert(sim.schedule_route(1, 1, 120).is_empty())
			sim.start()
			assert(sim.advance(86400.0))
			sim.pause()
		var entries: Array = sim.get_milestones()
		assert(entries.size() == 15)
		assert(entries.filter(func(e: Dictionary): return e.reached >= 0).size() > 0 if populated else entries.all(func(e: Dictionary): return e.reached < 0))
		for language in ["de", "en"]:
			TranslationServer.set_locale(language)
			for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
				root.size = dimensions
				main._sync_view()
				main.milestones_panel.refresh()
				var scroll: ScrollContainer = main.finance_panel.get_parent().get_parent()
				scroll.scroll_vertical = 0
				for frame in range(5): await process_frame
				if dimensions.x < 900:
					scroll.scroll_vertical = int(main.milestones_panel.position.y + main.finance_panel.position.y)
					for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("milestones-%s-%s-%dx%d.png" % ["reached" if populated else "empty", language, dimensions.x, dimensions.y])) == OK)
				main.milestones_panel.open_list()
				for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("milestones-list-%s-%s-%dx%d.png" % ["reached" if populated else "empty", language, dimensions.x, dimensions.y])) == OK)
				for child in main.milestones_panel.get_children():
					if child is Window: child.queue_free()
				await process_frame
		if populated:
			main.milestones_panel.notify_reached()
			assert(sim.take_milestone_notifications().is_empty())
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("milestones-toast-680x520.png")) == OK)

func _economy_views(sim: RefCounted) -> void:
	var captured: Array[String] = []
	for week in range(520):
		var economy: Dictionary = sim.get_world_economy()
		if not captured.has(economy.phase):
			captured.append(economy.phase)
			for language in ["de", "en"]:
				TranslationServer.set_locale(language)
				for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
					root.size = dimensions
					main._sync_view()
					main._show_page(0)
					for frame in range(5): await process_frame
					var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
					scroll.ensure_control_visible(main.metrics.economy.get_parent().get_parent())
					for frame in range(5): await process_frame
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("economy-%s-%s-%dx%d.png" % [economy.phase, language, dimensions.x, dimensions.y])) == OK)
		if captured.size() == 3: break
		sim.start()
		assert(sim.advance(7.0 * 86400.0))
		sim.pause()
	assert(captured.size() == 3)


func _fare_views(sim: RefCounted) -> void:
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.set_cabin_layout(1, 12).is_empty())
	sim.hire_missing_staff()
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.set_ticket_price(1, 150).is_empty())
	assert(sim.schedule_route(1, 1, 120).is_empty())
	assert(sim.set_pricing_autopilot(1, true).is_empty())
	sim.start()
	assert(sim.advance(604800.0))
	sim.pause()
	main._sync_view()
	for locale in ["de", "en"]:
		TranslationServer.set_locale(locale)
		# Recreate the presentation to exercise translated captions and controls.
		var old: Control = main.routes_panel
		var panel = load("res://scripts/routes_panel.gd").new()
		panel.simulation = sim
		var parent: Node = old.get_parent()
		parent.remove_child(old)
		old.queue_free()
		parent.add_child(panel)
		main.routes_panel = panel
		main._show_page(2)
		panel.refresh()
		panel.route_tree.get_root().get_first_child().select(0)
		panel._selection_changed()
		assert(not panel.fare_cached.is_empty())
		assert(panel.fare_switch.button_pressed)
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			for frame in range(5): await process_frame
			var scroll: ScrollContainer = panel.get_parent().get_parent()
			for entry in [[panel.fare_box, "advisor"], [panel.route_tree, "chip"], [panel.ticket_input.get_parent(), "fare-inputs"]]:
				scroll.ensure_control_visible(entry[0])
				for frame in range(5): await process_frame
				if entry[1] == "advisor": scroll.ensure_control_visible(panel.fare_last)
				if entry[1] == "chip":
					for child in panel.route_tree.get_children(true):
						if child is HScrollBar: child.value = child.max_value
				if entry[1] != "chip": scroll.scroll_vertical += 24
				for frame in range(5): await process_frame
				await RenderingServer.frame_post_draw
				if not capture_dir.is_empty():
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("routes-%s-%s-%dx%d.png" % [entry[1], locale, dimensions.x, dimensions.y])) == OK)

func _freighter_views(_sim: RefCounted) -> void:
	await preload("res://tests/freighter_fixture.gd").setup(main, self)
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._sync_view()
		for page in [1, 2, 4, 3]:
			main._show_page(page)
			if page == 1: main.fleet_tabs.current_tab = 1
			for frame in range(5): await process_frame
			var scroll: ScrollContainer = main.routes_panel.get_parent().get_parent()
			var targets: Array = []
			if page == 1:
				targets = [[main.aircraft_cards[5]["shell"].get_child(0).get_child(0), "fleet-freighter"], [main.aircraft_cards[5]["lease"], "fleet-freighter-actions"], [main.used_rows, "used-freighter"]]
			elif page == 2:
				targets = [[main.routes_panel.route_tree, "routes-freighter"], [main.routes_panel.cargo_input.get_parent(), "route-freight-form"], [main.routes_panel.results_tree, "route-freight-results"]]
			elif page == 4:
				main.finances_panel.route_tree.get_root().get_first_child().select(0)
				main.finances_panel.route_tree.item_selected.emit()
				targets = [[main.finances_panel.metrics["cargo"].get_parent().get_parent(), "finances-freighter"], [main.finances_panel.route_tree, "finances-freight-routes"], [main.finances_panel.flights_tree, "finances-freight-flights"]]
			else:
				main.map_panel.canvas.selected_route = 1
				main.map_panel.refresh_flights()
				targets = [[main.map_panel.card, "map-freighter"]]
			for target in targets:
				scroll.ensure_control_visible(target[0])
				await process_frame
				scroll.scroll_vertical += int(target[0].get_global_rect().position.y - scroll.get_global_rect().position.y) - 16
				if target[0] is Tree:
					for child in target[0].get_children(true):
						if child is HScrollBar: child.value = child.max_value
				for frame in range(5): await process_frame
				if not capture_dir.is_empty():
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("%s-%dx%d.png" % [target[1], dimensions.x, dimensions.y])) == OK)

func _codeshare_views(sim: RefCounted) -> void:
	var fixture := OS.get_environment("AIRLINE_CODESHARE_FIXTURE")
	assert(not fixture.is_empty())
	assert(sim.load_game(fixture).is_empty())
	main._sync_view()
	main._show_page(2)
	var routes = main.routes_panel
	var panel = routes.codeshare_panel
	var scroll := routes.get_parent().get_parent() as ScrollContainer
	for lang in ["de", "en"]:
		TranslationServer.set_locale(lang)
		# Recreate the panel so every translated static label follows this locale.
		routes.remove_child(panel)
		panel.queue_free()
		panel = load("res://scripts/codeshare_panel.gd").new()
		panel.simulation = sim
		routes.add_child(panel)
		routes.move_child(panel, routes.competition_label.get_parent().get_meta("panel_shell").get_index())
		routes.codeshare_panel = panel
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			for frame in range(5): await process_frame
			scroll.scroll_vertical += int(panel.get_global_rect().position.y - scroll.get_global_rect().position.y)
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-top-%s-%d-%dx%d.png" % [lang, int(sim.get_elapsed_seconds()), dimensions.x, dimensions.y])) == OK)
			scroll.ensure_control_visible(panel.result)
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-empty-%s-%dx%d.png" % [lang, dimensions.x, dimensions.y])) == OK)
		var partner_index := 0
		for row: Dictionary in panel.rows:
			if row["name"] == "Continental Sky": break
			partner_index += 1
		panel.partner.select(partner_index)
		panel.partner.item_selected.emit(partner_index)
		panel.propose.pressed.emit()
		sim.start()
		assert(sim.advance(3.0 * 86400.0))
		sim.pause()
		panel.refresh()
		assert(panel.rows[partner_index]["active"])
		main._refresh_time()
		for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
			root.size = dimensions
			for frame in range(5): await process_frame
			scroll.scroll_vertical += int(panel.get_global_rect().position.y - scroll.get_global_rect().position.y)
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-top-%s-%d-%dx%d.png" % [lang, int(sim.get_elapsed_seconds()), dimensions.x, dimensions.y])) == OK)
			scroll.ensure_control_visible(panel.result)
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-active-%s-%dx%d.png" % [lang, dimensions.x, dimensions.y])) == OK)
			routes.refresh()
			routes.route_tree.get_root().get_first_child().select(0)
			routes._selection_changed()
			scroll.scroll_vertical += int(routes.connections_label.get_global_rect().position.y - scroll.get_global_rect().position.y)
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-paths-%s-%dx%d.png" % [lang, dimensions.x, dimensions.y])) == OK)
			panel.end.pressed.emit()
			for frame in range(5): await process_frame
			if not capture_dir.is_empty():
				await RenderingServer.frame_post_draw
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("codeshare-dialog-%s-%dx%d.png" % [lang, dimensions.x, dimensions.y])) == OK)
			panel.dialog.safe_button.pressed.emit()
			await process_frame
		panel.end.pressed.emit()
		await process_frame
		panel.dialog.danger_button.pressed.emit()
		await process_frame

func _order_views(sim: RefCounted) -> void:
	main._sync_view()
	main._show_page(1)
	main.fleet_tabs.current_tab = 1
	var panel = main.orders_panel
	var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
	panel.quantity.value = 2
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(panel.place)
		scroll.scroll_vertical += 16
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-order-preview-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		panel.place.pressed.emit()
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_visible_rect().encloses(panel.dialog.get_visible_rect()))
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-order-confirm-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		panel.dialog.get_cancel_button().pressed.emit()
		await process_frame
	panel.place.pressed.emit()
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(panel.cancel)
		scroll.scroll_vertical += 16
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-orders-open-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		panel.cancel.pressed.emit()
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_visible_rect().encloses(panel.dialog.get_visible_rect()))
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-order-cancel-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		panel.dialog.get_cancel_button().pressed.emit()
		await process_frame

	# The first delivery can be paid; the second is held using the remaining cash.
	sim.start()
	assert(sim.advance(67.0 * 86400.0))
	sim.pause()
	panel.refresh()
	assert(panel.rows.any(func(entry: Dictionary): return entry["status"] == "ORDER_HELD"))
	main._sync_view()
	main._show_page(1)
	main.fleet_tabs.current_tab = 1
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(panel.cancel)
		scroll.scroll_vertical += 16
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		assert(root.get_texture().get_image().save_png(capture_dir.path_join("fleet-orders-held-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)

func _staff_views(sim: RefCounted) -> void:
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.hire_missing_staff().is_empty())
	main._sync_view()
	main._show_page(5)
	var panel: Control = main.staff_panel
	var scroll: ScrollContainer = panel.get_parent().get_parent()
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		main._sync_view()
		main._show_page(5)
		for key in ["pilot", "cabin_crew", "technician"]:
			panel.refresh(true)
			var card: Dictionary = panel.pay_cards[key]
			for frame in range(5): await process_frame
			scroll.ensure_control_visible(card["slider"].get_parent().get_parent())
			for frame in range(5): await process_frame
			assert(root.get_visible_rect().encloses(card["apply"].get_global_rect()))
			await RenderingServer.frame_post_draw
			if not capture_dir.is_empty():
				assert(root.get_texture().get_image().save_png(capture_dir.path_join("staff-%s-%dx%d.png" % [key, dimensions.x, dimensions.y])) == OK)
		panel.pay_cards["pilot"]["slider"].value = 85
		panel._confirm_pay("pilot")
		for frame in range(5): await process_frame
		await RenderingServer.frame_post_draw
		if not capture_dir.is_empty():
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("staff-confirm-%dx%d.png" % [dimensions.x, dimensions.y])) == OK)
		panel.pay_dialog.queue_free()
		await process_frame

func _ancillary_views(sim: RefCounted) -> void:
	assert(sim.buy_aircraft("atr72_600").is_empty())
	sim.hire_missing_staff()
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	assert(sim.schedule_route(1, 1, 120).is_empty())
	assert(sim.set_ancillary_policy("aggressive").is_empty())
	sim.start()
	sim.advance(86400.0)
	sim.pause()
	main._sync_view()
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		for page in [4, 2]:
			main._show_page(page)
			var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
			scroll.scroll_vertical = 0
			for frame in range(5): await process_frame
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ancillary-%s-top-%dx%d.png" % ["finances" if page == 4 else "routes", dimensions.x, dimensions.y])) == OK)
			var target: Control
			if page == 4:
				target = main.finances_panel.ancillary_status
			else:
				main.routes_panel.route_tree.get_root().get_first_child().select(0)
				main.routes_panel._selection_changed()
				target = main.routes_panel.service_cost
			for frame in range(5): await process_frame
			scroll.ensure_control_visible(target)
			scroll.scroll_vertical += 24
			for frame in range(5): await process_frame
			assert(root.get_visible_rect().encloses(target.get_global_rect()))
			assert(main.size.x <= dimensions.x)
			await RenderingServer.frame_post_draw
			assert(root.get_texture().get_image().save_png(capture_dir.path_join("ancillary-%s-policy-%dx%d.png" % ["finances" if page == 4 else "routes", dimensions.x, dimensions.y])) == OK)

			if page == 4:
				main.finances_panel.chart_select.select(5)
				main.finances_panel._chart()
				main.finances_panel.route_tree.get_root().get_first_child().select(0)
				main.finances_panel._select_route()
				for view in [[main.finances_panel.chart, "chart"], [main.finances_panel.detail, "report"]]:
					var control: Control = view[0]
					scroll.ensure_control_visible(control)
					for frame in range(5): await process_frame
					assert(root.get_visible_rect().encloses(control.get_global_rect()))
					await RenderingServer.frame_post_draw
					assert(root.get_texture().get_image().save_png(capture_dir.path_join("ancillary-finances-%s-%dx%d.png" % [view[1], dimensions.x, dimensions.y])) == OK)
