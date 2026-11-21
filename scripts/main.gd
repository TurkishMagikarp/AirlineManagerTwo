extends Control
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Presentation only. No simulation state or economic calculations.

const RoutesPanel = preload("res://scripts/routes_panel.gd")
const MapPanel = preload("res://scripts/map_panel.gd")
const FinancesPanel = preload("res://scripts/finances_panel.gd")
const StaffPanel = preload("res://scripts/staff_panel.gd")
const MilestonesPanel = preload("res://scripts/milestones_panel.gd")
var milestones_panel: VBoxContainer

const UI = preload("res://scripts/ui/components.gd")

const PAGES: Array[Dictionary] = [
	{"title": "Übersicht", "heading": "Unternehmensübersicht", "body": ""},
	{"title": "Flotte", "heading": "Deine Flotte",
	"body": ""},
	{"title": "Routen", "heading": "Dein Routennetz",
	"body": "Verbindungen planen und Flugzeuge zuweisen."},
	{"title": "Karte", "heading": "Live-Flugkarte", "body": "Dein Routennetz · Positionen zur simulierten Spielzeit"},
	{"title": "Finanzen", "heading": "Finanzen & Routencontrolling", "body": "Echte Buchungen · Abrechnung bei Landung · ganze EUR"},
	{"title": "Personal", "heading": "Personalverwaltung", "body": "Piloten, Flugbegleiter und Techniker · Gehälter werden regelmäßig gezahlt"},
]

var compact_navigation := -1
var rail: PanelContainer
var rail_brand: VBoxContainer
var difficulty_options: Dictionary
var difficulty_choices: Dictionary = {}
var difficulty_preset: Array[Button] = []
var difficulty_caption: Label
var founding_scroll: ScrollContainer
var airline_name: Label
var airline_base: Label
var nav_badges: Array[Label] = []
var cash_label: Label
var attention_button: Button
var attention_scroll: ScrollContainer
var attention_rows: VBoxContainer
var attention_signature := ""
var attention_expanded := true
var navigation: Array[Button] = []
var page_title: Label
var heading: Label
var description: Label
var current_page: int = 0
var simulation := SimulationClock.new()
var time_label: Label
var state_label: Label
var start_button: Button
var pause_button: Button
var speed_buttons: Array[Button] = []
var jump_buttons: Array[Button] = []
var jump_status: Label
var jump_attention_button: Button
var jump_attention_target: Dictionary = {}
var jump_in_progress := false
var save_slots_dialog: Control
var autosave
var save_user_dir := ProjectSettings.globalize_path("user://")
var save_button: Button
var search_button: Button
var search_dialog: Control
var load_button: Button
var save_status: Label
# Platform path only; file format, validation and all state remain in Rust.
var save_path := ProjectSettings.globalize_path("user://savegame.json")
var overview: MarginContainer
var founding: MarginContainer
var name_input: LineEdit
var country_select: OptionButton
var airport_select: OptionButton
var found_button: Button
var founding_load_button: Button
var cancel_founding_button: Button
var founding_status: Label
var new_game_dialog: Control
var quit_dialog: Control
var quit_handler: Callable
var quit_requested_during_jump := false
var founding_capital: Label
var new_game_button: Button
var founding_opener: Button
var terminal_notice: PanelContainer
var terminal_title: Label
var terminal_body: Label
var terminal_load_button: Button
var terminal_new_game_button: Button
var terminal_state := false
var orders_panel: VBoxContainer
var charters_panel: VBoxContainer
var used_rows: VBoxContainer
var used_signature := ""
var fleet_panel: VBoxContainer
var model_select: OptionButton
var model_details: Label
var buy_button: Button
var fleet_status: Label
var empty_fleet: Control
var fleet_tree: Tree
var fleet_rows: Dictionary = {}
var lease_button: Button
var disposal_button: Button
var disposal_details: Label
var selected_aircraft: int = 0
var cabin_input: SpinBox
var cabin_button: Button
var maintenance_box: VBoxContainer
var maintenance_details: Label
var maintenance_button: Button
var cancel_maintenance_button: Button
var maintenance_status: Label
var base_airports: OptionButton
var base_details: Label
var base_action: Button
var base_rows: Array[Dictionary] = []
var routes_panel: VBoxContainer
var map_panel: VBoxContainer
var finances_panel: VBoxContainer
var staff_panel: VBoxContainer
var last_operation_second: int = -1
var finance_panel: VBoxContainer
var metrics: Dictionary = {}
var finance_breakdown: Label
var operation_summary: Label
var economy_signature := ""
var events_title: Label
var events_table: Tree
var events_empty: Label
var events_signature := ""
var economy_support: HBoxContainer
var economy_toast: CanvasLayer
var metric_grid: GridContainer
var decision_dialog: Control
var previous_fleet_tab := 0
var fleet_tabs: TabBar
var fleet_sections: Array[Control] = []
var aircraft_cards: Array[Dictionary] = []
var overview_register: Tree
var commitments_table: Tree
var commitments_empty: Control
var commitments_jump_button: Button
var commitments_jump_target: Dictionary = {}
var overview_operations: VBoxContainer
var fleet_operations: VBoxContainer
var last_clock_view := ""


var ferry_card: VBoxContainer
var ferry_dialog: Control
var ferry_destination: OptionButton
var ferry_preview: Label
var ferry_button: Button
var ferry_chip: Control
var ferry_title: Label
var ferry_toast: CanvasLayer
var ferry_aircraft := 0
var ferry_staff_button: Button
var ferry_retained := {}

func _build_ferry_card(parent: VBoxContainer) -> void:
	ferry_card = UI.panel(parent, "FLUGZEUG  /  ÜBERFÜHRUNG")
	ferry_title = UI.label("", UI.EMPTY_TITLE)
	ferry_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	ferry_card.add_child(ferry_title)
	ferry_button = Button.new()
	ferry_button.text = "Flugzeug überführen"
	ferry_button.pressed.connect(func(): _open_ferry())
	ferry_card.add_child(ferry_button)

func _refresh_ferry_card(plane: Dictionary) -> void:
	ferry_card.get_parent().get_parent().get_parent().visible = not plane.is_empty()
	ferry_title.text = "%s · %s · %s" % [plane.get("identifier", ""), plane.get("model_name", ""), plane.get("location_icao", "")]
	if is_instance_valid(ferry_chip):
		ferry_card.remove_child(ferry_chip)
		ferry_chip.queue_free()
	var state: String = plane.get("ferry_state", "")
	ferry_chip = UI.status_chip(state if not state.is_empty() else {"parked": "Geparkt", "in_flight": "Unterwegs", "ground": "Bodenzeit", "maintenance": "Wartung"}.get(plane.get("status", ""), "Geparkt"), "info" if plane.get("status") == "in_flight" else "neutral")
	ferry_card.add_child(ferry_chip)
	ferry_card.move_child(ferry_chip, 1)
	ferry_button.disabled = not String(plane.get("ferry_error", "")).is_empty() and int(plane.get("ferry_pilots_missing", 0)) == 0
	ferry_button.tooltip_text = plane.get("ferry_error", "")
	if is_instance_valid(ferry_dialog): _preview_ferry()

func _open_ferry(aircraft_id := 0, destination := "", opener: Control = null) -> void:
	ferry_aircraft = aircraft_id if aircraft_id != 0 else selected_aircraft
	var from_fleet := opener == null or opener == ferry_button
	if opener == null: opener = ferry_button
	if destination.is_empty() and from_fleet and ferry_retained.get("aircraft_id", 0) == ferry_aircraft:
		destination = ferry_retained.get("destination", "")
	var plane: Dictionary = fleet_rows.get(ferry_aircraft, {})
	if plane.is_empty():
		for candidate in simulation.get_fleet():
			if candidate["id"] == ferry_aircraft:
				plane = candidate
	ferry_dialog = UI.dialog(self, "%s überführen?" % plane["identifier"], "Einmaliger Leerflug ohne Erlös. Kosten werden bei Landung gebucht; danach gilt die normale Bodenzeit.", "Abbrechen", "Überführung starten", _send_ferry)
	ferry_dialog.get_ok_button().theme_type_variation = "PrimaryButton"
	var content: VBoxContainer = ferry_dialog.card.get_child(0)
	ferry_destination = OptionButton.new()
	ferry_destination.fit_to_longest_item = false
	ferry_destination.clip_text = true
	for airport in simulation.get_maintenance_airports():
		if airport["icao"] == plane["location_icao"]: continue
		ferry_destination.add_item("%s · %s" % [airport["icao"], airport["name"]])
		ferry_destination.set_item_metadata(ferry_destination.item_count - 1, airport["icao"])
		if airport["icao"] == destination:
			ferry_destination.select(ferry_destination.item_count - 1)
	ferry_destination.item_selected.connect(func(_index: int): _preview_ferry())
	var field := UI.field(content, "Zielflughafen", ferry_destination)
	content.move_child(field, 2)
	ferry_preview = UI.label("", ThemeTokens.DATA)
	ferry_preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(ferry_preview)
	content.move_child(ferry_preview, 3)
	ferry_staff_button = Button.new()
	ferry_staff_button.theme_type_variation = "PrimaryButton"
	ferry_staff_button.visible = false
	content.add_child(ferry_staff_button)
	content.move_child(ferry_staff_button, 4)
	var dialog := ferry_dialog
	var staff_button := ferry_staff_button
	staff_button.pressed.connect(func():
		var rating: String = staff_button.get_meta("rating", "")
		var missing: int = staff_button.get_meta("missing", 0)
		dialog.set_meta("to_staff", true)
		if from_fleet:
			ferry_retained = {"aircraft_id": ferry_aircraft, "destination": str(ferry_destination.get_selected_metadata())}
		dialog.queue_free()
		_show_page(5)
		staff_panel.prepare_pilot_hire(rating, missing, "Flotte" if from_fleet else "Routen"))
	_preview_ferry()
	if opener != null:
		ferry_dialog.tree_exited.connect(func():
			if not dialog.get_meta("to_staff", false) and is_instance_valid(opener) and opener.is_visible_in_tree():
				opener.grab_focus.call_deferred())
	ferry_dialog.popup_centered()

func _preview_ferry() -> void:
	if ferry_destination.selected < 0: return
	var quote: Dictionary = simulation.preview_ferry(ferry_aircraft, ferry_destination.get_selected_metadata())
	var error: String = quote["error"]
	ferry_dialog.get_ok_button().disabled = not error.is_empty()
	ferry_dialog.get_ok_button().tooltip_text = error
	ferry_preview.text = error if not error.is_empty() else "%.0f km · %.0f min\nErwartete Kosten: %s\nTreibstoff %s · Flughäfen %s · Betrieb %s" % [quote["distance_km"], quote["flight_seconds"] / 60.0, UI.money(quote["cost_eur"]), UI.money(quote["fuel_eur"]), UI.money(quote["airport_eur"]), UI.money(quote["overhead_eur"])]
	ferry_preview.add_theme_color_override("font_color", ThemeTokens.STOP if not error.is_empty() else ThemeTokens.INK)
	var missing: int = quote.get("pilots_missing", 0)
	ferry_staff_button.visible = missing > 0
	if missing > 0:
		var rating: String = quote["pilot_rating"]
		ferry_staff_button.set_meta("rating", rating)
		ferry_staff_button.set_meta("missing", missing)
		ferry_staff_button.text = "Personal öffnen: %d Pilot%s %s" % [missing, "" if missing == 1 else "en", rating]
		ferry_preview.text = "Berechtigte Piloten fehlen: %d mit Muster %s." % [missing, rating]
	# Keep the selector and the optional Staff action in keyboard traversal inside the dialog.
	var last: Control = ferry_staff_button if missing > 0 else ferry_destination
	ferry_dialog.safe_button.focus_next = ferry_dialog.safe_button.get_path_to(ferry_destination)
	ferry_destination.focus_previous = ferry_destination.get_path_to(ferry_dialog.safe_button)
	ferry_destination.focus_next = ferry_destination.get_path_to(last if missing > 0 else ferry_dialog.danger_button)
	ferry_staff_button.focus_previous = ferry_staff_button.get_path_to(ferry_destination)
	ferry_staff_button.focus_next = ferry_staff_button.get_path_to(ferry_dialog.danger_button)
	ferry_dialog.danger_button.focus_previous = ferry_dialog.danger_button.get_path_to(last)

