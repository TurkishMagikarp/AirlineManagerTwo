//! Charter flights: a small rotating set of one-off jobs for idle aircraft with a
//! fixed fee and a contract penalty. Offers are a pure function of the world seed
//! and the calendar (nothing but the taken set and the accepted jobs is saved).
//! The flights themselves are ordinary ferry flights (`operations`).
use crate::{
    aircraft::{self, AircraftStatus},
    game::Game,
    operations::Ferry,
    trading::Kind,
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

const DAY: u64 = 86400;

#[derive(Deserialize)]
pub struct KindDef {
    pub id: String,
    pub name: String,
    pub weight: u64,
    pub freight: bool,
    pub min: u32,
    pub max: u32,
    pub rate_milli_eur: u64,
}
#[derive(Deserialize)]
pub struct Rules {
    pub epoch_days: u64,
    pub offer_lifetime_days: u64,
    pub lead_days_min: u64,
    pub lead_days_max: u64,
    pub window_days: u64,
    pub base_offers_per_epoch: f64,
    pub min_offers_per_epoch: u64,
    pub max_offers_per_epoch: u64,
    pub origin_pool: usize,
    pub destination_pool: usize,
    pub min_distance_km: f64,
    pub max_distance_km: f64,
    pub base_fee_eur: u64,
    pub penalty_percent: i64,
    pub reputation_gain_micro: i64,
    pub reputation_loss_micro: i64,
    pub expiry_warning_hours: u64,
    pub notice_days: u64,
    pub recent_limit: usize,
    pub ai_min_net_eur: i64,
    pub ai_min_margin_percent: i64,
    pub kinds: Vec<KindDef>,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/charters.json")).expect("charters config");
    assert!((1..=30).contains(&r.epoch_days));
    assert!(r.offer_lifetime_days >= 1 && r.offer_lifetime_days <= 60);
    assert!(r.lead_days_min <= r.lead_days_max && r.window_days >= 1);
    assert!(r.base_offers_per_epoch > 0.0);
    assert!(r.min_offers_per_epoch <= r.max_offers_per_epoch && r.max_offers_per_epoch <= 100);
    assert!(r.origin_pool >= 2 && r.destination_pool >= 2);
    assert!(0.0 < r.min_distance_km && r.min_distance_km < r.max_distance_km);
    assert!((0..=100).contains(&r.penalty_percent));
    assert!(r.reputation_gain_micro >= 0 && r.reputation_loss_micro >= 0);
    assert!(r.recent_limit >= 1 && !r.kinds.is_empty());
    assert!(
        r.kinds
            .iter()
            .all(|k| k.weight > 0 && 0 < k.min && k.min <= k.max)
    );
    r
});

#[cfg(test)]
thread_local! {
    /// Existing AI scenarios predate charters; the charter tests switch this on.
    pub(crate) static AI_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

fn kind_def(id: &str) -> Option<&'static KindDef> {
    RULES.kinds.iter().find(|k| k.id == id)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: u64,
    pub kind: String,
    pub origin: String,
    pub destination: String,
    /// Passengers, or tonnes of freight.
    pub quantity: u32,
    pub fee_eur: i64,
    pub created: u64,
    pub window_start: u64,
    pub window_end: u64,
    pub expires: u64,
}
impl Offer {
    pub fn freight(&self) -> bool {
        kind_def(&self.kind).is_some_and(|k| k.freight)
    }
    pub fn kind_name(&self) -> &'static str {
        kind_def(&self.kind).map_or("", |k| k.name.as_str())
    }
    pub fn penalty_eur(&self) -> i64 {
        self.fee_eur * RULES.penalty_percent / 100
    }
    pub fn label(&self) -> String {
        format!("{} → {}", self.origin, self.destination)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub offer: Offer,
    pub aircraft_id: u32,
    pub accepted_at: u64,
    /// Earliest time the charter leg can leave (window start / end of positioning).
    pub dispatch_at: u64,
    /// Set once the charter leg is airborne.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub departure: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub landing: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub offer_id: u64,
    pub label: String,
    pub aircraft_id: u32,
    pub at: u64,
    pub delivered: bool,
    /// Fee earned (positive) or penalty paid (negative).
    pub cash_eur: i64,
    pub reason: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Offers no longer available to anybody: (offer id, expiry). Player's game only.
    #[serde(default)]
    pub taken: Vec<(u64, u64)>,
    #[serde(default)]
    pub jobs: Vec<Job>,
    #[serde(default)]
    pub recent: Vec<Outcome>,
    /// Offers are only generated for epochs starting at or after this time
    /// (old saves start with none).
    #[serde(default)]
    pub since: u64,
}
impl State {
    pub fn is_empty(&self) -> bool {
        self.taken.is_empty() && self.jobs.is_empty() && self.recent.is_empty() && self.since == 0
    }
}

