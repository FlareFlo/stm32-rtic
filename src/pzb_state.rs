use crate::{led_state::PzbLedState, pzb::PzbCategory};

#[derive(Copy, Clone)]
pub enum PzbState {
	Free,
	Restricted1000hz,
	Restricted500hz,
	ForcedStop,
}

impl PzbState {
	pub const fn enabled(self, state: PzbCategory) -> PzbLedState {
		let led = PzbLedState::off();
		match self {
			PzbState::Free => match state {
				PzbCategory::O => led.b85(),
				PzbCategory::M => led.b70(),
				PzbCategory::U => led.b55(),
			},
			PzbState::Restricted1000hz => led.hz1000().b85(),
			PzbState::Restricted500hz => led.hz500().b85(),
			PzbState::ForcedStop => led.hz1000_blink().hz500_blink(),
		}
	}

	// Not proud but it works alright
	pub fn next(self) -> Self {
		match self {
			PzbState::Free => Self::Restricted1000hz,
			PzbState::Restricted1000hz => Self::Restricted500hz,
			PzbState::Restricted500hz => Self::ForcedStop,
			PzbState::ForcedStop => Self::Free,
		}
	}
}
