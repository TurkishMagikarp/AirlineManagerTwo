extends VBoxContainer
## Manufacturer quotes, deadlines and payments are supplied by Rust.
signal changed
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var simulation: RefCounted
var model: OptionButton
var quantity: SpinBox
var preview: Label
var place: Button
var cancel: Button
var table: Tree
var catalog_title: Label
var catalog: Tree
var catalog_signature := ""
var empty: Control
var result: Label
var selection_details: Label
var dialog: Control
var rows: Array = []
var signature := ""
var quote: Dictionary = {}
var selected_id := 0

func _ready() -> void:
	add_theme_constant_override("separation", Tokens.SPACE_4)
	var body := UI.panel(self, tr("ORDER_TITLE"))
	var hint := UI.label(tr("ORDER_HINT"), Tokens.SMALL, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(hint)
	body.add_child(UI.label(tr("ORDER_MODEL"), Tokens.LABEL, UI.MUTED))
	model = OptionButton.new()
	model.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	model.fit_to_longest_item = false
	model.clip_text = true
	model.item_selected.connect(func(_index: int): _preview())
	body.add_child(model)
	body.add_child(UI.label(tr("ORDER_QUANTITY"), Tokens.LABEL, UI.MUTED))
	quantity = SpinBox.new()
	quantity.min_value = 1
	quantity.max_value = simulation.get_order_limit()
	quantity.step = 1
	quantity.get_line_edit().theme_type_variation = "NumericInput"
	quantity.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	quantity.value_changed.connect(func(_value: float): _preview())
	body.add_child(quantity)
	preview = UI.label("")
	preview.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(preview)
	place = Button.new()
	place.text = tr("ORDER_PLACE")
	place.theme_type_variation = "PrimaryButton"
	place.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	place.pressed.connect(_confirm_order)
	body.add_child(place)
	result = UI.label("")
	result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(result)
	catalog_title = UI.label(tr("ORDER_AVAILABILITY"), Tokens.LABEL, UI.MUTED)
	body.add_child(catalog_title)
	catalog = UI.table([tr("ORDER_MODEL"), tr("ORDER_STATUS")], [150, 190], 120)
	body.add_child(catalog)
	var open := UI.panel(self, tr("ORDER_OPEN"))
	empty = UI.empty_state(tr("ORDER_EMPTY"), tr("ORDER_EMPTY_HINT"), tr("ORDER_SELECT"), func(): model.grab_focus())
	open.add_child(empty)
	table = UI.table([tr("ORDER_MODEL"), tr("ORDER_DATE"), tr("ORDER_BALANCE"), tr("ORDER_STATUS")], [110, 145, 125, 105], 180)
	table.item_selected.connect(_selection)
	open.add_child(table)
	selection_details = UI.label("")
	selection_details.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	open.add_child(selection_details)
	cancel = Button.new()
	cancel.text = tr("ORDER_CANCEL")
	cancel.theme_type_variation = "DangerButton"
	cancel.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	cancel.pressed.connect(_confirm_cancel)
	open.add_child(cancel)
	resized.connect(_resize_table)
	_resize_table()
	refresh()

func _refresh_catalog() -> void:
	var models: Array = simulation.get_aircraft_models()
	var next := str(models.map(func(m: Dictionary): return [m["id"], m["availability"]]))
	if next == catalog_signature: return
	catalog_signature = next
	var selected_id = model.get_selected_metadata() if model.selected >= 0 else null
	model.clear()
	catalog.clear()
	var root := catalog.create_item()
	for entry: Dictionary in models:
		var text: String = entry["name"]
		if entry["availability"] == "upcoming":
			text += " · " + tr("ORDER_FROM") % entry["available_from"]
			var row := catalog.create_item(root)
			UI.cell_text(row, 0, entry["name"])
			UI.cell_status(row, 1, tr("ORDER_FROM") % entry["available_from"], "info")
			row.set_tooltip_text(1, tr("ORDER_UPCOMING_TIP"))
		elif entry["availability"] == "out_of_production":
			text += " · " + tr("ORDER_EOP")
			var row := catalog.create_item(root)
			UI.cell_text(row, 0, entry["name"])
			UI.cell_status(row, 1, tr("ORDER_EOP"), "caution")
			row.set_tooltip_text(1, tr("ORDER_EOP_TIP"))
		model.add_item(text)
		model.set_item_metadata(model.item_count - 1, entry["id"])
		if entry["id"] == selected_id: model.select(model.item_count - 1)
	if model.selected < 0 and model.item_count > 0: model.select(0)
	var any := root.get_first_child() != null
	catalog.custom_minimum_size.y = 36 + 40 * mini(root.get_child_count(), 6)
	catalog_title.visible = any
	catalog.visible = any
	_preview()

func refresh() -> void:
	if table == null: return
	_refresh_catalog()
	var data: Array = simulation.get_orders()
	var next := str([data, simulation.get_airline_info().get("capital_eur", 0)])
	if signature == next: return
	signature = next
	rows = data
	table.clear()
	var root := table.create_item()
	for entry: Dictionary in rows:
		var row := table.create_item(root)
		UI.cell_text(row, 0, entry["model"])
		UI.cell_text(row, 1, entry["delivery_date"])
		UI.cell_text(row, 2, UI.money(entry["balance"]))
		UI.cell_status(row, 3, tr(entry["status"]), "caution" if entry["status"] == "ORDER_HELD" else "info")
		row.set_tooltip_text(3, tr("ORDER_GRACE") % entry["grace_deadline"])
		row.set_metadata(0, entry["id"])
		if entry["id"] == selected_id: row.select(0)
	if table.get_selected() == null and root.get_first_child() != null:
		root.get_first_child().select(0)
	empty.visible = rows.is_empty()
	table.visible = not rows.is_empty()
	cancel.visible = not rows.is_empty()
	_selection()
	_preview()

func _selection() -> void:
	var selected := table.get_selected()
	selected_id = selected.get_metadata(0) if selected != null else 0
	selection_details.text = ""
	for entry: Dictionary in rows:
		if entry["id"] == selected_id:
			selection_details.text = tr("ORDER_SELECTION") % UI.money(entry["deposit"])
			selection_details.text += "\n" + tr("ORDER_GRACE") % entry["grace_deadline"]
	selection_details.visible = selected_id != 0
	cancel.disabled = selected_id == 0
	cancel.tooltip_text = tr("ORDER_SELECT_ROW") if cancel.disabled else tr("ORDER_LOSS_HINT")

func _preview() -> void:
	if preview == null or model.selected < 0: return
	quote = simulation.quote_order(model.get_selected_metadata(), int(quantity.value))
	var error: String = quote.get("error", "")
	place.disabled = not error.is_empty() or not quote.get("can_afford", false)
	place.tooltip_text = error if not error.is_empty() else tr("ORDER_NO_CASH") if place.disabled else ""
	if not error.is_empty():
		preview.text = tr(error)
		return
	preview.text = tr("ORDER_QUOTE") % [UI.money(quote["unit_price"]), UI.money(quote["total_deposit"]), UI.money(quote["balance"])]
	preview.text += "\n" + tr("ORDER_DELIVERIES") + "\n" + " · ".join(quote["delivery_dates"])
	if place.disabled: preview.text += "\n" + place.tooltip_text

func _confirm_order() -> void:
	_preview()
	if place.disabled: return
	var model_id: String = model.get_selected_metadata()
	var count := int(quantity.value)
	var confirmation := tr("ORDER_QUOTE") % [UI.money(quote["unit_price"]), UI.money(quote["total_deposit"]), UI.money(quote["balance"])]
	confirmation += "\n" + tr("ORDER_DELIVERY_WINDOW") % [quote["delivery_dates"][0], quote["delivery_dates"][-1]]
	dialog = UI.dialog(self, tr("ORDER_CONFIRM") % [count, model.get_item_text(model.selected)], confirmation + "\n" + tr("ORDER_LOSS_HINT"), tr("ORDER_BACK"), tr("ORDER_PLACE"), func():
		var error: String = simulation.place_order(model_id, count)
		result.text = tr("ORDER_PLACED") if error.is_empty() else tr(error)
		refresh()
		changed.emit())
	dialog.get_ok_button().theme_type_variation = "PrimaryButton"
	dialog.popup_centered()

func _confirm_cancel() -> void:
	var selected: Dictionary = {}
	for entry: Dictionary in rows:
		if entry["id"] == selected_id: selected = entry
	if selected.is_empty(): return
	var order_id := selected_id
	dialog = UI.dialog(self, tr("ORDER_CANCEL_CONFIRM") % selected["model"], tr("ORDER_CANCEL_LOSS") % UI.money(selected["deposit"]), tr("ORDER_BACK"), tr("ORDER_CANCEL"), func():
		var error: String = simulation.cancel_order(order_id)
		result.text = tr("ORDER_CANCELLED") if error.is_empty() else tr(error)
		refresh()
		changed.emit())
	dialog.popup_centered()

func select_order(order_id: int) -> void:
	if table.get_root() == null: return
	for row in table.get_root().get_children():
		if row.get_metadata(0) == order_id:
			row.select(0)
			table.ensure_cursor_is_visible()
			_selection()
			return

func _resize_table() -> void:
	if table != null:
		table.custom_minimum_size.y = 140 if size.x < 700 else 180
