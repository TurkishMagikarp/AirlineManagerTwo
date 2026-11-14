//! Lossless run-length archive of settled flights; never recomputes historical quotes.
use crate::{
    economy::{FlightResult, Totals},
    game::Game,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Batch {
    pub route_id: u32,
    pub first: u64,
    pub interval: u64,
    pub count: u64,
    pub origin: String,
    pub destination: String,
    pub ticket_eur: i64,
    pub totals: Totals,
}
impl Batch {
    pub fn at(&self, index: u64) -> FlightResult {
        let (a, b) = if index.is_multiple_of(2) {
            (&self.origin, &self.destination)
        } else {
            (&self.destination, &self.origin)
        };
        FlightResult {
            route_id: self.route_id,
            landing: self.first + index * self.interval,
            origin: a.clone(),
            destination: b.clone(),
            ticket_eur: self.ticket_eur,
            totals: self.totals.clone(),
        }
    }
    pub fn through(&self, time: u64) -> u64 {
        if time < self.first {
            0
        } else {
            ((time - self.first) / self.interval)
                .saturating_add(1)
                .min(self.count)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Purchase {
    pub time: u64,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub since: u64,
    pub opening_balance: i64,
    pub prior_totals: Totals,
    pub batches: Vec<Batch>,
    /// Stable service-to-public-route mapping, retained after route deletion.
    #[serde(default)]
    pub route_groups: std::collections::BTreeMap<u32, u32>,
    pub purchases: Vec<Purchase>,
    #[serde(default)]
    pub aircraft_cash: Vec<crate::trading::Entry>,
    /// Additive (version 10): maintenance checks, debited at their start.
    #[serde(default)]
    pub maintenance: Vec<Purchase>,
    /// Additive (version 11): salaries paid at payroll boundaries and dismissals.
    #[serde(default)]
    pub payroll: Vec<Purchase>,
    #[serde(default)]
    pub financing: Vec<crate::financing::Entry>,
    #[serde(default)]
    pub taxes: Vec<crate::tax::Assessment>,
    #[serde(default)]
    pub marketing_costs: Vec<Purchase>,
    #[serde(default)]
    pub insurance_costs: Vec<crate::insurance::Expense>,
    #[serde(skip)]
    settled: SettledCache,
}

// Derived service totals accelerate AI lifetime reviews. Cache population must
// not affect game equality, serialization or save/load determinism.
#[derive(Clone, Debug, Default)]
struct SettledCache(std::cell::RefCell<Option<SettledTotals>>);
impl PartialEq for SettledCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
#[derive(Clone, Debug, Default)]
struct SettledTotals {
    through: u64,
    services: std::collections::BTreeMap<u32, Totals>,
}
impl SettledTotals {
    fn record(&mut self, batch: &Batch) {
        self.through = self
            .through
            .max(batch.first + (batch.count - 1) * batch.interval);
        self.services
            .entry(batch.route_id)
            .or_default()
            .add(&batch.totals, batch.count)
            .expect("validated archive totals");
    }
}
impl History {
    pub fn new(since: u64, opening_balance: i64) -> Self {
        Self {
            since,
            opening_balance,
            prior_totals: Totals::default(),
            batches: Vec::new(),
            route_groups: Default::default(),
            purchases: Vec::new(),
            aircraft_cash: Vec::new(),
            maintenance: Vec::new(),
            payroll: Vec::new(),
            financing: Vec::new(),
            taxes: Vec::new(),
            marketing_costs: Vec::new(),
            insurance_costs: Vec::new(),
            settled: SettledCache::default(),
        }
    }
    pub fn record(&mut self, batch: Batch) {
        if let Some(totals) = self.settled.0.get_mut() {
            totals.record(&batch);
        }
        if let Some(last) = self
            .batches
            .iter_mut()
            .rev()
            .find(|b| b.route_id == batch.route_id)
        {
            let next = last
                .first
                .checked_add(last.count.saturating_mul(last.interval));
            let same_direction = if last.count % 2 == 0 {
                last.origin == batch.origin && last.destination == batch.destination
            } else {
                last.destination == batch.origin && last.origin == batch.destination
            };
            if next == Some(batch.first)
                && last.interval == batch.interval
                && same_direction
                && last.ticket_eur == batch.ticket_eur
                && last.totals == batch.totals
            {
                last.count += batch.count;
                return;
            }
        }
        // Most settlements append chronologically. Insert only the new batch
        // at its canonical position; sorting the entire lifetime archive here
        // made long advances quadratic as fuel/season changes added blocks.
        let key = (batch.first, batch.route_id);
        let index = self
            .batches
            .partition_point(|b| (b.first, b.route_id) <= key);
        self.batches.insert(index, batch);
    }
    pub fn public_route(&self, service: u32) -> u32 {
        self.route_groups.get(&service).copied().unwrap_or(service)
    }

    fn with_settled<T>(&self, f: impl FnOnce(&SettledTotals) -> T) -> T {
        let mut cached = self.settled.0.borrow_mut();
        let totals = cached.get_or_insert_with(|| {
            let mut totals = SettledTotals::default();
            for batch in &self.batches {
                totals.record(batch);
            }
            totals
        });
        f(totals)
    }

    pub(crate) fn settled_service_totals(&self, service: u32) -> Totals {
        self.with_settled(|t| t.services.get(&service).cloned().unwrap_or_default())
    }

    pub fn totals(&self, route: u32, through: u64) -> Totals {
        if let Some(totals) = self.with_settled(|totals| {
            (through >= totals.through).then(|| {
                let mut sum = Totals::default();
                for (service, total) in &totals.services {
                    if route == 0 || self.public_route(*service) == route {
                        sum.add(total, 1).expect("validated archive totals");
                    }
                }
                sum
            })
        }) {
            return totals;
        }
        let mut t = Totals::default();
        for b in self
            .batches
            .iter()
            .filter(|b| route == 0 || self.public_route(b.route_id) == route)
        {
            t.add(&b.totals, b.through(through))
                .expect("validated archive totals");
        }
        t
    }
    pub fn balance(&self, time: u64) -> i64 {
        self.balance_wide(time) as i64
    }
    fn balance_wide(&self, time: u64) -> i128 {
        let profit = self.totals(0, time).profit();
        let purchases: i128 = self
            .purchases
            .iter()
            .filter(|p| p.time <= time)
            .map(|p| i128::from(p.amount))
            .sum();
        i128::from(self.opening_balance)
            + i128::from(profit)
            + self
                .financing
                .iter()
                .filter(|e| e.time <= time)
                .map(|e| i128::from(e.cash))
                .sum::<i128>()
            + self.aircraft_cash_through(time)
            - purchases
            - self.maintenance_through(time)
            - self.payroll_through(time)
            - self.tax_through(time)
            - self.marketing_through(time)
            - self.insurance_through(time)
    }
    pub fn aircraft_cash_through(&self, time: u64) -> i128 {
        self.aircraft_cash
            .iter()
            .filter(|e| e.time <= time)
            .map(|e| i128::from(e.cash))
            .sum()
    }
    pub fn rent_through(&self, time: u64) -> i128 {
        self.aircraft_cash
            .iter()
            .filter(|e| e.time <= time && e.kind == crate::trading::Kind::Rent)
            .map(|e| -i128::from(e.cash))
            .sum()
    }
    /// Salaries paid up to and including `time`.
    pub fn payroll_through(&self, time: u64) -> i128 {
        self.payroll
            .iter()
            .filter(|p| p.time <= time)
            .map(|p| i128::from(p.amount))
            .sum()
    }
    pub fn interest_through(&self, time: u64) -> i128 {
        self.financing
            .iter()
            .filter(|e| e.time <= time)
            .map(|e| i128::from(e.interest))
            .sum()
    }
    /// Maintenance spending up to and including `time`.
    pub fn maintenance_through(&self, time: u64) -> i128 {
        self.maintenance
            .iter()
            .filter(|p| p.time <= time)
            .map(|p| i128::from(p.amount))
            .sum()
    }
    /// Batches of a route never overlap. Work depends on batches and page size, not flights.
    pub fn page(&self, route: u32, offset: u64, limit: usize) -> Vec<FlightResult> {
        let batches: Vec<_> = self
            .batches
            .iter()
            .filter(|b| self.public_route(b.route_id) == route)
            .collect();
        // Merge interleaved aircraft histories by landing time, without expanding
        // the lifetime archive into one object per flight.
        let mut heap = std::collections::BinaryHeap::new();
        for (batch, b) in batches.iter().enumerate() {
            if b.count > 0 {
                let index = if batches.len() == 1 {
                    offset.checked_add(1).and_then(|n| b.count.checked_sub(n))
                } else {
                    Some(b.count - 1)
                };
                if let Some(i) = index {
                    heap.push((b.at(i).landing, b.route_id, batch, i));
                }
            }
        }
        let mut skip = if batches.len() == 1 { 0 } else { offset };
        let mut rows = Vec::new();
        while rows.len() < limit {
            let Some((_, _, batch, index)) = heap.pop() else {
                break;
            };
            let b = batches[batch];
            if skip > 0 {
                skip -= 1;
            } else {
                rows.push(b.at(index));
            }
            if index > 0 {
                heap.push((b.at(index - 1).landing, b.route_id, batch, index - 1));
            }
        }
        rows
    }
}

impl Game {
    pub fn validate_history(&self) -> Result<(), String> {
        self.validate_marketing()?;
        self.reputation.validate()?;
        let Some(h) = &self.history else {
            return if self.airline.is_none() {
                Ok(())
            } else {
                Err("Finanzhistorie fehlt.".into())
            };
        };
        let now = self.clock.elapsed().as_secs();
        if self.airline.is_none() || h.since > now {
            return Err("Ungültiger Finanzbeginn.".into());
        }
        if h.route_groups.iter().any(|(service, route)| {
            *route == 0
                || *service <= *route
                || *service >= self.next_route_id
                || h.route_groups.contains_key(route)
        }) || self
            .routes
            .iter()
            .any(|r| h.public_route(r.id) != r.public_id())
        {
            return Err("Ungültige Archiv-Routengruppe.".into());
        }
        let mut sum = Totals::default();
        let mut ends = std::collections::HashMap::new();
        for b in &h.batches {
            let end = b
                .count
                .checked_sub(1)
                .and_then(|n| n.checked_mul(b.interval))
                .and_then(|n| b.first.checked_add(n));
            let t = &b.totals;
            if b.route_id == 0
                || b.route_id >= self.next_route_id
                || b.interval == 0
                || b.first < h.since
                || end.is_none_or(|e| e > now)
                || ends.get(&b.route_id).is_some_and(|e| *e >= b.first)
                || t.flights != 1
                || t.passengers < 0
                || t.seats < t.passengers
                || !(0..=100_000).contains(&b.ticket_eur)
                || !t.valid_cabins(b.ticket_eur)
                || [t.fuel, t.airport, t.crew, t.overhead]
                    .iter()
                    .any(|v| *v < 0)
                || crate::routes::distance_km(&b.origin, &b.destination).is_err()
                || sum.add(t, b.count).is_none()
            {
                return Err("Ungültiges Flugarchiv.".into());
            }
            ends.insert(b.route_id, end.unwrap());
        }
        let mut expected = h.prior_totals.clone();
        if expected.add(&sum, 1).is_none() || expected != self.finance.totals {
            return Err("Flugarchiv und Finanzsummen widersprechen sich.".into());
        }
        if h.purchases
            .iter()
            .any(|p| p.time < h.since || p.time > now || p.amount <= 0)
            || h.purchases.windows(2).any(|p| p[0].time > p[1].time)
            || h.maintenance
                .iter()
                .any(|p| p.time < h.since || p.time > now || p.amount < 0)
            || h.maintenance.windows(2).any(|p| p[0].time > p[1].time)
            || sum.flights > self.finance.totals.flights
            || sum.revenue > self.finance.totals.revenue
            || sum.passengers > self.finance.totals.passengers
            || sum.seats > self.finance.totals.seats
            || sum.fuel > self.finance.totals.fuel
            || sum.airport > self.finance.totals.airport
            || sum.crew > self.finance.totals.crew
            || sum.overhead > self.finance.totals.overhead
        {
            return Err("Ungültige Finanzhistorie.".into());
        }
        if h.balance_wide(now) != i128::from(self.airline.as_ref().unwrap().capital_eur) {
            return Err("Kontostand und Finanzhistorie widersprechen sich.".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::default();
        g.found("Archive Air", "DE", "EDDF").unwrap();
        g.competitors.clear();
        g.buy_aircraft("atr72_600").unwrap();
        g.staff_up();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, 71).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g.clock.set_running(true);
        g
    }
    #[test]
    fn interleaved_route_batches_have_canonical_order_and_still_merge() {
        let batch = |route_id, first, count| Batch {
            route_id,
            first,
            count,
            interval: 60,
            origin: "EDDF".into(),
            destination: "EDDM".into(),
            ticket_eur: 0,
            totals: Totals::default(),
        };
        let mut large = History::new(0, 0);
        large.record(batch(1, 120, 2));
        large.record(batch(2, 100, 2));
        let mut small = History::new(0, 0);
        small.record(batch(2, 100, 1));
        small.record(batch(1, 120, 1));
        for (route, first) in [(1, 180), (2, 160)] {
            let mut next = batch(route, first, 1);
            std::mem::swap(&mut next.origin, &mut next.destination);
            small.record(next);
        }
        assert_eq!(large, small);
        assert_eq!(large.batches.len(), 2);
    }

    #[test]
    fn settled_cache_matches_scans_after_merges_groups_and_reload() {
        let mut h = History::new(0, 0);
        h.route_groups.insert(2, 1);
        for (route_id, first, count) in [(1, 100, 2), (2, 110, 3), (1, 220, 2), (3, 90, 8)] {
            h.record(Batch {
                route_id,
                first,
                count,
                interval: 60,
                origin: "EDDF".into(),
                destination: "EDDM".into(),
                ticket_eur: 71,
                totals: Totals {
                    flights: 1,
                    seats: 72,
                    passengers: 50,
                    revenue: 3550,
                    fuel: 100,
                    ..Default::default()
                },
            });
            for route in 0..=3 {
                for through in [0, 109, 200, u64::MAX] {
                    let mut expected = Totals::default();
                    for b in &h.batches {
                        if route == 0 || h.public_route(b.route_id) == route {
                            expected.add(&b.totals, b.through(through)).unwrap();
                        }
                    }
                    assert_eq!(h.totals(route, through), expected);
                }
            }
            let cold: History = serde_json::from_value(serde_json::to_value(&h).unwrap()).unwrap();
            assert!(cold.settled.0.borrow().is_none());
            assert_eq!(cold, h);
            for service in 1..=3 {
                let mut expected = Totals::default();
                for b in h.batches.iter().filter(|b| b.route_id == service) {
                    expected.add(&b.totals, b.count).unwrap();
                }
                assert_eq!(h.settled_service_totals(service), expected);
                assert_eq!(cold.settled_service_totals(service), expected);
            }
        }
    }

    #[test]
    fn million_flights_are_lossless_compact_and_random_access() {
        let mut g = game();
        g.advance(60.0 + 7200.0 * 1_000_000.0);
        let h = g.history.as_ref().unwrap();
        let n = g.finance.totals.flights as u64;
        // Blocks split at checks, fuel changes and disruptions; regular
        // on-time stretches still compress instead of storing every flight.
        let checks = g.fleet[0].wear.checks + u64::from(g.fleet[0].maintenance.is_some());
        assert!(n > 900_000 && checks > 1000);
        let weeks = g.clock.elapsed().as_secs() / (7 * 86400);
        assert!(h.batches.len() as u64 > checks);
        let delayed: u64 = h
            .batches
            .iter()
            .filter(|b| b.totals.delay_seconds > 0)
            .map(|b| b.count)
            .sum();
        assert!(h.batches.len() as u64 <= checks + weeks + delayed * 2 + 1);
        assert!(h.batches.len() as u64 * 5 < n);
        assert_eq!(h.maintenance.len() as u64, checks);
        assert_eq!(h.totals(1, g.clock.elapsed().as_secs()), g.finance.totals);
        assert_eq!(
            h.balance(g.clock.elapsed().as_secs()),
            g.airline.as_ref().unwrap().capital_eur
        );
        let page = h.page(1, n - 10, 50);
        assert_eq!(page.len(), 10);
        assert_eq!(page[9].origin, "EDDF");
        assert_eq!(page[8].origin, "EDDM");
        assert!(h.page(1, n, 50).is_empty());
        assert_eq!(h.page(99, 0, 50).len(), 0);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("million.json");
        crate::save::save(&path, &g).unwrap();
        let bytes = std::fs::metadata(&path).unwrap().len();
        eprintln!(
            "Million-flight compact archive: {} batches, {bytes} bytes",
            h.batches.len()
        );
        // Delay details introduce more event boundaries than fuel/checks alone.
        assert!(bytes < 32 * 1024 * 1024);
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.history, g.history);
        g.advance(7200.0);
        loaded.advance(7200.0);
        assert_eq!(loaded.history, g.history);
    }
    #[test]
    fn million_flight_finance_scenario_stays_within_budget() {
        // Match finances_test.gd: two player fares, normal AI airlines, and an
        // endowment to keep the intentionally loss-making service flying.
        let mut g = game();
        g.competitors = crate::competition::initial_competitors().unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.create_route("EDDF", "EDDM", 2).unwrap();
        g.set_ticket_price(2, 134).unwrap();
        g.staff_up();
        g.schedule(2, 1, 120).unwrap();
        g.airline.as_mut().unwrap().capital_eur += 100_000_000_000;
        g.history.as_mut().unwrap().opening_balance += 100_000_000_000;
        let started = std::time::Instant::now();
        assert!(g.advance(60.0 + 7200.0 * 1_000_000.0));
        let elapsed = started.elapsed();
        eprintln!("Finance stress advance: {elapsed:?}");
        assert!(
            elapsed < std::time::Duration::from_secs(300),
            "finance stress advance took {elapsed:?}"
        );
        let h = g.history.as_ref().unwrap();
        assert!(h.totals(1, u64::MAX).flights > 900_000);
        assert!(h.totals(2, u64::MAX).flights > 900_000);
        assert!(h.totals(1, u64::MAX).profit() > 0);
        assert!(h.totals(2, u64::MAX).profit() < 0);
        // Additional disruption boundaries still compress at least 90% of
        // settled player flights in the mixed two-route scenario.
        assert!((h.batches.len() as i64) * 10 < g.finance.totals.flights);
        assert_eq!(g.competitors.len(), 2);
        assert!(
            g.competitors
                .iter()
                .all(|c| c.game.finance.totals.flights > 0)
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("finance-stress.json");
        crate::save::save(&path, &g).unwrap();
        eprintln!(
            "Finance stress save: {} bytes",
            std::fs::metadata(&path).unwrap().len()
        );
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&g).unwrap()
        );
        assert!(g.advance(7200.0));
        assert!(loaded.advance(7200.0));
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&g).unwrap()
        );
        eprintln!(
            "Finance stress including save/reload/continuation: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn frame_partitioning_and_simultaneous_routes_match_archive() {
        let mut a = game();
        let mut b = game();
        for g in [&mut a, &mut b] {
            g.buy_aircraft("atr72_600").unwrap();
            g.staff_up();
            g.create_route("EDDF", "EDDM", 2).unwrap();
            g.set_ticket_price(2, 134).unwrap();
            g.schedule(2, 1, 120).unwrap();
            g.clock.set_speed(16);
        }
        a.advance(100_000.0);
        for _ in 0..10_000 {
            b.advance(10.0);
        }
        assert_eq!(a.history, b.history);
        let h = a.history.as_ref().unwrap();
        assert!(h.totals(1, u64::MAX).profit() > 0);
        assert!(h.totals(2, u64::MAX).profit() < 0);
        assert_eq!(
            h.balance(a.clock.elapsed().as_secs()),
            a.airline.unwrap().capital_eur
        );
    }
    #[test]
    fn price_changes_preserve_old_load_costs_and_deleted_routes() {
        let mut g = game();
        g.advance(12_000.0);
        g.stop_schedule(1).unwrap();
        let old = g.history.as_ref().unwrap().page(1, 0, 50);
        g.set_ticket_price(1, 134).unwrap();
        g.schedule(1, 1, 120).unwrap();
        g.advance(12_000.0);
        g.stop_schedule(1).unwrap();
        let h = g.history.as_ref().unwrap();
        assert_eq!(h.batches.len(), 2);
        let page = h.page(1, 0, 50);
        assert_eq!(&page[2..], old.as_slice());
        assert!(page[0].totals.profit() < 0);
        assert!(page[2].totals.load_percent() > page[0].totals.load_percent());
        let before = g.history.clone();
        g.delete_route(1).unwrap();
        assert_eq!(g.history, before);
        g.validate_history().unwrap();
    }
    #[test]
    fn legacy_migration_starts_at_known_balance_without_fabricating_flights() {
        let mut g = game();
        g.advance(100_000.0);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.json");
        crate::save::save(&path, &g).unwrap();
        let mut json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["version"] = 6.into();
        json["state"].as_object_mut().unwrap().remove("history");
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert!(loaded.history.as_ref().unwrap().batches.is_empty());
        assert_eq!(loaded.finance, g.finance);
        assert_eq!(loaded.history.as_ref().unwrap().since, 100_000);
        loaded.advance(7200.0);
        assert_eq!(
            loaded.history.as_ref().unwrap().totals(1, u64::MAX).flights,
            1
        );
        assert_eq!(
            loaded.history.as_ref().unwrap().balance(107_200),
            loaded.airline.unwrap().capital_eur
        );
    }
    #[test]
    fn corrupt_archive_is_rejected_and_overflow_leaves_it_unchanged() {
        let mut g = game();
        let before = g.history.clone();
        g.airline.as_mut().unwrap().capital_eur = i64::MAX;
        assert!(!g.advance(12_000.0));
        assert_eq!(g.history, before);
        g.airline.as_mut().unwrap().capital_eur = 7_000_000;
        g.advance(12_000.0);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (key, value) in [
            ("count", 0),
            ("count", u64::MAX),
            ("interval", 0),
            ("route_id", 99),
            ("first", u64::MAX),
        ] {
            let mut bad = valid.clone();
            bad["state"]["history"]["batches"][0][key] = value.into();
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err(), "accepted {key}");
        }
        for key in ["opening_balance", "since"] {
            let mut bad = valid.clone();
            bad["state"]["history"][key] = i64::MAX.into();
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err(), "accepted {key}");
        }
    }
}
