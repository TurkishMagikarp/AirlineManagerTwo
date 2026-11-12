extends Control
## Presentation only. No simulation state or economic calculations.

const RoutesPanel = preload("res://scripts/routes_panel.gd")
const UI = preload("res://scripts/ui/components.gd")

const PAGES: Array[Dictionary] = [
	{"title": "Übersicht", "heading": "Unternehmensübersicht", "body": ""},
	{"title": "Flotte", "heading": "Deine Flotte",
	"body": ""},
	{"title": "Routen", "heading": "Dein Routennetz",
	"body": "Verbindungen planen und Flugzeuge zuweisen."},
]

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
var save_button: Button
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
var founding_capital: Label
var new_game_button: Button
var fleet_panel: VBoxContainer
var model_select: OptionButton
var model_details: Label
var buy_button: Button
var fleet_status: Label
var empty_fleet: Label
var fleet_tree: Tree
var routes_panel: VBoxContainer
var last_operation_second: int = -1
var finance_panel: VBoxContainer
var metrics: Dictionary = {}
var finance_breakdown: Label
var operation_summary: Label
var metric_grid: GridContainer


func _ready() -> void:
	get_window().min_size = Vector2i(680, 520)
	var margin := MarginContainer.new()
	overview = margin
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "top", "right", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 20)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 14)
	margin.add_child(column)
	var brand := HBoxContainer.new()
	column.add_child(brand)
	var wordmark := _label("AM  /  AIRLINE MANAGER", 21)
	wordmark.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	brand.add_child(wordmark)
	brand.add_child(_label("OPERATIONS CENTER", 12, UI.ACCENT))
	var nav := HBoxContainer.new()
	nav.add_theme_constant_override("separation", 12)
	column.add_child(nav)
	for index in range(PAGES.size()):
		var button := Button.new()
		button.text = PAGES[index]["title"]
		button.icon = load("res://resources/icons/%s.svg" % ["overview", "fleet", "routes"][index])
		button.custom_minimum_size = Vector2(120, 42)
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		button.toggle_mode = true
		button.pressed.connect(_show_page.bind(index))
		nav.add_child(button)
		navigation.append(button)
	new_game_button = Button.new()
	new_game_button.text = "Neues Spiel"
	new_game_button.pressed.connect(_open_founding)
	nav.add_child(new_game_button)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.follow_focus = true
	column.add_child(scroll)
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", 12)
	scroll.add_child(content)
	page_title = _label("", 16, Color("90d6dc"))
	heading = _label("", 30)
	description = _label("", 18)
	for label in [page_title, heading, description]:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		content.add_child(label)
	_build_finance(content)
	_build_fleet(content)
	routes_panel = RoutesPanel.new()
	routes_panel.simulation = simulation
	routes_panel.changed.connect(func(): save_status.text = "Routen geändert · bitte Spielstand speichern.")
	content.add_child(routes_panel)
	_add_time_controls(column)
	_build_founding()
	_sync_view()


func _add_time_controls(column: VBoxContainer) -> void:
	var panel := UI.panel(column, "")
	var layout := GridContainer.new()
	layout.add_theme_constant_override("h_separation", 24)
	panel.add_child(layout)
	layout.resized.connect(func(): layout.columns = 2 if layout.size.x >= 980 else 1)
	var clock_row := HBoxContainer.new()
	clock_row.add_theme_constant_override("separation", 24)
	layout.add_child(clock_row)
	time_label = _label("", 21, UI.ACCENT)
	clock_row.add_child(time_label)
	state_label = _label("", 16)
	state_label.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	clock_row.add_child(state_label)
	var controls := HBoxContainer.new()
	controls.add_theme_constant_override("separation", 8)
	layout.add_child(controls)
	start_button = Button.new()
	start_button.text = "Start"
	start_button.theme_type_variation = "PrimaryButton"
	start_button.pressed.connect(func():
		simulation.start()
		_refresh_time()
	)
	controls.add_child(start_button)
	pause_button = Button.new()
	pause_button.text = "Pause"
	pause_button.pressed.connect(func():
		simulation.pause()
		_refresh_time()
	)
	controls.add_child(pause_button)
	for speed in [1, 4, 16]:
		var button := Button.new()
		button.text = "%d×" % speed
		button.toggle_mode = true
		button.pressed.connect(_set_speed.bind(speed))
		controls.add_child(button)
		speed_buttons.append(button)
	save_button = Button.new()
	save_button.text = "Speichern"
	save_button.pressed.connect(_save_game)
	controls.add_child(save_button)
	load_button = Button.new()
	load_button.text = "Laden"
	load_button.pressed.connect(_load_game)
	controls.add_child(load_button)
	save_status = _label("Ein lokaler Spielstand · manuell speichern und laden", 14, Color("b0becf"))
	save_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	save_status.tooltip_text = save_path
	panel.add_child(save_status)
	_refresh_time()


