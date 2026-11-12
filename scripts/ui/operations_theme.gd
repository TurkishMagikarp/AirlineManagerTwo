extends Theme
## Shared application theme. New controls inherit these defaults automatically.

func _init() -> void:
	default_font_size = 15
	set_color("font_color", "Label", Color("e2eaf4"))
	set_constant("separation", "VBoxContainer", 10)
	set_constant("separation", "HBoxContainer", 10)
	for type in ["Button", "OptionButton", "LineEdit", "Tree", "PopupMenu", "PanelContainer", "TabContainer"]:
		for state in ["normal", "hover", "pressed", "disabled", "focus", "read_only", "panel", "selected", "selected_focus", "button_pressed", "title_button_normal", "title_button_hover", "title_button_pressed"]:
			var box := StyleBoxFlat.new()
			box.bg_color = Color("142236")
			box.border_color = Color("293c53")
			box.set_border_width_all(1)
			box.set_corner_radius_all(5)
			box.content_margin_left = 14
			box.content_margin_right = 14
			box.content_margin_top = 10
			box.content_margin_bottom = 10
			if state in ["hover", "title_button_hover"]:
				box.bg_color = Color("20364e")
			if state in ["pressed", "selected", "selected_focus", "button_pressed"]:
				box.bg_color = Color("204454")
				box.border_color = Color("63b9c5")
			if state == "disabled":
				box.bg_color = Color("101b2b")
			if state == "focus":
				box.bg_color = Color.TRANSPARENT
				box.border_color = Color("8bd5df")
				box.set_border_width_all(2)
			set_stylebox(state, type, box)
		for color in ["font_color", "font_hover_color", "font_pressed_color", "font_focus_color", "font_selected_color"]:
			set_color(color, type, Color("e2eaf4"))
		set_color("font_disabled_color", type, Color("718298"))
	set_color("font_placeholder_color", "LineEdit", Color("91a3b9"))
	set_color("guide_color", "Tree", Color("26384c"))
	set_constant("v_separation", "Tree", 14)
	set_constant("h_separation", "Tree", 12)
	set_constant("draw_guides", "Tree", 0)
	set_constant("scrollbar_width", "Tree", 10)
	set_font_size("title_button_font_size", "Tree", 13)
	set_type_variation("PrimaryButton", "Button")
	var primary := get_stylebox("normal", "Button").duplicate() as StyleBoxFlat
	primary.bg_color = Color("235262")
	primary.border_color = Color("63b9c5")
	set_stylebox("normal", "PrimaryButton", primary)
	set_type_variation("DangerButton", "Button")
	set_color("font_color", "DangerButton", Color("f1a6ad"))
	for type in ["VScrollBar", "HScrollBar"]:
		for state in ["scroll", "grabber", "grabber_highlight", "grabber_pressed"]:
			var box := StyleBoxFlat.new()
			box.bg_color = Color("101b2b") if state == "scroll" else Color("3d596f")
			box.set_corner_radius_all(3)
			box.content_margin_left = 4
			box.content_margin_top = 4
			set_stylebox(state, type, box)
