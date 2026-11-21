extends VBoxContainer
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## Staff tab. Every number, shortage and rule comes from Rust; this only renders
## it and forwards hire / dismiss requests.
const UI = preload("res://scripts/ui/components.gd")
signal changed
var simulation: RefCounted
var metrics: Dictionary = {}
var shortage_label: Label
var hire_missing_button: Button
var roles_tree: Tree
var aircraft_tree: Tree
var staff_tree: Tree
var rating_select: OptionButton
var ratings_tree: Tree
var train_button: Button
var course_select: OptionButton
var course_terms: Label
var courses: Array = []
var role_select: OptionButton
var count_spin: SpinBox
var hire_button: Button
var dismiss_button: Button
var action_status: Label
var dismiss_status: Label
var role_keys: Array = []
var signature := ""
var elapsed := 0.0
var selected_id := 0
var staff_signature := ""
var staff_empty: Control
var aircraft_empty: Control
var pay_cards: Dictionary = {}
var pay_dialog: UI.DecisionDialog
var training_rows: VBoxContainer

func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 4 if grid.size.x >= 1150 else (2 if grid.size.x >= 600 else 1))
	for item in [["employees", "MITARBEITER"], ["payroll", "GEHÄLTER PRO MONAT"], ["next_pay", "NÄCHSTE GEHALTSZAHLUNG"], ["missing", "PERSONALMANGEL"]]:
		metrics[item[0]] = UI.metric(grid, item[1])
	for key in ["pilot", "cabin_crew", "technician"]:
		var card := UI.panel(self, tr("STAFF_PAY_" + key.to_upper()))
		var status := HBoxContainer.new()
		card.add_child(status)
		var progress := UI.progress(0)
		card.add_child(progress)
		var factors := UI.label("", ThemeTokens.SMALL, UI.MUTED)
		factors.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		card.add_child(factors)
		var row := HBoxContainer.new()
		card.add_child(row)
		var slider := HSlider.new()
		slider.step = 1
		slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		slider.custom_minimum_size.y = 36
		row.add_child(slider)
		var value := UI.label("", ThemeTokens.DATA)
		row.add_child(value)
		var preview := UI.label("", ThemeTokens.SMALL, UI.MUTED)
		preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		card.add_child(preview)
		var apply := Button.new()
		apply.text = tr("STAFF_PAY_APPLY")
		apply.pressed.connect(func(): _confirm_pay(key))
		card.add_child(apply)
		pay_cards[key] = {"status": status, "progress": progress, "factors": factors, "slider": slider, "value": value, "preview": preview, "apply": apply, "row": {}}
		slider.value_changed.connect(func(_v): _pay_preview(key))
	var shortage := UI.panel(self, "ENGPÄSSE")
	shortage_label = UI.label("", ThemeTokens.BODY)
	shortage_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shortage.add_child(shortage_label)
	hire_missing_button = Button.new()
	hire_missing_button.text = "Fehlendes Personal einstellen"
	hire_missing_button.theme_type_variation = "PrimaryButton"
	hire_missing_button.pressed.connect(_hire_missing)
	shortage.add_child(hire_missing_button)

	var roles := UI.panel(self, "PERSONALBESTAND NACH ROLLE")
	roles_tree = UI.table(["Rolle", "Beschäftigt", "Im Einsatz", "Verfügbar", "Fehlt", "Gehalt je Person / Monat", "Kosten / Monat"], [150, 100, 100, 100, 80, 170, 130], 200)
	roles.add_child(roles_tree)
	var rules := UI.label("Piloten und Flugbegleiter sind an einen aktiven Flugplan gebunden (auch während einer Wartung). Techniker werden nur während einer Wartung gebraucht. Ohne ausreichend freies Personal startet kein Flug und keine Wartung.", ThemeTokens.SMALL, UI.MUTED)
	rules.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	roles.add_child(rules)

	var ratings := UI.panel(self, "MUSTERBERECHTIGUNGEN")
	ratings_tree = UI.table(["Muster", "Berechtigt", "Im Einsatz", "Verfügbar"], [180, 100, 100, 100], 130)
	ratings.add_child(ratings_tree)
	var hire := UI.panel(self, "EINSTELLEN")
	var hire_row := VBoxContainer.new()
	hire_row.add_theme_constant_override("separation", 8)
	hire.add_child(hire_row)
	role_select = OptionButton.new()
	role_select.custom_minimum_size.x = 300
	role_select.fit_to_longest_item = false
	role_select.clip_text = true
	UI.field(hire_row, "Rolle", role_select)
	rating_select = OptionButton.new()
	rating_select.tooltip_text = "Muster für Piloten-Einstellung und Umschulung"
	UI.field(hire_row, "Muster", rating_select, rating_select.tooltip_text)
	count_spin = SpinBox.new()
	count_spin.get_line_edit().theme_type_variation = "NumericInput"
	count_spin.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	count_spin.custom_minimum_size.x = 170
	count_spin.min_value = 1
	count_spin.max_value = 50
	count_spin.value = 1
	count_spin.suffix = "Personen"
	UI.field(hire_row, "Personen", count_spin, "", "Personen")
	hire_button = Button.new()
	hire_button.text = "Einstellen"
	hire_button.pressed.connect(_hire)
	hire_row.add_child(hire_button)
	action_status = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	action_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hire.add_child(action_status)
	hire.move_child(action_status, 1)

	var planes := UI.panel(self, "PERSONALBEDARF JE FLUGZEUG")
	aircraft_tree = UI.table(["Flugzeug", "Modell", "Piloten", "Flugbegleiter", "Techniker (Wartung)", "Status"], [110, 160, 80, 110, 150, 320], 150)
	planes.add_child(aircraft_tree)
	aircraft_empty = UI.empty_state("PERSONALBEDARF JE FLUGZEUG", rules.text, "Flugzeug kaufen", func():
		var page: Node = self
		while not page.has_method("_show_page"): page = page.get_parent()
		page._show_page(1)
	)
	planes.add_child(aircraft_empty)

	var people := UI.panel(self, "MITARBEITER")
	staff_tree = UI.table(["Name", "Rolle", "Gehalt / Monat", "Status", "Eingestellt", "Muster"], [190, 190, 130, 340, 180, 110], 300)
	staff_tree.item_selected.connect(_selection_changed)
	staff_tree.nothing_selected.connect(_selection_changed)
	people.add_child(staff_tree)
	staff_empty = UI.empty_state("MITARBEITER", rules.text, hire_button.text, func():
		get_parent().get_parent().ensure_control_visible(role_select)
		role_select.grab_focus()
	)
	people.add_child(staff_empty)
	training_rows = VBoxContainer.new()
	training_rows.add_theme_constant_override("separation", ThemeTokens.SPACE_3)
	people.add_child(training_rows)
	dismiss_button = Button.new()
	dismiss_button.text = "Ausgewählten Mitarbeiter entlassen"
	dismiss_button.pressed.connect(_dismiss)
	dismiss_status = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	dismiss_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	people.add_child(dismiss_status)
	people.add_child(dismiss_button)
	course_select = OptionButton.new()
	course_select.fit_to_longest_item = false
	course_select.clip_text = true
	course_select.item_selected.connect(func(_i: int) -> void: _course_text())
	UI.field(people, "Umschulungskurs", course_select, "Intern: günstiger, dafür länger. Extern: schneller, aber teurer. Der Pilot bleibt bis zum Abschluss nicht verfügbar.")
	course_terms = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	course_terms.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	people.add_child(course_terms)
	train_button = Button.new()
	train_button.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	train_button.pressed.connect(_train)
	people.add_child(train_button)
	var dismissal_rules := UI.label("Bei der Entlassung wird das bis dahin angefallene Gehalt anteilig auf volle EUR aufgerundet gezahlt. Mitarbeiter im Einsatz können nicht entlassen werden.", ThemeTokens.SMALL, UI.MUTED)
	dismissal_rules.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	people.add_child(dismissal_rules)
	refresh(true)

