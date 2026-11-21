//! Deterministic aircraft/cycle technical delays. The cursor retains only the
//! last landing, so projection never replays an aircraft's lifetime history.
use crate::{
    aircraft::AircraftStatus,
    economy::Totals,
    game::Game,
    maintenance::Wear,
    operations::{self, Movement},
    routes::Route,
};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

#[derive(Deserialize)]
pub struct Rules {
    pub reputation_response_days: i64,
    pub reputation_neutral_otp_per_million: i64,
    pub reputation_max_choice_bps: i64,
    pub reputation_attention_score: i64,
    pub base_probability_per_million: u64,
    pub worn_probability_per_million: u64,
    pub age_probability_per_year_per_million: u64,
    pub maximum_age_years: u64,
    pub minimum_seconds: u64,
    pub maximum_seconds: u64,
    pub worn_extra_seconds: u64,
    pub age_extra_seconds_per_year: u64,
    pub on_time_threshold_seconds: u64,
    pub care_threshold_seconds: u64,
    pub care_eur_per_passenger: i64,
    pub attention_threshold_seconds: u64,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/disruptions.json"))
        .expect("Störungskonfiguration ungültig");
    assert!(r.minimum_seconds <= r.maximum_seconds && r.maximum_seconds <= 86400);
    assert!(
        r.maximum_age_years <= 100
            && r.worn_extra_seconds <= 86400
            && r.age_extra_seconds_per_year <= 86400
    );
    assert!(
        r.base_probability_per_million
            + r.worn_probability_per_million
            + r.maximum_age_years * r.age_probability_per_year_per_million
            <= 1_000_000
    );
    assert!((2..=365).contains(&r.reputation_response_days));
    assert!((1..1_000_000).contains(&r.reputation_neutral_otp_per_million));
    assert!((0..=500).contains(&r.reputation_max_choice_bps));
    assert!((-1_000_000..0).contains(&r.reputation_attention_score));
    assert!((0..=100_000).contains(&r.care_eur_per_passenger));
    r
});

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Leg {
    pub index: u64,
    pub scheduled: u64,
    pub departure: u64,
    pub landing: u64,
    pub ready: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub seed: u64,
    pub acquired_at: u64,
    /// Wear at initialization, before `base_index` lands.
    pub wear: Wear,
    pub base_index: u64,
    pub index: u64,
    pub previous: Option<Leg>,
    /// A dispatched quote retains its departure and arrival across migrations
    /// and changes to the holds for later departures.
    pub frozen: Option<Leg>,
    /// World weather applied to departures; `None` (old saves) is neutral.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weather: Option<crate::weather::Weather>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub staff_hold: Option<Box<crate::labour::Hold>>,
}

pub fn technical_delay(
    seed: u64,
    wear: &Wear,
    model: &str,
    scheduled: u64,
    acquired: u64,
) -> Result<u64, String> {
    let worn = 1000 - wear.condition_permille(crate::maintenance::rules(model)?);
    let age = (scheduled.saturating_sub(acquired) / crate::demand::YEAR_SECONDS)
        .min(RULES.maximum_age_years);
    let probability = RULES.base_probability_per_million
        + RULES.worn_probability_per_million * worn / 1000
        + RULES.age_probability_per_year_per_million * age;
    let roll = crate::insurance::roll(seed ^ 0x544543484e494341, wear.cycles);
    if roll % 1_000_000 >= probability {
        return Ok(0);
    }
    let maximum = RULES.maximum_seconds
        + RULES.worn_extra_seconds * worn / 1000
        + RULES.age_extra_seconds_per_year * age;
    Ok(RULES.minimum_seconds
        + crate::insurance::roll(roll, 1) % (maximum - RULES.minimum_seconds + 1))
}

