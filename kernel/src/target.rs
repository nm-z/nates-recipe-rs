use crate::{LoweringError, LoweringErrorKind};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AmdTarget {
	/// Complete HSA target ID, including feature modifiers when present.
	pub target_id: String,
	/// Required AMDGPU code-object version.
	pub code_object_version: u8,
}

impl AmdTarget {
	pub fn validate(&self) -> Result<(), LoweringError> {
		validate_token("AMD target ID", &self.target_id)?;
		if !self.target_id.starts_with("gfx") {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				"AMD target ID must begin with `gfx`",
			));
		}
		if self.code_object_version == 0 {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				"AMDGPU code-object version must be nonzero",
			));
		}
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NvidiaTarget {
	pub sm_major: u8,
	pub sm_minor: u8,
	/// PTX ISA encoded as major * 10 + minor, for example 75.
	pub ptx_isa: u16,
}

impl NvidiaTarget {
	pub fn validate(self) -> Result<(), LoweringError> {
		if self.sm_major < 3 || self.sm_major > 12 || self.sm_minor > 9 {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				format!(
					"unsupported NVIDIA target sm_{}{}",
					self.sm_major, self.sm_minor
				),
			));
		}
		if !(32..=90).contains(&self.ptx_isa) {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidTarget,
				format!("unsupported PTX ISA {}", self.ptx_isa),
			));
		}
		Ok(())
	}

	#[must_use]
	pub fn architecture(self) -> String { format!("sm_{}{}", self.sm_major, self.sm_minor) }

	#[must_use]
	pub fn llvm_ptx_feature(self) -> String { format!("+ptx{}", self.ptx_isa) }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KernelTarget {
	Amd(AmdTarget),
	Nvidia(NvidiaTarget),
}

impl KernelTarget {
	pub fn validate(&self) -> Result<(), LoweringError> {
		match self {
			Self::Amd(target) => target.validate(),
			Self::Nvidia(target) => target.validate(),
		}
	}
}

fn validate_token(field: &str, value: &str) -> Result<(), LoweringError> {
	if value.is_empty()
		|| !value
			.bytes()
			.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b':' | b'.'))
	{
		return Err(LoweringError::new(
			LoweringErrorKind::InvalidTarget,
			format!("{field} contains unsupported characters"),
		));
	}
	Ok(())
}
