//! Addressable landing RNG; derived next-event cache is disposable, never saved.
use crate::{
    aircraft::AircraftStatus, game::Game, history::History, maintenance::Visit, operations,
};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, sync::LazyLock};
const DAY: u64 = 86400;
const YEAR: u64 = 365 * DAY;
#[derive(Deserialize)]
pub struct Rate {
    pub annual_bps: i64,
    pub deductible_bps: i64,
    pub limit_bps: i64,
}
#[derive(Deserialize)]
pub struct Rules {
    pub incident_per_million_landings: u64,
    pub repair_min_bps: i64,
    pub repair_max_bps: i64,
    pub downtime_hours: u64,
    pub basic: Rate,
    pub full: Rate,
    pub claim_loading_bps: i64,
    pub clean_year_decay_bps: i64,
    pub max_loading_bps: i64,
    pub history_limit: usize,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/insurance.json"))
        .expect("insurance config");
    assert!((1..=100_000).contains(&r.incident_per_million_landings));
    assert!((1..=r.repair_max_bps).contains(&r.repair_min_bps) && r.repair_max_bps <= 10000);
    assert!((1..=720).contains(&r.downtime_hours) && (1..=100).contains(&r.history_limit));
    assert!(
        (0..=10000).contains(&r.max_loading_bps)
            && (1..=10000).contains(&r.claim_loading_bps)
            && (1..=10000).contains(&r.clean_year_decay_bps)
    );
    for rate in [&r.basic, &r.full] {
        assert!(
            (1..=10000).contains(&rate.annual_bps)
                && (0..=10000).contains(&rate.deductible_bps)
                && (1..=10000).contains(&rate.limit_bps)
        );
    }
    r
});
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    #[default]
    None,
    Basic,
    Full,
}
impl Coverage {
    pub fn rate(self) -> Option<&'static Rate> {
        match self {
            Self::None => None,
            Self::Basic => Some(&RULES.basic),
            Self::Full => Some(&RULES.full),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Basic => "basic",
            Self::Full => "full",
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    pub coverage: Coverage,
    pub settled_at: u64,
    pub renewed_at: u64,
    pub annual_premium: i64,
    pub remainder: u64,
    pub loading_bps: i64,
    pub year_claims: u32,
    pub recent: Vec<Claim>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub time: u64,
    pub aircraft: u32,
    pub cycle: u64,
    pub kind: u64,
    pub repair: i64,
    pub insurer: i64,
    pub paid: i64,
    pub coverage: Coverage,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Expense {
    pub time: u64,
    pub premium: i64,
    pub deductible: i64,
    pub uninsured: i64,
}
impl Expense {
    pub fn total(&self) -> i128 {
        i128::from(self.premium) + i128::from(self.deductible) + i128::from(self.uninsured)
    }
}
impl History {
    pub fn insurance_through(&self, time: u64) -> i128 {
        self.insurance_costs
            .iter()
            .filter(|e| e.time <= time)
            .map(Expense::total)
            .sum()
    }
}
fn hash(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
pub(crate) fn roll(seed: u64, cycle: u64) -> u64 {
    hash(seed.wrapping_add(cycle.wrapping_mul(0x9e3779b97f4a7c15)))
}
pub(crate) fn incident(seed: u64, cycle: u64) -> bool {
    roll(seed, cycle) % 1_000_000 < RULES.incident_per_million_landings
}
thread_local! { static NEXT: RefCell<HashMap<u64, (u64,u64)>> = RefCell::new(HashMap::new()); }
fn next_cycle(seed: u64, after: u64) -> Option<u64> {
    NEXT.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(&(start, next)) = cache.get(&seed)
            && after >= start
            && after < next
        {
            return Some(next);
        }
        let mut next = after.checked_add(1)?;
        while !incident(seed, next) {
            next = next.checked_add(1)?;
        }
        if cache.len() >= 128 {
            cache.clear();
        }
        cache.insert(seed, (after, next));
        Some(next)
    })
}
impl Game {
    /// Earlier feature migration tests must include the new neutral migration.
    #[cfg(test)]
    pub fn migrate_insurance_fixture(&mut self) {
        let now = if self.airline.is_some() {
            self.clock.elapsed().as_secs()
        } else {
            0
        };
        self.insurance = Policy {
            settled_at: now,
            renewed_at: now,
            ..Policy::default()
        };
        for c in &mut self.competitors {
            c.game.migrate_insurance_fixture();
        }
    }