fn mix(seed: u64, a: u64, b: u64) -> u64 {
    let mut z = seed
        .wrapping_add(a.wrapping_mul(0x9e3779b97f4a7c15))
        .wrapping_add(b.wrapping_mul(0xd1342543de82ef95))
        .wrapping_add(0x636861727465);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

fn hubs(n: usize) -> Result<Vec<(&'static str, f64)>, String> {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<usize, Vec<(&'static str, f64)>>> =
            Default::default();
    }
    if let Some(v) = CACHE.with(|c| c.borrow().get(&n).cloned()) {
        return Ok(v);
    }
    let mut a: Vec<_> = crate::airports::catalog()?
        .airports
        .iter()
        .map(|a| (a.icao.as_str(), a.market_size))
        .collect();
    a.sort_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(y.0)));
    a.truncate(n);
    CACHE.with(|c| c.borrow_mut().insert(n, a.clone()));
    Ok(a)
}

/// Offers created at the start of `epoch`, a pure function of seed and calendar.
pub fn epoch_offers(cycle: &crate::world_economy::Cycle, epoch: u64) -> Vec<Offer> {
    let created = epoch * RULES.epoch_days * DAY;
    let season = crate::demand::season_factors(created).leisure;
    let economy = cycle.index_at(created) as f64 / 10000.0;
    let count = ((RULES.base_offers_per_epoch * season * economy)
        .round()
        .max(0.0) as u64)
        .clamp(RULES.min_offers_per_epoch, RULES.max_offers_per_epoch);
    let (Ok(origins), Ok(destinations)) = (hubs(RULES.origin_pool), hubs(RULES.destination_pool))
    else {
        return Vec::new();
    };
    let total_weight: u64 = RULES.kinds.iter().map(|k| k.weight).sum();
    let mut offers = Vec::new();
    for slot in 0..count {
        let r = mix(cycle.seed, epoch, slot);
        let mut pick = r % total_weight;
        let kind = RULES
            .kinds
            .iter()
            .find(|k| {
                if pick < k.weight {
                    true
                } else {
                    pick -= k.weight;
                    false
                }
            })
            .unwrap();
        let origin = origins[(mix(r, 1, 0) % origins.len() as u64) as usize].0;
        let start = (mix(r, 2, 0) % destinations.len() as u64) as usize;
        let Some((destination, distance)) = (0..destinations.len()).find_map(|i| {
            let d = destinations[(start + i) % destinations.len()].0;
            let km = crate::routes::distance_km(origin, d).ok()?;
            (RULES.min_distance_km..=RULES.max_distance_km)
                .contains(&km)
                .then_some((d, km))
        }) else {
            continue;
        };
        let quantity = kind.min + (mix(r, 3, 0) % u64::from(kind.max - kind.min + 1)) as u32;
        let fee = RULES.base_fee_eur
            + (distance * f64::from(quantity) * kind.rate_milli_eur as f64 / 1000.0).round() as u64;
        let lead =
            RULES.lead_days_min + mix(r, 4, 0) % (RULES.lead_days_max - RULES.lead_days_min + 1);
        let window_start = created + lead * DAY;
        offers.push(Offer {
            id: epoch * 1000 + slot,
            kind: kind.id.clone(),
            origin: origin.into(),
            destination: destination.into(),
            quantity,
            fee_eur: fee as i64,
            created,
            window_start,
            window_end: window_start + RULES.window_days * DAY,
            expires: created + RULES.offer_lifetime_days * DAY,
        });
    }
    offers
}

