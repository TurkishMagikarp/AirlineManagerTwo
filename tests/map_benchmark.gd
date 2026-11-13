extends SceneTree
## Run without --headless to include the real renderer. No personal save access.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	root.size = Vector2i(1100, 720)
	main.simulation.found_airline("Map benchmark", "DE", "EDDF")
	main.simulation.buy_aircraft("atr72_600")
	main.simulation.create_route("EDDF", "EDDM", 1)
	main.simulation.schedule_route(1, 1, 120)
	main.simulation.start()
	main.simulation.advance(600)
	main.simulation.pause()
	main._sync_view()
	main._show_page(3)
	for _i in range(10): await process_frame
	var canvas = main.map_panel.canvas
	for mode in ["flights", "pan", "zoom"]:
		var samples: Array[float] = []
		for i in range(120):
			var start := Time.get_ticks_usec()
			if mode == "pan":
				canvas.offset.x += 1
				canvas._camera_changed()
			elif mode == "zoom":
				canvas.zoom_at(canvas.size / 2, 1.01 if i < 60 else 1.0 / 1.01)
			main.map_panel.refresh_flights()
			canvas.queue_redraw()
			await RenderingServer.frame_post_draw
			samples.append(float(Time.get_ticks_usec() - start) / 1000)
			await process_frame
		samples.sort()
		print("MAP_BENCH %s median_ms=%.2f p95_ms=%.2f" % [mode, samples[60], samples[114]])
	main.queue_free()
	await process_frame
	quit()
