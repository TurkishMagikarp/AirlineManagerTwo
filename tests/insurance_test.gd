extends SceneTree
## Adapter and presentation regression. Does not verify visual layout or OS restart.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	if not _check(sim.found_airline("Insurance UI", "DE", "EDDF").is_empty(), "Found airline"):
		return
	if not _check(sim.buy_aircraft("atr72_600").is_empty(), "Buy insured hull"):
		return
	main._sync_view()
	var panel = main.finances_panel
	for index in range(3):
		panel.insurance_choice.select(index)
		panel.insurance_choice.item_selected.emit(index)
		var state: Dictionary = sim.get_insurance()
		if not _check(state["coverage"] == ["none", "basic", "full"][index], "Control forwards coverage to Rust"):
			return
		if not _check(panel.insurance_status.text.contains("Prämien:") and panel.insurance_status.text.contains("Selbstbehalte:") and panel.insurance_status.text.contains("Unversicherte Reparaturen:"), "Separate finance lines"):
			return
	var before: Dictionary = sim.get_insurance()
	if not _check(not sim.set_insurance("invalid").is_empty() and sim.get_insurance() == before, "Invalid input leaves state unchanged"):
		return
	var path := ProjectSettings.globalize_path("user://insurance-test.json")
	if not _check(sim.save_game(path).is_empty(), "Save policy"):
		return
	var restored := SimulationClock.new()
	if not _check(restored.load_game(path).is_empty() and restored.get_insurance() == before, "Restore policy via adapter"):
		return
	var saved := FileAccess.get_file_as_string(path)
	var old := FileAccess.open(path, FileAccess.WRITE)
	old.store_string(saved.replace('"version":23', '"version":22'))
	old.close()
	if not _check(restored.load_game(path).is_empty() and restored.get_insurance()["coverage"] == "none" and restored.get_insurance()["claims"].is_empty(), "v22 defaults to no coverage and empty history"):
		return
	DirAccess.remove_absolute(path)
	print("insurance_test: PASS (adapter and controls; visual/restart checks remain owner-side)")
	quit(0)

func _check(ok: bool, label: String) -> bool:
	if not ok:
		push_error(label)
		quit(1)
	return ok
