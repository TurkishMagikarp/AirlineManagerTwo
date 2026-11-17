# Long-advance performance, AIR-239

## Scope and method

Base: `796220c` (PR #74). Existing debug tests, locked dependencies, the same
Rust toolchain and linker wrapper. The formal comparison runs the prebuilt main
binary first, then the prebuilt branch binary, without another repository build
or test of ours running. Bash `time` measures the **entire** exact stress test,
including its large save/reload/continuation; the existing test timer measures
only the advance. Other server workloads are not controlled.

The scenario, flight budget, 300-second assertion and opt-in environment switch
are unchanged. There are no tuning, simulation-rule, time-step, save-schema or
migration changes. All new caches/scratch are in unsaved derived helper types.

## Formal comparison

Prebuilt main ran first, followed immediately by the prebuilt delivery branch.
Both use `AIRLINE_ENFORCE_PERF_BUDGET=1` and `--test-threads=1`.

| Revision | Advance wall | Whole-test user CPU | Whole-test wall |
| --- | ---: | ---: | ---: |
| Main `796220c` | 287.784833 s | 395.063 s | 445.169 s |
| Delivery branch | 260.148141 s | 367.067 s | 418.771 s |

The advance improves by **27.636693 s (9.6%)**. The requested 200-second
target is **not met**; this is also below the suggested 15–20% gain.

The earlier fresh-main sample was 288.944553 s advance and 373.657 s user
CPU. The final branch saves 27.996 s (7.1%) of whole-test user CPU.
Both formal runs passed the unchanged 300 s assertion and their complete
save/reload/continuation checks, with 5,779,250 flights, 250,041,627-byte saves
and transfer counters `(37450, 106623, 106623)`.

Whole-test CPU includes the large serialization/reload and continuation;
it is not advance-only CPU.

## Profiling the regression

Temporary nested `Instant` probes were inserted into existing functions, first
for `profile_twenty_thousand_flights`, then for the complete million-flight
advance. They count calls and separate child time from own time. The probes and
state-dump hook are removed from the delivered code. No benchmark framework or
profiling tools are added to the repository. These are diagnostic timings, not
the uninstrumented performance gate; inclusive rows overlap.

Short historical runs at each feature merge ranged from 5.710 to 7.678 seconds
(PR #67: 6.658 s; current main: 6.133 s). They did not reliably explain the long-run
regression. The full instrumented PR #67 run on this server took **241.804 s**;
current main took **279.750 s**, with 5,784,017 / 5,779,250 flights respectively.
The published 175-second PR #67 result cannot be directly treated as today's
baseline: probes and uncontrolled server conditions differ. The full diagnostic
comparison reproduces about 38 seconds of extra advance cost, not 100 seconds.

| Full advance path (inclusive unless marked own) | PR #67 | Main #74 |
| --- | ---: | ---: |
| Company advancement | 162.651 s | 195.429 s |
| Quotes (dispatch and planning) | 55.634 s | 79.890 s |
| Prepared quote-market construction | 37.585 s | 43.203 s |
| Local passenger allocation | 32.612 s | 43.007 s |
| Weekly AI reviews | 27.047 s | 28.371 s |
| Company advancement, own time | 51.784 s | 56.437 s |
| Archive record | 11.488 s | 11.358 s |
| Disruption projection | 11.556 s | 11.568 s |

Feature attribution in the actual stress scenario:

- **World economy:** 13,998,224 index lookups cost 12.622 s; factors' own time is
  another 4.813 s, including the pre-existing seasonal calculation. Quotes grow
  by 24.256 s overall, including these paths and transfer handling. Transfers
  now retain the exact-time overbooking fallback: the shared estimator is called
  598,666 times versus 194,590 before; allocation counters are 106,623 versus
  99,061. Networks built increase from 28,560 to 37,450. The fallback remains.
- **Milestones:** 83,333 daily evaluations cost 4.585 s.
- **Fare advisor:** the shared weekly fare-review function costs 7.836 s for
  142,666 calls, including the AI fare search that existed previously. Total AI
  review time grows by 1.324 s; the empty player-autopilot pass costs 0.021 s.
- **Freighters:** market construction costs 4.248 s versus 3.615 s before;
  freight estimates cost 12.164 s including 5.281 s of economy lookups. Those
  totals include the existing belly-freight allocator, not just freighter work.
  This stress scenario flies passenger aircraft, not main-deck freighters.
- **Codeshares:** 219,220 agreement reviews cost 0.083 s. There are no agreements
  in this scenario, so this does not measure active partnership receipt costs.
- **Orders:** 328,830 empty-book passes cost 0.051 s. No delivery backlog exists
  in this scenario; this is not a measurement of active order fulfillment.
- **Difficulty:** Standard uses the existing fast scale path. Fuel lookups total
  3.084 s versus 2.694 s before; that difference is not an isolated estimate of
  difficulty's incremental cost. It does not explain the long-run regression.

Feature rows are whole-path costs, not additive isolated feature-ablation
measurements. In particular, economy time is nested inside passenger and freight
quotes; market preparation is also inside dispatch and AI reviews.

## Changes

- Reuse the raw economy index for an exactly matching seed and existing period;
  apply the requested crisis scale after the cache lookup. Shifted starts still
  compute their own period; no new time quantisation is introduced.
- Reuse a freight allocation only when the exact economy-adjusted base and all
  relevant integer passenger loads agree. A miss runs the original allocator.
- Avoid redundant own-offer and economy-cabin copies during market preparation;
  preserve cabin identity and the canonical floating-point reduction order.
- Retain transfer-load scratch across daily-cache hits instead of allocating a
  fresh load vector for each flight. Exact-time fallback and cache keys remain.
- Refresh reputation weights without rebuilding already canonical cabin storage;
  a changed ordering still takes the original stable canonicalization path.
- Avoid checked multiplication by one when adding a single flight to totals;
  all overflow checks that can fail are retained.
- Try a bounded direct forecast slot before the existing landing-time hash map;
  the complete immutable schedule key still guards every hit and collision.
- Sort staged recent flights, append when their keys follow the canonical
  ledger, and retain the full stable sort for deferred/interleaved arrivals.
  Trimming, transactional failure behavior and archive recording are unchanged.

## Remaining cost

The full candidate diagnostic still concentrates time in passenger quotes and
market construction: quote evaluation 81.318 s, market preparation 50.849 s,
local allocation 44.906 s and weekly AI reviews 32.708 s (inclusive and
overlapping). Settlement outside those children, disruption projection,
maintenance boundaries, history recording and milestone evaluation also remain.
This instrumented candidate run took 341.184 s overall; extra probes and server
conditions make those absolute times unsuitable for comparison with the formal
uninstrumented result. It predates the final reputation-order, single-flight
arithmetic and direct-forecast refinements. It identifies remaining work, not
an isolated cost or speedup for each final change.

Exact-time local demand and transfer overbooking fallback still have to run;
the implementation does not round their time or replace their allocations.
No actual shared-logic bug was found or fixed.

## Identity evidence

The complete compact JSON state after the existing 20,000-flight diagnostic is
byte-identical (`cmp`) between main and this implementation:

`df0c4090e0a056b54ea647410c8ddd642c73c6ddeb37af3a3315483b7e1a5d25` (SHA-256).

Both settle 111,091 flights; transfer counters are `(1632, 1912, 1912)`.
The final branch dump was rechecked after all candidate code changes; the
short diagnostic is used for identity, not the long-run performance claim.

Existing Standard EDDF manufacturer-order audit, daily steps:

| Years | Rival | Fleet, both | Cash EUR, both | Insolvent / distressed, both |
| --- | --- | ---: | ---: | --- |
| 2 | Regional Wings | 6 | 116,187,129 | false / false |
| 2 | Continental Sky | 5 | 42,705,584 | false / false |
| 10 | Regional Wings | 6 | 599,495,783 | false / false |
| 10 | Continental Sky | 6 | 403,738,665 | false / false |

New regression tests compare warmed freight scratch with fresh quotes across
loads, periods, clones and both freight-market types; interleave economy seeds,
start offsets and crisis settings; compare reputation refreshes with fresh
allocation when canonical order is stable or changes; check single-flight totals
and overflow; and compare recent-ledger updates with the previous full stable-sort
algorithm, including equal keys and deferred arrivals. The existing reschedule
collision test covers the forecast cache too.

## Final validation

- `cargo build --locked --manifest-path rust/Cargo.toml`: passed.
- `cargo test --locked --manifest-path rust/Cargo.toml -- --test-threads=2`:
  351 passed, 0 failed, 7 ignored; doc tests passed. The ignored profile and
  Standard EDDF audit were run explicitly as described above.
- `cargo clippy --locked --manifest-path rust/Cargo.toml`: passed without warnings.
- `tools/godot_check.sh tests`: 51 passed, 0 failed; the Windows-only
  `save_restart` driver was skipped by the runner.
- Both prebuilt enforced million-flight tests passed. Six focused cache,
  arithmetic, ordering and forecast regressions also passed.

The full Rust suite includes pause, 1x/4x/16x, frame partitioning, large/small
steps, time jumps, annual restart, old-save migration and save/reload equivalence.
Godot checks exercise the shared extension through simulation, economy, finance,
transfer, cargo, codeshare, orders and UI integration scripts.

No UI changed and no screenshots were requested or taken. Linux Godot checks
use the available server renderer. Windows `START.bat`, the PowerShell restart
drivers and a real GPU remain unverified here.

## AIR-309 update: CPU-time budget, trimmed repeated work

The server's load average is 11-15 permanently, so wall time is not a usable gate. The million-flight test now prints wall time and process CPU time (user+system from `/proc/self/stat`) and **the enforced budget (`AIRLINE_ENFORCE_PERF_BUDGET=1`) is process CPU time: 350 s**. Without `/proc` it falls back to wall time. The budget is re-based from back-to-back runs of main and the branch (both started together at load ~14):

| Revision | Advance wall | Advance process CPU |
| --- | ---: | ---: |
| Main `01c5a97` | 396.7 s | 335.7 s |
| AIR-309 | 376.6 s | 318.4 s |

350 s leaves about 10% headroom over the branch; CPU time still varies a few percent with load (shared caches, SMT), so treat a result within the headroom as a pass and a result above it as a regression to investigate.

Changes (caching/skipping only): the next-event scan computes each company's maintenance and payroll events once instead of again inside the financing event; the market supply is not rebuilt when only reputation moved (scores are refreshed in place); alliance review only invalidates commercial caches when membership or codeshares actually changed. A 200,000-flight run (`AIRLINE_PROFILE_FLIGHTS=200000`) ends in a byte-identical serialized game state on main and branch (`AIRLINE_PROFILE_DUMP`, `cmp`); CPU 64.06 s vs 60.22 s. The 300 s budget in wall terms is **not** met by either revision on this server; no further bit-identical savings were found in this round.
