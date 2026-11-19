extends Control
## Global search presentation. Reads existing airport and route snapshots; changes no game state.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
const MAX_RESULTS := 60
const FOLD := {"ä": "a", "ö": "o", "ü": "u", "ß": "ss", "é": "e", "è": "e", "ê": "e", "á": "a", "à": "a", "â": "a", "í": "i", "î": "i", "ó": "o", "ô": "o", "ú": "u", "û": "u", "ç": "c", "ğ": "g", "ı": "i", "ş": "s", "ñ": "n", "ø": "o", "å": "a", "–": "-", "—": "-"}

signal activated(result: Dictionary)
signal dismissed

var simulation: RefCounted
var airports: Array = []
var routes: Array = []
var results: Array = []
var query: LineEdit
var list: ItemList
var status: Label
var close_button: Button
var focus_before: Control


static func fold(text: String) -> String:
	var lowered := text.strip_edges().to_lower()
	for character in FOLD:
		lowered = lowered.replace(character, FOLD[character])
	return lowered


## Lower is better: 0 exact code, 1 code prefix, 2 name word start, 3 name contains, -1 no match.
static func _rank(needle: String, codes: Array, names: Array) -> int:
	var best := -1
	for code in codes:
		var folded := fold(code)
		if folded.is_empty(): continue
		var rank := 0 if folded == needle else 1 if folded.begins_with(needle) else -1
		if rank >= 0 and (best < 0 or rank < best): best = rank
	for text in names:
		var folded := fold(text)
		var rank := -1
		if folded.begins_with(needle) or (" " + folded).contains(" " + needle): rank = 2
		elif folded.contains(needle): rank = 3
		if rank >= 0 and (best < 0 or rank < best): best = rank
	return best


## Airport and route snapshots in, ranked labelled results out.
static func build_results(text: String, airport_rows: Array, route_rows: Array) -> Array:
	var needle := fold(text)
	if needle.is_empty(): return []
	var by_icao := {}
	for airport in airport_rows: by_icao[airport["icao"]] = airport
	var found: Array = []
	for airport in airport_rows:
		var iata: String = airport.get("iata", "")
		var rank := _rank(needle, [airport["icao"], iata], [airport["name"]])
		if rank < 0: continue
		var codes: String = airport["icao"] if iata.is_empty() else "%s · %s" % [iata, airport["icao"]]
		found.append({"kind": "airport", "icao": airport["icao"], "rank": rank, "sort": fold(airport["name"]), "id": 0,
			"title": TranslationServer.translate("Flughafen") + " · " + codes, "detail": airport["name"]})
	for route in route_rows:
		var ends: Array = []
		var codes: Array = []
		var names: Array = []
		for key in ["origin_icao", "destination_icao"]:
			var airport: Dictionary = by_icao.get(route[key], {"icao": route[key], "iata": "", "name": route[key]})
			ends.append(airport)
			codes.append_array([airport["icao"], airport.get("iata", "")])
			names.append(airport["name"])
		var pair: String = "%s-%s" % [route["origin_icao"], route["destination_icao"]]
		var iata_pair: String = "%s-%s" % [ends[0].get("iata", ""), ends[1].get("iata", "")]
		names.append_array([pair, iata_pair])
		var rank := _rank(needle, codes + ["#%d" % route["id"]], names)
		if rank < 0: continue
		found.append({"kind": "route", "icao": "", "rank": rank, "sort": pair, "id": route["id"],
			"title": "%s · #%d · %s" % [TranslationServer.translate("Route"), route["id"], pair],
			"detail": "%s → %s" % [ends[0]["name"], ends[1]["name"]]})
	found.sort_custom(func(a, b):
		if a["rank"] != b["rank"]: return a["rank"] < b["rank"]
		if a["kind"] != b["kind"]: return a["kind"] == "airport"
		if a["sort"] != b["sort"]: return a["sort"] < b["sort"]
		return a["id"] < b["id"])
	return found.slice(0, MAX_RESULTS)


