use core::ffi::{c_char, c_void}; use std::{ ffi::{CStr, CString, OsStr}, os::unix::ffi::OsStrExt, ptr::NonNull, };

use crate::{ Error, Result, abi::{
		HsaAgentGetInfo, HsaAgentIterateIsas, HsaAmdAgentIterateMemoryPools, HsaAmdAgentsAllowAccess,
		HsaAmdMemoryAsyncCopy, HsaAmdMemoryPoolAllocate, HsaAmdMemoryPoolFree, HsaAmdMemoryPoolGetInfo,
		HsaCodeObjectReaderCreateFromMemory, HsaCodeObjectReaderDestroy, HsaExecutableCreateAlt,
		HsaExecutableDestroy, HsaExecutableFreeze, HsaExecutableGetSymbolByName, HsaExecutableLoadAgentCodeObject,
		HsaExecutableSymbolGetInfo, HsaInit, HsaIsaGetInfoAlt, HsaIterateAgents, HsaQueueCreate, HsaQueueDestroy,
		HsaQueueLoadReadIndexScAcquire, HsaQueueLoadWriteIndexRelaxed, HsaQueueStoreWriteIndexScRelease,
		HsaShutDown, HsaSignalCreate, HsaSignalDestroy, HsaSignalLoadScAcquire, HsaSignalStoreScRelease,
		HsaSignalWaitScAcquire, HsaStatus, HsaStatusString, HsaSystemGetInfo, STATUS_SUCCESS, }, };

struct Library { handle: NonNull<c_void>, display_name: String, }

impl Library { fn open(path: &OsStr) -> Result<Self> { let display_name = path.to_string_lossy().into_owned();
		let path = CString::new(path.as_bytes()).map_err(|_| { Error::PathContainsNul { path: display_name.clone(), } })?;

		// SAFETY: `path` is NUL terminated and remains alive for the call. The
		// returned handle is checked and is uniquely owned by `Library`.
		let handle = unsafe { clear_dlerror(); libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL) };
		let Some(handle) = NonNull::new(handle) else { return Err(Error::LibraryOpen { path: display_name,
				// SAFETY: `dlerror` returns a thread-local NUL-terminated
				// diagnostic, copied before any subsequent dynamic-loader call.
				detail: unsafe { take_dlerror() }
					.unwrap_or_else(|| "dynamic loader returned a null handle".to_owned()),
			}); };

		Ok(Self { handle, display_name, }) }

	fn symbol(&self, name: &'static str, name_with_nul: &'static [u8]) -> Result<*mut c_void> {
		debug_assert_eq!(name_with_nul.last(), Some(&0));
		// SAFETY: each call site supplies a static, NUL-terminated ASCII symbol
		// name. `self.handle` remains valid until every resolved pointer is
		// dropped with the enclosing `Api`.
		let address = unsafe { clear_dlerror(); libc::dlsym( self.handle.as_ptr(), name_with_nul.as_ptr().cast::<c_char>(), )
		};
		// POSIX requires inspecting `dlerror`, since a null address alone is
		// not sufficient to distinguish an error.
		// SAFETY: the diagnostic is immediately copied as described above.
		if let Some(detail) = unsafe { take_dlerror() } { return Err(Error::MissingSymbol {
				library: self.display_name.clone(), symbol: name, detail, }); }
		if address.is_null() { return Err(Error::MissingSymbol { library: self.display_name.clone(), symbol: name,
				detail: "symbol resolved to a null function address".to_owned(),
			}); }
		Ok(address) } }

impl Drop for Library { fn drop(&mut self) {
		// SAFETY: this is the one matching close for the successful `dlopen`.
		// Function pointers are fields dropped before this owner is destroyed.
		unsafe { libc::dlclose(self.handle.as_ptr()); } } }

// SAFETY: POSIX dynamic-loader handles may be used across threads. `Library`
// provides immutable symbol lookup after construction and performs exactly one
// close after all `Arc<Api>` owners are gone.
unsafe impl Send for Library {}
// SAFETY: symbol lookup state is immutable after `Api` construction, and POSIX
// permits concurrent calls through functions resolved from the same handle.
unsafe impl Sync for Library {}

// SAFETY: clears only this thread's loader diagnostic state.
unsafe fn clear_dlerror() {
	// SAFETY: `dlerror` takes no arguments and its result is intentionally
	// ignored when clearing the prior diagnostic.
	unsafe { libc::dlerror(); } }