func _send_ferry() -> void:
	var error: String = simulation.send_ferry(ferry_aircraft, ferry_destination.get_selected_metadata())
	if is_instance_valid(ferry_toast): ferry_toast.queue_free()
	ferry_toast = UI.toast(self, "Überführung gestartet." if error.is_empty() else error, error.is_empty())
	if error.is_empty(): ferry_retained = {}
	_refresh_fleet()
	routes_panel.refresh()


func _ready() -> void:
	RenderingServer.set_default_clear_color(ThemeTokens.GROUND)
	get_window().min_size = Vector2i(680, 520)
	get_tree().set_auto_accept_quit(false)
	var margin := MarginContainer.new()
	overview = margin
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(margin)
	var frame := HBoxContainer.new()
	frame.add_theme_constant_override("separation", 0)
	margin.add_child(frame)
	rail = UI.frame_panel(frame, ThemeTokens.SPACE_3)
	rail.custom_minimum_size.x = 208
	var nav := VBoxContainer.new()
	nav.add_theme_constant_override("separation", ThemeTokens.SPACE_1)
	rail.add_child(nav)
	rail_brand = VBoxContainer.new()
	rail_brand.custom_minimum_size.y = 80
	nav.add_child(rail_brand)
	airline_name = UI.label("", UI.EMPTY_TITLE)
	airline_name.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	rail_brand.add_child(airline_name)
	airline_base = UI.label("", ThemeTokens.CODE, ThemeTokens.INK_MUTED)
	rail_brand.add_child(airline_base)
	for index in range(PAGES.size()):
		var button := Button.new()
		button.text = PAGES[index]["title"]
		button.theme_type_variation = "NavButton"
		button.icon = load("res://resources/icons/%s.svg" % ["overview", "fleet", "routes", "map", "finance", "staff"][index])
		button.alignment = HORIZONTAL_ALIGNMENT_LEFT
		button.custom_minimum_size.y = 40
		button.toggle_mode = true
		button.pressed.connect(_show_page.bind(index))
		nav.add_child(button)
		navigation.append(button)
		nav_badges.append(UI.nav_badge(button))
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	nav.add_child(spacer)
	search_button = Button.new()
	search_button.text = tr("Suche")
	search_button.tooltip_text = tr("Flughäfen und eigene Routen suchen (Strg+F)")
	search_button.theme_type_variation = "GhostButton"
	search_button.pressed.connect(_open_search)
	nav.add_child(search_button)
	new_game_button = Button.new()
	new_game_button.text = "Neues Spiel"
	new_game_button.tooltip_text = new_game_button.text
	new_game_button.pressed.connect(_open_founding.bind(new_game_button))
	nav.add_child(new_game_button)
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 0)
	frame.add_child(column)
	_add_time_controls(column)
	_build_terminal_notice(column)
	resized.connect(_resize_frame)
	_resize_frame()
	var scroll := ScrollContainer.new()
	founding_scroll = scroll
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.follow_focus = true
	column.add_child(scroll)
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	scroll.add_child(content)
	UI.content_margin(content, ThemeTokens.SPACE_5)
	_build_attention(content)
	page_title = _label("", ThemeTokens.LABEL, ThemeTokens.ACCENT_TEXT)
	heading = _label("", ThemeTokens.SCREEN_TITLE)
	description = _label("", ThemeTokens.BODY)
	for label in [page_title, heading, description]:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		content.add_child(label)
	_build_finance(content)
	_build_fleet(content)
	routes_panel = RoutesPanel.new()
	routes_panel.simulation = simulation
	routes_panel.changed.connect(func(): save_status.text = "Routen geändert · bitte Spielstand speichern.")
	routes_panel.procurement_requested.connect(_open_procurement)
	routes_panel.reposition_requested.connect(func(aircraft_id: int, destination: String, opener: Control):
		_open_ferry(aircraft_id, destination, opener))
	content.add_child(routes_panel)
	map_panel = MapPanel.new()
	map_panel.simulation = simulation
	content.add_child(map_panel)
	finances_panel = FinancesPanel.new()
	finances_panel.simulation = simulation
	content.add_child(finances_panel)
	staff_panel = StaffPanel.new()
	staff_panel.simulation = simulation
	staff_panel.changed.connect(func(): save_status.text = "Personal geändert · bitte Spielstand speichern.")
	content.add_child(staff_panel)
	_build_founding()
	_sync_view()


func _add_time_controls(column: VBoxContainer) -> void:
	var shell := UI.frame_panel(column, ThemeTokens.SPACE_3)
	var panel := VBoxContainer.new()
	shell.add_child(panel)
	var readouts := HFlowContainer.new()
	readouts.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	panel.add_child(readouts)
	time_label = UI.flap_readout()
	readouts.add_child(time_label)
	state_label = _label("", ThemeTokens.SMALL)
	readouts.add_child(state_label)
	cash_label = UI.label("", UI.CASH_STYLE)
	readouts.add_child(cash_label)
	var controls := HFlowContainer.new()
	controls.add_theme_constant_override("h_separation", ThemeTokens.SPACE_2)
	panel.add_child(controls)
	start_button = Button.new()
	start_button.text = "Start"
	start_button.pressed.connect(func():
		simulation.start()
		_refresh_time())
	controls.add_child(start_button)
	var segments := UI.segment_tray(controls)
	var group := ButtonGroup.new()
	pause_button = Button.new()
	pause_button.text = "Pause"
	pause_button.toggle_mode = true
	pause_button.button_group = group
	pause_button.theme_type_variation = "SegmentButton"
	pause_button.pressed.connect(func():
		simulation.pause()
		last_clock_view = ""
		_refresh_time())
	segments.add_child(pause_button)
	for speed in [1, 4, 16]:
		var button := Button.new()
		button.text = "%d×" % speed
		button.toggle_mode = true
		button.button_group = group
		button.theme_type_variation = "SegmentButton"
		button.pressed.connect(_set_speed.bind(speed))
		segments.add_child(button)
		speed_buttons.append(button)
	for option in simulation.get_jump_options():
		var button := Button.new()
		button.text = tr(option["label"])
		button.tooltip_text = tr("Springt unabhängig von Pause und Tempo; hält bei neuen Hinweisen oder Finanznot am Tagesabschnitt an.")
		button.pressed.connect(_jump.bind(option["seconds"]))
		controls.add_child(button)
		jump_buttons.append(button)
	jump_status = _label("", ThemeTokens.SMALL, ThemeTokens.INK_MUTED)
	jump_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	jump_status.hide()
	panel.add_child(jump_status)
	jump_attention_button = Button.new()
	jump_attention_button.text = tr("Hinweis öffnen")
	jump_attention_button.tooltip_text = tr("Öffnet den Hinweis, der den Zeitsprung angehalten hat.")
	jump_attention_button.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	jump_attention_button.hide()
	jump_attention_button.pressed.connect(_open_jump_attention)
	panel.add_child(jump_attention_button)
	save_button = Button.new()
	save_button.text = "Speichern"
	save_button.pressed.connect(_save_game)
	controls.add_child(save_button)
	load_button = Button.new()
	load_button.text = "Laden"
	load_button.theme_type_variation = "GhostButton"
	load_button.pressed.connect(_load_game)
	controls.add_child(load_button)
	autosave = preload("res://scripts/autosave.gd").new(save_user_dir.path_join("preferences.cfg"))
	save_status = _label(tr("Speicherplätze · manuell speichern und laden"), ThemeTokens.SMALL, ThemeTokens.INK_MUTED)
	save_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	save_status.tooltip_text = save_user_dir.path_join("saves")
	panel.add_child(save_status)
	_refresh_time()


func _build_terminal_notice(column: VBoxContainer) -> void:
	terminal_notice = UI.frame_panel(column, ThemeTokens.SPACE_3)
	terminal_notice.hide()
	var box := VBoxContainer.new()
	terminal_notice.add_child(box)
	terminal_title = _label("", ThemeTokens.LABEL, ThemeTokens.STOP)
	terminal_title.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(terminal_title)
	terminal_body = _label("", ThemeTokens.BODY)
	terminal_body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(terminal_body)
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", ThemeTokens.SPACE_2)
	box.add_child(row)
	terminal_load_button = Button.new()
	terminal_load_button.text = "Laden"
	terminal_load_button.pressed.connect(_load_game)
	row.add_child(terminal_load_button)
	terminal_new_game_button = Button.new()
	terminal_new_game_button.text = "Neues Spiel"
	terminal_new_game_button.pressed.connect(_open_founding.bind(terminal_new_game_button))
	row.add_child(terminal_new_game_button)


func _set_terminal_state(airline: Dictionary) -> void:
	if terminal_notice == null: return
	var ended: bool = airline.get("insolvent", false)
	if ended:
		var body := tr("%s ist insolvent: Die Finanznot wurde bis zur Frist nicht behoben. Die eingefrorene Firma bleibt einsehbar und speicherbar.") % airline.get("name", "—")
		if terminal_body.text != body: terminal_body.text = body
		terminal_title.text = tr("Unternehmensführung beendet")
	if ended == terminal_state and terminal_notice.visible == ended: return
	terminal_state = ended
	terminal_notice.visible = ended
	if is_instance_valid(commitments_jump_button): _refresh_commitments_jump()


func _quit_prompt_open() -> bool:
	return is_instance_valid(quit_dialog) and not quit_dialog.is_queued_for_deletion()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST: request_quit()


func _exit_application() -> void:
	if quit_handler.is_valid(): quit_handler.call()
	else: get_tree().quit()


func request_quit() -> void:
	if not simulation.has_airline():
		_exit_application()
		return
	if jump_in_progress:
		quit_requested_during_jump = true
		return
	if _quit_prompt_open(): return
	var return_focus := get_viewport().gui_get_focus_owner()
	var body := tr("Aktuelle Airline: %s") % simulation.get_airline_info().get("name", "—")
	body += "\n" + tr("Nicht gespeicherter Fortschritt geht verloren. Mit Abbrechen kannst du zurückkehren und über Speichern einen benannten Spielstand sichern.")
	quit_dialog = UI.dialog(self, tr("Spiel beenden?"), body, tr("Abbrechen"), tr("Beenden"), _exit_application)
	quit_dialog.add_to_group("quit_prompt")
	quit_dialog.tree_exited.connect(func():
		if is_inside_tree() and is_instance_valid(return_focus) and return_focus.is_inside_tree(): return_focus.grab_focus.call_deferred()
	)
	quit_dialog.popup_centered()


func _autosave_tick(delta: float) -> void:
	var ready: bool = simulation.has_airline() and not founding.visible and not jump_in_progress and not is_instance_valid(save_slots_dialog) and not _quit_prompt_open()
	if not autosave.tick(delta, ready): return
	var error: String = simulation.autosave(save_user_dir)
	save_status.text = tr("Autosave gespeichert.") if error.is_empty() else tr("Autosave fehlgeschlagen: %s") % error


