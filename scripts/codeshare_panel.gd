extends VBoxContainer
## Rust supplies eligibility, status and settled receipts; UI forwards commands.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var simulation: RefCounted
var partner: OptionButton
var table: Tree
var chip: Control
var reason: Label
var result: Label
var propose: Button
var end: Button
var empty: Control
var rows: Array = []
var signature := ""
var dialog: Control

func _ready() -> void:
	var body := UI.panel(self, tr("CODESHARE_TITLE"))
	var hint := UI.label(tr("CODESHARE_HINT"), Tokens.SMALL, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(hint)
	empty = UI.empty_state(tr("CODESHARE_EMPTY"), tr("CODESHARE_EMPTY_HINT"), tr("CODESHARE_SELECT"), func(): partner.grab_focus())
	body.add_child(empty)
	table = UI.table([tr("CODESHARE_PARTNER"), tr("CODESHARE_PAX"), tr("CODESHARE_REVENUE")], [170, 120, 150], 160)
	body.add_child(table)
	partner = OptionButton.new()
	partner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	partner.item_selected.connect(func(_index: int): _selection())
	body.add_child(partner)
	chip = UI.status_chip(tr("CODESHARE_AVAILABLE"), "neutral")
	body.add_child(chip)
	reason = UI.label("")
	reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(reason)
	var actions := HFlowContainer.new()
	body.add_child(actions)
	propose = Button.new()
	propose.text = tr("CODESHARE_PROPOSE")
	propose.theme_type_variation = "PrimaryButton"
	propose.pressed.connect(func(): _command(false))
	actions.add_child(propose)
	end = Button.new()
	end.text = tr("CODESHARE_END")
	end.theme_type_variation = "DangerButton"
	end.pressed.connect(_confirm_end)
	actions.add_child(end)
	result = UI.label("")
	result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(result)
	refresh()

func refresh() -> void:
	if table == null: return
	var data: Array = simulation.get_codeshares()
	var next := str(data)
	if signature == next: return
	signature = next
	var selected: String = partner.get_selected_metadata() if partner.selected >= 0 else ""
	rows = data
	partner.clear()
	table.clear()
	var root := table.create_item()
	var active := false
	for row: Dictionary in rows:
		partner.add_item(row["name"])
		partner.set_item_metadata(partner.item_count - 1, row["partner"])
		if row["partner"] == selected: partner.select(partner.item_count - 1)
		active = active or row["active"]
		if row["active"] or row["passengers"] > 0:
			var item := table.create_item(root)
			UI.cell_text(item, 0, row["name"])
			UI.cell_text(item, 1, str(row["passengers"]))
			UI.cell_text(item, 2, UI.money(row["revenue"]))
	empty.visible = not active
	table.visible = root.get_first_child() != null
	_selection()

func _selection() -> void:
	var row: Dictionary = rows[partner.selected] if partner.selected >= 0 else {}
	var active: bool = row.get("active", false)
	var parent := chip.get_parent()
	var index := chip.get_index()
	parent.remove_child(chip)
	chip.queue_free()
	chip = UI.status_chip(tr("CODESHARE_ACTIVE") if active else tr("CODESHARE_AVAILABLE"), "go" if active else "neutral")
	parent.add_child(chip)
	parent.move_child(chip, index)
	reason.text = tr(row.get("reason", ""))
	# EmptyState already owns the panel's primary action.
	propose.theme_type_variation = "" if empty.visible else "PrimaryButton"
	propose.disabled = row.is_empty() or active
	end.disabled = not active

func _command(ending: bool) -> void:
	var key: String = partner.get_selected_metadata()
	var error: String = simulation.end_codeshare(key) if ending else simulation.propose_codeshare(key)
	result.text = tr(error) if not error.is_empty() else tr("CODESHARE_ENDED" if ending else "CODESHARE_SIGNED")
	refresh()
	get_parent().refresh_operations()

func _confirm_end() -> void:
	dialog = UI.dialog(self, tr("CODESHARE_END_QUESTION") % partner.get_item_text(partner.selected), tr("CODESHARE_END_HINT"), tr("CODESHARE_CANCEL"), tr("CODESHARE_END"), func(): _command(true))
	dialog.popup_centered()
