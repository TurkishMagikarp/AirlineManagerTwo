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
var role_select: OptionButton
var count_spin: SpinBox
var hire_button: Button
var dismiss_button: Button
var action_status: Label
var role_keys: Array = []
var signature := ""
var elapsed := 0.0
var selected_id := 0

func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 4 if grid.size.x >= 1150 else 2)
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
	roles_tree = UI.table(["Rolle", "Beschäftigt", "Im Einsatz", "Verfügbar", "Fehlt", "Gehalt je Person / Monat", "Kosten / Monat"], [150, 100, 100, 100, 80, 170, 130], 130)
	roles.add_child(roles_tree)
	roles.add_child(UI.label("Piloten und Flugbegleiter sind an einen aktiven Flugplan gebunden (auch während einer Wartung). Techniker werden nur während einer Wartung gebraucht. Ohne ausreichend freies Personal startet kein Flug und keine Wartung.", 13, UI.MUTED))

	var hire := UI.panel(self, "EINSTELLEN")
	var hire_row := HBoxContainer.new()
	hire_row.add_theme_constant_override("separation", 8)
	hire.add_child(hire_row)
	role_select = OptionButton.new()
	role_select.custom_minimum_size.x = 200
	hire_row.add_child(role_select)
	count_spin = SpinBox.new()
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

	var planes := UI.panel(self, "PERSONALBEDARF JE FLUGZEUG")
	aircraft_tree = UI.table(["Flugzeug", "Modell", "Piloten", "Flugbegleiter", "Techniker (Wartung)", "Status"], [110, 160, 80, 110, 150, 320], 150)
	planes.add_child(aircraft_tree)

	var people := UI.panel(self, "MITARBEITER")
	staff_tree = UI.table(["Name", "Rolle", "Gehalt / Monat", "Status", "Eingestellt"], [190, 130, 130, 150, 180], 300)
	staff_tree.item_selected.connect(_selection_changed)
	staff_tree.nothing_selected.connect(_selection_changed)
	people.add_child(staff_tree)
	dismiss_button = Button.new()
	dismiss_button.text = "Ausgewählten Mitarbeiter entlassen"
	dismiss_button.pressed.connect(_dismiss)
	people.add_child(dismiss_button)
	people.add_child(UI.label("Bei der Entlassung wird das bis dahin angefallene Gehalt anteilig gezahlt. Mitarbeiter im Einsatz können nicht entlassen werden.", 13, UI.MUTED))
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
	staff_tree.clear()
	var root := staff_tree.create_item()
	var selected: TreeItem = null
	for employee in staff:
		var row := staff_tree.create_item(root)
		row.set_metadata(0, employee)
		var values := [employee["name"], employee["role_name"], UI.money(employee["salary_eur"]), employee["status_text"], "%s (%d Tage)" % [employee["hired"], employee["days_employed"]]]
		for i in range(values.size()):
			row.set_text(i, values[i])
		row.set_custom_color(3, UI.MUTED if employee["status"] == "available" else UI.ACCENT)
		if employee["id"] == selected_id:
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
		return
	var employee: Dictionary = row.get_metadata(0)
	selected_id = employee["id"]
	dismiss_button.disabled = not employee["can_dismiss"]
	dismiss_button.tooltip_text = "" if employee["can_dismiss"] else "Im Einsatz: erst Flugplan beenden bzw. Wartung abwarten."

func _report(error: String, success: String) -> void:
	action_status.text = success if error.is_empty() else error
	action_status.add_theme_color_override("font_color", UI.POSITIVE if error.is_empty() else UI.NEGATIVE)
	if error.is_empty():
		changed.emit()
	refresh(true)

func _hire() -> void:
	var role: String = role_keys[role_select.selected]
	var count := int(count_spin.value)
	_report(simulation.hire_staff(role, count), "%d Mitarbeiter eingestellt. Bitte Spielstand speichern." % count)

func _hire_missing() -> void:
	_report(simulation.hire_missing_staff(), "Fehlendes Personal eingestellt. Bitte Spielstand speichern.")

func _dismiss() -> void:
	if selected_id == 0:
		return
	_report(simulation.dismiss_staff(selected_id), "Mitarbeiter entlassen · anteiliges Gehalt gezahlt.")