impl Cursor {
    #[cfg(test)]
    pub fn next(&self, route: &Route, model: &str) -> Result<Leg, String> {
        let (flight, ground) = operations::timings(route, model)?;
        self.next_timed(route, model, flight, ground)
    }
    fn next_timed(
        &self,
        route: &Route,
        model: &str,
        flight: u64,
        ground: u64,
    ) -> Result<Leg, String> {
        if let Some(f) = &self.frozen
            && f.index == self.index
        {
            return Ok(f.clone());
        }
        let p = route.plan.as_ref().ok_or("Kein Flugplan.")?;
        let scheduled = self
            .index
            .checked_mul(p.interval_seconds)
            .and_then(|v| v.checked_add(p.first_departure))
            .ok_or("Flugplan-Zeitüberlauf.")?;
        let wear = self
            .wear
            .added(
                self.index
                    .checked_sub(self.base_index)
                    .ok_or("Ungültiger Störungscursor.")?,
                flight,
            )
            .ok_or("Verschleißüberlauf.")?;
        let delay = technical_delay(self.seed, &wear, model, scheduled, self.acquired_at)?
            + self.weather_delay(route, self.index, scheduled);
        let departure = scheduled
            .max(self.previous.as_ref().map_or(0, |l| l.ready))
            .checked_add(delay)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        let mut departure = departure;
        if let Some(h) = &self.staff_hold
            && self.index >= h.index
            && departure >= h.from
            && departure < h.until
        {
            departure = h.until;
        }
        let landing = departure
            .checked_add(flight)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        let ready = landing
            .checked_add(ground)
            .ok_or("Flugplan-Zeitüberlauf.")?;
        Ok(Leg {
            index: self.index,
            scheduled,
            departure,
            landing,
            ready,
        })
    }
    pub(crate) fn weather_delay(&self, route: &Route, index: u64, scheduled: u64) -> u64 {
        let Some(w) = &self.weather else { return 0 };
        let reversed = route.plan.as_ref().is_some_and(|p| p.is_return(index));
        let origin = if reversed {
            &route.destination_icao
        } else {
            &route.origin_icao
        };
        crate::weather::delay_seconds(w, origin, scheduled)
    }
    pub fn landed(&mut self, leg: Leg) -> Result<(), String> {
        self.index = leg.index.checked_add(1).ok_or("Landungszähler-Überlauf.")?;
        self.previous = Some(leg);
        Ok(())
    }
}

pub fn validate(route: &Route, plane: &crate::aircraft::Aircraft) -> Result<(), String> {
    let p = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    let Some(c) = &p.disruption else {
        return Ok(());
    };
    let (flight, ground) = operations::timings(route, &plane.model_id)?;
    let valid_leg = |l: &Leg| {
        l.index
            .checked_mul(p.interval_seconds)
            .and_then(|v| v.checked_add(p.first_departure))
            == Some(l.scheduled)
            && l.departure >= l.scheduled
            && l.departure.checked_add(flight) == Some(l.landing)
            && l.landing.checked_add(ground) == Some(l.ready)
    };
    if c.staff_hold.as_ref().is_some_and(|h| {
        h.from > h.until || h.until > u64::MAX / 2 || h.index > c.index.saturating_add(1)
    }) || c.index != p.completed_flights
        || c.base_index > c.index
        || c.acquired_at != plane.acquired_at.unwrap_or(0)
        || c.wear.added(c.index - c.base_index, flight).as_ref() != Some(&plane.wear)
        || c.previous
            .as_ref()
            .is_some_and(|l| !valid_leg(l) || l.index.checked_add(1) != Some(c.index))
        || (c.index > 0) != c.previous.is_some()
        || c.frozen.as_ref().is_some_and(|l| {
            !valid_leg(l)
                || l.index < c.base_index
                || l.index > c.index
                || (l.index == c.index
                    && p.airborne.as_ref().is_none_or(|f| f.landing != l.landing))
        })
    {
        return Err("Ungültiger Störungscursor im Spielstand.".into());
    }
    Ok(())
}

