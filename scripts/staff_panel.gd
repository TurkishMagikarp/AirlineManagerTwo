extends VBoxContainer
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

func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 4 if grid.size.x >= 1150 else (2 if grid.size.x >= 600 else 1))
	for item in [["employees", "MITARBEITER"], ["payroll", "GEHÄLTER PRO MONAT"], ["next_pay", "NÄCHSTE GEHALTSZAHLUNG"], ["missing", "PERSONALMANGEL"]]:
		metrics[item[0]] = UI.metric(grid, item[1])
	var shortage := UI.panel(self, "ENGPÄSSE")
	shortage_label = UI.label("", 15)
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
	var rules := UI.label("Piloten und Flugbegleiter sind an einen aktiven Flugplan gebunden (auch während einer Wartung). Techniker werden nur während einer Wartung gebraucht. Ohne ausreichend freies Personal startet kein Flug und keine Wartung.", 13, UI.MUTED)
	rules.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	roles.add_child(rules)

	var ratings := UI.panel(self, "MUSTERBERECHTIGUNGEN")
	ratings_tree = UI.table(["Muster", "Berechtigt", "Im Einsatz", "Verfügbar"], [180, 100, 100, 100], 130)
	ratings.add_child(ratings_tree)
	var hire := UI.panel(self, "EINSTELLEN")
	var hire_row := HFlowContainer.new()
	hire_row.add_theme_constant_override("separation", 8)
	hire.add_child(hire_row)
	role_select = OptionButton.new()
	role_select.custom_minimum_size.x = 300
	role_select.fit_to_longest_item = false
	role_select.clip_text = true
	hire_row.add_child(role_select)
	rating_select = OptionButton.new()
	rating_select.tooltip_text = "Muster für Piloten-Einstellung und Umschulung"
	hire_row.add_child(rating_select)
	count_spin = SpinBox.new()
	count_spin.custom_minimum_size.x = 170
	count_spin.min_value = 1
	count_spin.max_value = 50
	count_spin.value = 1
	count_spin.suffix = "Personen"
	hire_row.add_child(count_spin)
	hire_button = Button.new()
	hire_button.text = "Einstellen"
	hire_button.pressed.connect(_hire)
	hire_row.add_child(hire_button)
	action_status = UI.label("", 14, UI.MUTED)
	action_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hire.add_child(action_status)
	hire.move_child(action_status, 1)

	var planes := UI.panel(self, "PERSONALBEDARF JE FLUGZEUG")
	aircraft_tree = UI.table(["Flugzeug", "Modell", "Piloten", "Flugbegleiter", "Techniker (Wartung)", "Status"], [110, 160, 80, 110, 150, 320], 150)
	planes.add_child(aircraft_tree)

	var people := UI.panel(self, "MITARBEITER")
	staff_tree = UI.table(["Name", "Rolle", "Gehalt / Monat", "Status", "Eingestellt"], [190, 190, 130, 340, 180], 300)
	staff_tree.item_selected.connect(_selection_changed)
	staff_tree.nothing_selected.connect(_selection_changed)
	people.add_child(staff_tree)
	dismiss_button = Button.new()
	dismiss_button.text = "Ausgewählten Mitarbeiter entlassen"
	dismiss_button.pressed.connect(_dismiss)
	dismiss_status = UI.label("", 14, UI.MUTED)
	dismiss_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	people.add_child(dismiss_status)
	people.add_child(dismiss_button)
	train_button = Button.new()
	train_button.pressed.connect(_train)
	people.add_child(train_button)
	var dismissal_rules := UI.label("Bei der Entlassung wird das bis dahin angefallene Gehalt anteilig auf volle EUR aufgerundet gezahlt. Mitarbeiter im Einsatz können nicht entlassen werden.", 13, UI.MUTED)
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
	var key := "%s/%s/%s" % [overview, staff, airline.get("capital_eur", 0)]
	if key == signature and not force:
		return
	signature = key
	metrics["employees"].text = str(overview["employees"])
	metrics["payroll"].text = UI.money(overview["monthly_payroll_eur"])
	metrics["next_pay"].text = overview["next_payroll"]
	metrics["missing"].text = "Keiner" if overview["missing_total"] == 0 else "%d fehlen" % overview["missing_total"]
	metrics["missing"].add_theme_color_override("font_color", UI.POSITIVE if overview["missing_total"] == 0 else UI.NEGATIVE)
	ratings_tree.clear()
	var rating_root := ratings_tree.create_item()
	var first_rating := rating_select.item_count == 0
	for rating in overview["ratings"]:
		if first_rating:
			rating_select.add_item(rating["rating"])
		var row := ratings_tree.create_item(rating_root)
		var values := [rating["rating"], str(rating["rated"]), str(rating["busy"]), str(rating["available"])]
		for i in range(values.size()):
			row.set_text(i, values[i])
	train_button.text = "Umschulung auf gewähltes Muster · %s · %d Tage" % [UI.money(overview["conversion_cost_eur"]), overview["conversion_days"]]
	_roles(overview)
	_aircraft(overview)
	_staff(staff)
	_shortage(overview)
	hire_missing_button.disabled = overview["missing_total"] == 0 or not simulation.has_airline()
	hire_button.disabled = not simulation.has_airline()
	count_spin.max_value = overview["max_batch"]

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
			row.set_text(i, values[i])
		if role["missing"] > 0:
			row.set_custom_color(4, UI.NEGATIVE)

func _aircraft(overview: Dictionary) -> void:
	aircraft_tree.clear()
	var root := aircraft_tree.create_item()
	for plane in overview["aircraft"]:
		var row := aircraft_tree.create_item(root)
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
			row.set_text(i, values[i])
			row.set_tooltip_text(i, status)
		row.set_custom_color(5, color)

func _staff(staff: Array) -> void:
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
			row.set_text(i, values[i])
			row.set_tooltip_text(i, values[i])
		row.set_custom_color(3, UI.MUTED if employee["status"] == "available" else UI.ACCENT)
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

func _report(error: String, success: String) -> void:
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
	_report(error, "%d Mitarbeiter eingestellt. Bitte Spielstand speichern." % count)

func _hire_missing() -> void:
	_report(simulation.hire_missing_staff(), "Fehlendes Personal eingestellt. Bitte Spielstand speichern.")

func _dismiss() -> void:
	if selected_id == 0:
		return
	_report(simulation.dismiss_staff(selected_id), "Mitarbeiter entlassen · anteiliges Gehalt gezahlt.")

func _train() -> void:
	_report(simulation.train_pilot(selected_id, rating_select.get_item_text(rating_select.selected)), "Umschulung begonnen. Bitte Spielstand speichern.")


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
