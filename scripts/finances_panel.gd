extends VBoxContainer
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## All balances, route aggregates, period values and flight pages come from Rust.
const UI = preload("res://scripts/ui/components.gd")
const Chart = preload("res://scripts/finance_chart.gd")
const RouteCsv = preload("res://scripts/route_finance_csv.gd")
var simulation: RefCounted
var metrics: Dictionary = {}
var note: Label
var chart: Control
var load_chart: Control
var chart_comparison: Label
var chart_select: OptionButton
var route_tree: Tree
var export_button: Button
var export_dialog: FileDialog
var export_status: Label
var flights_tree: Tree
var detail: Label
var page_label: Label
var previous_button: Button
var next_button: Button
var route_id := 0
var offset := 0
var flight_count := 0
var signature := ""
var elapsed := 0.0
var company_series: Array = []
var chart_range: Label
var loan_status: Label
var loan_rules: Label
var loan_amount: SpinBox
var loan_borrow: Button
var loan_reschedule: Button
var loan_repay: Button
var loan_result: Label
var aircraft_journal: Tree
var tax_history: Tree
var marketing_budget: SpinBox
var marketing_apply: Button
var marketing_status: Label
var marketing_result: Label
var fuel_status: Label
var fuel_share: OptionButton
var fuel_term: OptionButton
var fuel_start: Button
var fuel_result: Label
var insurance_status: Label
var insurance_choice: OptionButton
var insurance_result: Label
var ferry_history: Tree
var marketing_history: Tree
var ancillary_buttons: Array[Button] = []
var loyalty_buttons: Array[Button] = []
var loyalty_metrics: Dictionary = {}
var loyalty_status: Label
var loyalty_result: Label
var loyalty_history: Tree
var ancillary_status: Label
var ancillary_result: Label

