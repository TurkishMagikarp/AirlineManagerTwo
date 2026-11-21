mod aircraft;
#[cfg(test)]
mod airport_charge_tests;
mod airports;
mod alliances;
#[cfg(test)]
mod alliances_tests;
mod ancillaries;
#[cfg(test)]
mod ancillary_tests;
mod attention;
#[cfg(test)]
mod attention_tests;
#[cfg(test)]
mod cabin_tests;
mod cabins;
mod cargo;
#[cfg(test)]
mod cargo_tests;
mod charters;
#[cfg(test)]
mod charters_tests;
mod clock;
mod codeshare;
#[cfg(test)]
mod codeshare_tests;
mod competition;
mod demand;
mod difficulty;
#[cfg(test)]
mod difficulty_tests;
#[cfg(test)]
mod disruption_tests;
mod disruptions;
mod economy;
mod emissions;
#[cfg(test)]
mod emissions_tests;
#[cfg(test)]
mod ferry_tests;
mod financing;
#[cfg(test)]
mod financing_tests;
#[cfg(test)]
mod fuel_contract_tests;
mod fuel_contracts;
#[cfg(test)]
mod fuel_tests;
mod game;
mod history;
mod insurance;
#[cfg(test)]
mod insurance_tests;
#[cfg(test)]
mod jump_tests;
mod jumps;
mod labour;
#[cfg(test)]
mod labour_tests;
#[cfg(test)]
mod lifecycle_tests;
mod loyalty;
#[cfg(test)]
mod loyalty_tests;
mod maintenance;
mod map;
mod marketing;
#[cfg(test)]
mod marketing_tests;
#[cfg(test)]
mod milestone_tests;
mod milestones;
mod operations;
mod orders;
#[cfg(test)]
mod orders_tests;
mod pricing;
#[cfg(test)]
mod pricing_tests;
#[cfg(test)]
mod quality;
mod reputation;
#[cfg(test)]
mod reputation_tests;
#[cfg(test)]
mod rotation_tests;
mod route_save;
mod routes;
mod save;
#[cfg(test)]
mod save_slot_tests;
mod save_slots;
#[cfg(test)]
mod seasonal_tests;
mod service;
#[cfg(test)]
mod service_tests;
#[cfg(test)]
mod slot_tests;
mod slots;
mod staff;
#[cfg(test)]
mod staff_tests;
mod tax;
#[cfg(test)]
mod tax_tests;
mod trading;
#[cfg(test)]
mod trading_tests;
#[cfg(test)]
mod transfer_tests;
mod weather;
#[cfg(test)]
mod weather_tests;
#[cfg(test)]
mod widebody_tests;
mod world_economy;
#[cfg(test)]
mod world_economy_tests;
mod world_events;
#[cfg(test)]
mod world_events_tests;

use game::Game;
use godot::prelude::*;

struct AirlineSimulation;

// SAFETY: All extension classes are registered through godot-rust's macros.
#[gdextension]
unsafe impl ExtensionLibrary for AirlineSimulation {}

/// Thin Godot adapter. Historical class name kept for existing callers.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct SimulationClock {
    game: Game,
    jump_worker: Option<std::thread::JoinHandle<jumps::WorkerResult>>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for SimulationClock {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            game: Game::default(),
            jump_worker: None,
            base,
        }
    }
}

#[godot_api]
impl SimulationClock {
    #[func]
    fn get_milestones(&self) -> Array<VarDictionary> {
        let mut rows = Array::new();
        if let Some(state) = &self.game.milestones {
            for (rule, value) in milestones::rules()
                .unwrap_or_default()
                .iter()
                .zip(self.game.milestone_values())
            {
                let mut row = VarDictionary::new();
                row.set("id", rule.id.as_str());
                row.set("target", rule.threshold);
                row.set("current", value);
                row.set(
                    "reached",
                    state.reached.get(&rule.id).map_or(-1, |t| *t as i64),
                );
                rows.push(&row);
            }
        }
        rows
    }
    #[func]
    fn take_milestone_notifications(&mut self) -> PackedStringArray {
        self.game
            .take_milestone_notifications()
            .iter()
            .map(|id| GString::from(id.as_str()))
            .collect()
    }
    #[func]
    fn get_attention_snapshot(&self) -> Array<VarDictionary> {
        self.game
            .attention_snapshot()
            .iter()
            .map(|e| {
                vdict! {
                    "kind" => e.kind, "priority" => e.priority as i64, "text" => e.text.as_str(),
                    "page" => e.page, "route_id" => e.route_id, "aircraft_id" => e.aircraft_id,
                    "employee_id" => e.employee_id, "order_id" => e.order_id as i64
                }
            })
            .collect()
    }

    /// Departure snapshots, not live estimates or already settled finances.
    #[func]
    fn get_active_flights(&self) -> Array<VarDictionary> {
        self.game
            .routes
            .iter()
            .filter_map(|r| r.plan.as_ref()?.airborne.as_ref())
            .chain(
                self.game
                    .ferries
                    .iter()
                    .filter(|f| !f.settled)
                    .map(|f| &f.quote),
            )
            .map(flight_dictionary)
            .collect()
    }

    #[func]
    fn get_finance_history_info(&self) -> VarDictionary {
        self.game
            .history
            .as_ref()
            .map(|h| {
                vdict! {
                    "since" => operations::display_time(h.since).as_str(),
                    "batch_count" => h.batches.len() as i64
                }
            })
            .unwrap_or_default()
    }

