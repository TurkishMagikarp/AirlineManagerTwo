//! Small, deterministic companies using exactly the player's operations and ledger.
use crate::{clock::Clock, game::Game, routes::Route};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REVIEW: u64 = 7 * 86400;

#[derive(Clone, Serialize, Deserialize)]
pub struct Competitor {
    pub game: Box<Game>,
    pub next_review: u64,
    pub losing_reviews: u32,
    pub last_profit: i64,
    pub last_action: String,
}

pub fn initial_competitors() -> Result<Vec<Competitor>, String> {
    initial_competitors_at(Duration::ZERO)
}

pub fn initial_competitors_at(now: Duration) -> Result<Vec<Competitor>, String> {
    let mut companies = Vec::new();
    for (name, origin, destination, model, price) in [
        ("Regional Wings", "EDDF", "EDDM", "atr72_600", 90),
        ("Continental Sky", "EDDF", "LTFM", "a320_200", 150),
    ] {
        let airline = crate::game::Airline {
            name: name.into(),
            country_code: "DE".into(),
            home_airport_icao: origin.into(),
            capital_eur: crate::game::starting_capital()?,
        };
        airline.validate()?;
        let mut g = Game {
            clock: Clock::from_state(now, false, 1).unwrap(),
            history: Some(crate::history::History::new(
                now.as_secs(),
                airline.capital_eur,
            )),
            airline: Some(airline),
            ..Game::default()
        };
        g.buy_aircraft(model)?;
        g.create_route(origin, destination, 1)?;
        g.set_ticket_price(1, price)?;
        g.schedule(1, 1, 360)?;
        companies.push(Competitor {
            game: Box::new(g),
            next_review: now
                .as_secs()
                .checked_add(REVIEW)
                .ok_or("KI-Zeitüberlauf.")?,
            losing_reviews: 0,
            last_profit: 0,
            last_action: "Flugbetrieb aufgenommen".into(),
        });
    }
    Ok(companies)
}

impl Game {
    pub fn market_routes(&self) -> Vec<Route> {
        self.routes
            .iter()
            .chain(self.competitors.iter().flat_map(|c| c.game.routes.iter()))
            .cloned()
            .collect()
    }

    pub fn validate_competitors(&self) -> Result<(), String> {
        if self.competitors.len() > 4 || (self.airline.is_none() && !self.competitors.is_empty()) {
            return Err("Ungültige KI-Airlines.".into());
        }
        let mut names = std::collections::HashSet::new();
        for c in &self.competitors {
            let g = &c.game;
            if !g.clock.is_valid()
                || !g.competitors.is_empty()
                || g.clock.elapsed() != self.clock.elapsed()
                || c.next_review <= self.clock.elapsed().as_secs()
                || c.next_review - self.clock.elapsed().as_secs() > REVIEW
                || c.losing_reviews > 2
                || c.last_action.len() > 200
            {
                return Err("Widersprüchlicher KI-Zustand.".into());
            }
            let a = g.airline.as_ref().ok_or("KI-Airline fehlt.")?;
            if !names.insert(&a.name) {
                return Err("Doppelte KI-Airline.".into());
            }
            a.validate()?;
            g.validate_fleet()?;
            g.validate_routes()?;
            g.validate_operations()?;
            g.validate_finance()?;
            g.validate_history()?;
            if g.history.is_none() {
                return Err("KI-Finanzhistorie fehlt.".into());
            }
        }
        Ok(())
    }

