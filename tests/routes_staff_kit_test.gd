extends SceneTree
## Real Rust state drives the kit presentation, including pause/load-safe training.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim = main.simulation
	assert(sim.found_airline("Kit Routes", "DE", "EDDF").is_empty())
	main._sync_view()
	var staff = main.staff_panel
	assert(staff.staff_empty.visible and not staff.staff_tree.visible)
	assert(staff.aircraft_empty.visible and not staff.aircraft_tree.visible)
	assert(sim.buy_aircraft("atr72_600").is_empty())
	assert(sim.create_route("EDDF", "EDDM", 1).is_empty())
	main._sync_view()
	main._show_page(2)
	var routes = main.routes_panel
	var row: TreeItem = routes.route_tree.get_root().get_first_child()
	assert(row.get_text(1) == "EDDF–EDDM")
	assert(row.get_custom_font(1) == Tokens.font_for(Tokens.CODE))
	assert(row.get_icon(3) != null and row.get_custom_color(3) == Tokens.STOP)
	assert(sim.get_routes()[0]["staff_status"] in row.get_text(3))
	assert(routes.results_empty.visible and not routes.results_tree.visible)
	assert(sim.hire_missing_staff().is_empty())
	routes.refresh()
	row = routes.route_tree.get_root().get_first_child()
	row.select(0)
	routes._selection_changed()
	var before: Array = sim.get_routes().duplicate(true)
	routes.interval_input.value = 1
	routes.schedule_button.pressed.emit()
	assert(sim.get_routes() == before, "Rejected schedule cannot change the route")
	var error: Label = routes.interval_input.get_meta("field_help")
	assert(error.visible and not error.text.is_empty() and error.text == routes.status.text)
	assert(routes.interval_input.get_line_edit().get_theme_stylebox("normal").border_color == Tokens.STOP)
	routes.interval_input.value = 120
	assert(routes.interval_input.get_line_edit().get_theme_stylebox("normal").border_color == Tokens.LINE_STRONG)
	assert(sim.hire_pilots("ATR 72", 1).is_empty())
	var pilot: int = sim.get_staff()[-1]["id"]
	assert(sim.train_pilot(pilot, "737").is_empty())
	staff.refresh(true)
	assert(staff.training_rows.get_child_count() == 2)
	var progress: ProgressBar = staff.training_rows.get_child(1).get_child(0)
	assert(is_zero_approx(progress.value))
	var duration: int = sim.get_staff_overview()["conversion_days"] * 86400
	sim.start()
	sim.advance(duration / 2.0)
	sim.pause()
	staff.refresh(true)
	progress = staff.training_rows.get_child(1).get_child(0)
	assert(is_equal_approx(progress.value, 50.0))
	var path := ProjectSettings.globalize_path("user://routes-staff-kit.json")
	assert(sim.save_game(path).is_empty())
	assert(sim.load_game(path).is_empty())
	staff.refresh(true)
	progress = staff.training_rows.get_child(1).get_child(0)
	assert(is_equal_approx(progress.value, 50.0), "Reload renders the Rust deadline without local elapsed state")
	await process_frame
	staff.refresh(true)
	progress = staff.training_rows.get_child(1).get_child(0)
	assert(is_equal_approx(progress.value, 50.0), "Paused simulation cannot advance the progress bar")
	DirAccess.remove_absolute(path)
	assert(UI.signed_money(123) == "+123 EUR" and UI.signed_money(-123) == "−123 EUR")
	main.queue_free()
	await process_frame
	print("PASS: routes/staff kit states, signed results and simulation-driven training progress across pause/load")
	quit(0)
