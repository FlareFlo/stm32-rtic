use crate::units::speed::Speed;
use crate::units::time::Time;

#[derive(Copy, Clone)]
pub struct Length {
	// Stored as meters
	amount: f32,
}

impl Length {
	pub fn from_centimeters(amount: f32) -> Self {
		Self {
			amount: amount / 100.0,
		}
	}

	pub fn as_kilometer(&self) -> f32 {
		self.amount / 1000.0
	}

	pub fn as_meter(&self) -> f32 {
		self.amount
	}

	pub fn scale(self, scale: f32) -> Self {
		Self {
			amount: self.amount * scale,
		}
	}

	pub fn div(self, scale: f32) -> Self {
		Self {
			amount: self.amount / scale,
		}
	}

	pub fn to_speed(self, time: Time) -> Speed {
		Speed::from_meter_per_second(self.as_meter() / time.as_seconds())
	}
}