func _jump(seconds: int) -> void:
	if terminal_state or jump_in_progress or is_instance_valid(save_slots_dialog) or _quit_prompt_open() or not simulation.begin_jump(seconds): return
	await _finish_jump()


func _jump_to_commitment() -> void:
	if terminal_state or jump_in_progress or is_instance_valid(save_slots_dialog) or _quit_prompt_open() or commitments_jump_target.is_empty(): return
	var started: Dictionary = simulation.begin_jump_to_commitment(commitments_jump_target["at"])
	if not started["started"]:
		jump_status.text = started["message"]
		jump_status.show()
		jump_attention_target = {}
		_refresh_jump_attention()
		_refresh_commitments()
		return
	await _finish_jump()


func _finish_jump() -> void:
	jump_in_progress = true
	var return_focus := get_viewport().gui_get_focus_owner()
	var progress := _jump_progress()
	var result: Dictionary = {}
	while result.is_empty():
		await get_tree().process_frame
		result = simulation.poll_jump()
	progress.hide()
	progress.queue_free()
	jump_in_progress = false
	milestones_panel.notify_reached()
	_notify_economy()
	jump_status.text = result["message"]
	jump_status.show()
	jump_attention_target = result.get("attention", {})
	last_clock_view = ""
	_refresh_time()
	_refresh_attention()
	_show_page(current_page)
	if is_instance_valid(return_focus): return_focus.grab_focus()
	if quit_requested_during_jump:
		quit_requested_during_jump = false
		request_quit()


func _current_jump_attention() -> Dictionary:
	if jump_attention_target.is_empty(): return {}
	for entry: Dictionary in simulation.get_attention_snapshot():
		var same := true
		for key in jump_attention_target:
			if entry.get(key) != jump_attention_target[key]:
				same = false
				break
		if same: return entry
	return {}


func _refresh_jump_attention() -> void:
	if jump_attention_button == null: return
	var entry := _current_jump_attention()
	if entry.is_empty(): jump_attention_target = {}
	jump_attention_button.visible = not entry.is_empty() and jump_status.visible


func _open_jump_attention() -> void:
	var entry := _current_jump_attention()
	_refresh_jump_attention()
	if not entry.is_empty(): _open_attention(entry)


func _jump_progress() -> Window:
	var progress := Window.new()
	progress.title = tr("Zeitsprung läuft")
	progress.transient = true
	progress.exclusive = true
	progress.unresizable = true
	progress.borderless = true
	add_child(progress)
	var panel := UI.frame_panel(progress, ThemeTokens.SPACE_4)
	panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var message := UI.label(tr("Zeitsprung läuft · Flüge und Abrechnungen werden verarbeitet."), ThemeTokens.BODY)
	message.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	panel.add_child(message)
	progress.popup_centered(Vector2i(420, 120))
	return progress


func _save_game() -> void:
	_open_save_slots(true)


func _load_game() -> void:
	_open_save_slots(false)


func _open_save_slots(saving: bool) -> void:
	if jump_in_progress or is_instance_valid(save_slots_dialog): return
	var layer := CanvasLayer.new()
	layer.layer = 90
	add_child(layer)
	save_slots_dialog = preload("res://scripts/save_slots_dialog.gd").new()
	layer.add_child(save_slots_dialog)
	save_slots_dialog.tree_exited.connect(layer.queue_free)
	save_slots_dialog.finished.connect(func(message: String, loaded: bool):
		save_status.text = message
		founding_status.text = message
		if loaded:
			last_clock_view = ""
			_sync_view()
			routes_panel.reset_for_world()
			_refresh_time()
	)
	save_slots_dialog.autosave_settings = autosave
	save_slots_dialog.open(simulation, save_user_dir, saving)


func _unhandled_key_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo and event.keycode == KEY_F and (event.ctrl_pressed or event.meta_pressed) and not event.alt_pressed:
		_open_search()
		get_viewport().set_input_as_handled()


func _open_search() -> void:
	if jump_in_progress or not overview.visible: return
	if is_instance_valid(search_dialog) or is_instance_valid(save_slots_dialog) or is_instance_valid(decision_dialog) or is_instance_valid(ferry_dialog): return
	var layer := CanvasLayer.new()
	layer.layer = 90
	add_child(layer)
	search_dialog = preload("res://scripts/search_dialog.gd").new()
	layer.add_child(search_dialog)
	search_dialog.tree_exited.connect(layer.queue_free)
	search_dialog.activated.connect(_open_search_result)
	search_dialog.open(simulation)


func _open_search_result(result: Dictionary) -> void:
	if result["kind"] == "airport":
		_show_page(3)
		map_panel.select_airport(result["icao"])
		navigation[3].grab_focus()
	else:
		_show_page(2)
		routes_panel.focus_route(result["id"])


# Single-file compatibility entry points for the Windows restart fixtures.
func _save_legacy_game() -> void:
	var error: String = simulation.save_game(save_path)
	save_status.text = "Spielstand gespeichert." if error.is_empty() else error
	save_status.tooltip_text = save_path


func _load_legacy_game() -> void:
	var error: String = simulation.load_game(save_path)
	save_status.text = "Spielstand geladen." if error.is_empty() else error
	save_status.add_theme_color_override("font_color", ThemeTokens.INK_MUTED)
	founding_status.text = save_status.text
	save_status.tooltip_text = save_path
	if error.is_empty():
		_sync_view()
		routes_panel.reset_for_world()
		if not simulation.has_airline():
			founding_status.text = "Alter Zeit-Spielstand geladen. Noch keine Airline vorhanden; Gründen beginnt ein neues Spiel."
	_refresh_time()


func _set_speed(speed: int) -> void:
	if not simulation.set_speed(speed):
		push_error("Rust hat die Zeitgeschwindigkeit abgelehnt.")
	last_clock_view = "" # Clicking the selected toggle must still reselect it.
	_refresh_time()


func _process(delta: float) -> void:
	_autosave_tick(delta)
	if jump_in_progress: return
	if founding.visible:
		return
	if _quit_prompt_open() or (is_instance_valid(save_slots_dialog) and not save_slots_dialog.is_queued_for_deletion()):
		_refresh_time()
		return
	if not simulation.advance(delta):
		simulation.pause()
		var message := "Simulation angehalten: Die Spielzeit konnte nicht fortgeschrieben werden (z. B. Zahlenüberlauf). Bitte letzten Spielstand laden."
		push_error(message)
		save_status.text = message
		save_status.add_theme_color_override("font_color", UI.NEGATIVE)
	_refresh_time()
	_refresh_attention()
	milestones_panel.notify_reached()
	_notify_economy()
	var second := int(simulation.get_elapsed_seconds())
	if current_page == 3 and simulation.is_running():
		map_panel.refresh_flights()
	if second != last_operation_second:
		last_operation_second = second
		if current_page == 3:
			map_panel.refresh()
		if current_page == 2:
			routes_panel.refresh_operations()
		if current_page == 0 and simulation.has_airline():
			_refresh_finance()
		if current_page == 1:
			_refresh_fleet()


func _refresh_time() -> void:
	var airline: Dictionary = simulation.get_airline_info()
	cash_label.text = UI.money(airline.get("capital_eur", 0))
	cash_label.add_theme_color_override("font_color", ThemeTokens.STOP if airline.get("capital_eur", 0) < 0 else ThemeTokens.INK)
	_set_terminal_state(airline)
	for button in jump_buttons:
		button.disabled = not simulation.has_airline() or terminal_state
	var display: String = simulation.get_display_time()
	var view := "%s/%s/%d/%s" % [display, simulation.is_running(), simulation.get_speed(), terminal_state]
	if view == last_clock_view: return
	last_clock_view = view
	time_label.text = display
	state_label.text = "%s · %d×" % ["Läuft" if simulation.is_running() else "Pausiert", simulation.get_speed()]
	state_label.add_theme_color_override("font_color", UI.POSITIVE if simulation.is_running() else UI.MUTED)
	start_button.disabled = simulation.is_running() or terminal_state
	pause_button.disabled = not simulation.is_running()
	for button in speed_buttons:
		button.disabled = terminal_state
	pause_button.set_pressed_no_signal(not simulation.is_running())
	for button in speed_buttons:
		button.set_pressed_no_signal(simulation.is_running() and button.text == "%d×" % simulation.get_speed())


func _show_page(index: int) -> void:
	if index < 0 or index >= PAGES.size():
		return
	current_page = index
	var page: Dictionary = PAGES[index]
	page_title.text = page["title"]
	page_title.visible = index == 0
	heading.text = page["heading"]
	description.text = page["body"]
	finance_panel.visible = index == 0
	description.show()
	fleet_panel.visible = index == 1
	routes_panel.visible = index == 2
	map_panel.visible = index == 3
	finances_panel.visible = index == 4
	staff_panel.visible = index == 5
	if index == 5:
		staff_panel.refresh(true)
	if index == 4:
		finances_panel.refresh(true)
	if index == 3:
		map_panel.refresh()
	if index == 2:
		routes_panel.refresh()
	if index == 0 and simulation.has_airline():
		_refresh_finance()
	if index == 1:
		fleet_tabs.current_tab = 0
		_refresh_fleet()
	for button_index in range(navigation.size()):
		navigation[button_index].set_pressed_no_signal(button_index == index)


func _notify_economy() -> void:
	var phases: PackedStringArray = simulation.take_economy_notifications()
	if not phases.is_empty():
		var names := PackedStringArray()
		for phase in phases: names.append(tr("ECONOMY_" + phase.to_upper()))
		if is_instance_valid(economy_toast): economy_toast.queue_free()
		economy_toast = UI.toast(self, tr("ECONOMY_TOAST") % " → ".join(names), true)


func _refresh_events() -> void:
	var events: Array = simulation.get_world_events()
	var key := str(events) + TranslationServer.get_locale()
	if key == events_signature: return
	events_signature = key
	events_title.text = tr("EVENTS_TITLE")
	for i in range(5):
		events_table.set_column_title(i, tr(["EVENTS_PLACE", "EVENTS_KIND", "EVENTS_DATES", "EVENTS_EFFECT", "EVENTS_STATUS"][i]))
	events_table.clear()
	events_table.visible = not events.is_empty()
	events_empty.visible = events.is_empty()
	events_empty.text = tr("EVENTS_NONE")
	var root := events_table.create_item()
	for event in events:
		var row := events_table.create_item(root)
		var weather: bool = event["kind"] == "weather"
		var kind_name: String = tr("EVENTS_WEATHER_" + str(event["weather_kind"]).to_upper()) if weather else tr("EVENTS_" + str(event["kind"]).to_upper())
		UI.cell_text(row, 0, event["place"])
		UI.cell_text(row, 1, kind_name)
		UI.cell_text(row, 2, tr("EVENTS_DAYS") % [event["starts_at"] / 86400 + 1, event["ends_at"] / 86400])
		var tooltip: String
		if weather:
			UI.cell_text(row, 3, tr("EVENTS_EFFECT_WEATHER") % ceili(event["delay_seconds"] / 60.0))
			tooltip = tr("EVENTS_TOOLTIP_WEATHER") % [kind_name, event["place"], ceili(event["delay_seconds"] / 60.0)]
		else:
			UI.cell_text(row, 3, tr("EVENTS_EFFECT_VALUE") % [_percent_change(event["leisure"]), _percent_change(event["business"])])
			tooltip = tr("EVENTS_TOOLTIP") % [kind_name, event["place"], _percent_change(event["leisure"]), _percent_change(event["business"]), _percent_change(event["cargo"])]
		if event["mine"]: tooltip += "\n" + tr("EVENTS_MINE")
		for i in range(4): row.set_tooltip_text(i, tooltip)
		var active: bool = event["status"] == "active"
		UI.cell_status(row, 4, tr("EVENTS_ACTIVE") if active else tr("EVENTS_UPCOMING"), ("go" if event["kind"] == "boom" else "caution") if active else "info")


