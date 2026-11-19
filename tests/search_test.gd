extends SceneTree
## Global search: ranking, labels, keyboard flow from two screens, and refresh after route changes and loads.
const Search = preload("res://scripts/search_dialog.gd")
var main: Control

func _initialize() -> void:
	call_deferred("_run")

func _key(keycode: Key, ctrl := false) -> void:
	for pressed in [true, false]:
		var event := InputEventKey.new()
		event.keycode = keycode
		event.physical_keycode = keycode
		event.ctrl_pressed = ctrl
		event.pressed = pressed
		root.push_input(event)
	await process_frame

func _type(text: String) -> void:
	main.search_dialog.query.text = text
	main.search_dialog.query.text_changed.emit(text)

func _titles(rows: Array) -> Array:
	return rows.map(func(r): return r["title"])

func _run() -> void:
	TranslationServer.set_locale("en")
	var airports := [
		{"icao": "EDDF", "iata": "FRA", "name": "Frankfurt Airport"},
		{"icao": "EDDM", "iata": "MUC", "name": "Munich Airport"},
		{"icao": "LTFJ", "iata": "SAW", "name": "Istanbul Sabiha Gökçen International Airport"},
		{"icao": "LTFM", "iata": "IST", "name": "Istanbul Airport"},
		{"icao": "KXYZ", "iata": "", "name": "Munich Airport"},
	]
	var routes := [{"id": 1, "origin_icao": "EDDF", "destination_icao": "EDDM"}, {"id": 2, "origin_icao": "EDDF", "destination_icao": "EDDM"}]
	assert(Search.build_results("", airports, routes).is_empty())
	assert(Search.build_results("zzz", airports, routes).is_empty())
	var exact: Array = Search.build_results("fra", airports, routes)
	assert(exact[0]["kind"] == "airport" and exact[0]["icao"] == "EDDF")
	assert("Airport" in exact[0]["title"] and "FRA" in exact[0]["title"] and "EDDF" in exact[0]["title"])
	assert(exact.slice(1).all(func(r): return r["kind"] == "route"))
	assert(exact.size() == 3)
	assert(Search.build_results("EDDF", airports, routes)[0]["icao"] == "EDDF")
	var partial: Array = Search.build_results("istan", airports, routes)
	assert(partial.size() == 2 and partial.all(func(r): return r["kind"] == "airport"))
	assert(Search.build_results("gokcen", airports, routes)[0]["icao"] == "LTFJ")
	assert(Search.build_results("GÖKÇEN", airports, routes)[0]["icao"] == "LTFJ")
	var by_end: Array = Search.build_results("saw", airports, [{"id": 7, "origin_icao": "EDDF", "destination_icao": "LTFJ"}])
	assert(by_end.size() == 2 and by_end[1]["kind"] == "route" and by_end[1]["id"] == 7)
	assert(Search.build_results("muc", airports, routes).filter(func(r): return r["kind"] == "route").size() == 2)
	var munich: Array = Search.build_results("munich", airports, routes)
	assert(munich.filter(func(r): return r["kind"] == "airport").size() == 2)
	assert(_titles(munich).size() == munich.size() and munich.filter(func(r): return r["kind"] == "airport").map(func(r): return r["title"]).size() == 2)
	assert(munich[0]["title"] != munich[1]["title"])
	var pair_ids: Array = Search.build_results("eddf-eddm", airports, routes).filter(func(r): return r["kind"] == "route").map(func(r): return r["id"])
	assert(pair_ids == [1, 2])
	assert(Search.build_results("#2", airports, routes)[0]["id"] == 2)

	main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	# Search is a management action; without an airline nothing opens.
	main._open_search()
	assert(not is_instance_valid(main.search_dialog))
	assert(sim.found_airline("Search Air", "DE", "EDDF").is_empty())
	assert(sim.buy_aircraft("atr72_600").is_empty() and sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	main._sync_view()
	await process_frame
	var before: Dictionary = sim.get_financial_summary().duplicate(true)
	var routes_before: Array = sim.get_routes().duplicate(true)

	# Keyboard open from Finances; Escape returns to the same screen and focus.
	main._show_page(4)
	main.navigation[4].grab_focus()
	await _key(KEY_F, true)
	var dialog: Control = main.search_dialog
	assert(is_instance_valid(dialog))
	await process_frame
	assert(dialog.query.has_focus())
	assert(dialog.status.text.contains("Esc"))
	_type("zzzz")
	assert(dialog.results.is_empty() and dialog.list.item_count == 0 and dialog.status.text.contains("zzzz"))
	await _key(KEY_ESCAPE)
	assert(not is_instance_valid(main.search_dialog) or main.search_dialog.is_queued_for_deletion())
	await process_frame
	assert(main.current_page == 4 and main.navigation[4].has_focus())
	# A second shortcut press while closed reopens; the button does the same.
	main.search_button.pressed.emit()
	await process_frame
	assert(is_instance_valid(main.search_dialog))
	main.search_dialog.dismiss()
	await process_frame

	# Open from Finances, pick the airport result with arrows + Enter -> Map selection.
	await _key(KEY_F, true)
	dialog = main.search_dialog
	await process_frame
	_type("munich")
	assert(dialog.results[0]["kind"] == "airport" and dialog.results[0]["icao"] == "EDDM")
	assert(dialog.results[1]["kind"] == "route")
	await _key(KEY_DOWN)
	assert(dialog._selected() == 1)
	await _key(KEY_UP)
	assert(dialog._selected() == 0)
	await _key(KEY_ENTER)
	await process_frame
	assert(main.current_page == 3 and main.map_panel.visible)
	var airport_index: int = -main.map_panel.selected_id - 1
	assert(main.map_panel.canvas.airports[airport_index]["icao"] == "EDDM")
	assert(main.map_panel.card.visible)

	# From the Map, a route result selects the existing detail in Routes.
	await _key(KEY_F, true)
	dialog = main.search_dialog
	await process_frame
	_type("EDDM")
	var route_index := 0
	for i in range(dialog.results.size()):
		if dialog.results[i]["kind"] == "route": route_index = i
	assert(route_index > 0)
	dialog.list.select(route_index)
	dialog.query.text_submitted.emit("EDDM")
	await process_frame
	assert(main.current_page == 2 and main.routes_panel.visible)
	assert(main.routes_panel._route().get("id", 0) == routes_before[0]["id"])

	# Mouse: clicking a result opens it; clicking the scrim dismisses and restores focus.
	main._show_page(5)
	main.navigation[5].grab_focus()
	main._open_search()
	dialog = main.search_dialog
	await process_frame
	_type("EDDF")
	dialog.list.item_clicked.emit(0, Vector2.ZERO, MOUSE_BUTTON_LEFT)
	await process_frame
	assert(main.current_page == 3 and main.map_panel.canvas.airports[-main.map_panel.selected_id - 1]["icao"] == "EDDF")
	main._show_page(5)
	main.navigation[5].grab_focus()
	main._open_search()
	await process_frame
	main.search_dialog.close_button.pressed.emit()
	await process_frame
	assert(main.current_page == 5 and main.navigation[5].has_focus())

	# Results follow route changes and loads.
	assert(sim.create_route("EDDF", "EDLP", 2).is_empty())
	main._open_search()
	dialog = main.search_dialog
	await process_frame
	_type("pad")
	assert(dialog.results.filter(func(r): return r["kind"] == "route").size() == 1)
	dialog.dismiss()
	await process_frame
	var path := OS.get_cache_dir().path_join("air-search-%d.json" % OS.get_process_id())
	assert(sim.save_game(path).is_empty())
	assert(sim.delete_route(sim.get_routes()[-1]["id"]).is_empty())
	main._open_search()
	dialog = main.search_dialog
	await process_frame
	_type("pad")
	assert(dialog.results.filter(func(r): return r["kind"] == "route").is_empty())
	dialog.dismiss()
	await process_frame
	assert(sim.load_game(path).is_empty())
	main._sync_view()
	main._open_search()
	dialog = main.search_dialog
	await process_frame
	_type("pad")
	assert(dialog.results.filter(func(r): return r["kind"] == "route").size() == 1)
	dialog.dismiss()
	await process_frame
	DirAccess.remove_absolute(path)
	# Searching never changes simulation state.
	assert(sim.get_routes().size() == routes_before.size() + 1)
	assert(sim.get_financial_summary() == before)
	main.queue_free()
	await process_frame
	print("PASS: global search ranks airports and routes, works by keyboard and mouse, and tracks route changes and loads")
	quit(0)