    pub fn advance(&mut self, seconds: f64) -> bool {
        if self.competitors.is_empty() {
            return self.advance_company(seconds, &self.routes.clone());
        }
        let mut target = self.clock.clone();
        if !target.advance(seconds) {
            return false;
        }
        if target.elapsed() == self.clock.elapsed() {
            return true;
        }
        // Transactional across all companies: no partial time or cash updates.
        let mut staged = self.clone();
        while staged.clock.elapsed() < target.elapsed() {
            let review = staged
                .competitors
                .iter()
                .map(|c| c.next_review)
                .min()
                .unwrap();
            let end = target.elapsed().min(Duration::from_secs(review));
            let supply = staged.market_routes();
            // Exact integer/nanosecond target, not repeated floating-point conversion.
            if !staged.advance_company_to(Clock::from_state(end, true, 1).unwrap(), &supply) {
                return false;
            }
            for c in &mut staged.competitors {
                if !c
                    .game
                    .advance_company_to(Clock::from_state(end, true, 1).unwrap(), &supply)
                {
                    return false;
                }
            }
            // All companies settle against the same immutable offer snapshot first.
            let supply = staged.market_routes();
            for c in &mut staged.competitors {
                if c.next_review == end.as_secs() && !c.review(&supply) {
                    return false;
                }
            }
        }
        staged.clock = target;
        *self = staged;
        true
    }
}