func _percent_change(factor: float) -> String:
	return ("%+d" % roundi((factor - 1.0) * 100.0)) + (" %" if TranslationServer.get_locale().begins_with("de") else "%")


func _refresh_finance() -> void:
	var economy: Dictionary = simulation.get_world_economy()
	metrics["economy"].text = UI.decimal(economy.percent) + (" %" if TranslationServer.get_locale().begins_with("de") else "%")
	metrics["economy"].get_parent().get_child(0).text = tr("ECONOMY_TITLE")
	metrics["economy"].get_parent().get_parent().tooltip_text = tr("ECONOMY_TOOLTIP")
	var economy_key: String = str(economy.phase) + TranslationServer.get_locale()
	if economy_signature != economy_key:
		economy_signature = economy_key
		for child in economy_support.get_children():
			economy_support.remove_child(child)
			child.queue_free()
		economy_support.add_child(UI.status_chip(tr("ECONOMY_" + str(economy.phase).to_upper()), {"boom": "go", "normal": "neutral", "downturn": "caution"}[economy.phase]))
	_refresh_events()
	milestones_panel.refresh()
	var airline: Dictionary = simulation.get_airline_info()
	var finance: Dictionary = simulation.get_financial_summary()
	var financing: Dictionary = simulation.get_financing()
	heading.text = airline["name"]
	description.text = "%s  /  %s · %s" % [airline["airport_icao"], airline["airport_name"], tr(airline["country_name"])]
	metrics["balance"].text = UI.money(airline["capital_eur"])
	metrics["balance"].add_theme_color_override("font_color", UI.NEGATIVE if airline["capital_eur"] < 0 else ThemeTokens.INK)
	metrics["profit"].text = UI.money(finance["profit_eur"])
	metrics["profit"].add_theme_color_override("font_color", ThemeTokens.INK if finance["profit_eur"] >= 0 else UI.NEGATIVE)
	metrics["load"].text = "%.1f %%" % finance["load_percent"]
	metrics["flights"].text = "%d  /  %d Pax" % [finance["flights"], finance["passengers"]]
	var ticket_revenue: int = finance["ticket_revenue_eur"]
	finance_breakdown.text = "%s  %s\n    %s  %s\n    %s  %s\n    %s  %s\n    %s  %s\n%s  %s\n    %s  %s\n%s  %s\n\n" % [tr("COMPANY_REVENUE"), UI.money(finance["company_revenue_eur"]), tr("REVENUE_TICKETS"), UI.money(ticket_revenue), tr("REVENUE_CARGO"), UI.money(finance["cargo_revenue_eur"]), tr("ANCILLARY_REVENUE"), UI.money(finance["ancillary_revenue_eur"]), tr("REVENUE_CHARTER"), UI.money(finance["charter_revenue_eur"]), tr("COMPANY_COSTS"), UI.money(finance["company_costs_eur"]), tr("COST_CHARTER_PENALTY"), UI.money(finance["charter_penalty_eur"]), tr("COMPANY_RESULT"), UI.signed_money(finance["profit_eur"])]
	finance_breakdown.text += "Treibstoff                    %s\nCO2-Abgabe                 %s\nFlughäfen                   %s\nCrewkosten (historisch) %s\nGehälter                       %s\nWartung / Verwaltung  %s\nWartungschecks           %s\n\nFlugzeugkäufe             %s" % [UI.money(finance["fuel_eur"]), UI.money(finance["carbon_eur"]), UI.money(finance["airport_eur"]), UI.money(finance["crew_eur"]), UI.money(finance["payroll_eur"]), UI.money(finance["overhead_eur"]), UI.money(finance["maintenance_eur"]), UI.money(finance["purchases_eur"])]
	var operations: Array = simulation.get_operations()
	operation_summary.text = "%d Flugzeuge  ·  %d Routen  ·  %d aktive Flugpläne" % [simulation.get_fleet().size(), simulation.get_routes().size(), operations.size()]
	operation_summary.text += "\n\n%s\nRestschuld: %s · Zinsen bisher: %s · Details und Kreditaktionen unter Finanzen." % [financing["status"], UI.money(financing["debt_eur"]), UI.money(financing["interest_eur"])]
	_refresh_overview_register()
	_refresh_commitments()
	_render_operations(overview_operations)


