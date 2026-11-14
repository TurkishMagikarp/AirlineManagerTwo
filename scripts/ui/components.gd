extends RefCounted
## Small composable UI primitives; never reads or changes simulation state.

const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
const MUTED = ThemeTokens.INK_MUTED
const ACCENT = ThemeTokens.ACCENT_TEXT
const POSITIVE = ThemeTokens.GO
const NEGATIVE = ThemeTokens.STOP
static var _numeric: RegEx
static var _code: RegEx
const METRIC_MIN_WIDTH = 232
const DIALOG_TITLE = {"family": "display", "size": 24, "height": 28, "weight": 600, "tracking": 0.0}
const EMPTY_TITLE = {"family": "display", "size": 20, "height": 24, "weight": 600, "tracking": 0.0}
const CHIP_STYLE = {"family": "mono", "size": 11, "height": 11, "weight": 500, "tracking": 0.06}
const KINDS = ["go", "info", "neutral", "caution", "stop"]

## Keep the Label API used by panels and tests; its child renders styled runs.
## This observes presentation changes only, never advances simulation time.
class MetricValue extends Label:
	var rendered := RichTextLabel.new()
	var previous := ""
	var previous_color := Color.TRANSPARENT

	func _init() -> void:
		self_modulate = ThemeTokens.TRANSPARENT
		autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		rendered.bbcode_enabled = true
		rendered.fit_content = true
		rendered.scroll_active = false
		rendered.mouse_filter = Control.MOUSE_FILTER_IGNORE
		rendered.add_theme_font_override("normal_font", ThemeTokens.font_for(ThemeTokens.FIGURE))
		rendered.add_theme_font_size_override("normal_font_size", ThemeTokens.FIGURE["size"])
		add_child(rendered)
		rendered.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)

	func _process(_delta: float) -> void:
		var ink := get_theme_color("font_color")
		if ink == ThemeTokens.GO: ink = ThemeTokens.INK
		if text != previous or ink != previous_color:
			previous = text
			previous_color = ink
			var parts := text.rsplit(" ", true, 1)
			var value := text
			var unit := ""
			if parts.size() == 2 and parts[1] in ["EUR", "%", "Pax", "fehlen"]:
				value = parts[0]
				unit = " [font=res://resources/fonts/dmmono/DMMono-Regular.ttf][font_size=%d][color=%s]%s[/color][/font_size][/font]" % [ThemeTokens.DATA["size"], ThemeTokens.INK_MUTED.to_html(), parts[1]]
			rendered.text = "[color=%s]%s[/color]%s" % [ink.to_html(), value.replace("[", "[lb]"), unit]
		custom_minimum_size.y = rendered.get_content_height()

class Glyph extends Control:
	var kind := "neutral"
	var painter: Callable
	var ink: Color
	func _draw() -> void:
		painter.call(self, kind, Rect2(Vector2.ZERO, size), ink)

class LegTrack extends Control:
	var percent := 0.0
	var departed := true
	func _draw() -> void:
		var y := size.y / 2
		draw_line(Vector2(0, y), Vector2(size.x, y), ThemeTokens.LINE_STRONG, 2)
		if departed:
			var x := size.x * clampf(percent / 100, 0, 1)
			draw_line(Vector2(0, y), Vector2(x, y), ThemeTokens.ACCENT_TEXT, 2)
			draw_colored_polygon(PackedVector2Array([Vector2(x, y - 5), Vector2(x + 5, y), Vector2(x, y + 5), Vector2(x - 5, y)]), ThemeTokens.ACCENT_TEXT)

class EmptyFrame extends PanelContainer:
	func _draw() -> void:
		var ink := ThemeTokens.LINE_STRONG
		draw_dashed_line(Vector2.ZERO, Vector2(size.x, 0), ink, 1, 4)
		draw_dashed_line(Vector2(0, size.y), size, ink, 1, 4)
		draw_dashed_line(Vector2.ZERO, Vector2(0, size.y), ink, 1, 4)
		draw_dashed_line(Vector2(size.x, 0), size, ink, 1, 4)


