extends SceneTree
const UI = preload("res://scripts/ui/components.gd")
const Tokens = preload("res://scripts/ui/operations_theme.gd")
var host: VBoxContainer

func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	root.size = Vector2i(680, 520)
	host = VBoxContainer.new()
	host.theme = Tokens.new()
	host.size = Vector2(440, 1000)
	root.add_child(host)
	var accessory := Button.new()
	accessory.text = "Flugplan bearbeiten"
	var body := UI.panel(host, "AUSGEWÄHLTE ROUTE · ZUWEISUNG", accessory)
	body.add_child(UI.label("Alle Flugzeuge können besetzt werden."))
	var metric_host := VBoxContainer.new()
	metric_host.custom_minimum_size.x = UI.METRIC_MIN_WIDTH
	metric_host.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN
	host.add_child(metric_host)
	var value := UI.metric(metric_host, "BETRIEBSERGEBNIS DER VORPERIODE", UI.progress(78.5))
	value.text = "−999.999.999.999 EUR"
	value.add_theme_color_override("font_color", Tokens.STOP)
	for kind in UI.KINDS:
		host.add_child(UI.status_chip("Besatzung verfügbar", kind))
	host.add_child(UI.route_leg("EDDF", "EDDM", 45, "14.11.2026 18:42"))
	host.add_child(UI.route_leg("EDDF", "EDDM", 0, "14.11.2026 18:42", false))
	var table := UI.table(["Kennzeichen", "Betriebsergebnis", "Status"], [115, 160, 180])
	host.add_child(table)
	var row := table.create_item(table.create_item())
	UI.cell_text(row, 0, "D-ANFA")
	UI.cell_text(row, 1, "−12.480 EUR")
	UI.cell_status(row, 2, "Wartung erforderlich", "caution")
	var second := table.create_item(table.get_root())
	UI.cell_text(second, 0, "D-ANFB")
	table.set_column_title(1, "Pax / Auslastung")
	UI.cell_text(second, 1, "Y 72/72 · J 0/0")
	var empty := UI.empty_state("Noch keine Flugzeuge beschafft.", "Vorläufige Kaufpreise · Sitzkonfiguration und Reichweite sind vereinfachte Modellangaben.", "Flugzeug kaufen", func(): pass)
	host.add_child(empty)
	var decision := {"confirmed": 0}
	var dialog := UI.dialog(host, "D-ANFA verkaufen?", "Das Flugzeug wird abgegeben und die Gutschrift gebucht. Der Flugplan kann dieses Flugzeug anschließend nicht mehr verwenden.", "Abbrechen", "Flugzeug verkaufen", func(): decision["confirmed"] += 1)
	dialog.popup_centered()
	for i in range(12): await process_frame
	assert(body.get_child_count() == 1, "Panel returns only the body")
	assert(accessory.size.y == Tokens.HEADER_CONTROL_HEIGHT)
	assert(body.get_meta("panel_shell").get_global_rect().encloses(accessory.get_global_rect()))
	assert(body.get_parent().get_parent().get_child(0).size.y >= 48)
	assert(metric_host.size.x == UI.METRIC_MIN_WIDTH, "Metric must not expand to the number's natural width")
	assert(value.get_global_rect().encloses(value.rendered.get_global_rect()), "Styled value stays inside its Label")
	assert(metric_host.get_global_rect().encloses(value.get_global_rect()), "Value stays inside minimum-width card")
	assert(value.rendered.get_line_count() >= 2, "Long figure and unit wrap")
	assert(value.rendered.get_content_height() <= value.size.y)
	assert(value.rendered.get_content_width() <= value.size.x)
	assert(value.rendered.text.contains("font_size=14"), "Unit uses data size")
	assert(table.get_item_area_rect(second).position.y - table.get_item_area_rect(row).position.y == Tokens.TABLE_ROW_HEIGHT)
	assert(row.get_text_alignment(1) == HORIZONTAL_ALIGNMENT_RIGHT)
	assert(row.get_custom_font(1) == Tokens.font_for(Tokens.DATA))
	assert(second.get_text_alignment(1) == HORIZONTAL_ALIGNMENT_RIGHT)
	assert(second.get_custom_font(1) == Tokens.font_for(Tokens.DATA))
	assert(row.get_icon(2) != null)
	assert(row.get_text(2) == "Wartung erforderlich")
	assert(row.get_custom_font(2) == Tokens.font_for(UI.CHIP_STYLE))
	assert(dialog.get_cancel_button().has_focus(), "Safe choice receives focus")
	assert(dialog.get_cancel_button().position.x < dialog.get_ok_button().position.x)
	assert(dialog.get_ok_button().theme_type_variation == "DangerSolidButton")
	assert(root.get_visible_rect().encloses(dialog.get_visible_rect()))
	for child in empty.get_child(0).get_children():
		assert(empty.get_global_rect().encloses(child.get_global_rect()), "German empty-state copy and button fit")
	value.add_theme_color_override("font_color", Tokens.GO)
	await process_frame
	await process_frame
	assert(not value.rendered.text.contains(Tokens.GO.to_html()), "Metric figure never turns go")
	for child in host.get_children():
		if child is PanelContainer and child.theme_type_variation.begins_with("Chip"):
			assert(child.get_global_rect().encloses(child.get_child(0).get_global_rect()), "German status text fits its chip")
	assert(dialog.safe_button.focus_next == dialog.safe_button.get_path_to(dialog.danger_button))
	dialog.safe_button.pressed.emit()
	await process_frame
	assert(decision["confirmed"] == 0 and not is_instance_valid(dialog), "Safe choice closes without confirmation")
	var second_dialog := UI.dialog(host, "Flugzeug AM-000001 nach Abschluss der laufenden Wartungsarbeiten verkaufen?", "Das Flugzeug wird abgegeben und die Gutschrift gebucht. Der Flugplan kann dieses Flugzeug anschließend nicht mehr verwenden.", "Abbrechen", "Flugzeug verkaufen", func(): decision["confirmed"] += 1)
	second_dialog.popup_centered()
	for i in range(8): await process_frame
	assert(root.get_visible_rect().encloses(second_dialog.get_visible_rect()), "Long German question and consequence fit")
	second_dialog.danger_button.pressed.emit()
	await process_frame
	assert(decision["confirmed"] == 1 and not is_instance_valid(second_dialog), "Only the destructive choice confirms")
	host.queue_free()
	await process_frame
	print("PASS: kit helpers build; German copy, safe dialog focus, numeric columns and minimum-width metric wrapping")
	quit()
