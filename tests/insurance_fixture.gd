extends RefCounted
## Earlier feature migrations now also reset insurance. Compare their original
## fields losslessly; the v22 insurance migration has its own Rust regression.
static func without_policy(contents: String) -> String:
	return RegEx.create_from_string('"insurance":\\{(?:[^{}]|\\{[^{}]*\\})*\\}').sub(contents, '"insurance":null', true)
