use core::{fmt, num::NonZeroU8};

pub const F32_GUARANTEED_SIGNIFICANT_DIGITS: u8 = 6;
pub const I32_GUARANTEED_SIGNIFICANT_DIGITS: u8 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecimalErrorKind {
	Empty,
	InvalidSyntax,
	TooManySignificantDigits,
	OutsidePayloadRange,
	PrecisionLoss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecimalError {
	pub kind: DecimalErrorKind,
	pub input: String,
	pub detail: String,
}

impl DecimalError {
	fn new(kind: DecimalErrorKind, input: &str, detail: impl Into<String>) -> Self {
		Self {
			kind,
			input: input.to_owned(),
			detail: detail.into(),
		}
	}
}

impl fmt::Display for DecimalError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"{:?}: {} for {:?}",
			self.kind, self.detail, self.input
		)
	}
}

impl std::error::Error for DecimalError {}

pub type DecimalResult<T> = Result<T, DecimalError>;

/// A finite f32 payload plus the decimal precision proven at ingestion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct F32Decimal {
	bits: u32,
	significant_digits: NonZeroU8,
}

impl F32Decimal {
	#[must_use]
	pub const fn bits(self) -> u32 { self.bits }

	#[must_use]
	pub const fn value(self) -> f32 { f32::from_bits(self.bits) }

	#[must_use]
	pub const fn significant_digits(self) -> NonZeroU8 { self.significant_digits }

	#[must_use]
	pub const fn to_le_bytes(self) -> [u8; 4] { self.bits.to_le_bytes() }
}

/// An exact int32 payload plus its validated decimal digit count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I32Decimal {
	value: i32,
	significant_digits: NonZeroU8,
}

impl I32Decimal {
	#[must_use]
	pub const fn value(self) -> i32 { self.value }

	#[must_use]
	pub const fn significant_digits(self) -> NonZeroU8 { self.significant_digits }

	#[must_use]
	pub const fn to_le_bytes(self) -> [u8; 4] { self.value.to_le_bytes() }
}

/// Parse and prove the normative six-significant-digit f32 representation
/// boundary.
///
/// The syntax is a finite decimal with an optional sign, decimal point, and
/// base-ten exponent. Whitespace, separators, NaN, and infinity are rejected.
/// Values that cannot round-trip at their declared precision (including a
/// subnormal precision edge) fail closed instead of weakening the claim.
pub fn parse_contract_f32(input: &str) -> DecimalResult<F32Decimal> {
	let scan = scan_decimal(input, DecimalSyntax::Float)?;
	require_digit_bound(
		input,
		scan.significant_digits,
		F32_GUARANTEED_SIGNIFICANT_DIGITS,
		"f32",
	)?;
	let reference = input.parse::<f64>().map_err(|_| {
		DecimalError::new(
			DecimalErrorKind::OutsidePayloadRange,
			input,
			"decimal cannot be represented by the validation domain",
		)
	})?;
	let value = input.parse::<f32>().map_err(|_| {
		DecimalError::new(
			DecimalErrorKind::OutsidePayloadRange,
			input,
			"decimal cannot be represented as f32",
		)
	})?;
	if !reference.is_finite() || !value.is_finite() {
		return Err(DecimalError::new(
			DecimalErrorKind::OutsidePayloadRange,
			input,
			"calculation payloads require a finite f32 representation",
		));
	}
	if reference == 0.0 {
		if value != 0.0 || reference.is_sign_negative() != value.is_sign_negative() {
			return Err(DecimalError::new(
				DecimalErrorKind::PrecisionLoss,
				input,
				"signed zero did not survive f32 representation",
			));
		}
	} else if value == 0.0 {
		return Err(DecimalError::new(
			DecimalErrorKind::PrecisionLoss,
			input,
			"nonzero decimal underflows to f32 zero",
		));
	}
	let expected = format_at_precision(reference, scan.significant_digits);
	let observed = format_at_precision(f64::from(value), scan.significant_digits);
	if expected != observed {
		return Err(DecimalError::new(
			DecimalErrorKind::PrecisionLoss,
			input,
			format!(
				"f32 round-trip is {observed}, expected {expected} at {} significant digit(s)",
				scan.significant_digits
			),
		));
	}
	Ok(F32Decimal {
		bits: value.to_bits(),
		significant_digits: nonzero_digits(scan.significant_digits),
	})
}