pub struct Projection {
    pub state: Movement,
    pub cursor: Cursor,
    pub landed: Vec<Leg>,
    pub active: Option<Leg>,
}
pub fn project(route: &Route, model: &str, now: u64) -> Result<Projection, String> {
    let p = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    let mut cursor = p.disruption.clone().ok_or("Störungscursor fehlt.")?;
    let (flight, ground) = operations::timings(route, model)?;
    let mut landed = Vec::new();
    let next = loop {
        let next = cursor.next_timed(route, model, flight, ground)?;
        if next.landing > now {
            break next;
        }
        cursor.landed(next.clone())?;
        landed.push(next);
    };
    let active = (now >= next.departure).then_some(next.clone());
    let displayed = active
        .as_ref()
        .or(cursor.previous.as_ref())
        .unwrap_or(&next);
    let (origin, destination) = if p.is_return(displayed.index) {
        (&route.destination_icao, &route.origin_icao)
    } else {
        (&route.origin_icao, &route.destination_icao)
    };
    let in_flight = active.is_some();
    let at_destination = !in_flight && cursor.previous.is_some();
    let status = if in_flight {
        AircraftStatus::InFlight
    } else if at_destination && now < displayed.ready {
        AircraftStatus::Ground
    } else {
        AircraftStatus::Parked
    };
    // During ground time show the last flight's delay. Once ready, show the
    // next flight's delay once its scheduled departure has been reached.
    let delay_leg = if in_flight || status == AircraftStatus::Ground {
        displayed
    } else {
        &next
    };
    let state = Movement {
        status,
        location: if at_destination { destination } else { origin }.clone(),
        origin: origin.clone(),
        destination: destination.clone(),
        next_departure: if in_flight {
            let mut after = cursor.clone();
            after.landed(next.clone())?;
            after.next_timed(route, model, flight, ground)?.departure
        } else {
            next.departure
        },
        landing: displayed.landing,
        ready: displayed.ready,
        completed: cursor.index,
        weather_seconds: if now < delay_leg.scheduled {
            0
        } else {
            let delay = delay_leg.departure - delay_leg.scheduled;
            cursor
                .weather_delay(route, delay_leg.index, delay_leg.scheduled)
                .min(delay)
        },
        delay_seconds: if now < delay_leg.scheduled {
            0
        } else {
            delay_leg.departure - delay_leg.scheduled
        },
    };
    Ok(Projection {
        state,
        cursor,
        landed,
        active,
    })
}

// A plan's immutable seed, baseline wear and schedule completely determine
// its future landings. Retain a few forecasts across frames; this cache is
// disposable and never serialized or consulted for settlement.
#[derive(Clone, PartialEq, Eq)]
struct ForecastKey {
    seed: u64,
    acquired: u64,
    wear: Wear,
    base: u64,
    first: u64,
    interval: u64,
    flight: u64,
    ground: u64,
    index: u64,
    frozen: Option<Leg>,
    staff_hold: Option<Box<crate::labour::Hold>>,
    weather: Option<crate::weather::Weather>,
}
type Forecasts = std::collections::HashMap<(u64, u64), (ForecastKey, u64)>;
thread_local! {
    static FORECASTS: std::cell::RefCell<Forecasts> = std::cell::RefCell::new(Forecasts::new());
    // Direct recent hits avoid hashing; collisions fall back to the existing map.
    // Every hit still checks the complete immutable schedule key.
    static RECENT_FORECASTS: std::cell::RefCell<[Option<(ForecastKey, u64)>; 64]> = const {
        std::cell::RefCell::new([const { None }; 64])
    };
}

/// Actual arrival of an outstanding landing, used by maintenance and insurance.
pub fn landing_at(route: &Route, model: &str, index: u64) -> Result<u64, String> {
    let p = route.plan.as_ref().ok_or("Kein Flugplan.")?;
    if let Some(mut cursor) = p.disruption.clone() {
        if let Some(last) = &cursor.previous
            && last.index == index
        {
            return Ok(last.landing);
        }
        let (flight, ground) = operations::timings(route, model)?;
        if cursor.index == index {
            return Ok(cursor.next_timed(route, model, flight, ground)?.landing);
        }
        let key = ForecastKey {
            seed: cursor.seed,
            acquired: cursor.acquired_at,
            wear: cursor.wear.clone(),
            base: cursor.base_index,
            first: p.first_departure,
            interval: p.interval_seconds,
            flight,
            ground,
            index,
            frozen: cursor.frozen.clone(),
            staff_hold: cursor.staff_hold.clone(),
            weather: cursor.weather,
        };
        // Hash only the plan seed and requested index on this hot path. The
        // complete immutable key still guards hits across reschedules and saves.
        let bucket = (key.seed, key.index);
        let slot = ((key.seed ^ key.index.rotate_left(17)) & 63) as usize;
        if let Some(landing) = RECENT_FORECASTS.with(|recent| {
            recent.borrow()[slot]
                .as_ref()
                .filter(|(stored, _)| *stored == key)
                .map(|(_, landing)| *landing)
        }) {
            return Ok(landing);
        }
        if let Some(landing) = FORECASTS.with(|f| {
            f.borrow()
                .get(&bucket)
                .filter(|(stored, _)| *stored == key)
                .map(|(_, landing)| *landing)
        }) {
            RECENT_FORECASTS.with(|recent| recent.borrow_mut()[slot] = Some((key, landing)));
            return Ok(landing);
        }
        while cursor.index <= index {
            let leg = cursor.next_timed(route, model, flight, ground)?;
            if leg.index == index {
                RECENT_FORECASTS.with(|recent| {
                    recent.borrow_mut()[slot] = Some((key.clone(), leg.landing));
                });
                FORECASTS.with(|f| {
                    let mut f = f.borrow_mut();
                    if f.len() >= 128 {
                        f.clear();
                    }
                    f.insert(bucket, (key, leg.landing));
                });
                return Ok(leg.landing);
            }
            cursor.landed(leg)?;
        }
        return Err("Vergangene Landung fehlt.".into());
    }
    p.first_departure
        .checked_add(
            index
                .checked_mul(p.interval_seconds)
                .ok_or("Zeitüberlauf.")?,
        )
        .and_then(|v| v.checked_add(operations::timings(route, model).ok()?.0))
        .ok_or("Zeitüberlauf.".into())
}

