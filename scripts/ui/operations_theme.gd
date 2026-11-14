extends Theme
## Night UI kit tokens; source: docs/ui-kit/tokens.json.

const GROUND = Color("11100e")
const SURFACE = Color("1a1816")
const SURFACE_RAISED = Color("24211e")
const SURFACE_SUNKEN = Color("0a0908")
const LINE = Color("35312c")
const LINE_STRONG = Color("7a7368")
const INK = Color("f3eee4")
const INK_MUTED = Color("ada596")
const INK_FAINT = Color("7c766b")
const ACCENT = Color("f4b740")
const ACCENT_HOVER = Color("ffc95c")
const ON_ACCENT = Color("1b1405")
const ACCENT_TEXT = Color("f4b740")
const ACCENT_WASH = Color("2f2612")
const CAUTION = Color("f4b740")
const CAUTION_WASH = Color("2f2612")
const GO = Color("4fd0b8")
const GO_WASH = Color("12302b")
const STOP = Color("ff8a78")
const ON_STOP = Color("2a0a06")
const STOP_WASH = Color("3b1b16")
const SKY = Color("86b8ff")
const SKY_WASH = Color("162741")
const FOCUS = Color("fff4d6")
const FLAP = Color("0a0908")
const FLAP_INK = Color("f4b740")
const FLAP_SEAM = Color("35312c")
const MAP_WATER = Color("0c1519")
const MAP_LAND = Color("26231f")
const SCRIM = Color(0, 0, 0, 0.62)
const SPACE_1 = 4
const SPACE_2 = 8
const SPACE_3 = 12
const SPACE_4 = 16
const SPACE_5 = 24
const SPACE_6 = 32
const SPACE_7 = 48
const RADIUS_SM = 2
const RADIUS_MD = 4
const RADIUS_LG = 8
const SHADOW_POP = {"offset": 12, "size": 32, "color": Color(0, 0, 0, 0.55)}
const TRANSPARENT = Color.TRANSPARENT
# Component geometry from bundle.css, in pixels.
const CONTROL_HEIGHT = 36
const TAB_HEIGHT = 40
const TABLE_ROW_HEIGHT = 40
static var _fonts: Dictionary = {}
const SCREEN_TITLE = {"family": "display", "size": 32, "height": 36, "weight": 600, "tracking": -0.01}
const PANEL_TITLE = {"family": "display", "size": 15, "height": 20, "weight": 600, "tracking": 0.08}
const COLUMN_HEAD = {"family": "display", "size": 12, "height": 16, "weight": 600, "tracking": 0.08}
const LABEL = {"family": "display", "size": 12, "height": 16, "weight": 600, "tracking": 0.08}
const BODY = {"family": "sans", "size": 16, "height": 22, "weight": 400, "tracking": 0.0}
const BODY_STRONG = {"family": "sans", "size": 16, "height": 22, "weight": 600, "tracking": 0.0}
const SMALL = {"family": "sans", "size": 14, "height": 18, "weight": 400, "tracking": 0.0}
const FIGURE = {"family": "mono", "size": 24, "height": 28, "weight": 500, "tracking": 0.0}
const DATA = {"family": "mono", "size": 14, "height": 20, "weight": 400, "tracking": 0.0}
const CODE = {"family": "mono", "size": 12, "height": 16, "weight": 500, "tracking": 0.06}

const DISPLAY_FONT = preload("res://resources/fonts/outfit/Outfit[wght].ttf")
const SANS_FONT = preload("res://resources/fonts/nunito/Nunito[wght].ttf")
const MONO_FONT = preload("res://resources/fonts/dmmono/DMMono-Regular.ttf")
const MONO_MEDIUM_FONT = preload("res://resources/fonts/dmmono/DMMono-Medium.ttf")

static func font_for(style: Dictionary) -> Font:
	var key := "%s/%s/%s/%s" % [style["family"], style["weight"], style["size"], style["tracking"]]
	if _fonts.has(key):
		return _fonts[key]
	var font := FontVariation.new()
	if style["family"] == "mono":
		font.base_font = MONO_MEDIUM_FONT if style["weight"] == 500 else MONO_FONT
	else:
		font.base_font = DISPLAY_FONT if style["family"] == "display" else SANS_FONT
		font.variation_opentype = {"wght": style["weight"]}
	font.spacing_glyph = roundi(style["size"] * style["tracking"])
	var extra := roundi(style["height"] - font.get_height(style["size"]))
	font.spacing_top = floori(extra / 2.0)
	font.spacing_bottom = extra - font.spacing_top
	_fonts[key] = font
	return font