func _ready() -> void:
	add_theme_constant_override("separation", ThemeTokens.SPACE_4)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	grid.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	add_child(grid)
	get_viewport().size_changed.connect(func(): grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2)
	grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2
	for item in [["balance", "KONTOSTAND"], ["revenue", "COMPANY_REVENUE"], ["ticket", "REVENUE_TICKETS"], ["cargo", "REVENUE_CARGO"], ["ancillary", "ANCILLARY_REVENUE"], ["charter", "REVENUE_CHARTER"], ["costs", "COMPANY_COSTS"], ["penalty", "COST_CHARTER_PENALTY"], ["profit", "COMPANY_RESULT"]]:
		metrics[item[0]] = UI.metric(grid, tr(item[1]))
	note = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(note)
	var loyalty := UI.panel(self, "LOYALTY_TITLE")
	var loyalty_segments := UI.segment_tray(loyalty)
	var loyalty_group := ButtonGroup.new()
	for i in range(3):
		var button := Button.new()
		button.text = tr(["LOYALTY_NONE", "LOYALTY_BASIC", "LOYALTY_GENEROUS"][i])
		button.theme_type_variation = "SegmentButton"
		button.toggle_mode = true
		button.button_group = loyalty_group
		button.pressed.connect(func():
			var error: String = simulation.set_loyalty_level(["none", "basic", "generous"][i])
			loyalty_result.text = error if not error.is_empty() else tr("LOYALTY_CHANGED")
			refresh(true))
		loyalty_segments.add_child(button)
		loyalty_buttons.append(button)
	var loyalty_grid := GridContainer.new()
	loyalty_grid.add_theme_constant_override("h_separation", ThemeTokens.SPACE_4)
	loyalty_grid.add_theme_constant_override("v_separation", ThemeTokens.SPACE_4)
	loyalty.add_child(loyalty_grid)
	get_viewport().size_changed.connect(func(): loyalty_grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2)
	loyalty_grid.columns = 4 if get_viewport_rect().size.x >= 1280 else 2
	for item in [["members", "LOYALTY_MEMBERS"], ["trend", "LOYALTY_TREND"], ["bonus", "LOYALTY_BONUS"], ["cost", "LOYALTY_COST"]]:
		loyalty_metrics[item[0]] = UI.metric(loyalty_grid, item[1])
	loyalty_status = UI.label("")
	loyalty_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	loyalty.add_child(loyalty_status)
	loyalty_result = UI.label("")
	loyalty_result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	loyalty.add_child(loyalty_result)
	loyalty_history = UI.table([tr("LOYALTY_PERIOD"), tr("LOYALTY_COST")], [180, 160], 140)
	loyalty_history.set_column_title_alignment(1, HORIZONTAL_ALIGNMENT_RIGHT)
	loyalty.add_child(loyalty_history)
	var ancillary := UI.panel(self, tr("ANCILLARY_TITLE"))
	var segments := UI.segment_tray(ancillary)
	var group := ButtonGroup.new()
	for i in range(3):
		var button := Button.new()
		button.text = tr(["ANCILLARY_INCLUDED_SHORT", "ANCILLARY_STANDARD_SHORT", "ANCILLARY_AGGRESSIVE_SHORT"][i])
		button.theme_type_variation = "SegmentButton"
		button.toggle_mode = true
		button.button_group = group
		button.pressed.connect(func():
			var error: String = simulation.set_ancillary_policy(["included", "standard", "aggressive"][i])
			ancillary_result.text = error if not error.is_empty() else tr("ANCILLARY_CHANGED")
			refresh(true))
		segments.add_child(button)
		ancillary_buttons.append(button)
	ancillary_status = UI.label("")
	ancillary_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	ancillary.add_child(ancillary_status)
	ancillary_result = UI.label("")
	ancillary_result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	ancillary.add_child(ancillary_result)
	var insurance := UI.panel(self, "FLUGZEUGVERSICHERUNG")
	insurance_status = UI.label("")
	insurance_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	insurance.add_child(insurance_status)
	insurance_choice = OptionButton.new()
	for title in ["Keine Deckung", "Basis", "Voll"]:
		insurance_choice.add_item(title)
	insurance_choice.item_selected.connect(func(index: int):
		var error: String = simulation.set_insurance(["none", "basic", "full"][index])
		insurance_result.text = error if not error.is_empty() else "Deckung geändert."
		refresh(true))
	insurance.add_child(insurance_choice)
	insurance_result = UI.label("")
	insurance.add_child(insurance_result)
	var fuel := UI.panel(self, "TREIBSTOFF / LIEFERVERTRAG")
	fuel_status = UI.label("")
	fuel_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fuel.add_child(fuel_status)
	var fuel_actions := HFlowContainer.new()
	fuel.add_child(fuel_actions)
	fuel_share = OptionButton.new()
	fuel_term = OptionButton.new()
	var fuel_rules: Dictionary = simulation.get_fuel_contract()
	for share in fuel_rules["shares"]:
		fuel_share.add_item("%d %%" % share, share)
	for months in fuel_rules["terms"]:
		fuel_term.add_item("%d Monate" % months, months)
	fuel_actions.add_child(fuel_share)
	fuel_actions.add_child(fuel_term)
	fuel_start = Button.new()
	fuel_start.text = "Preis sichern"
	fuel_start.pressed.connect(func():
		var error: String = simulation.start_fuel_contract(fuel_share.get_selected_id(), fuel_term.get_selected_id())
		fuel_result.text = error if not error.is_empty() else "Treibstoffvertrag abgeschlossen."
		refresh(true))
	fuel_actions.add_child(fuel_start)
	fuel_result = UI.label("")
	fuel_result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	fuel.add_child(fuel_result)
	var marketing := UI.panel(self, "MARKETING / MARKENBEKANNTHEIT")
	marketing_status = UI.label("")
	marketing_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	marketing.add_child(marketing_status)
	var marketing_actions := HFlowContainer.new()
	marketing.add_child(marketing_actions)
	marketing_budget = SpinBox.new()
	marketing_budget.get_line_edit().theme_type_variation = "NumericInput"
	marketing_budget.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	marketing_budget.min_value = 0
	marketing_budget.step = 1
	marketing_budget.custom_minimum_size.x = 180
	marketing_actions.add_child(marketing_budget)
	marketing_apply = Button.new()
	marketing_apply.text = "Monatsbudget übernehmen"
	marketing_apply.pressed.connect(func():
		marketing_budget.apply()
		var error: String = simulation.set_marketing_budget(int(marketing_budget.value))
		marketing_result.text = error if not error.is_empty() else "Marketingbudget übernommen."
		refresh(true))
	marketing_actions.add_child(marketing_apply)
	marketing_result = UI.label("")
	marketing_result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	marketing.add_child(marketing_result)
	marketing_history = UI.table(["Zeit", "Marketingkosten · letzte 100"], [170, 260], 140)
	marketing.add_child(marketing_history)
	var taxes := UI.panel(self, "ERTRAGSTEUER / JAHRESABSCHLÜSSE", null, true)
	tax_history = UI.table(["Jahr", "Vorsteuerergebnis", "Steuersatz", "Steuer", "Verlustvortrag"], [65, 160, 100, 140, 160], 140)
	taxes.add_child(tax_history)
	var loans := UI.panel(self, "KREDITE / LIQUIDITÄT")
	loan_status = UI.label("")
	loan_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	loans.add_child(loan_status)
	loan_rules = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	loan_rules.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	loans.add_child(loan_rules)
	var actions := HFlowContainer.new()
	loans.add_child(actions)
	loan_amount = SpinBox.new()
	loan_amount.get_line_edit().theme_type_variation = "NumericInput"
	loan_amount.get_line_edit().alignment = HORIZONTAL_ALIGNMENT_RIGHT
	loan_amount.min_value = 1
	loan_amount.max_value = 1000000000
	loan_amount.step = 1
	loan_amount.value = 1000000
	loan_amount.custom_minimum_size.x = 180
	actions.add_child(loan_amount)
	loan_borrow = Button.new()
	loan_borrow.text = "Kredit aufnehmen"
	loan_borrow.pressed.connect(func(): _loan_action(true))
	actions.add_child(loan_borrow)
	loan_repay = Button.new()
	loan_repay.text = "Sondertilgung"
	loan_repay.pressed.connect(func(): _loan_action(false))
	actions.add_child(loan_repay)
	loan_reschedule = Button.new()
	loan_reschedule.text = "Kredit einmalig umschulden"
	loan_reschedule.pressed.connect(func():
		var error: String = simulation.reschedule_loan()
		loan_result.text = error if not error.is_empty() else "Kredit umgeschuldet. Gebühr gebucht; Frist bleibt bestehen."
		refresh(true))
	actions.add_child(loan_reschedule)
	loan_result = UI.label("")
	loan_result.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	loans.add_child(loan_result)
	var aircraft := UI.panel(self, "FLUGZEUGBUCHUNGEN / LETZTE 100", null, true)
	aircraft_journal = UI.table(["Zeit", "Buchung", "Geldbewegung"], [170, 330, 140], 160)
	aircraft.add_child(aircraft_journal)
	var company := UI.panel(self, "WIRTSCHAFTLICHE ENTWICKLUNG")
	chart_select = OptionButton.new()
	chart_select.fit_to_longest_item = false
	for title in ["Kontostand · EUR", tr("ANCILLARY_OPERATING_REVENUE"), "Kosten inkl. Zinsen, Gebühren, Leasing und Steuern · EUR je Zeitabschnitt", "Ergebnis · EUR je Zeitabschnitt", "Ertragsteuer · EUR je Zeitabschnitt", tr("ANCILLARY_PERIOD"), tr("LOYALTY_COST_PERIOD")]:
		chart_select.add_item(title)
	chart_select.item_selected.connect(func(_i: int): _chart())
	company.add_child(chart_select)
	chart_comparison = UI.label("", ThemeTokens.CODE)
	chart_comparison.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	company.add_child(chart_comparison)
	chart = Chart.new()
	company.add_child(chart)
	chart_range = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	chart_range.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	company.add_child(chart_range)
	var ferries := UI.panel(self, "ÜBERFÜHRUNGEN  /  FLUGARCHIV · LETZTE 100")
	ferry_history = UI.table(["Landung", "Flugzeug", "Strecke", "Kosten"], [160, 105, 130, 120], 150)
	ferries.add_child(ferry_history)
	export_button = Button.new()
	export_button.text = "Als CSV exportieren…"
	export_button.tooltip_text = "Routenergebnisse als CSV-Datei speichern (Route, Flüge, Umsatz, Kosten, Ergebnis in EUR)."
	export_button.pressed.connect(_open_export)
	var routes := UI.panel(self, "ROUTENCONTROLLING  /  ROUTE ANKLICKEN", export_button, true)
	export_status = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	export_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	export_status.visible = false
	export_dialog = FileDialog.new()
	export_dialog.file_mode = FileDialog.FILE_MODE_SAVE_FILE
	export_dialog.access = FileDialog.ACCESS_FILESYSTEM
	export_dialog.use_native_dialog = true
	export_dialog.title = "Routenergebnisse exportieren"
	export_dialog.filters = PackedStringArray(["*.csv ; CSV-Datei"])
	export_dialog.current_file = "routenergebnisse.csv"
	export_dialog.file_selected.connect(export_route_csv)
	add_child(export_dialog)
	route_tree = UI.table(["Route / Strecke", "Flüge", "Gesamtumsatz", "Flugkosten", "Ergebnis", "Auslastung", "Fracht kg / EUR (in Umsatz)"], [190, 65, 125, 115, 115, 95, 260], 210)
	route_tree.item_selected.connect(_select_route)
	routes.add_child(route_tree)
	routes.add_child(export_status)
	var history := UI.panel(self, "ROUTENDETAILS  /  FLUGARCHIV")
	detail = UI.label("Wähle eine Route. Noch keine Landungen werden ausdrücklich als solche angezeigt.")
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	history.add_child(detail)
	load_chart = Chart.new()
	history.add_child(load_chart)
	history.add_child(UI.label("Auslastung je Zeitabschnitt · Lücken = keine Landungen · nach Sitzplätzen gewichtet", ThemeTokens.SMALL, UI.MUTED))
	flights_tree = UI.table(["Landung", "Richtung", "Ticketpreis je Pax", "Pax / Sitze", "Auslastung", "Gesamtumsatz", "Flugkosten", "Ergebnis", "Fracht kg / EUR (in Umsatz)"], [170, 140, 90, 95, 90, 110, 110, 110, 260], 250)
	history.add_child(flights_tree)
	# The archive count grows with the save; wrap actions when its label no longer fits.
	var paging := HFlowContainer.new()
	history.add_child(paging)
	previous_button = Button.new()
	previous_button.text = "Neuere Flüge"
	previous_button.pressed.connect(func(): offset = maxi(0, offset - 50); _flights())
	paging.add_child(previous_button)
	page_label = UI.label("")
	page_label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	paging.add_child(page_label)
	next_button = Button.new()
	next_button.text = "Ältere Flüge"
	next_button.pressed.connect(func(): offset += 50; _flights())
	paging.add_child(next_button)
	# Numeric headers stay aligned even before the archive has any entries.
	for definition in [[marketing_history, [1]], [tax_history, [0, 1, 2, 3, 4]], [aircraft_journal, [2]], [route_tree, [1, 2, 3, 4, 5, 6]], [flights_tree, [2, 3, 4, 5, 6, 7, 8]]]:
		for column in definition[1]:
			definition[0].set_column_title_alignment(column, HORIZONTAL_ALIGNMENT_RIGHT)

