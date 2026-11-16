//! Airport weather (snow, storms, fog). Like world events, every episode is a pure
//! function of the save's world seed, airport, kind and an absolute slot number:
//! nothing random is stored, and query order, speed and step size are irrelevant.
//! Slots are longer than the longest episode, so episodes of one kind never overlap.
//! Departures from an affected airport pick up an extra delay in `disruptions`.
use crate::{airports::Airport, world_economy::Cycle};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, sync::LazyLock};

pub const DAY: u64 = 86400;
const YEAR_DAYS: u64 = 365;

#[derive(Deserialize)]
pub struct Kind {
    pub key: String,
    pub name: String,
    pub min_abs_latitude: f64,
    pub max_abs_latitude: f64,
    pub season_start_day: u64,
    pub season_end_day: u64,
    pub chance_permille: u64,
    pub min_days: u64,
    pub max_days: u64,
    pub min_delay_seconds: u64,
    pub max_delay_seconds: u64,
}
#[derive(Deserialize)]
pub struct Rules {
    pub enabled: bool,
    pub warning_days: u64,
    pub slot_days: u64,
    pub southern_shift_days: u64,
    pub kinds: Vec<Kind>,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules = serde_json::from_str(include_str!("../../config/weather.json"))
        .expect("weather config");
    assert!((1..=14).contains(&r.warning_days) && (1..=30).contains(&r.slot_days));
    assert!(r.southern_shift_days < YEAR_DAYS && (1..=8).contains(&r.kinds.len()));
    for k in &r.kinds {
        assert!(0.0 <= k.min_abs_latitude && k.min_abs_latitude <= k.max_abs_latitude);
        assert!(k.max_abs_latitude <= 90.0);
        assert!(k.season_start_day < YEAR_DAYS && k.season_end_day < YEAR_DAYS);
        assert!(k.chance_permille <= 1000);
        assert!(0 < k.min_days && k.min_days <= k.max_days && k.max_days <= r.slot_days);
        assert!(k.min_delay_seconds <= k.max_delay_seconds && k.max_delay_seconds <= 6 * 3600);
    }
    r
});