func _save_game() -> void:
	var error: String = simulation.save_game(save_path)
	save_status.text = "Spielstand gespeichert." if error.is_empty() else error
	save_status.tooltip_text = save_path


func _load_game() -> void:
	var error: String = simulation.load_game(save_path)
	save_status.text = "Spielstand geladen." if error.is_empty() else error
	founding_status.text = save_status.text
	save_status.tooltip_text = save_path
	if error.is_empty():
		_sync_view()
		if not simulation.has_airline():
			founding_status.text = "Alter Zeit-Spielstand geladen. Noch keine Airline vorhanden; Gründen beginnt ein neues Spiel."
	_refresh_time()


func _set_speed(speed: int) -> void:
	if not simulation.set_speed(speed):
		push_error("Rust hat die Zeitgeschwindigkeit abgelehnt.")
	_refresh_time()


func _process(delta: float) -> void:
	if founding.visible:
		return
	if not simulation.advance(delta):
		simulation.pause()
		push_error("Rust konnte die Spielzeit nicht fortschreiben; Simulation pausiert.")
	_refresh_time()
	var second := int(simulation.get_elapsed_seconds())
	if second != last_operation_second:
		last_operation_second = second
		routes_panel.refresh_operations()
		if current_page == 0 and simulation.has_airline():
			_refresh_finance()
		if current_page == 1:
			_refresh_fleet()


func _refresh_time() -> void:
	time_label.text = simulation.get_display_time()
	state_label.text = "%s · %d×" % ["Läuft" if simulation.is_running() else "Pausiert", simulation.get_speed()]
	state_label.add_theme_color_override("font_color", UI.POSITIVE if simulation.is_running() else UI.MUTED)
	start_button.disabled = simulation.is_running()
	pause_button.disabled = not simulation.is_running()
	for button in speed_buttons:
		button.set_pressed_no_signal(button.text == "%d×" % simulation.get_speed())


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
	if index == 2:
		routes_panel.refresh()
	if index == 0 and simulation.has_airline():
		_refresh_finance()
	if index == 1:
		_refresh_fleet()
	for button_index in range(navigation.size()):
		navigation[button_index].set_pressed_no_signal(button_index == index)


func _refresh_finance() -> void:
	var airline: Dictionary = simulation.get_airline_info()
	var finance: Dictionary = simulation.get_financial_summary()
	heading.text = airline["name"]
	description.text = "%s  /  %s · %s" % [airline["airport_icao"], airline["airport_name"], airline["country_name"]]
	metrics["balance"].text = UI.money(airline["capital_eur"])
	metrics["balance"].add_theme_color_override("font_color", UI.NEGATIVE if airline["capital_eur"] < 0 else Color("e2eaf4"))
	metrics["profit"].text = UI.money(finance["profit_eur"])
	metrics["profit"].add_theme_color_override("font_color", UI.POSITIVE if finance["profit_eur"] >= 0 else UI.NEGATIVE)
	metrics["load"].text = "%.1f %%" % finance["load_percent"]
	metrics["flights"].text = "%d  /  %d Pax" % [finance["flights"], finance["passengers"]]
	finance_breakdown.text = "Ticketeinnahmen     %s\nBetriebskosten          %s\n\nTreibstoff                    %s\nFlughäfen                   %s\nPersonal                      %s\nWartung / Verwaltung  %s\n\nFlugzeugkäufe             %s" % [UI.money(finance["revenue_eur"]), UI.money(finance["costs_eur"]), UI.money(finance["fuel_eur"]), UI.money(finance["airport_eur"]), UI.money(finance["crew_eur"]), UI.money(finance["overhead_eur"]), UI.money(finance["purchases_eur"])]
	var operations: Array = simulation.get_operations()
	operation_summary.text = "%d Flugzeuge  ·  %d Routen  ·  %d aktive Flugpläne" % [simulation.get_fleet().size(), simulation.get_routes().size(), operations.size()]
	for operation in operations:
		operation_summary.text += "\n\n%s  /  Route #%d\n%s\nNächster Abflug: %s" % [operation["identifier"], operation["route_id"], operation["status_text"], operation["next_departure"]]