/// Parse the normative exact nine-significant-digit int32 representation
/// boundary.
pub fn parse_contract_i32(input: &str) -> DecimalResult<I32Decimal> {
	let scan = scan_decimal(input, DecimalSyntax::Integer)?;
	require_digit_bound(
		input,
		scan.significant_digits,
		I32_GUARANTEED_SIGNIFICANT_DIGITS,
		"int32",
	)?;
	let value = input.parse::<i32>().map_err(|_| {
		DecimalError::new(
			DecimalErrorKind::OutsidePayloadRange,
			input,
			"decimal integer is outside the int32 payload range",
		)
	})?;
	Ok(I32Decimal {
		value,
		significant_digits: nonzero_digits(scan.significant_digits),
	})
}

#[derive(Clone, Copy)]
enum DecimalSyntax {
	Float,
	Integer,
}

#[derive(Clone, Copy)]
struct DecimalScan {
	significant_digits: u8,
}

fn scan_decimal(input: &str, syntax: DecimalSyntax) -> DecimalResult<DecimalScan> {
	if input.is_empty() {
		return Err(DecimalError::new(
			DecimalErrorKind::Empty,
			input,
			"decimal input is empty",
		));
	}
	let bytes = input.as_bytes();
	let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
	if index == bytes.len() {
		return Err(invalid_syntax(input));
	}
	let mut saw_digit = false;
	let mut saw_nonzero = false;
	let mut significant_digits = 0_u8;
	let mut saw_point = false;
	while let Some(byte) = bytes.get(index).copied() {
		match byte {
			b'0'..=b'9' => {
				saw_digit = true;
				if byte != b'0' {
					saw_nonzero = true;
				}
				if saw_nonzero {
					significant_digits = significant_digits.saturating_add(1);
				}
				index += 1;
			}
			b'.' if matches!(syntax, DecimalSyntax::Float) && !saw_point => {
				saw_point = true;
				index += 1;
			}
			b'e' | b'E' if matches!(syntax, DecimalSyntax::Float) => {
				break;
			}
			_ => return Err(invalid_syntax(input)),
		}
	}
	if !saw_digit {
		return Err(invalid_syntax(input));
	}
	if index < bytes.len() {
		index += 1;
		if matches!(bytes.get(index), Some(b'+' | b'-')) {
			index += 1;
		}
		let exponent_start = index;
		while matches!(bytes.get(index), Some(b'0'..=b'9')) {
			index += 1;
		}
		if index == exponent_start || index != bytes.len() {
			return Err(invalid_syntax(input));
		}
	}
	if index != bytes.len() {
		return Err(invalid_syntax(input));
	}
	Ok(DecimalScan {
		significant_digits: significant_digits.max(1),
	})
}

fn require_digit_bound(input: &str, digits: u8, limit: u8, payload: &str) -> DecimalResult<()> {
	if digits > limit {
		Err(DecimalError::new(
			DecimalErrorKind::TooManySignificantDigits,
			input,
			format!("{payload} contract accepts at most {limit} significant digits, got {digits}"),
		))
	} else {
		Ok(())
	}
}

fn format_at_precision(value: f64, significant_digits: u8) -> String {
	format!(
		"{:.*e}",
		usize::from(significant_digits.saturating_sub(1)),
		value
	)
}

fn nonzero_digits(digits: u8) -> NonZeroU8 { NonZeroU8::new(digits).unwrap_or(NonZeroU8::MIN) }

fn invalid_syntax(input: &str) -> DecimalError {
	DecimalError::new(
		DecimalErrorKind::InvalidSyntax,
		input,
		"expected a decimal without whitespace or separators",
	)
}
