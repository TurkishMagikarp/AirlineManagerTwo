extends SceneTree
## Overview "Anstehende Termine" renders the Rust commitment list; empty, ties, time advance and reload.

func _initialize() -> void:
	call_deferred("_run")

func _rows() -> Array:
	var rows := []
	var root_item: TreeItem = _main.commitments_table.get_root()
	for item in root_item.get_children():
		rows.append([item.get_text(0), item.get_text(1), item.get_text(2), item.get_text(3)])
	return rows

var _main: Control

func _run() -> void:
	TranslationServer.set_locale("de")
	_main = load("res://scenes/main.tscn").instantiate()
	root.add_child(_main)
	_main.set_process(false)
	var sim: RefCounted = _main.simulation
	assert(sim.found_airline("Commitments", "DE", "EDDF").is_empty())
	_main._sync_view()
	_main._show_page(0)
	_main._refresh_commitments()
	assert(sim.get_upcoming_commitments().is_empty())
	assert(_main.commitments_empty.visible and not _main.commitments_table.visible)
	assert(_main.commitments_jump_button.disabled and sim.get_next_commitment_jump().is_empty())
	assert(not sim.begin_jump_to_commitment(1)["started"])

	assert(sim.place_order("atr72_600", 2).is_empty())
	assert(sim.hire_pilots("737", 2).is_empty())
	var pilots: Array = sim.get_staff().filter(func(e): return e["role"] == "pilot")
	assert(sim.train_pilot(pilots[0]["id"], "ATR 72").is_empty())
	assert(sim.train_pilot(pilots[1]["id"], "ATR 72").is_empty())
	var list: Array = sim.get_upcoming_commitments()
	assert(list.size() == 4)
	for i in range(1, list.size()):
		assert(list[i - 1]["at"] <= list[i]["at"])
	assert(list[0]["kind"] == "training" and list[0]["detail"] == "ATR 72")
	assert(list[2]["kind"] == "delivery" and list[2]["title"] == "ATR 72-600")
	_main._refresh_commitments()
	assert(not _main.commitments_empty.visible and _main.commitments_table.visible)
	var rows := _rows()
	assert(rows.size() == 4)
	# Tied training deadlines: the action names the shared timestamp and is keyboard-operable.
	assert(list[0]["at"] == list[1]["at"])
	assert(not _main.commitments_jump_button.disabled and _main.commitments_jump_button.focus_mode == Control.FOCUS_ALL)
	assert(_main.commitments_jump_button.text.contains(list[0]["date"]))
	assert(_main.commitments_jump_target["at"] == list[0]["at"])
	assert(rows[0][0].begins_with("Tag ") and rows[0][1] == "Umschulung" and rows[0][3] == "ATR 72")
	assert(rows[2][1] == "Lieferung" and rows[2][2] == "ATR 72-600" and rows[2][3] == "—")

	await _capture("mixed")
	var path := "user://commitments_test.json"
	assert(sim.save_game(path).is_empty())
	assert(sim.load_game(path).is_empty())
	assert(sim.get_upcoming_commitments() == list)
	_main._refresh_commitments()
	assert(_rows() == rows)

	sim.set_speed(16)
	sim.start()
	assert(sim.advance(float(list[0]["at"]) / 16.0 + 1.0))
	var after: Array = sim.get_upcoming_commitments()
	assert(after.size() == 2 and after[0]["kind"] == "delivery")
	assert(sim.save_game(path).is_empty() and sim.load_game(path).is_empty())
	assert(sim.get_upcoming_commitments() == after)
	_main._refresh_commitments()
	assert(_rows().size() == 2)

	await _capture("after")

	# A stale displayed target advances nothing and refreshes the action.
	var before_time: float = sim.get_elapsed_seconds()
	var stale_at: int = _main.commitments_jump_target["at"]
	_main.commitments_jump_target = {"at": stale_at + 1}
	await _main._jump_to_commitment()
	assert(sim.get_elapsed_seconds() == before_time)
	assert(_main.commitments_jump_target["at"] == stale_at)

	# Jump from a fractional clock position to the exact commitment timestamp.
	assert(sim.advance(0.0123))
	sim.pause()
	assert(sim.get_elapsed_seconds() != floor(sim.get_elapsed_seconds()))
	var next: Dictionary = sim.get_next_commitment_jump()
	assert(not next.is_empty())
	await _main._jump_to_commitment()
	if _main.jump_status.text.begins_with("Sprung um"):
		assert(sim.get_elapsed_seconds() == float(next["at"]))
	else:
		assert(sim.get_elapsed_seconds() <= float(next["at"]))
	var remaining: Array = sim.get_upcoming_commitments()
	var reachable: bool = not remaining.is_empty() and float(remaining[0]["at"]) - sim.get_elapsed_seconds() <= 2592000.0
	assert(_main.commitments_jump_button.disabled == not reachable)
	_main.queue_free()
	await process_frame
	print("PASS: overview commitments empty, merged order, reload and time advance")
	quit(0)

func _capture(prefix: String) -> void:
	var args := OS.get_cmdline_user_args()
	if args.is_empty() or not args[0].begins_with("--capture-dir="): return
	var dir: String = args[0].trim_prefix("--capture-dir=")
	for dimensions in [Vector2i(1440, 1000), Vector2i(680, 520)]:
		root.size = dimensions
		_main._show_page(0)
		for frame in range(5): await process_frame
		var scroll: ScrollContainer = _main.fleet_panel.get_parent().get_parent()
		scroll.ensure_control_visible(_main.commitments_jump_button)
		for frame in range(5): await process_frame
		scroll.ensure_control_visible(_main.commitments_jump_button)
		await RenderingServer.frame_post_draw
		assert(root.get_texture().get_image().save_png(dir.path_join("overview-commitments-%s-%dx%d.png" % [prefix, dimensions.x, dimensions.y])) == OK)
