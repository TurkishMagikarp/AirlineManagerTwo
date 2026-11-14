extends RefCounted
## Small composable UI primitives; never reads or changes simulation state.

const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
const MUTED = ThemeTokens.INK_MUTED
const ACCENT = ThemeTokens.ACCENT_TEXT
const POSITIVE = ThemeTokens.GO
const NEGATIVE = ThemeTokens.STOP
static var _numeric: RegEx
static var _code: RegEx

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
	item.set_text(column, text)
	if _numeric == null:
		_numeric = RegEx.new()
		_numeric.compile("^[−+0-9]|[0-9] (EUR|%|km|kg)$")
		_code = RegEx.new()
		_code.compile("^[A-Z0-9#– →-]+$")
	var numeric := _numeric.search(text) != null
	var code := not text.is_empty() and _code.search(text) != null and (not numeric or text.begins_with("#"))
	var style: Dictionary = ThemeTokens.CODE if code else ThemeTokens.DATA if numeric else ThemeTokens.BODY
	item.set_custom_font(column, ThemeTokens.font_for(style))
	item.set_custom_font_size(column, style["size"])
	item.set_text_alignment(column, HORIZONTAL_ALIGNMENT_RIGHT if numeric and not code else HORIZONTAL_ALIGNMENT_LEFT)

static func panel(parent: Node, title: String) -> VBoxContainer:
	var shell := PanelContainer.new()
	shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(shell)
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", ThemeTokens.SPACE_3)
	shell.add_child(column)
	if not title.is_empty():
		column.add_child(label(title, ThemeTokens.PANEL_TITLE, ThemeTokens.INK))
	return column

static func metric(parent: Node, title: String) -> Label:
	var column := panel(parent, "")
	column.add_child(label(title, ThemeTokens.LABEL, MUTED))
	column.custom_minimum_size.x = 220
	var value := label("—", ThemeTokens.FIGURE)
	column.add_child(value)
	return value

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
	for i in range(titles.size()):
		result.set_column_title(i, titles[i])
		result.set_column_custom_minimum_width(i, widths[i])
	return result