func _build_fleet(content: VBoxContainer) -> void:
	fleet_panel = VBoxContainer.new()
	fleet_panel.add_theme_constant_override("separation", 12)
	content.add_child(fleet_panel)
	fleet_tabs = TabBar.new()
	for title in ["REGISTER", "BESCHAFFUNG", "WARTUNGSBASEN"]:
		fleet_tabs.add_tab(title)
	fleet_panel.add_child(fleet_tabs)
	for index in range(3):
		var section := VBoxContainer.new()
		section.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
		fleet_panel.add_child(section)
		fleet_sections.append(section)
	fleet_tabs.tab_changed.connect(_fleet_tab_changed)
	var grid := fleet_sections[1]
	orders_panel = preload("res://scripts/orders_panel.gd").new()
	orders_panel.simulation = simulation
	orders_panel.changed.connect(_orders_changed)
	grid.add_child(orders_panel)
	var shop := UI.panel(grid, "BESCHAFFUNG  /  FLUGZEUGMARKT")
	var purchase := HBoxContainer.new()
	purchase.add_theme_constant_override("separation", 12)
	shop.add_child(purchase)
	model_select = OptionButton.new()
	model_select.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	model_select.fit_to_longest_item = false
	model_select.clip_text = true
	for model in simulation.get_aircraft_models():
		model_select.add_item(model["name"])
		model_select.set_item_metadata(model_select.item_count - 1, model)
	model_select.item_selected.connect(_model_selected)
	purchase.add_child(model_select)
	model_details = _label("", ThemeTokens.BODY)
	model_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop.add_child(model_details)
	var offers := HFlowContainer.new()
	offers.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	offers.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	shop.add_child(offers)
	for index in range(model_select.item_count):
		var model: Dictionary = model_select.get_item_metadata(index)
		var card := UI.aircraft_card(offers, model)
		card["buy"].pressed.connect(func():
			model_select.select(index)
			_model_selected(index)
			_buy_aircraft())
		card["lease"].pressed.connect(func():
			model_select.select(index)
			_model_selected(index)
			_lease_aircraft())
		aircraft_cards.append(card)
	fleet_status = _label("", ThemeTokens.BODY)
	fleet_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fleet_panel.add_child(fleet_status)
	fleet_panel.move_child(fleet_status, 1)
	var hint := _label("Vorläufige Kaufpreise · Sitzkonfiguration und Reichweite sind vereinfachte Modellangaben.", ThemeTokens.SMALL, ThemeTokens.INK_MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop.add_child(hint)
	var used := UI.panel(grid, "GEBRAUCHTFLUGZEUGE")
	var used_scroll := ScrollContainer.new()
	used_scroll.custom_minimum_size.y = 220
	used_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	used.add_child(used_scroll)
	used_rows = VBoxContainer.new()
	used_rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	used_scroll.add_child(used_rows)
	var owned := UI.panel(fleet_sections[0], "FLOTTENREGISTER  /  EIGENTUM & LEASING")
	empty_fleet = UI.empty_state("Noch keine Flugzeuge beschafft.", hint.text, "Flugzeug kaufen", func():
		fleet_tabs.current_tab = 1
		buy_button.grab_focus())
	owned.add_child(empty_fleet)
	fleet_tree = UI.table(["Kennung", "Typ", "Standort", "Status", "Zustand", "Nächste Wartung"], [115, 120, 90, 190, 80, 210], 220)
	for index in range(6):
		fleet_tree.set_column_expand(index, index == 1 or index == 5)
	fleet_tree.item_selected.connect(_refresh_maintenance)
	owned.add_child(fleet_tree)
	_build_ferry_card(owned)
	charters_panel = preload("res://scripts/charters_panel.gd").new()
	charters_panel.simulation = simulation
	charters_panel.changed.connect(_orders_changed)
	owned.add_child(charters_panel)
	disposal_details = _label("", ThemeTokens.BODY)
	disposal_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	owned.add_child(disposal_details)
	disposal_button = Button.new()
	disposal_button.pressed.connect(_confirm_disposal)
	owned.add_child(disposal_button)
	cabin_input = SpinBox.new()
	cabin_input.get_line_edit().theme_type_variation = "NumericInput"
	cabin_input.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	cabin_input.min_value = 0
	cabin_input.step = 1
	owned.add_child(_label("Kabine: Business-Sitze festlegen · Economy nutzt die restliche Fläche", ThemeTokens.BODY))
	owned.add_child(cabin_input)
	cabin_button = Button.new()
	cabin_button.text = "Kabine umbauen"
	cabin_button.pressed.connect(func():
		var error: String = simulation.set_cabin_layout(selected_aircraft, int(cabin_input.value))
		fleet_status.text = "Kabine geändert." if error.is_empty() else error
		_refresh_fleet()
		routes_panel.refresh()
	)
	owned.add_child(cabin_button)
	_build_maintenance(owned)
	fleet_operations = UI.panel(fleet_sections[0], "LIVE-BETRIEB  /  FLOTTE & NETZWERK")
	_model_selected(model_select.selected)
	_fleet_tab_changed(0)


func _open_procurement() -> void:
	_show_page(1)
	fleet_tabs.current_tab = 1
	var scroll := fleet_panel.get_parent().get_parent() as ScrollContainer
	model_select.grab_focus()
	for _pass in range(3):
		await get_tree().process_frame
		scroll.scroll_vertical += int(model_select.global_position.y - scroll.global_position.y) - 64


func _orders_changed() -> void:
	_refresh_time()
	_refresh_attention()
	_refresh_fleet()
	save_status.text = tr("ORDER_SAVE_HINT")


func _model_selected(index: int) -> void:
	if index >= 0:
		buy_button = aircraft_cards[index]["buy"]
		lease_button = aircraft_cards[index]["lease"]
	if index < 0:
		model_details.text = "Kein Flugzeugmodell verfügbar. Bitte Katalog und Preise prüfen."
		return
	var model: Dictionary = model_select.get_item_metadata(index)
	model_details.text = ("%s\n%d kg · %d km" % [UI.money(model["price_eur"]), model["main_deck_cargo_kg"], model["range_km"]]) if model.get("freighter", false) else "%s\n%d Sitzplätze   /   %d km Referenzreichweite" % [UI.money(model["price_eur"]), model["seats"], model["range_km"]]
	model_details.text += "\nLeasing: Kaution %s · Rate %s alle %d Spieltage · erste Rate nach einer Periode. Kaution bei Rückgabe abzüglich Zustandsgebühr." % [UI.money(model["lease_deposit_eur"]), UI.money(model["lease_monthly_eur"]), model["lease_period_days"]]

	_refresh_offers()
	if buy_button.disabled:
		fleet_status.text = buy_button.tooltip_text
	fleet_status.visible = not fleet_status.text.is_empty()


func _fleet_tab_changed(index: int) -> void:
	var plane: Dictionary = fleet_rows.get(selected_aircraft, {})
	_refresh_ferry_card(plane)
	if previous_fleet_tab == 0 and index != 0 and not plane.is_empty() and int(cabin_input.value) != plane["business_seats"]:
		fleet_tabs.set_block_signals(true)
		fleet_tabs.current_tab = 0
		fleet_tabs.set_block_signals(false)
		var dialog := UI.dialog(self, "Kabinenänderung verwerfen?", "Die noch nicht übernommene Sitzkonfiguration wird verworfen.", "Abbrechen", "Änderung verwerfen", func():
			cabin_input.value = plane["business_seats"]
			fleet_tabs.current_tab = index)
		dialog.popup_centered()
		return
	previous_fleet_tab = index
	for i in range(fleet_sections.size()):
		fleet_sections[i].visible = i == index


func _refresh_offers() -> void:
	var cash: int = simulation.get_airline_info().get("capital_eur", 0)
	var current: Array = simulation.get_aircraft_models()
	for i in range(aircraft_cards.size()):
		var model: Dictionary = model_select.get_item_metadata(i)
		var live: Dictionary = current[i]
		var card: Dictionary = aircraft_cards[i]
		for child in card["status"].get_children():
			card["status"].remove_child(child)
			child.queue_free()
		var reason := ""
		if live["availability"] == "upcoming":
			card["status"].add_child(UI.status_chip(tr("ORDER_FROM") % live["available_from"], "info"))
			reason = tr("ORDER_UPCOMING_TIP")
		elif live["availability"] == "out_of_production":
			card["status"].add_child(UI.status_chip(tr("ORDER_EOP"), "caution"))
			reason = tr("ORDER_EOP_TIP")
		var broke: bool = cash < model["price_eur"]
		var blocked: bool = broke or not live["orderable"]
		card["buy"].disabled = blocked
		card["buy"].tooltip_text = reason if not live["orderable"] else "Nicht genügend Guthaben für dieses Flugzeug." if broke else "Kauft das gewählte Modell sofort zum angezeigten Preis."
		card["reason"].text = card["buy"].tooltip_text if blocked else ""
		card["reason"].visible = blocked
		card["lease"].disabled = cash < model["lease_deposit_eur"] or not live["leasable"]
		card["lease"].tooltip_text = reason if not live["leasable"] else "Nicht genügend Guthaben für die Leasingkaution." if card["lease"].disabled else ""


func _confirm_disposal() -> void:
	var plane: Dictionary = fleet_rows.get(selected_aircraft, {})
	_refresh_ferry_card(plane)
	if plane.is_empty(): return
	var aircraft_id := selected_aircraft
	var dialog := UI.dialog(self, "%s verkaufen?" % plane["identifier"] if not plane["leased"] else "%s zurückgeben?" % plane["identifier"], disposal_details.text, "Abbrechen", disposal_button.text, func():
		selected_aircraft = aircraft_id
		_dispose_aircraft())
	dialog.popup_centered()


func _lease_aircraft() -> void:
	if model_select.selected < 0:
		return
	var model: Dictionary = model_select.get_item_metadata(model_select.selected)
	var error: String = simulation.lease_aircraft(model["id"])
	fleet_status.text = "Flugzeug geleast. Kaution gebucht. Bitte speichern." if error.is_empty() else error
	_refresh_fleet()
	save_status.text = "Flotte geändert · bitte Spielstand speichern."


func _dispose_aircraft() -> void:
	var plane: Dictionary = fleet_rows.get(selected_aircraft, {})
	_refresh_ferry_card(plane)
	if plane.is_empty():
		return
	var error: String = simulation.return_aircraft(selected_aircraft) if plane["leased"] else simulation.sell_aircraft(selected_aircraft)
	fleet_status.text = "Flugzeug abgegeben und Gutschrift gebucht. Bitte speichern." if error.is_empty() else error
	_refresh_fleet()
	save_status.text = "Flotte geändert · bitte Spielstand speichern."


func _buy_aircraft() -> void:
	if model_select.selected < 0:
		return
	var model: Dictionary = model_select.get_selected_metadata()
	var error: String = simulation.buy_aircraft(model["id"])
	fleet_status.text = "Flugzeug gekauft und am Heimatflughafen geparkt. Bitte speichern." if error.is_empty() else error
	if error.is_empty():
		save_status.text = "Flugzeug gekauft · bitte Spielstand speichern."
	_refresh_fleet()
	# Feedback must remain visible even in a small, scrolled window.
	await get_tree().process_frame
	if current_page == 1 and overview.visible:
		var scroll := fleet_panel.get_parent().get_parent() as ScrollContainer
		scroll.ensure_control_visible(fleet_status)


func _refresh_used_market() -> void:
	var listings: Array = simulation.get_used_market()
	var signature := str([listings, simulation.get_airline_info().get("capital_eur", 0)])
	if signature == used_signature:
		return
	used_signature = signature
	for child in used_rows.get_children():
		used_rows.remove_child(child)
		child.queue_free()
	if listings.is_empty():
		var empty := UI.label("Keine Angebote vorhanden. Verkaufte Flugzeuge und Leasingrückgaben erscheinen hier.", ThemeTokens.BODY, ThemeTokens.INK_MUTED)
		empty.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		used_rows.add_child(empty)
	for listing in listings:
		if used_rows.get_child_count() > 0:
			used_rows.add_child(HSeparator.new())
		var card := VBoxContainer.new()
		card.add_theme_constant_override("separation", ThemeTokens.SPACE_2)
		used_rows.add_child(card)
		var model := UI.label(listing["model"], UI.EMPTY_TITLE)
		model.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		card.add_child(model)
		for catalog_model in simulation.get_aircraft_models():
			if catalog_model["name"] == listing["model"]:
				card.add_child(UI.label(catalog_model["class"], ThemeTokens.CODE, ThemeTokens.INK_MUTED))
				UI.aircraft_facts(card, catalog_model)
		var facts := UI.label("%d Tage · %.0f %% Zustand\n%.1f FH / %d Zyklen" % [listing["age_days"], listing["condition_percent"], listing["flight_hours"], listing["cycles"]], ThemeTokens.DATA)
		facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		card.add_child(facts)
		var seller := UI.label(listing["seller"], ThemeTokens.SMALL, ThemeTokens.INK_MUTED)
		seller.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		card.add_child(seller)
		var row := HBoxContainer.new()
		card.add_child(row)
		var price := UI.label(UI.money(listing["price_eur"]), ThemeTokens.DATA)
		price.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		price.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(price)
		var button := Button.new()
		button.text = "Gebraucht kaufen"
		button.disabled = simulation.get_airline_info().get("capital_eur", 0) < listing["price_eur"]
		button.tooltip_text = "Nicht genügend Guthaben für dieses Flugzeug." if button.disabled else ""
		if button.disabled:
			var reason := UI.label(button.tooltip_text, ThemeTokens.SMALL, UI.NEGATIVE)
			reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			card.add_child(reason)
			card.move_child(reason, card.get_child_count() - 2)
		button.pressed.connect(_buy_used_aircraft.bind(listing["id"]))
		row.add_child(button)


func _buy_used_aircraft(listing_id: int) -> void:
	var error: String = simulation.buy_used_aircraft(listing_id)
	fleet_status.text = "Gebrauchtflugzeug gekauft. Bitte speichern." if error.is_empty() else error
	_refresh_fleet()
	if error.is_empty():
		save_status.text = "Flotte geändert · bitte Spielstand speichern."
	await get_tree().process_frame
	if current_page == 1 and overview.visible:
		var scroll := fleet_panel.get_parent().get_parent() as ScrollContainer
		scroll.ensure_control_visible(fleet_status)


func _refresh_fleet() -> void:
	fleet_status.visible = not fleet_status.text.is_empty()
	maintenance_status.visible = not maintenance_status.text.is_empty()
	_refresh_offers()
	_refresh_used_market()
	orders_panel.refresh()
	charters_panel.refresh()
	_render_operations(fleet_operations)
	var fleet: Array = simulation.get_fleet()
	var airline: Dictionary = simulation.get_airline_info()
	if current_page == 1:
		var punctuality: Dictionary = simulation.get_financial_summary()
		description.text = "%d Flugzeuge  /  Verfügbares Guthaben: %s · Pünktlichkeit: %s (%d Flüge)" % [fleet.size(), UI.money(airline.get("capital_eur", 0)), "—" if punctuality["punctuality_percent"] < 0 else "%.1f %%" % punctuality["punctuality_percent"], punctuality["punctuality_flights"]]
		description.text += " · Pünktlichkeitsruf: %+.1f %% (%s)" % [punctuality["reputation_percent"], punctuality["reputation_trend"]]
	empty_fleet.visible = fleet.is_empty()
	fleet_tree.visible = not fleet.is_empty()
	maintenance_box.visible = not fleet.is_empty()
	fleet_rows = {}
	# Rebuilt every simulated second: keep the selected aircraft.
	fleet_tree.clear()
	var root_item := fleet_tree.create_item()
	for plane in fleet:
		fleet_rows[plane["id"]] = plane
		var row := fleet_tree.create_item(root_item)
		UI.cell_text(row, 0, plane["identifier"])
		UI.cell_text(row, 1, "%s · %s · Y %d / J %d" % [plane["model_name"], plane["ownership"], plane["economy_seats"], plane["business_seats"]])
		UI.cell_text(row, 2, plane["location_icao"])
		var phase: String = plane["ferry_state"] if not plane["ferry_state"].is_empty() else {"parked": "Geparkt", "in_flight": "Unterwegs", "ground": "Gelandet · Bodenzeit", "maintenance": "In Wartung"}.get(plane["status"], "Unbekannt")
		UI.cell_text(row, 3, "%s · Route #%d" % [phase, plane["route_id"]] if plane["route_id"] != 0 else phase)
		UI.cell_text(row, 3, row.get_text(3) + " · %d min verspätet" % ceili(plane["delay_seconds"] / 60.0) + (tr("DELAY_WEATHER") if plane["weather_seconds"] > 0 else ""))
		UI.cell_status(row, 3, row.get_text(3), "info" if plane["status"] in ["in_flight", "maintenance"] else "neutral")
		if plane["status"] == "in_flight":
			UI.cell_text(row, 2, "Ab %s (in der Luft)" % plane["location_icao"])
		var condition: float = plane.get("condition_percent", 100.0)
		UI.cell_text(row, 4, "%.0f %%" % condition)
		UI.cell_text(row, 5, _maintenance_summary(plane))
		UI.cell_status(row, 5, _maintenance_summary(plane), "info" if plane["maintenance_state"] in ["active", "pending"] else "caution" if plane["maintenance_due"] else "neutral")
		row.set_tooltip_text(0, plane["identifier"])
		row.set_tooltip_text(1, "%s · gezahlter Kaufpreis: %d EUR" % [plane["model_name"], plane["purchase_price_eur"]])
		row.set_tooltip_text(2, plane["location_name"])
		row.set_tooltip_text(4, "%.1f / %d FH · %d / %d Zyklen seit letztem Check" % [plane.get("check_flight_hours", 0.0), plane.get("interval_flight_hours", 0), plane.get("check_cycles", 0), plane.get("interval_cycles", 0)])
		row.set_tooltip_text(5, "Geschätzte Kosten: %s · Dauer %.0f h" % [UI.money(plane.get("maintenance_cost_eur", 0)), plane.get("maintenance_hours", 0.0)])
		row.set_metadata(0, plane["id"])
		if plane["id"] == selected_aircraft:
			row.select(0)
	if not fleet_rows.has(selected_aircraft) and root_item.get_first_child() != null:
		root_item.get_first_child().select(0)
	_refresh_maintenance()


func _maintenance_summary(plane: Dictionary) -> String:
	match plane.get("maintenance_state", "none"):
		"active":
			if plane.get("maintenance_waiting_staff", false):
				return "Wartet auf Techniker"
			return "%sWartung bis %s" % ["Ungeplante " if plane["maintenance_unscheduled"] else "", plane["maintenance_end"]]
		"pending":
			return "Nach Landung · bis %s" % plane["maintenance_end"]
	if plane.get("maintenance_due", false):
		return "Fällig · Wartung erforderlich"
	if not String(plane.get("due_time", "")).is_empty():
		return "Spätestens %s" % plane["due_time"]
	return "In %.0f FH / %d Zyklen" % [plane.get("interval_flight_hours", 0) - plane.get("check_flight_hours", 0.0), plane.get("interval_cycles", 0) - plane.get("check_cycles", 0)]


func _build_maintenance(parent: VBoxContainer) -> void:
	maintenance_box = VBoxContainer.new()
	maintenance_box.add_theme_constant_override("separation", 8)
	parent.add_child(maintenance_box)
	maintenance_box.add_child(_label("WARTUNG  /  AUSGEWÄHLTES FLUGZEUG", ThemeTokens.PANEL_TITLE, UI.ACCENT))
	maintenance_details = _label("", ThemeTokens.BODY, ThemeTokens.INK)
	maintenance_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	maintenance_box.add_child(maintenance_details)
	maintenance_button = Button.new()
	maintenance_button.text = "Wartung durchführen"
	maintenance_button.theme_type_variation = "PrimaryButton"
	maintenance_button.tooltip_text = "Schickt das Flugzeug in die Wartung. Flugzeuge in der Luft beenden zuerst ihren Flug; während der Wartung finden keine Abflüge statt."
	maintenance_button.pressed.connect(_confirm_maintenance)
	maintenance_box.add_child(maintenance_button)
	cancel_maintenance_button = Button.new()
	cancel_maintenance_button.text = "Wartungsauftrag stornieren"
	cancel_maintenance_button.theme_type_variation = "DangerButton"
	cancel_maintenance_button.tooltip_text = "Nur solange die Wartung noch nicht begonnen hat (Flugzeug noch in der Luft). Es fallen keine Kosten an; der Flugplan läuft weiter."
	cancel_maintenance_button.visible = false
	cancel_maintenance_button.pressed.connect(_cancel_maintenance)
	maintenance_box.add_child(cancel_maintenance_button)
	maintenance_staff_button = Button.new()
	maintenance_staff_button.text = "Techniker einstellen"
	maintenance_staff_button.theme_type_variation = "PrimaryButton"
	maintenance_staff_button.visible = false
	maintenance_staff_button.pressed.connect(func():
		_show_page(5)
		staff_panel.focus_technician_hiring())
	maintenance_box.add_child(maintenance_staff_button)
	maintenance_status = _label("", ThemeTokens.BODY)
	maintenance_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fleet_panel.add_child(maintenance_status)
	fleet_panel.move_child(maintenance_status, 2)
	var bases := UI.panel(fleet_sections[2], "WARTUNGSBASEN")
	base_airports = OptionButton.new()
	base_airports.item_selected.connect(func(_index: int) -> void: _refresh_base_action())
	bases.add_child(base_airports)
	base_details = _label("", ThemeTokens.BODY)
	base_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	bases.add_child(base_details)
	base_action = Button.new()
	base_action.pressed.connect(_change_maintenance_base)
	bases.add_child(base_action)


func _refresh_base_action() -> void:
	if base_airports.selected < 0 or base_rows.is_empty():
		base_action.disabled = true
		return
	var row: Dictionary = base_rows[base_airports.selected]
	base_action.text = "Basis schließen (keine Erstattung)" if row["is_base"] else "Basis eröffnen · %s" % UI.money(row["setup_eur"])
	base_action.disabled = not String(row["action_error"]).is_empty()
	base_details.text = "Heimatbasis · dauerhaft, gebührenfrei" if row["is_home"] else "Einrichtung %s · alle 30 Spieltage %s%s" % [UI.money(row["setup_eur"]), UI.money(row["fee_eur"]), " · nächste Gebühr " + row["next_fee"] if row["is_base"] else ""]
	if not String(row["action_error"]).is_empty():
		base_details.text += "\n" + row["action_error"]


func _change_maintenance_base() -> void:
	var row: Dictionary = base_rows[base_airports.selected]
	var error: String = simulation.set_maintenance_base(row["icao"], not row["is_base"])
	maintenance_status.text = "Wartungsbasis geändert. Bitte speichern." if error.is_empty() else error
	_refresh_fleet()


func _refresh_maintenance() -> void:
	var previous_base := base_airports.selected
	base_rows.assign(simulation.get_maintenance_airports())
	base_airports.clear()
	for row in base_rows:
		base_airports.add_item("%s · %s%s" % [row["icao"], row["name"], " · Heimatbasis" if row["is_home"] else (" · Wartungsbasis" if row["is_base"] else "")])
	if not base_rows.is_empty():
		base_airports.select(clampi(previous_base, 0, base_rows.size() - 1))
	_refresh_base_action()
	var item := fleet_tree.get_selected()
	selected_aircraft = item.get_metadata(0) if item != null else 0
	var previous_cabin_id: int = cabin_input.get_meta("aircraft_id", -1)
	var plane: Dictionary = fleet_rows.get(selected_aircraft, {})
	_refresh_ferry_card(plane)
	cabin_button.disabled = plane.is_empty() or not plane.get("can_edit_cabin", false)
	cabin_input.editable = not cabin_button.disabled
	cabin_input.max_value = plane.get("max_business_seats", 0)
	if previous_cabin_id != selected_aircraft:
		cabin_input.value = plane.get("business_seats", 0)
		cabin_input.set_meta("aircraft_id", selected_aircraft)
	cabin_button.text = "Kabine umbauen · %s · Fläche J:Y %d:1" % [UI.money(plane.get("cabin_fee_eur", 0)), plane.get("cabin_space_ratio", 2)]
	maintenance_button.disabled = plane.is_empty() or not plane.get("can_request_maintenance", false)
	cancel_maintenance_button.visible = plane.get("can_cancel_maintenance", false)
	maintenance_staff_button.visible = plane.get("maintenance_waiting_staff", false)
	disposal_button.disabled = plane.is_empty() or not String(plane.get("disposal_error", "")).is_empty()
	disposal_button.text = "Leasing zurückgeben" if plane.get("leased", false) else "Flugzeug verkaufen"
	disposal_details.text = ""
	if not plane.is_empty():
		disposal_details.text = "%s · Erwerb: %s" % [plane["ownership"], plane["acquired"]]
		if plane["leased"]:
			disposal_details.text += "\nKaution: %s · Rate: %s · nächste Rate: %s" % [UI.money(plane["lease_deposit_eur"]), UI.money(plane["lease_monthly_eur"]), plane["lease_due"]]
		if String(plane["disposal_error"]).is_empty():
			disposal_details.text += "\nRückgabe: %s (Kaution abzüglich Zustandsgebühr %s)" % [UI.money(plane["disposal_quote_eur"]), UI.money(plane["lease_deposit_eur"] - plane["disposal_quote_eur"])] if plane["leased"] else "\nVerkaufsangebot: %s" % UI.money(plane["disposal_quote_eur"])
		else:
			disposal_details.text += "\n" + String(plane["disposal_error"])
	if plane.is_empty():
		maintenance_details.text = "Flugzeug in der Tabelle auswählen."
		return
	var lines: Array[String] = [
		"%s · %s · Zustand %.0f %%" % [plane["identifier"], plane["model_name"], plane["condition_percent"]],
		"Betriebszeit %.1f FH · %d Zyklen · %d abgeschlossene Checks" % [plane["flight_hours"], plane["cycles"], plane["checks"]],
		"Seit letztem Check: %.1f / %d FH · %d / %d Zyklen (was zuerst erreicht wird)" % [plane["check_flight_hours"], plane["interval_flight_hours"], plane["check_cycles"], plane["interval_cycles"]],
	]
	lines.append("Hier / nächster Landeort: %s · %s" % [plane["maintenance_location"], plane["maintenance_location_kind"]])
	match plane["maintenance_state"]:
		"active" when plane.get("maintenance_waiting_staff", false):
			lines.append("%s wartet auf Techniker · %s" % ["Ungeplante Wartung" if plane["maintenance_unscheduled"] else "Wartung", UI.money(plane["maintenance_cost_eur"])])
			lines.append("Das Flugzeug bleibt am Boden, bis genug Techniker zugeteilt sind und die Wartung abgeschlossen ist. Benötigt %d Techniker, derzeit frei: %d." % [plane["maintenance_technicians_needed"], plane["maintenance_technicians_available"]])
		"active":
			lines.append("%s läuft: %s – %s · %s" % ["Ungeplante Wartung" if plane["maintenance_unscheduled"] else "Wartung", plane["maintenance_start"], plane["maintenance_end"], UI.money(plane["maintenance_cost_eur"])])
		"pending":
			lines.append("Wartung beauftragt: beginnt nach der Landung %s, vorauss. Ende %s · %s (bei Beginn festgelegt)" % [plane["maintenance_start"], plane["maintenance_end"], UI.money(plane["maintenance_cost_eur"])])
		_:
			if not String(plane["due_time"]).is_empty():
				lines.append("Pflichtgrenze mit aktuellem Flugplan: Landung %s" % plane["due_time"])
			lines.append("Wartung jetzt: ca. %s · %.0f h Dauer" % [UI.money(plane["maintenance_cost_eur"]), plane["maintenance_hours"]])
			lines.append("Bei Erreichen der Grenze ungeplant: %s · %.0f h (teurer, länger)" % [UI.money(plane["unscheduled_cost_eur"]), plane["unscheduled_hours"]])
	if plane["status"] == "in_flight" and plane["maintenance_state"] == "none":
		lines.append("Unterwegs: Die Wartung beginnt erst nach der Landung.")
	if not String(plane["maintenance_hint"]).is_empty() and plane["maintenance_state"] == "none":
		lines.append(plane["maintenance_hint"])
	maintenance_details.text = "\n".join(lines)


func _confirm_maintenance() -> void:
	var plane: Dictionary = fleet_rows.get(selected_aircraft, {})
	_refresh_ferry_card(plane)
	if plane.is_empty(): return
	var aircraft_id := selected_aircraft
	var consequence := "Wartung jetzt: ca. %s · %.0f h Dauer. Die Kosten werden bei Beginn festgelegt. Während der Wartung finden keine Abflüge statt." % [UI.money(plane["maintenance_cost_eur"]), plane["maintenance_hours"]]
	if plane["status"] == "in_flight": consequence += " Unterwegs: Die Wartung beginnt erst nach der Landung."
	decision_dialog = UI.dialog(self, "%s warten?" % plane["identifier"], consequence, "Abbrechen", "Wartung durchführen", func():
		selected_aircraft = aircraft_id
		_request_maintenance())
	decision_dialog.popup_centered()


func _request_maintenance() -> void:
	if selected_aircraft == 0:
		return
	var error: String = simulation.request_maintenance(selected_aircraft)
	maintenance_status.text = "Wartung beauftragt. Bitte speichern." if error.is_empty() else error
	maintenance_status.add_theme_color_override("font_color", UI.POSITIVE if error.is_empty() else UI.NEGATIVE)
	if error.is_empty():
		save_status.text = "Wartung beauftragt · bitte Spielstand speichern."
	_refresh_fleet()


func _cancel_maintenance() -> void:
	if selected_aircraft == 0:
		return
	var error: String = simulation.cancel_maintenance(selected_aircraft)
	maintenance_status.text = "Wartungsauftrag storniert. Bitte speichern." if error.is_empty() else error
	maintenance_status.add_theme_color_override("font_color", UI.POSITIVE if error.is_empty() else UI.NEGATIVE)
	if error.is_empty():
		save_status.text = "Wartungsauftrag storniert · bitte Spielstand speichern."
	_refresh_fleet()


func _build_founding() -> void:
	founding = MarginContainer.new()
	founding.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "top", "right", "bottom"]:
		founding.add_theme_constant_override("margin_" + side, 28)
	add_child(founding)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	founding.add_child(column)
	column.add_child(_label("AIRLINE MANAGER", ThemeTokens.SCREEN_TITLE))
	column.add_child(_label("DEINE EIGENE FLUGGESELLSCHAFT", ThemeTokens.SMALL, ThemeTokens.ACCENT_TEXT))
	var scroll := ScrollContainer.new()
	founding_scroll = scroll
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.follow_focus = true
	column.add_child(scroll)
	var form := VBoxContainer.new()
	form.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	form.add_theme_constant_override("separation", 10)
	# Reserve space for the focus ring inside the clipping scroll viewport.
	var focus_margin := MarginContainer.new()
	focus_margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for side in ["left", "top", "right", "bottom"]:
		focus_margin.add_theme_constant_override("margin_" + side, ThemeTokens.SPACE_1)
	scroll.add_child(focus_margin)
	var panel := UI.panel(focus_margin, "")
	panel.add_child(form)
	form.add_child(_label("Ein neuer Anfang.", ThemeTokens.SCREEN_TITLE))
	form.add_child(_label("Name deiner Airline", ThemeTokens.BODY))
	name_input = LineEdit.new()
	name_input.placeholder_text = "Airline-Name (1–80 Zeichen)"
	name_input.max_length = 80
	name_input.custom_minimum_size.y = 40
	form.add_child(name_input)
	form.add_child(_label("Heimatland", ThemeTokens.BODY))
	country_select = OptionButton.new()
	country_select.custom_minimum_size.y = 40
	country_select.get_popup().max_size = Vector2i(600, 280)
	var countries: Array = simulation.get_countries()
	countries.sort_custom(func(a: Dictionary, b: Dictionary): return tr(a["name"]).naturalnocasecmp_to(tr(b["name"])) < 0)
	for country in countries:
		country_select.add_item(tr(country["name"]))
		country_select.set_item_metadata(country_select.item_count - 1, country["code"])
	country_select.item_selected.connect(_country_changed)
	form.add_child(country_select)
	form.add_child(_label("Heimatflughafen", ThemeTokens.BODY))
	airport_select = OptionButton.new()
	airport_select.custom_minimum_size.y = 40
	airport_select.get_popup().max_size = Vector2i(600, 280)
	airport_select.fit_to_longest_item = false
	airport_select.clip_text = true
	airport_select.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	form.add_child(airport_select)
	difficulty_options = JSON.parse_string(simulation.get_difficulty_options())
	form.add_child(_label(tr("DIFFICULTY_PRESET"), ThemeTokens.BODY))
	difficulty_preset = _difficulty_segments(form, ["relaxed", "standard", "demanding"], "DIFFICULTY_", func(key: String): _apply_difficulty_preset(key))
	for key in ["capital", "competition", "crises"]:
		var title := _label(tr("DIFFICULTY_" + key.to_upper()), ThemeTokens.BODY)
		title.tooltip_text = tr("DIFFICULTY_TIP_" + key.to_upper())
		form.add_child(title)
		difficulty_choices[key] = _difficulty_segments(form, difficulty_options["levels"], "DIFFICULTY_LEVEL_", func(_level: String): _difficulty_edited())
		for button in difficulty_choices[key]: button.tooltip_text = title.tooltip_text
	founding_capital = _label("Startkapital: %s · vorläufig konfiguriert" % simulation.get_starting_capital(), ThemeTokens.BODY, ThemeTokens.ACCENT_TEXT)
	founding_capital.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	form.add_child(founding_capital)
	_apply_difficulty_preset("standard")
	var actions := HBoxContainer.new()
	actions.add_theme_constant_override("separation", 12)
	column.add_child(actions)
	found_button = Button.new()
	found_button.text = "Airline gründen"
	found_button.theme_type_variation = "PrimaryButton"
	found_button.pressed.connect(_found_airline)
	actions.add_child(found_button)
	founding_load_button = Button.new()
	founding_load_button.text = "Spielstand laden"
	founding_load_button.pressed.connect(_load_game)
	actions.add_child(founding_load_button)
	cancel_founding_button = Button.new()
	cancel_founding_button.text = "Abbrechen"
	cancel_founding_button.theme_type_variation = "GhostButton"
	cancel_founding_button.pressed.connect(_cancel_founding)
	actions.add_child(cancel_founding_button)
	founding_status = _label("Die neue Airline startet pausiert bei Tag 1 und lässt sich danach in einem benannten Speicherplatz sichern.", ThemeTokens.SMALL, ThemeTokens.INK_MUTED)
	founding_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(founding_status)
	for i in range(country_select.item_count):
		if country_select.get_item_metadata(i) == "DE": country_select.select(i)
	_country_changed(country_select.selected)


