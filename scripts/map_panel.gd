extends VBoxContainer
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Read-only view: all flight coordinates and progress come from Rust.

const MapCanvas = preload("res://scripts/map_canvas.gd")
const UI = preload("res://scripts/ui/components.gd")
var simulation
var canvas: Control
var details: Label
var summary: Label
var selected_id: int = 0
var last_flights: Array = []
var last_selected_id := -1

func _ready() -> void:
	var toolbar := HBoxContainer.new()
	add_child(toolbar)
	summary = UI.label("", ThemeTokens.SMALL)
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
	details = UI.label("", ThemeTokens.BODY)
	details.custom_minimum_size.y = 52
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(details)
	var hint := UI.label("Ziehen: verschieben · Mausrad: zoomen · Klick: Details · Cyan: aktiv / Grau: geplant · Natural Earth (Public Domain)", ThemeTokens.CODE, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(hint)
	refresh()

func refresh() -> void:
	if not is_instance_valid(canvas): return
	canvas.airports = simulation.get_map_airports()
	canvas.routes = simulation.get_map_routes()
	last_selected_id = -2147483648 # Route/airport snapshots may change without a flight change.
	refresh_flights()

func refresh_flights() -> void:
	if not is_instance_valid(canvas): return
	canvas.flights = simulation.get_map_flights()
	canvas.selected_id = selected_id
	if canvas.flights == last_flights and selected_id == last_selected_id: return
	last_flights = canvas.flights
	last_selected_id = selected_id
	summary.text = "%d unterwegs  /  %d Routen" % [canvas.flights.size(), canvas.routes.size()]
	details.text = "Kein Flug ausgewählt · Nur tatsächlich fliegende Flugzeuge werden angezeigt."
	if selected_id < 0 and -selected_id - 1 < canvas.airports.size():
		var airport: Dictionary = canvas.airports[-selected_id - 1]
		details.text = "%s · %s\n%s" % [airport["icao"], airport["name"], airport["slots"]]
	for flight in canvas.flights:
		if flight["aircraft_id"] == selected_id:
			details.text = "%s  /  %s · %s\n%s → %s   ·   %.1f %%   ·   Ankunft: %s" % [flight["flight_number"], flight["model"], flight["identifier"], flight["origin"], flight["destination"], flight["progress"] * 100, flight["arrival"]]
	canvas.queue_redraw()
