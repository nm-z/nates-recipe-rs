use core::fmt;

/// Exact decimal byte count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteCount(u64);

impl ByteCount {
	pub const ZERO: Self = Self(0);

	#[must_use]
	pub const fn new(bytes: u64) -> Self { Self(bytes) }

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }

	#[must_use]
	pub fn checked_add(self, other: Self) -> Option<Self> { self.0.checked_add(other.0).map(Self) }

	#[must_use]
	pub fn checked_sub(self, other: Self) -> Option<Self> { self.0.checked_sub(other.0).map(Self) }

	#[must_use]
	pub fn checked_mul(self, factor: u64) -> Option<Self> { self.0.checked_mul(factor).map(Self) }

	pub fn checked_align_up(self, alignment: Self) -> Result<Self, UnitError> {
		if alignment.0 == 0 || !alignment.0.is_power_of_two() {
			return Err(UnitError::InvalidAlignment(alignment.0));
		}
		let mask = alignment.0 - 1;
		self.0.checked_add(mask)
			.map(|value| Self(value & !mask))
			.ok_or(UnitError::Overflow)
	}
}

impl fmt::Display for ByteCount {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { write!(formatter, "{} B", self.0) }
}

/// Exact byte offset within an arena.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(u64);

impl ByteOffset {
	#[must_use]
	pub const fn new(bytes: u64) -> Self { Self(bytes) }

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }

	#[must_use]
	pub fn checked_end(self, size: ByteCount) -> Option<ByteCount> { self.0.checked_add(size.get()).map(ByteCount::new) }
}

/// Nonzero decimal bytes per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytesPerSecond(u64);

impl BytesPerSecond {
	pub fn new(value: u64) -> Result<Self, UnitError> {
		if value == 0 {
			Err(UnitError::ZeroRate)
		} else {
			Ok(Self(value))
		}
	}

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }
}

/// Nonzero number of transfers that may be inflight in one direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransferLaneCount(u32);

impl TransferLaneCount {
	pub fn new(value: u32) -> Result<Self, UnitError> {
		if value == 0 {
			Err(UnitError::ZeroTransferLaneCount)
		} else {
			Ok(Self(value))
		}
	}

	#[must_use]
	pub const fn get(self) -> u32 { self.0 }
}

/// Exact operation FLOP count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlopCount(u64);

impl FlopCount {
	pub const ZERO: Self = Self(0);

	#[must_use]
	pub const fn new(value: u64) -> Self { Self(value) }

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }

	#[must_use]
	pub fn checked_add(self, other: Self) -> Option<Self> { self.0.checked_add(other.0).map(Self) }

	#[must_use]
	pub fn checked_mul(self, factor: u64) -> Option<Self> { self.0.checked_mul(factor).map(Self) }
}

/// Nonzero floating-point operations per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FlopsPerSecond(u64);

impl FlopsPerSecond {
	pub fn new(value: u64) -> Result<Self, UnitError> {
		if value == 0 {
			Err(UnitError::ZeroRate)
		} else {
			Ok(Self(value))
		}
	}

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }
}

/// Integer nanoseconds used by a deterministic schedule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nanoseconds(u64);

impl Nanoseconds {
	pub const ZERO: Self = Self(0);

	#[must_use]
	pub const fn new(value: u64) -> Self { Self(value) }

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }

	#[must_use]
	pub fn checked_add(self, other: Self) -> Option<Self> { self.0.checked_add(other.0).map(Self) }
}

/// Number of elements in a parallel index space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementCount(u64);

impl ElementCount {
	pub fn new(value: u64) -> Result<Self, UnitError> {
		if value == 0 {
			Err(UnitError::ZeroElementCount)
		} else {
			Ok(Self(value))
		}
	}

	#[must_use]
	pub const fn get(self) -> u64 { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitError {
	Overflow,
	ZeroRate,
	ZeroTransferLaneCount,
	ZeroElementCount,
	InvalidAlignment(u64),
}

impl fmt::Display for UnitError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Overflow => formatter.write_str("unit arithmetic overflow"),
			Self::ZeroRate => formatter.write_str("rate must be nonzero"),
			Self::ZeroTransferLaneCount => formatter.write_str("transfer lane count must be nonzero"),
			Self::ZeroElementCount => formatter.write_str("element count must be nonzero"),
			Self::InvalidAlignment(value) => {
				write!(
					formatter,
					"alignment must be a nonzero power of two, got {value}"
				)
			}
		}
	}
}

impl std::error::Error for UnitError {}

const NANOS_PER_SECOND: u128 = 1_000_000_000;

pub fn transfer_time_ceil(bytes: ByteCount, rate: BytesPerSecond) -> Result<Nanoseconds, UnitError> { ratio_time_ceil(bytes.get(), rate.get()) }

pub fn calculation_time_ceil(flops: FlopCount, rate: FlopsPerSecond) -> Result<Nanoseconds, UnitError> { ratio_time_ceil(flops.get(), rate.get()) }

fn ratio_time_ceil(work: u64, rate: u64) -> Result<Nanoseconds, UnitError> {
	let numerator = u128::from(work)
		.checked_mul(NANOS_PER_SECOND)
		.ok_or(UnitError::Overflow)?;
	let denominator = u128::from(rate);
	let value = numerator
		.checked_add(denominator - 1)
		.ok_or(UnitError::Overflow)?
		/ denominator;
	u64::try_from(value)
		.map(Nanoseconds::new)
		.map_err(|_| UnitError::Overflow)
}