func _process(delta: float) -> void:
	if not is_visible_in_tree():
		return
	elapsed += delta
	if elapsed >= 0.5:
		elapsed = 0
		refresh()

func refresh(force: bool = false) -> void:
	var summary: Dictionary = simulation.get_financial_summary()
	var airline: Dictionary = simulation.get_airline_info()
	var financing: Dictionary = simulation.get_financing()
	var marketing: Dictionary = simulation.get_marketing()
	var fuel: Dictionary = simulation.get_fuel_contract()
	var insurance: Dictionary = simulation.get_insurance()
	var key := "%s/%s/%s/%s/%s/%s/%s/%s/%s" % [summary, airline, financing, marketing, fuel, insurance, simulation.get_ancillary_policy(), simulation.get_loyalty(), TranslationServer.get_locale()]
	if key == signature and not force:
		return
	signature = key
	ferry_history.clear()
	var ferry_root := ferry_history.create_item()
	for flight in simulation.get_ferry_archive(0):
		var row := ferry_history.create_item(ferry_root)
		for column in range(4):
			UI.cell_text(row, column, [flight["landing"], "AM-%06d" % flight["aircraft_id"], "%s–%s" % [flight["origin"], flight["destination"]], UI.money(flight["costs_eur"])][column])
	insurance_choice.select(["none", "basic", "full"].find(insurance["coverage"]))
	insurance_choice.disabled = not insurance["can_change"]
	insurance_status.text = "Basis: %s/Jahr · Selbstbehalt %.2f %% · Limit %.2f %% des Zeitwerts\nVoll: %s/Jahr · Selbstbehalt %.2f %% · Limit %.2f %% des Zeitwerts\nAktuelle Jahresprämie: %s · Schadenaufschlag %.2f %%\nPrämien: %s\nSelbstbehalte: %s\nUnversicherte Reparaturen: %s\nPrämien anteilig täglich; Flottenwert täglich aktualisiert. Deckungswechsel sofort, alte Prämie anteilig abgerechnet. Jährliche Schadenprüfung. Betriebskosten können das Konto ins Minus bringen." % [UI.money(insurance["basic_annual"]), insurance["basic_deductible_bps"] / 100.0, insurance["basic_limit_bps"] / 100.0, UI.money(insurance["full_annual"]), insurance["full_deductible_bps"] / 100.0, insurance["full_limit_bps"] / 100.0, UI.money(insurance["annual"]), insurance["loading_bps"] / 100.0, UI.money(insurance["premium"]), UI.money(insurance["deductible"]), UI.money(insurance["uninsured"])]
	for claim in insurance["claims"]:
		insurance_status.text += "\n%s · AM-%06d · %s · Reparatur %s · Eigenanteil %s · Versicherer %s" % [claim["time"], claim["aircraft"], claim["kind"], UI.money(claim["repair"]), UI.money(claim["paid"]), UI.money(claim["insurer"])]
	fuel_status.text = "Markt: %d ct/kg · Aufschlag: %.2f %% · Monat: %d Tage · Vertragseffekt seit Gründung: %s (positiv = Mehrkosten, negativ = Ersparnis)." % [fuel["market_cents"], fuel["premium_bps"] / 100.0, fuel["month_days"], UI.money(fuel["effect_eur"])]
	if fuel["active"]:
		fuel_status.text += "\nAktiv: %d %% zu %d ct/kg · Rest: %d Sekunden. Keine vorzeitige Kündigung." % [fuel["share"], fuel["locked_cents"], fuel["remaining_seconds"]]
	else:
		fuel_status.text += "\nKein aktiver Vertrag. In Notlage / Restrukturierung kein Neuabschluss."
	fuel_start.disabled = not fuel["can_start"]
	marketing_budget.max_value = marketing["max_budget_eur"]
	if not marketing_budget.get_line_edit().has_focus():
		marketing_budget.value = marketing["budget_eur"]
	marketing_apply.disabled = marketing["insolvent"]
	marketing_budget.editable = not marketing["insolvent"]
	marketing_status.text = "Budget: %s je %d Tage · Bekanntheit: %.2f / 100 · letzter Tag: %+.4f Punkte\nSchrittweiser Aufbau mit abnehmendem Nutzen; ohne Budget langsamer Rückgang. Kosten anteilig täglich und bei Budgetwechsel, steuerlich abziehbar.\nMarketingkosten insgesamt: %s" % [UI.money(marketing["budget_eur"]), marketing["month_days"], marketing["awareness"], marketing["trend"], UI.money(summary["marketing_eur"])]
	marketing_history.clear()
	var marketing_root := marketing_history.create_item()
	for entry in simulation.get_marketing_history():
		var row := marketing_history.create_item(marketing_root)
		UI.cell_text(row, 0, entry["time"])
		UI.cell_text(row, 1, UI.money(entry["amount_eur"]))
	loan_status.text = "Verfügbar: %s · Restschuld: %s · Zinsen / Gebühren bisher: %s\nNächste Rate: %s am %s (davon Zinsen: %s) · Überfällige Tilgung: %s\n%s" % [UI.money(financing["available_cash_eur"]), UI.money(financing["debt_eur"]), UI.money(financing["interest_eur"]), UI.money(financing["payment_eur"]), financing["next_due"], UI.money(financing["next_interest_eur"]), UI.money(financing["overdue_eur"]), financing["status"]]
	loan_status.text += "\nNächste Gehälter: %s am %s · Verfügbares Guthaben ist nicht für künftige Ausgaben reserviert." % [UI.money(financing["payroll_eur"]), financing["payroll_due"]]
	loan_status.add_theme_color_override("font_color", UI.NEGATIVE if financing["distressed"] else UI.MUTED)
	loan_status.text += "\nRestrukturierungsfrist: %s" % financing["deadline"]
	loan_rules.text = financing["rules"] + "\n" + financing["rescue_rules"]
	loan_reschedule.disabled = not financing["can_reschedule"]
	loan_borrow.disabled = not financing["can_borrow"]
	loan_repay.disabled = financing["insolvent"] or financing["debt_eur"] <= 0 or financing["available_cash_eur"] <= 0
	note.tooltip_text = "Treibstoff: weltweiter Preis, wöchentlich aktualisiert; Kosten bei Abflug fixiert."
	metrics["balance"].text = UI.money(airline.get("capital_eur", 0))
	metrics["balance"].add_theme_color_override("font_color", UI.NEGATIVE if airline.get("capital_eur", 0) < 0 else ThemeTokens.INK)
	metrics["revenue"].text = UI.money(summary["company_revenue_eur"])
	metrics["costs"].text = UI.money(summary["company_costs_eur"])
	metrics["profit"].text = UI.signed_money(summary["profit_eur"])
	metrics["ticket"].text = UI.money(summary["ticket_revenue_eur"])
	metrics["charter"].text = UI.money(summary["charter_revenue_eur"])
	metrics["penalty"].text = UI.money(summary["charter_penalty_eur"])
	metrics["ancillary"].text = UI.money(summary["ancillary_revenue_eur"])
	_refresh_ancillaries()
	_refresh_loyalty(summary)
	metrics["cargo"].text = UI.money(summary["cargo_revenue_eur"])
	metrics["profit"].add_theme_color_override("font_color", ThemeTokens.INK if summary["profit_eur"] >= 0 else UI.NEGATIVE)
	var info: Dictionary = simulation.get_finance_history_info()
	note.text = "Summen: gesamte erfasste Wirtschaft. Routenarchiv und Diagramme ab %s.\nBei älteren Spielständen fehlt frühere Detailhistorie; sie wird nicht nachberechnet. Flugzeugkäufe insgesamt: %s · Wartungschecks insgesamt: %s · Personalaufwand (Gehälter + Umschulung) insgesamt: %s." % [info.get("since", "—"), UI.money(summary["purchases_eur"]), UI.money(summary["maintenance_eur"]), UI.money(summary["payroll_eur"])]
	note.text += "\nPassagierbetreuung / Entschädigung (in Kosten und Steuerbasis enthalten): %s." % UI.money(summary["care_eur"])
	note.text += "\n" + tr("FREIGHT_FINANCE") % [summary["cargo_kg"], UI.money(summary["cargo_revenue_eur"])]
	note.text += "\nTreibstoff weltweit: %.2f EUR/kg · %s%d Cent/kg gegenüber vor 30 Tagen (wöchentlich)." % [summary["fuel_cents_per_kg"] / 100.0, "↑ +" if summary["fuel_month_delta_cents"] > 0 else ("↓ " if summary["fuel_month_delta_cents"] < 0 else "→ "), summary["fuel_month_delta_cents"]]
	note.text += "\nCO2-Abgabe (eigene Kostenzeile, in Gesamtkosten enthalten): %s · Preis jetzt %d EUR/t, nächste Stufe %d EUR/t." % [UI.money(summary["carbon_eur"]), summary["carbon_eur_per_tonne"], summary["carbon_next_eur_per_tonne"]]
	note.text += "\nÜberführungen (in Gesamtkosten enthalten): %s." % UI.money(summary["ferry_eur"])
	note.text += "\nLeasingraten (in Kosten enthalten): %s · Flugzeugjournal netto inkl. Verkäufen, Kautionen, Raten und Rückgaben: %s." % [UI.money(summary["rent_eur"]), UI.money(summary["aircraft_cash_eur"])]
	note.text += "\nBordservice (Kosten / Ersparnis gegenüber Standard, in Gesamtkosten enthalten): %s." % UI.money(summary["service_eur"])
	note.text += "\nErtragsteuer · letzter Abschluss Jahr %d: %s · Verlustvortrag: %s · Heimatsteuersatz: %.2f %% · Steuern insgesamt: %s. Abschluss alle 365 Tage; Jahr 0 = noch kein Abschluss." % [summary["tax_year"], UI.money(summary["tax_year_eur"]), UI.money(summary["tax_loss_eur"]), summary["tax_rate_bps"] / 100.0, UI.money(summary["tax_eur"])]
	tax_history.clear()
	var tax_root := tax_history.create_item()
	for assessment in simulation.get_tax_history():
		var tax_row := tax_history.create_item(tax_root)
		var tax_values := [str(assessment["year"]), UI.money(assessment["profit_eur"]), "%.2f %%" % (assessment["rate_bps"] / 100.0), UI.money(assessment["tax_eur"]), UI.money(assessment["loss_eur"])]
		for column in range(tax_values.size()):
			UI.cell_text(tax_row, column, tax_values[column])
		UI.cell_result(tax_row, 1, assessment["profit_eur"])
	aircraft_journal.clear()
	var journal_root := aircraft_journal.create_item()
	for entry in simulation.get_aircraft_cash_history():
		var row := aircraft_journal.create_item(journal_root)
		UI.cell_text(row, 0, entry["time"])
		UI.cell_text(row, 1, entry["kind"])
		UI.cell_text(row, 2, UI.money(entry["cash_eur"]))
	company_series = simulation.get_finance_series(0)
	chart_range.text = "%s → %s\nBis zu 64 Zeitpunkte · Werte im Tooltip · Käufe und Tilgung sind keine Betriebskosten; Zinsen und Leasingraten sind enthalten; Verkäufe und Kautionen erscheinen im Kontostand." % [company_series[0]["time"], company_series[-1]["time"]] if not company_series.is_empty() else "Noch keine Finanzhistorie."
	_chart()
	route_tree.clear()
	route_tree.set_meta("breakdowns", {})
	var root := route_tree.create_item()
	var selected: TreeItem = null
	flight_count = 0
	for r in simulation.get_route_finances():
		var row := route_tree.create_item(root)
		row.set_metadata(0, r)
		UI.cell_text(row, 0, "#%d %s–%s%s" % [r["route_id"], r["origin"], r["destination"], " · gelöscht" if not r["active"] else ""])
		UI.cell_text(row, 1, str(r["flights"]))
		for pair in [[2, "revenue_eur"], [3, "costs_eur"], [4, "profit_eur"]]:
			UI.cell_text(row, pair[0], UI.money(r[pair[1]]))
		UI.set_breakdown(route_tree, row, 2, UI.revenue_parts(r))
		UI.set_breakdown(route_tree, row, 3, UI.cost_parts(r))
		UI.cell_text(row, 6, "%d kg / %s" % [r["cargo_kg"], UI.money(r["cargo_revenue_eur"])])
		UI.cell_text(row, 5, "%.1f %%" % r["load_percent"] if r["flights"] > 0 else "Keine Flüge")
		if r["flights"] > 0:
			UI.cell_result(row, 4, r["profit_eur"])
		else:
			row.set_custom_color(4, UI.MUTED)
		if r["route_id"] == route_id:
			selected = row
	if selected != null:
		selected.select(0)
		_select_route()
	else:
		route_id = 0
		offset = 0
		_reset_flights()

