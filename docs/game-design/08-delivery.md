# Development, scope and quality gates

## Approved
- **GDD-035:** Build a playable airline-management core first; extend and deepen iteratively rather than building every system before playability.
- **GDD-038:** Automated QA: deterministic simulation testing, integration tests, performance benchmarks, consistency checks and build automation.
- **GDD-039:** Automatic download and suitable-restart installation, no online requirement or forced mid-session interruption.

## Proposed milestone sequence (not yet ratified)
1. **Bootstrap:** Headless Rust engine, basic typed Godot bridge, native Windows/macOS build templates, CI test loop.
2. **Vertical slice:** Found airline, acquire aircraft, plan route, simulate bookings and flight operations, book costs/revenue, view results in functional Godot dashboards.
3. **Economic expansion:** Individual preferences, competitor interactions, pricing, cash flow.
4. **Operations:** Fleet condition, rosters, airports, disruptions.
5. **Global ecosystem:** Cargo, airlines, contracts, equipment market, managers and specialists.
6. **Long-run premium product:** BI, modding, detailed 3D, media, long-term scaling.

## Build and review gates
No fabricated outcomes. Tests for changed rules and save compatibility. UI screenshots and keyboard/contrast checks. Performance benchmarks on representative sample worlds. Approved decision changes require owner review. Merge/release not implicit in passing CI.

## Open
Milestone budgets, exit criteria, target minimum hardware, simulation timescale, data coverage and licensing, development ownership and release workflow.