func open(sim: RefCounted) -> void:
	simulation = sim
	focus_before = get_viewport().gui_get_focus_owner()
	theme = Tokens.new()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var scrim := ColorRect.new()
	scrim.color = Tokens.SCRIM
	add_child(scrim)
	scrim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	scrim.gui_input.connect(func(event: InputEvent):
		if event is InputEventMouseButton and event.pressed: dismiss())
	var center := CenterContainer.new()
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(center)
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var card := PanelContainer.new()
	card.add_theme_stylebox_override("panel", Tokens.new().floating_box(Tokens.RADIUS_LG, Tokens.SPACE_4))
	card.custom_minimum_size.x = clampf(get_viewport_rect().size.x - 32, 280, 560)
	center.add_child(card)
	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", Tokens.SPACE_3)
	card.add_child(content)
	content.add_child(UI.label(tr("Suche"), UI.DIALOG_TITLE))
	query = LineEdit.new()
	query.placeholder_text = tr("Flughafen, Ort, Code oder Route")
	query.clear_button_enabled = true
	query.accessibility_name = tr("Suche")
	query.text_changed.connect(func(_text: String): _refresh_results())
	query.text_submitted.connect(func(_text: String): _activate(_selected()))
	query.gui_input.connect(_query_input)
	content.add_child(query)
	list = ItemList.new()
	list.focus_mode = Control.FOCUS_NONE
	list.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	list.auto_height = false
	list.custom_minimum_size.y = clampf(get_viewport_rect().size.y - 280, 96, 320)
	list.item_clicked.connect(func(index: int, _at: Vector2, button: int):
		if button == MOUSE_BUTTON_LEFT: _activate(index))
	list.item_activated.connect(_activate)
	content.add_child(list)
	status = UI.label("", Tokens.SMALL, Tokens.INK_MUTED)
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(status)
	var actions := HBoxContainer.new()
	actions.alignment = BoxContainer.ALIGNMENT_END
	content.add_child(actions)
	close_button = Button.new()
	close_button.text = tr("Schließen")
	close_button.theme_type_variation = "GhostButton"
	close_button.pressed.connect(dismiss)
	close_button.gui_input.connect(_close_input)
	actions.add_child(close_button)
	query.focus_next = query.get_path_to(close_button)
	query.focus_previous = query.get_path_to(close_button)
	close_button.focus_next = close_button.get_path_to(query)
	close_button.focus_previous = close_button.get_path_to(query)
	refresh()
	query.grab_focus.call_deferred()


## Re-reads the existing snapshots, e.g. after a route change or save load.
func refresh() -> void:
	airports = simulation.get_map_airports()
	routes = simulation.get_routes()
	_refresh_results()


func _refresh_results() -> void:
	results = build_results(query.text, airports, routes)
	list.clear()
	for result in results:
		list.add_item("%s — %s" % [result["title"], result["detail"]])
		list.set_item_tooltip(list.item_count - 1, "%s\n%s" % [result["title"], result["detail"]])
	if query.text.strip_edges().is_empty():
		status.text = tr("Mit Pfeiltasten wählen, mit Enter öffnen, mit Esc schließen.")
	elif results.is_empty():
		status.text = tr("Keine Treffer für „%s“.") % query.text.strip_edges()
	else:
		status.text = tr("%d Treffer") % results.size()
	if not results.is_empty():
		list.select(0)


func _selected() -> int:
	var picked := list.get_selected_items() if is_instance_valid(list) else PackedInt32Array()
	return picked[0] if not picked.is_empty() else -1


func _move(step: int) -> void:
	if results.is_empty(): return
	var index := clampi(_selected() + step, 0, results.size() - 1)
	list.select(index)
	list.ensure_current_is_visible()


func _query_input(event: InputEvent) -> void:
	if not (event is InputEventKey) or not event.pressed: return
	match event.keycode:
		KEY_DOWN: _move(1)
		KEY_UP: _move(-1)
		KEY_PAGEDOWN: _move(5)
		KEY_PAGEUP: _move(-5)
		KEY_ESCAPE: dismiss()
		_: return
	query.accept_event()


func _close_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode in [KEY_UP, KEY_DOWN]:
		_move(1 if event.keycode == KEY_DOWN else -1)
		close_button.accept_event()


func _unhandled_key_input(event: InputEvent) -> void:
	if event.is_action_pressed("ui_cancel"):
		dismiss()
		get_viewport().set_input_as_handled()


func _activate(index: int) -> void:
	if index < 0 or index >= results.size(): return
	var result: Dictionary = results[index]
	queue_free()
	activated.emit(result)


func dismiss() -> void:
	if is_queued_for_deletion(): return
	if is_instance_valid(focus_before): focus_before.grab_focus()
	queue_free()
	dismissed.emit()
