extends VBoxContainer
## Charter offers and jobs. Rules, quotes and refusal reasons come from Rust.
signal changed
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var simulation: RefCounted
var offers_table: Tree
var jobs_table: Tree
var offers_empty: Control
var jobs_empty: Control
var aircraft: OptionButton
var accept: Button
var cancel: Button
var details: Label
var result: Label
var dialog: Control
var offers: Array = []
var jobs: Array = []
var options: Array = []
var signature := ""
var selected_offer := -1
var selected_job := -1

func _ready() -> void:
	add_theme_constant_override("separation", Tokens.SPACE_4)
	var body := UI.panel(self, "Charterangebote")
	var hint := UI.label("Einmalige Aufträge für freie Flugzeuge: feste Vergütung, bei Scheitern eine Vertragsstrafe.", Tokens.SMALL, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(hint)
	offers_empty = UI.empty_state("Keine Charterangebote", "Die Welt macht neue Angebote, sobald sich Nachfrage und Kalender ändern.", "Aktualisieren", func(): refresh(true))
	body.add_child(offers_empty)
	offers_table = UI.table(["Auftrag", "Strecke", "Menge", "Vergütung", "Fenster", "Angebot bis"], [120, 130, 80, 110, 230, 130], 170)
	offers_table.item_selected.connect(_offer_selected)
	body.add_child(offers_table)
	aircraft = OptionButton.new()
	aircraft.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	aircraft.fit_to_longest_item = false
	aircraft.clip_text = true
	aircraft.item_selected.connect(func(_i: int): _aircraft_changed())
	UI.field(body, "Flugzeug", aircraft)
	details = UI.label("")
	details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(details)
	accept = Button.new()
	accept.text = "Charter annehmen…"
	accept.theme_type_variation = "PrimaryButton"
	accept.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	accept.pressed.connect(_confirm_accept)
	body.add_child(accept)
	result = UI.label("")
	result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(result)
	var open := UI.panel(self, "Angenommene Charteraufträge")
	jobs_empty = UI.empty_state("Keine Charteraufträge", "Angenommene und kürzlich beendete Aufträge erscheinen hier.", "Angebote ansehen", func(): offers_table.grab_focus())
	open.add_child(jobs_empty)
	jobs_table = UI.table(["Status", "Strecke", "Flugzeug", "Vergütung", "Start / Ende", "Frist / Grund"], [100, 150, 90, 100, 140, 150], 140)
	jobs_table.item_selected.connect(_job_selected)
	open.add_child(jobs_table)
	cancel = Button.new()
	cancel.text = "Auftrag stornieren…"
	cancel.theme_type_variation = "DangerButton"
	cancel.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	cancel.pressed.connect(_confirm_cancel)
	open.add_child(cancel)
	refresh(true)

func refresh(force: bool = false) -> void:
	if offers_table == null: return
	var data_offers: Array = simulation.get_charter_offers()
	var data_jobs: Array = simulation.get_charter_jobs()
	var next := str([data_offers, data_jobs, simulation.get_fleet().size()])
	if signature == next and not force: return
	signature = next
	offers = data_offers
	jobs = data_jobs
	offers_table.clear()
	var root := offers_table.create_item()
	for entry: Dictionary in offers:
		var row := offers_table.create_item(root)
		UI.cell_text(row, 0, entry["kind_name"])
		UI.cell_text(row, 1, "%s → %s" % [entry["origin"], entry["destination"]])
		UI.cell_text(row, 2, ("%d t" if entry["freight"] else "%d Pax") % entry["quantity"])
		UI.cell_text(row, 3, UI.money(entry["fee_eur"]))
		UI.cell_text(row, 4, "%s – %s" % [_day(entry["window_start"]), _day(entry["window_end"])])
		UI.cell_text(row, 5, _day(entry["expires"]))
		row.set_metadata(0, entry["id"])
		if entry["id"] == selected_offer: row.select(0)
	if offers_table.get_selected() == null and root.get_first_child() != null:
		root.get_first_child().select(0)
		selected_offer = root.get_first_child().get_metadata(0)
	offers_empty.visible = offers.is_empty()
	offers_table.visible = not offers.is_empty()
	jobs_table.clear()
	var jroot := jobs_table.create_item()
	for entry: Dictionary in jobs:
		var row := jobs_table.create_item(jroot)
		var kind := "info"
		match entry["state"]:
			"delivered": kind = "go"
			"failed": kind = "stop"
			"scheduled": kind = "neutral"
		UI.cell_status(row, 0, entry["state_text"], kind)
		UI.cell_text(row, 1, entry["route"])
		UI.cell_text(row, 2, entry["aircraft"])
		UI.cell_text(row, 3, UI.signed_money(entry["fee_eur"]) if entry["state"] in ["delivered", "failed"] else UI.money(entry["fee_eur"]))
		UI.cell_text(row, 4, entry["departure"])
		UI.cell_text(row, 5, entry["deadline"])
		row.set_metadata(0, entry["id"])
		if entry["id"] == selected_job: row.select(0)
	jobs_empty.visible = jobs.is_empty()
	jobs_table.visible = not jobs.is_empty()
	_offer_selected()
	_job_selected()

func _day(text: String) -> String:
	return text.trim_suffix(" 00:00:00")

func _offer_selected() -> void:
	var item := offers_table.get_selected()
	if item != null: selected_offer = item.get_metadata(0)
	elif offers.is_empty(): selected_offer = -1
	var previous: int = aircraft.get_selected_metadata() if aircraft.selected >= 0 else -1
	aircraft.clear()
	options = simulation.get_charter_aircraft_options(selected_offer) if selected_offer >= 0 else []
	for option: Dictionary in options:
		aircraft.add_item(option["label"] if option["ok"] else "%s – nicht geeignet" % option["label"])
		aircraft.set_item_metadata(aircraft.item_count - 1, option["aircraft_id"])
		if option["aircraft_id"] == previous: aircraft.select(aircraft.item_count - 1)
	if aircraft.item_count > 0 and aircraft.selected < 0: aircraft.select(0)
	_aircraft_changed()

func _aircraft_changed() -> void:
	details.text = ""
	accept.disabled = true
	if selected_offer < 0 or aircraft.selected < 0 or aircraft.item_count == 0:
		accept.tooltip_text = "Wähle ein Angebot und ein Flugzeug."
		details.text = "Die Flotte hat kein Flugzeug für dieses Angebot." if selected_offer >= 0 else ""
		return
	var preview: Dictionary = simulation.preview_charter(selected_offer, aircraft.get_selected_metadata())
	var error: String = preview["error"]
	accept.disabled = not error.is_empty()
	accept.tooltip_text = error
	details.text = error if not error.is_empty() else "Nettoergebnis %s bei %s Kosten, Vertragsstrafe bei Scheitern %s." % [UI.signed_money(preview["net_eur"]), UI.money(preview["cost_eur"]), UI.money(preview["penalty_eur"])]

func _job_selected() -> void:
	var item := jobs_table.get_selected()
	selected_job = item.get_metadata(0) if item != null else -1
	var cancellable := false
	for entry: Dictionary in jobs:
		if entry["id"] == selected_job and entry["cancellable"]: cancellable = true
	cancel.visible = not jobs.is_empty()
	cancel.disabled = not cancellable
	cancel.tooltip_text = "" if cancellable else "Nur geplante Aufträge können storniert werden."

func _confirm_accept() -> void:
	if accept.disabled: return
	var offer_id := selected_offer
	var aircraft_id: int = aircraft.get_selected_metadata()
	var p: Dictionary = simulation.preview_charter(offer_id, aircraft_id)
	if not p["error"].is_empty():
		result.text = p["error"]
		return
	var lines := PackedStringArray()
	if p["has_positioning"]:
		lines.append("Überführung %s → Start (%d km): %s" % [p["positioning_from"], int(p["positioning_km"]), UI.money(p["positioning_eur"])])
	lines.append("Charterflug (%d km): %s" % [int(p["leg_km"]), UI.money(p["leg_eur"])])
	lines.append("Gesamtkosten: %s" % UI.money(p["cost_eur"]))
	lines.append("Vergütung: %s" % UI.money(p["fee_eur"]))
	lines.append("Ergebnis: %s" % UI.signed_money(p["net_eur"]))
	lines.append("Start %s · Ankunft %s · Frist %s" % [p["departure"], p["landing"], p["deadline"]])
	lines.append("Scheitern kostet %s Vertragsstrafe und Ruf." % UI.money(p["penalty_eur"]))
	dialog = UI.dialog(self, "Charter %s annehmen?" % p["route"], "\n".join(lines), "Zurück", "Annehmen", func():
		var error: String = simulation.accept_charter(offer_id, aircraft_id)
		result.text = "Charter angenommen." if error.is_empty() else error
		refresh(true)
		changed.emit())
	dialog.get_ok_button().theme_type_variation = "PrimaryButton"
	dialog.popup_centered()

func _confirm_cancel() -> void:
	var entry: Dictionary = {}
	for job: Dictionary in jobs:
		if job["id"] == selected_job: entry = job
	if entry.is_empty() or not entry["cancellable"]: return
	var offer_id := selected_job
	dialog = UI.dialog(self, "Charter %s stornieren?" % entry["route"], "Die Stornierung kostet %s Vertragsstrafe und Ruf." % UI.money(entry["penalty_eur"]), "Zurück", "Stornieren", func():
		var error: String = simulation.cancel_charter(offer_id)
		result.text = "Charter storniert." if error.is_empty() else error
		refresh(true)
		changed.emit())
	dialog.popup_centered()