func _process(delta: float) -> void:
	if not is_visible_in_tree():
		return
	elapsed += delta
	if elapsed >= 0.5:
		elapsed = 0
		refresh()

func refresh(force: bool = false) -> void:
	if simulation == null:
		return
	var overview: Dictionary = simulation.get_staff_overview()
	var staff: Array = simulation.get_staff()
	var airline: Dictionary = simulation.get_airline_info()
	_training(staff)
	var key := "%s/%s/%s" % [overview, staff, airline.get("capital_eur", 0)]
	if key == signature and not force:
		return
	signature = key
	metrics["employees"].text = str(overview["employees"])
	metrics["payroll"].text = UI.money(overview["monthly_payroll_eur"])
	metrics["next_pay"].text = overview["next_payroll"]
	metrics["missing"].text = "Keiner" if overview["missing_total"] == 0 else "%d fehlen" % overview["missing_total"]
	metrics["missing"].add_theme_color_override("font_color", ThemeTokens.INK if overview["missing_total"] == 0 else UI.NEGATIVE)
	ratings_tree.clear()
	var rating_root := ratings_tree.create_item()
	var first_rating := rating_select.item_count == 0
	for rating in overview["ratings"]:
		if first_rating:
			rating_select.add_item(rating["rating"])
		var row := ratings_tree.create_item(rating_root)
		var values := [rating["rating"], str(rating["rated"]), str(rating["busy"]), str(rating["available"])]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
		UI.cell_status(row, 0, rating["rating"], "neutral")
	_courses(overview["conversion_courses"])
	_pay_groups(overview)
	_roles(overview)
	_aircraft(overview)
	_staff(staff)
	_shortage(overview)
	hire_missing_button.disabled = overview["missing_total"] == 0 or not simulation.has_airline()
	hire_button.disabled = not simulation.has_airline()
	count_spin.max_value = overview["max_batch"]

