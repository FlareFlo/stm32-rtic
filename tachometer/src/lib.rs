#![cfg_attr(feature = "no-std", no_std)]

pub mod units;

use core::f32::consts::PI;

use ringbuffer::{ConstGenericRingBuffer as Ringbuffer, RingBuffer};

use crate::units::length::Length;

// A wrapping ring-buffer that tracks rotations of a wheel over time
// Every rotation should add one timestamp
pub struct Tachometer<const CAPACITY: usize> {
	// Leaking internal struct due to IntoIter impl
	pub buf:                Ringbuffer<i128, CAPACITY>,
	pub tire:               TireDimensions,
	pub total_revolutions:  usize,
	// Amount of equally spaced points on wheel that are measured
	pub pointers_per_wheel: usize,
	/// Factor required to convert from tire revolution to cadence
	pub gear_ratio:         f32,
}

pub struct Sample {
	pub distance: Length,
	pub cadence:  f32,
}

impl<const CAPACITY: usize> Tachometer<CAPACITY> {
	pub const fn new(tire: TireDimensions, wheel_measurements: usize, gear_ratio: f32) -> Self {
		Self {
			buf: Ringbuffer::new(),
			tire,
			total_revolutions: 0,
			pointers_per_wheel: wheel_measurements,
			gear_ratio,
		}
	}

	// Returns elements in the last n milliseconds
	pub fn last_millis(&self, threshold: i128, now: i128) -> impl Iterator<Item = i128> + '_ {
		self.buf
			.iter()
			.filter(move |&e| (now.saturating_sub(threshold)..).contains(e))
			.map(|e| *e)
	}

	// Returns distance covered in the last n milliseconds
	pub fn last_samples(&self, threshold: i128, now: i128) -> Sample {
		let last = self.last_millis(threshold, now);
		let last_count = last.count();
		let distance = self
			.tire
			.circumference()
			.scale(last_count as f32)
			.div(self.pointers_per_wheel as f32);

		let threshold_seconds = threshold as f32 / 1000.0;
		let revolutions_per_second = (last_count as f32 / self.gear_ratio) / (threshold_seconds);

		Sample { distance, cadence: revolutions_per_second * 60.0 }
	}

	/// Returns distance in centimeters
	pub fn total_distance_moved(&self) -> Length {
		self.tire
			.circumference()
			.scale(self.total_revolutions as f32)
			.div(self.pointers_per_wheel as f32)
	}

	pub fn insert(&mut self, timestamp: i128) {
		self.buf.push(timestamp);
		self.total_revolutions += 1;
	}
}

pub enum TireDimensions {
	Diameter(Length),
	Radius(Length),
	Circumference(Length),
}

impl TireDimensions {
	pub fn circumference(&self) -> Length {
		let diameter_to_circumference = |diam: Length| diam.scale(PI).scale(2.0);
		match self {
			TireDimensions::Diameter(diam) => diameter_to_circumference(diam.div(2.0)),
			TireDimensions::Circumference(circum) => *circum,
			TireDimensions::Radius(radius) => diameter_to_circumference(*radius),
		}
	}
}

#[cfg(test)]
mod test {
	#[cfg(feature = "no-std")]
	compiler_error!("cant run test in no-std");

	use alloc::{vec, vec::Vec};

	use ringbuffer::RingBuffer;

	use crate::{Tachometer, TireDimensions};

	#[test]
	fn last_millis() {
		let mut tach: Tachometer<20> = Tachometer::new(TireDimensions::Diameter(70.0));
		tach.buf.push(0);
		tach.buf.push(999);
		tach.buf.push(1000);
		tach.buf.push(2000);
		tach.buf.push(2500);
		tach.buf.push(4000);
		assert_eq!(
			tach.last_millis(3000).collect::<Vec<i64>>(),
			vec![1000, 2000, 2500, 4000]
		);
	}
}
