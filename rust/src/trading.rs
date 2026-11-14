//! Aircraft disposal and fixed-price leases. All postings use simulation time.
use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    game::Game,
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub dealer_markup_bps: i64,
    pub market_expiry_days: u64,
    pub market_size_cap: usize,
    pub resale_initial_bps: i64,
    pub resale_floor_bps: i64,
    pub annual_depreciation_bps: i64,
    pub condition_floor_bps: i64,
    pub lease_deposit_bps: i64,
    pub monthly_lease_bps: i64,
    pub lease_period_days: u64,
    pub return_damage_bps: i64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/aircraft_trading.json"))
        .expect("aircraft trading config");
    assert!((0..=2000).contains(&r.dealer_markup_bps));
    assert!((1..=3650).contains(&r.market_expiry_days));
    assert!((1..=1024).contains(&r.market_size_cap));
    assert!((1..=10000).contains(&r.resale_initial_bps));
    assert!((0..=r.resale_initial_bps).contains(&r.resale_floor_bps));
    assert!((0..=10000).contains(&r.annual_depreciation_bps));
    assert!((0..=10000).contains(&r.condition_floor_bps));
    assert!((1..=10000).contains(&r.lease_deposit_bps));
    assert!((1..=10000).contains(&r.monthly_lease_bps));
    assert!((1..=365).contains(&r.lease_period_days));
    assert!((0..=10000).contains(&r.return_damage_bps));
    r
});

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lease {
    pub deposit: i64,
    pub monthly: i64,
    pub period: u64,
    pub next_due: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Sale,
    Cabin,
    Deposit,
    Rent,
    Return,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub time: u64,
    pub cash: i64,
    pub kind: Kind,
}

pub fn lease_terms(model_id: &str) -> Result<(i64, i64), String> {
    let price = i128::from(aircraft::model(model_id)?.price_eur);
    Ok((
        (price * i128::from(RULES.lease_deposit_bps) / 10000).max(1) as i64,
        (price * i128::from(RULES.monthly_lease_bps) / 10000).max(1) as i64,
    ))
}

impl Game {
    pub(crate) fn post_aircraft_cash(&mut self, cash: i64, kind: Kind) -> Result<(), String> {
        let balance = self.cash().checked_add(cash).ok_or("Finanzüberlauf.")?;
        let a = self
            .airline
            .as_mut()
            .ok_or("Bitte zuerst eine Airline gründen.")?;
        let h = self.history.as_mut().ok_or("Finanzhistorie fehlt.")?;
        a.capital_eur = balance;
        h.aircraft_cash.push(Entry {
            time: self.clock.elapsed().as_secs(),
            cash,
            kind,
        });
        self.observe_distress();
        Ok(())
    }

    pub fn lease_aircraft(&mut self, model_id: &str) -> Result<(), String> {
        self.require_solvent()?;
        let model = aircraft::model(model_id)?;
        let home = self
            .airline
            .as_ref()
            .ok_or("Bitte zuerst eine Airline gründen.")?
            .home_airport_icao
            .clone();
        let (deposit, monthly) = lease_terms(model_id)?;
        if self.cash() < deposit {
            return Err("Nicht genügend Guthaben für die Leasingkaution.".into());
        }
        let next = self
            .next_aircraft_id
            .checked_add(1)
            .ok_or("Keine weitere Flugzeugkennung verfügbar.")?;
        let now = self.clock.elapsed().as_secs();
        let period = RULES.lease_period_days * 86400;
        let next_due = now.checked_add(period).ok_or("Zeitüberlauf.")?;
        self.post_aircraft_cash(-deposit, Kind::Deposit)?;
        self.fleet.push(Aircraft {
            business_seats: 0,
            id: self.next_aircraft_id,
            model_id: model_id.into(),
            location_icao: home,
            status: AircraftStatus::Parked,
            purchase_price_eur: model.price_eur,
            acquired_at: Some(now),
            purchased_at: None,
            lease: Some(Lease {
                deposit,
                monthly,
                period,
                next_due,
            }),
            wear: Default::default(),
            maintenance: None,
        });
        self.next_aircraft_id = next;
        Ok(())
    }

    fn disposable(&self, id: u32) -> Result<&Aircraft, String> {
        let p = self
            .fleet
            .iter()
            .find(|p| p.id == id)
            .ok_or("Unbekanntes Flugzeug.")?;
        if p.maintenance.is_some() || p.status == AircraftStatus::Maintenance {
            return Err("Verkauf/Rückgabe verweigert: Wartung beauftragt oder aktiv.".into());
        }
        if !matches!(p.status, AircraftStatus::Parked | AircraftStatus::Ground) {
            return Err("Verkauf/Rückgabe verweigert: Flugzeug ist in der Luft.".into());
        }
        Ok(p)
    }