func _courses(rows: Array) -> void:
	var labels: Array = []
	for row in rows:
		labels.append("%s · %s · %d Tage" % [row["label"], UI.money(row["cost_eur"]), row["days"]])
	if course_select.item_count != labels.size() or courses.size() != rows.size() or courses.map(func(c): return c["course"]) != rows.map(func(c): return c["course"]):
		var keep := maxi(course_select.selected, 0)
		course_select.clear()
		for label in labels: course_select.add_item(label)
		course_select.select(mini(keep, labels.size() - 1))
	else:
		for i in range(labels.size()): course_select.set_item_text(i, labels[i])
	courses = rows
	_course_text()

func _course_text() -> void:
	if courses.is_empty() or course_select.selected < 0: return
	var row: Dictionary = courses[course_select.selected]
	course_terms.text = "%s: %s, fertig voraussichtlich %s." % [row["label"], UI.money(row["cost_eur"]), row["ends_text"]]
	if not row["affordable"]: course_terms.text += " Nicht genügend Guthaben."
	course_terms.add_theme_color_override("font_color", UI.MUTED if row["affordable"] else UI.NEGATIVE)
	train_button.text = "Umschulen · %s · %s · %d Tage" % [row["label"], UI.money(row["cost_eur"]), row["days"]]

func _roles(overview: Dictionary) -> void:
	var first := role_keys.is_empty()
	roles_tree.clear()
	var root := roles_tree.create_item()
	for role in overview["roles"]:
		if first:
			role_keys.append(role["role"])
			role_select.add_item("%s · %s / Monat" % [role["singular"], UI.money(role["salary_eur"])])
		var row := roles_tree.create_item(root)
		var values := [role["name"], str(role["employed"]), str(role["busy"]), str(role["available"]), str(role["missing"]), UI.money(role["salary_eur"]), UI.money(role["monthly_cost_eur"])]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
		if role["missing"] > 0:
			row.set_custom_color(4, UI.NEGATIVE)

func _aircraft(overview: Dictionary) -> void:
	aircraft_tree.visible = not overview["aircraft"].is_empty()
	aircraft_empty.visible = not aircraft_tree.visible
	aircraft_tree.clear()
	var root := aircraft_tree.create_item()
	for plane in overview["aircraft"]:
		var row := aircraft_tree.create_item(root)
		row.set_metadata(0, plane["id"])
		var status := ""
		var color := UI.MUTED
		match plane["state"]:
			"crewed":
				status = "Besatzung zugeteilt · im Flugplan"
				color = UI.POSITIVE
			"hangar":
				status = "In der Wartung · Techniker im Einsatz"
				color = UI.POSITIVE
			"ready":
				status = "Besatzung verfügbar · Flugplan möglich"
				color = UI.POSITIVE
			"waiting_technicians":
				status = plane["problem"]
				color = UI.NEGATIVE
			_:
				status = plane["problem"]
				color = UI.NEGATIVE
		var values := [plane["identifier"], plane["model_name"], str(plane["pilots"]), str(plane["cabin_crew"]), str(plane["technicians"]), status]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
			row.set_tooltip_text(i, status)
		UI.cell_status(row, 5, status, "info" if plane["state"] == "hangar" else "go" if color == UI.POSITIVE else "stop")

func _staff(staff: Array) -> void:
	staff_tree.visible = not staff.is_empty()
	staff_empty.visible = staff.is_empty()
	var key := str(staff)
	if key == staff_signature:
		return
	staff_signature = key
	var previous_id := selected_id
	staff_tree.clear()
	var root := staff_tree.create_item()
	var selected: TreeItem = null
	for employee in staff:
		var row := staff_tree.create_item(root)
		row.set_metadata(0, employee)
		var values := [employee["name"], "%s · %s" % [employee["role_name"], employee["rating"]], UI.money(employee["salary_eur"]), employee["status_text"], "%s (%d Tage)" % [employee["hired"], employee["days_employed"]]]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
			row.set_tooltip_text(i, values[i])
		UI.cell_status(row, 5, employee["rating"], "neutral")
		UI.cell_status(row, 3, employee["status_text"], "neutral" if employee["status"] == "available" else "info")
		if employee["id"] == previous_id:
			selected = row
	if selected != null:
		selected.select(0)
	_selection_changed()