func _open_export() -> void:
	export_dialog.popup_centered_ratio(0.6)

## Writes the current Rust route report; the simulation is only read.
func export_route_csv(path: String) -> String:
	if path.get_extension().to_lower() != "csv":
		path += ".csv"
	var error := RouteCsv.write(path, simulation.get_route_finances())
	export_status.visible = true
	export_status.text = ("Export gespeichert: %s" % path) if error.is_empty() else "Export fehlgeschlagen: %s" % error
	export_status.add_theme_color_override("font_color", UI.MUTED if error.is_empty() else UI.NEGATIVE)
	return error

func _chart() -> void:
	chart.set_series(company_series, ["balance", "revenue", "costs", "profit", "tax", "ancillary", "loyalty_cost"][chart_select.selected])
	chart.set_comparison("costs" if chart_select.selected == 1 else "revenue" if chart_select.selected == 2 else "")
	chart_comparison.visible = chart_select.selected in [1, 2]
	chart_comparison.text = chart_select.get_item_text(2 if chart_select.selected == 1 else 1)
	chart_comparison.add_theme_color_override("font_color", ThemeTokens.STOP if chart_select.selected == 1 else ThemeTokens.GO)

func _refresh_ancillaries() -> void:
	var data: Dictionary = simulation.get_ancillary_policy()
	var lines: PackedStringArray = []
	for i in range(data["options"].size()):
		var option: Dictionary = data["options"][i]
		var title := tr(["ANCILLARY_INCLUDED", "ANCILLARY_STANDARD", "ANCILLARY_AGGRESSIVE"][i])
		ancillary_buttons[i].set_pressed_no_signal(option["policy"] == data["policy"])
		ancillary_buttons[i].disabled = not data["can_change"]
		var line := tr("ANCILLARY_RATE") % [title, UI.money(option["economy_eur"]), UI.money(option["business_eur"]), option["attractiveness_percent"]]
		ancillary_buttons[i].tooltip_text = line
		lines.append(line)
	lines.append(tr("ANCILLARY_NOTE"))
	ancillary_status.text = "\n".join(lines)