func _build_fleet(content: VBoxContainer) -> void:
	fleet_panel = VBoxContainer.new()
	fleet_panel.add_theme_constant_override("separation", 12)
	content.add_child(fleet_panel)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 24)
	grid.add_theme_constant_override("v_separation", 18)
	fleet_panel.add_child(grid)
	grid.resized.connect(func(): grid.columns = 2 if grid.size.x >= 1250 else 1)
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
	buy_button = Button.new()
	buy_button.text = "Flugzeug kaufen"
	buy_button.theme_type_variation = "PrimaryButton"
	buy_button.tooltip_text = "Kauft das gewählte Modell sofort zum angezeigten Preis."
	buy_button.pressed.connect(_buy_aircraft)
	purchase.add_child(buy_button)
	model_details = _label("", 18, Color("90d6dc"))
	model_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop.add_child(model_details)
	fleet_status = _label("", 16)
	fleet_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop.add_child(fleet_status)
	var hint := _label("Vorläufige Kaufpreise · Sitzkonfiguration und Reichweite sind vereinfachte Modellangaben.", 14, Color("b0becf"))
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop.add_child(hint)
	var owned := UI.panel(grid, "FLOTTENREGISTER  /  EIGENE FLUGZEUGE")
	empty_fleet = _label("Noch keine Flugzeuge gekauft.", 17, Color("c2cedc"))
	owned.add_child(empty_fleet)
	fleet_tree = Tree.new()
	fleet_tree.columns = 4
	fleet_tree.hide_root = true
	fleet_tree.column_titles_visible = true
	fleet_tree.custom_minimum_size.y = 220
	for index in range(4):
		fleet_tree.set_column_title(index, ["Kennung", "Typ", "Standort", "Status"][index])
		fleet_tree.set_column_expand(index, index == 1)
		fleet_tree.set_column_custom_minimum_width(index, [115, 145, 90, 210][index])
	owned.add_child(fleet_tree)
	# The operational register takes precedence over procurement.
	grid.move_child(owned.get_parent(), 0)
	_model_selected(model_select.selected)


func _model_selected(index: int) -> void:
	buy_button.disabled = index < 0
	if index < 0:
		model_details.text = "Kein Flugzeugmodell verfügbar. Bitte Katalog und Preise prüfen."
		return
	var model: Dictionary = model_select.get_item_metadata(index)
	model_details.text = "%s\n%d Sitzplätze   /   %d km Referenzreichweite" % [UI.money(model["price_eur"]), model["seats"], model["range_km"]]


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


func _refresh_fleet() -> void:
	var fleet: Array = simulation.get_fleet()
	var airline: Dictionary = simulation.get_airline_info()
	if current_page == 1:
		description.text = "%d Flugzeuge  /  Verfügbares Guthaben: %s" % [fleet.size(), UI.money(airline.get("capital_eur", 0))]
	empty_fleet.visible = fleet.is_empty()
	fleet_tree.visible = not fleet.is_empty()
	fleet_tree.clear()
	var root_item := fleet_tree.create_item()
	for plane in fleet:
		var row := fleet_tree.create_item(root_item)
		row.set_text(0, plane["identifier"])
		row.set_text(1, plane["model_name"])
		row.set_text(2, plane["location_icao"])
		var phase: String = {"parked": "Geparkt", "in_flight": "Unterwegs", "ground": "Gelandet · Bodenzeit"}.get(plane["status"], "Unbekannt")
		row.set_text(3, "%s · Route #%d" % [phase, plane["route_id"]] if plane["route_id"] != 0 else phase)
		row.set_custom_color(3, UI.ACCENT if plane["status"] == "in_flight" else UI.POSITIVE if plane["status"] == "parked" else Color("e4c287"))
		if plane["status"] == "in_flight":
			row.set_text(2, "Ab %s (in der Luft)" % plane["location_icao"])
		row.set_tooltip_text(0, plane["identifier"])
		row.set_tooltip_text(1, "%s · gezahlter Kaufpreis: %d EUR" % [plane["model_name"], plane["purchase_price_eur"]])
		row.set_tooltip_text(2, plane["location_name"])
		row.set_metadata(0, plane["id"])


