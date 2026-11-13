# Airline Manager — Claude notes

Godot 4.6+ UI over a Rust (godot-rust 0.5.5, edition 2024) simulation core. Docs/README are German.

## Rules
- All game rules and calculations live in Rust (`rust/src/`). GDScript only forwards input/frame delta and renders state. No second clock in GDScript.
- Tunable values: `config/*.json`; static world data: `data/*.json`.
- Long-term determinism: 1×/4×/16× and large/small steps and save/load must yield identical state.

## Map
- `rust/src/lib.rs` — GDExtension adapter only. Modules: `game` (airline state), `clock`, `aircraft`, `airports`, `routes`, `operations` (flights), `economy`, `demand`, `competition` (AI airlines), `maintenance`, `staff` (pilots/cabin/technicians, payroll), `history` (finance archive), `save` (versioned saves + migrations), `map`, `quality` (test-only long-run scenarios).
- `scripts/main.gd` — main UI; panels: `routes_panel.gd`, `finances_panel.gd`, `finance_chart.gd`, `map_*.gd`; theme in `scripts/ui/`.
- `tests/*_test.gd` — headless Godot tests; `tests/run_*_restart_test.ps1` — multi-process save/restart tests.
- Design docs: `docs/game-design/` (decisions in `decisions/DECISION-REGISTER.md`); topic docs: `docs/{economy,demand,map,ui-design,staff}.md`; history: `docs/verification-log.md`.

## Commands
```
cargo build --locked --manifest-path rust/Cargo.toml
cargo test  --locked --manifest-path rust/Cargo.toml
cargo clippy --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --script res://tests/<name>_test.gd
powershell -ExecutionPolicy Bypass -File tests/run_<name>_restart_test.ps1
START.bat --headless --quit-after 10   # full startup smoke test
```
Close Godot before rebuilding Rust (Windows locks the DLL). Godot path: `AIRLINE_GODOT` env var or `%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe`.

## Token hygiene
- Don't read `rust/target/`, `.godot/`, `data/world_land.geojson`, `Cargo.lock`, `*.uid` (denied in `.claude/settings.json`).
- README.md is long: grep for the `##` heading you need and read only that range.
- Large files (`competition.rs`, `lib.rs`, `operations.rs`, `maintenance.rs`, `main.gd`): grep for the symbol first, then read with offset/limit.
- Run test suites with output filtered (e.g. `cargo test ... 2>&1 | tail -30`).
