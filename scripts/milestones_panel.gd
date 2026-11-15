extends VBoxContainer
## Presentation only; dates, progress and notification delivery belong to Rust.
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var simulation: RefCounted
var body: VBoxContainer
var signature := ""
var notification: CanvasLayer
var action: Button
var heading: Label

func setup(sim: RefCounted) -> void:
	simulation = sim
	action = Button.new()
	action.text = tr("MILESTONES_VIEW")
	action.pressed.connect(open_list)
	body = UI.panel(self, tr("MILESTONES_TITLE"), action)
	body.add_theme_constant_override("separation", Tokens.SPACE_2)
	heading = body.get_meta("panel_shell").get_child(0).get_child(0).get_child(0).get_child(0)

func date_text(seconds: int) -> String:
	return tr("MILESTONES_DATE") % [seconds / 86400 + 1, seconds / 3600 % 24, seconds / 60 % 60]

func title(entry: Dictionary) -> String:
	return tr("MILESTONE_" + str(entry.id).to_upper())

func refresh() -> void:
	var entries: Array = simulation.get_milestones()
	var key := str(entries) + TranslationServer.get_locale()
	if signature == key: return
	signature = key
	heading.text = tr("MILESTONES_TITLE")
	action.text = tr("MILESTONES_VIEW")
	for child in body.get_children():
		body.remove_child(child)
		child.queue_free()
	var reached: Array = entries.filter(func(e: Dictionary): return e.reached >= 0)
	reached.sort_custom(func(a: Dictionary, b: Dictionary): return a.reached > b.reached)
	if reached.is_empty():
		var empty := UI.EmptyFrame.new()
		body.add_child(empty)
		var hint := UI.label(tr("MILESTONES_EMPTY"), Tokens.SMALL, UI.MUTED)
		hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		empty.add_child(hint)
	for entry in reached.slice(0, 2):
		add_entry(body, entry)
	var open: Array = entries.filter(func(e: Dictionary): return e.reached < 0)
	for entry in open.slice(0, 2):
		add_entry(body, entry)

func add_entry(parent: Node, entry: Dictionary) -> void:
	var block := VBoxContainer.new()
	block.add_theme_constant_override("separation", Tokens.SPACE_1)
	parent.add_child(block)
	var caption := UI.label(title(entry), Tokens.SMALL)
	caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	block.add_child(caption)
	if entry.reached >= 0:
		var row := HBoxContainer.new()
		block.add_child(row)
		row.add_child(UI.status_chip(tr("MILESTONES_REACHED"), "go"))
		row.add_child(UI.label(date_text(entry.reached), Tokens.CODE, UI.MUTED))
	else:
		var progress := UI.progress(clampf(float(entry.current) / float(entry.target) * 100.0, 0, 100))
		progress.get_child(1).text = "%d / %d" % [entry.current, entry.target]
		block.add_child(progress)

func notify_reached() -> void:
	var ids: PackedStringArray = simulation.take_milestone_notifications()
	if ids.is_empty(): return
	var titles := PackedStringArray()
	for id in ids: titles.append(tr("MILESTONE_" + id.to_upper()))
	if is_instance_valid(notification): notification.queue_free()
	notification = UI.toast(self, tr("MILESTONES_TOAST") % "; ".join(titles), true)

func open_list() -> void:
	var window := Window.new()
	window.title = tr("MILESTONES_TITLE")
	window.size = Vector2i(mini(600, get_viewport_rect().size.x - 32), mini(640, get_viewport_rect().size.y - 64))
	window.transient = true
	window.exclusive = true
	window.theme = theme
	add_child(window)
	var layout := VBoxContainer.new()
	layout.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	window.add_child(layout)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	layout.add_child(scroll)
	var content := VBoxContainer.new()
	content.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	content.add_theme_constant_override("separation", Tokens.SPACE_4)
	scroll.add_child(content)
	var panel := UI.panel(content, tr("MILESTONES_TITLE"))
	for entry in simulation.get_milestones(): add_entry(panel, entry)
	var close := Button.new()
	close.text = tr("MILESTONES_CLOSE")
	close.pressed.connect(window.queue_free)
	layout.add_child(close)
	window.close_requested.connect(window.queue_free)
	window.popup_centered()
	close.grab_focus()

func reset_view() -> void:
	if is_instance_valid(notification): notification.queue_free()
	notification = null
	for child in get_children():
		if child is Window: child.queue_free()
	signature = ""
