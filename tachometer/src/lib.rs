#![cfg_attr(feature = "no-std", no_std)]

pub mod units;

use core::f32::consts::PI;
use ringbuffer::{ConstGenericRingBuffer as Ringbuffer, RingBuffer};
use crate::units::length::Length;

// A wrapping ring-buffer that tracks rotations of a wheel over time
// Every rotation should add one timestamp
pub struct Tachometer<const CAPACITY: usize> {
	// Leaking internal struct due to IntoIter impl
	pub buf: Ringbuffer<i128, CAPACITY>,
	pub tire: TireDimensions,
	pub total_revolutions: usize,
}

impl<const CAPACITY: usize> Tachometer<CAPACITY> {
	pub const fn new(tire: TireDimensions) -> Self {
		Self {
			buf: Ringbuffer::new(),
			tire,
			total_revolutions: 0,
		}
	}

	// Returns elements in the last n milliseconds
	pub fn last_millis(&self, threshold: i128, now: i128) -> impl Iterator<Item = i128> + '_ {
		self.buf
			.iter()
			.filter(move |&e| (now.saturating_sub(threshold)..).contains(e))
			.map(|e|*e)
	}

	// Returns distance covered in the last n milliseconds
	pub fn last_distance_moved(&self, threshold: i128, now: i128) -> Length {
		let last = self.last_millis(threshold, now);
		self.tire.circumference().scale(last.count() as f32)
	}

	/// Returns distance in centimeters
	pub fn total_distance_moved(&self) -> Length {
		self.tire.circumference().scale(self.total_revolutions as f32)
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
			TireDimensions::Diameter(diam) => {diameter_to_circumference(diam.div(2.0))}
			TireDimensions::Circumference(circum) => { *circum }
			TireDimensions::Radius(radius) => {diameter_to_circumference(*radius)}
		}
	}
}

#[cfg(test)]
mod test {
	#[cfg(feature = "no-std")]
	compiler_error!("cant run test in no-std");

	use alloc::vec;
	use alloc::vec::Vec;
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
		assert_eq!(tach.last_millis(3000).collect::<Vec<i64>>(), vec![1000,2000,2500,4000]);
	}
}
