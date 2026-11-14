extends SceneTree
## The frame renders snapshots without changing their state.

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	assert(main.simulation.found_airline("Rahmen Air", "DE", "EDDF").is_empty())
	assert(main.simulation.buy_aircraft("atr72_600").is_empty())
	main._sync_view()
	var sim: RefCounted = main.simulation
	var entries: Array = sim.get_attention_snapshot()
	assert(not entries.is_empty())
	var counts := {}
	for entry in entries:
		var destination: int = {"fleet": 1, "routes": 2, "finances": 4, "staff": 5}.get(entry["page"], 0)
		counts[destination] = counts.get(destination, 0) + 1
	for index in range(6):
		assert(main.nav_badges[index].visible == counts.has(index))
		assert(main.nav_badges[index].text == str(counts.get(index, 0)))
	var elapsed: float = sim.get_elapsed_seconds()
	for width in [1440, 900, 899, 680]:
		root.size = Vector2i(width, 720)
		for frame in range(5): await process_frame
		assert(is_equal_approx(main.rail.size.x, 56.0 if width < 900 else 208.0))
		for index in range(6):
			assert(main.navigation[index].tooltip_text == main.PAGES[index]["title"])
			assert(main.navigation[index].text == ("" if width < 900 else main.PAGES[index]["title"]))
		var scroll: ScrollContainer = main.fleet_panel.get_parent().get_parent()
		var rail_position: Vector2 = main.rail.global_position
		var time_position: Vector2 = main.time_label.global_position
		scroll.scroll_vertical = 100
		for frame in range(3): await process_frame
		assert(main.rail.global_position == rail_position)
		assert(main.time_label.global_position == time_position)
		assert(main.time_label.cells.get_child_count() == main.time_label.text.length())
		assert(sim.get_elapsed_seconds() == elapsed)
	print("PASS: rail breakpoint, snapshot badges, fixed clock, character cells and unchanged time")
	main.queue_free()
	await process_frame
	quit(0)
