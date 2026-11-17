//! Factory positions settle only at dated delivery/retry boundaries.
use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    game::Game,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};
const DAY: u64 = 86400;
#[derive(Deserialize)]
pub struct Rules {
    pub lead_days: BTreeMap<String, u64>,
    pub stagger_days: u64,
    pub deposit_bps: i64,
    pub discount_tiers: Vec<(u32, i64)>,
    pub grace_days: u64,
    pub max_open_orders: usize,
    pub ai_min_fleet: usize,
    pub ai_max_fleet: usize,
    pub ai_order_every: usize,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/manufacturer_orders.json"))
        .expect("manufacturer orders config");
    assert!((1..10000).contains(&r.deposit_bps));
    assert!((1..=365).contains(&r.grace_days));
    assert!((1..=365).contains(&r.stagger_days));
    assert!((1..=100).contains(&r.max_open_orders));
    assert!(r.ai_order_every > 0);
    assert!(r.ai_max_fleet > r.ai_min_fleet);
    for m in aircraft::models().unwrap() {
        assert!((1..=3650).contains(&r.lead_days[&m.id]));
    }
    assert_eq!(r.discount_tiers.first().map(|x| x.0), Some(1));
    assert!(
        r.discount_tiers
            .windows(2)
            .all(|w| w[0].0 < w[1].0 && w[0].1 <= w[1].1)
    );
    assert!(r.discount_tiers.iter().all(|t| (0..10000).contains(&t.1)));
    r
});
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub model_id: String,
    pub unit_price: i64,
    pub deposit: i64,
    pub ordered_at: u64,
    pub delivery_at: u64,
    pub held_since: Option<u64>,
}
pub struct Quote {
    pub unit_price: i64,
    pub deposit_per_aircraft: i64,
    pub balance: i64,
    pub delivery_dates: Vec<u64>,
}
/// Whole-EUR terms; balance is the exact remainder after both floors.
pub(crate) fn price_terms(price: i64, discount_bps: i64, deposit_bps: i64) -> (i64, i64, i64) {
    let unit = (i128::from(price) * i128::from(10000 - discount_bps) / 10000) as i64;
    let deposit = (i128::from(unit) * i128::from(deposit_bps) / 10000).max(1) as i64;
    (unit, deposit, unit - deposit)
}
impl Game {
    pub fn quote_order(&self, model: &str, quantity: u32) -> Result<Quote, String> {
        self.require_solvent()?;
        if self.airline.is_none() {
            return Err("Bitte zuerst eine Airline gründen.".into());
        }
        if quantity == 0
            || quantity as usize > RULES.max_open_orders.saturating_sub(self.orders.len())
        {
            return Err(format!(
                "Höchstens {} offene Herstellerbestellungen.",
                RULES.max_open_orders
            ));
        }
        let model_data = aircraft::model(model)?;
        model_data.require_orderable(self.clock.elapsed().as_secs())?;
        let price = model_data.price_eur;
        let discount = RULES
            .discount_tiers
            .iter()
            .rev()
            .find(|t| quantity >= t.0)
            .unwrap()
            .1;
        // Round the discounted unit price and the deposit down to whole EUR;
        // the balance is the exact remainder, never independently rounded.
        let (unit_price, deposit_per_aircraft, balance) =
            price_terms(price, discount, RULES.deposit_bps);
        let now = self.clock.elapsed().as_secs();
        // Deliveries occur at midnight, including orders placed mid-day.
        let first_day = (now / DAY)
            .checked_add(u64::from(
                !now.is_multiple_of(DAY) || self.clock.elapsed().subsec_nanos() != 0,
            ))
            .and_then(|day| day.checked_add(RULES.lead_days[model]))
            .ok_or("Zeitüberlauf.")?;
        let delivery_dates = (0..quantity)
            .map(|i| {
                first_day
                    .checked_add(u64::from(i) * RULES.stagger_days)
                    .and_then(|d| d.checked_mul(DAY))
                    .filter(|t| {
                        t.checked_add(RULES.grace_days * DAY)
                            .is_some_and(|end| end <= i64::MAX as u64)
                    })
                    .ok_or_else(|| "Zeitüberlauf.".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Quote {
            unit_price,
            deposit_per_aircraft,
            balance,
            delivery_dates,
        })
    }
    pub fn place_order(&mut self, model: &str, quantity: u32) -> Result<(), String> {
        let q = self.quote_order(model, quantity)?;
        let total = q
            .deposit_per_aircraft
            .checked_mul(i64::from(quantity))
            .ok_or("Finanzüberlauf.")?;
        let next = self
            .next_order_id
            .checked_add(u64::from(quantity))
            .ok_or("Keine Bestellkennung verfügbar.")?;
        if self.cash() < total {
            return Err("Nicht genügend Guthaben für die Anzahlung.".into());
        }
        self.post_order_investment(total)?;
        for (i, delivery_at) in q.delivery_dates.into_iter().enumerate() {
            self.orders.push(Order {
                id: self.next_order_id + i as u64,
                model_id: model.into(),
                unit_price: q.unit_price,
                deposit: q.deposit_per_aircraft,
                ordered_at: self.clock.elapsed().as_secs(),
                delivery_at,
                held_since: None,
            });
        }
        self.next_order_id = next;
        Ok(())
    }
    pub fn cancel_order(&mut self, id: u64) -> Result<(), String> {
        self.require_active()?;
        let i = self
            .orders
            .iter()
            .position(|o| o.id == id)
            .ok_or("Bestellung nicht gefunden.")?;
        self.orders.remove(i); // The deposit was already invested; no refund.
        Ok(())
    }
    fn post_order_investment(&mut self, amount: i64) -> Result<(), String> {
        let balance = self.cash().checked_sub(amount).ok_or("Finanzüberlauf.")?;
        let airline = self.airline.as_mut().ok_or("Airline fehlt.")?;
        let h = self.history.as_mut().ok_or("Finanzhistorie fehlt.")?;
        h.purchases.push(crate::history::Purchase {
            time: self.clock.elapsed().as_secs(),
            amount,
        });
        airline.capital_eur = balance;
        Ok(())
    }
    pub fn next_order_event(&self) -> Option<u64> {
        self.orders
            .iter()
            .map(|o| {
                o.held_since.map_or(o.delivery_at, |_| {
                    (self.clock.elapsed().as_secs() / DAY + 1) * DAY
                })
            })
            .min()
    }
    pub(crate) fn apply_orders(&mut self) -> Result<(), String> {
        if self.insolvent() {
            self.orders.clear();
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        let mut i = 0;
        while i < self.orders.len() {
            let o = &self.orders[i];
            if o.delivery_at > now
                || (o.held_since.is_some()
                    && (!now.is_multiple_of(DAY) || self.clock.elapsed().subsec_nanos() != 0))
            {
                i += 1;
                continue;
            }
            let balance = o.unit_price - o.deposit;
            if self.cash() >= balance && self.next_aircraft_id < u32::MAX {
                let o = o.clone();
                let home = self
                    .airline
                    .as_ref()
                    .ok_or("Airline fehlt.")?
                    .home_airport_icao
                    .clone();
                self.post_order_investment(balance)?;
                self.fleet.push(Aircraft {
                    id: self.next_aircraft_id,
                    model_id: o.model_id,
                    location_icao: home,
                    status: AircraftStatus::Parked,
                    purchase_price_eur: o.unit_price,
                    acquired_at: Some(now),
                    purchased_at: None,
                    lease: None,
                    wear: Default::default(),
                    maintenance: None,
                    business_seats: 0,
                });
                self.next_aircraft_id += 1;
                self.orders.remove(i);
            } else if now >= o.delivery_at + RULES.grace_days * DAY {
                self.orders.remove(i);
            } else {
                self.orders[i].held_since.get_or_insert(now);
                i += 1;
            }
        }
        Ok(())
    }
    pub fn validate_orders(&self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let mut ids = HashSet::new();
        if self.next_order_id == 0
            || self.orders.len() > RULES.max_open_orders
            || ((self.airline.is_none() || self.insolvent()) && !self.orders.is_empty())
        {
            return Err("Ungültige Herstellerbestellungen.".into());
        }
        for o in &self.orders {
            aircraft::model(&o.model_id)?;
            if o.id == 0
                || o.id >= self.next_order_id
                || !ids.insert(o.id)
                || o.unit_price <= 0
                || o.deposit <= 0
                || o.deposit >= o.unit_price
                || o.ordered_at > now
                || o.delivery_at <= o.ordered_at
                || !o.delivery_at.is_multiple_of(DAY)
                || o.delivery_at
                    .checked_add(RULES.grace_days * DAY)
                    .is_none_or(|end| end <= now || end > i64::MAX as u64)
                || match o.held_since {
                    Some(t) => t != o.delivery_at || t > now,
                    None => o.delivery_at <= now,
                }
            {
                return Err("Ungültige Herstellerbestellung im Spielstand.".into());
            }
        }
        Ok(())
    }
}
