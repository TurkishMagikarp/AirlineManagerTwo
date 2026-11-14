extends VBoxContainer
const ThemeTokens = preload("res://scripts/ui/operations_theme.gd")
## All balances, route aggregates, period values and flight pages come from Rust.
const UI = preload("res://scripts/ui/components.gd")
const Chart = preload("res://scripts/finance_chart.gd")
var simulation: RefCounted
var metrics: Dictionary = {}
var note: Label
var chart: Control
var load_chart: Control
var chart_select: OptionButton
var route_tree: Tree
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
var marketing_history: Tree

func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 4 if grid.size.x >= 1150 else 2)
	for item in [["balance", "KONTOSTAND"], ["revenue", "TICKETUMSÄTZE"], ["cargo", "FRACHTUMSÄTZE"], ["costs", "GESAMTKOSTEN"], ["profit", "GEWINN / VERLUST"]]:
		metrics[item[0]] = UI.metric(grid, item[1])
	note = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(note)
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
	var taxes := UI.panel(self, "ERTRAGSTEUER / JAHRESABSCHLÜSSE")
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
	var aircraft := UI.panel(self, "FLUGZEUGBUCHUNGEN / LETZTE 100")
	aircraft_journal = UI.table(["Zeit", "Buchung", "Geldbewegung"], [170, 330, 140], 160)
	aircraft.add_child(aircraft_journal)
	var company := UI.panel(self, "WIRTSCHAFTLICHE ENTWICKLUNG")
	chart_select = OptionButton.new()
	for title in ["Kontostand · EUR", "Ticket- und Frachteinnahmen · EUR je Zeitabschnitt", "Kosten inkl. Zinsen, Gebühren, Leasing und Steuern · EUR je Zeitabschnitt", "Ergebnis · EUR je Zeitabschnitt", "Ertragsteuer · EUR je Zeitabschnitt"]:
		chart_select.add_item(title)
	chart_select.item_selected.connect(func(_i: int): _chart())
	company.add_child(chart_select)
	chart = Chart.new()
	company.add_child(chart)
	chart_range = UI.label("", ThemeTokens.SMALL, UI.MUTED)
	chart_range.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	company.add_child(chart_range)
	var routes := UI.panel(self, "ROUTENCONTROLLING  /  ROUTE ANKLICKEN")
	route_tree = UI.table(["Route / Strecke", "Flüge", "Tickets", "Kosten", "Ergebnis", "Auslastung", "Fracht kg / EUR"], [190, 65, 115, 115, 115, 95, 150], 210)
	route_tree.item_selected.connect(_select_route)
	routes.add_child(route_tree)
	var history := UI.panel(self, "ROUTENDETAILS  /  FLUGARCHIV")
	detail = UI.label("Wähle eine Route. Noch keine Landungen werden ausdrücklich als solche angezeigt.")
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	history.add_child(detail)
	load_chart = Chart.new()
	history.add_child(load_chart)
	history.add_child(UI.label("Auslastung je Zeitabschnitt · Lücken = keine Landungen · nach Sitzplätzen gewichtet", ThemeTokens.SMALL, UI.MUTED))
	flights_tree = UI.table(["Landung", "Richtung", "Ticket", "Pax / Sitze", "Auslastung", "Tickets", "Kosten", "Ergebnis", "Fracht kg / EUR"], [170, 140, 90, 95, 90, 110, 110, 110, 150], 250)
	history.add_child(flights_tree)
	var paging := HBoxContainer.new()
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
	var key := "%s/%s/%s/%s/%s/%s" % [summary, airline, financing, marketing, fuel, insurance]
	if key == signature and not force:
		return
	signature = key
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
	for field in ["revenue", "costs", "profit"]:
		metrics[field].text = UI.money(summary[field + "_eur"])
	metrics["revenue"].text = UI.money(summary["ticket_revenue_eur"])
	metrics["cargo"].text = UI.money(summary["cargo_revenue_eur"])
	metrics["profit"].add_theme_color_override("font_color", ThemeTokens.INK if summary["profit_eur"] >= 0 else UI.NEGATIVE)
	var info: Dictionary = simulation.get_finance_history_info()
	note.text = "Summen: gesamte erfasste Wirtschaft. Routenarchiv und Diagramme ab %s.\nBei älteren Spielständen fehlt frühere Detailhistorie; sie wird nicht nachberechnet. Flugzeugkäufe insgesamt: %s · Wartungschecks insgesamt: %s · Personalaufwand (Gehälter + Umschulung) insgesamt: %s." % [info.get("since", "—"), UI.money(summary["purchases_eur"]), UI.money(summary["maintenance_eur"]), UI.money(summary["payroll_eur"])]
	note.text += "\nPassagierbetreuung / Entschädigung (in Kosten und Steuerbasis enthalten): %s." % UI.money(summary["care_eur"])
	note.text += "\nBauchfracht: %d kg · %s Umsatz (im Ergebnis und in der Ertragsteuer enthalten)." % [summary["cargo_kg"], UI.money(summary["cargo_revenue_eur"])]
	note.text += "\nTreibstoff weltweit: %.2f EUR/kg · %s%d Cent/kg gegenüber vor 30 Tagen (wöchentlich)." % [summary["fuel_cents_per_kg"] / 100.0, "↑ +" if summary["fuel_month_delta_cents"] > 0 else ("↓ " if summary["fuel_month_delta_cents"] < 0 else "→ "), summary["fuel_month_delta_cents"]]
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
	var root := route_tree.create_item()
	var selected: TreeItem = null
	flight_count = 0
	for r in simulation.get_route_finances():
		var row := route_tree.create_item(root)
		row.set_metadata(0, r)
		UI.cell_text(row, 0, "#%d %s–%s%s" % [r["route_id"], r["origin"], r["destination"], " · gelöscht" if not r["active"] else ""])
		UI.cell_text(row, 1, str(r["flights"]))
		row.set_tooltip_text(2, "Economy: %s (%d Pax) · Business: %s (%d Pax)" % [UI.money(r["economy_revenue_eur"]), r["economy_passengers"], UI.money(r["business_revenue_eur"]), r["business_passengers"]])
		for pair in [[2, "revenue_eur"], [3, "costs_eur"], [4, "profit_eur"]]:
			UI.cell_text(row, pair[0], UI.money(r[pair[1]]))
		UI.cell_text(row, 2, UI.money(r["ticket_revenue_eur"]))
		row.set_tooltip_text(3, "Davon Betreuung / Entschädigung: %s" % UI.money(r["care_eur"]))
		UI.cell_text(row, 6, "%d kg / %s" % [r["cargo_kg"], UI.money(r["cargo_revenue_eur"])])
		UI.cell_text(row, 5, "%.1f %%" % r["load_percent"] if r["flights"] > 0 else "Keine Flüge")
		row.set_custom_color(4, UI.MUTED if r["flights"] == 0 else UI.POSITIVE if r["profit_eur"] >= 0 else UI.NEGATIVE)
		if r["route_id"] == route_id:
			selected = row
	if selected != null:
		selected.select(0)
		_select_route()
	else:
		route_id = 0
		offset = 0
		_reset_flights()