    #[func]
    fn get_tax_history(&self) -> Array<VarDictionary> {
        self.game
            .history
            .as_ref()
            .map(|h| {
                h.taxes
                    .iter()
                    .rev()
                    .map(|a| {
                        vdict! { "year" => (a.time / demand::YEAR_SECONDS) as i64,
                        "profit_eur" => a.profit, "tax_eur" => a.amount,
                        "loss_eur" => a.loss, "rate_bps" => a.rate_bps }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[func]
    fn get_route_finances(&self) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        let mut rows = std::collections::BTreeMap::new();
        for r in &self.game.routes {
            rows.insert(
                r.public_id(),
                (
                    r.origin_icao.as_str(),
                    r.destination_icao.as_str(),
                    true,
                    economy::Totals::default(),
                ),
            );
        }
        for b in &h.batches {
            let row = rows.entry(h.public_route(b.route_id)).or_insert((
                b.origin.as_str(),
                b.destination.as_str(),
                false,
                economy::Totals::default(),
            ));
            // Archive totals are validated on load; skip rather than panic over FFI.
            let _ = row.3.add(&b.totals, b.count);
        }
        rows.into_iter()
            .map(|(id, (a, b, active, t))| {
                vdict! {
                    "route_id" => id, "origin" => a, "destination" => b, "active" => active,
                    "connecting_passengers" => t.connecting_passengers, "local_passengers" => t.passengers - t.connecting_passengers,
                    "connecting_revenue_eur" => t.connecting_revenue,
                    "business_revenue_eur" => t.business_revenue, "economy_revenue_eur" => t.revenue - t.business_revenue - t.cargo_revenue - t.ancillary,
                    "business_passengers" => t.business_passengers, "economy_passengers" => t.passengers - t.business_passengers,
                    "fuel_eur" => t.fuel, "airport_eur" => t.airport, "crew_eur" => t.crew, "overhead_eur" => t.overhead, "care_eur" => t.care, "carbon_eur" => t.carbon, "punctuality_percent" => disruptions::percent(&t), "punctuality_flights" => t.punctuality_flights, "service_eur" => t.service, "ancillary_revenue_eur" => t.ancillary, "loyalty_cost_eur" => t.loyalty_cost, "flights" => t.flights, "cargo_kg" => t.cargo_kg, "cargo_revenue_eur" => t.cargo_revenue, "ticket_revenue_eur" => t.revenue - t.cargo_revenue - t.ancillary,
                    "revenue_eur" => t.revenue, "costs_eur" => t.costs(),
                    "profit_eur" => t.profit(), "load_percent" => t.load_percent()
                }
            })
            .collect()
    }

    #[func]
    fn get_route_flight_page(&self, route_id: u32, offset: i64) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        h.page(route_id, offset.max(0) as u64, 50)
            .iter()
            .map(|f| {
                let mut row = flight_dictionary(f);
                row.set("route_id", h.public_route(f.route_id));
                row
            })
            .collect()
    }

    #[func]
    fn get_finance_series(&self, route_id: u32) -> Array<VarDictionary> {
        let Some(h) = &self.game.history else {
            return Array::new();
        };
        let now = self.game.clock.elapsed().as_secs();
        let span = now - h.since;
        let steps = span.clamp(1, 63);
        let mut previous = economy::Totals::default();
        let mut previous_costs = 0i128;
        let mut previous_tax = 0i128;
        (0..=steps).map(|i| {
            let time = h.since + (u128::from(span) * u128::from(i) / u128::from(steps)) as u64;
            let t = h.totals(route_id, time);
            let seats = t.seats - previous.seats;
            let load = if seats == 0 { -1.0 } else { (t.passengers - previous.passengers) as f64 * 100.0 / seats as f64 };
            // Central expenses belong only to the company series, never to
            // every route (which would duplicate them again).
            let costs = i128::from(t.costs()) + if route_id == 0 {
                h.ferry_through(time) + h.maintenance_through(time) + h.payroll_through(time) + h.interest_through(time) + h.rent_through(time) + h.tax_through(time) + h.marketing_through(time) + h.insurance_through(time) + h.alliance_fee_through(time)
            } else { 0 };
            let tax = if route_id == 0 { h.tax_through(time) } else { 0 };
            let period_costs = costs - previous_costs;
            let period_profit = i128::from(t.revenue - previous.revenue) - period_costs;
            let row = vdict! { "seconds" => time as i64, "time" => operations::display_time(time).as_str(),
                "balance" => h.balance(time), "revenue" => t.revenue - previous.revenue,
                "ancillary" => t.ancillary - previous.ancillary,
                "loyalty_cost" => t.loyalty_cost - previous.loyalty_cost,
                "tax" => (tax - previous_tax).min(i128::from(i64::MAX)) as i64,
                "costs" => period_costs.min(i128::from(i64::MAX)) as i64,
                "profit" => period_profit.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
                "load" => load };
            previous = t;
            previous_costs = costs;
            previous_tax = tax;
            row
        }).collect()
    }

    #[func]
    fn get_map_airports(&self) -> Array<VarDictionary> {
        airports::catalog()
            .map(|c| {
                c.airports
                    .iter()
                    .map(|a| {
                        let airlines: Array<VarDictionary> = std::iter::once(&self.game)
                            .chain(self.game.competitors.iter().map(|c| &*c.game))
                            .filter_map(|g| {
                                let airline = g.airline.as_ref()?;
                                let frequency: f64 = g.routes.iter()
                                    .filter(|r| r.origin_icao == a.icao || r.destination_icao == a.icao)
                                    .filter_map(|r| r.plan.as_ref())
                                    .map(|p| 43200.0 / p.interval_seconds as f64).sum();
                                (frequency > 0.0).then(|| vdict! { "airline" => airline.name.as_str(), "departures_per_day" => frequency })
                            }).collect();
                        vdict! {
                            "icao" => a.icao.as_str(), "iata" => a.iata.as_deref().unwrap_or(""), "name" => a.name.as_str(),
                            "slots" => self.game.slot_text(&a.icao).as_str(), "airlines" => &airlines,
                            "position" => Vector2::new(a.longitude as f32, a.latitude as f32)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[func]
    fn get_map_routes(&self) -> Array<VarDictionary> {
        self.game
            .routes
            .iter()
            .filter(|r| r.parent_route_id.is_none())
            .filter_map(|r| {
                let a = map::airport(&r.origin_icao)?;
                let b = map::airport(&r.destination_icao)?;
                let points: PackedVector2Array = (0..=128)
                    .map(|i| {
                        let (lon, lat) = map::position(a, b, f64::from(i) / 128.0);
                        Vector2::new(lon as f32, lat as f32)
                    })
                    .collect();
                Some(vdict! { "id" => r.id, "points" => &points, "active" => self.game.routes.iter().any(|s| s.public_id() == r.id && s.plan.is_some()) })
            })
            .collect()
    }

    /// Small public schedule snapshot; callers compare it before requesting geometry.
    #[func]
    fn get_competitor_routes(&self) -> Array<VarDictionary> {
        map::competitor_routes(&self.game)
            .iter()
            .map(|r| {
                vdict! {
                    "airline" => r.airline.as_str(), "origin_icao" => r.origin_icao.as_str(),
                    "destination_icao" => r.destination_icao.as_str(),
                    "departures_per_day_per_direction" => r.departures_per_day_per_direction,
                    "ticket_eur" => r.ticket_eur, "business_ticket_eur" => r.business_ticket_eur,
                    "model" => r.model.as_str()
                }
            })
            .collect()
    }

    #[func]
    fn get_map_competitor_routes(&self) -> Array<PackedVector2Array> {
        map::competitor_routes(&self.game)
            .iter()
            .filter_map(|r| {
                let a = map::airport(&r.origin_icao)?;
                let b = map::airport(&r.destination_icao)?;
                Some(
                    (0..=128)
                        .map(|i| {
                            let (lon, lat) = map::position(a, b, f64::from(i) / 128.0);
                            Vector2::new(lon as f32, lat as f32)
                        })
                        .collect(),
                )
            })
            .collect()
    }

    #[func]
    fn get_map_flights(&self) -> Array<VarDictionary> {
        map::flights(&self.game)
            .iter()
            .filter_map(|f| {
                let p = self.game.fleet.iter().find(|p| p.id == f.aircraft_id)?;
                let model = aircraft::model(&p.model_id).ok()?;
                Some(vdict! { "aircraft_id" => f.aircraft_id, "route_id" => f.route_id,
                "flight_number" => if f.route_id == 0 { format!("Überführung AM-{:06}", f.aircraft_id) } else { format!("AM-{}-{}", f.route_id, f.departure) }.as_str(),
                "identifier" => p.identifier().as_str(), "model" => model.name.as_str(),
                "origin" => f.origin.as_str(), "destination" => f.destination.as_str(),
                "progress" => f.progress, "arrival" => operations::display_time(f.landing).as_str(),
                "position" => Vector2::new(f.position.0 as f32, f.position.1 as f32),
                "ahead" => Vector2::new(f.ahead.0 as f32, f.ahead.1 as f32) })
            })
            .collect()
    }

    #[func]
    fn get_economy_rules(&self) -> GString {
        economy::rules_text().as_str().into()
    }

    #[func]
    fn set_ticket_price(&mut self, route_id: u32, price: i64) -> GString {
        self.game
            .set_ticket_price(route_id, price)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_fare_suggestion(&self, id: u32) -> VarDictionary {
        match self.game.fare_suggestion(id) {
            Ok(Some(s)) => {
                vdict! { "fare" => s.fare, "business" => s.business.unwrap_or(-1), "current_profit" => s.current_profit, "profit" => s.profit, "at" => self.game.clock.elapsed().as_secs() as i64 }
            }
            _ => VarDictionary::new(),
        }
    }

    #[func]
    fn apply_fare_suggestion(&mut self, id: u32) -> GString {
        self.game
            .apply_fare_suggestion(id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn set_pricing_autopilot(&mut self, id: u32, enabled: bool) -> GString {
        self.game
            .set_pricing_autopilot(id, enabled)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn set_marketing_budget(&mut self, budget: i64) -> GString {
        self.game
            .set_marketing_budget(budget)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn start_fuel_contract(&mut self, share: u32, months: u32) -> GString {
        self.game
            .start_fuel_contract(share, months)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_fuel_contract(&self) -> VarDictionary {
        let now = self.game.clock.elapsed().as_secs();
        let c = self.game.active_fuel_contract();
        let r = &*fuel_contracts::RULES;
        vdict! { "market_cents" => self.game.fuel_market.price_at(now),
        "premium_bps" => r.premium_bps, "month_days" => r.month_days as i64,
        "shares" => &r.shares_percent.iter().map(|s| *s as i64).collect::<PackedInt64Array>().to_variant(),
        "terms" => &r.terms_months.iter().map(|s| *s as i64).collect::<PackedInt64Array>().to_variant(),
        "active" => c.is_some(), "share" => c.map_or(0, |c| c.share_percent as i64),
        "locked_cents" => c.map_or(0, |c| c.locked_cents),
        "remaining_seconds" => c.map_or(0, |c| (c.ends_at - now) as i64),
        "effect_eur" => self.game.finance.totals.fuel_contract_effect,
        "can_start" => self.game.airline.is_some() && !self.game.insolvent() && !self.game.distressed() && self.game.resolution.deadline.is_none() && c.is_none() }
    }

    #[func]
    fn get_marketing(&self) -> VarDictionary {
        let m = &self.game.marketing;
        vdict! { "budget_eur" => m.budget_eur, "awareness" => m.awareness as f64 / marketing::SCALE as f64,
        "trend" => m.trend as f64 / marketing::SCALE as f64,
        "max_budget_eur" => marketing::RULES.max_budget_eur, "month_days" => marketing::RULES.month_days as i64,
        "insolvent" => self.game.insolvent() }
    }

    #[func]
    fn get_marketing_history(&self) -> Array<VarDictionary> {
        self.game.history.as_ref().map_or_else(Array::new, |h| {
            h.marketing_costs.iter().rev().take(100).map(|p| {
            vdict! { "time" => operations::display_time(p.time).as_str(), "amount_eur" => p.amount }
        }).collect()
        })
    }

    #[func]
    fn get_insurance(&self) -> VarDictionary {
        let g = &self.game;
        let p = &g.insurance;
        let mut premium = 0i128;
        let mut deductible = 0i128;
        let mut uninsured = 0i128;
        if let Some(h) = &g.history {
            for e in &h.insurance_costs {
                premium += i128::from(e.premium);
                deductible += i128::from(e.deductible);
                uninsured += i128::from(e.uninsured);
            }
        }
        let claims: Array<VarDictionary> = p.recent.iter().rev().map(|c| {
            vdict! { "time" => operations::display_time(c.time).as_str(), "aircraft" => c.aircraft as i64,
                "kind" => ["Vogelschlag", "Bodenschaden", "Harte Landung"][c.kind.min(2) as usize],
                "repair" => c.repair, "paid" => c.paid, "insurer" => c.insurer }
        }).collect();
        vdict! { "coverage" => p.coverage.name(), "annual" => p.annual_premium,
        "basic_annual" => g.insurance_quote(insurance::Coverage::Basic).unwrap_or(0),
        "full_annual" => g.insurance_quote(insurance::Coverage::Full).unwrap_or(0),
        "basic_deductible_bps" => insurance::RULES.basic.deductible_bps,
        "full_deductible_bps" => insurance::RULES.full.deductible_bps,
        "basic_limit_bps" => insurance::RULES.basic.limit_bps, "full_limit_bps" => insurance::RULES.full.limit_bps,
        "loading_bps" => p.loading_bps,
        "premium" => premium.min(i128::from(i64::MAX)) as i64,
        "deductible" => deductible.min(i128::from(i64::MAX)) as i64,
        "uninsured" => uninsured.min(i128::from(i64::MAX)) as i64,
        "claims" => &claims, "can_change" => g.airline.is_some() && !g.insolvent() }
    }

    #[func]
    fn set_insurance(&mut self, level: GString) -> GString {
        let coverage = match level.to_string().as_str() {
            "none" => insurance::Coverage::None,
            "basic" => insurance::Coverage::Basic,
            "full" => insurance::Coverage::Full,
            _ => return "Unbekannte Deckung.".into(),
        };
        self.game
            .set_coverage(coverage)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_world_economy(&self) -> VarDictionary {
        let index = self
            .game
            .world_economy
            .index_at(self.game.clock.elapsed().as_secs());
        let r = &*world_economy::RULES;
        vdict! { "percent" => index as f64 / 100.0, "phase" => world_economy::phase(index),
        "business" => world_economy::factor(index, r.business_reaction),
        "leisure" => world_economy::factor(index, r.leisure_reaction),
        "cargo" => world_economy::factor(index, r.cargo_reaction) }
    }

    /// Public forecast: announced, not yet finished events, soonest first.
    #[func]
    fn get_world_events(&self) -> Array<VarDictionary> {
        let now = self.game.clock.elapsed().as_secs();
        let mine: Vec<&str> = self
            .game
            .airline
            .iter()
            .map(|a| a.home_airport_icao.as_str())
            .chain(
                self.game
                    .routes
                    .iter()
                    .flat_map(|r| [r.origin_icao.as_str(), r.destination_icao.as_str()]),
            )
            .collect();
        let airports = airports::catalog().map(|c| &c.airports[..]).unwrap_or(&[]);
        let weather = weather::visible_at(&weather::Weather::of(&self.game.world_economy), now);
        let mut forecast: Vec<(u64, u64, VarDictionary)> = weather
            .iter()
            .map(|e| {
                (e.starts_at(), e.id, vdict! {
                    "id" => e.id as i64, "kind" => "weather", "scope" => "airport", "place_code" => e.icao,
                    "place" => e.place_name().as_str(),
                    "weather_kind" => e.kind.key.as_str(), "weather_name" => e.kind.name.as_str(),
                    "delay_seconds" => e.delay_seconds as i64,
                    "status" => if e.is_active(now) { "active" } else { "upcoming" },
                    "announced_at" => e.announced_at() as i64, "starts_at" => e.starts_at() as i64,
                    "ends_at" => e.ends_at() as i64,
                    "business" => 1.0, "leisure" => 1.0, "cargo" => 1.0,
                    "mine" => mine.contains(&e.icao),
                    "airports" => &PackedStringArray::from([GString::from(e.icao)])
                })
            })
            .collect();
        forecast.extend(world_events::visible_at(&self.game.world_economy, now)
            .iter()
            .map(|e| {
                let f = e.factors();
                let covered: PackedStringArray = airports
                    .iter()
                    .filter(|a| e.touches(&a.icao))
                    .map(|a| GString::from(a.icao.as_str()))
                    .collect();
                let (scope, code) = match e.place {
                    world_events::Place::Airport(code) => ("airport", code),
                    world_events::Place::Country(code) => ("country", code),
                };
                let dict = vdict! {
                    "id" => e.id as i64, "kind" => e.kind.key(), "scope" => scope, "place_code" => code,
                    "place" => e.place_name().as_str(),
                    "status" => if e.is_active(now) { "active" } else { "upcoming" },
                    "announced_at" => e.announced_at() as i64, "starts_at" => e.starts_at() as i64,
                    "ends_at" => e.ends_at() as i64,
                    "business" => f.business, "leisure" => f.leisure, "cargo" => f.cargo,
                    "mine" => mine.iter().any(|icao| e.touches(icao)),
                    "airports" => &covered
                };
                (e.starts_at(), e.id, dict)
            }));
        forecast.sort_by_key(|(start, id, _)| (*start, *id));
        forecast.into_iter().map(|(_, _, d)| d).collect()
    }

    #[func]
    fn take_economy_notifications(&mut self) -> PackedStringArray {
        self.game
            .world_economy
            .take_notifications(self.game.clock.elapsed().as_secs())
            .into_iter()
            .map(GString::from)
            .collect()
    }

    #[func]
    fn get_financial_summary(&self) -> VarDictionary {
        let t = &self.game.finance.totals;
        let purchases = self.game.history.as_ref().map_or(0, |h| {
            h.purchases
                .iter()
                .map(|p| i128::from(p.amount))
                .sum::<i128>()
                .min(i128::from(i64::MAX)) as i64
        });
        let rent = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.rent_through(u64::MAX));
        let aircraft_cash = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.aircraft_cash_through(u64::MAX));
        let maintenance = self
            .game
            .history
            .as_ref()
            .map(|h| h.maintenance_through(u64::MAX).min(i128::from(i64::MAX)) as i64)
            .unwrap_or(0);
        let payroll = self
            .game
            .history
            .as_ref()
            .map(|h| h.payroll_through(u64::MAX).min(i128::from(i64::MAX)) as i64)
            .unwrap_or(0);
        let interest = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.interest_through(u64::MAX));
        let tax = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.tax_through(u64::MAX));
        let last_tax = self.game.history.as_ref().and_then(|h| h.taxes.last());
        let marketing = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.marketing_through(u64::MAX));
        let insurance = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.insurance_through(u64::MAX));
        let ferry_cost: i128 = self
            .game
            .history
            .iter()
            .flat_map(|h| h.ferries.iter())
            .map(|f| i128::from(f.quote.totals.costs()))
            .sum();
        let alliance_fees = self
            .game
            .history
            .as_ref()
            .map_or(0, |h| h.alliance_fee_through(u64::MAX));
        let costs = ferry_cost
            + alliance_fees
            + insurance
            + marketing
            + tax
            + i128::from(t.costs())
            + i128::from(maintenance)
            + i128::from(payroll)
            + interest
            + rent;
        let charter_fees = self.game.history.as_ref().map_or(0, |h| {
            h.charter_through(u64::MAX, trading::Kind::CharterFee)
        });
        let charter_penalties = -self.game.history.as_ref().map_or(0, |h| {
            h.charter_through(u64::MAX, trading::Kind::CharterPenalty)
        });
        let profit = i128::from(t.revenue) + charter_fees - charter_penalties - costs;
        let company_revenue = i128::from(t.revenue) + charter_fees;
        let company_costs = costs + charter_penalties;
        let now = self.game.clock.elapsed().as_secs();
        vdict! { "carbon_eur_per_tonne" => emissions::price_at(now),
        "carbon_next_eur_per_tonne" => emissions::price_at(now.saturating_add(emissions::year_seconds())),
        "fuel_cents_per_kg" => self.game.fuel_market.price_at(self.game.clock.elapsed().as_secs()),
        "fuel_month_delta_cents" => self.game.fuel_market.price_at(self.game.clock.elapsed().as_secs()) - self.game.fuel_market.price_at(self.game.clock.elapsed().as_secs().saturating_sub(30 * 86400)),
        "connecting_passengers" => t.connecting_passengers, "connecting_revenue_eur" => t.connecting_revenue,
        "local_passengers" => t.passengers - t.connecting_passengers,
        "flights" => t.flights, "passengers" => t.passengers, "cargo_kg" => t.cargo_kg, "cargo_revenue_eur" => t.cargo_revenue, "ticket_revenue_eur" => t.revenue - t.cargo_revenue - t.ancillary,
                    "revenue_eur" => t.revenue,
        "reputation_percent" => self.game.reputation.percent(),
        "reputation_trend" => self.game.reputation.trend_text(),
        "marketing_eur" => marketing.min(i128::from(i64::MAX)) as i64,
        "alliance_fee_eur" => alliance_fees.min(i128::from(i64::MAX)) as i64,
        "tax_eur" => tax.min(i128::from(i64::MAX)) as i64,
        "tax_year_eur" => last_tax.map_or(0, |a| a.amount),
        "tax_year" => last_tax.map_or(0, |a| a.time / demand::YEAR_SECONDS) as i64,
        "tax_loss_eur" => self.game.tax_loss(), "tax_rate_bps" => self.game.tax_rate(),
        "charter_revenue_eur" => charter_fees.min(i128::from(i64::MAX)) as i64,
        "charter_penalty_eur" => charter_penalties.min(i128::from(i64::MAX)) as i64,
        "ferry_eur" => ferry_cost.min(i128::from(i64::MAX)) as i64,
        "maintenance_eur" => maintenance, "payroll_eur" => payroll,
        "rent_eur" => rent.min(i128::from(i64::MAX)) as i64,
        "aircraft_cash_eur" => aircraft_cash.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
        "fuel_eur" => t.fuel, "carbon_eur" => t.carbon, "airport_eur" => t.airport, "crew_eur" => t.crew, "overhead_eur" => t.overhead, "care_eur" => t.care, "punctuality_percent" => disruptions::percent(t), "punctuality_flights" => t.punctuality_flights, "service_eur" => t.service, "ancillary_revenue_eur" => t.ancillary, "loyalty_cost_eur" => t.loyalty_cost,
        "costs_eur" => costs.min(i128::from(i64::MAX)) as i64,
        "company_revenue_eur" => company_revenue.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
        "company_costs_eur" => company_costs.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64,
        "profit_eur" => profit.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64, "purchases_eur" => purchases,
        "load_percent" => t.load_percent() }
    }

    #[func]
    fn get_financing(&self) -> VarDictionary {
        let g = &self.game;
        let r = &*financing::RULES;
        let interest = g
            .history
            .as_ref()
            .map_or(0, |h| h.interest_through(u64::MAX));
        vdict! {
            "cash_eur" => g.cash(), "available_cash_eur" => g.cash().max(0),
            "debt_eur" => g.financing.debt, "overdue_eur" => g.financing.overdue,
            "interest_eur" => interest.min(i128::from(i64::MAX)) as i64,
            "next_interest_eur" => g.loan_interest(), "payment_eur" => g.loan_payment(),
            "payroll_eur" => g.next_payroll_amount().unwrap_or(i64::MAX),
            "payroll_due" => g.next_payroll_event().map(operations::display_time).unwrap_or_else(|| "—".into()).as_str(),
            "next_due" => if g.financing.debt > 0 { operations::display_time(g.financing.next_due) } else { "—".into() }.as_str(),
            "distressed" => g.distressed(), "status" => g.financing_status(),
            "insolvent" => g.insolvent(), "can_reschedule" => g.can_reschedule(),
            "deadline" => g.resolution.deadline.map(operations::display_time).unwrap_or_else(|| "—".into()).as_str(),
            "rescue_rules" => format!("Finanznot: {} Tage Schonfrist, danach {} Tage Restrukturierung. Einmalige Umschuldung: {} Raten, {:.2} % Gebühr auf Restschuld (jetzt {} EUR), Zinssatz unverändert; Rückstände werden gestundet, Frist bleibt bestehen. Keine Geldzufuhr. Gebühr in Finanzierungskosten enthalten.", r.distress_grace_days, r.restructuring_days, r.reschedule_term_periods, r.reschedule_fee_bps as f64 / 100.0, g.reschedule_fee()).as_str(),
            "limit_eur" => r.maximum_loan_eur,
            "can_borrow" => g.airline.is_some() && !g.insolvent() && g.financing.debt == 0,
            "rules" => format!("Ein Kredit bis {} EUR · {:.2} % p.a. · Rate alle {} Tage · {} Raten. Zinsen auf Restschuld, ganze EUR aufgerundet, Jahr = 365 Tage. Sondertilgung kostenlos; keine automatische Kreditaufnahme. Negative Kontostände sind offene Verpflichtungen, kein frei verfügbares Geld.", r.maximum_loan_eur, r.annual_interest_bps as f64 / 100.0, r.repayment_period_days, r.term_periods).as_str()
        }
    }

    #[func]
    fn reschedule_loan(&mut self) -> GString {
        self.game
            .reschedule_loan()
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn borrow_money(&mut self, amount: i64) -> GString {
        self.game
            .borrow(amount)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn repay_loan(&mut self, amount: i64) -> GString {
        self.game
            .repay(amount)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_flight_results(&self) -> Array<VarDictionary> {
        self.game
            .finance
            .recent
            .iter()
            .rev()
            .map(|f| {
                let mut row = flight_dictionary(f);
                row.set(
                    "route_id",
                    self.game
                        .history
                        .as_ref()
                        .map_or(f.route_id, |h| h.public_route(f.route_id)),
                );
                row
            })
            .collect()
    }

    #[func]
    fn schedule_route(
        &mut self,
        route_id: u32,
        delay_minutes: u32,
        interval_minutes: u32,
    ) -> GString {
        self.game
            .schedule(route_id, delay_minutes, interval_minutes)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn stop_schedule(&mut self, route_id: u32) -> GString {
        self.game
            .stop_schedule(route_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_operations(&self) -> Array<VarDictionary> {
        let mut rows = Vec::new();
        for route in &self.game.routes {
            let Some(plane) = self
                .game
                .fleet
                .iter()
                .find(|p| Some(p.id) == route.aircraft_id)
            else {
                continue;
            };
            let Ok((flight, ground)) = operations::timings(route, &plane.model_id) else {
                continue;
            };
            let state =
                operations::movement(route, &plane.model_id, self.game.clock.elapsed().as_secs())
                    .ok();
            let text = if let Some(s) = &state {
                let phase = match s.status {
                    aircraft::AircraftStatus::InFlight => format!(
                        "Unterwegs {} → {} · Landung {}",
                        s.origin,
                        s.destination,
                        operations::display_time(s.landing)
                    ),
                    aircraft::AircraftStatus::Ground => format!(
                        "Gelandet in {} · Bodenzeit bis {}",
                        s.location,
                        operations::display_time(s.ready)
                    ),
                    aircraft::AircraftStatus::Parked | aircraft::AircraftStatus::Maintenance => {
                        format!("Geplant · bereit in {}", s.location)
                    }
                };
                let prior = route.plan.as_ref().map(|p| p.prior_flights).unwrap_or(0);
                let hangar = plane
                    .maintenance
                    .as_ref()
                    .map(|v| {
                        format!(
                            " · Wartung ab Landung bis {}",
                            operations::display_time(v.end)
                        )
                    })
                    .unwrap_or_default();
                format!(
                    "{} · {} Landungen{} · {} min Verspätung{}",
                    phase,
                    s.completed + prior,
                    hangar,
                    s.delay_seconds.div_ceil(60),
                    if s.weather_seconds > 0 {
                        " (Wetter)"
                    } else {
                        ""
                    }
                )
            } else if let Some(v) = plane
                .maintenance
                .as_ref()
                .filter(|_| plane.status == aircraft::AircraftStatus::Maintenance)
            {
                let resume = v.resume.as_ref().is_some_and(|r| r.route_id == route.id);
                format!(
                    "In Wartung bis {}{}",
                    operations::display_time(v.end),
                    if resume {
                        " · Flugplan wird danach fortgesetzt"
                    } else {
                        ""
                    }
                )
            } else {
                "Kein Flugplan".into()
            };
            let text = if route.remove_at.is_some() {
                format!("{text} · Freigabe nach Bodenzeit vorgemerkt")
            } else {
                text
            };
            let board = operations::board_row(route, plane, state.as_ref());
            let next_departure = board.next_departure;
            rows.push((next_departure.unwrap_or(u64::MAX), vdict! {
                "state_kind" => board.state_kind, "release_pending" => board.release_pending,
                "origin" => board.origin.as_str(), "destination" => board.destination.as_str(),
                "delay_seconds" => board.delay_seconds as i64,
                "weather_delay" => board.weather_delay,
                "expected_landing_seconds" => board.expected_landing.map(|t| t as i64).unwrap_or(-1),
                "expected_landing" => board.expected_landing.map(operations::display_time).unwrap_or_else(|| "—".into()).as_str(),
                "route_id" => route.public_id(), "aircraft_id" => plane.id,
                "identifier" => plane.identifier().as_str(), "status_text" => text.as_str(),
                "has_plan" => route.plan.is_some(),
                "flight_seconds" => flight as i64, "ground_seconds" => ground as i64,
                "completed_flights" => state.as_ref().map(|s| s.completed as i64).unwrap_or(0),
                "next_departure_seconds" => next_departure.map(|t| t as i64).unwrap_or(-1),
                "next_departure" => next_departure.map(operations::display_time).unwrap_or_else(|| "—".into()).as_str(),
                "landing_seconds" => state.as_ref().map(|s| s.landing as i64).unwrap_or(-1),
                "ready_seconds" => state.as_ref().map(|s| s.ready as i64).unwrap_or(-1),
            }));
        }
        rows.sort_by_key(|(time, _)| *time);
        rows.into_iter().map(|(_, row)| row).collect()
    }

    #[func]
    fn preview_route(
        &self,
        origin: GString,
        destination: GString,
        aircraft_id: u32,
        route_id: u32,
    ) -> VarDictionary {
        let a = origin.to_string();
        let b = destination.to_string();
        let distance = routes::distance_km(&a, &b).unwrap_or(0.0);
        let error = self
            .game
            .check_route(&a, &b, aircraft_id, (route_id != 0).then_some(route_id))
            .and_then(|distance| self.game.check_slots(&a, &b, 43200).map(|()| distance))
            .err()
            .unwrap_or_default();
        let quote = self.game.planning_quote(&a, &b, aircraft_id).ok();
        vdict! { "distance_km" => distance, "error" => error.as_str(),
        "origin_slots" => self.game.slot_text(&a).as_str(),
        "destination_slots" => self.game.slot_text(&b).as_str(),
        "has_cost_estimate" => quote.is_some(),
        "estimated_airport_eur" => quote.as_ref().map_or(0, |q| q.airport),
        "estimated_costs_eur" => quote.as_ref().map_or(0, |q| q.costs()),
        "estimated_passengers" => quote.as_ref().map_or(0, |q| q.passengers) }
    }

    #[func]
    fn get_route_destinations(&self, origin: GString, aircraft_id: u32) -> Array<VarDictionary> {
        let range = self.game.aircraft_range_km(aircraft_id);
        routes::destination_options(&origin.to_string(), range)
            .unwrap_or_default()
            .into_iter()
            .map(|o| {
                vdict! { "icao" => o.icao.as_str(), "iata" => o.iata.as_str(),
                "name" => o.name.as_str(), "charge_percent" => o.charge_percent,
                "distance_km" => o.distance_km, "in_range" => o.in_range,
                "ranged" => range.is_some() }
            })
            .collect()
    }

    #[func]
    fn create_route(&mut self, origin: GString, destination: GString, aircraft_id: u32) -> GString {
        self.game
            .create_route(&origin.to_string(), &destination.to_string(), aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn assign_route(&mut self, route_id: u32, aircraft_id: u32) -> GString {
        self.game
            .assign_route(route_id, (aircraft_id != 0).then_some(aircraft_id))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn add_route_aircraft(&mut self, route_id: u32, aircraft_id: u32) -> GString {
        self.game
            .add_route_aircraft(route_id, aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn remove_route_aircraft(&mut self, route_id: u32, aircraft_id: u32) -> GString {
        self.game
            .remove_route_aircraft(route_id, aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn wind_down_route(&mut self, route_id: u32) -> GString {
        self.game
            .wind_down_route(route_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn delete_route(&mut self, route_id: u32) -> GString {
        self.game
            .delete_route(route_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn propose_codeshare(&mut self, partner: GString) -> GString {
        self.game
            .propose_codeshare(&partner.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn end_codeshare(&mut self, partner: GString) -> GString {
        self.game
            .end_codeshare(&partner.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn get_codeshares(&self) -> Array<VarDictionary> {
        let mut partners: std::collections::BTreeSet<String> = self
            .game
            .competitors
            .iter()
            .map(|c| c.game.market_owner_key())
            .collect();
        partners.extend(
            self.game
                .finance
                .totals
                .codeshare
                .iter()
                .map(|r| r.partner.clone()),
        );
        partners
            .into_iter()
            .map(|partner| {
                let active = self.game.codeshares.iter().any(|a| a.partner == partner);
                let receipt = self
                    .game
                    .finance
                    .totals
                    .codeshare
                    .iter()
                    .find(|r| r.partner == partner);
                let reason = if active {
                    String::new()
                } else {
                    self.game
                        .codeshare_reason(&partner)
                        .err()
                        .unwrap_or_default()
                };
                let name = partner
                    .split_once(':')
                    .map_or(partner.as_str(), |(_, name)| name);
                vdict! { "partner" => partner.as_str(), "name" => name, "active" => active,
                "reason" => reason.as_str(), "passengers" => receipt.map_or(0, |r| r.passengers),
                "revenue" => receipt.map_or(0, |r| r.revenue) }
            })
            .collect()
    }

    #[func]
    fn apply_alliance(&mut self, alliance: GString) -> GString {
        self.game
            .apply_alliance(&alliance.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn leave_alliance(&mut self) -> GString {
        self.game
            .leave_alliance()
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    /// One row per alliance with members, the player's fee and qualification.
    #[func]
    fn get_alliances(&self) -> Array<VarDictionary> {
        let g = &self.game;
        let name_of = |owner: &str| -> String {
            if owner == alliances::PLAYER {
                return g.airline.as_ref().map_or(String::new(), |a| a.name.clone());
            }
            owner.split_once(':').map_or(owner, |(_, n)| n).to_string()
        };
        let fee = alliances::monthly_fee(g);
        let mine = g.alliance_of(alliances::PLAYER);
        alliances::RULES
            .alliances
            .iter()
            .map(|d| {
                let members: Array<VarDictionary> = g
                    .alliance_members(&d.id)
                    .map(|m| {
                        vdict! { "name" => name_of(&m.owner).as_str(), "player" => m.owner == alliances::PLAYER,
                        "leaving" => m.leaving_at.is_some_and(|_| true),
                        "since" => operations::display_time(m.joined_at).as_str() }
                    })
                    .collect();
                let state = if mine.is_some_and(|m| m.alliance == d.id) {
                    if mine.is_some_and(|m| m.leaving_at.is_some()) { "leaving" } else { "member" }
                } else if g.alliances.application.as_ref().is_some_and(|a| a.alliance == d.id) {
                    "pending"
                } else {
                    "none"
                };
                let check = if state == "none" { g.alliance_check(alliances::PLAYER, &d.id).err() } else { None };
                let (overlap, shared) = g.alliance_overlap(alliances::PLAYER, &d.id);
                let args: Array<GString> = check
                    .as_ref()
                    .map_or_else(Vec::new, |r| r.args.iter().map(|a| GString::from(a.as_str())).collect())
                    .into_iter()
                    .collect();
                vdict! { "id" => d.id.as_str(), "name" => d.name.as_str(), "members" => &members,
                "member_count" => g.alliance_members(&d.id).count() as i64, "state" => state,
                "qualifies" => state == "none" && check.is_none(),
                "reason" => check.as_ref().map_or("", |r| r.code), "reason_args" => &args,
                "overlap_percent" => i64::from(overlap), "shared_airports" => shared as i64,
                "fee_eur" => fee, "notice_ends" => mine.filter(|m| m.alliance == d.id).and_then(|m| m.leaving_at).map_or_else(String::new, operations::display_time).as_str() }
            })
            .collect()
    }
    #[func]
    fn get_alliance_rules(&self) -> VarDictionary {
        let r = &*alliances::RULES;
        let g = &self.game;
        let paid = g
            .history
            .as_ref()
            .map_or(0, |h| h.alliance_fee_through(u64::MAX));
        vdict! { "review_days" => r.review_days as i64, "notice_days" => r.leave_notice_days as i64,
        "cooldown_days" => r.reapply_cooldown_days as i64, "min_routes" => r.min_routes as i64,
        "min_reputation" => r.min_reputation_points, "max_overlap" => i64::from(r.max_overlap_percent),
        "min_cash_months" => r.min_cash_months, "max_members" => r.max_members as i64,
        "bonus_bps" => r.demand_bonus_bps_per_partner, "bonus_max_bps" => r.demand_bonus_max_bps,
        "fee_eur" => alliances::monthly_fee(g), "fees_paid_eur" => paid.min(i128::from(i64::MAX)) as i64,
        "next_review" => g.next_alliance_event().map_or_else(String::new, operations::display_time).as_str(),
        "pending" => g.alliances.application.as_ref().and_then(|a| alliances::def(&a.alliance)).map_or("", |d| d.name.as_str()) }
    }

    #[func]
    fn get_competitors(&self) -> Array<VarDictionary> {
        self.game
            .competitors
            .iter()
            .map(|c| {
                let g = &c.game;
                let routes = g
                    .routes
                    .iter()
                    .map(|r| {
                        format!(
                            "{} ↔ {} · {} EUR · {}",
                            r.origin_icao,
                            r.destination_icao,
                            r.ticket_eur,
                            if r.plan.is_some() { "aktiv" } else { "inaktiv" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let fleet = g
                    .fleet
                    .iter()
                    .map(|p| {
                        let condition = maintenance::rules(&p.model_id)
                            .map(|r| p.wear.condition_permille(r) as f64 / 10.0)
                            .unwrap_or(0.0);
                        let hangar = match (&p.maintenance, &p.status) {
                            (Some(v), aircraft::AircraftStatus::Maintenance) => {
                                format!(" · in Wartung bis {}", operations::display_time(v.end))
                            }
                            (Some(_), _) => " · Wartung nach Landung".into(),
                            _ => String::new(),
                        };
                        format!(
                            "{} · {} · {} · Kauf {} EUR · Zustand {:.0} % · {} Checks{}",
                            p.identifier(),
                            p.model_id,
                            p.location_icao,
                            p.purchase_price_eur,
                            condition,
                            p.wear.checks,
                            hangar
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let purchases = g
                    .history
                    .as_ref()
                    .unwrap()
                    .purchases
                    .iter()
                    .map(|p| format!("{}: {} EUR", operations::display_time(p.time), p.amount))
                    .collect::<Vec<_>>()
                    .join(" · ");
                vdict! { "airline" => g.airline.as_ref().unwrap().name.as_str(),
                "balance" => g.airline.as_ref().unwrap().capital_eur,
                "fleet_size" => g.fleet.len() as i64, "routes" => routes.as_str(),
                "maintenance_eur" => g.history.as_ref().map(|h| h.maintenance_through(u64::MAX).min(i128::from(i64::MAX)) as i64).unwrap_or(0),
                "fleet" => fleet.as_str(), "purchases" => purchases.as_str(),
                "last_action" => c.last_action.as_str() }
            })
            .collect()
    }

    #[func]
    fn get_route_competition(&self, route_id: u32) -> Array<VarDictionary> {
        let Some(route) = self.game.routes.iter().find(|r| r.id == route_id) else {
            return Array::new();
        };
        struct Offer<'a> {
            airline: &'a str,
            route: &'a routes::Route,
            model: &'a str,
            seats: u32,
            /// Departures per day in each direction.
            frequency: f64,
            passengers: f64,
            connecting: f64,
            action: &'a str,
        }
        let supply = self.game.market_routes();
        let mut offers = Vec::new();
        for (company, action) in std::iter::once((&self.game, "Spieler")).chain(
            self.game
                .competitors
                .iter()
                .map(|c| (&*c.game, c.last_action.as_str())),
        ) {
            let Some(airline) = &company.airline else {
                continue;
            };
            for r in company
                .routes
                .iter()
                .filter(|r| demand::same_market(route, r))
            {
                let Some(plane) = company.fleet.iter().find(|p| Some(p.id) == r.aircraft_id) else {
                    continue;
                };
                let Ok(model) = aircraft::model(&plane.model_id) else {
                    continue;
                };
                let frequency = r
                    .plan
                    .as_ref()
                    .map(|p| 43200.0 / p.interval_seconds as f64)
                    .unwrap_or(0.0);
                let company_supply = company.prioritize_market(&supply);
                let seats = cabins::offer(r, plane).map_or(model.seats, |o| o.seats);
                let estimate = demand::estimate_at(
                    r,
                    seats,
                    &company_supply,
                    self.game.clock.elapsed().as_secs(),
                )
                .unwrap_or_default();
                let passengers = estimate.per_departure as f64 * frequency;
                let connecting = estimate.connecting as f64 * frequency;
                offers.push(Offer {
                    airline: airline.name.as_str(),
                    route: r,
                    model: &plane.model_id,
                    seats,
                    frequency,
                    passengers,
                    connecting,
                    action,
                });
            }
        }
        let total: f64 = offers.iter().map(|o| o.passengers - o.connecting).sum();
        offers
            .into_iter()
            .map(|o| {
                vdict! {
                    "airline" => o.airline, "ticket_eur" => o.route.ticket_eur, "model" => o.model,
                    "seats" => o.seats, "departures_per_day_per_direction" => o.frequency,
                    "passengers_per_day" => o.passengers,
                    "local_per_day" => o.passengers - o.connecting, "connecting_per_day" => o.connecting,
                    "share_percent" => if total > 0.0 { (o.passengers - o.connecting) * 100.0 / total } else { 0.0 },
                    "active" => o.route.plan.is_some(), "last_action" => o.action
                }
            })
            .collect()
    }

    #[func]
    fn get_routes(&self) -> Array<VarDictionary> {
        let supply = self.game.market_routes();
        let now = self.game.clock.elapsed().as_secs();
        self.game
            .routes
            .iter()
            .filter(|r| r.parent_route_id.is_none())
            .filter_map(|r| {
                let season = demand::season_factors(now);
                let context = world_economy::Context { cycle: self.game.world_economy, quoted_at: now };
                let market = demand::market_in_economy(r, r.ticket_eur, context, Some(now)).ok()?;
                let base = demand::market_in_economy(r, 0, context, Some(now)).ok()?;
                let index = self.game.world_economy.index_at(now);
                let route_event = world_events::route_factors(&self.game.world_economy, now, &r.origin_icao, &r.destination_icao);
                let (event_kinds, event_places): (PackedStringArray, PackedStringArray) = world_events::visible_at(&self.game.world_economy, now)
                    .iter()
                    .filter(|e| e.is_active(now) && (e.touches(&r.origin_icao) || e.touches(&r.destination_icao)))
                    .map(|e| (GString::from(e.kind.key()), GString::from(e.place_name().as_str())))
                    .unzip();
                let services: Vec<_> = self.game.routes.iter().filter(|s| s.public_id() == r.id).collect();
                let aircraft_ids: Array<u32> = services.iter().filter_map(|s| s.aircraft_id).collect();
                let has_plan = services.iter().any(|s| s.plan.is_some());
                let pending = self.game.pending_releases(r.id);
                let pending_rows: Array<VarDictionary> = pending.iter().map(|(id, at)| vdict! {
                    "aircraft_id" => *id,
                    "identifier" => self.game.fleet.iter().find(|p| p.id == *id).map(|p| p.identifier()).unwrap_or_default().as_str(),
                    "at_seconds" => *at as i64,
                    "at" => operations::display_time(*at).as_str(),
                }).collect();
                let freighter_only = !aircraft_ids.is_empty() && services.iter().filter_map(|s| self.game.fleet.iter().find(|p| Some(p.id) == s.aircraft_id)).all(|p| aircraft::model(&p.model_id).is_ok_and(|m| m.main_deck_cargo_kg > 0));
                let mut freight_kg = 0;
                let mut freight_capacity = 0;
                let mut estimate = demand::Estimate::default();
                let mut connections: Array<VarDictionary> = Array::new();
                let mut service_cost = 0;
                let mut airport_cost = 0;
                for service in &services {
                    if service.plan.is_none() { continue; }
                    if let Some(plane) = self.game.fleet.iter().find(|p| Some(p.id) == service.aircraft_id)
                        && let Ok(model) = aircraft::model(&plane.model_id)
                        && let Ok(e) = demand::estimate_at(service, cabins::offer(service, plane).map_or(model.seats, |o| o.seats), &supply, now) {
                        if let Ok(q) = economy::quote_at_time(service, &plane.model_id, &supply, self.game.fuel_market.price_at(now), now) {
                            freight_kg += q.cargo_kg;
                            freight_capacity += cargo::capacity(model, q.passengers);
                        }
                        estimate.daily_connecting += e.daily_connecting;
                        for c in &e.connections {
                            connections.push(&vdict! { "origin" => c.origin.as_str(), "hub" => c.hub.as_str(), "destination" => c.destination.as_str(), "daily" => c.daily, "partner" => c.partner.split_once(':').map_or("", |(_, name)| name) });
                        }
                        estimate.daily_passengers += e.daily_passengers;
                        estimate.daily_spill += e.daily_spill;
                        estimate.per_departure += e.per_departure;
                        service_cost += service.service_level.cost(e.per_departure, e.business);
                        airport_cost += economy::airport_bill(service, &plane.model_id, e.per_departure).unwrap_or(0);
                    }
                }
                let maintenance = services.iter().map(|s| self.route_maintenance_status(s)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
                let staff = services.iter().map(|s| self.route_staff_status(s)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n");
                let active_count = services.iter().filter(|s| s.plan.is_some()).count() as i64;
                estimate.per_departure /= active_count.max(1);
                let actual = self
                    .game
                    .history
                    .as_ref()
                    .map(|h| h.totals(r.id, now))
                    .unwrap_or_default();
                Some(vdict! {
                    "pricing_changed_at" => self.game.pricing.iter().find(|p| p.route_id == r.id).and_then(|p| p.last_change.as_ref()).map_or(-1, |c| c.at as i64),
                    "pricing_autopilot" => self.game.pricing.iter().any(|p| p.route_id == r.id && p.next_review.is_some()),
                    "pricing_last" => self.game.pricing.iter().find(|p| p.route_id == r.id).and_then(|p| p.last_change.as_ref()).map(|c| format!("{} → {} EUR{}", c.old, c.new, c.new_business.zip(c.old_business).map(|(new, old)| format!(" · J {old} → {new} EUR")).unwrap_or_default())).unwrap_or_default().as_str(),
                    "id" => r.id, "origin_icao" => r.origin_icao.as_str(),
                    "destination_icao" => r.destination_icao.as_str(),
                    "distance_km" => routes::distance_km(&r.origin_icao, &r.destination_icao).unwrap_or(0.0),
                    "aircraft_id" => r.aircraft_id.unwrap_or(0), "aircraft_ids" => &aircraft_ids, "has_plan" => has_plan, "release_pending" => &pending_rows,
                    "can_wind_down" => services.iter().any(|s| s.remove_at.is_none() && (s.plan.is_some() || s.aircraft_id.and_then(|a| self.game.fleet.iter().find(|p| p.id == a)).and_then(|p| p.maintenance.as_ref()).and_then(|v| v.resume.as_ref()).is_some_and(|v| v.route_id == s.id))),
                    "can_delete" => !has_plan && pending.is_empty(),
                    "delete_available_at_text" => pending.iter().map(|(_, at)| *at).max().map(operations::display_time).unwrap_or_default().as_str(),
                    "delete_available_at" => pending.iter().map(|(_, at)| *at as i64).max().unwrap_or(-1),
                    "ticket_eur" => r.ticket_eur, "business_ticket_eur" => r.business_ticket_eur,
                    "freighter_only" => freighter_only,
                    "freight_load_percent" => if freight_capacity > 0 { freight_kg as f64 * 100.0 / freight_capacity as f64 } else { 0.0 },
                    "estimated_cargo_kg" => freight_kg / active_count.max(1),
                    "cargo_daily_demand_kg" => (cargo::daily_demand_for(r, freighter_only).unwrap_or(0.0) * world_economy::factor(index, world_economy::RULES.cargo_reaction)).floor() as i64,
                    "cargo_rate_cents" => r.cargo_rate_cents, "cargo_max_rate_cents" => cargo::RULES.maximum_rate_cents,
                    "actual_cargo_kg" => actual.cargo_kg, "actual_cargo_revenue_eur" => actual.cargo_revenue,
                    "service_level" => r.service_level.name(),
                    "service_economy_eur" => r.service_level.rate().economy_eur,
                    "service_business_eur" => r.service_level.rate().business_eur,
                    "estimated_service_eur" => service_cost / active_count.max(1),
                    "estimated_airport_eur" => airport_cost / active_count.max(1),
                    "actual_service_eur" => actual.service,
                    "actual_ancillary_revenue_eur" => actual.ancillary,
                    "punctuality_percent" => disruptions::percent(&actual), "punctuality_flights" => actual.punctuality_flights,
                    "economy_business_factor" => world_economy::factor(index, world_economy::RULES.business_reaction),
                    "economy_leisure_factor" => world_economy::factor(index, world_economy::RULES.leisure_reaction),
                    "economy_cargo_factor" => world_economy::factor(index, world_economy::RULES.cargo_reaction),
                    "event_kinds" => &event_kinds, "event_places" => &event_places,
                    "event_business_factor" => route_event.map_or(1.0, |e| e.business),
                    "event_leisure_factor" => route_event.map_or(1.0, |e| e.leisure),
                    "event_cargo_factor" => route_event.map_or(1.0, |e| e.cargo),
                    "season_business_factor" => season.business,
                    "season_leisure_factor" => season.leisure,
                    "daily_demand" => market.total().floor() as i64,
                    "base_daily_market" => base.total().floor() as i64,
                    "daily_business" => market.business.floor() as i64,
                    "daily_leisure" => market.leisure.floor() as i64,
                    "estimated_passengers" => estimate.per_departure,
                    "connections" => &connections,
                    "expected_daily_local" => (estimate.daily_passengers - estimate.daily_connecting).floor() as i64,
                    "expected_daily_connecting" => estimate.daily_connecting.floor() as i64,
                    "transfer_fed" => estimate.daily_connecting > estimate.daily_passengers / 2.0,
                    "actual_connecting_passengers" => actual.connecting_passengers,
                    "expected_daily_passengers" => estimate.daily_passengers.floor() as i64,
                    "daily_spill" => estimate.daily_spill.floor() as i64,
                    "actual_flights" => actual.flights, "actual_load_percent" => actual.load_percent(),
                    "maintenance_status" => maintenance.as_str(),
                    "staff_status" => staff.as_str(),
                    "can_schedule" => !has_plan && !aircraft_ids.is_empty() && maintenance.is_empty() && staff.is_empty()
                })
            })
            .collect()
    }

    #[func]
    fn get_aircraft_models(&self) -> Array<VarDictionary> {
        let Ok(models) = aircraft::models() else {
            return Array::new();
        };
        let now = self.game.clock.elapsed().as_secs();
        models.iter().filter_map(|model| {
            let needs = staff::needs(&model.id).ok()?;
            Some(vdict! {
            "id" => model.id.as_str(), "name" => model.name.as_str(),
            "class" => model.class.as_str(), "cruise_kmh" => model.cruise_kmh,
            "pilots" => needs.pilots, "cabin_crew" => needs.cabin_crew,
            "freighter" => model.main_deck_cargo_kg > 0, "main_deck_cargo_kg" => model.main_deck_cargo_kg,
            "belly_cargo_kg" => model.belly_cargo_kg, "payload_kg" => model.payload_kg,
            "seats" => model.seats, "range_km" => model.range_km, "price_eur" => model.price_eur,
            "co2_g_per_km" => economy::co2_grams_per_km(model),
            "lease_deposit_eur" => trading::lease_terms(&model.id).map(|t| t.0).unwrap_or(0),
            "lease_monthly_eur" => trading::lease_terms(&model.id).map(|t| t.1).unwrap_or(0),
            "lease_period_days" => trading::RULES.lease_period_days as i64,
            "availability" => match model.availability(now) {
                aircraft::Availability::Upcoming { .. } => "upcoming",
                aircraft::Availability::OnSale => "on_sale",
                aircraft::Availability::OutOfProduction { .. } => "out_of_production",
            },
            "available_from" => format!("Tag {}", model.entry_day + 1).as_str(),
            "orderable" => model.orderable(now), "leasable" => model.leasable(now),
            "out_of_production_from" => model.end_of_production_day
                .map_or(String::new(), |d| format!("Tag {}", d + 1)).as_str()
        }) }).collect()
    }

    #[func]
    fn buy_aircraft(&mut self, model_id: GString) -> GString {
        match self.game.buy_aircraft(&model_id.to_string()) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn get_order_limit(&self) -> i64 {
        orders::RULES.max_open_orders as i64
    }

    #[func]
    fn quote_order(&self, model_id: GString, quantity: u32) -> VarDictionary {
        match self.game.quote_order(&model_id.to_string(), quantity) {
            Ok(q) => {
                let dates: PackedStringArray = q
                    .delivery_dates
                    .iter()
                    .map(|t| GString::from(operations::display_time(*t).as_str()))
                    .collect();
                let total_deposit = q.deposit_per_aircraft * i64::from(quantity);
                vdict! {
                    "error" => "", "unit_price" => q.unit_price,
                    "deposit_per_aircraft" => q.deposit_per_aircraft,
                    "total_deposit" => total_deposit, "balance" => q.balance,
                    "delivery_dates" => &dates,
                    "can_afford" => self.game.airline.as_ref().is_some_and(|a| a.capital_eur >= total_deposit)
                }
            }
            Err(error) => vdict! { "error" => error.as_str() },
        }
    }

    #[func]
    fn place_order(&mut self, model_id: GString, quantity: u32) -> GString {
        self.game
            .place_order(&model_id.to_string(), quantity)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn cancel_order(&mut self, order_id: i64) -> GString {
        self.game
            .cancel_order(order_id as u64)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_orders(&self) -> Array<VarDictionary> {
        self.game.orders.iter().map(|o| {
            vdict! {
                "id" => o.id as i64,
                "model" => aircraft::model(&o.model_id).map_or("?", |m| m.name.as_str()),
                "delivery_date" => operations::display_time(o.delivery_at).as_str(),
                "grace_deadline" => operations::display_time(o.delivery_at + orders::RULES.grace_days * 86400).as_str(),
                "balance" => o.unit_price - o.deposit, "deposit" => o.deposit,
                "status" => if o.held_since.is_some() { "ORDER_HELD" } else { "ORDER_SCHEDULED" }
            }
        }).collect()
    }

    #[func]
    fn get_upcoming_commitments(&self) -> Array<VarDictionary> {
        self.game
            .upcoming_commitments()
            .iter()
            .map(|c| {
                let date = operations::display_time(c.at);
                match &c.kind {
                    orders::CommitmentKind::Delivery { order_id, model } => vdict! {
                        "kind" => "delivery", "date" => date.as_str(), "at" => c.at as i64,
                        "title" => model.as_str(), "detail" => "", "id" => *order_id as i64
                    },
                    orders::CommitmentKind::Training { employee_id, name, rating } => vdict! {
                        "kind" => "training", "date" => date.as_str(), "at" => c.at as i64,
                        "title" => name.as_str(), "detail" => rating.as_str(), "id" => i64::from(*employee_id)
                    },
                }
            })
            .collect()
    }

    #[func]
    fn get_used_market(&self) -> Array<VarDictionary> {
        let mut rows = Array::new();
        for listing in &self.game.used_market.listings {
            let p = &listing.aircraft;
            let mut row = VarDictionary::new();
            row.set("id", listing.id as i64);
            row.set(
                "model",
                aircraft::model(&p.model_id).map_or("?", |m| m.name.as_str()),
            );
            row.set("seller", listing.seller.as_str());
            row.set(
                "age_days",
                ((self.game.clock.elapsed().as_secs() - p.acquired_at.unwrap_or(0)) / 86400) as i64,
            );
            row.set("flight_hours", p.wear.flight_seconds as f64 / 3600.0);
            row.set("cycles", p.wear.cycles as i64);
            row.set(
                "condition_percent",
                p.wear
                    .condition_permille(crate::maintenance::rules(&p.model_id).unwrap())
                    as f64
                    / 10.0,
            );
            row.set("price_eur", self.game.used_price(listing).unwrap_or(0));
            rows.push(&row);
        }
        rows
    }

    #[func]
    fn buy_used_aircraft(&mut self, listing_id: i64) -> GString {
        self.game
            .buy_used_aircraft(listing_id as u64)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn lease_aircraft(&mut self, model_id: GString) -> GString {
        self.game
            .lease_aircraft(&model_id.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn sell_aircraft(&mut self, aircraft_id: u32) -> GString {
        self.game
            .sell_aircraft(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn return_aircraft(&mut self, aircraft_id: u32) -> GString {
        self.game
            .return_aircraft(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_aircraft_cash_history(&self) -> Array<VarDictionary> {
        self.game
            .history
            .iter()
            .flat_map(|h| h.aircraft_cash.iter())
            .rev()
            .take(100)
            .map(|e| {
                vdict! {
                    "time" => operations::display_time(e.time).as_str(),
                    "cash_eur" => e.cash,
                    "kind" => match e.kind {
                        trading::Kind::Cabin => "Kabinenumbau",
                        trading::Kind::Ferry => "Überführung",
                        trading::Kind::Sale => "Verkauf",
                        trading::Kind::Deposit => "Leasingkaution",
                        trading::Kind::Rent => "Leasingrate",
                        trading::Kind::Return => "Rückgabe (Kaution abzüglich Zustandsgebühr)",
                        trading::Kind::CharterFee => "Charter-Erlös",
                        trading::Kind::CharterPenalty => "Charter-Vertragsstrafe"
                    }
                }
            })
            .collect()
    }

    #[func]
    fn set_cabin_layout(&mut self, aircraft_id: u32, business_seats: u32) -> GString {
        self.game
            .set_cabin_layout(aircraft_id, business_seats)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn set_cargo_rate(&mut self, route_id: u32, cents: i64) -> GString {
        self.game
            .set_cargo_rate(route_id, cents)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_loyalty(&self) -> VarDictionary {
        let l = &self.game.loyalty;
        let options: Array<VarDictionary> = [loyalty::Level::None, loyalty::Level::Basic, loyalty::Level::Generous]
            .into_iter().map(|level| {
                let rate = level.rate();
                vdict! { "level" => level.name(), "economy_eur" => rate.economy_eur,
                    "business_eur" => rate.business_eur, "enrolment_percent" => rate.enrolment_bps as f64 / 100.0 }
            }).collect();
        vdict! { "level" => l.level.name(), "members" => l.members, "trend" => l.trend,
        "bonus_percent" => loyalty::bonus(l.members) * 100.0,
        "max_bonus_percent" => loyalty::RULES.max_bonus_bps as f64 / 100.0,
        "saturation_members" => loyalty::RULES.saturation_members,
        "lapse_percent" => loyalty::RULES.daily_lapse_bps as f64 / 100.0,
        "options" => &options, "can_change" => self.game.require_active().is_ok() }
    }

    #[func]
    fn set_loyalty_level(&mut self, level: GString) -> GString {
        loyalty::Level::parse(&level.to_string())
            .and_then(|level| self.game.set_loyalty_level(level))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_ancillary_policy(&self) -> VarDictionary {
        let p = self.game.ancillary_policy;
        let options: Array<VarDictionary> = [
            ancillaries::Policy::Included,
            ancillaries::Policy::Standard,
            ancillaries::Policy::Aggressive,
        ]
        .into_iter()
        .map(|p| {
            vdict! {
                "policy" => p.name(), "economy_eur" => p.rate().economy_eur,
                "business_eur" => p.rate().business_eur,
                "attractiveness_percent" => p.rate().attractiveness * 100.0
            }
        })
        .collect();
        vdict! { "policy" => p.name(), "options" => &options, "can_change" => self.game.require_active().is_ok() }
    }

    #[func]
    fn set_ancillary_policy(&mut self, policy: GString) -> GString {
        ancillaries::Policy::parse(&policy.to_string())
            .and_then(|p| self.game.set_ancillary_policy(p))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn set_service_level(&mut self, route_id: u32, level: GString) -> GString {
        service::Level::parse(&level.to_string())
            .and_then(|level| self.game.set_service_level(route_id, level))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn set_business_ticket_price(&mut self, route_id: u32, price: i64) -> GString {
        self.game
            .set_business_ticket_price(route_id, price)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn preview_ferry(&self, aircraft_id: u32, destination: GString) -> VarDictionary {
        match self.game.quote_ferry(aircraft_id, &destination.to_string()) {
            Ok(f) => {
                vdict! { "error" => "", "distance_km" => routes::distance_km(&f.quote.origin, &f.quote.destination).unwrap_or(0.0), "flight_seconds" => (f.quote.landing - f.departure) as i64, "cost_eur" => f.quote.totals.costs(), "fuel_eur" => f.quote.totals.fuel, "carbon_eur" => f.quote.totals.carbon, "airport_eur" => f.quote.totals.airport, "overhead_eur" => f.quote.totals.overhead }
            }
            Err(error) => vdict! { "error" => error.as_str() },
        }
    }

    #[func]
    fn send_ferry(&mut self, aircraft_id: u32, destination: GString) -> GString {
        self.game
            .send_ferry(aircraft_id, &destination.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_charter_offers(&self) -> Array<VarDictionary> {
        let idle: Vec<u32> = self
            .game
            .fleet
            .iter()
            .filter(|p| {
                p.status == aircraft::AircraftStatus::Parked
                    && self.game.require_ferry_aircraft(p.id).is_ok()
            })
            .map(|p| p.id)
            .collect();
        self.game
            .open_charters()
            .iter()
            .map(|o| {
                let fits = idle
                    .iter()
                    .any(|&id| self.game.quote_charter_offer(o, id).is_ok());
                vdict! {
                    "id" => o.id as i64,
                    "kind" => o.kind.as_str(),
                    "kind_name" => o.kind_name(),
                    "freight" => o.freight(),
                    "origin" => o.origin.as_str(),
                    "destination" => o.destination.as_str(),
                    "distance_km" => routes::distance_km(&o.origin, &o.destination).unwrap_or(0.0),
                    "quantity" => i64::from(o.quantity),
                    "fee_eur" => o.fee_eur,
                    "penalty_eur" => o.penalty_eur(),
                    "window_start" => operations::display_time(o.window_start).as_str(),
                    "window_end" => operations::display_time(o.window_end).as_str(),
                    "expires" => operations::display_time(o.expires).as_str(),
                    "fits_idle_aircraft" => fits,
                }
            })
            .collect()
    }

    #[func]
    fn get_charter_jobs(&self) -> Array<VarDictionary> {
        let now = self.game.clock.elapsed().as_secs();
        let mut rows: Array<VarDictionary> = Array::new();
        for j in &self.game.charters.jobs {
            let (state, text) = if j.departure.is_some() {
                ("airborne", "Im Flug")
            } else if self.game.ferry(j.aircraft_id).is_some_and(|f| !f.settled) {
                ("positioning", "Überführung")
            } else {
                ("scheduled", "Geplant")
            };
            rows.push(&vdict! {
                "id" => j.offer.id as i64,
                "state" => state,
                "state_text" => text,
                "kind_name" => j.offer.kind_name(),
                "route" => j.offer.label().as_str(),
                "aircraft_id" => j.aircraft_id,
                "aircraft" => format!("AM-{:06}", j.aircraft_id).as_str(),
                "fee_eur" => j.offer.fee_eur,
                "penalty_eur" => j.offer.penalty_eur(),
                "departure" => operations::display_time(j.dispatch_at.max(j.departure.unwrap_or(0))).as_str(),
                "deadline" => operations::display_time(j.offer.window_end).as_str(),
                "cancellable" => j.departure.is_none(),
                "overdue" => j.dispatch_at < now && j.departure.is_none(),
            });
        }
        for o in self.game.charters.recent.iter().rev() {
            rows.push(&vdict! {
                "id" => o.offer_id as i64,
                "state" => if o.delivered { "delivered" } else { "failed" },
                "state_text" => if o.delivered { "Geliefert" } else { "Gescheitert" },
                "kind_name" => "",
                "route" => o.label.as_str(),
                "aircraft_id" => o.aircraft_id,
                "aircraft" => format!("AM-{:06}", o.aircraft_id).as_str(),
                "fee_eur" => o.cash_eur,
                "penalty_eur" => 0,
                "departure" => operations::display_time(o.at).as_str(),
                "deadline" => o.reason.as_str(),
                "cancellable" => false,
                "overdue" => false,
            });
        }
        rows
    }

    #[func]
    fn get_charter_aircraft_options(&self, offer_id: i64) -> Array<VarDictionary> {
        let offer = self
            .game
            .open_charters()
            .into_iter()
            .find(|o| o.id as i64 == offer_id);
        self.game
            .fleet
            .iter()
            .map(|p| {
                let name = aircraft::model(&p.model_id).map_or("", |m| m.name.as_str());
                let verdict = match &offer {
                    Some(o) => self.game.quote_charter_offer(o, p.id).map(|_| ()),
                    None => Err("Das Angebot ist nicht mehr verfügbar.".to_string()),
                };
                vdict! {
                    "aircraft_id" => p.id,
                    "label" => format!("{} · {} · {}", p.identifier(), name, p.location_icao).as_str(),
                    "ok" => verdict.is_ok(),
                    "reason" => verdict.err().unwrap_or_default().as_str(),
                }
            })
            .collect()
    }

    #[func]
    fn preview_charter(&self, offer_id: i64, aircraft_id: u32) -> VarDictionary {
        let Some(offer) = self
            .game
            .open_charters()
            .into_iter()
            .find(|o| o.id as i64 == offer_id)
        else {
            return vdict! { "error" => "Das Angebot ist nicht mehr verfügbar." };
        };
        match self.game.quote_charter_offer(&offer, aircraft_id) {
            Ok(q) => {
                let (pos_cost, pos_fuel, pos_airport, pos_overhead, pos_km) =
                    q.positioning.as_ref().map_or((0, 0, 0, 0, 0.0), |p| {
                        let t = &p.quote.totals;
                        (
                            t.costs(),
                            t.fuel,
                            t.airport,
                            t.overhead,
                            routes::distance_km(&p.quote.origin, &p.quote.destination)
                                .unwrap_or(0.0),
                        )
                    });
                let t = &q.leg.quote.totals;
                vdict! {
                    "error" => "",
                    "has_positioning" => q.positioning.is_some(),
                    "positioning_from" => self.game.fleet.iter().find(|p| p.id == aircraft_id).map_or("", |p| p.location_icao.as_str()),
                    "positioning_km" => pos_km,
                    "positioning_eur" => pos_cost,
                    "positioning_fuel_eur" => pos_fuel,
                    "positioning_airport_eur" => pos_airport,
                    "positioning_overhead_eur" => pos_overhead,
                    "leg_km" => routes::distance_km(&offer.origin, &offer.destination).unwrap_or(0.0),
                    "leg_eur" => t.costs(),
                    "leg_fuel_eur" => t.fuel,
                    "leg_airport_eur" => t.airport,
                    "leg_overhead_eur" => t.overhead,
                    "cost_eur" => q.cost_eur(),
                    "fee_eur" => offer.fee_eur,
                    "net_eur" => offer.fee_eur - q.cost_eur(),
                    "penalty_eur" => offer.penalty_eur(),
                    "departure" => operations::display_time(q.dispatch_at).as_str(),
                    "landing" => operations::display_time(q.leg.quote.landing).as_str(),
                    "deadline" => operations::display_time(offer.window_end).as_str(),
                    "route" => offer.label().as_str(),
                }
            }
            Err(error) => vdict! { "error" => error.as_str() },
        }
    }

    #[func]
    fn accept_charter(&mut self, offer_id: i64, aircraft_id: u32) -> GString {
        self.game
            .accept_charter(offer_id as u64, aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn cancel_charter(&mut self, offer_id: i64) -> GString {
        self.game
            .cancel_charter(offer_id as u64)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_ferry_archive(&self, offset: i64) -> Array<VarDictionary> {
        self.game
            .history
            .iter()
            .flat_map(|h| h.ferries.iter())
            .rev()
            .skip(offset.max(0) as usize)
            .take(100)
            .map(|f| {
                let mut row = flight_dictionary(&f.quote);
                row.set("aircraft_id", f.aircraft_id);
                row
            })
            .collect()
    }

    #[func]
    fn get_fleet(&self) -> Array<VarDictionary> {
        let airports = airports::catalog().ok();
        self.game
            .fleet
            .iter()
            .map(|plane| {
                let name = aircraft::model(&plane.model_id)
                    .map(|model| model.name.as_str())
                    .unwrap_or("");
                let location_name = airports
                    .and_then(|catalog| {
                        catalog
                            .airports
                            .iter()
                            .find(|airport| airport.icao == plane.location_icao)
                    })
                    .map(|airport| airport.name.as_str())
                    .unwrap_or("");
                let movement = self.game.planned_route(plane.id).and_then(|i| operations::movement(&self.game.routes[i], &plane.model_id, self.game.clock.elapsed().as_secs()).ok());
                vdict! {
                    "ferry_state" => self.game.ferry(plane.id).map_or("", |f| if f.settled { "Bodenzeit" } else { "Überführung" }),
                    "ferry_error" => self.game.require_ferry_aircraft(plane.id).err().unwrap_or_default().as_str(),
                    "delay_seconds" => movement.as_ref().map_or(0, |s| s.delay_seconds) as i64,
                    "weather_seconds" => movement.as_ref().map_or(0, |s| s.weather_seconds) as i64,
                    "id" => plane.id, "identifier" => plane.identifier().as_str(),
                    "model_id" => plane.model_id.as_str(), "model_name" => name,
                    "freighter" => aircraft::model(&plane.model_id).is_ok_and(|m| m.main_deck_cargo_kg > 0),
                    "economy_seats" => cabins::layout(plane).map_or(0, |c| c.0),
                    "business_seats" => plane.business_seats,
                    "max_business_seats" => aircraft::model(&plane.model_id).map_or(0, |m| m.seats / cabins::RULES.business_space_ratio),
                    "cabin_space_ratio" => cabins::RULES.business_space_ratio,
                    "cabin_fee_eur" => cabins::RULES.layout_fee_eur,
                    "can_edit_cabin" => aircraft::model(&plane.model_id).is_ok_and(|m| m.seats > 0) && matches!(plane.status, aircraft::AircraftStatus::Parked | aircraft::AircraftStatus::Ground) && plane.maintenance.is_none(),
                    "location_icao" => plane.location_icao.as_str(),
                    "location_name" => location_name,
                    "status" => match plane.status { aircraft::AircraftStatus::Parked => "parked", aircraft::AircraftStatus::InFlight => "in_flight", aircraft::AircraftStatus::Ground => "ground", aircraft::AircraftStatus::Maintenance => "maintenance" },
                    "purchase_price_eur" => if plane.lease.is_none() { plane.purchase_price_eur } else { 0 },
                    "ownership" => if plane.lease.is_some() { "Leasing" } else { "Eigentum" },
                    "leased" => plane.lease.is_some(),
                    "acquired" => operations::display_time(plane.acquired_at.unwrap_or(0)).as_str(),
                    "disposal_quote_eur" => self.game.disposal_quote(plane.id).unwrap_or(0),
                    "disposal_error" => self.game.disposal_quote(plane.id).err().unwrap_or_default().as_str(),
                    "lease_deposit_eur" => plane.lease.as_ref().map_or(0, |l| l.deposit),
                    "lease_monthly_eur" => plane.lease.as_ref().map_or(0, |l| l.monthly),
                    "lease_due" => plane.lease.as_ref().map(|l| operations::display_time(l.next_due)).unwrap_or_default().as_str(),
                    "range_km" => aircraft::model(&plane.model_id).map(|m| m.range_km).unwrap_or(0),
                    "route_id" => self.game.routes.iter().find(|r| r.aircraft_id == Some(plane.id)).map(|r| r.public_id()).unwrap_or(0)
                }
            })
            .zip(self.game.fleet.iter().enumerate())
            .map(|(mut row, (p, plane))| {
                row.extend_dictionary(&self.maintenance_info(p, plane), true);
                row
            })
            .collect()
    }

    #[func]
    fn set_maintenance_base(&mut self, icao: GString, open: bool) -> GString {
        let result = if open {
            self.game.open_maintenance_base(&icao.to_string())
        } else {
            self.game.close_maintenance_base(&icao.to_string())
        };
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn get_maintenance_airports(&self) -> Array<VarDictionary> {
        let Ok(catalog) = airports::catalog() else {
            return Array::new();
        };
        let (setup, fee) = self.game.base_prices().unwrap_or_default();
        catalog.airports.iter().map(|a| {
            let base = self.game.is_maintenance_base(&a.icao);
            let home = self.game.airline.as_ref().is_some_and(|airline| airline.home_airport_icao == a.icao);
            let error = self.game.validate_base_action(&a.icao, !base).err().unwrap_or_default();
            vdict! {
                "icao" => a.icao.as_str(), "name" => a.name.as_str(),
                "is_base" => base, "is_home" => home,
                "setup_eur" => setup, "fee_eur" => fee,
                "next_fee" => self.game.maintenance_bases.iter().find(|b| b.icao == a.icao).map(|b| operations::display_time(b.next_fee)).unwrap_or_default().as_str(),
                "action_error" => error.as_str()
            }
        }).collect()
    }

    /// Empty string means success. Airborne aircraft enter the hangar after landing.
    #[func]
    fn request_maintenance(&mut self, aircraft_id: u32) -> GString {
        self.game
            .request_maintenance(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Empty string means success. Only orders that have not started yet.
    #[func]
    fn cancel_maintenance(&mut self, aircraft_id: u32) -> GString {
        self.game
            .cancel_maintenance(aircraft_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Every employee with role, salary and derived employment status.
    #[func]
    fn get_staff(&self) -> Array<VarDictionary> {
        let now = self.game.clock.elapsed().as_secs();
        let activity = self.game.staff_activity();
        self.game
            .staff
            .employees
            .iter()
            .zip(activity)
            .map(|(e, a)| {
                vdict! {
                    "id" => e.id, "name" => e.name.as_str(),
                    "role" => e.role.key(), "role_name" => e.role.label(),
                    "salary_eur" => e.salary_eur,
                    "status" => match a { staff::Activity::Available => "available", staff::Activity::Flying => "flying", staff::Activity::Hangar => "hangar", staff::Activity::Training => "training" },
                    "status_text" => e.training.as_ref().map(|t| format!("Umschulung ({}) → {} bis {}", if t.external { staff::Course::External } else { staff::Course::InHouse }.label(), t.rating, operations::display_time(t.ends_at))).unwrap_or_else(|| a.label().into()).as_str(),
                    "rating" => e.rating.as_deref().unwrap_or("—"),
                    "training_progress" => e.training.as_ref().map_or(0.0, |t| t.progress_percent(now)),
                    "can_dismiss" => a == staff::Activity::Available,
                    "hired" => operations::display_time(e.hired_at).as_str(),
                    "days_employed" => (now.saturating_sub(e.hired_at) / 86_400) as i64
                }
            })
            .collect()
    }

    /// Per-role headcount, shortages and what blocks each aircraft.
    #[func]
    fn get_staff_overview(&self) -> VarDictionary {
        let g = &self.game;
        let (_, idle_cabin) = self.idle_crew_demand();
        let waiting_techs: u32 = g
            .fleet
            .iter()
            .filter(|p| {
                p.status == aircraft::AircraftStatus::Maintenance
                    && p.maintenance.as_ref().is_some_and(|v| !v.staffed)
            })
            .filter_map(|p| staff::needs(&p.model_id).ok())
            .map(|n| n.technicians)
            .sum();
        let mut roles = Array::<VarDictionary>::new();
        let mut total_missing = 0i64;
        for role in staff::Role::ALL {
            let employed = g.staff_count(role);
            let available = g.available_staff(role);
            let missing = match role {
                staff::Role::Pilot => staff::ratings()
                    .iter()
                    .map(|r| g.idle_pilot_shortage(r))
                    .sum(),
                staff::Role::CabinCrew => {
                    (idle_cabin + g.reserved_crew().1).saturating_sub(employed)
                }
                staff::Role::Technician => g
                    .technician_target()
                    .saturating_sub(employed)
                    .max(waiting_techs.saturating_sub(available)),
            };
            total_missing += i64::from(missing);
            let labour = g.labour_group(role);
            roles.push(&vdict! {
                "role" => role.key(), "name" => role.plural(), "singular" => role.label(),
                "employed" => i64::from(employed), "busy" => i64::from(g.busy_staff(role)),
                "available" => i64::from(available), "missing" => i64::from(missing),
                "pay_percent" => i64::from(labour.pay_percent), "pending_percent" => i64::from(labour.pending_percent),
                "morale" => i64::from(labour.morale),
                "labour_state" => if labour.strike_until.is_some() { "strike" } else if labour.warning_at.is_some() { "warning" } else if labour.morale < labour::rules().calm_threshold { "unrest" } else { "calm" },
                "labour_date" => labour.strike_until.or(labour.warning_at).map(operations::display_time).unwrap_or_default().as_str(),
                "pay_target" => i64::from(labour::rules().target(labour.pay_percent)),
                "understaffed" => labour.understaffed, "missed_payroll" => labour.missed_payroll, "restructuring" => labour.restructuring,
                "resigned" => i64::from(labour.resigned),
                "salary_eur" => g.pay_salary(role),
                "monthly_cost_eur" => g.staff.employees.iter().filter(|e| e.role == role).map(|e| e.salary_eur).sum::<i64>()
            });
        }
        let next_pay = g
            .next_payroll_event()
            .map(operations::display_time)
            .unwrap_or_else(|| "—".to_string());
        let gaps = g.crew_gaps();
        let mut aircraft_rows = Array::<VarDictionary>::new();
        for p in &g.fleet {
            let Ok(n) = staff::needs(&p.model_id) else {
                continue;
            };
            let reserved = g.planned_route(p.id).is_some()
                || p.maintenance.as_ref().is_some_and(|v| v.resume.is_some());
            let gap = gaps.iter().find(|x| x.0 == p.id);
            let (state, problem) = match &p.maintenance {
                Some(v) if p.status == aircraft::AircraftStatus::Maintenance && !v.staffed => (
                    "waiting_technicians",
                    format!(
                        "Wartung wartet auf {} Techniker (frei: {})",
                        n.technicians,
                        g.available_staff(staff::Role::Technician)
                    ),
                ),
                Some(_) if p.status == aircraft::AircraftStatus::Maintenance => {
                    ("hangar", String::new())
                }
                _ if let Some(&(_, lack_pilots, lack_cabin)) = gap => {
                    let mut parts = Vec::new();
                    if lack_pilots > 0 {
                        parts.push(format!("{lack_pilots} Piloten ({})", n.rating));
                    }
                    if lack_cabin > 0 {
                        parts.push(format!("{lack_cabin} Flugbegleiter"));
                    }
                    (
                        "short",
                        format!(
                            "Besatzung fehlt: {} · Flugplan angehalten",
                            parts.join(" und ")
                        ),
                    )
                }
                _ if reserved => ("crewed", String::new()),
                _ => match g.require_crew(&p.model_id) {
                    Ok(()) => ("ready", String::new()),
                    Err(e) => ("short", e),
                },
            };
            aircraft_rows.push(&vdict! {
                "id" => p.id, "identifier" => p.identifier().as_str(),
                "model_name" => aircraft::model(&p.model_id).map(|m| m.name.as_str()).unwrap_or(""),
                "pilots" => i64::from(n.pilots), "cabin_crew" => i64::from(n.cabin_crew),
                "technicians" => i64::from(n.technicians),
                "state" => state, "problem" => problem.as_str(),
                "rating" => n.rating.as_str(),
                "missing_pilots" => gap.map_or(0, |x| i64::from(x.1)),
                "missing_cabin" => gap.map_or(0, |x| i64::from(x.2))
            });
        }
        let mut overview = vdict! {
            "employees" => g.staff.employees.len() as i64,
            "monthly_payroll_eur" => g.monthly_payroll(),
            "next_payroll" => next_pay.as_str(),
            "missing_total" => total_missing,
            "payroll_paid_eur" => g.history.as_ref().map(|h| h.payroll_through(u64::MAX).min(i128::from(i64::MAX)) as i64).unwrap_or(0),
            "pay_min" => i64::from(labour::rules().minimum_pay_percent), "pay_max" => i64::from(labour::rules().maximum_pay_percent),
            "pay_market" => i64::from(labour::rules().market_pay_percent),
            "max_batch" => i64::from(staff::max_hire_batch())
        };
        let mut types = Array::<VarDictionary>::new();
        for rating in staff::ratings() {
            types.push(&vdict! { "rating" => rating.as_str(), "rated" => i64::from(g.rated_count(&rating)), "busy" => i64::from(g.rated_busy(&rating)), "available" => i64::from(g.rated_available(&rating)) });
        }
        overview.set("ratings", &types);
        let (cost, days) = staff::conversion_terms();
        overview.set("conversion_cost_eur", cost);
        overview.set("conversion_days", days as i64);
        let capital = g.airline.as_ref().map_or(0, |a| a.capital_eur);
        let mut courses = Array::<VarDictionary>::new();
        for course in [staff::Course::InHouse, staff::Course::External] {
            let (cost, days) = course.terms();
            courses.push(&vdict! {
                "course" => course.key(), "label" => course.label(),
                "cost_eur" => cost, "days" => days as i64,
                "ends_text" => operations::display_time(g.conversion_ends_at(course)).as_str(),
                "affordable" => capital >= cost
            });
        }
        overview.set("conversion_courses", &courses);
        overview.set("roles", &roles);
        overview.set("aircraft", &aircraft_rows);
        overview
    }

    #[func]
    fn hire_pilots(&mut self, rating: GString, count: u32) -> GString {
        self.game
            .hire_pilots(&rating.to_string(), count)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn train_pilot_course(&mut self, id: u32, rating: GString, course: GString) -> GString {
        staff::Course::parse(&course.to_string())
            .and_then(|c| self.game.train_pilot_course(id, &rating.to_string(), c))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn train_pilot(&mut self, id: u32, rating: GString) -> GString {
        self.game
            .train_pilot(id, &rating.to_string())
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Empty string means success.
    #[func]
    fn hire_staff(&mut self, role: GString, count: i64) -> GString {
        let result = staff::Role::parse(&role.to_string()).and_then(|role| {
            u32::try_from(count)
                .map_err(|_| "Ungültige Anzahl.".to_string())
                .and_then(|n| self.game.hire_staff(role, n))
        });
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn set_staff_pay(&mut self, role: GString, percent: u32) -> GString {
        staff::Role::parse(&role.to_string())
            .and_then(|r| self.game.set_pay_level(r, percent))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn preview_staff_pay(&self, role: GString, percent: u32) -> VarDictionary {
        let Ok(r) = staff::Role::parse(&role.to_string()) else {
            return VarDictionary::new();
        };
        let Ok(cost) = self.game.pay_preview(r, percent) else {
            return VarDictionary::new();
        };
        let current: i64 = self
            .game
            .staff
            .employees
            .iter()
            .filter(|e| e.role == r)
            .map(|e| e.salary_eur)
            .sum();
        vdict! {"cost" => cost, "delta" => cost-current, "percent" => i64::from(percent)}
    }

    /// Hire exactly the staff missing for aircraft that cannot fly yet.
    #[func]
    fn hire_missing_staff(&mut self) -> GString {
        let g = &self.game;
        let (_, cabin) = self.idle_crew_demand();
        let techs = g
            .technician_target()
            .saturating_sub(g.staff_count(staff::Role::Technician))
            .max(
                g.fleet
                    .iter()
                    .filter(|p| {
                        p.status == aircraft::AircraftStatus::Maintenance
                            && p.maintenance.as_ref().is_some_and(|v| !v.staffed)
                    })
                    .filter_map(|p| staff::needs(&p.model_id).ok())
                    .map(|n| n.technicians)
                    .sum::<u32>()
                    .saturating_sub(g.available_staff(staff::Role::Technician)),
            );
        let cabin =
            (cabin + g.reserved_crew().1).saturating_sub(g.staff_count(staff::Role::CabinCrew));
        self.game
            .hire_missing_rated_staff(cabin, techs)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    /// Empty string means success. Only available (not assigned) staff.
    #[func]
    fn dismiss_staff(&mut self, employee_id: u32) -> GString {
        self.game
            .dismiss_staff(employee_id)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn get_countries(&self) -> Array<VarDictionary> {
        let Ok(catalog) = airports::catalog() else {
            return Array::new();
        };
        catalog
            .countries
            .iter()
            .map(|country| {
                vdict! {
                    "code" => country.code.as_str(), "name" => country.name.as_str()
                }
            })
            .collect()
    }

    #[func]
    fn get_airports(&self, country_code: GString) -> Array<VarDictionary> {
        let Ok(catalog) = airports::catalog() else {
            return Array::new();
        };
        catalog
            .airports
            .iter()
            .filter(|a| a.country_code == country_code.to_string())
            .map(|airport| {
                vdict! {
                    "icao" => airport.icao.as_str(),
                    "name" => airport.name.as_str(),
                    "iata" => airport.iata.as_deref().unwrap_or(""),
                    "maintenance_base" => self.game.is_maintenance_base(&airport.icao),
                    "slots" => self.game.slot_text(&airport.icao).as_str(),
                    "charge_percent" => airport.charge_percent
                }
            })
            .collect()
    }

    #[func]
    fn get_starting_capital(&self) -> GString {
        match game::starting_capital() {
            Ok(amount) => format!("{amount} EUR").as_str().into(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn get_difficulty_options(&self) -> GString {
        serde_json::json!({
            "levels": ["low", "standard", "high"],
            "presets": difficulty::RULES.presets,
            "capital_eur": difficulty::RULES.capital_eur,
        })
        .to_string()
        .as_str()
        .into()
    }

    #[func]
    fn found_airline_with_levels(
        &mut self,
        name: GString,
        country: GString,
        airport: GString,
        capital: GString,
        competition: GString,
        crises: GString,
    ) -> GString {
        match self.game.found_with_levels(
            &name.to_string(),
            &country.to_string(),
            &airport.to_string(),
            &capital.to_string(),
            &competition.to_string(),
            &crises.to_string(),
        ) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    /// Checks a New Game form with the founding rules without touching the current game.
    #[func]
    fn validate_new_world(
        &self,
        name: GString,
        country: GString,
        airport: GString,
        capital: GString,
        competition: GString,
        crises: GString,
    ) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        match Game::default().found_with_levels(
            &name.to_string(),
            &country.to_string(),
            &airport.to_string(),
            &capital.to_string(),
            &competition.to_string(),
            &crises.to_string(),
        ) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    /// Founds a separate world; the current game is replaced only after the new one is valid.
    #[func]
    fn start_new_world(
        &mut self,
        name: GString,
        country: GString,
        airport: GString,
        capital: GString,
        competition: GString,
        crises: GString,
    ) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let mut staged = Game::default();
        match staged.found_with_levels(
            &name.to_string(),
            &country.to_string(),
            &airport.to_string(),
            &capital.to_string(),
            &competition.to_string(),
            &crises.to_string(),
        ) {
            Ok(()) => {
                self.game = staged;
                GString::new()
            }
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn has_airline(&self) -> bool {
        self.game.airline.is_some()
    }

    #[func]
    fn found_airline(&mut self, name: GString, country: GString, airport: GString) -> GString {
        match self.game.found(
            &name.to_string(),
            &country.to_string(),
            &airport.to_string(),
        ) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn get_airline_info(&self) -> VarDictionary {
        let Some(airline) = &self.game.airline else {
            return VarDictionary::new();
        };
        let Ok(catalog) = airports::catalog() else {
            return VarDictionary::new();
        };
        let country = catalog
            .countries
            .iter()
            .find(|c| c.code == airline.country_code);
        let airport = catalog
            .airports
            .iter()
            .find(|a| a.icao == airline.home_airport_icao);
        vdict! {
            "name" => airline.name.as_str(),
            "country_code" => airline.country_code.as_str(),
            "country_name" => country.map(|c| c.name.as_str()).unwrap_or(""),
            "airport_icao" => airline.home_airport_icao.as_str(),
            "airport_name" => airport.map(|a| a.name.as_str()).unwrap_or(""),
            "difficulty_capital" => self.game.difficulty.capital.key(),
            "difficulty_competition" => self.game.difficulty.competition.key(),
            "difficulty_crises" => self.game.difficulty.crises.key(),
            "capital_eur" => airline.capital_eur,
            "insolvent" => self.game.insolvent()
        }
    }

    /// Header-only slot list; legacy import is performed once on first access.
    #[func]
    fn get_save_slots(&self, user_dir: GString) -> VarDictionary {
        if self.jump_worker.is_some() {
            return vdict! {"error" => "Zeitsprung läuft."};
        }
        let user = std::path::PathBuf::from(user_dir.to_string());
        match save_slots::list(&user.join("saves"), &user.join("savegame.json")) {
            Ok(slots) => {
                let header = |h: Option<save_slots::Header>| -> VarDictionary {
                    h.map(|h| {
                        let mut d = vdict! {"name" => h.name.as_str(), "game_time" => h.game_time.as_str(),
                            "cash" => h.cash, "fleet" => h.fleet as i64, "written_at" => h.written_at as i64,
                            "format_version" => h.format_version as i64, "save_version" => h.save_version as i64};
                        if let (Some(airline), Some(icao)) = (&h.airline_name, &h.home_airport_icao) {
                            let airport_name = airports::catalog().ok().and_then(|c| {
                                c.airports.iter().find(|a| a.icao == *icao).map(|a| a.name.clone())
                            });
                            d.set("airline_name", airline.as_str());
                            d.set("home_airport_icao", icao.as_str());
                            d.set("home_airport_name", airport_name.unwrap_or_default().as_str());
                        }
                        d
                    }).unwrap_or_default()
                };
                let rows: Array<VarDictionary> = slots.into_iter().map(|s| {
                    vdict! {"id" => s.id as i64, "occupied" => s.occupied, "header" => &header(s.header),
                        "backup" => &header(s.backup), "error" => s.error.as_str()}
                }).collect();
                let autosave = save_slots::autosave_entry(&user.join("saves"));
                let autosave = vdict! {"occupied" => autosave.occupied, "header" => &header(autosave.header),
                "backup" => &header(autosave.backup), "error" => autosave.error.as_str()};
                vdict! {"slots" => &rows, "autosave" => &autosave, "name_length" => save_slots::RULES.name_length as i64, "error" => ""}
            }
            Err(e) => vdict! {"error" => e.as_str()},
        }
    }

    /// Periodic autosave into its dedicated entry; the previous autosave survives failures.
    #[func]
    fn autosave(&self, user_dir: GString) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let root = std::path::PathBuf::from(user_dir.to_string()).join("saves");
        save_slots::store_autosave(&root, &self.game)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }

    #[func]
    fn load_autosave(&mut self, user_dir: GString, backup: bool) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let root = std::path::PathBuf::from(user_dir.to_string()).join("saves");
        match save_slots::load_autosave(&root, backup) {
            Ok(game) => {
                self.game = game;
                GString::new()
            }
            Err(e) => e.as_str().into(),
        }
    }

    #[func]
    fn save_slot(&self, user_dir: GString, id: i64, name: GString, overwrite: bool) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let result = u32::try_from(id)
            .map_err(|_| "Ungültiger Speicherplatz.".to_string())
            .and_then(|id| {
                save_slots::store(
                    &std::path::PathBuf::from(user_dir.to_string()).join("saves"),
                    id,
                    &name.to_string(),
                    &self.game,
                    overwrite,
                )
            });
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn load_slot(&mut self, user_dir: GString, id: i64, backup: bool) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let result = u32::try_from(id)
            .map_err(|_| "Ungültiger Speicherplatz.".to_string())
            .and_then(|id| {
                save_slots::load(
                    &std::path::PathBuf::from(user_dir.to_string()).join("saves"),
                    id,
                    backup,
                )
            });
        match result {
            Ok(game) => {
                self.game = game;
                GString::new()
            }
            Err(e) => e.as_str().into(),
        }
    }

    #[func]
    fn delete_save_slot(&self, user_dir: GString, id: i64) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        let result = u32::try_from(id)
            .map_err(|_| "Ungültiger Speicherplatz.".to_string())
            .and_then(|id| {
                save_slots::delete(
                    &std::path::PathBuf::from(user_dir.to_string()).join("saves"),
                    id,
                )
            });
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn save_game(&self, path: GString) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        match save::save(std::path::Path::new(&path.to_string()), &self.game) {
            Ok(()) => GString::new(),
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn load_game(&mut self, path: GString) -> GString {
        if self.jump_worker.is_some() {
            return "Zeitsprung läuft.".into();
        }
        match save::load(std::path::Path::new(&path.to_string())) {
            Ok(game) => {
                self.game = game;
                GString::new()
            }
            Err(error) => error.as_str().into(),
        }
    }

    #[func]
    fn start(&mut self) {
        if !self.game.insolvent() {
            self.game.clock.set_running(true);
        }
    }

    #[func]
    fn pause(&mut self) {
        self.game.clock.set_running(false);
    }

    #[func]
    fn is_running(&self) -> bool {
        self.game.clock.is_running()
    }

    #[func]
    fn set_speed(&mut self, multiplier: i64) -> bool {
        u32::try_from(multiplier).is_ok_and(|speed| self.game.clock.set_speed(speed))
    }

    #[func]
    fn get_speed(&self) -> i64 {
        i64::from(self.game.clock.speed())
    }

    #[func]
    fn advance(&mut self, real_seconds: f64) -> bool {
        self.jump_worker.is_some() || self.game.advance(real_seconds)
    }

    #[func]
    fn get_jump_options(&self) -> Array<VarDictionary> {
        jumps::RULES
            .options
            .iter()
            .map(|o| {
                vdict! {
                    "label" => o.label.as_str(), "seconds" => o.seconds as i64
                }
            })
            .collect()
    }

    #[func]
    fn jump(&mut self, game_seconds: f64) -> VarDictionary {
        jump_dictionary(self.game.jump(game_seconds))
    }

    /// Worker owns only plain Rust data; no Godot objects cross threads.
    /// The UI keeps a modal progress window open until poll_jump commits it.
    #[func]
    fn begin_jump(&mut self, game_seconds: f64) -> bool {
        if self.jump_worker.is_some() {
            return false;
        }
        let mut game = self.game.clone();
        self.jump_worker = Some(std::thread::spawn(move || {
            let result = game.jump(game_seconds);
            (game, result)
        }));
        true
    }

    /// Next commitment the jump action may target; empty when unavailable.
    #[func]
    fn get_next_commitment_jump(&self) -> VarDictionary {
        let Some(c) = self.game.next_commitment_jump() else {
            return VarDictionary::new();
        };
        let (kind, title) = match &c.kind {
            orders::CommitmentKind::Delivery { model, .. } => ("delivery", model.as_str()),
            orders::CommitmentKind::Training { name, .. } => ("training", name.as_str()),
        };
        vdict! {
            "kind" => kind, "title" => title, "at" => c.at as i64,
            "date" => operations::display_time(c.at).as_str()
        }
    }

    /// Validates the displayed target against current state before starting the
    /// worker; a stale target starts nothing and reports `stale`.
    #[func]
    fn begin_jump_to_commitment(&mut self, at: i64) -> VarDictionary {
        if self.jump_worker.is_some() {
            return vdict! { "started" => false, "stale" => false, "message" => "Ein Zeitsprung läuft bereits." };
        }
        let target = u64::try_from(at).ok();
        if target.is_none_or(|t| self.game.next_commitment_jump().is_none_or(|c| c.at != t)) {
            return vdict! { "started" => false, "stale" => true, "message" => jumps::STALE_TARGET };
        }
        let target = target.unwrap();
        let mut game = self.game.clone();
        self.jump_worker = Some(std::thread::spawn(move || {
            let result = game.jump_to_commitment(target);
            (game, result)
        }));
        vdict! { "started" => true, "stale" => false, "message" => "" }
    }

    #[func]
    fn poll_jump(&mut self) -> VarDictionary {
        if !self
            .jump_worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return VarDictionary::new();
        }
        match self.jump_worker.take().unwrap().join() {
            Ok((game, result)) => {
                if result.is_ok() {
                    self.game = game;
                }
                jump_dictionary(result)
            }
            Err(_) => jump_dictionary(Err(
                "Zeitsprung abgebrochen: Simulation konnte nicht fortgeschrieben werden.",
            )),
        }
    }

    #[func]
    fn get_elapsed_seconds(&self) -> f64 {
        self.game.clock.elapsed().as_secs_f64()
    }

    #[func]
    fn get_display_time(&self) -> GString {
        self.game.clock.display_time().as_str().into()
    }
}

impl SimulationClock {
    /// Why the route's aircraft cannot start a new flight plan; empty if it can.
    fn route_maintenance_status(&self, route: &routes::Route) -> String {
        let Some(plane) = self
            .game
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
        else {
            return String::new();
        };
        match &plane.maintenance {
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance && !v.staffed => {
                "In Wartung · wartet auf Techniker (Personal-Tab)".to_string()
            }
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance => format!(
                "{} bis {}{}",
                if v.unscheduled {
                    "In Pflichtwartung"
                } else {
                    "In Wartung"
                },
                operations::display_time(v.end),
                if v.resume.as_ref().is_some_and(|r| r.route_id == route.id) {
                    " · Flugplan wird danach automatisch fortgesetzt"
                } else {
                    ""
                }
            ),
            Some(v) => format!(
                "Wartung beauftragt · beginnt nach der Landung {}",
                operations::display_time(v.start)
            ),
            None if route.plan.is_none()
                && maintenance::rules(&plane.model_id).is_ok_and(|r| plane.wear.is_due(r)) =>
            {
                "Wartung erforderlich (Zustand 0 %) · im Flotte-Tab warten".into()
            }
            None => String::new(),
        }
    }

    /// Crew needed by parked aircraft without a flight plan.
    fn idle_crew_demand(&self) -> (u32, u32) {
        let g = &self.game;
        let (mut pilots, mut cabin) = (0, 0);
        for p in g.fleet.iter().filter(|p| {
            p.status == aircraft::AircraftStatus::Parked
                && g.planned_route(p.id).is_none()
                && p.maintenance.is_none()
        }) {
            if let Ok(n) = staff::needs(&p.model_id) {
                pilots += n.pilots;
                cabin += n.cabin_crew;
            }
        }
        (pilots, cabin)
    }

    /// Missing crew that keeps the route's aircraft from flying; empty if none.
    fn route_staff_status(&self, route: &routes::Route) -> String {
        let Some(plane) = self
            .game
            .fleet
            .iter()
            .find(|p| Some(p.id) == route.aircraft_id)
        else {
            return String::new();
        };
        if route.plan.is_some() || plane.maintenance.is_some() {
            return String::new();
        }
        self.game
            .require_crew(&plane.model_id)
            .err()
            .unwrap_or_default()
    }

    fn maintenance_info(&self, p: usize, plane: &aircraft::Aircraft) -> VarDictionary {
        let Ok(r) = maintenance::rules(&plane.model_id) else {
            return VarDictionary::new();
        };
        let w = &plane.wear;
        let hours = |s: u64| s as f64 / 3600.0;
        let quote = self.game.quote_maintenance(plane.id);
        let (state, visit) = match &plane.maintenance {
            Some(v) if plane.status == aircraft::AircraftStatus::Maintenance => ("active", Some(v)),
            Some(v) => ("pending", Some(v)),
            None => ("none", None),
        };
        let shown = visit.cloned().or_else(|| quote.as_ref().ok().cloned());
        let due = self.game.due_landing(p);
        let location = self
            .game
            .maintenance_location(plane.id)
            .unwrap_or_else(|_| plane.location_icao.clone());
        let regular = self
            .game
            .check_at(&location, w, r, false)
            .unwrap_or_default();
        let forced = self
            .game
            .check_at(&location, w, r, true)
            .unwrap_or_default();
        let waiting = visit.is_some_and(|v| !v.staffed)
            && plane.status == aircraft::AircraftStatus::Maintenance;
        let needed = staff::needs(&plane.model_id).map_or(0, |n| i64::from(n.technicians));
        vdict! {
            "maintenance_technicians_needed" => if waiting { needed } else { 0 },
            "maintenance_technicians_available" => if waiting { i64::from(self.game.available_staff(staff::Role::Technician)) } else { 0 },
            "maintenance_location" => location.as_str(),
            "maintenance_location_kind" => if self.game.is_maintenance_base(&location) { "Wartungsbasis" } else { "Außenstation (Aufschlag)" },
            "condition_percent" => w.condition_permille(r) as f64 / 10.0,
            "flight_hours" => hours(w.flight_seconds), "cycles" => w.cycles as i64,
            "checks" => w.checks as i64,
            "check_flight_hours" => hours(w.check_seconds), "check_cycles" => w.check_cycles as i64,
            "interval_flight_hours" => r.interval_flight_hours as i64,
            "interval_cycles" => r.interval_cycles as i64,
            "maintenance_due" => w.is_due(r),
            "maintenance_state" => state,
            "maintenance_waiting_staff" => waiting,
            "maintenance_unscheduled" => visit.is_some_and(|v| v.unscheduled),
            "maintenance_start" => shown.as_ref().map(|v| operations::display_time(v.start)).unwrap_or_default().as_str(),
            "maintenance_end" => shown.as_ref().map(|v| operations::display_time(v.end)).unwrap_or_default().as_str(),
            "maintenance_end_seconds" => shown.as_ref().map(|v| v.end as i64).unwrap_or(-1),
            "maintenance_cost_eur" => shown.as_ref().map(|v| v.cost_eur).unwrap_or_else(|| regular.0),
            "maintenance_hours" => shown.as_ref().map_or(regular.1, |v| v.end - v.start) as f64 / 3600.0,
            "unscheduled_cost_eur" => forced.0,
            "unscheduled_hours" => forced.1 as f64 / 3600.0,
            "due_time" => due.map(operations::display_time).unwrap_or_default().as_str(),
            "can_request_maintenance" => quote.is_ok(),
            "can_cancel_maintenance" => state == "pending" && !visit.is_some_and(|v| v.unscheduled),
            "maintenance_hint" => quote.err().unwrap_or_default().as_str()
        }
    }
}

fn flight_dictionary(f: &economy::FlightResult) -> VarDictionary {
    let t = &f.totals;
    vdict! { "route_id" => f.route_id, "landing" => operations::display_time(f.landing).as_str(),
    "origin" => f.origin.as_str(), "destination" => f.destination.as_str(), "ticket_eur" => f.ticket_eur,
    "connecting_passengers" => t.connecting_passengers, "local_passengers" => t.passengers - t.connecting_passengers,
    "connecting_revenue_eur" => t.connecting_revenue,
    "passengers" => t.passengers, "seats" => t.seats, "load_percent" => t.load_percent(),
    "business_passengers" => t.business_passengers, "economy_passengers" => t.passengers - t.business_passengers,
    "business_revenue_eur" => t.business_revenue, "economy_revenue_eur" => t.revenue - t.business_revenue - t.cargo_revenue - t.ancillary,
    "business_seats" => t.business_seats, "economy_seats" => t.seats - t.business_seats,
    "cargo_kg" => t.cargo_kg, "cargo_revenue_eur" => t.cargo_revenue, "ticket_revenue_eur" => t.revenue - t.cargo_revenue - t.ancillary,
                    "revenue_eur" => t.revenue, "fuel_eur" => t.fuel, "carbon_eur" => t.carbon, "airport_eur" => t.airport,
    "crew_eur" => t.crew, "overhead_eur" => t.overhead, "care_eur" => t.care, "punctuality_percent" => disruptions::percent(t), "punctuality_flights" => t.punctuality_flights, "service_eur" => t.service, "ancillary_revenue_eur" => t.ancillary, "loyalty_cost_eur" => t.loyalty_cost, "costs_eur" => t.costs(), "profit_eur" => t.profit() }
}

fn jump_dictionary(result: Result<jumps::ResultSummary, &'static str>) -> VarDictionary {
    match result {
        Ok(result) => {
            let attention = result
                .attention
                .as_ref()
                .map_or_else(VarDictionary::new, |e| {
                    vdict! {
                        "kind" => e.kind, "page" => e.page, "route_id" => e.route_id,
                        "aircraft_id" => e.aircraft_id, "employee_id" => e.employee_id,
                        "order_id" => e.order_id as i64
                    }
                });
            vdict! {
                "advanced_seconds" => result.advanced_seconds as i64,
                "stopped" => !result.reason.is_empty(),
                "reason" => result.reason.as_str(), "message" => result.message(),
                "attention" => &attention
            }
        }
        Err(error) => vdict! { "advanced_seconds" => 0, "error" => error, "message" => error },
    }
}

#[cfg(test)]
mod freighter_tests;

#[cfg(test)]
mod codeshare_standard_tests;
