#![cfg_attr(feature = "no-std", no_std)]

use core::f32::consts::PI;
use ringbuffer::{ConstGenericRingBuffer as Ringbuffer, RingBuffer};

// A wrapping ring-buffer that tracks rotations of a wheel over time
// Every rotation should add one timestamp
pub struct Tachometer<const CAPACITY: usize> {
	// Leaking internal struct due to IntoIter impl
	pub buf: Ringbuffer<i128, CAPACITY>,
	pub tire: TireDimensions,
}

impl<const CAPACITY: usize> Tachometer<CAPACITY> {
	pub const fn new(tire: TireDimensions) -> Self {
		Self {
			buf: Ringbuffer::new(),
			tire,
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
	pub fn last_distance_moved(&self, threshold: i128, now: i128) -> f32 {
		let last = self.last_millis(threshold, now);
		last.count() as f32 * self.tire.circumference()
	}

	pub fn insert(&mut self, timestamp: i128) {
		self.buf.push(timestamp)
	}
}

pub enum TireDimensions {
	Diameter(f32),
	Radius(f32),
	Circumference(f32),
}

impl TireDimensions {
	pub fn circumference(&self) -> f32 {
		let diameter_to_circumference = |diam| PI * diam / 2.0;
		match self {
			TireDimensions::Diameter(diam) => {diameter_to_circumference(*diam)}
			TireDimensions::Circumference(circum) => { *circum }
			TireDimensions::Radius(radius) => {diameter_to_circumference(*radius * 2.0)}
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
