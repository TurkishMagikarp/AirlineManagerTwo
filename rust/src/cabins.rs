//! Cabin layout and fares; capacity shares the existing segmented market.
use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    demand::Offer,
    game::Game,
    routes::Route,
};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub business_space_ratio: u32,
    pub layout_fee_eur: i64,
    pub business_price_multiplier: f64,
    pub business_preference: f64,
    pub ai_business_share_min: f64,
    pub ai_business_space_percent: u32,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/cabins.json")).expect("cabin config");
    assert!((2..=10).contains(&r.business_space_ratio));
    assert!((0..=100_000).contains(&r.layout_fee_eur));
    assert!(
        r.business_price_multiplier.is_finite()
            && (1.0..=10.0).contains(&r.business_price_multiplier)
    );
    assert!(r.business_preference.is_finite() && (1.0..=10.0).contains(&r.business_preference));
    assert!((0.0..=1.0).contains(&r.ai_business_share_min));
    assert!(r.ai_business_space_percent <= 50);
    r
});

pub fn default_business_fare() -> i64 {
    premium_fare(crate::economy::default_ticket())
}

pub fn premium_fare(economy: i64) -> i64 {
    (economy as f64 * RULES.business_price_multiplier)
        .round()
        .clamp(0.0, 100_000.0) as i64
}

/// Model seats are economy floor-space units. Unused fractional space stays empty.
pub fn layout(plane: &Aircraft) -> Result<(u32, u32), String> {
    let capacity = aircraft::model(&plane.model_id)?.seats;
    let used = plane
        .business_seats
        .checked_mul(RULES.business_space_ratio)
        .ok_or("Ungültige Kabine.")?;
    let economy = capacity
        .checked_sub(used)
        .ok_or("Kabine überschreitet die Fläche.")?;
    Ok((economy, plane.business_seats))
}

pub fn offer(route: &Route, plane: &Aircraft) -> Result<Offer, String> {
    let (economy, business) = layout(plane)?;
    let mut o = Offer::new(route, economy + business);
    o.business_seats = business;
    o.cargo_model = Some(plane.model_id.clone());
    Ok(o)
}

impl Game {
    pub fn set_cabin_layout(&mut self, id: u32, business: u32) -> Result<(), String> {
        self.require_solvent()?;
        let p = self
            .fleet
            .iter()
            .position(|p| p.id == id)
            .ok_or("Unbekanntes Flugzeug.")?;
        let plane = &self.fleet[p];
        if !matches!(
            plane.status,
            AircraftStatus::Parked | AircraftStatus::Ground
        ) || plane.maintenance.is_some()
        {
            return Err("Kabinenumbau nur am Boden und ohne Wartung möglich.".into());
        }
        let mut candidate = plane.clone();
        candidate.business_seats = business;
        layout(&candidate)?;
        if plane.business_seats == business {
            return Ok(());
        }
        if self.cash() < RULES.layout_fee_eur {
            return Err("Nicht genügend Guthaben für Kabinenumbau.".into());
        }
        if RULES.layout_fee_eur > 0 {
            self.post_aircraft_cash(-RULES.layout_fee_eur, crate::trading::Kind::Cabin)?;
        }
        self.fleet[p].business_seats = business;
        Ok(())
    }

    pub fn set_business_ticket_price(&mut self, id: u32, price: i64) -> Result<(), String> {
        if !(0..=100_000).contains(&price) {
            return Err("Ticketpreis muss zwischen 0 und 100000 EUR liegen.".into());
        }
        self.route_mut(id)?;
        for r in self.routes.iter_mut().filter(|r| r.public_id() == id) {
            r.business_ticket_eur = price;
        }
        Ok(())
    }

    pub fn review_cabins(&mut self) {
        let changes: Vec<_> = self
            .routes
            .iter()
            .filter_map(|r| {
                let p = self.fleet.iter().find(|p| Some(p.id) == r.aircraft_id)?;
                if p.business_seats > 0 {
                    return None;
                }
                let m = crate::demand::market(r, 0).ok()?;
                if m.business / m.total() < RULES.ai_business_share_min {
                    return None;
                }
                let seats = aircraft::model(&p.model_id).ok()?.seats;
                Some((
                    p.id,
                    seats * RULES.ai_business_space_percent / 100 / RULES.business_space_ratio,
                ))
            })
            .collect();
        for (id, business) in changes {
            if self.set_cabin_layout(id, business).is_ok() {
                let fares: Vec<_> = self
                    .routes
                    .iter()
                    .filter(|r| r.aircraft_id == Some(id))
                    .map(|r| (r.public_id(), premium_fare(r.ticket_eur)))
                    .collect();
                for (route, fare) in fares {
                    let _ = self.set_business_ticket_price(route, fare);
                }
            }
        }
    }
}

/// Older saves have no premium cabin, including AI fleets. Keep historical totals.
pub fn migrate(state: &mut serde_json::Value) {
    fn fleet(value: &mut serde_json::Value) {
        if let Some(planes) = value.as_array_mut() {
            for p in planes {
                if let Some(object) = p.as_object_mut() {
                    object.insert("business_seats".into(), 0.into());
                }
            }
        }
    }
    if let Some(planes) = state.get_mut("fleet").and_then(|f| f.get_mut("aircraft")) {
        fleet(planes);
    }
    if let Some(companies) = state["competitors"].as_array_mut() {
        for c in companies {
            if let Some(planes) = c.get_mut("game").and_then(|g| g.get_mut("fleet")) {
                fleet(planes);
            }
        }
    }
}
