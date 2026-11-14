# Airport slots: Linux verification

Environment: official Godot 4.7.2 stable Linux x86_64 (`ed1daf0bf`), release
archive checked against the official SHA512 manifest, freshly built debug Rust
GDExtension. Rendering uses Xvfb and Mesa llvmpipe OpenGL compatibility at
1100×720; this says nothing about native Windows rendering or scaling.

The slot regression now runs a synthetic grandfathered world with 21 player
FRA–MUC services at 120-minute intervals, one spare aircraft, adequate crew,
and both unchanged initial AI airlines. FRA reserves 126 player slots plus four
AI slots: **130 used, zero of 120 free per day**. Fixture construction preserves
JSON integer seeds and accounting bytes without decoding/re-encoding the world.

The test checks:

- v26→v27 identity migration and current v27 restore preserve the entire world.
- A fresh reader matches the writer immediately and after seven more days,
  including player/AI schedules, finances, maintenance, staff and API snapshots.
- Route preview, selected map airport detail and the single attention entry
  show the exact shared reservation count. Secondary airports stay unlimited.
- A full FRA disables creation and explains `FRA is full: 0 of 120 daily slots
  free (1 required)`; an API attempt leaves every owner's state unchanged.
- Releasing two schedules updates both views to **118 used, two free**, enables
  creation and retains near-full attention. Releasing the rest clears attention.
  These UI probes restore the full original world before continuation comparisons.

Run the new restart family on Linux (use an isolated temporary directory):

```bash
for version in v26 v27; do
  for action in write read; do
    "$AIRLINE_GODOT" --headless --path . --script res://tests/slots_test.gd -- \
      "--slots-mode=$action-$version" "--slots-save=$directory/$version.json" || exit
  done
done
```

`tests/run_slots_restart_test.ps1` provides the corresponding owner-run Windows
wrapper. The default `slots_test.gd` mode checks the UI; adding
`--capture-dir=<existing directory>` in a rendered process writes preview and map
captures. Both Linux captures are attached to Paperclip AIR-95.

No production Rust, economy/configuration, UI or save-format changes were needed.

## Results (2026-11-14)

- `cargo build --locked --manifest-path rust/Cargo.toml`: passed.
- `cargo test --locked --manifest-path rust/Cargo.toml`: 226 passed, zero failures.
- `cargo clippy --locked --manifest-path rust/Cargo.toml`: passed without warnings.
- All 32 runnable `*_test.gd` scripts passed (save/restart scripts use their
  explicit modes). Linux main-scene smoke (`--headless --quit-after 10`) passed.
- All 19 restart families passed with 62 successful separate Linux process
  invocations: airport, cabin, cargo, competition, disruption, economy, finances,
  financing, flight, fuel contract, insurance, maintenance, marketing, pilot,
  save, service, slots, staff and tax. The PowerShell wrappers were not executed.
- Both slot restart pairs passed immediately and after seven days; the updated
  live-release UI probes also passed in the default test and both restart pairs.
- Real Xvfb preview/map captures passed and were visually inspected. The map
  benchmark completed on llvmpipe; its timings are software-renderer/container
  measurements, not a native GPU performance claim.

Tooling observations: the editor import command imported resources, then crashed
on editor shutdown with a Godot `cowdata.h` bounds error. Runtime test processes
use the imported project directly. The first rendered startup lacked XCursor;
after supplying the missing Linux libraries, rendering succeeded (audio fell
back to Dummy; V-Sync is unsupported under this Xvfb driver). Initial 150-second
finance stress-test attempts timed out without an assertion failure; the longer
900-second-budget writer and fresh reader both passed. The resulting archive
was 223,100,603 bytes. The same finance script passed in explicit writer mode
instead of repeating the timed-out default invocation.

Owner checks remain **unrun**: `START.bat`, every PowerShell restart wrapper,
native Windows DLL/process restart behavior, and Windows rendering/scaling.
Linux script invocations do not certify any of those Windows paths.
