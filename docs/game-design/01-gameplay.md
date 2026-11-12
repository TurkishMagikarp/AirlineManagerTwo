# Gameplay, time and progression

## Approved
- **GDD-003 — Mode:** Local offline single-player. Simulated time stops while the application is closed. Pause and adjustable speed.
- **GDD-006 — Route planning:** Intelligent automatic schedules with optional full manual control of flight times, aircraft rotations, turnaround and connections. Manually locked choices must not be silently overwritten.
- **GDD-025 — Time controls:** Precise simulation time, pause, speed changes and jumps to events or specified times. Event processing must be correct regardless of presentation speed.
- **GDD-026 — Saving:** Multiple manual saves/worlds, configurable autosave, crash recovery, backup history, long-term company statistics, snapshots and recovery of earlier simulation states.
- **GDD-035 — Development:** Playable core first, incremental expansion toward all approved systems.

## Core loop
Found airline → acquire/lease aircraft → plan route and crew/airport capacity → offer tickets/cargo → advance simulation → inspect actual revenues/costs and operational events → change business strategy/expand.

## Technical invariants
- Speed and time jumps do not change business rules or skip obligations.
- Saved state includes scheduled events and RNG state or equivalent reproducibility mechanism.
- Historical reports are not automatically full restorable snapshots.
- A time-travel restore must preserve a separate branch or otherwise protect the original state.

## Open
Exact game-time ratio and speed steps, schedule granularity, leap bounds, snapshot retention/compression, save format and migrations, and first-slice starting conditions.