func _difficulty_segments(parent: Node, keys: Array, prefix: String, changed: Callable) -> Array[Button]:
	var row := UI.segment_tray(parent)
	var group := ButtonGroup.new()
	var buttons: Array[Button] = []
	for key in keys:
		var button := Button.new()
		button.text = tr(prefix + String(key).to_upper())
		button.theme_type_variation = "SegmentButton"
		button.toggle_mode = true
		button.button_group = group
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.set_meta("level", key)
		button.pressed.connect(func(): changed.call(key))
		row.add_child(button)
		buttons.append(button)
	return buttons

func _difficulty_level(key: String) -> String:
	for button in difficulty_choices[key]:
		if button.button_pressed: return button.get_meta("level")
	return "standard"

func _apply_difficulty_preset(key: String) -> void:
	var values: Array = difficulty_options["presets"][key]
	var fields := ["capital", "competition", "crises"]
	for i in range(fields.size()):
		for button in difficulty_choices[fields[i]]:
			button.set_pressed_no_signal(button.get_meta("level") == values[i])
	for button in difficulty_preset: button.set_pressed_no_signal(button.get_meta("level") == key)
	_refresh_founding_capital()

func _difficulty_edited() -> void:
	# A manually edited preset becomes custom; the three selections are authoritative.
	for button in difficulty_preset: button.set_pressed_no_signal(false)
	_refresh_founding_capital()