pub fn valid(t: &Totals) -> bool {
    t.care >= 0
        && t.punctuality_flights >= 0
        && t.punctuality_flights <= t.flights
        && t.late_flights >= 0
        && t.late_flights <= t.punctuality_flights
        && t.delay_seconds >= 0
}
pub fn percent(t: &Totals) -> f64 {
    if t.punctuality_flights == 0 {
        -1.0
    } else {
        (t.punctuality_flights - t.late_flights) as f64 * 100.0 / t.punctuality_flights as f64
    }
}
pub fn apply(t: &mut Totals, leg: &Leg) -> Result<(), String> {
    let delay = leg.departure - leg.scheduled;
    t.punctuality_flights = i64::from(t.seats > 0);
    t.late_flights = i64::from(t.seats > 0 && delay > RULES.on_time_threshold_seconds);
    t.delay_seconds = i64::try_from(delay).map_err(|_| "Verspätungsüberlauf.")?;
    if delay > RULES.care_threshold_seconds {
        t.care = t
            .passengers
            .checked_mul(RULES.care_eur_per_passenger)
            .ok_or("Finanzüberlauf.")?;
    }
    Ok(())
}

impl Game {
    /// Old plans have no weather context; they get the world's, starting at `weather_from`.
    pub(crate) fn adopt_weather(&mut self) {
        let weather = crate::weather::Weather::of(&self.world_economy);
        for r in &mut self.routes {
            if let Some(c) = r.plan.as_mut().and_then(|p| p.disruption.as_mut()) {
                c.weather.get_or_insert(weather);
            }
        }
        for c in &mut self.competitors {
            c.game.adopt_weather();
        }
    }
    pub(crate) fn initialize_disruptions(&mut self) -> Result<(), String> {
        for i in 0..self.routes.len() {
            let route = &self.routes[i];
            let Some(p) = &route.plan else {
                continue;
            };
            if p.disruption.is_some() {
                continue;
            }
            let plane = self
                .fleet
                .iter()
                .find(|a| Some(a.id) == route.aircraft_id)
                .ok_or("Flugzeug fehlt.")?;
            let (flight, ground) = operations::timings(route, &plane.model_id)?;
            let leg = |index: u64| -> Result<Leg, String> {
                let scheduled = p
                    .first_departure
                    .checked_add(
                        index
                            .checked_mul(p.interval_seconds)
                            .ok_or("Zeitüberlauf.")?,
                    )
                    .ok_or("Zeitüberlauf.")?;
                let landing = scheduled.checked_add(flight).ok_or("Zeitüberlauf.")?;
                Ok(Leg {
                    index,
                    scheduled,
                    departure: scheduled,
                    landing,
                    ready: landing.checked_add(ground).ok_or("Zeitüberlauf.")?,
                })
            };
            let cursor = Cursor {
                staff_hold: None,
                weather: Some(crate::weather::Weather::of(&self.world_economy)),
                seed: self.incident_seed(plane.id),
                acquired_at: plane.acquired_at.unwrap_or(0),
                wear: plane.wear.clone(),
                base_index: p.completed_flights,
                index: p.completed_flights,
                previous: p.completed_flights.checked_sub(1).map(leg).transpose()?,
                frozen: p
                    .airborne
                    .as_ref()
                    .map(|_| leg(p.completed_flights))
                    .transpose()?,
            };
            self.routes[i].plan.as_mut().unwrap().disruption = Some(cursor);
        }
        Ok(())
    }
}