impl Competitor {
    fn review(&mut self, supply: &[Route]) -> bool {
        let profit = self.game.finance.totals.profit();
        let Some(period) = profit.checked_sub(self.last_profit) else {
            return false;
        };
        self.last_profit = profit;
        self.losing_reviews = if period < 0 {
            (self.losing_reviews + 1).min(2)
        } else {
            0
        };
        for i in 0..self.game.routes.len() {
            let r = &self.game.routes[i];
            if r.plan.is_none() {
                continue;
            }
            let id = r.id;
            if self.losing_reviews >= 2 && self.game.stop_schedule(id).is_ok() {
                self.last_action = "Verlustverbindung nach zwei Verlustwochen eingestellt".into();
                continue;
            }
            let r = &self.game.routes[i];
            let Some(plane) = self.game.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
                return false;
            };
            // Evaluate nearby fares with the same demand/cost functions as players.
            let old = r.ticket_eur;
            let mut best = (i64::MIN, old);
            for price in [old.saturating_sub(10).max(0), old, (old + 10).min(100_000)] {
                let mut candidate = r.clone();
                candidate.ticket_eur = price;
                let mut offers = supply.to_vec();
                // Replace precisely this offer, not an extra fictional service.
                if let Some(index) = offers.iter().position(|o| o == r) {
                    offers.remove(index);
                }
                offers.push(candidate.clone());
                let Ok(t) = crate::economy::quote_shared(&candidate, &plane.model_id, &offers)
                else {
                    return false;
                };
                if t.profit() > best.0 || (t.profit() == best.0 && price == old) {
                    best = (t.profit(), price);
                }
            }
            if self.game.set_ticket_price(id, best.1).is_err() {
                return false;
            }
            self.last_action = format!("Preisprüfung: {old} → {} EUR", best.1);
        }
        let Some(next) = self.next_review.checked_add(REVIEW) else {
            return false;
        };
        self.next_review = next;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::default();
        g.found("Player", "DE", "EDDF").unwrap();
        g.buy_aircraft("atr72_600").unwrap();
        g.create_route("EDDF", "EDDM", 1).unwrap();
        g.set_ticket_price(1, 90).unwrap();
        g.schedule(1, 1, 360).unwrap();
        g.clock.set_running(true);
        g
    }

    fn state(g: &Game) -> serde_json::Value {
        serde_json::to_value(g).unwrap()
    }

    #[test]
    fn common_market_price_frequency_capacity_and_separate_ledgers() {
        let mut g = game();
        let route = &g.routes[0];
        let supply = g.market_routes();
        let own = crate::demand::passengers(route, &supply).unwrap();
        let rival = &g.competitors[0].game.routes[0];
        assert_eq!(own, crate::demand::passengers(rival, &supply).unwrap());
        let base = crate::demand::market(route, 0).unwrap().total();
        assert!((own * 4) as f64 <= base); // two airlines × two departures/direction
        let mut cheap = route.clone();
        cheap.ticket_eur = 50;
        let offers = vec![cheap.clone(), rival.clone()];
        let cheap_pax = crate::demand::passengers(&cheap, &offers).unwrap();
        let rival_pax = crate::demand::passengers(rival, &offers).unwrap();
        assert!(cheap_pax > own && rival_pax < own);
        let cheap_market = crate::demand::market(&cheap, 0).unwrap().total();
        assert!(((cheap_pax + rival_pax) * 2) as f64 <= cheap_market);
        let mut frequent = route.clone();
        frequent.plan.as_mut().unwrap().interval_seconds /= 2;
        let offers = vec![frequent.clone(), rival.clone()];
        let f = crate::demand::passengers(&frequent, &offers).unwrap();
        let r = crate::demand::passengers(rival, &offers).unwrap();
        assert!(f * 4 > r * 2); // frequency changes daily share, not market size
        assert!((f * 4 + r * 2) as f64 <= base);
        let mut free = route.clone();
        free.ticket_eur = 0;
        free.plan.as_mut().unwrap().interval_seconds = 86400;
        let t = crate::economy::quote(&free, "atr72_600").unwrap();
        assert_eq!(t.passengers, t.seats); // spare demand cannot exceed aircraft capacity
        assert!(crate::economy::quote(&free, "a320_200").unwrap().passengers > t.passengers);
        let before = g.airline.as_ref().unwrap().capital_eur;
        assert!(g.advance(86400.0));
        assert_eq!(g.finance.totals.flights, 4);
        assert_eq!(g.competitors[0].game.finance.totals.flights, 4);
        assert_eq!(
            g.airline.as_ref().unwrap().capital_eur,
            before + g.finance.totals.profit()
        );
        g.validate_competitors().unwrap();
        g.validate_history().unwrap();
    }

    #[test]
    fn mixed_fares_and_frequencies_never_duplicate_the_base_market() {
        let g = game();
        let base = crate::demand::market(&g.routes[0], 0).unwrap().total();
        for p in [0, 50, 90, 130, 100_000] {
            for q in [0, 50, 90, 130, 100_000] {
                for interval in [7200, 21600, 86400 * 7] {
                    let mut a = g.routes[0].clone();
                    let mut b = g.competitors[0].game.routes[0].clone();
                    a.ticket_eur = p;
                    b.ticket_eur = q;
                    b.plan.as_mut().unwrap().interval_seconds = interval;
                    std::mem::swap(&mut b.origin_icao, &mut b.destination_icao);
                    let offers = vec![a.clone(), b.clone()];
                    let daily: f64 = offers
                        .iter()
                        .map(|r| {
                            let frequency =
                                43200.0 / r.plan.as_ref().unwrap().interval_seconds as f64;
                            crate::demand::passengers(r, &offers).unwrap() as f64 * frequency
                        })
                        .sum();
                    assert!(
                        daily <= base,
                        "duplicated market at fares {p}/{q}, interval {interval}"
                    );
                }
            }
        }
    }

    #[test]
    fn speed_steps_reviews_and_restart_produce_identical_results() {
        for speed in Clock::SPEEDS {
            let mut fast = game();
            fast.clock.set_speed(speed);
            let mut slow = fast.clone();
            assert!(fast.advance(30.0 * 86400.0 / f64::from(speed)));
            for _ in 0..720 {
                assert!(slow.advance(3600.0 / f64::from(speed)));
            }
            assert_eq!(state(&fast), state(&slow));
            assert_ne!(fast.competitors[0].last_action, "Flugbetrieb aufgenommen");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("multi.json");
            crate::save::save(&path, &fast).unwrap();
            let mut loaded = crate::save::load(&path).unwrap();
            assert_eq!(state(&fast), state(&loaded));
            assert!(loaded.advance(0.0));
            assert_eq!(state(&fast), state(&loaded));
            assert!(fast.advance(86400.0));
            assert!(loaded.advance(86400.0));
            assert_eq!(state(&fast), state(&loaded));
            fast.clock.set_running(false);
            let paused = state(&fast);
            assert!(fast.advance(1e9));
            assert_eq!(paused, state(&fast));
        }
    }

    #[test]
    fn losses_close_service_without_erasing_aircraft_or_finances() {
        let mut g = game();
        // An overpriced rival runs empty but pays exactly the normal operating costs.
        g.competitors[0].game.routes[0].ticket_eur = 100_000;
        assert!(g.advance(2.0 * REVIEW as f64));
        let c = &g.competitors[0];
        assert!(c.game.routes[0].plan.is_none());
        assert_eq!(c.game.fleet.len(), 1);
        assert!(c.game.finance.totals.profit() < 0);
        assert_eq!(c.game.finance.totals.passengers, 0);
        let flights = c.game.finance.totals.flights;
        assert!(g.advance(REVIEW as f64));
        assert_eq!(g.competitors[0].game.finance.totals.flights, flights);
        g.validate_competitors().unwrap();
    }

    #[test]
    fn old_saves_keep_departure_snapshot_and_start_rivals_only_at_load_time() {
        let mut g = game();
        g.competitors.clear();
        assert!(g.advance(100.25));
        let frozen = g.routes[0].plan.as_ref().unwrap().airborne.clone().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v8.json");
        crate::save::save(&path, &g).unwrap();
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        v["version"] = 8.into();
        v["state"].as_object_mut().unwrap().remove("competitors");
        std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        let mut loaded = crate::save::load(&path).unwrap();
        assert_eq!(loaded.finance, g.finance);
        assert_eq!(loaded.airline, g.airline);
        assert_eq!(loaded.routes, g.routes);
        assert_eq!(loaded.competitors.len(), 2);
        assert!(
            loaded
                .competitors
                .iter()
                .all(|c| c.game.finance.totals.flights == 0)
        );
        assert!(loaded.advance((frozen.landing - 100) as f64));
        assert_eq!(loaded.finance.recent[0], frozen);
        crate::save::save(&path, &loaded).unwrap();
        assert_eq!(state(&loaded), state(&crate::save::load(&path).unwrap()));
    }

    #[test]
    fn price_reviews_preserve_airborne_quotes_for_both_owners() {
        let mut g = game();
        g.competitors[0].next_review = 60;
        assert!(g.advance(59.0));
        let supply = g.market_routes();
        let r = &g.competitors[0].game.routes[0];
        let frozen = crate::operations::departure_quote(r, "atr72_600", 0, &supply).unwrap();
        assert!(g.advance(1.0));
        assert_eq!(
            g.competitors[0].game.routes[0]
                .plan
                .as_ref()
                .unwrap()
                .airborne
                .as_ref(),
            Some(&frozen)
        );
        let player = g.routes[0].plan.as_ref().unwrap().airborne.clone();
        g.set_ticket_price(1, 100_000).unwrap();
        assert_eq!(g.routes[0].plan.as_ref().unwrap().airborne, player);
        assert!(g.advance((frozen.landing - 60) as f64));
        assert_eq!(g.competitors[0].game.finance.recent[0], frozen);
        assert_eq!(g.finance.recent[0], player.unwrap());
        g.validate_competitors().unwrap();
    }

    #[test]
    fn invalid_ai_saves_and_overflow_cannot_partially_commit() {
        let mut g = game();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai.json");
        crate::save::save(&path, &g).unwrap();
        let valid: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("next_review", serde_json::json!(0)),
            ("losing_reviews", serde_json::json!(3)),
        ] {
            let mut bad = valid.clone();
            bad["state"]["competitors"][0][field] = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(crate::save::load(&path).is_err());
        }
        for (pointer, value) in [
            (
                "/state/competitors/0/game/clock/speed_index",
                serde_json::json!(99),
            ),
            (
                "/state/competitors/0/game/routes/0/aircraft_id",
                serde_json::json!(99),
            ),
            ("/state/competitors/0/game/airline", serde_json::Value::Null),
            (
                "/state/competitors/0/game/competitors",
                valid["state"]["competitors"].clone(),
            ),
        ] {
            let mut bad = valid.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
            assert!(
                crate::save::load(&path).is_err(),
                "accepted corrupt AI: {pointer}"
            );
        }
        g.competitors[0].game.airline.as_mut().unwrap().capital_eur = i64::MIN;
        g.competitors[0].game.routes[0].ticket_eur = 100_000;
        let before = state(&g);
        assert!(!g.advance(86400.0));
        assert_eq!(state(&g), before);
    }
}
