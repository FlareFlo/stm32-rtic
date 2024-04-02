#![cfg_attr(not(test), no_std)]

use core::fmt::{Display, Formatter};

// A wrapping ring-buffer that tracks elements over a certain time frame
pub struct Tachometer<const CAPACITY: usize> {
	buf: [Measurement; CAPACITY],
	head: usize,
	tail: usize,
}

impl<const CAPACITY: usize> Tachometer<CAPACITY> {
	pub const fn new() -> Self {
		Self {
			buf: [Measurement{time: 0}; CAPACITY],
			head: 0,
			tail: 0,
		}
	}

	pub const fn capacity(&self) -> usize {
		todo!()
	}

	pub const fn len(&self) -> usize {
		todo!()
	}

	pub fn push_elem(&mut self, elem: Measurement) -> usize {
		let new_idx = self.tail;
		self.tail = (self.tail + 1).rem_euclid(CAPACITY);
		self.buf[new_idx] = elem;
		new_idx
	}

	pub fn push(&mut self, hour: u8, minute: u8, millisecond: u16) -> usize {
		self.push_elem(Measurement::from_parts(hour, minute, millisecond))
	}

	pub fn get(&self, idx: usize) -> Option<&Measurement> {
		let offset = (self.head + idx).rem_euclid(1);
		if offset >= self.tail { return None  }
		self.buf.get(offset)
	}
}

impl<const CAPACITY: usize> IntoIterator for Tachometer<CAPACITY> {
	type Item = Measurement;
	type IntoIter = IteratorTachometer<CAPACITY>;

	fn into_iter(self) -> Self::IntoIter {
		Self::IntoIter {
			tacho: self,
			idx: 0,
		}
	}
}

pub struct IteratorTachometer<const CAPACITY: usize> {
	tacho: Tachometer<CAPACITY>,
	idx: usize,
}

impl<const CAPACITY: usize> Iterator for IteratorTachometer<CAPACITY> {
	type Item = Measurement;

	fn next(&mut self) -> Option<Self::Item> {
		let tacho = &self.tacho;
		let ret = tacho.get(self.idx)?;
		self.idx += 1;
		Some(*ret)
	}
}

#[derive(Copy, Clone, Default)]
pub struct Measurement {
	// Timestamp made from hours, minutes and milliseconds
	time: u32,
}

impl Measurement {
	pub fn from_parts(hour: u8, minute: u8, millisecond: u16) -> Self {
		let hour = hour as u32 * 24 * 60 * 1000;
		let minute = minute as u32 * 60 * 1000;
		Self {
			time: hour + minute + millisecond as u32,
		}
	}

	pub const fn hour(&self) -> u32 {
		self.time / 24 / 60 / 1000
	}

	pub const fn minute(&self) -> u32 {
		self.time / 60 / 1000
	}

	pub const fn millis(&self) -> u32 {
		self.time
	}
}

impl Display for Measurement {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}:{}.{}", self.hour(), self.minute(), self.millis())
	}
}

#[cfg(test)]
mod test {
	use crate::Tachometer;

	#[test]
	fn push() {
		let mut tach: Tachometer<3>  = Tachometer::new();
		tach.push(0, 0, 100);
		for tach in tach {
			eprintln!("{}", tach);
		}
	}

}