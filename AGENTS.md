# Airline Manager — agent notes

Godot 4.6+ UI over a Rust (godot-rust 0.5.5, edition 2024) simulation core. Docs/README are German.

## Rules
- All game rules and calculations live in Rust (`rust/src/`). GDScript only forwards input/frame delta and renders state. No second clock in GDScript.
- Tunable values: `config/*.json`; static world data: `data/*.json`.
- Long-term determinism: 1×/4×/16× and large/small steps and save/load must yield identical state.

## Map
- `rust/src/lib.rs` — GDExtension adapter only. Modules: `game` (airline state), `clock`, `aircraft`, `airports`, `routes`, `operations` (flights), `economy`, `demand`, `competition` (AI airlines), `maintenance`, `staff` (pilots/cabin/technicians, payroll), `history` (finance archive), `save` (versioned saves + migrations), `map`, `quality` (test-only long-run scenarios).
- `scripts/main.gd` — main UI; panels: `routes_panel.gd`, `finances_panel.gd`, `finance_chart.gd`, `map_*.gd`; theme in `scripts/ui/`.
- `tests/*_test.gd` — headless Godot tests; `tests/run_*_restart_test.ps1` — multi-process save/restart tests.
- UI kit (target look): `docs/ui-kit/` — `README.md` (rules), `tokens.json` (all colours/type/spacing), `components/<Name>/README.md` (spec + Godot mapping), `bundle.css` (exact sizes/states). For UI work read the README and only the component folders you implement; don't read `preview.html` files unless you need the markup.
- Design docs: `docs/game-design/` (decisions in `decisions/DECISION-REGISTER.md`); topic docs: `docs/{economy,demand,map,ui-design,staff}.md`; history: `docs/verification-log.md`.

## Commands
Use `AIRLINE_ENFORCE_PERF_BUDGET=1 cargo test --locked --manifest-path rust/Cargo.toml million_flight -- --test-threads=1` after long-advance changes; it prints wall and process CPU time and the enforced budget is process CPU time (the server is never quiet). Report both in the PR.
```
cargo build --locked --manifest-path rust/Cargo.toml
cargo test  --locked --manifest-path rust/Cargo.toml
cargo clippy --locked --manifest-path rust/Cargo.toml
Godot --headless --path . --script res://tests/<name>_test.gd
powershell -ExecutionPolicy Bypass -File tests/run_<name>_restart_test.ps1
START.bat --headless --quit-after 10   # full startup smoke test
```
Linux without a display (agent server): `tools/godot_check.sh test <name>` runs one Godot test headless, `tools/godot_check.sh tests` runs all of them, `tools/godot_check.sh shots <out-dir>` renders real screenshots of overview/fleet/routes at three window sizes via Xvfb. Look at the PNGs after UI changes. The PowerShell restart tests and START.bat stay Windows-only.

Before merging a change that touches GDScript, scenes, resources or a Rust API Godot calls, run `tools/godot_check.sh tests`. For a change to how the UI looks, also capture screenshots, open the PNGs (at least 1440x1000 and 680x520 of each screen you touched), fix clipped or overlapping text, values leaving their cards, wrong colours or fonts, and say in the PR which screenshots you checked. The capture covers overview, fleet and routes; extend the list in `tests/ui_layout_test.gd` when you change another screen. Rendering there is software OpenGL and can differ slightly from Windows.

CI (`.github/workflows/ci.yml`) verifies Rust (Clippy and tests), the Linux headless Godot suite and screenshot capture, Windows startup and current-save restart, and macOS native startup. The historical Windows migration scenarios run separately in `windows-compatibility.yml`. Run focused checks while developing and open a PR before waiting for full verification. For Paperclip, follow [the delivery runbook](docs/delivery/runbook.md); agents do not merge their own PRs.

Close Godot before rebuilding Rust (Windows locks the DLL). Godot path: `AIRLINE_GODOT` env var or `%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe`.

## Read less
- Find code with `bash tools/sym.sh <symbol> [lines]` (prints the definition) or `bash tools/sym.sh -u <text>` (lists where it is used), then `Read` with offset/limit. Don't read large files whole (`competition.rs`, `lib.rs`, `operations.rs`, `demand.rs`, `main.gd`). No raw `grep`/`rg`/`find`: their output is unbounded.
- Never read `rust/target/`, `.godot/`, `docs/screenshots/`, `data/world_land.geojson`, `Cargo.lock`, `*.uid`, `*.log` (denied for Claude in `.claude/settings.json`; other agents follow this by hand).
- Read `README.md`, `docs/game-design/` and `docs/verification-log.md` only when the task is about them.
- Keep command output small (`| tail -30`, `--quiet`). Run only the one relevant test (`cargo test <name>`, a single `*_test.gd`) until the final check.