func _refresh_founding_capital() -> void:
	founding_capital.text = tr("DIFFICULTY_AMOUNT") % UI.money(difficulty_options["capital_eur"][_difficulty_level("capital")])


func _country_changed(index: int) -> void:
	airport_select.clear()
	if index < 0 or index >= country_select.item_count:
		found_button.disabled = true
		return
	var country_code: String = country_select.get_item_metadata(index)
	var airports: Array = simulation.get_airports(country_code)
	airports.sort_custom(func(a: Dictionary, b: Dictionary): return a["iata"].naturalnocasecmp_to(b["iata"]) < 0)
	for airport in airports:
		airport_select.add_item("%s · %s (%s)" % [airport["iata"], String(airport["name"]).left(44) + ("…" if String(airport["name"]).length() > 44 else ""), airport["icao"]])
		airport_select.set_item_metadata(airport_select.item_count - 1, airport["icao"])
		airport_select.set_item_tooltip(airport_select.item_count - 1, airport["name"])
	found_button.disabled = airport_select.item_count == 0
	if airport_select.item_count > 0:
		airport_select.select(0)


func _found_airline() -> void:
	if is_instance_valid(new_game_dialog) and not new_game_dialog.is_queued_for_deletion(): return
	if country_select.selected < 0 or airport_select.selected < 0:
		founding_status.text = "Bitte Heimatland und Heimatflughafen auswählen."
		return
	if not simulation.has_airline():
		_start_new_world()
		return
	var error: String = simulation.validate_new_world(name_input.text, country_select.get_selected_metadata(), airport_select.get_selected_metadata(), _difficulty_level("capital"), _difficulty_level("competition"), _difficulty_level("crises"))
	if not error.is_empty():
		founding_status.text = error
		return
	var current: Dictionary = simulation.get_airline_info()
	var body := tr("Aktuelle Airline: %s (Basis %s)") % [current.get("name", "—"), current.get("airport_icao", "—")]
	body += "\n" + tr("Neue Airline: %s (Basis %s)") % [name_input.text.strip_edges(), airport_select.get_selected_metadata()]
	body += "\n" + tr("Nicht gespeicherter Fortschritt in der aktuellen Airline geht verloren. Benannte Spielstände bleiben unter Laden erhalten.")
	new_game_dialog = UI.dialog(self, tr("Neue Airline gründen?"), body, tr("Abbrechen"), tr("Airline gründen"), _start_new_world)
	new_game_dialog.tree_exited.connect(func():
		if is_inside_tree() and founding.visible: found_button.grab_focus.call_deferred()
	)
	new_game_dialog.popup_centered()


func _start_new_world() -> void:
	var error: String = simulation.start_new_world(name_input.text, country_select.get_selected_metadata(), airport_select.get_selected_metadata(), _difficulty_level("capital"), _difficulty_level("competition"), _difficulty_level("crises"))
	if not error.is_empty():
		founding_status.text = error
		return
	save_status.text = "Airline gegründet · bitte Spielstand speichern."
	_sync_view()
	routes_panel.reset_for_world()


func _sync_view() -> void:
	if is_instance_valid(economy_toast): economy_toast.queue_free()
	milestones_panel.reset_view()
	jump_status.hide()
	jump_attention_target = {}
	_refresh_jump_attention()
	fleet_status.text = ""
	maintenance_status.text = ""
	fleet_status.hide()
	maintenance_status.hide()
	selected_aircraft = 0
	ferry_retained = {}
	routes_panel.status.text = ""
	overview.visible = simulation.has_airline()
	founding.visible = not simulation.has_airline()
	cancel_founding_button.visible = simulation.has_airline()
	var airline: Dictionary = simulation.get_airline_info()
	difficulty_caption.text = tr("DIFFICULTY_SAVED") % [tr("DIFFICULTY_LEVEL_" + String(airline.get("difficulty_capital", "standard")).to_upper()), tr("DIFFICULTY_LEVEL_" + String(airline.get("difficulty_competition", "standard")).to_upper()), tr("DIFFICULTY_LEVEL_" + String(airline.get("difficulty_crises", "standard")).to_upper())]
	airline_name.text = airline.get("name", "—")
	airline_base.text = "— · Basis %s" % airline.get("airport_icao", "—")
	_refresh_time()
	_show_page(0)
	_refresh_attention()
	if overview.visible:
		navigation[0].grab_focus()
	else:
		name_input.grab_focus()


func _open_founding(opener: Button = null) -> void:
	founding_opener = opener
	overview.hide()
	founding.show()
	cancel_founding_button.show()
	name_input.clear()
	_apply_difficulty_preset("standard")
	founding_scroll.scroll_vertical = 0
	founding_status.text = "Neues Spiel: Gründen ersetzt die laufende Airline samt ungespeichertem Fortschritt. Bestehende manuelle Spielstände bleiben unter Laden erhalten. Abbrechen kehrt zur laufenden Airline zurück."
	name_input.grab_focus()


func _cancel_founding() -> void:
	if not simulation.has_airline():
		_sync_view()
		return
	founding.hide()
	overview.show()
	var target: Button = founding_opener if is_instance_valid(founding_opener) and founding_opener.is_visible_in_tree() else new_game_button
	founding_opener = null
	target.grab_focus()


func _label(text: String, style: Dictionary, color: Color = ThemeTokens.INK) -> Label:
	return UI.label(text, style, color)


