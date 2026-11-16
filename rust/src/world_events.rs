//! Regional demand events (airport booms, country slumps). Every event is a pure
//! function of the save's seed, a kind, a lane and an absolute slot number, so
//! no random state is stored and query order, speed and step size are irrelevant.
//! Slots are longer than the longest event: events of one lane never overlap.
use crate::world_economy::Cycle;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, sync::LazyLock};

pub const DAY: u64 = 86400;

#[derive(Deserialize)]
pub struct Kind {
    pub lanes: u64,
    pub slot_days: u64,
    pub chance_permille: u64,
    pub min_days: u64,
    pub max_days: u64,
    pub warning_days: u64,
    pub min_effect_bps: i64,
    pub max_effect_bps: i64,
    pub business_reaction: f64,
    pub leisure_reaction: f64,
    pub cargo_reaction: f64,
}
#[derive(Deserialize)]
pub struct Rules {
    pub enabled: bool,
    pub size_weight_power: u32,
    pub boom: Kind,
    pub slump: Kind,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/world_events.json"))
        .expect("world events config");
    assert!((1..=4).contains(&r.size_weight_power));
    for (kind, sign) in [(&r.boom, 1.0), (&r.slump, -1.0)] {
        assert!(kind.lanes <= 16 && kind.chance_permille <= 1000);
        assert!(0 < kind.min_days && kind.min_days <= kind.max_days);
        assert!(kind.max_days <= kind.slot_days && kind.warning_days <= 90);
        assert!(0 < kind.min_effect_bps && kind.min_effect_bps <= kind.max_effect_bps);
        for reaction in [
            kind.business_reaction,
            kind.leisure_reaction,
            kind.cargo_reaction,
        ] {
            assert!(reaction.is_finite() && (0.0..=3.0).contains(&reaction));
            assert!(1.0 + sign * kind.max_effect_bps as f64 / 10000.0 * reaction > 0.0);
        }
    }
    r
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Boom,
    Slump,
}
impl Type {
    pub fn key(self) -> &'static str {
        match self {
            Type::Boom => "boom",
            Type::Slump => "slump",
        }
    }
    fn rules(self) -> &'static Kind {
        match self {
            Type::Boom => &RULES.boom,
            Type::Slump => &RULES.slump,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Airport(&'static str),
    Country(&'static str),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    /// Stable identity: kind, lane and slot.
    pub id: u64,
    pub kind: Type,
    pub place: Place,
    pub start_day: u64,
    pub end_day: u64,
    pub effect_bps: i64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Factors {
    pub business: f64,
    pub leisure: f64,
    pub cargo: f64,
}
impl Event {
    pub fn announced_at(&self) -> u64 {
        self.start_day.saturating_sub(self.kind.rules().warning_days) * DAY
    }
    pub fn starts_at(&self) -> u64 {
        self.start_day * DAY
    }
    pub fn ends_at(&self) -> u64 {
        self.end_day * DAY
    }
    pub fn is_active(&self, time: u64) -> bool {
        self.starts_at() <= time && time < self.ends_at()
    }
    pub fn factors(&self) -> Factors {
        let r = self.kind.rules();
        #[cfg(test)]
        let zero = ZERO_REACTIONS.with(|v| v.get());
        #[cfg(not(test))]
        let zero = false;
        let effect = self.effect_bps as f64 / 10000.0
            * if self.kind == Type::Boom { 1.0 } else { -1.0 };
        let f = |reaction: f64| 1.0 + if zero { 0.0 } else { effect * reaction };
        Factors {
            business: f(r.business_reaction),
            leisure: f(r.leisure_reaction),
            cargo: f(r.cargo_reaction),
        }
    }
    pub fn place_name(&self) -> String {
        let Ok(catalog) = crate::airports::catalog() else {
            return String::new();
        };
        match self.place {
            Place::Airport(icao) => catalog
                .airports
                .iter()
                .find(|a| a.icao == icao)
                .map_or(String::new(), |a| format!("{} ({icao})", a.name)),
            Place::Country(code) => catalog
                .countries
                .iter()
                .find(|c| c.code == code)
                .map_or(String::new(), |c| c.name.clone()),
        }
    }
    pub fn text(&self) -> String {
        let title = match self.kind {
            Type::Boom => "Nachfrageschub",
            Type::Slump => "Nachfrageflaute",
        };
        format!(
            "Ereignis · {title} · {} · {} bis {} · {:+} % Nachfrage",
            self.place_name(),
            crate::operations::display_time(self.starts_at()),
            crate::operations::display_time(self.ends_at()),
            match self.kind {
                Type::Boom => self.effect_bps / 100,
                Type::Slump => -self.effect_bps / 100,
            }
        )
    }
    pub fn touches(&self, icao: &str) -> bool {
        match self.place {
            Place::Airport(a) => a == icao,
            Place::Country(code) => crate::airports::catalog()
                .ok()
                .and_then(|c| c.airports.iter().find(|a| a.icao == icao))
                .is_some_and(|a| a.country_code == code),
        }
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static ZERO_REACTIONS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub(crate) static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}
fn enabled() -> bool {
    #[cfg(test)]
    if DISABLED.with(|v| v.get()) {
        return false;
    }
    RULES.enabled
        && (RULES.boom.lanes > 0 && RULES.boom.chance_permille > 0
            || RULES.slump.lanes > 0 && RULES.slump.chance_permille > 0)
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
struct Draws(u64);
impl Draws {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        mix(self.0)
    }
}

fn weight(size: f64) -> u64 {
    ((size * 100.0).round().max(1.0) as u64).pow(RULES.size_weight_power)
}
fn pick<T: Copy>(items: impl Iterator<Item = (T, u64)>, draw: u64) -> Option<T> {
    let items: Vec<_> = items.collect();
    let total: u64 = items.iter().map(|(_, w)| w).sum();
    if total == 0 {
        return None;
    }
    let mut r = draw % total;
    for (item, w) in items {
        if r < w {
            return Some(item);
        }
        r -= w;
    }
    None
}

fn derive(cycle: &Cycle, kind: Type, lane: u64, slot: u64) -> Option<Event> {
    let rules = kind.rules();
    let mut d = Draws(mix(
        cycle.seed ^ 0x45564e5453 ^ ((kind as u64 + 1) << 56) ^ (lane << 48),
    ) ^ slot.wrapping_mul(0xd6e8feb86659fd93));
    let chance = d.next() % 1000;
    let duration = rules.min_days + d.next() % (rules.max_days - rules.min_days + 1);
    let offset = d.next() % (rules.slot_days - duration + 1);
    let effect_bps = rules.min_effect_bps
        + (d.next() % (rules.max_effect_bps - rules.min_effect_bps + 1) as u64) as i64;
    let target = d.next();
    if chance >= rules.chance_permille {
        return None;
    }
    let catalog = crate::airports::catalog().ok()?;
    let place = match kind {
        Type::Boom => Place::Airport(pick(
            catalog
                .airports
                .iter()
                .map(|a| (a.icao.as_str(), weight(a.market_size))),
            target,
        )?),
        Type::Slump => Place::Country(pick(
            catalog.countries.iter().map(|c| {
                (
                    c.code.as_str(),
                    catalog
                        .airports
                        .iter()
                        .filter(|a| a.country_code == c.code)
                        .map(|a| weight(a.market_size))
                        .sum(),
                )
            }),
            target,
        )?),
    };
    let start_day = slot * rules.slot_days + offset;
    let event = Event {
        id: ((kind as u64 + 1) << 56) | (lane << 48) | slot,
        kind,
        place,
        start_day,
        end_day: start_day + duration,
        effect_bps,
    };
    // A world loaded from an older save only learns of events announced after loading.
    (event.announced_at() >= cycle.events_from).then_some(event)
}

/// Events announced and not yet over at `time`, ordered by start and identity.
pub fn visible_at(cycle: &Cycle, time: u64) -> Vec<Event> {
    let mut events = Vec::new();
    if !enabled() {
        return events;
    }
    let day = time / DAY;
    for kind in [Type::Boom, Type::Slump] {
        let rules = kind.rules();
        if rules.chance_permille == 0 {
            continue;
        }
        for lane in 0..rules.lanes {
            for slot in day / rules.slot_days..=(day + rules.warning_days) / rules.slot_days {
                events.extend(
                    derive(cycle, kind, lane, slot)
                        .filter(|e| e.announced_at() <= time && time < e.ends_at()),
                );
            }
        }
    }
    events.sort_by_key(|e| (e.start_day, e.id));
    events
}

struct DayCache {
    key: (u64, u64, u64),
    factors: Vec<Factors>,
    by_airport: HashMap<&'static str, Vec<usize>>,
}
thread_local! {
    static DAY_CACHE: RefCell<Option<DayCache>> = const { RefCell::new(None) };
}
#[cfg(test)]
thread_local! { pub(crate) static BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

fn build(cycle: &Cycle, day: u64) -> DayCache {
    #[cfg(test)]
    BUILDS.with(|b| b.set(b.get() + 1));
    let mut cache = DayCache {
        key: (cycle.seed, cycle.events_from, day),
        factors: Vec::new(),
        by_airport: HashMap::new(),
    };
    let airports = crate::airports::catalog().map(|c| &c.airports[..]).unwrap_or(&[]);
    for e in visible_at(cycle, day * DAY)
        .into_iter()
        .filter(|e| e.is_active(day * DAY))
    {
        let index = cache.factors.len();
        cache.factors.push(e.factors());
        for a in airports.iter().filter(|a| match e.place {
            Place::Airport(icao) => a.icao == icao,
            Place::Country(code) => a.country_code == code,
        }) {
            cache.by_airport.entry(a.icao.as_str()).or_default().push(index);
        }
    }
    cache
}

/// Combined factor of all events active on the whole day containing `time` that
/// touch either end of the route. `None` means exactly neutral.
pub fn route_factors(cycle: &Cycle, time: u64, origin: &str, destination: &str) -> Option<Factors> {
    if !enabled() {
        return None;
    }
    let key = (cycle.seed, cycle.events_from, time / DAY);
    DAY_CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|c| c.key != key) {
            *slot = Some(build(cycle, key.2));
        }
        let cache = slot.as_ref().unwrap();
        if cache.by_airport.is_empty() {
            return None;
        }
        let from = cache.by_airport.get(origin);
        let to = cache.by_airport.get(destination);
        if from.is_none() && to.is_none() {
            return None;
        }
        let mut total = Factors {
            business: 1.0,
            leisure: 1.0,
            cargo: 1.0,
        };
        let from = from.map(Vec::as_slice).unwrap_or(&[]);
        let to = to.map(Vec::as_slice).unwrap_or(&[]);
        for &i in from.iter().chain(to.iter().filter(|i| !from.contains(i))) {
            let f = cache.factors[i];
            total.business *= f.business;
            total.leisure *= f.leisure;
            total.cargo *= f.cargo;
        }
        Some(total)
    })
}
