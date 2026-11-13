use std::time::Duration;

/// Simulation time only. No wall clock, engine dependency or UI state.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Clock {
    elapsed: Duration,
    running: bool,
    speed_index: usize,
}

impl Clock {
    pub const SPEEDS: [u32; 3] = [1, 4, 16];

    pub fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn set_speed(&mut self, speed: u32) -> bool {
        if let Some(index) = Self::SPEEDS.iter().position(|&value| value == speed) {
            self.speed_index = index;
            true
        } else {
            false
        }
    }

    pub fn speed(&self) -> u32 {
        Self::SPEEDS[self.speed_index]
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.speed_index < Self::SPEEDS.len()
    }

    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    pub fn from_state(elapsed: Duration, running: bool, speed: u32) -> Option<Self> {
        let mut clock = Self {
            elapsed,
            running,
            ..Self::default()
        };
        clock.set_speed(speed).then_some(clock)
    }

    /// Godot supplies real frame duration; Rust alone decides time progression.
    /// Invalid inputs and overflow leave the clock unchanged, never panicking over FFI.
    pub fn advance(&mut self, real_seconds: f64) -> bool {
        if !real_seconds.is_finite() || real_seconds < 0.0 {
            return false;
        }
        if !self.running {
            return true;
        }
        let Ok(step) = Duration::try_from_secs_f64(real_seconds * f64::from(self.speed())) else {
            return false;
        };
        let Some(next) = self.elapsed.checked_add(step) else {
            return false;
        };
        self.elapsed = next;
        true
    }

    pub fn display_time(&self) -> String {
        let seconds = self.elapsed.as_secs();
        format!(
            "Tag {} · {:02}:{:02}:{:02}",
            seconds / 86_400 + 1,
            seconds / 3_600 % 24,
            seconds / 60 % 60,
            seconds % 60
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_paused_and_resumes_without_reset() {
        let mut clock = Clock::default();
        assert!(!clock.is_running());
        assert_eq!(clock.speed(), 1);
        assert!(clock.advance(10.0));
        assert_eq!(clock.elapsed(), Duration::ZERO);
        clock.set_running(true);
        assert!(clock.advance(1.25));
        clock.set_running(false);
        assert!(clock.advance(100.0));
        assert_eq!(clock.elapsed(), Duration::from_millis(1250));
        clock.set_running(true);
        clock.advance(0.75);
        assert_eq!(clock.elapsed(), Duration::from_secs(2));
    }

    #[test]
    fn every_speed_and_changes_while_paused() {
        let mut clock = Clock::default();
        for speed in Clock::SPEEDS {
            clock.set_running(false);
            assert!(clock.set_speed(speed));
            let before = clock.elapsed();
            clock.advance(3.0);
            assert_eq!(clock.elapsed(), before);
            clock.set_running(true);
            clock.advance(0.5);
            assert_eq!(
                clock.elapsed() - before,
                Duration::from_secs_f64(0.5 * f64::from(speed))
            );
        }
    }

    #[test]
    fn rejects_invalid_values_without_mutating_state() {
        let mut clock = Clock::default();
        clock.set_running(true);
        for speed in [0, 2, u32::MAX] {
            assert!(!clock.set_speed(speed));
            assert_eq!(clock.speed(), 1);
        }
        for delta in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
            assert!(!clock.advance(delta));
            assert_eq!(clock.elapsed(), Duration::ZERO);
        }
        clock.elapsed = Duration::MAX;
        assert!(!clock.advance(1.0));
        assert_eq!(clock.elapsed(), Duration::MAX);
    }

    #[test]
    fn fractional_frames_and_day_rollover() {
        let mut clock = Clock::default();
        assert_eq!(clock.display_time(), "Tag 1 · 00:00:00");
        clock.set_running(true);
        for _ in 0..8 {
            clock.advance(0.125);
        }
        assert_eq!(clock.elapsed(), Duration::from_secs(1));
        clock.advance(86_399.0);
        assert_eq!(clock.display_time(), "Tag 2 · 00:00:00");
        clock.advance(3_661.0);
        assert_eq!(clock.display_time(), "Tag 2 · 01:01:01");
    }
}