/// Why an aircraft cannot take a job, with the full cost when it can.
#[derive(Clone, Debug)]
pub struct Quote {
    pub positioning: Option<Ferry>,
    pub leg: Ferry,
    pub dispatch_at: u64,
}
impl Quote {
    pub fn cost_eur(&self) -> i64 {
        self.positioning
            .as_ref()
            .map_or(0, |p| p.quote.totals.costs())
            + self.leg.quote.totals.costs()
    }
}

impl Game {
    /// Open offers at the current time (player's game: also minus AI-taken ones).
    pub fn open_charters(&self) -> Vec<Offer> {
        let now = self.clock.elapsed().as_secs();
        let from = (now / DAY).saturating_sub(RULES.offer_lifetime_days) / RULES.epoch_days;
        let mut offers = Vec::new();
        for epoch in from..=now / (RULES.epoch_days * DAY) {
            if epoch * RULES.epoch_days * DAY < self.charters.since {
                continue;
            }
            offers.extend(
                epoch_offers(&self.world_economy, epoch)
                    .into_iter()
                    .filter(|o| {
                        o.created <= now
                            && now < o.expires
                            && !self.charters.taken.iter().any(|t| t.0 == o.id)
                    }),
            );
        }
        offers
    }

    pub fn charter_job(&self, aircraft_id: u32) -> Option<&Job> {
        self.charters
            .jobs
            .iter()
            .find(|j| j.aircraft_id == aircraft_id)
    }

    fn charter_refusal(&self, offer: &Offer, id: u32) -> Result<(), String> {
        let plane = self
            .fleet
            .iter()
            .find(|p| p.id == id)
            .ok_or("Unbekanntes Flugzeug.")?;
        if self.charter_job(id).is_some() {
            return Err("Flugzeug hat bereits einen Charterauftrag.".into());
        }
        self.require_ferry_aircraft(id)?;
        let model = aircraft::model(&plane.model_id)?;
        if plane
            .wear
            .is_due(crate::maintenance::rules(&plane.model_id)?)
        {
            return Err("Wartung fällig: Flugzeug muss zuerst zum Check.".into());
        }
        if offer.freight() {
            if model.seats > 0 {
                return Err("Fracht benötigt einen Frachter.".into());
            }
            if model.payload_kg < offer.quantity * 1000 {
                return Err(format!(
                    "Nutzlast zu gering: {} t benötigt, {} t möglich.",
                    offer.quantity,
                    model.payload_kg / 1000
                ));
            }
        } else if model.seats == 0 {
            return Err("Ein Frachter kann keine Passagiere befördern.".into());
        } else if model.seats < offer.quantity {
            return Err(format!(
                "Zu wenige Sitze: {} benötigt, {} vorhanden.",
                offer.quantity, model.seats
            ));
        }
        let km = crate::routes::distance_km(&offer.origin, &offer.destination)?;
        if km > f64::from(model.range_km) {
            return Err(format!(
                "Reichweite zu gering: {:.0} km benötigt, {} km möglich.",
                km, model.range_km
            ));
        }
        Ok(())
    }

