# Loyalty verification — AIR-253

PR: https://github.com/TurkishMagikarp/AirlineManagerTwo/pull/78

## Functional and visual evidence

The implementation-session results recorded in the PR are reused here; this
continuation changes documentation only. Locked Rust build and clippy passed.
The full Rust suite excluding the separately measured million-flight tests had
374 passes, seven existing ignored tests and passing doc-tests. The normal
million-flight scenario previously passed at 246.17 s, including save/reload and
continuation. All runnable Godot scripts passed, as did separate-process v18 tax
migration checks.

The implementation session captured and opened Finances screenshots in German
and English at 1440×1000 and 680×520 and checked the kit layout, text, controls
and values. The screenshot archive is attached to the issue. No new visual check
was performed during this documentation-only continuation. START.bat, Windows
PowerShell restart drivers and real GPU rendering remain unverified; Linux
captures use software OpenGL.

## AI comparison

Same initial EDDF world, loyalty enabled / disabled; both airlines remain solvent
and neither distressed nor restructuring at both checkpoints.

| Year | Airline | Fleet | Cash EUR |
| --- | --- | --- | --- |
| 2 | Regional Wings | 6 / 6 | 107,649,134 / 116,187,129 |
| 2 | Continental Sky | 5 / 5 | 41,816,476 / 42,705,584 |
| 10 | Regional Wings | 6 / 6 | 553,067,033 / 599,495,783 |
| 10 | Continental Sky | 6 / 6 | 378,920,685 / 403,738,665 |

## Performance continuation, 2026-11-16

The enforced retry on implementation revision `c4613d4` failed: advance
315.450259021 s against the unchanged 300 s limit; 5,783,981 settled flights and
transfer counters (32,627, 100,066, 100,066). The compact-archive test passed.
The failure occurs before the finance scenario's save/load assertions.

Following the Planner's task directive, compare the branch's neutral None path
with unchanged main `6dda291` using the unoptimized debug test profile, locked
dependencies, one test thread, and the same exact test and flags. Runs are
sequential in the same session. The optional timing assertion is disabled for
this comparison so both runs can reach the functional save/load assertions.
No test, 300 s threshold or production rule is modified for this continuation.

```bash
export RUSTUP_HOME=/paperclip/airline/rustup
export CARGO_HOME=/paperclip/airline/cargo
export PATH=/paperclip/airline/cargo/bin:/paperclip/airline/bin:$PATH
unset AIRLINE_ENFORCE_PERF_BUDGET
AIRLINE_PERF_LOYALTY_NONE=1 cargo test --locked --manifest-path rust/Cargo.toml \
  million_flight_finance_scenario_stays_within_budget -- \
  --exact history::tests::million_flight_finance_scenario_stays_within_budget \
  --test-threads=1 --nocapture
```

On main the loyalty environment switch has no effect because loyalty is absent.
Main initially stopped before executing the test because the compiler wrapper
directory was absent from PATH; adding it allowed the build to proceed. That
attempt produced no timing sample. Build time is excluded from the advance timer.

The host has other active workloads. A single sequential pair can demonstrate
no observed slowdown in this sample, but cannot isolate small regressions or
prove that host contention alone explains absolute timing variation.

| Revision / mode | Advance | Flights | Transfer counters | Functional result |
| --- | ---: | ---: | --- | --- |
| Branch `c4613d4`, None | 302.352475439 s | 5,779,250 | (37,450, 106,623, 106,623) | Passed, including save/load/continuation |
| Main `6dda291` | 258.519742800 s | 5,779,250 | (37,450, 106,623, 106,623) | Passed, including save/load/continuation |

The branch is 43.832732639 s (**16.96%**) slower in this pair, exceeding the
Planner's approximately 5% fallback criterion. Including save/load/continuation,
the internal timers were 500.784895097 s and 411.900995500 s. The supplied older
main reference was 205.70 s; today's main was 25.68% slower than that reference.
These results do not establish the cause of the difference. Neither the absolute
retry nor the relative fallback qualifies this PR for merge. PR #78 remains
unmerged; profiling/correction of the neutral path and qualifying performance
evidence remain necessary. No additional benchmark retry was made this run.