func focus_lending() -> void:
	refresh(true)
	loan_status.focus_mode = Control.FOCUS_ALL
	loan_status.add_theme_stylebox_override("focus", loan_reschedule.get_theme_stylebox("focus", "Button"))
	var target: Control = loan_status
	for button: Button in [loan_reschedule, loan_repay, loan_borrow]:
		if not button.disabled:
			target = button
			break
	for frame in range(3):
		await get_tree().process_frame
	var scroll := get_parent().get_parent() as ScrollContainer
	var section := loan_status.get_parent() as Control
	var top := section.global_position.y - scroll.global_position.y + scroll.scroll_vertical
	var bottom := target.global_position.y + target.size.y - scroll.global_position.y + scroll.scroll_vertical
	# Show the status and restriction text above the action when both fit.
	scroll.scroll_vertical = int(top) if bottom - top <= scroll.size.y else int(bottom - scroll.size.y * 0.6)
	await get_tree().process_frame
	target.grab_focus()


func _loan_action(borrowing: bool) -> void:
	var error: String = simulation.borrow_money(int(loan_amount.value)) if borrowing else simulation.repay_loan(int(loan_amount.value))
	loan_result.text = error if not error.is_empty() else "Kredit aufgenommen." if borrowing else "Tilgung gebucht."
	refresh(true)

