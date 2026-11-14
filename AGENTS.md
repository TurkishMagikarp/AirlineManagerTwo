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
- UI kit (target look): `docs/ui-kit/` — `README.md` (rules), `tokens.json` (all colours/type/spacing), `components/<Name>/README.md` (spec + Godot mapping), `bundle.css` (exact sizes/states). For UI work read the README and only the component folders you implement; don't read `preview.html` files unless you need the markup.
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
Linux without a display (agent server): `tools/godot_check.sh test <name>` runs one Godot test headless, `tools/godot_check.sh tests` runs all of them, `tools/godot_check.sh shots <out-dir>` renders real screenshots of overview/fleet/routes at three window sizes via Xvfb. Look at the PNGs after UI changes. The PowerShell restart tests and START.bat stay Windows-only.

Close Godot before rebuilding Rust (Windows locks the DLL). Godot path: `AIRLINE_GODOT` env var or `%USERPROFILE%\Documents\Godot\Godot_v4.7.2-stable_win64.exe`.

## Token hygiene
- Don't read `rust/target/`, `.godot/`, `data/world_land.geojson`, `Cargo.lock`, `*.uid` (denied in `.claude/settings.json`).
- README.md is long: look up the heading's line in the last section of `docs/CODEMAP.md` and read that range.
- Large files (`competition.rs`, `lib.rs`, `operations.rs`, `maintenance.rs`, `main.gd`): use `tools/sym.sh <symbol>` or CODEMAP, then read with offset/limit.
- Run test suites with output filtered (e.g. `cargo test ... 2>&1 | tail -30`).

## Navigation (read less)
- **No grep/rg/find/findstr/Select-String.** Use `bash tools/sym.sh <symbol> [lines]` to print a symbol's definition, or look it up in `docs/CODEMAP.md`.
- `docs/CODEMAP.md` lists every fn/struct/func with its line number. Look there first, then `Read` with offset/limit — don't grep or read whole files. Regenerate with `bash tools/gen_codemap.sh` after adding/moving symbols.
- Edit with `Edit` on the exact range; don't re-read a file after editing.
- Run only the one relevant test (`cargo test <name>`, a single `*_test.gd`), not the full suites, until the final check.
- Don't read `docs/verification-log.md` (history only), README.md, or `docs/game-design/` unless the task is about them.

## Agent-neutral rules
- Tool-level read denies exist only in `.claude/settings.json`; other agents must follow them by hand: never read `rust/target/`, `.godot/`, `docs/screenshots/`, `data/world_land.geojson`, `Cargo.lock`, `*.uid`, `*.log`.
- Keep command output small: pipe through `tail`/`head`, use `cargo test <name>`, `--quiet`.
- Prefer small ranged reads (e.g. `sed -n 100,160p file`) over `cat` of whole files.
- Commit hook (`.githooks/pre-commit`) regenerates `docs/CODEMAP.md`; enable once with `git config core.hooksPath .githooks`.
