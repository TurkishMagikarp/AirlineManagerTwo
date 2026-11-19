extends SceneTree

func _initialize() -> void:
	var panel = preload("res://scripts/routes_panel.gd")
	var routes := [
		{"airline": "B Air", "origin_icao": "EDDM", "destination_icao": "EDDF", "model": "A320"},
		{"airline": "A Air", "origin_icao": "EDDF", "destination_icao": "EDDM", "model": "ATR"},
		{"airline": "C Air", "origin_icao": "EDDF", "destination_icao": "EDDH", "model": "A320"},
	]
	var forward: Array = panel.rival_services(routes, "EDDF", "EDDM")
	var reverse: Array = panel.rival_services(routes, "EDDM", "EDDF")
	var ok: bool = forward.size() == 2 and forward[0]["airline"] == "A Air" and forward[1]["airline"] == "B Air"
	ok = ok and reverse.size() == 2 and reverse[0]["airline"] == "A Air"
	ok = ok and panel.rival_services(routes, "EDDH", "EDDM").is_empty() and panel.rival_services([], "EDDF", "EDDM").is_empty()
	print("rival_schedule_test: ", "PASS" if ok else "FAIL")
	quit(0 if ok else 1)
