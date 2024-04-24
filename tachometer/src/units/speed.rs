#[derive(Copy, Clone)]
pub struct Speed {
	// Stored as meters per second
	amount: f32,
}

impl Speed {
	pub fn from_meter_per_second(amount: f32) -> Self {
		Self { amount }
	}

	pub fn as_meter_per_second(&self) -> f32 {
		self.amount
	}

	pub fn as_kilometers_per_hour(&self) -> f32 {
		self.amount * 3.6
	}
}