static func label(text: String, style: Dictionary = ThemeTokens.BODY, color: Color = ThemeTokens.INK) -> Label:
	var result := Label.new()
	result.text = text
	result.add_theme_font_override("font", ThemeTokens.font_for(style))
	result.add_theme_font_size_override("font_size", style["size"])
	result.add_theme_constant_override("line_spacing", 0)
	result.add_theme_color_override("font_color", color)
	return result

## Classify rendered table text only; values and units still come from Rust.
static func cell_text(item: TreeItem, column: int, text: String) -> void:
	item.set_custom_minimum_height(ThemeTokens.BODY["height"])
	item.set_text(column, text)
	item.set_tooltip_text(column, text)
	if _numeric == null:
		_numeric = RegEx.new()
		_numeric.compile("^[−+0-9]|[0-9] (EUR|%|km|kg)$")
		_code = RegEx.new()
		_code.compile("^[A-Z0-9#– →-]+$")
	var numeric := _numeric.search(text) != null or item.get_tree().get_column_title(column) in [
		"Flüge", "Tickets", "Kosten", "Ergebnis", "Auslastung", "Fracht kg / EUR",
		"Beschäftigt", "Im Einsatz", "Verfügbar", "Fehlt", "Berechtigt", "Piloten",
		"Flugbegleiter", "Techniker (Wartung)", "Gehalt / Monat", "Kosten / Monat",
		"Gehalt je Person / Monat", "Pax / Sitze", "Pax / Auslastung"
	]
	var code := not text.is_empty() and (_code.search(text) != null and (not numeric or text.begins_with("#")) or item.get_tree().get_column_title(column) in ["Muster", "Kennung", "Kennzeichen", "Route", "Route / Strecke", "Strecke", "Richtung"])
	var style: Dictionary = ThemeTokens.CODE if code else ThemeTokens.DATA if numeric else ThemeTokens.BODY
	item.set_custom_font(column, ThemeTokens.font_for(style))
	item.set_custom_font_size(column, style["size"])
	item.set_text_alignment(column, HORIZONTAL_ALIGNMENT_RIGHT if numeric and not code else HORIZONTAL_ALIGNMENT_LEFT)
	if numeric and not code:
		item.get_tree().set_column_title_alignment(column, HORIZONTAL_ALIGNMENT_RIGHT)

static func panel(parent: Node, title: String, accessory: Control = null, flush: bool = false) -> VBoxContainer:
	var shell := PanelContainer.new()
	shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	shell.add_theme_stylebox_override("panel", ThemeTokens.box(ThemeTokens.SURFACE, ThemeTokens.LINE, ThemeTokens.RADIUS_LG, 0, 0))
	parent.add_child(shell)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", 0)
	shell.add_child(column)
	if not title.is_empty():
		var header := PanelContainer.new()
		var style := ThemeTokens.box(ThemeTokens.TRANSPARENT, ThemeTokens.LINE, 0, ThemeTokens.SPACE_4, ThemeTokens.SPACE_2)
		style.set_border_width_all(0)
		style.border_width_bottom = 1
		header.add_theme_stylebox_override("panel", style)
		header.custom_minimum_size.y = 48
		column.add_child(header)
		var row := HBoxContainer.new()
		header.add_child(row)
		var heading := label(title, ThemeTokens.PANEL_TITLE)
		heading.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		heading.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(heading)
		if accessory != null:
			if accessory is Button: accessory.theme_type_variation = "PanelActionButton"
			accessory.size_flags_vertical = Control.SIZE_SHRINK_CENTER
			row.add_child(accessory)
	var padding := MarginContainer.new()
	var inset := 0 if flush else ThemeTokens.SPACE_4
	for side in ["left", "top", "right", "bottom"]:
		padding.add_theme_constant_override("margin_" + side, inset)
	column.add_child(padding)
	var body := VBoxContainer.new()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	padding.add_child(body)
	body.set_meta("panel_shell", shell)
	return body

