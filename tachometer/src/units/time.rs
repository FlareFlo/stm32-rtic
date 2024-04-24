#[derive(Copy, Clone)]
pub struct Time {
	// Stored as seconds
	amount: f32,
}

impl Time {
	pub fn seconds(amount: f32) -> Self {
		Self { amount }
	}

	pub fn milliseconds(amount: f32) -> Self {
		Self {
			amount: amount / 1000.0,
		}
	}

	pub fn as_seconds(&self) -> f32 {
		self.amount
	}

	pub fn as_milliseconds(&self) -> f32 {
		self.amount * 1000.0
	}
}