    pub fn quote_charter_offer(&self, offer: &Offer, id: u32) -> Result<Quote, String> {
        let now = self.clock.elapsed().as_secs();
        if now >= offer.expires {
            return Err("Das Angebot ist abgelaufen.".into());
        }
        self.charter_refusal(offer, id)?;
        let plane = self.fleet.iter().find(|p| p.id == id).unwrap();
        let passengers = if offer.freight() {
            0
        } else {
            i64::from(offer.quantity)
        };
        let positioning = if plane.location_icao == offer.origin {
            None
        } else {
            let f = self.quote_leg(id, &plane.location_icao, &offer.origin, now, 0)?;
            Some(f)
        };
        let ready = positioning.as_ref().map_or(now, |p| p.ready);
        let dispatch_at = ready.max(offer.window_start);
        if dispatch_at > offer.window_end {
            return Err("Zeitfenster nicht erreichbar: Überführung dauert zu lange.".into());
        }
        let leg = self.quote_leg(
            id,
            &offer.origin,
            &offer.destination,
            dispatch_at,
            passengers,
        )?;
        Ok(Quote {
            positioning,
            leg,
            dispatch_at,
        })
    }

    /// Books the job (and the positioning flight) on this airline only. The
    /// caller removes the offer from the shared market.
    pub fn accept_charter_offer(&mut self, offer: &Offer, id: u32) -> Result<Quote, String> {
        let quote = self.quote_charter_offer(offer, id)?;
        let now = self.clock.elapsed().as_secs();
        if let Some(p) = &quote.positioning {
            self.start_ferry(p.clone());
        }
        self.charters.jobs.push(Job {
            offer: offer.clone(),
            aircraft_id: id,
            accepted_at: now,
            dispatch_at: quote.dispatch_at,
            departure: None,
            landing: None,
        });
        self.charters.jobs.sort_by_key(|j| j.offer.id);
        Ok(quote)
    }

    pub fn accept_charter(&mut self, offer_id: u64, id: u32) -> Result<(), String> {
        let offer = self
            .open_charters()
            .into_iter()
            .find(|o| o.id == offer_id)
            .ok_or("Das Angebot ist nicht mehr verfügbar.")?;
        let mut staged = self.clone();
        staged.accept_charter_offer(&offer, id)?;
        staged.charters.taken.push((offer.id, offer.expires));
        staged.charters.taken.sort();
        *self = staged;
        Ok(())
    }

    pub fn cancel_charter(&mut self, offer_id: u64) -> Result<(), String> {
        let job = self
            .charters
            .jobs
            .iter()
            .find(|j| j.offer.id == offer_id)
            .ok_or("Unbekannter Charterauftrag.")?;
        if job.departure.is_some() {
            return Err("Der Charterflug ist bereits unterwegs.".into());
        }
        self.fail_charter(offer_id, "Vom Spieler storniert")
    }

    pub(crate) fn fail_charter_aircraft(&mut self, id: u32, why: &str) -> Result<(), String> {
        let ids: Vec<u64> = self
            .charters
            .jobs
            .iter()
            .filter(|j| j.aircraft_id == id && j.departure.is_none())
            .map(|j| j.offer.id)
            .collect();
        for offer in ids {
            self.fail_charter(offer, why)?;
        }
        Ok(())
    }

    fn record_outcome(&mut self, job: &Job, delivered: bool, cash: i64, reason: &str) {
        self.charters.recent.push(Outcome {
            offer_id: job.offer.id,
            label: job.offer.label(),
            aircraft_id: job.aircraft_id,
            at: self.clock.elapsed().as_secs(),
            delivered,
            cash_eur: cash,
            reason: reason.into(),
        });
        let excess = self
            .charters
            .recent
            .len()
            .saturating_sub(RULES.recent_limit);
        self.charters.recent.drain(..excess);
    }

    fn bump_reputation(&mut self, delta: i64) {
        use crate::reputation::SCALE;
        self.reputation.score = (self.reputation.score + delta).clamp(-SCALE, SCALE);
        self.reputation_changed();
    }