static func metric(parent: Node, title: String, support: Control = null) -> Label:
	var shell := PanelContainer.new()
	shell.custom_minimum_size.x = METRIC_MIN_WIDTH
	shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(shell)
	var column := VBoxContainer.new()
	column.add_theme_constant_override("separation", ThemeTokens.SPACE_2)
	shell.add_child(column)
	var caption := label(title, ThemeTokens.LABEL, MUTED)
	caption.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	column.add_child(caption)
	var value := MetricValue.new()
	value.text = "—"
	value.add_theme_font_override("font", ThemeTokens.font_for(ThemeTokens.FIGURE))
	value.add_theme_font_size_override("font_size", ThemeTokens.FIGURE["size"])
	column.add_child(value)
	if support != null: column.add_child(support)
	return value

static func kind_color(kind: String) -> Color:
	return {"go": ThemeTokens.GO, "info": ThemeTokens.SKY, "neutral": MUTED, "caution": ThemeTokens.CAUTION, "stop": NEGATIVE}.get(kind, MUTED)

static func kind_wash(kind: String) -> Color:
	return {"go": ThemeTokens.GO_WASH, "info": ThemeTokens.SKY_WASH, "neutral": ThemeTokens.SURFACE_RAISED, "caution": ThemeTokens.CAUTION_WASH, "stop": ThemeTokens.STOP_WASH}.get(kind, ThemeTokens.SURFACE_RAISED)

static func draw_glyph(canvas: CanvasItem, kind: String, rect: Rect2, ink: Color) -> void:
	var c := rect.get_center()
	match kind:
		"go": canvas.draw_circle(c, 4, ink)
		"info": canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(0, -4), c + Vector2(4, 0), c + Vector2(0, 4), c + Vector2(-4, 0)]), ink)
		"caution": canvas.draw_colored_polygon(PackedVector2Array([c + Vector2(0, -4), c + Vector2(4, 4), c + Vector2(-4, 4)]), ink)
		"stop": canvas.draw_rect(Rect2(c - Vector2(4, 4), Vector2(8, 8)), ink)
		_: canvas.draw_line(c - Vector2(4, 0), c + Vector2(4, 0), ink, 2)

static func status_chip(text: String, kind: String = "neutral") -> PanelContainer:
	assert(kind in KINDS)
	var result := PanelContainer.new()
	result.theme_type_variation = "Chip" + kind.capitalize()
	result.custom_minimum_size.y = 22
	result.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	result.add_theme_stylebox_override("panel", ThemeTokens.box(kind_wash(kind), ThemeTokens.TRANSPARENT, ThemeTokens.RADIUS_SM, ThemeTokens.SPACE_2, 0))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	result.add_child(row)
	var glyph := Glyph.new()
	glyph.kind = kind
	glyph.painter = draw_glyph
	glyph.ink = kind_color(kind)
	glyph.custom_minimum_size = Vector2(8, 8)
	row.add_child(glyph)
	row.add_child(label(text, CHIP_STYLE, kind_color(kind)))
	return result

