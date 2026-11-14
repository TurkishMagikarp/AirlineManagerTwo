extends SceneTree
## Exercise real font variation resolution: string axis names silently fell back to thin.
const Tokens = preload("res://scripts/ui/operations_theme.gd")

func _initialize() -> void:
	var server := TextServerManager.get_primary_interface()
	var weight_axis := server.name_to_tag("wght")
	for style in [Tokens.SCREEN_TITLE, Tokens.PANEL_TITLE, Tokens.COLUMN_HEAD, Tokens.LABEL, Tokens.BODY, Tokens.BODY_STRONG, Tokens.SMALL]:
		var font := Tokens.font_for(style)
		for rid in font.get_rids():
			assert(server.font_get_variation_coordinates(rid).get(weight_axis) == style["weight"], "Font weight must reach the text server")
	assert(Tokens.font_for(Tokens.DATA).base_font == Tokens.MONO_FONT)
	assert(Tokens.font_for(Tokens.CODE).base_font == Tokens.MONO_MEDIUM_FONT)
	assert(Tokens.font_for(Tokens.FIGURE).base_font == Tokens.MONO_MEDIUM_FONT)
	print("PASS: Outfit/Nunito weights reach renderer; DM Mono uses bundled 400/500 faces")
	quit()
