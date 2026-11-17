extends SceneTree
## Exercise real Fleet inputs and both irreversible confirmation dialogs.
func _initialize() -> void:
	call_deferred("_run")

func _run() -> void:
	TranslationServer.set_locale("de")
	var main = load("res://scenes/main.tscn").instantiate()
	root.add_child(main)
	main.set_process(false)
	var sim: RefCounted = main.simulation
	assert(sim.found_airline("Manufacturer UI", "DE", "EDDF").is_empty())
	main._sync_view()
	main._show_page(1)
	main.fleet_tabs.current_tab = 1
	var panel = main.orders_panel
	assert(panel.empty.visible)
	assert(panel.catalog.visible and panel.catalog.get_root().get_child_count() == 5)
	var neo_index := -1
	for i in range(panel.model.item_count):
		if panel.model.get_item_metadata(i) == "a320neo": neo_index = i
	assert(neo_index >= 0 and "Tag 731" in panel.model.get_item_text(neo_index))
	panel.model.select(neo_index)
	panel.model.item_selected.emit(neo_index)
	assert(panel.place.disabled and "Tag 731" in panel.place.tooltip_text)
	var card: Dictionary = main.aircraft_cards[neo_index]
	assert(card["buy"].disabled and card["lease"].disabled and card["status"].get_child_count() == 1)
	var before: int = sim.get_airline_info()["capital_eur"]
	panel.model.select(0)
	panel.model.item_selected.emit(0)
	panel.quantity.value = 2
	assert(not panel.place.disabled)
	var deposit: int = panel.quote["total_deposit"]
	panel.place.pressed.emit()
	assert(panel.dialog.visible)
	assert(sim.get_orders().is_empty())
	panel.dialog.get_cancel_button().pressed.emit()
	await process_frame
	assert(sim.get_orders().is_empty())
	panel.place.pressed.emit()
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_orders().size() == 2)
	assert(sim.get_airline_info()["capital_eur"] == before - deposit)
	assert(sim.get_fleet().is_empty())
	assert(not panel.empty.visible)
	assert(panel.rows[0]["status"] == "ORDER_SCHEDULED")
	assert(panel.rows[0]["delivery_date"] != panel.rows[1]["delivery_date"])
	panel.cancel.pressed.emit()
	assert(panel.dialog.visible)
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_orders().size() == 1)
	assert(sim.get_airline_info()["capital_eur"] == before - deposit)
	panel.cancel.pressed.emit()
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_orders().is_empty())
	assert(panel.empty.visible)
	# A later unpaid delivery opens the same table from its attention entry.
	panel.quantity.value = 1
	panel.place.pressed.emit()
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	sim.start()
	assert(sim.advance(60.0 * 86400.0))
	sim.pause()
	main._sync_view()
	var attention: Array = sim.get_attention_snapshot()
	var deliveries: Array = attention.filter(func(entry: Dictionary): return entry["kind"] == "delivery")
	assert(deliveries.size() == 1)
	main._open_attention(deliveries[0])
	assert(main.current_page == 1 and main.fleet_tabs.current_tab == 1)
	assert(panel.selected_id == deliveries[0]["order_id"])
	assert(panel.rows[0]["status"] == "ORDER_HELD")
	assert(not panel.rows[0]["grace_deadline"].is_empty())
	var held_cash: int = sim.get_airline_info()["capital_eur"]
	panel.cancel.pressed.emit()
	panel.dialog.get_ok_button().pressed.emit()
	await process_frame
	assert(sim.get_orders().is_empty() and sim.get_airline_info()["capital_eur"] == held_cash)
	main.queue_free()
	await process_frame
	print("PASS: real Fleet order preview, safe dismissal, confirmed deposit, staggered dates, held attention navigation and confirmed cancellation with no refund")
	quit(0)
