use recipe_core::{Digest, Label}; use sha2::{Digest as _, Sha256};

#[derive(Debug)]
pub(crate) struct StableDigest { hasher: Sha256, }

impl StableDigest { pub(crate) fn new(domain: &str) -> Self { let mut result = Self { hasher: Sha256::new(), };
		result.bytes(domain.as_bytes()); result }

	pub(crate) fn bytes(&mut self, value: &[u8]) { self.hasher.update(value.len().to_string().as_bytes());
		self.hasher.update(b":");
		self.hasher.update(value); }

	pub(crate) fn digest(&mut self, value: Digest) { self.hasher.update(value.bytes()); }

	pub(crate) fn label(&mut self, value: &Label) { self.bytes(value.as_str().as_bytes()); }

	pub(crate) fn length(&mut self, value: usize) { self.bytes(value.to_string().as_bytes()); }

	pub(crate) fn bool(&mut self, value: bool) { self.hasher.update([u8::from(value)]); }

	pub(crate) fn u8(&mut self, value: u8) { self.hasher.update([value]); }

	pub(crate) fn u64(&mut self, value: u64) { self.hasher.update(value.to_le_bytes()); }

	pub(crate) fn finish(self) -> Digest { let finalized = self.hasher.finalize(); let mut bytes = [0_u8; 32];
		bytes.copy_from_slice(&finalized); Digest::new(bytes) } }
