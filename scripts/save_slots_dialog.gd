extends Control
## Slot presentation only. Rust owns paths, validation, metadata and transactions.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
signal finished(message: String, loaded: bool)
var simulation: RefCounted
var user_dir: String
var saving := false
var selected := -1
var rows: Array = []
var name_field: LineEdit
var list_box: VBoxContainer
var status: Label
var save_action: Button
var content: VBoxContainer
var decision: Control
var failed_slots: Dictionary = {}
const AUTOSAVE_ID := 1000
var autosave: Dictionary = {}
var autosave_settings: RefCounted
var autosave_choice: OptionButton
var focus_before: Control

func open(sim: RefCounted, directory: String, save_mode: bool) -> void:
	simulation = sim
	user_dir = directory
	saving = save_mode
	focus_before = get_viewport().gui_get_focus_owner()
	theme = Tokens.new()
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var scrim := ColorRect.new()
	scrim.color = Tokens.SCRIM
	add_child(scrim)
	scrim.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var center := CenterContainer.new()
	add_child(center)
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var card := PanelContainer.new()
	card.add_theme_stylebox_override("panel", Tokens.new().floating_box(Tokens.RADIUS_LG, Tokens.SPACE_5))
	center.add_child(card)
	content = VBoxContainer.new()
	content.add_theme_constant_override("separation", Tokens.SPACE_3)
	card.add_child(content)
	content.add_child(UI.label(tr("Spielstand speichern") if saving else tr("Spielstand laden"), UI.DIALOG_TITLE))
	var hold := UI.label(tr("Die Spielzeit ist angehalten, solange dieses Fenster offen ist."), Tokens.SMALL, Tokens.INK_MUTED)
	hold.name = "TimeHoldNotice"
	hold.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(hold)
	if autosave_settings != null:
		autosave_choice = OptionButton.new()
		autosave_choice.tooltip_text = tr("Autosave in festen Echtzeit-Abständen in einen eigenen Eintrag. Standardmäßig aus.")
		autosave_choice.add_item(tr("Autosave aus"), 0)
		for minutes in autosave_settings.INTERVALS:
			autosave_choice.add_item(tr("Autosave alle %d Min.") % minutes, minutes)
		autosave_choice.select(autosave_choice.get_item_index(autosave_settings.interval_minutes if autosave_settings.enabled else 0))
		autosave_choice.item_selected.connect(func(_index: int):
			var minutes: int = autosave_choice.get_selected_id()
			autosave_settings.configure(minutes > 0, minutes if minutes > 0 else autosave_settings.interval_minutes))
		content.add_child(autosave_choice)
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	scroll.add_theme_stylebox_override("panel", Tokens.box(Tokens.TRANSPARENT, Tokens.TRANSPARENT, Tokens.RADIUS_MD, Tokens.SPACE_1, Tokens.SPACE_1))
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	content.add_child(scroll)
	list_box = VBoxContainer.new()
	list_box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list_box.add_theme_constant_override("separation", Tokens.SPACE_2)
	scroll.add_child(list_box)
	if saving:
		content.add_child(UI.label(tr("Name des Spielstands"), Tokens.LABEL))
		name_field = LineEdit.new()
		name_field.placeholder_text = tr("Zum Beispiel: Vor dem Zeitsprung")
		content.add_child(name_field)
	status = UI.label("", Tokens.SMALL, Tokens.INK_MUTED)
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(status)
	var actions := HBoxContainer.new()
	actions.alignment = BoxContainer.ALIGNMENT_END
	content.add_child(actions)
	var close := _button(tr("Schließen"), close_dialog, "GhostButton")
	actions.add_child(close)
	if saving:
		save_action = _button(tr("Hier speichern"), request_save, "PrimaryButton")
		actions.add_child(save_action)
	var resize_card := func():
		card.custom_minimum_size = Vector2(minf(760, size.x - 32), maxf(320, minf(680, size.y - 32)))
	card.set_meta("slot_card", true)
	resized.connect(resize_card)
	resize_card.call()
	refresh()
	close.call_deferred("grab_focus")
	_trap_focus.call_deferred()

func _button(text: String, action: Callable, variation: String = "") -> Button:
	var button := Button.new()
	button.text = text
	button.theme_type_variation = variation
	button.pressed.connect(action)
	return button

