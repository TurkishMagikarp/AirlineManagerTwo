extends RefCounted
## Real-time autosave scheduling and preferences. Application time only: nothing here enters simulation state.
const INTERVALS: Array[int] = [1, 5, 10, 15, 30]
const DEFAULT_INTERVAL := 10
var enabled := false
var interval_minutes := DEFAULT_INTERVAL
var elapsed := 0.0
var pending := false
var preferences_path: String

func _init(path: String) -> void:
	preferences_path = path
	var config := ConfigFile.new()
	if config.load(path) != OK: return
	enabled = config.get_value("autosave", "enabled", false) == true
	var minutes = config.get_value("autosave", "interval_minutes", DEFAULT_INTERVAL)
	interval_minutes = minutes if minutes is int and minutes in INTERVALS else DEFAULT_INTERVAL

func configure(on: bool, minutes: int) -> void:
	enabled = on
	interval_minutes = minutes if minutes in INTERVALS else DEFAULT_INTERVAL
	elapsed = 0.0
	pending = false
	var config := ConfigFile.new()
	config.load(preferences_path)
	config.set_value("autosave", "enabled", enabled)
	config.set_value("autosave", "interval_minutes", interval_minutes)
	config.save(preferences_path)

## True when a write should happen now. A save that falls due while `ready` is false stays pending.
func tick(delta: float, ready: bool) -> bool:
	if not enabled:
		elapsed = 0.0
		pending = false
		return false
	elapsed += delta
	if elapsed >= interval_minutes * 60.0: pending = true
	if pending and ready:
		pending = false
		elapsed = 0.0
		return true
	return false