func _chart() -> void:
	chart.set_series(company_series, ["balance", "revenue", "costs", "profit", "tax"][chart_select.selected])

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
	detail.text = "Route #%d · %s–%s · %d Landungen · %s\nFlugkosten ohne zentrale Gehälter und Wartungschecks; diese sind im Unternehmensergebnis enthalten. Historische Crewkosten bleiben erhalten. Kostenaufschlüsselung und Ticketpreis im Flug-Tooltip." % [route_id, r["origin"], r["destination"], flight_count, "Keine Flüge" if flight_count == 0 else "Gewinn" if r["profit_eur"] >= 0 else "Verlust"]
	detail.text += "\nBordservice (Kosten / Ersparnis gegenüber Standard): %s." % UI.money(r["service_eur"])
	load_chart.set_series(simulation.get_finance_series(route_id), "load", true)
	_flights()

func _reset_flights() -> void:
	detail.text = "Wähle eine Route · ohne Landungen gibt es noch keine Flugergebnisse."
	load_chart.set_series([], "load", true)
	_flights()

func _flights() -> void:
	flights_tree.clear()
	var root := flights_tree.create_item()
	var rows: Array = simulation.get_route_flight_page(route_id, offset)
	for f in rows:
		var row := flights_tree.create_item(root)
		var values := [f["landing"], "%s → %s" % [f["origin"], f["destination"]], UI.money(f["ticket_eur"]), "%d / %d" % [f["passengers"], f["seats"]], "%.1f %%" % f["load_percent"], UI.money(f["ticket_revenue_eur"]), UI.money(f["costs_eur"]), UI.money(f["profit_eur"]), "%d kg / %s" % [f["cargo_kg"], UI.money(f["cargo_revenue_eur"])]]
		for i in range(values.size()):
			UI.cell_text(row, i, values[i])
			row.set_tooltip_text(i, "Ticket: %s\nTreibstoff: %s\nFlughäfen: %s\nPersonal: %s\nWartung / Verwaltung: %s\nBordservice (± Standard): %s" % [UI.money(f["ticket_eur"]), UI.money(f["fuel_eur"]), UI.money(f["airport_eur"]), UI.money(f["crew_eur"]), UI.money(f["overhead_eur"]), UI.money(f["service_eur"])])
			row.set_tooltip_text(i, row.get_tooltip_text(i) + "\nBetreuung / Entschädigung: %s" % UI.money(f["care_eur"]))
		row.set_custom_color(7, UI.POSITIVE if f["profit_eur"] >= 0 else UI.NEGATIVE)
	page_label.text = "%d–%d von %d · neueste zuerst" % [offset + 1 if not rows.is_empty() else 0, offset + rows.size(), flight_count]
	previous_button.disabled = offset == 0
	next_button.disabled = offset + 50 >= flight_count
