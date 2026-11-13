//! Aircraft disposal and fixed-price leases. All postings use simulation time.
use crate::{
    aircraft::{self, Aircraft, AircraftStatus},
    game::Game,
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
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
    fn post_aircraft_cash(&mut self, cash: i64, kind: Kind) -> Result<(), String> {
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
            id: self.next_aircraft_id,
            model_id: model_id.into(),
            location_icao: home,
            status: AircraftStatus::Parked,
            purchase_price_eur: model.price_eur,
            acquired_at: Some(now),
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
        let age = self
            .clock
            .elapsed()
            .as_secs()
            .checked_sub(p.acquired_at.ok_or("Erwerbsdatum fehlt.")?)
            .ok_or("Ungültiges Erwerbsdatum.")?;
        let depreciation =
            i128::from(age) * i128::from(RULES.annual_depreciation_bps) / (365 * 86400);
        let age_bps = (i128::from(RULES.resale_initial_bps) - depreciation)
            .max(i128::from(RULES.resale_floor_bps));
        let condition_bps = i128::from(RULES.condition_floor_bps)
            + (10000 - i128::from(RULES.condition_floor_bps)) * condition / 1000;
        Ok((price * age_bps * condition_bps / 100_000_000) as i64)
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
        self.post_aircraft_cash(quote, kind)?;
        for r in &mut self.routes {
            if r.aircraft_id == Some(id) {
                r.plan = None;
                r.aircraft_id = None;
            }
        }
        self.fleet.retain(|p| p.id != id);
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
        for p in &self.fleet {
            let acquired = p.acquired_at.ok_or("Erwerbsdatum fehlt.")?;
            if acquired > now {
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
                        Kind::Deposit | Kind::Rent => e.cash >= 0,
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