## Native icon/text cells retain selection, accessibility and keyboard behaviour.
static var _chip_icons: Dictionary = {}
static func cell_status(item: TreeItem, column: int, text: String, kind: String) -> void:
	assert(kind in KINDS)
	item.set_cell_mode(column, TreeItem.CELL_MODE_STRING)
	item.set_text(column, text)
	item.set_tooltip_text(column, text)
	item.set_custom_minimum_height(ThemeTokens.BODY["height"])
	item.set_custom_font(column, ThemeTokens.font_for(CHIP_STYLE))
	item.set_custom_font_size(column, CHIP_STYLE["size"])
	item.set_custom_color(column, kind_color(kind))
	item.set_custom_bg_color(column, kind_wash(kind))
	if not _chip_icons.has(kind):
		var pixels := Image.create(8, 8, false, Image.FORMAT_RGBA8)
		pixels.fill(ThemeTokens.TRANSPARENT)
		for y in range(8):
			for x in range(8):
				var point := Vector2(x + 0.5, y + 0.5) - Vector2(4, 4)
				var filled := false
				match kind:
					"go": filled = point.length() <= 4
					"info": filled = absf(point.x) + absf(point.y) <= 4
					"neutral": filled = absf(point.y) < 1
					"caution": filled = point.y >= absf(point.x) * 2 - 4
					"stop": filled = true
				if filled: pixels.set_pixel(x, y, kind_color(kind))
		_chip_icons[kind] = ImageTexture.create_from_image(pixels)
	item.set_icon(column, _chip_icons[kind])
	var minimum_width := ThemeTokens.font_for(CHIP_STYLE).get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, CHIP_STYLE["size"]).x + 38
	var owner_tree := item.get_tree()
	var minimums: Dictionary = owner_tree.get_meta("column_minimums", {})
	minimums[column] = maxi(minimums.get(column, 0), ceili(minimum_width))
	owner_tree.set_meta("column_minimums", minimums)
	owner_tree.set_column_custom_minimum_width(column, minimums[column])

static func progress(percent: float, kind: String = "") -> HBoxContainer:
	var row := HBoxContainer.new()
	var bar := ProgressBar.new()
	bar.value = percent
	bar.show_percentage = false
	bar.custom_minimum_size.y = 8
	bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bar.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	bar.add_theme_stylebox_override("background", ThemeTokens.box(ThemeTokens.SURFACE_SUNKEN, ThemeTokens.LINE, ThemeTokens.RADIUS_SM, 0, 0))
	bar.add_theme_stylebox_override("fill", ThemeTokens.box(kind_color(kind) if kind in ["go", "stop", "info"] else ACCENT, ThemeTokens.TRANSPARENT, ThemeTokens.RADIUS_SM, 0, 0))
	row.add_child(bar)
	row.add_child(label("%.1f %%" % percent, ThemeTokens.DATA))
	return row

static func route_leg(origin: String, destination: String, percent: float, arrival: String, departed: bool = true) -> VBoxContainer:
	var body := VBoxContainer.new()
	var row := HBoxContainer.new()
	body.add_child(row)
	var ink := ThemeTokens.INK if departed else MUTED
	row.add_child(label(origin, ThemeTokens.CODE, ink))
	var track := LegTrack.new()
	track.percent = percent
	track.departed = departed
	track.custom_minimum_size = Vector2(60, 16)
	track.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(track)
	row.add_child(label(destination, ThemeTokens.CODE, ink))
	var eta := label(arrival, ThemeTokens.CODE, MUTED)
	eta.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	body.add_child(eta)
	return body

static func dialog(parent: Node, question: String, consequence: String, safe: String, destructive: String, confirmed: Callable) -> DecisionDialog:
	var result := DecisionDialog.new()
	result.theme = ThemeTokens.new()
	result.color = ThemeTokens.SCRIM
	result.hide()
	var popup_layer := CanvasLayer.new()
	popup_layer.layer = 100
	parent.get_tree().root.add_child(popup_layer)
	popup_layer.add_child(result)
	parent.tree_exiting.connect(popup_layer.queue_free)
	result.tree_exited.connect(popup_layer.queue_free)
	result.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	var center := CenterContainer.new()
	center.mouse_filter = Control.MOUSE_FILTER_IGNORE
	result.add_child(center)
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	result.card = PanelContainer.new()
	result.card.custom_minimum_size.x = 440
	result.card.add_theme_stylebox_override("panel", ThemeTokens.new().floating_box(ThemeTokens.RADIUS_LG, ThemeTokens.SPACE_6))
	center.add_child(result.card)
	var content := VBoxContainer.new()
	content.add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	result.card.add_child(content)
	var heading := label(question, DIALOG_TITLE)
	heading.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(heading)
	var explanation := label(consequence, ThemeTokens.BODY, MUTED)
	explanation.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	content.add_child(explanation)
	var actions := HBoxContainer.new()
	actions.alignment = BoxContainer.ALIGNMENT_END
	content.add_child(actions)
	result.safe_button = Button.new()
	result.safe_button.text = safe
	actions.add_child(result.safe_button)
	result.danger_button = Button.new()
	result.danger_button.text = destructive
	result.danger_button.theme_type_variation = "DangerSolidButton"
	actions.add_child(result.danger_button)
	result.safe_button.pressed.connect(result.queue_free)
	result.danger_button.pressed.connect(confirmed)
	result.danger_button.pressed.connect(result.queue_free)
	# Keep keyboard traversal inside the decision until a choice is made.
	for button in [result.safe_button, result.danger_button]:
		var other := result.danger_button if button == result.safe_button else result.safe_button
		button.focus_next = button.get_path_to(other)
		button.focus_previous = button.get_path_to(other)
		for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
			button.set_focus_neighbor(side, button.get_path_to(other))
	return result

