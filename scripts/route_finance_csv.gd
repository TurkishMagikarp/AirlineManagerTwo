extends RefCounted
## CSV encoding of Rust's get_route_finances(); values are copied, never recomputed.
const HEADER := ["route_id", "origin", "destination", "flights", "revenue_eur", "costs_eur", "profit_eur"]

static func build(rows: Array) -> String:
	var lines := PackedStringArray([",".join(HEADER)])
	for r in rows:
		lines.append("%d,%s,%s,%d,%d,%d,%d" % [r["route_id"], r["origin"], r["destination"], r["flights"], r["revenue_eur"], r["costs_eur"], r["profit_eur"]])
	return "\r\n".join(lines) + "\r\n"

## Returns an empty string on success, otherwise a player-facing error.
static func write(path: String, rows: Array) -> String:
	var file := FileAccess.open(path, FileAccess.WRITE)
	if file == null:
		return "Datei konnte nicht geschrieben werden (%s)." % error_string(FileAccess.get_open_error())
	file.store_string(build(rows))
	var error := file.get_error()
	file.close()
	return "" if error == OK else "Datei konnte nicht geschrieben werden (%s)." % error_string(error)
