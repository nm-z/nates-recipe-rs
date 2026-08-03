use core::fmt;

/// Validated nonempty textual identity supplied by configuration or tooling.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label(String);

impl Label { pub fn new(value: impl Into<String>) -> Result<Self, LabelError> { let value = value.into();
		if value.trim().is_empty() { Err(LabelError) } else { Ok(Self(value)) } }

	#[must_use]
	pub fn as_str(&self) -> &str { &self.0 } }

impl fmt::Display for Label {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(formatter) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelError;

impl fmt::Display for LabelError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str("label must not be empty or whitespace") }
}

impl std::error::Error for LabelError {}

/// A caller-computed 256-bit content or manifest digest.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest([u8; 32]);

impl Digest { pub const ZERO: Self = Self([0; 32]);

	#[must_use]
	pub const fn new(bytes: [u8; 32]) -> Self { Self(bytes) }

	#[must_use]
	pub const fn bytes(self) -> [u8; 32] { self.0 }

	#[must_use]
	pub fn is_zero(self) -> bool { self == Self::ZERO } }

impl fmt::Debug for Digest {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("Digest(")?;
		for byte in &self.0[..6] {
			write!(formatter, "{byte:02x}")?;
		}
		formatter.write_str("...)")
	} }

macro_rules! digest_identities { ($($name:ident),+ $(,)?) => { $(
			#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
			pub struct $name(Digest);

			impl $name {
				#[must_use]
				pub const fn new(digest: Digest) -> Self { Self(digest) }

				#[must_use]
				pub const fn digest(self) -> Digest { self.0 }

				#[must_use]
				pub fn is_zero(self) -> bool { self.0.is_zero() } } )+ }; }

digest_identities!( TopologyIdentity, DiscoveryIdentity, DraftIdentity, CandidateIdentity, RealizationIdentity,
	BundleIdentity, );