func refresh() -> void:
	var result: Dictionary = simulation.get_save_slots(user_dir)
	if not result.get("error", "").is_empty():
		status.text = result["error"]
		if saving: save_action.disabled = true
		return
	rows = result["slots"]
	autosave = result["autosave"] if not saving and result["autosave"]["occupied"] else {}
	if status.text.is_empty(): status.text = tr("%d Speicherplätze · Namen bis %d Zeichen.") % [rows.size(), result["name_length"]]
	if saving: name_field.max_length = result["name_length"]
	for child in list_box.get_children():
		list_box.remove_child(child)
		child.queue_free()
	var occupied := 0
	var free_id := -1
	if not autosave.is_empty():
		occupied += 1
		_render_autosave()
	for row in rows:
		if row["occupied"]:
			occupied += 1
			if selected < 0 and not saving: selected = row["id"]
			_render_slot(row)
		elif free_id < 0: free_id = row["id"]
	if occupied == 0:
		var empty := UI.empty_state(tr("Noch keine Spielstände"), tr("Ein Speicherplatz hält dein Unternehmen vor dem nächsten Zeitsprung fest."), tr("Hier speichern") if saving else tr("Schließen"), func():
			if saving: select_slot(free_id, "")
			else: close_dialog()
		)
		empty.get_child(0).get_child(empty.get_child(0).get_child_count() - 1).theme_type_variation = "GhostButton"
		list_box.add_child(empty)
	elif free_id >= 0 and saving:
		var free := UI.EmptyFrame.new()
		free.add_theme_stylebox_override("panel", Tokens.save_slot_box(false, true))
		list_box.add_child(free)
		free.add_child(_button(tr("Hier speichern") + " · " + tr("Freier Speicherplatz"), func(): select_slot(free_id, ""), "GhostButton"))
	if saving:
		if selected < 0: selected = free_id
		save_action.disabled = selected < 0
		save_action.text = tr("Überschreiben") if selected >= 0 and rows[selected]["occupied"] else tr("Hier speichern")
	_trap_focus.call_deferred()
	_reveal_selection()

func _reveal_selection() -> void:
	await get_tree().process_frame
	if not is_inside_tree(): return
	for card in list_box.get_children():
		if card.get_meta("slot_id", -1) == selected:
			list_box.get_parent().ensure_control_visible(card)

func _render_slot(row: Dictionary) -> void:
	var id: int = row["id"]
	var header: Dictionary = row["header"]
	var damaged: bool = header.is_empty() or not row["error"].is_empty() or failed_slots.has(id)
	var title: String = header.get("name", tr("Beschädigter Spielstand"))
	var card := PanelContainer.new()
	card.set_meta("slot_id", id)
	card.add_theme_stylebox_override("panel", Tokens.save_slot_box(id == selected))
	list_box.add_child(card)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", Tokens.SPACE_1)
	card.add_child(column)
	var choose := _button(title, func(): select_slot(id, title), "GhostButton")
	choose.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choose.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	choose.tooltip_text = title
	choose.add_theme_color_override("font_color", Tokens.INK)
	choose.add_theme_font_override("font", Tokens.font_for(Tokens.BODY_STRONG))
	column.add_child(choose)
	if not header.is_empty():
		var facts := UI.label(_facts(header) + "\n" + _identity(header), Tokens.CODE, Tokens.INK_MUTED)
		facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(facts)
	if damaged:
		var warning := UI.label(tr("Beschädigt · Hauptdatei kann nicht geladen werden."), Tokens.SMALL, Tokens.STOP)
		warning.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(warning)
	if id == selected:
		var actions := HFlowContainer.new()
		column.add_child(actions)
		if not damaged: actions.add_child(_button(tr("Laden"), func(): request_load(id, false), "" if saving else "PrimaryButton"))
		actions.add_child(_button(tr("Löschen"), func(): request_delete(id, title), "DangerButton"))
		if not row["backup"].is_empty():
			var backup: Dictionary = row["backup"]
			var backup_facts := UI.label(tr("Sicherung") + " · " + backup.get("name", "") + "\n" + _facts(backup) + "\n" + _identity(backup), Tokens.CODE, Tokens.INK_MUTED)
			backup_facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			column.add_child(backup_facts)
			column.add_child(_button(tr("Sicherung laden"), func(): request_load(id, true), "PrimaryButton" if damaged and not saving else ""))