static func box(fill: Color, border: Color = LINE, radius: int = RADIUS_MD, horizontal: int = SPACE_4, vertical: int = SPACE_2) -> StyleBoxFlat:
	var result := StyleBoxFlat.new()
	result.bg_color = fill
	result.border_color = border
	result.set_border_width_all(1)
	result.set_corner_radius_all(radius)
	result.content_margin_left = horizontal
	result.content_margin_right = horizontal
	result.content_margin_top = vertical
	result.content_margin_bottom = vertical
	return result

static func focus_box() -> StyleBoxFlat:
	var result := box(TRANSPARENT, FOCUS)
	result.set_border_width_all(2)
	# 2px gap plus the 2px border, outside the control's rectangle.
	for side in [SIDE_LEFT, SIDE_TOP, SIDE_RIGHT, SIDE_BOTTOM]:
		result.set_expand_margin(side, 4)
	return result

func text_style(type: String, style: Dictionary, font_name: String = "font", size_name: String = "font_size") -> void:
	set_font(font_name, type, font_for(style))
	set_font_size(size_name, type, style["size"])

func floating_box(radius: int, padding: int) -> StyleBoxFlat:
	var result := box(SURFACE_RAISED, LINE_STRONG, radius, padding, padding)
	result.shadow_color = SHADOW_POP["color"]
	result.shadow_size = SHADOW_POP["size"]
	result.shadow_offset = Vector2(0, SHADOW_POP["offset"])
	return result

func button_styles(
	type: String, fill: Color, hover: Color, pressed: Color, border: Color, ink: Color
) -> void:
	if type != "Button":
		set_type_variation(type, "Button")
	text_style(type, BODY_STRONG)
	for state in ["normal", "hover", "pressed", "hover_pressed", "disabled"]:
		var bg := fill
		var edge := border
		if state == "hover":
			bg = hover
			if type == "Button": edge = INK_MUTED
		elif state in ["pressed", "hover_pressed"]: bg = pressed
		elif state == "disabled":
			bg = SURFACE
			edge = LINE
		# Nunito's 22px line and 7px vertical padding give a 36px control.
		set_stylebox(state, type, box(bg, edge, RADIUS_MD, SPACE_4, (CONTROL_HEIGHT - BODY["height"]) / 2))
		var foreground := INK_FAINT if state == "disabled" else ink
		if type == "GhostButton" and state == "hover": foreground = INK
		if type == "PrimaryButton" and state in ["pressed", "hover_pressed"]: foreground = SURFACE
		set_color("font_" + state + "_color" if state != "normal" else "font_color", type, foreground)
		set_color("icon_" + state + "_color" if state != "normal" else "icon_normal_color", type, foreground)
	set_color("font_focus_color", type, ink)
	set_color("icon_focus_color", type, ink)
	set_constant("h_separation", type, SPACE_2)
	set_stylebox("focus", type, focus_box())

