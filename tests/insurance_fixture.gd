extends RefCounted
## Earlier feature migrations now also reset insurance. Compare their original
## fields losslessly; the v22 insurance migration has its own Rust regression.
static func without_policy(contents: String) -> String:
	return without_economy(RegEx.create_from_string('"insurance":\\{(?:[^{}]|\\{[^{}]*\\})*\\}').sub(contents, '"insurance":null', true))

## v34 has an independently tested neutral timeline migration.
static func without_economy(contents: String) -> String:
	return RegEx.create_from_string('"world_economy":\\{[^{}]*\\}').sub(contents, '"world_economy":null', true)

static func without_economy_snapshot(snapshot: Array) -> Array:
	var result := snapshot.duplicate(true)
	result[0] = without_economy(result[0])
	return result
