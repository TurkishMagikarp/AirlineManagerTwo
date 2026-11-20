//! Explicit simulation-time advances through the existing event loop.
use crate::{attention::Entry, clock::Clock, game::Game};
use serde::Deserialize;
use std::{sync::LazyLock, time::Duration};

pub type WorkerResult = (Game, Result<ResultSummary, &'static str>);

const DAY: u64 = 86400;
pub const STALE_TARGET: &str =
    "Der nächste Termin hat sich geändert. Bitte erneut auswählen.";
#[derive(Deserialize)]
pub struct OptionRule {
    pub label: String,
    pub seconds: u64,
}
#[derive(Deserialize)]
pub struct Rules {
    pub options: Vec<OptionRule>,
}
pub static RULES: LazyLock<Rules> = LazyLock::new(|| {
    let r: Rules =
        serde_json::from_str(include_str!("../../config/jumps.json")).expect("jump config");
    assert!(!r.options.is_empty());
    assert!(
        r.options
            .iter()
            .all(|o| !o.label.is_empty() && o.seconds > 0 && o.seconds <= i64::MAX as u64)
    );
    r
});

#[derive(Debug)]
pub struct ResultSummary {
    pub advanced_seconds: u64,
    pub reason: String,
}
impl ResultSummary {
    pub fn message(&self) -> String {
        let stopped = !self.reason.is_empty();
        let seconds = self.advanced_seconds;
        let duration = if seconds > 0 && seconds.is_multiple_of(DAY) {
            let days = seconds / DAY;
            format!(
                "{days} {}",
                if days == 1 {
                    "Tag"
                } else if stopped {
                    "Tagen"
                } else {
                    "Tage"
                }
            )
        } else if seconds > 0 && seconds.is_multiple_of(3600) {
            let hours = seconds / 3600;
            format!("{hours} {}", if hours == 1 { "Stunde" } else { "Stunden" })
        } else {
            format!("{seconds} Sekunden")
        };
        if self.reason.is_empty() {
            format!("Sprung um {duration} abgeschlossen.")
        } else {
            format!("Sprung nach {duration} angehalten: {}", self.reason)
        }
    }
}

// Text contains changing figures; identity is the existing attention kind/target.
fn same_entry(a: &Entry, b: &Entry) -> bool {
    (
        a.kind,
        a.page,
        a.route_id,
        a.aircraft_id,
        a.employee_id,
        a.order_id,
    ) == (
        b.kind,
        b.page,
        b.route_id,
        b.aircraft_id,
        b.employee_id,
        b.order_id,
    )
}

impl Game {
    /// Invalid inputs or event-loop failures leave all owners untouched. No jump
    /// state is persisted: save/load between jumps uses the ordinary game state.
    pub fn jump(&mut self, game_seconds: f64) -> Result<ResultSummary, &'static str> {
        let error = "Zeitsprung abgelehnt: ungültige Dauer, fehlende Airline oder Zeitüberlauf.";
        if self.airline.is_none()
            || self.insolvent()
            || !game_seconds.is_finite()
            || game_seconds <= 0.0
            || game_seconds.fract() != 0.0
        {
            return Err(error);
        }
        let step = Duration::try_from_secs_f64(game_seconds).map_err(|_| error)?;
        // Events use u64 seconds, and UI results use signed integers.
        let target = self.clock.elapsed().checked_add(step).ok_or(error)?;
        if step.as_secs() > i64::MAX as u64 || target.as_secs() > i64::MAX as u64 {
            return Err(error);
        }
        self.jump_until(target)
    }

    /// First future commitment, if it lies within the longest jump preset.
    /// Held orders are already excluded by `upcoming_commitments`.
    pub fn next_commitment_jump(&self) -> Option<crate::orders::Commitment> {
        let now = self.clock.elapsed();
        let longest = RULES.options.iter().map(|o| o.seconds).max()?;
        let next = self
            .upcoming_commitments()
            .into_iter()
            .find(|c| Duration::from_secs(c.at) > now)?;
        (Duration::from_secs(next.at) - now <= Duration::from_secs(longest)).then_some(next)
    }

    /// The target is the timestamp the UI displayed; a changed next commitment
    /// is rejected without advancing.
    pub fn jump_to_commitment(&mut self, at: u64) -> Result<ResultSummary, &'static str> {
        if self.next_commitment_jump().is_none_or(|c| c.at != at) {
            return Err(STALE_TARGET);
        }
        self.jump_until(Duration::from_secs(at))
    }

    fn jump_until(&mut self, target: Duration) -> Result<ResultSummary, &'static str> {
        let error = "Zeitsprung abgelehnt: ungültige Dauer, fehlende Airline oder Zeitüberlauf.";
        if self.airline.is_none() || self.insolvent() || target <= self.clock.elapsed() {
            return Err(error);
        }
        let start = self.clock.elapsed();
        let running = self.clock.is_running();
        let speed = self.clock.speed();
        let mut staged = self.clone();
        let mut previous = staged.attention_snapshot();
        let mut distressed = staged.distressed();
        staged.clock = Clock::from_state(start, true, 1).ok_or(error)?;
        let mut reason = String::new();
        while staged.clock.elapsed() < target {
            let mut chunk = (target - staged.clock.elapsed()).min(Duration::from_secs(DAY));
            if let Some(event) = staged.next_order_event() {
                let until = Duration::from_secs(event).saturating_sub(staged.clock.elapsed());
                if !until.is_zero() {
                    chunk = chunk.min(until);
                }
            }
            // A forced check can start and end inside one chunk: end the chunk at the grounding.
            let now = staged.clock.elapsed().as_secs();
            if let Some(due) = (0..staged.fleet.len())
                .filter_map(|p| staged.due_landing(p))
                .min()
            {
                let until = Duration::from_secs(due.saturating_sub(now).max(1));
                chunk = chunk.min(until);
            }
            let expected = staged.clock.elapsed() + chunk;
            if !staged.advance(chunk.as_secs_f64()) {
                return Err(error);
            }
            // The f64 round trip may miss the intended end by nanoseconds.
            let reached = staged.clock.elapsed();
            if reached != expected && reached.abs_diff(expected) <= Duration::from_micros(1) {
                staged.clock = Clock::from_state(expected, true, 1).ok_or(error)?;
            }
            let entries = staged.attention_snapshot();
            if staged.insolvent() || (!distressed && staged.distressed()) {
                reason = staged.financing_status().into();
            } else if let Some(entry) = entries
                .iter()
                .find(|e| !previous.iter().any(|p| same_entry(e, p)))
            {
                reason = entry.text.clone();
            }
            if !reason.is_empty() {
                break;
            }
            distressed = staged.distressed();
            previous = entries;
        }
        let advanced_seconds = (staged.clock.elapsed() - start).as_secs();
        // Insolvency is terminal and must retain the ordinary engine's pause.
        staged.clock = Clock::from_state(
            staged.clock.elapsed(),
            running && !staged.insolvent(),
            speed,
        )
        .ok_or(error)?;
        *self = staged;
        Ok(ResultSummary {
            advanced_seconds,
            reason,
        })
    }
}