func _render_autosave() -> void:
	var header: Dictionary = autosave["header"]
	var damaged: bool = header.is_empty() or not autosave["error"].is_empty() or failed_slots.has(AUTOSAVE_ID)
	var title := tr("Autosave") + " · " + tr("automatisch gespeichert")
	var card := PanelContainer.new()
	card.set_meta("slot_id", AUTOSAVE_ID)
	card.add_theme_stylebox_override("panel", Tokens.save_slot_box(AUTOSAVE_ID == selected))
	list_box.add_child(card)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", Tokens.SPACE_1)
	card.add_child(column)
	var choose := _button(title, func(): select_slot(AUTOSAVE_ID, ""), "GhostButton")
	choose.alignment = HORIZONTAL_ALIGNMENT_LEFT
	choose.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	choose.tooltip_text = title
	choose.add_theme_color_override("font_color", Tokens.INK)
	choose.add_theme_font_override("font", Tokens.font_for(Tokens.BODY_STRONG))
	column.add_child(choose)
	if not header.is_empty():
		var facts := UI.label(_facts(header) + "\n" + _identity(header), Tokens.CODE, Tokens.INK_MUTED)
		facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(facts)
	if damaged:
		var warning := UI.label(tr("Beschädigt · Hauptdatei kann nicht geladen werden."), Tokens.SMALL, Tokens.STOP)
		warning.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		column.add_child(warning)
	if AUTOSAVE_ID == selected:
		var actions := HFlowContainer.new()
		column.add_child(actions)
		if not damaged: actions.add_child(_button(tr("Laden"), func(): request_load(AUTOSAVE_ID, false), "PrimaryButton"))
		if not autosave["backup"].is_empty():
			var backup: Dictionary = autosave["backup"]
			var backup_facts := UI.label(tr("Vorheriger Autosave") + "\n" + _facts(backup) + "\n" + _identity(backup), Tokens.CODE, Tokens.INK_MUTED)
			backup_facts.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			column.add_child(backup_facts)
			column.add_child(_button(tr("Sicherung laden"), func(): request_load(AUTOSAVE_ID, true), "PrimaryButton" if damaged else ""))

func _facts(header: Dictionary) -> String:
	# Wall clock is display only; simulation time is already formatted by Rust.
	var written := Time.get_datetime_string_from_unix_time(header["written_at"]).replace("T", " ")
	return "%s · %s · %d %s · %s UTC" % [header["game_time"], UI.money(header["cash"]), header["fleet"], tr("Flugzeuge"), written]

func _identity(header: Dictionary) -> String:
	if not header.has("airline_name"): return tr("Airline unbekannt · älterer Spielstand ohne Airline-Angaben")
	var airport: String = header["home_airport_icao"]
	if not header.get("home_airport_name", "").is_empty(): airport += " " + header["home_airport_name"]
	return "%s · %s" % [header["airline_name"], airport]

func select_slot(id: int, title: String) -> void:
	selected = id
	if saving: name_field.text = title
	refresh()
	if saving: name_field.grab_focus()
	else:
		for control in list_box.get_children():
			if control.get_meta("slot_id", -1) == id:
				control.get_child(0).get_child(0).grab_focus()

func request_save() -> void:
	if selected < 0 or save_action.disabled: return
	var id := selected
	var title := name_field.text
	if rows[id]["occupied"]:
		_confirm(tr("Spielstand überschreiben?"), tr("Der bisherige Spielstand wird zur Sicherung. Eine ältere Sicherung wird ersetzt.") + "\n" + title, tr("Überschreiben"), func(): _store(id, title, true))
	else: _store(id, title, false)

func _store(id: int, title: String, overwrite: bool) -> void:
	var error: String = simulation.save_slot(user_dir, id, title, overwrite)
	status.text = tr("Spielstand gespeichert.") if error.is_empty() else error
	finished.emit(status.text, false)
	if error.is_empty(): failed_slots.erase(id)
	refresh()

