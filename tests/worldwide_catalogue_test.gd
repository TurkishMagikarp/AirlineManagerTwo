extends SceneTree

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	for language in ["de", "en"]:
		TranslationServer.set_locale(language)
		var main = load("res://scenes/main.tscn").instantiate()
		root.add_child(main)
		main.set_process(false)
		var countries: Array = main.simulation.get_countries()
		countries.sort_custom(func(a: Dictionary, b: Dictionary): return tr(a["name"]).naturalnocasecmp_to(tr(b["name"])) < 0)
		assert(main.country_select.item_count == countries.size() and countries.size() == 29)
		for i in range(countries.size()):
			assert(main.country_select.get_item_metadata(i) == countries[i]["code"])
			assert(main.country_select.get_item_text(i) == tr(countries[i]["name"]))
		assert(tr("Südafrika") == ("South Africa" if language == "en" else "Südafrika"))
		var expected: Array = []
		for country in countries:
			expected.append_array(main.simulation.get_airports(country["code"]))
		expected.sort_custom(func(a: Dictionary, b: Dictionary): return a["iata"].naturalnocasecmp_to(b["iata"]) < 0)
		for select in [main.routes_panel.origin, main.routes_panel.destination]:
			assert(select.item_count == 40)
			assert(select.get_popup().max_size.y == 280)
			for i in range(expected.size()):
				assert(select.get_item_metadata(i) == expected[i]["icao"])
				assert(select.get_item_text(i).begins_with(expected[i]["iata"]))
		main.queue_free()
		await process_frame
	TranslationServer.set_locale("de")
	print("PASS: worldwide countries localized and sorted in German/English; every route airport sorted by IATA with bounded menus")
	quit(0)