    fn fail_charter(&mut self, offer_id: u64, why: &str) -> Result<(), String> {
        let i = self
            .charters
            .jobs
            .iter()
            .position(|j| j.offer.id == offer_id)
            .ok_or("Unbekannter Charterauftrag.")?;
        let job = self.charters.jobs[i].clone();
        let penalty = job.offer.penalty_eur();
        self.post_aircraft_cash(-penalty, Kind::CharterPenalty)?;
        self.charters.jobs.remove(i);
        self.record_outcome(&job, false, -penalty, why);
        self.bump_reputation(-RULES.reputation_loss_micro);
        Ok(())
    }

    pub fn next_charter_event(&self) -> Option<u64> {
        let now = self.clock.elapsed().as_secs();
        self.charters
            .jobs
            .iter()
            .map(|j| j.landing.unwrap_or(j.dispatch_at))
            .chain(self.charters.taken.iter().map(|t| t.1))
            .chain(
                self.charters
                    .recent
                    .iter()
                    .map(|o| o.at + RULES.notice_days * DAY),
            )
            .filter(|&t| t > now)
            .min()
            .or_else(|| {
                // Due work at this very second (e.g. right after loading).
                (self
                    .charters
                    .jobs
                    .iter()
                    .any(|j| j.landing.unwrap_or(j.dispatch_at) <= now)
                    || self.charters.taken.iter().any(|t| t.1 <= now))
                .then_some(now)
            })
    }