func _select_route() -> void:
	var row := route_tree.get_selected()
	if row == null:
		return
	var r: Dictionary = row.get_metadata(0)
	if route_id != r["route_id"]:
		offset = 0
	route_id = r["route_id"]
	flight_count = r["flights"]
	detail.text = "Route #%d · %s–%s · %d Landungen · %s\nFlugkosten ohne zentrale Gehälter und Wartungschecks; diese sind im Unternehmensergebnis enthalten. Historische Crewkosten bleiben erhalten. Gesamtumsatz = Ticket + Fracht + Zusatzumsätze; Ergebnis = Gesamtumsatz − Flugkosten. Aufschlüsselung im Tooltip der Umsatz- und Kostenzellen." % [route_id, r["origin"], r["destination"], flight_count, "Keine Flüge" if flight_count == 0 else "Gewinn" if r["profit_eur"] >= 0 else "Verlust"]
	detail.text += "\n" + tr("ANCILLARY_ROUTE_REVENUE") % UI.money(r["ancillary_revenue_eur"])
	detail.text += "\n" + tr("LOYALTY_ROUTE_COST") % UI.money(r["loyalty_cost_eur"])
	detail.text += "\nBordservice (Kosten / Ersparnis gegenüber Standard): %s." % UI.money(r["service_eur"])
	detail.text += "\nCO2-Abgabe (in Flugkosten enthalten): %s." % UI.money(r["carbon_eur"])
	load_chart.set_series(simulation.get_finance_series(route_id), "load", true)
	_flights()

