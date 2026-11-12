mod clock;

use clock::Clock;
use godot::prelude::*;

struct AirlineSimulation;

// SAFETY: All extension classes are registered through godot-rust's macros.
#[gdextension]
unsafe impl ExtensionLibrary for AirlineSimulation {}

/// Thin Godot adapter. All simulation decisions live in the engine-independent Clock.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct SimulationClock {
    clock: Clock,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for SimulationClock {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            clock: Clock::default(),
            base,
        }
    }
}

#[godot_api]
impl SimulationClock {
    #[func]
    fn start(&mut self) {
        self.clock.set_running(true);
    }

    #[func]
    fn pause(&mut self) {
        self.clock.set_running(false);
    }

    #[func]
    fn is_running(&self) -> bool {
        self.clock.is_running()
    }

    #[func]
    fn set_speed(&mut self, multiplier: i64) -> bool {
        u32::try_from(multiplier).is_ok_and(|speed| self.clock.set_speed(speed))
    }

    #[func]
    fn get_speed(&self) -> i64 {
        i64::from(self.clock.speed())
    }

    #[func]
    fn advance(&mut self, real_seconds: f64) -> bool {
        self.clock.advance(real_seconds)
    }

    #[func]
    fn get_elapsed_seconds(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    #[func]
    fn get_display_time(&self) -> GString {
        self.clock.display_time().as_str().into()
    }
}
