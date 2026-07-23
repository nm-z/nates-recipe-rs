use crate::error::{CudaError, DriverCallError, DriverStatus, Result};
use crate::ffi::{Api, CUDA_SUCCESS, CuResult, DriverCapabilities, DriverSymbol, DynamicLibrary};
use core::ffi::{CStr, c_char};
use core::ptr;
use std::sync::Arc;

#[derive(Clone)]
pub struct Driver {
	pub(crate) inner: Arc<DriverInner>,
}

pub(crate) struct DriverInner {
	pub(crate) api: Api,
	library: LibraryOwner,
}

enum LibraryOwner {
	Dynamic(DynamicLibrary),
	#[cfg(test)]
	Test,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModuleLoadingMode {
	Eager,
	Lazy,
}

impl Driver {
	pub fn load() -> Result<Self> {
		let library = DynamicLibrary::open_default()?;
		Self::from_library(library)
	}

	pub fn load_from_path(path: &str) -> Result<Self> {
		let library = DynamicLibrary::open(path)?;
		Self::from_library(library)
	}

	fn from_library(library: DynamicLibrary) -> Result<Self> {
		let api = library.load_api()?;
		Ok(Self {
			inner: Arc::new(DriverInner {
				api,
				library: LibraryOwner::Dynamic(library),
			}),
		})
	}

	pub fn loaded_library(&self) -> &str {
		match &self.inner.library {
			LibraryOwner::Dynamic(library) => library.name(),
			#[cfg(test)]
			LibraryOwner::Test => "<test>",
		}
	}

	pub fn capabilities(&self) -> &DriverCapabilities {
		self.inner.api.capabilities()
	}

	pub fn supports(&self, symbol: DriverSymbol) -> bool {
		self.capabilities().supports(symbol)
	}

	pub fn module_loading_mode(&self) -> Result<Option<ModuleLoadingMode>> {
		let Some(function) = self.inner.api.module_get_loading_mode else {
			return Ok(None);
		};
		let mut raw_mode = 0;
		self.check("cuModuleGetLoadingMode", unsafe {
			function(&raw mut raw_mode)
		})?;
		match raw_mode {
			1 => Ok(Some(ModuleLoadingMode::Eager)),
			2 => Ok(Some(ModuleLoadingMode::Lazy)),
			_ => Err(CudaError::InvalidDriverValue {
				operation: "cuModuleGetLoadingMode",
				detail: format!("unknown loading mode {raw_mode}"),
			}),
		}
	}

	pub(crate) fn check(&self, operation: &'static str, status: CuResult) -> Result<()> {
		if status == CUDA_SUCCESS {
			return Ok(());
		}
		let (name, description) = self.error_detail(status);
		Err(CudaError::DriverCall(DriverCallError {
			operation,
			status: DriverStatus(status),
			name,
			description,
		}))
	}

	/// Allocation-free status conversion for post-realization submit and poll
	/// paths. Rich driver text is collected during discovery and realization;
	/// a live-loop failure retains the exact numeric status.
	pub(crate) fn check_status_only(&self, operation: &'static str, status: CuResult) -> Result<()> {
		if status == CUDA_SUCCESS {
			return Ok(());
		}
		Err(CudaError::DriverCall(DriverCallError {
			operation,
			status: DriverStatus(status),
			name: None,
			description: None,
		}))
	}

	fn error_detail(&self, status: CuResult) -> (Option<String>, Option<String>) {
		(
			self.error_text(status, self.inner.api.get_error_name),
			self.error_text(status, self.inner.api.get_error_string),
		)
	}

	fn error_text(
		&self,
		status: CuResult,
		function: Option<unsafe extern "C" fn(CuResult, *mut *const c_char) -> CuResult>,
	) -> Option<String> {
		let function = function?;
		let mut text = ptr::null();
		let result = unsafe { function(status, &raw mut text) };
		if result != CUDA_SUCCESS || text.is_null() {
			return None;
		}
		Some(unsafe { CStr::from_ptr(text) }
			.to_string_lossy()
			.into_owned())
	}

	#[cfg(test)]
	pub(crate) fn from_test_api(api: Api) -> Self {
		Self {
			inner: Arc::new(DriverInner {
				api,
				library: LibraryOwner::Test,
			}),
		}
	}
}

impl core::fmt::Debug for Driver {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Driver")
			.field("loaded_library", &self.loaded_library())
			.field("capabilities", self.capabilities())
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ffi::test_support;

	#[test]
	fn driver_clone_keeps_the_symbol_owner_alive() {
		let driver = Driver::from_test_api(test_support::api(false));
		let clone = driver.clone();
		drop(driver);
		assert_eq!(clone.loaded_library(), "<test>");
		assert!(clone.supports(DriverSymbol::Init));
	}

	#[test]
	fn loading_mode_query_is_optional() {
		let r470 = Driver::from_test_api(test_support::api(false));
		assert_eq!(r470.module_loading_mode().unwrap(), None);

		let modern = Driver::from_test_api(test_support::api(true));
		assert_eq!(
			modern.module_loading_mode().unwrap(),
			Some(ModuleLoadingMode::Eager)
		);
	}
}
