# AIR-107 finance budget comparison

On 2026-11-14, compare the unchanged
`history::tests::million_flight_finance_scenario_stays_within_budget` before and
after maintenance bases (PR #44). The 300-second assertion remains unchanged.

## Revisions and method

- Before: `947b210`, the first parent of merge `97051b1`, immediately before #44.
- After: `97051b1`, the merge of #44.
- `0dc1801^` is `22d25e3` on the feature branch and already includes maintenance
  bases; it is not the pre-merge main baseline.
- Detached worktrees, identical locked dependencies, Rust 1.100.0 / Cargo 1.100.0,
  Linux x86_64, unoptimized debug test profile, one test thread, sequential runs,
  shared build cache outside the source trees. No Godot stress runs were started.
- Clear the `airline_sim` package from that cache before switching revisions:
  Cargo initially reused the before-revision executable when invoked from the
  after-revision worktree. That invalid attempt reported 309.982535034 s and
  231 filtered tests (the baseline count), and is excluded from the comparison.
  The rebuilt after-revision executable must report 235 filtered tests.
- The existing Zig 0.14.1 `cc` wrapper was added to PATH for both builds. The first
  baseline build attempt stopped before running tests because `cc` was absent
  from PATH; it yielded no timing measurement.
- This is a shared runner (six CPU quota); unrelated builds/processes were active.
  A single pair of wall-clock measurements cannot isolate small performance
  differences or prove that runner contention is the sole cause.

Command in each revision's worktree (after exporting the supplied Rust toolchain
and adding `/paperclip/airline/bin` to PATH):

```bash
set -o pipefail
cargo clean --manifest-path rust/Cargo.toml \
  --target-dir /paperclip/airline/perf-target -p airline_sim
cargo test --locked --manifest-path rust/Cargo.toml \
  --target-dir /paperclip/airline/perf-target \
  million_flight_finance_scenario_stays_within_budget -- \
  --exact history::tests::million_flight_finance_scenario_stays_within_budget \
  --nocapture --test-threads=1 2>&1 | tail -25
```

The reported duration is the test's internal `g.advance` timer, excluding build
and fixture setup. A budget assertion failure stops before the later save/load
and continuation checks, so those checks are not verified by a failing run.

## Results

| Revision | Advance time | Result |
| --- | ---: | --- |
| `947b210` (before #44) | 325.017163925 s | Failed: above 300 s |
| `97051b1` (after #44, rebuilt) | 275.151906381 s | Passed, including save/load and continuation |

The after revision was 49.865 s (15.34%) faster in this pair. This provides no
evidence that maintenance bases slowed this scenario. The before revision itself
exceeded the bound; the variation across these and the earlier runs is consistent
with runner sensitivity, but the experiment does not prove its sole cause.
Production code and the bound have not been changed.

The rebuilt after test reported 235 filtered tests, confirming the four new
maintenance-base tests were present. Its save was 224,672,631 bytes; the internal
timer including save/load/continuation was 389.514848368 s, and the full test took
390.55 s. Only `advance` is subject to the 300-second assertion. A long-running
process alone therefore cannot determine whether that assertion failed.

## Final PR checks

After clearing the crate cache again and rebuilding the AIR-107 PR worktree, the
full locked Rust suite passed serially: **236 passed, 0 failed** in 758.32 s,
including the unchanged finance budget test, AI scenarios, determinism and save
migrations. The command was `cargo test --locked --manifest-path rust/Cargo.toml
--target-dir /paperclip/airline/perf-target -- --test-threads=1`, with output
filtered through `tail -30`. Doc-tests also passed (zero tests).

`cargo clippy --locked --manifest-path rust/Cargo.toml --target-dir
/paperclip/airline/perf-target`, filtered through `tail -25`, passed without
warnings. The previously recorded Godot verification is reused; no Godot, GUI,
START.bat, PowerShell wrapper or native Windows checks were run in this comparison
session. Windows checks remain owner-only as listed in PR #45.
