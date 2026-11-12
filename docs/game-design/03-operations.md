# Fleet, airports, operations and cargo

## Approved
- **GDD-009 — Fleet:** Deep individual aircraft simulation: engines and variants, wear on components, life, hours/cycles, maintenance schedules, spare parts, operational and financial differences.
- **GDD-010 — Airports:** Deep slots, terminals, gates, ground handling, maintenance bases, rented infrastructure, connections and hub waves; airline management, not airport ownership sim.
- **GDD-016 — Aircraft market:** Economically operating manufacturers and lessors, production capacity, orders, delivery delays, model development, retirements and used market.
- **GDD-018 — Cargo:** Full air cargo/logistics: cargo orders, commodity types, deadlines, dedicated freighters, belly capacity, hubs, terminals, logistics contracts and cargo subsidiaries.
- **GDD-030 — Visualization:** Interactive strategic flight map and optional detailed 3D aircraft, airport and flight views. View state does not drive simulation state.
- **GDD-032 — AOCC:** Operations Control Center monitors flights, crews, airports and passengers; realistic disruption propagation and automated or manual recovery.
- **GDD-033 — Revenue:** Booking classes, individual fares/rules, sales channels, interline journeys, dynamic pricing, overbooking, cancellations, refunds and ancillary services; can be delegated.
- **GDD-042 — Cabin:** Detailed seat configurations, comfort, catering offers, IFE, WLAN, baggage and fare-related products. **Not** the unchosen full catering supply chain simulation.
- **GDD-053 — Training:** In-house and external pilot, simulator, cabin and technician training.
- **GDD-054 — Insurance:** Detailed coverage types, limits, deductibles, claims history and dynamic premiums; **not** a full standalone global insurer simulation.
- **GDD-055 — Fuel:** Regional prices, supply contracts, bulk discounts, hedging and SAF; **not** the unchosen full fuel-storage logistics simulation.

## Invariants
Aircraft/crew/airport physical limits and safety rules apply to every airline. Passenger and freight loads jointly respect capacity/weight limits. Assets leased out under ACMI cannot be simultaneously booked for owned operations. Disruption impacts must propagate to downstream aircraft and passengers.

## Open
Exact wear formulas, aircraft/engine dataset and performance assumptions, aviation regulations and slot allocation algorithms, ticket revenue recognition, maintenance scheduling, cargo routing, insurance contract math, hedging instrument mechanics.
