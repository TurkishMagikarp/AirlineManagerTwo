extends Control
## Presentation only. No simulation state or economic calculations.

const PAGES: Array[Dictionary] = [
	{"title": "Übersicht", "heading": "Ein klarer Anfang.",
	"body": "Die Spielzeit kommt direkt aus dem Rust-Simulationskern.\nStart, Pause und Zeitraffer sind unten verfügbar.",
	"detail": "Die Verbindung zur Simulation steht.\n\nNoch keine Finanzzahlen, Flugbewegungen oder Unternehmensdaten. Spielregeln und Berechnungen bleiben ausschließlich in Rust."},
	{"title": "Flotte", "heading": "Flotte noch nicht verfügbar",
	"body": "Es liegt kein Flottenzustand aus einer Simulation vor.",
	"detail": "Hier werden später Flugzeuge und ihr Betriebszustand dargestellt.\n\nDiese Ansicht zeigt bewusst keine Beispiel-Flugzeuge und bietet noch keinen Kauf oder Leasing an."},
	{"title": "Routen", "heading": "Routennetz noch nicht verfügbar",
	"body": "Es liegt kein Routenzustand aus einer Simulation vor.",
	"detail": "Hier wird später die Planung des Streckennetzes zugänglich.\n\nEs werden noch keine Verbindungen, Flugpläne oder Nachfragewerte berechnet."},
]

var navigation: Array[Button] = []
var page_title: Label
var heading: Label
var description: Label
var detail: Label
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


func _ready() -> void:
	get_window().min_size = Vector2i(680, 520)
	theme = _make_theme()
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "top", "right", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 28)
	add_child(margin)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 22)
	margin.add_child(column)
	column.add_child(_label("AIRLINE MANAGER", 22))
	column.add_child(_label("OFFLINE  /  RUST-SIMULATION · SPIELZEIT", 14, Color("a8c7d5")))
	var nav := HBoxContainer.new()
	nav.add_theme_constant_override("separation", 12)
	column.add_child(nav)
	for index in range(PAGES.size()):
		var button := Button.new()
		button.text = PAGES[index]["title"]
		button.custom_minimum_size = Vector2(150, 48)
		button.toggle_mode = true
		button.pressed.connect(_show_page.bind(index))
		nav.add_child(button)
		navigation.append(button)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	column.add_child(scroll)
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", 18)
	scroll.add_child(content)
	page_title = _label("", 16, Color("90d6dc"))
	heading = _label("", 30)
	description = _label("", 18)
	detail = _label("", 17, Color("c2cedc"))
	for label in [page_title, heading, description, detail]:
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		content.add_child(label)
	_add_time_controls(column)
	column.add_child(_label("Navigation: Tab / Umschalt+Tab · Öffnen: Enter / Leertaste", 14, Color("b0becf")))
	_show_page(0)
	navigation[0].grab_focus()


func _add_time_controls(column: VBoxContainer) -> void:
	time_label = _label("", 24, Color("90d6dc"))
	column.add_child(time_label)
	state_label = _label("", 16)
	column.add_child(state_label)
	var controls := HBoxContainer.new()
	controls.add_theme_constant_override("separation", 8)
	column.add_child(controls)
	start_button = Button.new()
	start_button.text = "Start"
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
	column.add_child(save_status)
	_refresh_time()


func _save_game() -> void:
	var error: String = simulation.save_game(save_path)
	save_status.text = "Spielstand gespeichert." if error.is_empty() else error
	save_status.tooltip_text = save_path


func _load_game() -> void:
	var error: String = simulation.load_game(save_path)
	save_status.text = "Spielstand geladen." if error.is_empty() else error
	save_status.tooltip_text = save_path
	_refresh_time()


func _set_speed(speed: int) -> void:
	if not simulation.set_speed(speed):
		push_error("Rust hat die Zeitgeschwindigkeit abgelehnt.")
	_refresh_time()


func _process(delta: float) -> void:
	if not simulation.advance(delta):
		simulation.pause()
		push_error("Rust konnte die Spielzeit nicht fortschreiben; Simulation pausiert.")
	_refresh_time()


func _refresh_time() -> void:
	time_label.text = simulation.get_display_time()
	state_label.text = "%s · %d×" % ["Läuft" if simulation.is_running() else "Pausiert", simulation.get_speed()]
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
	heading.text = page["heading"]
	description.text = page["body"]
	detail.text = page["detail"]
	for button_index in range(navigation.size()):
		navigation[button_index].set_pressed_no_signal(button_index == index)


func _label(text: String, size: int, color: Color = Color("f0f4fa")) -> Label:
	var label := Label.new()
	label.text = text
	label.add_theme_font_size_override("font_size", size)
	label.add_theme_color_override("font_color", color)
	return label


func _make_theme() -> Theme:
	var result := Theme.new()
	result.default_font_size = 18
	for state in ["normal", "hover", "pressed", "focus"]:
		var style := StyleBoxFlat.new()
		style.bg_color = Color("18283b")
		style.set_corner_radius_all(8)
		style.set_content_margin_all(12)
		if state == "hover":
			style.bg_color = Color("29455c")
		elif state == "pressed":
			style.bg_color = Color("235866")
		elif state == "focus":
			style.bg_color = Color.TRANSPARENT
			style.border_color = Color("b5f3f4")
			style.set_border_width_all(3)
		result.set_stylebox(state, "Button", style)
	for state in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color"]:
		result.set_color(state, "Button", Color("f0f4fa"))
	return result