// SAFETY: the caller must copy the returned thread-local diagnostic before
// making another loader call; this function performs that copy itself.
unsafe fn take_dlerror() -> Option<String> {
	// SAFETY: `dlerror` returns null or a valid NUL-terminated diagnostic owned
	// by the loader.
	let pointer = unsafe { libc::dlerror() }; (!pointer.is_null()).then(|| {
		// SAFETY: non-null `dlerror` results are NUL terminated.
		unsafe { CStr::from_ptr(pointer) } .to_string_lossy() .into_owned() }) }

pub(crate) struct Api { _library: Library, pub(crate) init: HsaInit, pub(crate) shut_down: HsaShutDown,
	pub(crate) status_string: HsaStatusString, pub(crate) system_get_info: HsaSystemGetInfo,
	pub(crate) iterate_agents: HsaIterateAgents, pub(crate) agent_get_info: HsaAgentGetInfo,
	pub(crate) agent_iterate_isas: HsaAgentIterateIsas, pub(crate) isa_get_info_alt: HsaIsaGetInfoAlt,
	pub(crate) queue_create: HsaQueueCreate, pub(crate) queue_destroy: HsaQueueDestroy,
	pub(crate) amd_agent_iterate_memory_pools: HsaAmdAgentIterateMemoryPools,
	pub(crate) amd_memory_pool_get_info: HsaAmdMemoryPoolGetInfo,
	pub(crate) code_object_reader_create_from_memory: HsaCodeObjectReaderCreateFromMemory,
	pub(crate) code_object_reader_destroy: HsaCodeObjectReaderDestroy,
	pub(crate) executable_create_alt: HsaExecutableCreateAlt, pub(crate) executable_destroy: HsaExecutableDestroy,
	pub(crate) executable_load_agent_code_object: HsaExecutableLoadAgentCodeObject,
	pub(crate) executable_freeze: HsaExecutableFreeze,
	pub(crate) executable_get_symbol_by_name: HsaExecutableGetSymbolByName,
	pub(crate) executable_symbol_get_info: HsaExecutableSymbolGetInfo,
	pub(crate) amd_memory_pool_allocate: HsaAmdMemoryPoolAllocate, pub(crate) amd_memory_pool_free: HsaAmdMemoryPoolFree,
	pub(crate) amd_agents_allow_access: HsaAmdAgentsAllowAccess, pub(crate) amd_memory_async_copy: HsaAmdMemoryAsyncCopy,
	pub(crate) signal_create: HsaSignalCreate, pub(crate) signal_destroy: HsaSignalDestroy,
	pub(crate) signal_load_scacquire: HsaSignalLoadScAcquire, pub(crate) signal_store_screlease: HsaSignalStoreScRelease,
	pub(crate) signal_wait_scacquire: HsaSignalWaitScAcquire,
	pub(crate) queue_load_read_index_scacquire: HsaQueueLoadReadIndexScAcquire,
	pub(crate) queue_load_write_index_relaxed: HsaQueueLoadWriteIndexRelaxed,
	pub(crate) queue_store_write_index_screlease: HsaQueueStoreWriteIndexScRelease, }

impl Api { pub(crate) fn load(path: &OsStr) -> Result<Self> { let library = Library::open(path)?;

		macro_rules! symbol { ($name:literal, $kind:ty) => {{
				let address = library.symbol($name, concat!($name, "\0").as_bytes())?;
				// SAFETY: the public ROCr header declares this symbol with
				// exactly `$kind`; ABI aliases in `abi.rs` mirror that header.
				unsafe { core::mem::transmute::<*mut c_void, $kind>(address) } }}; }

