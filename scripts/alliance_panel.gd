extends VBoxContainer
## Rust supplies alliances, eligibility and exact refusal reasons; UI forwards commands.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var simulation: RefCounted
var choice: OptionButton
var table: Tree
var chip: Control
var members: Label
var reason: Label
var result: Label
var apply: Button
var leave: Button
var rules_label: Label
var conditions_label: Label
var rows: Array = []
var signature := ""
var dialog: Control

func _ready() -> void:
	var body := UI.panel(self, tr("ALLIANCE_TITLE"))
	var hint := UI.label(tr("ALLIANCE_HINT"), Tokens.SMALL, UI.MUTED)
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(hint)
	table = UI.table([tr("ALLIANCE_COL_NAME"), tr("ALLIANCE_COL_MEMBERS"), tr("ALLIANCE_COL_STATUS")], [150, 90, 170], 156)
	body.add_child(table)
	rules_label = UI.label("", Tokens.SMALL, UI.MUTED)
	rules_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(rules_label)
	conditions_label = UI.label("", Tokens.SMALL, UI.MUTED)
	conditions_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(conditions_label)
	choice = OptionButton.new()
	choice.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	choice.item_selected.connect(func(_index: int): _selection())
	body.add_child(choice)
	chip = UI.status_chip(tr("ALLIANCE_STATE_NONE"), "neutral")
	body.add_child(chip)
	members = UI.label("")
	members.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(members)
	reason = UI.label("")
	reason.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(reason)
	var actions := HFlowContainer.new()
	body.add_child(actions)
	apply = Button.new()
	apply.text = tr("ALLIANCE_APPLY")
	apply.theme_type_variation = "PrimaryButton"
	apply.pressed.connect(_confirm_apply)
	actions.add_child(apply)
	leave = Button.new()
	leave.text = tr("ALLIANCE_LEAVE")
	leave.theme_type_variation = "DangerButton"
	leave.pressed.connect(_confirm_leave)
	actions.add_child(leave)
	result = UI.label("")
	result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.add_child(result)
	refresh()

func _status(row: Dictionary) -> String:
	match row["state"]:
		"member": return tr("ALLIANCE_STATE_MEMBER")
		"pending": return tr("ALLIANCE_STATE_PENDING")
		"leaving": return tr("ALLIANCE_STATE_LEAVING") % row["notice_ends"]
	return tr("ALLIANCE_STATE_NONE")

func refresh() -> void:
	if table == null: return
	var data: Array = simulation.get_alliances()
	var rules: Dictionary = simulation.get_alliance_rules()
	var next := str([data, rules])
	if signature == next: return
	signature = next
	var selected: String = choice.get_selected_metadata() if choice.selected >= 0 else ""
	rows = data
	choice.clear()
	table.clear()
	var root := table.create_item()
	for row: Dictionary in rows:
		choice.add_item(row["name"])
		choice.set_item_metadata(choice.item_count - 1, row["id"])
		if row["id"] == selected: choice.select(choice.item_count - 1)
		var item := table.create_item(root)
		UI.cell_text(item, 0, row["name"])
		UI.cell_text(item, 1, str(row["member_count"]))
		UI.cell_text(item, 2, _status(row))
	rules_label.text = tr("ALLIANCE_RULES") % [rules["next_review"], UI.money(rules["fee_eur"]), rules["notice_days"], rules["cooldown_days"]]
	conditions_label.text = tr("ALLIANCE_CONDITIONS") % [rules["min_routes"], rules["max_overlap"], rules["min_cash_months"], rules["min_reputation"], rules["max_members"]] + " " + tr("ALLIANCE_BONUS") % [rules["bonus_bps"], rules["bonus_max_bps"]]
	_selection()

func _selection() -> void:
	var row: Dictionary = rows[choice.selected] if choice.selected >= 0 else {}
	var state: String = row.get("state", "none")
	var parent := chip.get_parent()
	var index := chip.get_index()
	parent.remove_child(chip)
	chip.queue_free()
	var kind := "go" if state == "member" else ("caution" if state == "leaving" or state == "pending" else "neutral")
	chip = UI.status_chip(_status(row) if not row.is_empty() else tr("ALLIANCE_STATE_NONE"), kind)
	parent.add_child(chip)
	parent.move_child(chip, index)
	var names := PackedStringArray()
	for m: Dictionary in row.get("members", []):
		names.append(m["name"])
	members.text = tr("ALLIANCE_MEMBER_LINE") % (", ".join(names) if not names.is_empty() else tr("ALLIANCE_NO_MEMBERS"))
	if row.get("qualifies", false):
		reason.text = tr("ALLIANCE_QUALIFIES")
	elif row.get("reason", "") != "":
		reason.text = tr(row["reason"]) % row["reason_args"] if not row["reason_args"].is_empty() else tr(row["reason"])
	else:
		reason.text = ""
	apply.disabled = row.is_empty() or not row.get("qualifies", false)
	leave.text = tr("ALLIANCE_WITHDRAW") if state == "pending" else tr("ALLIANCE_LEAVE")
	leave.disabled = not (state == "member" or state == "pending")

func _command(leaving: bool, withdrawing: bool = false) -> void:
	var error: String = simulation.leave_alliance() if leaving else simulation.apply_alliance(choice.get_selected_metadata())
	if not error.is_empty():
		result.text = tr(error)
	elif withdrawing:
		result.text = tr("ALLIANCE_WITHDRAWN")
	else:
		result.text = tr("ALLIANCE_LEFT" if leaving else "ALLIANCE_APPLIED")
	refresh()
	get_parent().refresh_operations()

func _confirm_apply() -> void:
	dialog = UI.dialog(self, tr("ALLIANCE_APPLY_QUESTION") % choice.get_item_text(choice.selected), tr("ALLIANCE_APPLY_HINT"), tr("ALLIANCE_CANCEL"), tr("ALLIANCE_APPLY"), func(): _command(false))
	dialog.popup_centered()

func _confirm_leave() -> void:
	var row: Dictionary = rows[choice.selected]
	var withdrawing: bool = row["state"] == "pending"
	var question: String = tr("ALLIANCE_WITHDRAW_QUESTION") if withdrawing else tr("ALLIANCE_LEAVE_QUESTION") % row["name"]
	var consequence: String = "" if withdrawing else tr("ALLIANCE_LEAVE_HINT")
	dialog = UI.dialog(self, question, consequence, tr("ALLIANCE_CANCEL"), leave.text, func(): _command(true, withdrawing))
	dialog.popup_centered()