    /// Positive cash quote; returns refund after condition-based damage fee for leases.
    pub fn disposal_quote(&self, id: u32) -> Result<i64, String> {
        let p = self.disposable(id)?;
        let price = i128::from(aircraft::model(&p.model_id)?.price_eur);
        let condition = i128::from(
            p.wear
                .condition_permille(crate::maintenance::rules(&p.model_id)?),
        );
        if let Some(l) = &p.lease {
            let fee = price * i128::from(RULES.return_damage_bps) * (1000 - condition) / 10_000_000;
            return Ok((i128::from(l.deposit) - fee).max(0) as i64);
        }
        self.hull_value(p)
    }

    pub fn hull_value(&self, p: &Aircraft) -> Result<i64, String> {
        hull_value_at(p, self.clock.elapsed().as_secs())
    }

    pub fn sell_aircraft(&mut self, id: u32) -> Result<(), String> {
        self.require_active()?;
        if self.disposable(id)?.lease.is_some() {
            return Err("Leasingflugzeuge können nur zurückgegeben, nicht verkauft werden.".into());
        }
        self.dispose_aircraft(id, Kind::Sale)
    }
    pub fn return_aircraft(&mut self, id: u32) -> Result<(), String> {
        self.require_active()?;
        if self.disposable(id)?.lease.is_none() {
            return Err("Eigene Flugzeuge müssen verkauft werden.".into());
        }
        self.dispose_aircraft(id, Kind::Return)
    }
    fn dispose_aircraft(&mut self, id: u32, kind: Kind) -> Result<(), String> {
        let quote = self.disposal_quote(id)?;
        self.used_market
            .next_id
            .checked_add(1)
            .ok_or("Marktkennung erschöpft.")?;
        self.post_aircraft_cash(quote, kind)?;
        for r in &mut self.routes {
            if r.aircraft_id == Some(id) {
                r.plan = None;
                r.aircraft_id = None;
                r.remove_at = None;
                r.rotation_phase = None;
            }
        }
        let index = self.fleet.iter().position(|p| p.id == id).unwrap();
        let mut plane = self.fleet.remove(index);
        let seller = if plane.lease.take().is_some() {
            "Leasinggesellschaft".to_string()
        } else {
            self.airline.as_ref().unwrap().name.clone()
        };
        self.prune_used_market();
        // IDs never repeat, including when a listing expires or is scrapped.
        let listing_id = self.used_market.next_id;
        self.used_market.next_id += 1;
        plane.status = AircraftStatus::Parked;
        self.used_market.listings.push(Listing {
            id: listing_id,
            listed_at: self.clock.elapsed().as_secs(),
            seller,
            aircraft: plane,
        });
        if self.used_market.listings.len() > RULES.market_size_cap {
            self.used_market.listings.remove(0);
        }
        Ok(())
    }

    pub fn next_lease_event(&self) -> Option<u64> {
        self.fleet
            .iter()
            .filter_map(|p| p.lease.as_ref().map(|l| l.next_due))
            .min()
    }
    pub fn apply_leases(&mut self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        for i in 0..self.fleet.len() {
            if let Some(l) = &self.fleet[i].lease
                && l.next_due <= now
            {
                let next = l.next_due.checked_add(l.period).ok_or("Zeitüberlauf.")?;
                let monthly = l.monthly;
                self.post_aircraft_cash(-monthly, Kind::Rent)?;
                self.fleet[i].lease.as_mut().unwrap().next_due = next;
            }
        }
        Ok(())
    }

