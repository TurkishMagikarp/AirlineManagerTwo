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
        }
    }
    pub fn record(&mut self, batch: Batch) {
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
        self.batches.push(batch);
        // Different frame sizes can discover simultaneous route batches in a
        // different order. Store them by first landing for identical saves.
        self.batches.sort_by_key(|b| (b.first, b.route_id));
    }
    pub fn public_route(&self, service: u32) -> u32 {
        self.route_groups.get(&service).copied().unwrap_or(service)
    }

    pub fn totals(&self, route: u32, through: u64) -> Totals {
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
                || t.passengers.checked_mul(b.ticket_eur) != Some(t.revenue)
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
    fn million_flights_are_lossless_compact_and_random_access() {
        let mut g = game();
        g.advance(60.0 + 7200.0 * 1_000_000.0);
        let h = g.history.as_ref().unwrap();
        let n = g.finance.totals.flights as u64;
        // Uninterrupted service merges into one block; each mandatory check
        // pauses the shuttle and starts exactly one new block.
        let checks = g.fleet[0].wear.checks + u64::from(g.fleet[0].maintenance.is_some());
        assert!(n > 900_000 && checks > 1000);
        assert_eq!(
            h.batches.len() as u64,
            checks + u64::from(g.routes[0].plan.is_some())
        );
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
        assert!(std::fs::metadata(&path).unwrap().len() < 1_000_000);
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.history, g.history);
        g.advance(7200.0);
        loaded.advance(7200.0);
        assert_eq!(loaded.history, g.history);
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
