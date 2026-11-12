extends Control
## Presentation only. No simulation state or economic calculations.

const PAGES: Array[Dictionary] = [
	{"title": "Übersicht", "heading": "Ein klarer Anfang.",
	"body": "Diese native Oberfläche ist die Grundlage für Airline Manager.\nDie Navigation funktioniert bereits; eine Spielwelt ist noch nicht verbunden.",
	"detail": "UI-Basis ohne Simulation\n\nKeine Finanzzahlen, Flugbewegungen oder Unternehmensdaten werden erzeugt. Spätere Spielzustände liefert ausschließlich die Rust-Simulation."},
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
	column.add_child(_label("OFFLINE  /  UI-BASIS · KEINE SIMULATION", 14, Color("a8c7d5")))
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
	column.add_child(_label("Navigation: Tab / Umschalt+Tab · Öffnen: Enter / Leertaste", 14, Color("b0becf")))
	_show_page(0)
	navigation[0].grab_focus()


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
