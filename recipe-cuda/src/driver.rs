use core::{
	ffi::{CStr, c_char},
	ptr,
};
use std::sync::Arc;

use crate::{
	error::{CudaError, DriverCallError, DriverStatus, Result},
	ffi::{Api, CUDA_SUCCESS, CuResult, DriverCapabilities, DriverSymbol, DynamicLibrary},
};

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
		}
	}

	pub fn capabilities(&self) -> &DriverCapabilities { self.inner.api.capabilities() }

	pub fn supports(&self, symbol: DriverSymbol) -> bool { self.capabilities().supports(symbol) }

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
			_ => {
				Err(CudaError::InvalidDriverValue {
					operation: "cuModuleGetLoadingMode",
					detail: format!("unknown loading mode {raw_mode}"),
				})
			}
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
}

impl core::fmt::Debug for Driver {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Driver")
			.field("loaded_library", &self.loaded_library())
			.field("capabilities", self.capabilities())
			.finish()
	}
}
