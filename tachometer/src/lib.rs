#![cfg_attr(not(test), no_std)]

extern crate alloc;

use ringbuffer::{ConstGenericRingBuffer as Ringbuffer, RingBuffer};

// A wrapping ring-buffer that tracks elements over a certain time frame
pub struct Tachometer<const CAPACITY: usize> {
	// Leaking internal struct due to IntoIter impl
	pub buf: Ringbuffer<i64, CAPACITY>,
}

impl<const CAPACITY: usize> Tachometer<CAPACITY> {
	pub const fn new() -> Self {
		Self {
			buf: Ringbuffer::new(),
		}
	}

	pub fn last_millis(&self, threshold: i64) -> impl Iterator<Item = i64> + '_ {
		// TODO: Using default of max works? Should mean nothing falls within threshold
		let newest = self.buf.get_signed(-1).unwrap_or(&i64::MAX);
		self.buf
			.iter()
			.filter(move |&e| (newest.saturating_sub(threshold)..=*newest).contains(e))
			.map(|e|*e)
	}
}
#[cfg(test)]
mod test {
	use alloc::vec;
	use alloc::vec::Vec;
	use ringbuffer::RingBuffer;

	use crate::Tachometer;

	#[test]
	fn last_millis() {
		let mut tach: Tachometer<20> = Tachometer::new();
		tach.buf.push(0);
		tach.buf.push(999);
		tach.buf.push(1000);
		tach.buf.push(2000);
		tach.buf.push(2500);
		tach.buf.push(4000);
		assert_eq!(tach.last_millis(3000).collect::<Vec<i64>>(), vec![1000,2000,2500,4000]);
	}
}