    pub(crate) fn incident_seed(&self, id: u32) -> u64 {
        let owner = self.airline.as_ref().map_or(0, |a| {
            a.name
                .bytes()
                .fold(0u64, |s, b| s.wrapping_mul(31).wrapping_add(u64::from(b)))
        });
        hash(self.fuel_market.seed ^ hash(owner) ^ hash(u64::from(id)))
    }
    pub fn insurance_quote(&self, coverage: Coverage) -> Result<i64, String> {
        let Some(rate) = coverage.rate() else {
            return Ok(0);
        };
        let value = self
            .fleet
            .iter()
            .try_fold(0i128, |s, p| self.hull_value(p).map(|v| s + i128::from(v)))?;
        i64::try_from(
            value * i128::from(rate.annual_bps) * i128::from(10000 + self.insurance.loading_bps)
                / 100_000_000,
        )
        .map_err(|_| "Versicherungsüberlauf.".into())
    }
    pub fn set_coverage(&mut self, coverage: Coverage) -> Result<(), String> {
        self.require_active()?;
        if self.airline.is_none() {
            return Err("Airline fehlt.".into());
        }
        self.settle_insurance()?;
        self.renew_insurance();
        self.insurance.annual_premium = self.insurance_quote(coverage)?;
        self.insurance.coverage = coverage;
        self.observe_distress();
        Ok(())
    }
    fn post_insurance(&mut self, expense: Expense) -> Result<(), String> {
        let balance = i64::try_from(i128::from(self.cash()) - expense.total())
            .map_err(|_| "Versicherungsüberlauf.")?;
        self.airline.as_mut().ok_or("Airline fehlt.")?.capital_eur = balance;
        if expense.total() > 0 {
            self.history
                .as_mut()
                .ok_or("Historie fehlt.")?
                .insurance_costs
                .push(expense);
        }
        Ok(())
    }
    fn settle_insurance(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let p = &self.insurance;
        let seconds = now
            .checked_sub(p.settled_at)
            .ok_or("Versicherungszeit ungültig.")?;
        let n = u128::from(p.remainder) + p.annual_premium as u128 * u128::from(seconds);
        let premium = i64::try_from(n / u128::from(YEAR)).map_err(|_| "Versicherungsüberlauf.")?;
        self.post_insurance(Expense {
            time: now,
            premium,
            deductible: 0,
            uninsured: 0,
        })?;
        self.insurance.remainder = (n % u128::from(YEAR)) as u64;
        self.insurance.settled_at = now;
        Ok(())
    }
    pub fn next_insurance_event(&self) -> Option<u64> {
        if self.airline.is_none() || self.insolvent() {
            return None;
        }
        let p = &self.insurance;
        let premium = (p.coverage != Coverage::None || p.loading_bps > 0 || p.year_claims > 0)
            .then(|| ((p.settled_at / DAY + 1) * DAY).min(p.renewed_at.saturating_add(YEAR)));
        self.routes
            .iter()
            .filter_map(|r| {
                let plan = r.plan.as_ref()?;
                let plane = self.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
                let cycles = next_cycle(self.incident_seed(plane.id), plane.wear.cycles)?
                    .checked_sub(plane.wear.cycles)?;
                let index = plan.completed_flights.checked_add(cycles)?.checked_sub(1)?;
                let flight = operations::timings(r, &plane.model_id).ok()?.0;
                if cycles
                    > plane
                        .wear
                        .flights_until_due(crate::maintenance::rules(&plane.model_id).ok()?, flight)
                {
                    return None;
                }
                crate::disruptions::landing_at(r, &plane.model_id, index).ok()
            })
            .chain(premium)
            .min()
    }
    pub fn apply_insurance(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        if self.airline.is_none() {
            return Ok(());
        }
        for i in 0..self.fleet.len() {
            let plane = &self.fleet[i];
            let Some(route) = self
                .routes
                .iter()
                .find(|r| Some(plane.id) == r.aircraft_id && r.plan.is_some())
            else {
                continue;
            };
            let plan = route.plan.as_ref().unwrap();
            if plan.completed_flights == 0 || plane.status == AircraftStatus::InFlight {
                continue;
            }
            let landing =
                crate::disruptions::landing_at(route, &plane.model_id, plan.completed_flights - 1)?;
            if landing != now || !incident(self.incident_seed(plane.id), plane.wear.cycles) {
                continue;
            }
            let value = self.hull_value(plane)?;
            let random = roll(
                self.incident_seed(plane.id) ^ 0x524550414952,
                plane.wear.cycles,
            );
            let bps = RULES.repair_min_bps
                + (random % (RULES.repair_max_bps - RULES.repair_min_bps + 1) as u64) as i64;
            let repair = (i128::from(value) * i128::from(bps) / 10000).max(1) as i64;
            let coverage = self.insurance.coverage;
            let insurer = coverage.rate().map_or(0, |r| {
                let deductible = (i128::from(value) * i128::from(r.deductible_bps) / 10000) as i64;
                (repair - deductible)
                    .max(0)
                    .min((i128::from(value) * i128::from(r.limit_bps) / 10000) as i64)
            });
            let paid = repair - insurer;
            let deductible = coverage.rate().map_or(0, |r| {
                ((i128::from(value) * i128::from(r.deductible_bps) / 10000) as i64).min(repair)
            });
            let claim = Claim {
                time: now,
                aircraft: plane.id,
                cycle: plane.wear.cycles,
                kind: random % 3,
                repair,
                insurer,
                paid,
                coverage,
            };
            self.post_insurance(Expense {
                time: now,
                premium: 0,
                deductible,
                uninsured: paid - deductible,
            })?;
            if insurer > 0 {
                self.insurance.year_claims = self.insurance.year_claims.saturating_add(1);
            }
            self.insurance.recent.push(claim);
            if self.insurance.recent.len() > RULES.history_limit {
                self.insurance.recent.remove(0);
            }
            let end = now
                .checked_add(RULES.downtime_hours * 3600)
                .ok_or("Reparaturzeitüberlauf.")?;
            if let Some(v) = &mut self.fleet[i].maintenance {
                v.end = v.end.max(end);
            } else {
                // A regular check due at this landing is still paid; repairs do not waive it.
                let plane = &self.fleet[i];
                let r = crate::maintenance::rules(&plane.model_id)?;
                let due = plane.wear.is_due(r);
                if due {
                    self.apply_maintenance()?;
                    let v = self.fleet[i]
                        .maintenance
                        .as_mut()
                        .ok_or("Pflichtwartung fehlt.")?;
                    v.end = v.end.max(end);
                    continue;
                }
                self.fleet[i].maintenance = Some(Visit {
                    start: now,
                    end,
                    cost_eur: 0,
                    unscheduled: true,
                    resume: None,
                    staffed: true,
                });
            }
            self.begin_visit(i)?;
        }
        if (self.insurance.coverage != Coverage::None
            || self.insurance.loading_bps > 0
            || self.insurance.year_claims > 0)
            && now > self.insurance.settled_at
            && (now.is_multiple_of(DAY) || now >= self.insurance.renewed_at.saturating_add(YEAR))
        {
            self.settle_insurance()?;
            self.renew_insurance();
            self.insurance.annual_premium = self.insurance_quote(self.insurance.coverage)?;
        }
        self.observe_distress();
        Ok(())
    }
    fn renew_insurance(&mut self) {
        let now = self.clock.elapsed().as_secs();
        let p = &mut self.insurance;
        let years = now.saturating_sub(p.renewed_at) / YEAR;
        if years == 0 {
            return;
        }
        let clean = if p.year_claims > 0 {
            p.loading_bps = (p.loading_bps + i64::from(p.year_claims) * RULES.claim_loading_bps)
                .min(RULES.max_loading_bps);
            years - 1
        } else {
            years
        };
        p.loading_bps = (i128::from(p.loading_bps)
            - i128::from(clean) * i128::from(RULES.clean_year_decay_bps))
        .max(0) as i64;
        p.year_claims = 0;
        p.renewed_at += years * YEAR;
    }
    pub fn validate_insurance(&self) -> Result<(), String> {
        let p = &self.insurance;
        let now = self.clock.elapsed().as_secs();
        if (self.airline.is_none() && p != &Policy::default())
            || (!self.insolvent()
                && (p.coverage != Coverage::None || p.loading_bps > 0 || p.year_claims > 0)
                && (p.settled_at / DAY < now / DAY || now.saturating_sub(p.renewed_at) >= YEAR))
            || p.settled_at > now
            || p.renewed_at > now
            || p.annual_premium < 0
            || p.remainder >= YEAR
            || !(0..=RULES.max_loading_bps).contains(&p.loading_bps)
            || p.recent.len() > RULES.history_limit
            || (p.coverage == Coverage::None && p.annual_premium != 0)
            || p.recent.iter().any(|c| {
                c.time > now
                    || c.aircraft == 0
                    || c.aircraft >= self.next_aircraft_id
                    || c.kind >= 3
                    || c.repair <= 0
                    || c.insurer < 0
                    || c.paid < 0
                    || c.insurer.checked_add(c.paid) != Some(c.repair)
            })
            || p.recent.windows(2).any(|c| c[0].time > c[1].time)
            || self.history.as_ref().is_some_and(|h| {
                h.insurance_costs.iter().any(|e| {
                    e.time < h.since
                        || e.time > now
                        || e.premium < 0
                        || e.deductible < 0
                        || e.uninsured < 0
                        || e.total() <= 0
                }) || h.insurance_costs.windows(2).any(|e| e[0].time > e[1].time)
            })
        {
            return Err("Ungültige Versicherungsdaten.".into());
        }
        Ok(())
    }
}
pub fn migrate(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(o) => {
            if o.contains_key("clock") {
                let now = if o.get("airline").is_none_or(|a| a.is_null()) {
                    0
                } else {
                    o["clock"]["elapsed_seconds"]
                        .as_u64()
                        .unwrap_or_else(|| o["clock"]["elapsed"]["secs"].as_u64().unwrap_or(0))
                };
                o.insert(
                    "insurance".into(),
                    serde_json::to_value(Policy {
                        settled_at: now,
                        renewed_at: now,
                        ..Policy::default()
                    })
                    .unwrap(),
                );
            }
            for v in o.values_mut() {
                migrate(v);
            }
        }
        serde_json::Value::Array(a) => {
            for v in a {
                migrate(v);
            }
        }
        _ => {}
    }
}