func _shortage(overview: Dictionary) -> void:
	var lines: PackedStringArray = []
	if not simulation.has_airline():
		shortage_label.text = "Noch keine Airline gegründet."
		return
	for role in overview["roles"]:
		if role["missing"] > 0:
			lines.append("%s: %d fehlen (verfügbar %d)" % [role["name"], role["missing"], role["available"]])
	for plane in overview["aircraft"]:
		if plane["state"] == "short" or plane["state"] == "waiting_technicians":
			lines.append("%s %s: %s" % [plane["identifier"], plane["model_name"], plane["problem"]])
	if lines.is_empty():
		shortage_label.text = "Kein Personalmangel. Alle Flugzeuge können besetzt werden."
		shortage_label.add_theme_color_override("font_color", UI.POSITIVE)
	else:
		shortage_label.text = "\n".join(lines)
		shortage_label.add_theme_color_override("font_color", UI.NEGATIVE)

func _selection_changed() -> void:
	var row := staff_tree.get_selected()
	if row == null:
		selected_id = 0
		dismiss_button.disabled = true
		train_button.disabled = true
		return
	var employee: Dictionary = row.get_metadata(0)
	selected_id = employee["id"]
	train_button.disabled = employee["role"] != "pilot" or not employee["can_dismiss"]
	dismiss_button.disabled = not employee["can_dismiss"]
	dismiss_button.tooltip_text = "" if employee["can_dismiss"] else "Im Einsatz: erst Flugplan beenden bzw. Wartung abwarten."

func _report(error: String, success: String, field: Control = null) -> void:
	if field != null: UI.field_error(field, error)
	for status in [action_status, dismiss_status]:
		status.text = success if error.is_empty() else error
		status.add_theme_color_override("font_color", UI.POSITIVE if error.is_empty() else UI.NEGATIVE)
	if error.is_empty():
		changed.emit()
	refresh(true)

func _hire() -> void:
	count_spin.apply()
	var role: String = role_keys[role_select.selected]
	var count := int(count_spin.value)
	var error: String = simulation.hire_pilots(rating_select.get_item_text(rating_select.selected), count) if role == "pilot" else simulation.hire_staff(role, count)
	_report(error, "%d Mitarbeiter eingestellt. Bitte Spielstand speichern." % count, count_spin)

func _hire_missing() -> void:
	_report(simulation.hire_missing_staff(), "Fehlendes Personal eingestellt. Bitte Spielstand speichern.")

func _dismiss() -> void:
	if selected_id == 0:
		return
	_report(simulation.dismiss_staff(selected_id), "Mitarbeiter entlassen · anteiliges Gehalt gezahlt.")

func _train() -> void:
	_report(simulation.train_pilot_course(selected_id, rating_select.get_item_text(rating_select.selected), courses[course_select.selected]["course"]), "Umschulung begonnen. Bitte Spielstand speichern.")


func focus_employee(id: int) -> void:
	var row := staff_tree.get_root().get_first_child()
	while row != null:
		if row.get_metadata(0)["id"] == id:
			row.select(0)
			_selection_changed()
			staff_tree.ensure_cursor_is_visible()
			staff_tree.grab_focus()
			return
		row = row.get_next()

## The existing API exposes the Rust deadline in status_text, not as a number.
## Decode that display value only; now and duration both come from Rust.
## No accumulated UI time, disk save access or simulation mutation is involved.
func _training(staff: Array) -> void:
	for child in training_rows.get_children():
		training_rows.remove_child(child)
		child.queue_free()
	for employee in staff:
		if employee["status"] != "training": continue
		var sentence := UI.label(employee["name"] + " · " + employee["status_text"], ThemeTokens.SMALL, UI.MUTED)
		sentence.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		training_rows.add_child(sentence)
		training_rows.add_child(UI.progress(employee["training_progress"], "info"))