class DecisionDialog extends ColorRect:
	var safe_button: Button
	var danger_button: Button
	var card: PanelContainer
	func popup_centered() -> void:
		show()
		safe_button.call_deferred("grab_focus")
	func get_cancel_button() -> Button:
		return safe_button
	func get_ok_button() -> Button:
		return danger_button
	func get_visible_rect() -> Rect2:
		return card.get_global_rect()
	func _unhandled_key_input(event: InputEvent) -> void:
		if not visible: return
		if event.is_action_pressed("ui_cancel"): queue_free()
		get_viewport().set_input_as_handled()

static func empty_state(title: String, description: String, action: String, callback: Callable) -> PanelContainer:
	var frame := EmptyFrame.new()
	frame.add_theme_stylebox_override("panel", ThemeTokens.box(ThemeTokens.TRANSPARENT, ThemeTokens.TRANSPARENT, ThemeTokens.RADIUS_LG, ThemeTokens.SPACE_5, ThemeTokens.SPACE_7))
	var column := VBoxContainer.new()
	frame.add_child(column)
	for content in [title, description]:
		if content.is_empty(): continue
		var text := label(content, EMPTY_TITLE if content == title else ThemeTokens.BODY, ThemeTokens.INK if content == title else MUTED)
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		column.add_child(text)
	var button := Button.new()
	button.text = action
	button.theme_type_variation = "PrimaryButton"
	button.size_flags_horizontal = Control.SIZE_SHRINK_CENTER
	button.pressed.connect(callback)
	column.add_child(button)
	return frame

static func money(value: int) -> String:
	var digits := str(absi(value))
	var result := ""
	for i in range(digits.length()):
		if i > 0 and (digits.length() - i) % 3 == 0:
			result += "."
		result += digits[i]
	return ("−" if value < 0 else "") + result + " EUR"

static func table(titles: Array, widths: Array, height: int = 200) -> Tree:
	var result := Tree.new()
	result.columns = titles.size()
	result.hide_root = true
	result.select_mode = Tree.SELECT_ROW
	result.column_titles_visible = true
	result.custom_minimum_size.y = height
	result.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	result.draw.connect(func():
		var root := result.get_root()
		if root == null: return
		for item in root.get_children():
			var rect := result.get_item_area_rect(item)
			if item.get_next() != null and rect.end.y > 32 and rect.end.y < result.size.y - ThemeTokens.SPACE_4:
				result.draw_line(Vector2(1, rect.end.y), Vector2(result.size.x - 1, rect.end.y), ThemeTokens.LINE, 1)
	)
	for i in range(titles.size()):
		result.set_column_title(i, titles[i])
		result.set_column_title_alignment(i, HORIZONTAL_ALIGNMENT_LEFT)
		result.set_column_custom_minimum_width(i, widths[i])
	var minimums := {}
	for i in range(widths.size()): minimums[i] = widths[i]
	result.set_meta("column_minimums", minimums)
	return result
