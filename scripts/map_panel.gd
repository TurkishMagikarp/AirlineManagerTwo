extends VBoxContainer
## Read-only view: all flight coordinates and progress come from Rust.

const MapCanvas = preload("res://scripts/map_canvas.gd")
const UI = preload("res://scripts/ui/components.gd")
var simulation
var canvas: Control
var details: Label
var summary: Label
var selected_id: int = 0

func _ready() -> void:
	var toolbar := HBoxContainer.new()
	add_child(toolbar)
	summary = UI.label("", 14)
	summary.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	toolbar.add_child(summary)
	for caption in ["−", "+", "Welt"]:
		var button := Button.new()
		button.text = caption
		button.pressed.connect(func():
			if caption == "Welt": canvas.reset_view()
			else: canvas.zoom_at(canvas.size / 2, 1.3 if caption == "+" else 1.0 / 1.3))
		toolbar.add_child(button)
	canvas = MapCanvas.new()
	canvas.custom_minimum_size = Vector2(0, 250)
	canvas.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	canvas.selected.connect(func(id): selected_id = id; refresh())
	add_child(canvas)
	details = UI.label("", 16)
	details.custom_minimum_size.y = 52
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(details)
	var hint := UI.label("Ziehen: verschieben · Mausrad: zoomen · Klick: Details · Cyan: aktiv / Grau: geplant · Natural Earth (Public Domain)", 12, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(hint)
	refresh()

func refresh() -> void:
	if not is_instance_valid(canvas): return
	canvas.airports = simulation.get_map_airports()
	canvas.routes = simulation.get_map_routes()
	refresh_flights()

func refresh_flights() -> void:
	if not is_instance_valid(canvas): return
	canvas.flights = simulation.get_map_flights()
	canvas.selected_id = selected_id
	summary.text = "%d unterwegs  /  %d Routen" % [canvas.flights.size(), canvas.routes.size()]
	details.text = "Kein Flug ausgewählt · Nur tatsächlich fliegende Flugzeuge werden angezeigt."
	for flight in canvas.flights:
		if flight["aircraft_id"] == selected_id:
			details.text = "%s  /  %s · %s\n%s → %s   ·   %.1f %%   ·   Ankunft: %s" % [flight["flight_number"], flight["model"], flight["identifier"], flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]]
	canvas.queue_redraw()