    pub(crate) fn apply_charters(&mut self) -> Result<(), String> {
        if self.charters.jobs.is_empty()
            && self.charters.taken.is_empty()
            && self.charters.recent.is_empty()
        {
            return Ok(());
        }
        let now = self.clock.elapsed().as_secs();
        self.charters.taken.retain(|t| t.1 > now);
        let notice = RULES.notice_days * DAY;
        self.charters.recent.retain(|o| o.at + notice > now);
        for offer_id in self
            .charters
            .jobs
            .iter()
            .map(|j| j.offer.id)
            .collect::<Vec<_>>()
        {
            let Some(job) = self
                .charters
                .jobs
                .iter()
                .find(|j| j.offer.id == offer_id)
                .cloned()
            else {
                continue;
            };
            if let Some(landing) = job.landing {
                if landing <= now {
                    self.post_aircraft_cash(job.offer.fee_eur, Kind::CharterFee)?;
                    self.charters.jobs.retain(|j| j.offer.id != offer_id);
                    self.record_outcome(&job, true, job.offer.fee_eur, "");
                    self.bump_reputation(RULES.reputation_gain_micro);
                }
                continue;
            }
            if job.dispatch_at > now {
                continue;
            }
            let plane = self.fleet.iter().find(|p| p.id == job.aircraft_id);
            let Some(plane) = plane else {
                self.fail_charter(offer_id, "Flugzeug nicht mehr vorhanden")?;
                continue;
            };
            let blocked = if plane.status != AircraftStatus::Parked {
                Some("Flugzeug nicht startbereit".to_string())
            } else if plane.location_icao != job.offer.origin {
                Some("Flugzeug nicht am Abflughafen".to_string())
            } else if plane
                .wear
                .is_due(crate::maintenance::rules(&plane.model_id)?)
            {
                Some("Wartung fällig".to_string())
            } else {
                self.require_free_aircraft(job.aircraft_id).err()
            };
            if let Some(reason) = blocked {
                self.fail_charter(offer_id, &reason)?;
                continue;
            }
            let passengers = if job.offer.freight() {
                0
            } else {
                i64::from(job.offer.quantity)
            };
            let leg = self.quote_leg(
                job.aircraft_id,
                &job.offer.origin,
                &job.offer.destination,
                now,
                passengers,
            );
            match leg {
                Ok(leg) => {
                    let landing = leg.quote.landing;
                    self.start_ferry(leg);
                    let j = self
                        .charters
                        .jobs
                        .iter_mut()
                        .find(|j| j.offer.id == offer_id)
                        .unwrap();
                    j.departure = Some(now);
                    j.landing = Some(landing);
                }
                Err(reason) => self.fail_charter(offer_id, &reason)?,
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn ai_charter_candidates_len(&self) -> usize {
        self.ai_charter_candidates().len()
    }

    /// Idle aircraft an AI airline may use for a charter.
    pub(crate) fn ai_charter_candidates(&self) -> Vec<u32> {
        self.fleet
            .iter()
            .filter(|p| {
                p.status == AircraftStatus::Parked
                    && p.maintenance.is_none()
                    && self.planned_route(p.id).is_none()
                    && !self.routes.iter().any(|r| r.aircraft_id == Some(p.id))
                    && self.charter_job(p.id).is_none()
                    && self.ferry(p.id).is_none()
            })
            .map(|p| p.id)
            .collect()
    }

    /// One review step: competitor `i` takes the best fitting open offers with its own
    /// idle aircraft under the player's rules; taken offers vanish for everybody.
    pub(crate) fn ai_charters(&mut self, i: usize) -> Result<(), String> {
        #[cfg(test)]
        if AI_DISABLED.with(|v| v.get()) {
            return Ok(());
        }
        if self.competitors[i].game.distressed() {
            return Ok(());
        }
        let candidates = self.competitors[i].game.ai_charter_candidates();
        if candidates.is_empty() {
            return Ok(());
        }
        let offers = self.open_charters();
        if offers.is_empty() {
            return Ok(());
        }
        let ai = &mut self.competitors[i].game;
        for id in candidates {
            let mut best: Option<(i64, usize)> = None;
            for (k, offer) in offers.iter().enumerate() {
                if self.charters.taken.iter().any(|t| t.0 == offer.id) {
                    continue;
                }
                let Ok(q) = ai.quote_charter_offer(offer, id) else {
                    continue;
                };
                let cost = q.cost_eur();
                let net = offer.fee_eur - cost;
                if net >= RULES.ai_min_net_eur
                    && net * 100 >= cost * RULES.ai_min_margin_percent
                    && best.is_none_or(|(n, _)| net > n)
                {
                    best = Some((net, k));
                }
            }
            if let Some((_, k)) = best {
                let offer = &offers[k];
                ai.accept_charter_offer(offer, id)?;
                self.charters.taken.push((offer.id, offer.expires));
                self.charters.taken.sort();
            }
        }
        Ok(())
    }

    pub fn validate_charters(&self) -> Result<(), String> {
        let bad = || Err("Ungültige Charterdaten.".to_string());
        let now = self.clock.elapsed().as_secs();
        let s = &self.charters;
        if s.since > now
            || !self
                .competitors
                .iter()
                .all(|c| c.game.charters.taken.is_empty() && c.game.charters.since == 0)
            || s.taken.windows(2).any(|w| w[0] >= w[1])
            || s.jobs.windows(2).any(|w| w[0].offer.id >= w[1].offer.id)
            || s.recent.len() > RULES.recent_limit
            || s.recent.iter().any(|o| o.at > now)
        {
            return bad();
        }
        let mut planes = std::collections::HashSet::new();
        for j in &s.jobs {
            let o = &j.offer;
            let Some(def) = kind_def(&o.kind) else {
                return bad();
            };
            if !self.fleet.iter().any(|p| p.id == j.aircraft_id)
                || !planes.insert(j.aircraft_id)
                || o.quantity == 0
                || o.fee_eur <= 0
                || def.freight != o.freight()
                || o.window_start > o.window_end
                || j.accepted_at > now
                || j.dispatch_at < j.accepted_at
                || j.dispatch_at > o.window_end
                || j.departure.is_some() != j.landing.is_some()
                || j.departure
                    .is_some_and(|d| d > now || j.landing.is_none_or(|l| l <= d))
                || crate::routes::distance_km(&o.origin, &o.destination).is_err()
            {
                return bad();
            }
        }
        Ok(())
    }
}
