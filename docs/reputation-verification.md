# Punctuality reputation: Linux verification

Environment: official Godot 4.7.2 stable Linux x86_64 (`ed1daf0bf`), release
archive checked against its SHA512 manifest, freshly built debug Rust
GDExtension. Captures use Xvfb and Mesa llvmpipe in OpenGL compatibility mode
at 1100×720. Native Windows rendering/scaling remains an owner check.

`tests/reputation_test.gd` builds a scheduled FRA–MUC player service and retains
both normal AI airlines. It advances to 21,700 seconds, with frozen airborne
departure quotes for all three owners, then seeds only their reputation fields:
score −800,000, falling trend and 1,000 pending late observations. These are
explicit synthetic fixture inputs, not a claim that 1,000 real flights occurred.
Every other world byte remains intact, including u64 seeds, fractional journals,
airborne quotes, crew and AI schedules. JSON is parsed only for assertions;
the serialized world never passes through Godot's floating-point JSON writer.

The default UI regression checks:

- Fleet/airline description percentage and trend match `get_financial_summary`.
  This is where PR #42 displays reputation; routes display punctuality instead.
- Every attention row matches the exact Rust `get_attention_snapshot` text;
  the poor, falling reputation produces exactly one reputation warning.
- Route tooltip and operations text match Rust punctuality percentages/counts.
- Rendering and page changes leave the complete serialized world and APIs intact.
- The first midnight consumes the seeded bad day and lowers reputation. Two
  subsequent days use real Rust flight operations, recover the score and change
  the trend to `steigend`. The normal presentation frame refreshes the fleet
  description and clears the warning without reloading a save or rebuilding the
  scene. Settled flight counts increase and punctuality exceeds 90%.

The separate-process modes cover both **v27→v28 migration** and **v28 restart**.
For v27 the test removes reputation from every owner, loads it, and compares the
entire serialized world and API snapshots with an independently loaded neutral
v28 baseline. No lifetime punctuality history is replayed. The v28 pair preserves
the negative score, falling trend and nonzero pending observations. Each fresh
reader compares full player/AI world bytes and API snapshots immediately after
load and after exactly seven further simulated days. All three owners must
settle flights and update reputation during continuation.

Run with a fresh debug build and an imported Godot project:

```bash
cargo build --locked --manifest-path rust/Cargo.toml
"$AIRLINE_GODOT" --headless --editor --path . --import
"$AIRLINE_GODOT" --headless --path . --script res://tests/reputation_test.gd
directory=$(mktemp -d)
for version in v27 v28; do
  for action in write read; do
    "$AIRLINE_GODOT" --headless --path . --script res://tests/reputation_test.gd -- \
      "--reputation-mode=$action-$version" \
      "--reputation-save=$directory/$version.json" || exit
  done
done
```

`tests/run_reputation_restart_test.ps1` provides the matching Windows wrapper.
For captures, run the default UI mode in a rendered process and add
`-- --capture-dir=<existing directory>`. Captures of falling and recovering
reputation were visually inspected and uploaded to
[AIR-101](https://paperclip.iamsec.de/AIR/issues/AIR-101). The falling capture shows
−2.4% (`sinkend`) and the full warning; the recovering capture shows −2.0%
(`steigend`) with no warning. Text is readable and wraps within the fleet page.

No production code, economy/configuration or save-format changes were needed.

## Results (2026-11-14)

- `cargo build --locked --manifest-path rust/Cargo.toml`: passed.
- `cargo test --locked --manifest-path rust/Cargo.toml`: 232 passed, zero failures
  (409.57 seconds), including migration/determinism, million-flight and ten-year
  player/AI scenarios.
- `cargo clippy --locked --manifest-path rust/Cargo.toml`: passed without warnings.
- The default reputation UI regression and all four separate-process reputation
  restart modes passed.
- All 33 `*_test.gd` scripts passed through their supported modes. The default
  sweep excludes the explicit-mode-only save test and runs the finance stress
  script in its writer/reader modes instead of duplicating its default writer.
- All 20 restart families passed: airport, cabin, cargo, competition, disruption,
  economy, finances, financing, flight, fuel contract, insurance, maintenance,
  marketing, pilot, reputation, save, service, slots, staff and tax. Together with
  the default sweep and successful Linux main-scene smoke
  (`--headless --quit-after 10`), the final run completed **98 successful process
  invocations**, including **66 restart invocations**. This invokes the GDScript
  modes directly on Linux, not the PowerShell wrappers.
- The finance writer and fresh reader both passed within a 900-second harness
  budget; the saved archive was 226,214,437 bytes. No finance assertion or timeout
  failed in the final sweep.
- The editor resource import completed successfully. The real Xvfb captures
  passed and were visually inspected. The rendered map benchmark also completed;
  its llvmpipe timings describe this container's software renderer.

Owner checks remain **unrun**: `START.bat`, all PowerShell wrappers (including
the new wrapper), native Windows DLL/process restarts, and Windows
rendering/scaling. Linux process tests and software-rendered captures do not
certify those paths.
