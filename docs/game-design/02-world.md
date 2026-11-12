# World, markets and competition

## Approved
- **GDD-007 — Passengers:** Detailed individual passenger preference/trip choice simulation. Profile factors include budget, destination, schedule, convenience, price, comfort and connection tolerance. Do not assume permanent one-object-per-person storage.
- **GDD-008 — AI airlines:** Economically operating competitors with their own fleets, cash, strategies, investments, routes and possible insolvency; same fundamental economic rules as player.
- **GDD-013 — World economy:** Interacting weather, macroeconomy, tourism, fuel markets and crises. Events propagate through systems and should be understandable.
- **GDD-014 — Brand and loyalty:** Passenger experience and long-term preferences; targeted marketing, frequent flyer programs and multidimensional reputation.
- **GDD-015 — Cooperation:** Codeshare, interline, alliances, shared loyalty schemes and joint ventures with independently deciding AI partners.
- **GDD-017 — Technology:** Dynamic manufacturer research, engines/propulsion, SAF, emissions and changing costs. Technology cannot appear without plausible progress.
- **GDD-040 — Data provenance:** Use actual reference data where licensed/possible and explicitly simulated substitutes otherwise.
- **GDD-047 — Competitor intelligence:** Public information distinguished from estimates; hire analysts whose expertise changes estimate accuracy; confidential competitor facts not directly exposed to player.
- **GDD-048 — Analysts:** Individual employees with specializations, experience and training.
- **GDD-049 — Research information market:** Data vendors, industry publications and research firms with differentiated freshness, accuracy and prices, interpreted by analysts.
- **GDD-060 — Aviation media:** Different simulated publications cover events which actually occurred in the simulated world; separate fact, estimate and forecast.
- **GDD-061 — Calendar:** Internal commitments, known market events, milestones, management tasks, reminders and optional automatic pause.

## Modeling constraints
Global world operates independently of player's visible map. Model-driven competitor intelligence must not leak true hidden values. AI companies and information providers operate under resource constraints. News cannot fabricate a game event.

## Open
Passenger-choice and aggregation methods, regional demand calibration, AI decision cadence, news templates and publication lag, research confidence calibration, global simulation fidelity boundaries.