func _build_founding() -> void:
	founding = MarginContainer.new()
	founding.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "top", "right", "bottom"]:
		founding.add_theme_constant_override("margin_" + side, 28)
	add_child(founding)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 12)
	founding.add_child(column)
	column.add_child(_label("AIRLINE MANAGER", 22))
	column.add_child(_label("DEINE EIGENE FLUGGESELLSCHAFT", 14, Color("90d6dc")))
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.follow_focus = true
	column.add_child(scroll)
	var form := VBoxContainer.new()
	form.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	form.add_theme_constant_override("separation", 10)
	scroll.add_child(form)
	form.add_child(_label("Ein neuer Anfang.", 28))
	form.add_child(_label("Name deiner Airline", 16))
	name_input = LineEdit.new()
	name_input.placeholder_text = "Airline-Name (1–80 Zeichen)"
	name_input.max_length = 80
	name_input.custom_minimum_size.y = 40
	form.add_child(name_input)
	form.add_child(_label("Heimatland", 16))
	country_select = OptionButton.new()
	country_select.custom_minimum_size.y = 40
	for country in simulation.get_countries():
		country_select.add_item(country["name"])
		country_select.set_item_metadata(country_select.item_count - 1, country["code"])
	country_select.item_selected.connect(_country_changed)
	form.add_child(country_select)
	form.add_child(_label("Heimatflughafen", 16))
	airport_select = OptionButton.new()
	airport_select.custom_minimum_size.y = 40
	airport_select.fit_to_longest_item = false
	airport_select.clip_text = true
	airport_select.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	form.add_child(airport_select)
	founding_capital = _label("Startkapital: %s · vorläufig konfiguriert" % simulation.get_starting_capital(), 16, Color("90d6dc"))
	founding_capital.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	form.add_child(founding_capital)
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
	cancel_founding_button.pressed.connect(_sync_view)
	actions.add_child(cancel_founding_button)
	founding_status = _label("Gründen startet pausiert bei Tag 1. Erst Speichern ersetzt den lokalen Spielstand.", 14, Color("b0becf"))
	founding_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(founding_status)
	_country_changed(0)


func _country_changed(index: int) -> void:
	airport_select.clear()
	if index < 0 or index >= country_select.item_count:
		found_button.disabled = true
		return
	var country_code: String = country_select.get_item_metadata(index)
	for airport in simulation.get_airports(country_code):
		airport_select.add_item("%s · %s (%s)" % [airport["iata"], airport["name"], airport["icao"]])
		airport_select.set_item_metadata(airport_select.item_count - 1, airport["icao"])
	found_button.disabled = airport_select.item_count == 0
	if airport_select.item_count > 0:
		airport_select.select(0)


func _found_airline() -> void:
	if country_select.selected < 0 or airport_select.selected < 0:
		founding_status.text = "Bitte Heimatland und Heimatflughafen auswählen."
		return
	var error: String = simulation.found_airline(name_input.text, country_select.get_selected_metadata(), airport_select.get_selected_metadata())
	if not error.is_empty():
		founding_status.text = error
		return
	save_status.text = "Airline gegründet · bitte Spielstand speichern."
	_sync_view()


func _sync_view() -> void:
	fleet_status.text = ""
	routes_panel.status.text = ""
	overview.visible = simulation.has_airline()
	founding.visible = not simulation.has_airline()
	cancel_founding_button.visible = simulation.has_airline()
	_refresh_time()
	_show_page(0)
	if overview.visible:
		navigation[0].grab_focus()
	else:
		name_input.grab_focus()


func _open_founding() -> void:
	overview.hide()
	founding.show()
	cancel_founding_button.show()
	name_input.clear()
	founding_status.text = "Neues Spiel: Gründen ersetzt den laufenden Zustand. Ungespeicherter Fortschritt geht verloren. Abbrechen kehrt zurück."
	name_input.grab_focus()


func _label(text: String, size: int, color: Color = Color("f0f4fa")) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	return label


func _build_finance(content: VBoxContainer) -> void:
	finance_panel = VBoxContainer.new()
	content.add_child(finance_panel)
	metric_grid = GridContainer.new()
	metric_grid.add_theme_constant_override("h_separation", 12)
	metric_grid.add_theme_constant_override("v_separation", 12)
	finance_panel.add_child(metric_grid)
	metric_grid.resized.connect(func(): metric_grid.columns = 4 if metric_grid.size.x >= 1150 else 2)
	for item in [["balance", "KONTOSTAND"], ["profit", "BETRIEBSERGEBNIS"], ["load", "AUSLASTUNG · GESAMT"], ["flights", "LANDUNGEN / PASSAGIERE"]]:
		metrics[item[0]] = UI.metric(metric_grid, item[1])
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	finance_panel.add_child(grid)
	grid.resized.connect(func(): grid.columns = 2 if grid.size.x >= 900 else 1)
	var finance := UI.panel(grid, "FINANZÜBERSICHT  /  KUMULIERT")
	finance_breakdown = UI.label("")
	finance.add_child(finance_breakdown)
	var operations := UI.panel(grid, "LIVE-BETRIEB  /  FLOTTE & NETZWERK")
	operation_summary = UI.label("")
	operation_summary.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	operations.add_child(operation_summary)
	var note := UI.label("Abrechnung bei Landung · Käufe separat ausgewiesen.\nVereinfachte Wirtschaftswerte · Fortschritt manuell speichern.", 13, UI.MUTED)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	finance_panel.add_child(note)