func _reset_flights() -> void:
	detail.text = "Wähle eine Route · ohne Landungen gibt es noch keine Flugergebnisse."
	load_chart.set_series([], "load", true)
	_flights()

func _flights() -> void:
	flights_tree.clear()
	flights_tree.set_meta("breakdowns", {})
	var root := flights_tree.create_item()
	var rows: Array = simulation.get_route_flight_page(route_id, offset)
	for f in rows:
		var row := flights_tree.create_item(root)
		var values := [f["landing"], "%s → %s" % [f["origin"], f["destination"]], ("—" if f["seats"] == 0 else UI.money(f["ticket_eur"])), "%d / %d" % [f["passengers"], f["seats"]], "%.1f %%" % f["load_percent"], UI.money(f["revenue_eur"]), UI.money(f["costs_eur"]), UI.money(f["profit_eur"]), "%d kg / %s" % [f["cargo_kg"], UI.money(f["cargo_revenue_eur"])]]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
		UI.set_breakdown(flights_tree, row, 5, UI.revenue_parts(f))
		UI.set_breakdown(flights_tree, row, 6, UI.cost_parts(f))
		UI.cell_result(row, 7, f["profit_eur"])
	page_label.text = "%d–%d von %d · neueste zuerst" % [offset + 1 if not rows.is_empty() else 0, offset + rows.size(), flight_count]
	previous_button.disabled = offset == 0
	next_button.disabled = offset + 50 >= flight_count