/// World weather context kept in every plan's disruption cursor. Episodes whose
/// announcement lies before `from` are never applied (no retroactive weather).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Weather {
    pub seed: u64,
    pub from: u64,
}
impl Weather {
    pub fn of(cycle: &Cycle) -> Self {
        Self {
            seed: cycle.seed,
            from: cycle.weather_from,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Episode {
    /// Stable identity: kind, airport and slot.
    pub id: u64,
    pub kind: &'static Kind,
    pub icao: &'static str,
    pub start_day: u64,
    pub end_day: u64,
    pub delay_seconds: u64,
}
impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Kind {}
impl std::fmt::Debug for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.key)
    }
}
impl Episode {
    pub fn announced_at(&self) -> u64 {
        self.start_day.saturating_sub(RULES.warning_days) * DAY
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
    pub fn place_name(&self) -> String {
        crate::airports::catalog()
            .ok()
            .and_then(|c| c.airports.iter().find(|a| a.icao == self.icao))
            .map_or(String::new(), |a| format!("{} ({})", a.name, a.icao))
    }
    pub fn attention_text(&self) -> String {
        format!(
            "Wetter in {}: {}, {} bis {} · bis zu {} min Verspätung bei Abflügen",
            self.place_name(),
            self.kind.name,
            crate::operations::display_time(self.starts_at()),
            crate::operations::display_time(self.ends_at()),
            self.delay_seconds.div_ceil(60)
        )
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}
fn enabled() -> bool {
    #[cfg(test)]
    if DISABLED.with(|v| v.get()) {
        return false;
    }
    RULES.enabled && RULES.kinds.iter().any(|k| k.chance_permille > 0)
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
fn icao_hash(icao: &str) -> u64 {
    icao.bytes()
        .fold(0xcbf29ce484222325u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100000001b3))
}

fn in_season(kind: &Kind, latitude: f64, day: u64) -> bool {
    let shift = if latitude < 0.0 { RULES.southern_shift_days } else { 0 };
    let doy = (day + YEAR_DAYS - shift) % YEAR_DAYS;
    let (a, b) = (kind.season_start_day, kind.season_end_day);
    if a <= b {
        (a..b).contains(&doy)
    } else {
        doy >= a || doy < b
    }
}
fn applies(kind: &Kind, latitude: f64) -> bool {
    (kind.min_abs_latitude..=kind.max_abs_latitude).contains(&latitude.abs())
}

fn derive(seed: u64, index: usize, airport: &'static Airport, slot: u64) -> Option<Episode> {
    let kind = &RULES.kinds[index];
    if !applies(kind, airport.latitude) {
        return None;
    }
    let mut d = Draws(
        mix(seed ^ 0x57455448 ^ ((index as u64 + 1) << 56) ^ icao_hash(&airport.icao).rotate_left(8))
            ^ slot.wrapping_mul(0xd6e8feb86659fd93),
    );
    let chance = d.next() % 1000;
    let duration = kind.min_days + d.next() % (kind.max_days - kind.min_days + 1);
    let offset = d.next() % (RULES.slot_days - duration + 1);
    let delay = kind.min_delay_seconds + d.next() % (kind.max_delay_seconds - kind.min_delay_seconds + 1);
    let start_day = slot * RULES.slot_days + offset;
    if chance >= kind.chance_permille || !in_season(kind, airport.latitude, start_day) {
        return None;
    }
    Some(Episode {
        id: ((index as u64 + 1) << 56) | ((icao_hash(&airport.icao) & 0xffff_ffff) << 16) | (slot & 0xffff),
        kind,
        icao: &airport.icao,
        start_day,
        end_day: start_day + duration,
        delay_seconds: delay,
    })
}
fn learnt(w: &Weather, e: &Episode) -> bool {
    e.announced_at() >= w.from
}

type DayKey = (usize, Weather, u64);
type LegKey = (usize, Weather, u64, u64);
type LegSlot = Option<(LegKey, u64)>;

struct DayCache {
    key: DayKey,
    episodes: Vec<Episode>,
}
thread_local! {
    static DAY_CACHE: RefCell<Option<DayCache>> = const { RefCell::new(None) };
    static LEG_CACHE: RefCell<[LegSlot; 256]> =
        const { RefCell::new([None; 256]) };
}
#[cfg(test)]
thread_local! { pub(crate) static BUILDS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

/// Episodes announced and not yet over on the whole day containing `time`,
/// ordered by start and identity. Changes only at day boundaries.
pub fn visible_at(w: &Weather, time: u64) -> Vec<Episode> {
    if !enabled() {
        return Vec::new();
    }
    let Ok(catalog) = crate::airports::catalog() else {
        return Vec::new();
    };
    let day = time / DAY;
    let key = (catalog as *const _ as usize, *w, day);
    DAY_CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|c| c.key != key) {
            #[cfg(test)]
            BUILDS.with(|b| b.set(b.get() + 1));
            let mut episodes = Vec::new();
            for airport in &catalog.airports {
                for index in 0..RULES.kinds.len() {
                    for s in day / RULES.slot_days..=(day + RULES.warning_days) / RULES.slot_days {
                        episodes.extend(derive(w.seed, index, airport, s).filter(|e| {
                            learnt(w, e) && e.announced_at() <= day * DAY && day * DAY < e.ends_at()
                        }));
                    }
                }
            }
            episodes.sort_by_key(|e| (e.start_day, e.id, e.icao));
            *slot = Some(DayCache { key, episodes });
        }
        slot.as_ref().unwrap().episodes.clone()
    })
}

/// Extra departure delay at `icao` for a departure scheduled at `scheduled`:
/// the strongest episode active on that day, zero when none (or disabled).
pub fn delay_seconds(w: &Weather, icao: &str, scheduled: u64) -> u64 {
    if !enabled() {
        return 0;
    }
    let Ok(catalog) = crate::airports::catalog() else {
        return 0;
    };
    let day = scheduled / DAY;
    let hash = icao_hash(icao);
    let key = (catalog as *const _ as usize, *w, hash, day);
    let slot = (mix(hash ^ day.wrapping_mul(0x9e3779b97f4a7c15)) & 255) as usize;
    if let Some(hit) = LEG_CACHE.with(|c| c.borrow()[slot].filter(|(k, _)| *k == key)) {
        return hit.1;
    }
    let delay = catalog
        .airports
        .iter()
        .find(|a| a.icao == icao)
        .map_or(0, |airport| {
            (0..RULES.kinds.len())
                .filter_map(|index| derive(w.seed, index, airport, day / RULES.slot_days))
                .filter(|e| learnt(w, e) && (e.start_day..e.end_day).contains(&day))
                .map(|e| e.delay_seconds)
                .max()
                .unwrap_or(0)
        });
    LEG_CACHE.with(|c| c.borrow_mut()[slot] = Some((key, delay)));
    delay
}