func _init() -> void:
	default_font = font_for(BODY)
	default_font_size = BODY["size"]
	set_color("font_color", "Label", INK)
	text_style("Label", BODY)
	for type in ["VBoxContainer", "HBoxContainer"]:
		set_constant("separation", type, SPACE_3)
	button_styles("Button", SURFACE_RAISED, LINE, SURFACE_SUNKEN, LINE_STRONG, INK)
	button_styles("PrimaryButton", ACCENT, ACCENT_HOVER, ACCENT_TEXT, ACCENT_TEXT, ON_ACCENT)
	button_styles("DangerButton", SURFACE_RAISED, STOP_WASH, SURFACE_SUNKEN, STOP, STOP)
	button_styles("DangerSolidButton", STOP, STOP, STOP, STOP, ON_STOP)
	button_styles("GhostButton", TRANSPARENT, SURFACE_RAISED, SURFACE_SUNKEN, TRANSPARENT, INK_MUTED)
	button_styles("OptionButton", SURFACE_RAISED, SURFACE_RAISED, SURFACE_SUNKEN, LINE_STRONG, INK)
	set_type_variation("OptionButton", "Button")
	text_style("OptionButton", BODY)
	for type in ["LineEdit", "Tree", "PopupMenu", "TabBar", "TabContainer"]:
		text_style(type, BODY)
		set_color("font_color", type, INK)
		set_color("font_disabled_color", type, INK_FAINT)
		set_color("font_selected_color", type, INK)
		set_stylebox("focus", type, focus_box())
	set_stylebox("normal", "LineEdit", box(SURFACE_SUNKEN, LINE_STRONG, RADIUS_MD, SPACE_3, (CONTROL_HEIGHT - BODY["height"]) / 2))
	set_stylebox("read_only", "LineEdit", box(SURFACE, LINE, RADIUS_MD, SPACE_3, (CONTROL_HEIGHT - BODY["height"]) / 2))
	set_color("font_uneditable_color", "LineEdit", INK_FAINT)
	set_color("font_placeholder_color", "LineEdit", INK_MUTED)
	set_color("caret_color", "LineEdit", INK)
	set_color("selection_color", "LineEdit", ACCENT_WASH)
	set_type_variation("NumericInput", "LineEdit")
	text_style("NumericInput", DATA)
	set_type_variation("FieldError", "LineEdit")
	set_stylebox("normal", "FieldError", box(SURFACE_SUNKEN, STOP, RADIUS_MD, SPACE_3, (CONTROL_HEIGHT - BODY["height"]) / 2))
	set_stylebox("panel", "PanelContainer", box(SURFACE, LINE, RADIUS_LG, SPACE_4, SPACE_4))
	set_stylebox("panel", "PopupMenu", floating_box(RADIUS_LG, SPACE_1))
	set_stylebox("hover", "PopupMenu", box(LINE, TRANSPARENT, RADIUS_SM, SPACE_2, SPACE_1))
	set_color("font_hover_color", "PopupMenu", INK)
	set_constant("v_separation", "PopupMenu", SPACE_2)
	set_stylebox("panel", "Tree", box(SURFACE, LINE, RADIUS_LG, 0, 0))
	for state in ["selected", "selected_focus"]:
		set_stylebox(state, "Tree", box(ACCENT_WASH, TRANSPARENT, 0, 0, 0))
	set_stylebox("hovered", "Tree", box(SURFACE_RAISED, TRANSPARENT, 0, 0, 0))
	for state in ["normal", "hover", "pressed"]:
		set_stylebox("title_button_" + state, "Tree", box(SURFACE_SUNKEN, LINE, 0, SPACE_3, SPACE_2))
		set_color("title_button_color" if state == "normal" else "title_button_color_" + state, "Tree", INK_MUTED)
	text_style("Tree", COLUMN_HEAD, "title_button_font", "title_button_font_size")
	set_color("guide_color", "Tree", LINE)
	set_color("font_hovered_color", "Tree", INK)
	set_color("font_hovered_selected_color", "Tree", INK)
	set_constant("v_separation", "Tree", TABLE_ROW_HEIGHT - BODY["height"])
	set_constant("h_separation", "Tree", SPACE_3)
	set_constant("draw_guides", "Tree", 0)
	for type in ["TabBar", "TabContainer"]:
		text_style(type, PANEL_TITLE)
		set_stylebox("panel", type, box(SURFACE, LINE, RADIUS_LG, SPACE_4, SPACE_4))
		for state in ["tab_selected", "tab_unselected", "tab_hovered", "tab_disabled"]:
			var tab := box(TRANSPARENT, TRANSPARENT, 0, SPACE_3, (TAB_HEIGHT - PANEL_TITLE["height"]) / 2)
			tab.set_border_width_all(0)
			if state == "tab_selected":
				tab.border_width_bottom = 2
				tab.border_color = ACCENT_TEXT
			set_stylebox(state, type, tab)
		set_stylebox("tab_focus", type, focus_box())
		set_color("font_unselected_color", type, INK_MUTED)
		set_color("font_hovered_color", type, INK)
	for type in ["VScrollBar", "HScrollBar"]:
		for state in ["scroll", "grabber", "grabber_highlight", "grabber_pressed"]:
			var fill := SURFACE_SUNKEN if state == "scroll" else INK_MUTED if state == "grabber_highlight" else LINE_STRONG
			set_stylebox(state, type, box(fill, TRANSPARENT, RADIUS_SM, SPACE_1, SPACE_1))
		set_stylebox("scroll_focus", type, focus_box())
	set_stylebox("panel", "TooltipPanel", floating_box(RADIUS_MD, SPACE_3))
	text_style("TooltipLabel", SMALL)
	set_color("font_color", "TooltipLabel", INK)