func _refresh_loyalty(summary: Dictionary) -> void:
	var data: Dictionary = simulation.get_loyalty()
	loyalty_metrics["members"].text = UI.money(data["members"]).trim_suffix(" EUR")
	loyalty_metrics["trend"].text = ("+" if data["trend"] >= 0 else "") + UI.money(data["trend"]).trim_suffix(" EUR")
	loyalty_metrics["bonus"].text = "+%s %%" % UI.decimal(data["bonus_percent"], 2)
	loyalty_metrics["cost"].text = UI.money(summary["loyalty_cost_eur"])
	var lines: PackedStringArray = []
	for i in range(data["options"].size()):
		var option: Dictionary = data["options"][i]
		var title := tr(["LOYALTY_NONE", "LOYALTY_BASIC", "LOYALTY_GENEROUS"][i])
		loyalty_buttons[i].text = title
		loyalty_buttons[i].set_pressed_no_signal(option["level"] == data["level"])
		loyalty_buttons[i].disabled = not data["can_change"]
		var line := tr("LOYALTY_RATE") % [title, UI.money(option["economy_eur"]), UI.money(option["business_eur"]), option["enrolment_percent"]]
		loyalty_buttons[i].tooltip_text = line
		lines.append(line)
	lines.append(tr("LOYALTY_EFFECT") % [UI.decimal(data["max_bonus_percent"]), UI.money(data["saturation_members"]).trim_suffix(" EUR"), UI.decimal(data["lapse_percent"], 2)])
	lines.append(tr("LOYALTY_NOTE"))
	loyalty_status.text = "\n".join(lines)
	loyalty_history.set_column_title(0, tr("LOYALTY_PERIOD"))
	loyalty_history.set_column_title(1, tr("LOYALTY_COST"))
	chart_select.set_item_text(6, tr("LOYALTY_COST_PERIOD"))
	loyalty_history.clear()
	var root := loyalty_history.create_item()
	for period in simulation.get_finance_series(0):
		var row := loyalty_history.create_item(root)
		UI.cell_text(row, 0, period["time"])
		UI.cell_text(row, 1, UI.money(period["loyalty_cost"]))
