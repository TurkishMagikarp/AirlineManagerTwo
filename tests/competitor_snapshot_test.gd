extends SceneTree
## Public snapshot and geometry survive a real process restart, without UI ticks.
var failed := false

func _initialize() -> void:
	call_deferred("_run")

func check(condition: bool, message: String) -> void:
	if not condition:
		failed = true
		push_error("FAIL: " + message)

func snapshot(sim: RefCounted) -> Dictionary:
	return {"offers": sim.get_competitor_routes(), "geometry": sim.get_map_competitor_routes(), "airports": sim.get_map_airports()}

func _run() -> void:
	var phase := ""
	var path := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--competitor-phase="): phase = arg.trim_prefix("--competitor-phase=")
		if arg.begins_with("--competitor-path="): path = arg.trim_prefix("--competitor-path=")
	if phase.is_empty():
		path = ProjectSettings.globalize_path("user://competitor-snapshot-%d.json" % OS.get_process_id())
		for child_phase in ["write", "read"]:
			var output: Array = []
			var result := OS.execute(OS.get_executable_path(), ["--headless", "--audio-driver", "Dummy", "--path", ProjectSettings.globalize_path("res://"), "--script", "res://tests/competitor_snapshot_test.gd", "--", "--competitor-phase=" + child_phase, "--competitor-path=" + path], output, true)
			check(result == 0, "Fresh process %s: %s" % [child_phase, output])
			if failed: break
		for file in [path, path + ".snapshot"]:
			if FileAccess.file_exists(file): DirAccess.remove_absolute(file)
	else:
		var sim := SimulationClock.new()
		if phase == "write":
			check(sim.found_airline("Public restart", "DE", "EDDF").is_empty(), "Found airline")
			sim.start()
			sim.advance(7 * 86400.0)
			var before := snapshot(sim)
			check(sim.save_game(path).is_empty(), "Save isolated game")
			sim.advance(86400.0)
			var file := FileAccess.open(path + ".snapshot", FileAccess.WRITE)
			file.store_var({"before": before, "after": snapshot(sim)})
			file.close()
		else:
			check(sim.load_game(path).is_empty(), "Load game in fresh process")
			var file := FileAccess.open(path + ".snapshot", FileAccess.READ)
			var expected: Dictionary = file.get_var()
			file.close()
			check(snapshot(sim) == expected["before"], "Exact public offers, geometry and airport services after restart")
			sim.start()
			sim.advance(86400.0)
			check(snapshot(sim) == expected["after"], "Schedule continuation matches after restart")
	if not failed: print("PASS: competitor public snapshot, geometry and airport services across fresh processes")
	quit(1 if failed else 0)
