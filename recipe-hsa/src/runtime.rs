use std::{ffi::OsStr, sync::Arc};

use crate::{
	Error, Result,
	abi::STATUS_SUCCESS,
	discovery::{self, Discovery},
	loader::Api,
};

/// One balanced ROCr initialization reference.
///
/// This is intentionally not a singleton. Rust borrows prevent the runtime
/// from being closed while discovery records, sessions, or queues still refer
/// to it.
pub struct Runtime {
	pub(crate) api: Arc<Api>,
	active: bool,
}

impl Runtime {
	/// Load the first normal ROCr soname that is installed and initialize it.
	pub fn open_default() -> Result<Self> {
		let mut last_open_error = None;
		for candidate in ["libhsa-runtime64.so.1", "libhsa-runtime64.so"] {
			match Self::open(candidate) {
				Ok(runtime) => return Ok(runtime),
				Err(error @ Error::LibraryOpen { .. }) => last_open_error = Some(error),
				Err(error) => return Err(error),
			}
		}
		Err(last_open_error.expect("the default ROCr candidate list is nonempty"))
	}

	/// Load and initialize ROCr from an explicit shared-library path or soname.
	pub fn open(path: impl AsRef<OsStr>) -> Result<Self> {
		let api = Arc::new(Api::load(path.as_ref())?);
		// SAFETY: the function pointer was resolved with the reviewed ABI and
		// `api` keeps its defining library loaded.
		let status = unsafe { (api.init)() };
		if status != STATUS_SUCCESS {
			// `hsa_status_string` cannot be called safely before a successful
			// `hsa_init`, so this typed error intentionally has no message.
			return Err(Error::Hsa {
				operation: "hsa_init",
				status,
				message: None,
			});
		}
		Ok(Self { api, active: true })
	}

	/// Exhaustively enumerate the currently visible ROCr topology.
	pub fn discover(&self) -> Result<Discovery<'_>> {
		self.ensure_active()?;
		discovery::discover(self)
	}

	/// Explicitly release this crate's HSA initialization reference.
	///
	/// Drop provides a fallback, but this method reports teardown errors.
	pub fn close(mut self) -> Result<()> {
		self.ensure_active()?;
		self.active = false;
		// SAFETY: this is the one balanced shutdown for this runtime object's
		// successful `hsa_init`. Borrowing prevents live child objects.
		let status = unsafe { (self.api.shut_down)() };
		self.api.check("hsa_shut_down", status)
	}

	pub(crate) fn ensure_active(&self) -> Result<()> {
		if self.active {
			Ok(())
		} else {
			Err(Error::RuntimeClosed)
		}
	}
}

impl Drop for Runtime {
	fn drop(&mut self) {
		if !self.active {
			return;
		}
		self.active = false;
		// SAFETY: same invariant as `close`; Drop cannot return the status.
		unsafe {
			(self.api.shut_down)();
		}
	}
}
