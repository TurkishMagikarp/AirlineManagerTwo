extends RefCounted
## Small composable UI primitives; never reads or changes simulation state.

const MUTED = Color("91a3b9")
const ACCENT = Color("83cbd2")
const POSITIVE = Color("8fd5b5")
const NEGATIVE = Color("f1a6ad")

static func label(text: String, size: int = 15, color: Color = Color("e2eaf4")) -> Label:
	var result := Label.new()
	result.text = text
	result.add_theme_font_size_override("font_size", size)
	result.add_theme_color_override("font_color", color)
	return result

static func panel(parent: Node, title: String) -> VBoxContainer:
	var shell := PanelContainer.new()
	shell.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	parent.add_child(shell)
	var column := VBoxContainer.new()
	column.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	column.add_theme_constant_override("separation", 12)
	shell.add_child(column)
	if not title.is_empty():
		column.add_child(label(title, 13, ACCENT))
	return column

static func metric(parent: Node, title: String) -> Label:
	var column := panel(parent, title)
	column.custom_minimum_size.x = 220
	var value := label("—", 27)
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