    pub fn validate_trading(&self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        self.validate_used_market()?;
        for p in &self.fleet {
            let acquired = p.acquired_at.ok_or("Erwerbsdatum fehlt.")?;
            if acquired > now || p.purchased_at.is_some_and(|t| t < acquired || t > now) {
                return Err("Ungültiges Erwerbsdatum.".into());
            }
            if let Some(l) = &p.lease
                && (l.deposit <= 0
                    || l.deposit > p.purchase_price_eur
                    || l.monthly <= 0
                    || l.monthly > p.purchase_price_eur
                    || l.period == 0
                    || l.period > 365 * 86400
                    || l.next_due <= now
                    || l.next_due <= acquired
                    || l.next_due - now > l.period
                    || !(l.next_due - acquired).is_multiple_of(l.period))
            {
                return Err("Ungültiger Leasingvertrag.".into());
            }
        }
        if let Some(h) = &self.history {
            let mut previous = h.since;
            for e in &h.aircraft_cash {
                if e.time < previous
                    || e.time > now
                    || match e.kind {
                        Kind::Sale | Kind::Return => e.cash < 0,
                        Kind::Deposit | Kind::Rent | Kind::Cabin => e.cash >= 0,
                    }
                {
                    return Err("Ungültiges Flugzeugjournal.".into());
                }
                previous = e.time;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    pub id: u64,
    pub listed_at: u64,
    pub seller: String,
    pub aircraft: Aircraft,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Market {
    pub next_id: u64,
    pub listings: Vec<Listing>,
}
impl Default for Market {
    fn default() -> Self {
        Self {
            next_id: 1,
            listings: Vec::new(),
        }
    }
}

impl Game {
    pub fn prune_used_market(&mut self) {
        let now = self.clock.elapsed().as_secs();
        self.used_market
            .listings
            .retain(|l| now.saturating_sub(l.listed_at) < RULES.market_expiry_days * 86400);
    }

    pub fn used_price(&self, listing: &Listing) -> Result<i64, String> {
        let hull = self.hull_value(&listing.aircraft)?;
        Ok((i128::from(hull) * i128::from(10000 + RULES.dealer_markup_bps) / 10000).max(1) as i64)
    }

    /// Same admission and purchase journal as a factory purchase, preserving wear.
    pub fn buy_used_aircraft(&mut self, listing_id: u64) -> Result<(), String> {
        self.require_solvent()?;
        let home = self
            .airline
            .as_ref()
            .ok_or("Bitte zuerst eine Airline gründen.")?
            .home_airport_icao
            .clone();
        let now = self.clock.elapsed().as_secs();
        let i = self
            .used_market
            .listings
            .iter()
            .position(|l| {
                l.id == listing_id
                    && now.saturating_sub(l.listed_at) < RULES.market_expiry_days * 86400
            })
            .ok_or("Angebot nicht mehr verfügbar.")?;
        let price = self.used_price(&self.used_market.listings[i])?;
        if self.cash() < price {
            return Err("Nicht genügend Guthaben.".into());
        }
        let next = self
            .next_aircraft_id
            .checked_add(1)
            .ok_or("Keine weitere Flugzeugkennung verfügbar.")?;
        let mut plane = self.used_market.listings.remove(i).aircraft;
        plane.id = self.next_aircraft_id;
        plane.location_icao = home;
        plane.purchase_price_eur = price;
        plane.purchased_at = Some(now);
        self.airline.as_mut().unwrap().capital_eur -= price;
        if let Some(h) = &mut self.history {
            h.purchases.push(crate::history::Purchase {
                time: now,
                amount: price,
            });
        }
        self.fleet.push(plane);
        self.next_aircraft_id = next;
        Ok(())
    }

    fn validate_used_market(&self) -> Result<(), String> {
        let now = self.clock.elapsed().as_secs();
        let mut previous = (0, 0);
        if self.used_market.next_id == 0 || self.used_market.listings.len() > RULES.market_size_cap
        {
            return Err("Ungültiger Gebrauchtmarkt.".into());
        }
        for l in &self.used_market.listings {
            let p = &l.aircraft;
            aircraft::model(&p.model_id)?;
            crate::cabins::layout(p)?;
            if l.id <= previous.0
                || l.id >= self.used_market.next_id
                || l.listed_at < previous.1
                || l.listed_at > now
                || now - l.listed_at >= RULES.market_expiry_days * 86400
                || l.seller.is_empty()
                || l.seller.chars().count() > 80
                || p.acquired_at.is_none_or(|t| t > l.listed_at)
                || p.purchased_at
                    .is_some_and(|t| t > l.listed_at || Some(t) < p.acquired_at)
                || p.status != AircraftStatus::Parked
                || p.lease.is_some()
                || p.maintenance.is_some()
                || p.wear.check_seconds > p.wear.flight_seconds
                || p.wear.check_cycles > p.wear.cycles
                || p.purchase_price_eur <= 0
                || !crate::airports::catalog()?
                    .airports
                    .iter()
                    .any(|a| a.icao == p.location_icao)
            {
                return Err("Ungültiges Gebrauchtangebot.".into());
            }
            previous = (l.id, l.listed_at);
        }
        Ok(())
    }
}

/// Same valuation for current quotes and AI residual value, with existing wear.
pub fn hull_value_at(p: &Aircraft, now: u64) -> Result<i64, String> {
    let price = i128::from(aircraft::model(&p.model_id)?.price_eur);
    let condition = i128::from(
        p.wear
            .condition_permille(crate::maintenance::rules(&p.model_id)?),
    );
    let age = now
        .checked_sub(p.acquired_at.ok_or("Erwerbsdatum fehlt.")?)
        .ok_or("Ungültiges Erwerbsdatum.")?;
    let depreciation = i128::from(age) * i128::from(RULES.annual_depreciation_bps) / (365 * 86400);
    let age_bps = (i128::from(RULES.resale_initial_bps) - depreciation)
        .max(i128::from(RULES.resale_floor_bps));
    let condition_bps = i128::from(RULES.condition_floor_bps)
        + (10000 - i128::from(RULES.condition_floor_bps)) * condition / 1000;
    Ok((price * age_bps * condition_bps / 100_000_000) as i64)
}
