pub const RESTRICTED_SPEED: u16 = 45; // KMH - Speed at which the train can travel when restricted
pub const LOWER_RESTRICTED_SPEED: u16 = 10; // KMH

pub enum PzbCategory {
	O, // 85
	M, // 70
	U, // 55
}

impl PzbCategory {
	pub const fn get_constraints(self) -> PzbConstraints {
		match self {
			PzbCategory::O => PzbConstraints {
				top_speed:         165,
				deceleration_time: 23,
			},
			PzbCategory::M => PzbConstraints {
				top_speed:         125,
				deceleration_time: 29,
			},
			PzbCategory::U => PzbConstraints {
				top_speed:         105,
				deceleration_time: 38,
			},
		}
	}
}

pub struct PzbConstraints {
	top_speed:         u16, // KMH
	deceleration_time: u8,  // Seconds
}

impl PzbConstraints {}