func _pay_groups(overview: Dictionary) -> void:
	for group: Dictionary in overview["roles"]:
		var card: Dictionary = pay_cards[group["role"]]
		var previous: Dictionary = card["row"]
		card["row"] = group
		var slider: HSlider = card["slider"]
		slider.min_value = overview["pay_min"]
		slider.max_value = overview["pay_max"]
		if previous.is_empty() or previous["pending_percent"] != group["pending_percent"]:
			slider.set_value_no_signal(group["pending_percent"])
		var kind: String = {"calm": "go", "unrest": "caution", "warning": "caution", "strike": "stop"}[group["labour_state"]]
		for child in card["status"].get_children():
			card["status"].remove_child(child)
			child.queue_free()
		card["status"].add_child(UI.status_chip(tr("STAFF_STATE_" + str(group["labour_state"]).to_upper()), kind))
		if not group["labour_date"].is_empty():
			var date := UI.label(group["labour_date"], ThemeTokens.SMALL)
			date.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			date.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			card["status"].add_child(date)
		card["progress"].get_child(0).value = group["morale"]
		card["progress"].get_child(1).text = tr("STAFF_MORALE") % group["morale"]
		var factors: Array[String] = [tr("STAFF_PAY_TARGET") % [group["pay_percent"], group["pay_target"]]]
		if group["understaffed"]: factors.append(tr("STAFF_FACTOR_SHORT"))
		if group["missed_payroll"]: factors.append(tr("STAFF_FACTOR_MISSED"))
		if group["restructuring"]: factors.append(tr("STAFF_FACTOR_RESTRUCTURE"))
		card["factors"].text = " · ".join(factors)
		_pay_preview(group["role"])

func _pay_preview(key: String) -> void:
	var card: Dictionary = pay_cards[key]
	if card["row"].is_empty(): return
	var percent := int(card["slider"].value)
	var quote: Dictionary = simulation.preview_staff_pay(key, percent)
	card["value"].text = "%d %%" % percent
	card["preview"].text = tr("STAFF_PAY_PREVIEW") % [UI.money(quote["cost"]), UI.money(quote["delta"]), card["row"]["pending_percent"]]
	card["apply"].disabled = percent == card["row"]["pending_percent"] or not simulation.has_airline()

func _confirm_pay(key: String) -> void:
	var card: Dictionary = pay_cards[key]
	var percent := int(card["slider"].value)
	pay_dialog = UI.dialog(self, tr("STAFF_PAY_CONFIRM") % percent, card["preview"].text + "\n" + tr("STAFF_PAY_TIMING"), tr("STAFF_PAY_BACK"), tr("STAFF_PAY_APPLY"), func():
		var error: String = simulation.set_staff_pay(key, percent)
		card["preview"].text = tr("STAFF_PAY_SAVED") if error.is_empty() else error
		refresh(true)
		changed.emit())
	pay_dialog.get_ok_button().theme_type_variation = "PrimaryButton"
	pay_dialog.popup_centered()

func focus_shortage(aircraft_id: int) -> void:
	refresh(true)
	var row := aircraft_tree.get_root().get_first_child() if aircraft_tree.get_root() != null else null
	while row != null:
		if aircraft_id == row.get_metadata(0):
			aircraft_tree.set_selected(row, 0)
			break
		row = row.get_next()
	for frame in range(4):
		await get_tree().process_frame
	hire_missing_button.grab_focus()
	var scroll := get_parent().get_parent() as ScrollContainer
	scroll.ensure_control_visible(hire_missing_button.get_parent())
	await get_tree().process_frame
	scroll.ensure_control_visible(hire_missing_button.get_parent())

func focus_pay_group(index: int) -> void:
	if index < 0 or index >= role_keys.size(): return
	var slider: HSlider = pay_cards[role_keys[index]]["slider"]
	get_parent().get_parent().ensure_control_visible(slider.get_parent().get_parent())
	slider.grab_focus()

func focus_technician_hiring() -> void:
	var index := role_keys.find("technician")
	if index >= 0:
		role_select.select(index)
	for frame in range(4):
		await get_tree().process_frame
	get_parent().get_parent().ensure_control_visible(role_select.get_parent())
	role_select.grab_focus()

func prepare_pilot_hire(rating: String, missing: int) -> void:
	refresh(true)
	role_select.select(role_keys.find("pilot"))
	for i in range(rating_select.item_count):
		if rating_select.get_item_text(i) == rating:
			rating_select.select(i)
	count_spin.value = clampi(missing, int(count_spin.min_value), int(count_spin.max_value))
	var text := "Für die Überführung fehlen %d Piloten mit Muster %s. Bitte prüfen und einstellen; danach in Routen zurückkehren." % [missing, rating]
	action_status.text = text
	action_status.add_theme_color_override("font_color", UI.MUTED)
	for frame in range(4):
		await get_tree().process_frame
	var scroll := get_parent().get_parent() as ScrollContainer
	scroll.ensure_control_visible(count_spin.get_parent())
	count_spin.get_line_edit().grab_focus()
	await get_tree().process_frame
	scroll.ensure_control_visible(count_spin.get_parent())
