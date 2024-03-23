use crate::{led_state::PzbLedState, pzb::PzbCategory};

#[derive(Copy, Clone)]
pub enum PzbState {
	Free, // Static number
	Restricted1000hz, // Blinking number and static 1000hz
	Restricted500hz, // Blinking number and static 500hz
	Supervised, // Alternating blinking 70 and 85
	ForcedStop,
}

impl PzbState {
	pub const fn enabled(self, state: PzbCategory) -> PzbLedState {
		let led = PzbLedState::off();
		match self {
			PzbState::Free => {state.set_led(led, false)},
			PzbState::Restricted1000hz => {state.set_led(led, true).hz1000()},
			PzbState::Restricted500hz => {state.set_led(led, true).hz500()},
			PzbState::Supervised => {led.b85_blink(true).b70_blink(false)}
			PzbState::ForcedStop => led.hz1000_blink(false).hz500_blink(false),
		}
	}

	// Not proud but it works alright
	pub fn next(self) -> Self {
		match self {
			PzbState::Free => Self::Restricted1000hz,
			PzbState::Restricted1000hz => Self::Restricted500hz,
			PzbState::Restricted500hz => Self::Supervised,
			PzbState::Supervised => Self::ForcedStop,
			PzbState::ForcedStop => Self::Free,
		}
	}
}
