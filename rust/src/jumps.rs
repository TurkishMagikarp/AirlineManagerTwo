//! Explicit simulation-time advances through the existing event loop.
use crate::{attention::Entry, clock::Clock, game::Game};
use serde::Deserialize;
use std::{sync::LazyLock, time::Duration};

pub type WorkerResult = (Game, Result<ResultSummary, &'static str>);

const DAY: u64 = 86400;
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
    (a.kind, a.page, a.route_id, a.aircraft_id, a.employee_id)
        == (b.kind, b.page, b.route_id, b.aircraft_id, b.employee_id)
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
        let start = self.clock.elapsed();
        let target = start.checked_add(step).ok_or(error)?;
        if step.as_secs() > i64::MAX as u64 || target.as_secs() > i64::MAX as u64 {
            return Err(error);
        }
        let running = self.clock.is_running();
        let speed = self.clock.speed();
        let mut staged = self.clone();
        let mut previous = staged.attention_snapshot();
        let mut distressed = staged.distressed();
        staged.clock = Clock::from_state(start, true, 1).ok_or(error)?;
        let mut reason = String::new();
        while staged.clock.elapsed() < target {
            let chunk = (target - staged.clock.elapsed()).min(Duration::from_secs(DAY));
            if !staged.advance(chunk.as_secs_f64()) {
                return Err(error);
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