		let init = symbol!("hsa_init", HsaInit);
		let shut_down = symbol!("hsa_shut_down", HsaShutDown);
		let status_string = symbol!("hsa_status_string", HsaStatusString);
		let system_get_info = symbol!("hsa_system_get_info", HsaSystemGetInfo);
		let iterate_agents = symbol!("hsa_iterate_agents", HsaIterateAgents);
		let agent_get_info = symbol!("hsa_agent_get_info", HsaAgentGetInfo);
		let agent_iterate_isas = symbol!("hsa_agent_iterate_isas", HsaAgentIterateIsas);
		let isa_get_info_alt = symbol!("hsa_isa_get_info_alt", HsaIsaGetInfoAlt);
		let queue_create = symbol!("hsa_queue_create", HsaQueueCreate);
		let queue_destroy = symbol!("hsa_queue_destroy", HsaQueueDestroy);
		let amd_agent_iterate_memory_pools = symbol!(
			"hsa_amd_agent_iterate_memory_pools",
			HsaAmdAgentIterateMemoryPools );
		let amd_memory_pool_get_info = symbol!("hsa_amd_memory_pool_get_info", HsaAmdMemoryPoolGetInfo);
		let code_object_reader_create_from_memory = symbol!(
			"hsa_code_object_reader_create_from_memory",
			HsaCodeObjectReaderCreateFromMemory );
		let code_object_reader_destroy = symbol!("hsa_code_object_reader_destroy", HsaCodeObjectReaderDestroy);
		let executable_create_alt = symbol!("hsa_executable_create_alt", HsaExecutableCreateAlt);
		let executable_destroy = symbol!("hsa_executable_destroy", HsaExecutableDestroy);
		let executable_load_agent_code_object = symbol!(
			"hsa_executable_load_agent_code_object",
			HsaExecutableLoadAgentCodeObject );
		let executable_freeze = symbol!("hsa_executable_freeze", HsaExecutableFreeze);
		let executable_get_symbol_by_name = symbol!(
			"hsa_executable_get_symbol_by_name",
			HsaExecutableGetSymbolByName );
		let executable_symbol_get_info = symbol!("hsa_executable_symbol_get_info", HsaExecutableSymbolGetInfo);
		let amd_memory_pool_allocate = symbol!("hsa_amd_memory_pool_allocate", HsaAmdMemoryPoolAllocate);
		let amd_memory_pool_free = symbol!("hsa_amd_memory_pool_free", HsaAmdMemoryPoolFree);
		let amd_agents_allow_access = symbol!("hsa_amd_agents_allow_access", HsaAmdAgentsAllowAccess);
		let amd_memory_async_copy = symbol!("hsa_amd_memory_async_copy", HsaAmdMemoryAsyncCopy);
		let signal_create = symbol!("hsa_signal_create", HsaSignalCreate);
		let signal_destroy = symbol!("hsa_signal_destroy", HsaSignalDestroy);
		let signal_load_scacquire = symbol!("hsa_signal_load_scacquire", HsaSignalLoadScAcquire);
		let signal_store_screlease = symbol!("hsa_signal_store_screlease", HsaSignalStoreScRelease);
		let signal_wait_scacquire = symbol!("hsa_signal_wait_scacquire", HsaSignalWaitScAcquire);
		let queue_load_read_index_scacquire = symbol!(
			"hsa_queue_load_read_index_scacquire",
			HsaQueueLoadReadIndexScAcquire ); let queue_load_write_index_relaxed = symbol!(
			"hsa_queue_load_write_index_relaxed",
			HsaQueueLoadWriteIndexRelaxed ); let queue_store_write_index_screlease = symbol!(
			"hsa_queue_store_write_index_screlease",
			HsaQueueStoreWriteIndexScRelease );

		Ok(Self { _library: library, init, shut_down, status_string, system_get_info, iterate_agents, agent_get_info,
			agent_iterate_isas, isa_get_info_alt, queue_create, queue_destroy, amd_agent_iterate_memory_pools,
			amd_memory_pool_get_info, code_object_reader_create_from_memory, code_object_reader_destroy, executable_create_alt,
			executable_destroy, executable_load_agent_code_object, executable_freeze, executable_get_symbol_by_name,
			executable_symbol_get_info, amd_memory_pool_allocate, amd_memory_pool_free, amd_agents_allow_access,
			amd_memory_async_copy, signal_create, signal_destroy, signal_load_scacquire, signal_store_screlease,
			signal_wait_scacquire, queue_load_read_index_scacquire, queue_load_write_index_relaxed,
			queue_store_write_index_screlease, }) }

	pub(crate) fn check(&self, operation: &'static str, status: HsaStatus) -> Result<()> {
		if status == STATUS_SUCCESS { return Ok(()); }
		Err(Error::Hsa { operation, status, message: self.status_message(status), }) }

	/// Allocation-free status conversion for post-realization submit paths.
	///
	/// Rich runtime-owned status text is captured during discovery and
	/// realization. The live loop retains the numeric HSA status without
	/// allocating an owned diagnostic string.
	pub(crate) fn check_status_only(&self, operation: &'static str, status: HsaStatus) -> Result<()> {
		if status == STATUS_SUCCESS { return Ok(()); }
		Err(Error::Hsa { operation, status, message: None, }) }

	fn status_message(&self, status: HsaStatus) -> Option<String> { let mut pointer = std::ptr::null();
		// SAFETY: the runtime is initialized whenever `check` is used. The
		// output points to runtime-owned immutable NUL-terminated storage.
		let query_status = unsafe { (self.status_string)(status, &mut pointer) };
		if query_status != STATUS_SUCCESS || pointer.is_null() { return None; }
		// SAFETY: successful `hsa_status_string` returns a NUL-terminated
		// string valid for the initialized runtime's lifetime.
		Some(unsafe { CStr::from_ptr(pointer) } .to_string_lossy() .into_owned()) } }