func _build_finance(content: VBoxContainer) -> void:
	finance_panel = VBoxContainer.new()
	content.add_child(finance_panel)
	difficulty_caption = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	difficulty_caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	finance_panel.add_child(difficulty_caption)
	milestones_panel = MilestonesPanel.new()
	finance_panel.add_child(milestones_panel)
	milestones_panel.setup(simulation)
	metric_grid = GridContainer.new()
	metric_grid.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	metric_grid.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	finance_panel.add_child(metric_grid)
	get_viewport().size_changed.connect(func(): metric_grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2)
	metric_grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2
	for item in [["balance", "KONTOSTAND"], ["profit", "BETRIEBSERGEBNIS"], ["load", "AUSLASTUNG · GESAMT"], ["flights", "LANDUNGEN / PASSAGIERE"]]:
		metrics[item[0]] = UI.metric(metric_grid, item[1])
	economy_support = HBoxContainer.new()
	metrics["economy"] = UI.metric(metric_grid, tr("ECONOMY_TITLE"), economy_support)
	finance_panel.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	var events := UI.panel(finance_panel, tr("EVENTS_TITLE"))
	events_title = events.get_meta("panel_shell").find_children("*", "Label", true, false)[0]
	events_table = UI.table([tr("EVENTS_PLACE"), tr("EVENTS_KIND"), tr("EVENTS_DATES"), tr("EVENTS_EFFECT"), tr("EVENTS_STATUS")], [150, 70, 100, 170, 100], 132)
	events.add_child(events_table)
	events_empty = UI.label(tr("EVENTS_NONE"), ThemeTokens.BODY, UI.MUTED)
	events.add_child(events_empty)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	grid.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	finance_panel.add_child(grid)
	get_viewport().size_changed.connect(func(): grid.columns = 2 if get_viewport_rect().size.x >= 1100 else 1)
	grid.columns = 2 if get_viewport_rect().size.x >= 1100 else 1
	var register := UI.panel(grid, "FLOTTENREGISTER  /  EIGENTUM & LEASING")
	overview_register = UI.table(["Kennung", "Typ", "Standort", "Status"], [115, 150, 90, 130], 220)
	register.add_child(overview_register)
	overview_operations = UI.panel(grid, "LIVE-BETRIEB  /  FLOTTE & NETZWERK")
	var commitments := UI.panel(finance_panel, "ANSTEHENDE TERMINE  /  LIEFERUNGEN & UMSCHULUNGEN")
	commitments_table = UI.table(["Datum", "Art", "Flugzeug / Pilot", "Zielrating"], [140, 100, 130, 90], 232)
	commitments.add_child(commitments_table)
	commitments_empty = UI.empty_state("Keine anstehenden Termine", "Geplante Auslieferungen und Pilotenumschulungen erscheinen hier.", "Zu den Bestellungen", func():
		_show_page(1)
		fleet_tabs.current_tab = 1)
	commitments.add_child(commitments_empty)
	commitments_jump_button = Button.new()
	commitments_jump_button.focus_mode = Control.FOCUS_ALL
	commitments_jump_button.clip_text = true
	commitments_jump_button.pressed.connect(_jump_to_commitment)
	commitments.add_child(commitments_jump_button)
	var finance := UI.panel(finance_panel, "FINANZÜBERSICHT  /  KUMULIERT")
	finance_breakdown = UI.label("")
	finance.add_child(finance_breakdown)
	var operations := UI.panel(finance_panel, "LIVE-BETRIEB  /  FLOTTE & NETZWERK")
	operation_summary = UI.label("")
	operation_summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	operations.add_child(operation_summary)
	var note := UI.label("Abrechnung bei Landung · Käufe separat ausgewiesen.\nVereinfachte Wirtschaftswerte · Fortschritt manuell speichern.", ThemeTokens.SMALL, UI.MUTED)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	finance_panel.add_child(note)


func _build_attention(parent: VBoxContainer) -> void:
	var shell := UI.frame_panel(parent, 0)
	var body := VBoxContainer.new()
	body.add_theme_constant_override("separation", 0)
	shell.add_child(body)
	attention_button = Button.new()
	attention_button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	attention_button.pressed.connect(func():
		attention_expanded = not attention_expanded
		attention_scroll.visible = attention_expanded)
	attention_button.theme_type_variation = "AttentionHeader"
	body.add_child(attention_button)
	attention_scroll = ScrollContainer.new()
	attention_scroll.custom_minimum_size.y = 176
	attention_scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	attention_scroll.follow_focus = true
	body.add_child(attention_scroll)
	attention_rows = VBoxContainer.new()
	attention_rows.add_theme_constant_override("separation", 0)
	attention_rows.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	attention_scroll.add_child(attention_rows)


func _refresh_attention() -> void:
	var entries: Array = simulation.get_attention_snapshot()
	var key := str(entries)
	if key == attention_signature:
		return
	attention_signature = key
	_refresh_jump_attention()
	attention_button.text = "Aufmerksamkeit (%d) · zum Ein-/Ausklappen" % entries.size() if not entries.is_empty() else "Aufmerksamkeit · Keine aktuellen Hinweise"
	attention_button.disabled = entries.is_empty()
	attention_scroll.visible = attention_expanded and not entries.is_empty()
	for child in attention_rows.get_children():
		attention_rows.remove_child(child)
		child.queue_free()
	var counts := {}
	for entry in entries:
		var page: int = {"fleet": 1, "routes": 2, "finances": 4, "staff": 5}.get(entry["page"], 0)
		counts[page] = counts.get(page, 0) + 1
		var button := UI.attention_row(entry, PAGES[page]["title"])
		button.pressed.connect(_open_attention.bind(entry))
		attention_rows.add_child(button)
	attention_scroll.custom_minimum_size.y = mini(entries.size(), 4) * 44
	for index in range(navigation.size()):
		nav_badges[index].text = str(counts.get(index, 0))
		nav_badges[index].visible = counts.get(index, 0) > 0


func _resize_frame() -> void:
	var compact := size.x < 900
	# Re-theme only when crossing the breakpoint, not on each layout pass.
	if compact_navigation == int(compact): return
	compact_navigation = int(compact)
	rail.custom_minimum_size.x = 56 if compact else 208
	UI.rail_padding(rail, ThemeTokens.SPACE_1 if compact else ThemeTokens.SPACE_3)
	rail_brand.visible = not compact
	for index in range(navigation.size()):
		UI.nav_geometry(navigation[index], compact)
		navigation[index].tooltip_text = PAGES[index]["title"] if compact else ""
		navigation[index].text = "" if compact else PAGES[index]["title"]
		navigation[index].alignment = HORIZONTAL_ALIGNMENT_CENTER if compact else HORIZONTAL_ALIGNMENT_LEFT
	new_game_button.text = "+" if compact else "Neues Spiel"
	search_button.text = "?" if compact else tr("Suche")


var maintenance_staff_button: Button


func _open_attention(entry: Dictionary) -> void:
	var page: int = {"fleet": 1, "routes": 2, "finances": 4, "staff": 5}.get(entry["page"], 0)
	if page == 1:
		selected_aircraft = entry["aircraft_id"]
	_show_page(page)
	if page == 4 and entry.get("kind", "") in ["financial_distress", "restructuring"]:
		finances_panel.focus_lending()
		return
	if page == 1 and entry.get("kind", "") == "delivery":
		fleet_tabs.current_tab = 1
		orders_panel.select_order(entry.get("order_id", 0))
		var scroll := fleet_panel.get_parent().get_parent() as ScrollContainer
		scroll.call_deferred("ensure_control_visible", orders_panel.table)
		orders_panel.table.grab_focus()
		return
	if page == 1 and entry.get("kind", "") in ["forced_check", "technicians"]:
		fleet_tabs.current_tab = 0
		_select_fleet_aircraft(entry["aircraft_id"])
		var scroll := fleet_panel.get_parent().get_parent() as ScrollContainer
		fleet_tree.grab_focus()
		for frame in range(2):
			await get_tree().process_frame
		scroll.ensure_control_visible(maintenance_box)
		return
	if page == 1:
		fleet_tree.ensure_cursor_is_visible()
		fleet_tree.grab_focus()
	elif page == 2:
		routes_panel.focus_route(entry["route_id"])
	elif page == 5:
		if entry.get("kind", "") in ["strike", "strike_warning"]:
			staff_panel.focus_pay_group(entry["employee_id"] - 1)
		elif entry.get("kind", "") == "crew":
			staff_panel.focus_shortage(entry["aircraft_id"])
		else:
			staff_panel.focus_employee(entry["employee_id"])


func _select_fleet_aircraft(aircraft_id: int) -> void:
	var item := fleet_tree.get_root().get_first_child() if fleet_tree.get_root() != null else null
	while item != null:
		if item.get_metadata(0) == aircraft_id:
			item.select(0)
			fleet_tree.scroll_to_item(item)
			_refresh_maintenance()
			return
		item = item.get_next()


func _refresh_overview_register() -> void:
	overview_register.clear()
	var root_item := overview_register.create_item()
	for plane in simulation.get_fleet():
		var row := overview_register.create_item(root_item)
		UI.cell_text(row, 0, plane["identifier"])
		UI.cell_text(row, 1, plane["model_name"])
		UI.cell_text(row, 2, plane["location_icao"])
		var phase: String = plane["ferry_state"] if not plane["ferry_state"].is_empty() else {"parked": "Geparkt", "in_flight": "Unterwegs", "ground": "Gelandet · Bodenzeit", "maintenance": "In Wartung"}.get(plane["status"], "Unbekannt")
		UI.cell_status(row, 3, phase, "info" if plane["status"] in ["in_flight", "maintenance"] else "neutral")


func _refresh_commitments() -> void:
	var entries: Array = simulation.get_upcoming_commitments()
	commitments_table.clear()
	commitments_table.visible = not entries.is_empty()
	commitments_empty.visible = entries.is_empty()
	var root_item := commitments_table.create_item()
	for entry in entries:
		var row := commitments_table.create_item(root_item)
		var delivery: bool = entry["kind"] == "delivery"
		UI.cell_text(row, 0, entry["date"])
		UI.cell_status(row, 1, "Lieferung" if delivery else "Umschulung", "info" if delivery else "neutral")
		UI.cell_text(row, 2, entry["title"])
		UI.cell_text(row, 3, "—" if delivery else entry["detail"])
	_refresh_commitments_jump()


func _refresh_commitments_jump() -> void:
	commitments_jump_target = simulation.get_next_commitment_jump()
	var available := not commitments_jump_target.is_empty()
	available = available and not terminal_state
	commitments_jump_button.disabled = not available
	if available:
		commitments_jump_button.text = tr("Zum nächsten Termin springen · %s · %s") % [commitments_jump_target["date"], commitments_jump_target["title"]]
		commitments_jump_button.tooltip_text = ""
	else:
		commitments_jump_button.text = tr("Zum nächsten Termin springen")
		commitments_jump_button.tooltip_text = tr("Nicht verfügbar: Unternehmensführung beendet.") if terminal_state else tr("Nicht verfügbar: kein Termin oder der nächste Termin liegt jenseits des längsten Sprungs.")


func _render_operations(parent: VBoxContainer) -> void:
	if parent == null: return
	for child in parent.get_children():
		parent.remove_child(child)
		child.queue_free()
	var flights: Array = simulation.get_map_flights()
	for flight in flights:
		parent.add_child(UI.label(flight["identifier"], ThemeTokens.CODE))
		parent.add_child(UI.status_chip("Unterwegs", "info"))
		parent.add_child(UI.route_leg(flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]))
	for operation in simulation.get_operations():
		var airborne := false
		for flight in flights:
			if flight["aircraft_id"] == operation["aircraft_id"]: airborne = true
		if airborne: continue
		var text := UI.label("%s · %s\nNächster Abflug: %s" % [operation["identifier"], operation["status_text"], operation["next_departure"]])
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		parent.add_child(text)
