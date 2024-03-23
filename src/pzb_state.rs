use crate::{led_state::PzbLedState, pzb::PzbCategory};

pub const PZB_STATE_COUNT: usize = 4;

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
			PzbState::Restricted1000hz => led.hz1000(),
			PzbState::Restricted500hz => led.hz500(),
			PzbState::ForcedStop => led.hz1000_blink().hz500_blink(),
		}
	}

	pub fn states() -> [PzbState; PZB_STATE_COUNT] {
		[
			PzbState::Free,
			PzbState::Restricted1000hz,
			PzbState::Restricted500hz,
			PzbState::ForcedStop,
		]
	}
}
