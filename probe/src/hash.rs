use recipe_core::Digest; use sha2::{Digest as _, Sha256};

#[derive(Clone, Debug)]
pub(crate) struct CanonicalDigest { hasher: Sha256, }

impl CanonicalDigest { pub(crate) fn new(domain: &str, schema: u32) -> Self { let mut result = Self {
			hasher: Sha256::new(), }; result.string(domain); result.u64(u64::from(schema)); result }

	pub(crate) fn string(&mut self, value: &str) { self.bytes(value.as_bytes()); }

	pub(crate) fn bytes(&mut self, value: &[u8]) { self.hasher.update((value.len() as u64).to_le_bytes());
		self.hasher.update(value); }

	pub(crate) fn u64(&mut self, value: u64) { self.hasher.update(value.to_le_bytes()); }

	pub(crate) fn bool(&mut self, value: bool) { self.hasher.update([u8::from(value)]); }

	pub(crate) fn digest(&mut self, value: Digest) { self.hasher.update(value.bytes()); }

	pub(crate) fn finish(self) -> Digest { let output: [u8; 32] = self.hasher.finalize().into(); Digest::new(output) } }
