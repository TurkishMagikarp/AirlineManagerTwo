extends VBoxContainer
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

func _ready() -> void:
	add_theme_constant_override("separation", 12)
	var grid := GridContainer.new()
	grid.add_theme_constant_override("h_separation", 12)
	grid.add_theme_constant_override("v_separation", 12)
	add_child(grid)
	grid.resized.connect(func(): grid.columns = 4 if grid.size.x >= 1150 else 2)
	for item in [["balance", "KONTOSTAND"], ["revenue", "TICKETUMSÄTZE"], ["costs", "BETRIEBSKOSTEN"], ["profit", "GEWINN / VERLUST"]]:
		metrics[item[0]] = UI.metric(grid, item[1])
	note = UI.label("", 13, UI.MUTED)
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	add_child(note)
	var company := UI.panel(self, "WIRTSCHAFTLICHE ENTWICKLUNG")
	chart_select = OptionButton.new()
	for title in ["Kontostand · EUR", "Ticketeinnahmen · EUR je Zeitabschnitt", "Betriebskosten · EUR je Zeitabschnitt", "Ergebnis · EUR je Zeitabschnitt"]:
		chart_select.add_item(title)
	chart_select.item_selected.connect(func(_i: int): _chart())
	company.add_child(chart_select)
	chart = Chart.new()
	company.add_child(chart)
	chart_range = UI.label("", 13, UI.MUTED)
	chart_range.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	company.add_child(chart_range)
	var routes := UI.panel(self, "ROUTENCONTROLLING  /  ROUTE ANKLICKEN")
	route_tree = UI.table(["Route / Strecke", "Flüge", "Umsatz", "Kosten", "Ergebnis", "Auslastung"], [190, 65, 115, 115, 115, 95], 210)
	route_tree.item_selected.connect(_select_route)
	routes.add_child(route_tree)
	var history := UI.panel(self, "ROUTENDETAILS  /  FLUGARCHIV")
	detail = UI.label("Wähle eine Route. Noch keine Landungen werden ausdrücklich als solche angezeigt.")
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	history.add_child(detail)
	load_chart = Chart.new()
	history.add_child(load_chart)
	history.add_child(UI.label("Auslastung je Zeitabschnitt · Lücken = keine Landungen · nach Sitzplätzen gewichtet", 13, UI.MUTED))
	flights_tree = UI.table(["Landung", "Richtung", "Ticket", "Pax / Sitze", "Auslastung", "Umsatz", "Kosten", "Ergebnis"], [170, 140, 90, 95, 90, 110, 110, 110], 250)
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
	var key := "%s/%s" % [summary, airline]
	if key == signature and not force:
		return
	signature = key
	metrics["balance"].text = UI.money(airline.get("capital_eur", 0))
	for field in ["revenue", "costs", "profit"]:
		metrics[field].text = UI.money(summary[field + "_eur"])
	metrics["profit"].add_theme_color_override("font_color", UI.POSITIVE if summary["profit_eur"] >= 0 else UI.NEGATIVE)
	var info: Dictionary = simulation.get_finance_history_info()
	note.text = "Summen: gesamte erfasste Wirtschaft. Routenarchiv und Diagramme ab %s.\nBei älteren Spielständen fehlt frühere Detailhistorie; sie wird nicht nachberechnet. Flugzeugkäufe insgesamt: %s · Wartungschecks insgesamt: %s · Gehälter insgesamt: %s." % [info.get("since", "—"), UI.money(summary["purchases_eur"]), UI.money(summary["maintenance_eur"]), UI.money(summary["payroll_eur"])]
	company_series = simulation.get_finance_series(0)
	chart_range.text = "%s → %s\nBis zu 64 Zeitpunkte · Werte im Tooltip · Käufe sind keine Betriebskosten." % [company_series[0]["time"], company_series[-1]["time"]] if not company_series.is_empty() else "Noch keine Finanzhistorie."
	_chart()
	route_tree.clear()
	var root := route_tree.create_item()
	var selected: TreeItem = null
	flight_count = 0
	for r in simulation.get_route_finances():
		var row := route_tree.create_item(root)
		row.set_metadata(0, r)
		row.set_text(0, "#%d %s–%s%s" % [r["route_id"], r["origin"], r["destination"], " · gelöscht" if not r["active"] else ""])
		row.set_text(1, str(r["flights"]))
		for pair in [[2, "revenue_eur"], [3, "costs_eur"], [4, "profit_eur"]]:
			row.set_text(pair[0], UI.money(r[pair[1]]))
		row.set_text(5, "%.1f %%" % r["load_percent"] if r["flights"] > 0 else "Keine Flüge")
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
	chart.set_series(company_series, ["balance", "revenue", "costs", "profit"][chart_select.selected])

func _select_route() -> void:
	var row := route_tree.get_selected()
	if row == null:
		return
	var r: Dictionary = row.get_metadata(0)
	if route_id != r["route_id"]:
		offset = 0
	route_id = r["route_id"]
	flight_count = r["flights"]
	detail.text = "Route #%d · %s–%s · %d Landungen · %s\nKostenaufschlüsselung und Ticketpreis im Flug-Tooltip." % [route_id, r["origin"], r["destination"], flight_count, "Keine Flüge" if flight_count == 0 else "Gewinn" if r["profit_eur"] >= 0 else "Verlust"]
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
		var values := [f["landing"], "%s → %s" % [f["origin"], f["destination"]], UI.money(f["ticket_eur"]), "%d / %d" % [f["passengers"], f["seats"]], "%.1f %%" % f["load_percent"], UI.money(f["revenue_eur"]), UI.money(f["costs_eur"]), UI.money(f["profit_eur"])]
		for i in range(values.size()):
			row.set_text(i, values[i])
			row.set_tooltip_text(i, "Ticket: %s\nTreibstoff: %s\nFlughäfen: %s\nPersonal: %s\nWartung / Verwaltung: %s" % [UI.money(f["ticket_eur"]), UI.money(f["fuel_eur"]), UI.money(f["airport_eur"]), UI.money(f["crew_eur"]), UI.money(f["overhead_eur"])])
		row.set_custom_color(7, UI.POSITIVE if f["profit_eur"] >= 0 else UI.NEGATIVE)
	page_label.text = "%d–%d von %d · neueste zuerst" % [offset + 1 if not rows.is_empty() else 0, offset + rows.size(), flight_count]
	previous_button.disabled = offset == 0
	next_button.disabled = offset + 50 >= flight_count