func request_load(id: int, backup: bool) -> void:
	if not simulation.has_airline():
		load_slot(id, backup)
		return
	var row := _entry(id)
	var header: Dictionary = row.get("backup", {}) if backup else row.get("header", {})
	var shown: String = header.get("name", "")
	if id == AUTOSAVE_ID: shown = tr("Vorheriger Autosave") if backup else tr("Autosave")
	elif shown.is_empty(): shown = tr("Beschädigter Spielstand")
	if header.has("game_time"): shown += " · " + header["game_time"]
	var question := tr("Sicherung laden?") if backup else (tr("Autosave laden?") if id == AUTOSAVE_ID else tr("Spielstand laden?"))
	var current: Dictionary = simulation.get_airline_info()
	var current_line: String = tr("Aktuelle Airline") + ": " + current.get("name", "") + " · " + current.get("airport_icao", "")
	var target_line: String = tr("Ziel") + ": " + shown + "\n" + (_identity(header) if not header.is_empty() else tr("Airline unbekannt · älterer Spielstand ohne Airline-Angaben"))
	_confirm(question, current_line + "\n" + target_line + "\n" + tr("Nicht gespeicherter Fortschritt in der aktuellen Airline geht verloren."), tr("Sicherung laden") if backup else tr("Laden"), func(): load_slot(id, backup), _focus_selected)

func _entry(id: int) -> Dictionary:
	if id == AUTOSAVE_ID: return autosave
	for row in rows:
		if row["id"] == id: return row
	return {}

func _focus_selected() -> void:
	if not is_inside_tree() or is_queued_for_deletion(): return
	for card in list_box.get_children():
		if card.get_meta("slot_id", -1) == selected:
			card.get_child(0).get_child(0).grab_focus()
			return
	_focus_default()

func load_slot(id: int, backup: bool) -> void:
	var error: String = simulation.load_autosave(user_dir, backup) if id == AUTOSAVE_ID else simulation.load_slot(user_dir, id, backup)
	if error.is_empty():
		finished.emit(tr("Sicherung geladen.") if backup else tr("Spielstand geladen."), true)
		close_dialog()
	else:
		status.text = error
		if not backup: failed_slots[id] = true
		refresh()

func request_delete(id: int, title: String) -> void:
	_confirm(tr("Spielstand löschen?"), title + "\n" + tr("Spielstand und Sicherung werden endgültig gelöscht."), tr("Löschen"), func():
		var error: String = simulation.delete_save_slot(user_dir, id)
		status.text = tr("Spielstand gelöscht.") if error.is_empty() else error
		finished.emit(status.text, false)
		if error.is_empty(): selected = -1
		refresh()
	)

func _confirm(title: String, body: String, action: String, confirmed: Callable, refocus: Callable = Callable()) -> void:
	decision = UI.dialog(self, title, body, tr("Abbrechen"), action, confirmed)
	decision.tree_exited.connect(func():
		if is_inside_tree(): (refocus if refocus.is_valid() else _focus_default).call_deferred()
	)
	decision.popup_centered()

func _focus_default() -> void:
	if saving:
		name_field.grab_focus()
	else:
		content.get_child(content.get_child_count() - 1).get_child(0).grab_focus()

func close_dialog() -> void:
	if is_instance_valid(focus_before): focus_before.grab_focus()
	queue_free()

func _trap_focus() -> void:
	var focusable: Array[Control] = []
	_collect_focus(content, focusable)
	for i in range(focusable.size()):
		var control := focusable[i]
		if list_box.is_ancestor_of(control) and not control.has_meta("reveal_bound"):
			control.set_meta("reveal_bound", true)
			control.focus_entered.connect(_reveal_focused.bind(control))
		control.focus_next = control.get_path_to(focusable[(i + 1) % focusable.size()])
		control.focus_previous = control.get_path_to(focusable[(i - 1 + focusable.size()) % focusable.size()])
		for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
			control.set_focus_neighbor(side, control.focus_previous if side in [SIDE_LEFT, SIDE_TOP] else control.focus_next)

func _reveal_focused(control: Control) -> void:
	if is_instance_valid(control) and control.is_inside_tree():
		list_box.get_parent().ensure_control_visible(control)

func _collect_focus(node: Node, controls: Array[Control]) -> void:
	if node is Control and node.focus_mode == Control.FOCUS_ALL and node.is_visible_in_tree():
		if not node is Button or not node.disabled: controls.append(node)
	for child in node.get_children(): _collect_focus(child, controls)

func _input(event: InputEvent) -> void:
	if is_instance_valid(decision) or not get_tree().get_nodes_in_group("quit_prompt").is_empty(): return
	if event.is_action_pressed("ui_cancel"):
		get_viewport().set_input_as_handled()
		close_dialog()
