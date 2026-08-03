use core::{ ffi::{CStr, c_char, c_int, c_uint, c_void}, mem, ptr::NonNull, };
use std::{collections::BTreeSet, ffi::CString};

use crate::error::{CudaError, Result};

pub(crate) type CuResult = c_int; pub(crate) type CuDevice = c_int; pub(crate) type CuContext = *mut c_void;
pub(crate) type CuDevicePtr = u64; pub(crate) type CuModule = *mut c_void; pub(crate) type CuFunction = *mut c_void;
pub(crate) type CuStream = *mut c_void; pub(crate) type CuEvent = *mut c_void;

pub(crate) const CUDA_SUCCESS: CuResult = 0; pub(crate) const CUDA_ERROR_NOT_READY: CuResult = 600;
pub(crate) const CUDA_ERROR_NOT_SUPPORTED: CuResult = 801;
pub(crate) const CU_DEVICE_ATTRIBUTE_MAX_THREADS_PER_BLOCK: c_int = 1;
pub(crate) const CU_DEVICE_ATTRIBUTE_MAX_SHARED_MEMORY_PER_BLOCK: c_int = 8;
pub(crate) const CU_DEVICE_ATTRIBUTE_WARP_SIZE: c_int = 10; pub(crate) const CU_DEVICE_ATTRIBUTE_CLOCK_RATE: c_int = 13;
pub(crate) const CU_DEVICE_ATTRIBUTE_MULTIPROCESSOR_COUNT: c_int = 16;
pub(crate) const CU_DEVICE_ATTRIBUTE_CONCURRENT_KERNELS: c_int = 31;
pub(crate) const CU_DEVICE_ATTRIBUTE_PCI_BUS_ID: c_int = 33;
pub(crate) const CU_DEVICE_ATTRIBUTE_PCI_DEVICE_ID: c_int = 34;
pub(crate) const CU_DEVICE_ATTRIBUTE_MEMORY_CLOCK_RATE: c_int = 36;
pub(crate) const CU_DEVICE_ATTRIBUTE_GLOBAL_MEMORY_BUS_WIDTH: c_int = 37;
pub(crate) const CU_DEVICE_ATTRIBUTE_ASYNC_ENGINE_COUNT: c_int = 40;
pub(crate) const CU_DEVICE_ATTRIBUTE_PCI_DOMAIN_ID: c_int = 50;
pub(crate) const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MAJOR: c_int = 75;
pub(crate) const CU_DEVICE_ATTRIBUTE_COMPUTE_CAPABILITY_MINOR: c_int = 76;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct CuUuid { pub(crate) bytes: [u8; 16], }

pub(crate) type CuInit = unsafe extern "C" fn(c_uint) -> CuResult;
pub(crate) type CuDriverGetVersion = unsafe extern "C" fn(*mut c_int) -> CuResult;
pub(crate) type CuDeviceGetCount = unsafe extern "C" fn(*mut c_int) -> CuResult;
pub(crate) type CuDeviceGet = unsafe extern "C" fn(*mut CuDevice, c_int) -> CuResult;
pub(crate) type CuDeviceGetName = unsafe extern "C" fn(*mut c_char, c_int, CuDevice) -> CuResult;
pub(crate) type CuDeviceGetUuid = unsafe extern "C" fn(*mut CuUuid, CuDevice) -> CuResult;
pub(crate) type CuDeviceGetPciBusId = unsafe extern "C" fn(*mut c_char, c_int, CuDevice) -> CuResult;
pub(crate) type CuDeviceTotalMemV2 = unsafe extern "C" fn(*mut usize, CuDevice) -> CuResult;
pub(crate) type CuDeviceGetAttribute = unsafe extern "C" fn(*mut c_int, c_int, CuDevice) -> CuResult;
pub(crate) type CuCtxCreateV2 = unsafe extern "C" fn(*mut CuContext, c_uint, CuDevice) -> CuResult;
pub(crate) type CuCtxDestroyV2 = unsafe extern "C" fn(CuContext) -> CuResult;
pub(crate) type CuCtxPushCurrentV2 = unsafe extern "C" fn(CuContext) -> CuResult;
pub(crate) type CuCtxPopCurrentV2 = unsafe extern "C" fn(*mut CuContext) -> CuResult;
pub(crate) type CuModuleLoadData = unsafe extern "C" fn(*mut CuModule, *const c_void) -> CuResult;
pub(crate) type CuModuleUnload = unsafe extern "C" fn(CuModule) -> CuResult;
pub(crate) type CuModuleGetFunction = unsafe extern "C" fn(*mut CuFunction, CuModule, *const c_char) -> CuResult;
pub(crate) type CuMemAllocV2 = unsafe extern "C" fn(*mut CuDevicePtr, usize) -> CuResult;
pub(crate) type CuMemFreeV2 = unsafe extern "C" fn(CuDevicePtr) -> CuResult;
pub(crate) type CuMemGetInfoV2 = unsafe extern "C" fn(*mut usize, *mut usize) -> CuResult;
pub(crate) type CuMemHostAlloc = unsafe extern "C" fn(*mut *mut c_void, usize, c_uint) -> CuResult;
pub(crate) type CuMemFreeHost = unsafe extern "C" fn(*mut c_void) -> CuResult;
pub(crate) type CuStreamCreate = unsafe extern "C" fn(*mut CuStream, c_uint) -> CuResult;
pub(crate) type CuStreamDestroyV2 = unsafe extern "C" fn(CuStream) -> CuResult;
pub(crate) type CuStreamQuery = unsafe extern "C" fn(CuStream) -> CuResult;
pub(crate) type CuEventCreate = unsafe extern "C" fn(*mut CuEvent, c_uint) -> CuResult;
pub(crate) type CuEventRecord = unsafe extern "C" fn(CuEvent, CuStream) -> CuResult;
pub(crate) type CuEventQuery = unsafe extern "C" fn(CuEvent) -> CuResult;
pub(crate) type CuEventDestroyV2 = unsafe extern "C" fn(CuEvent) -> CuResult;
pub(crate) type CuMemcpyHtoDAsyncV2 = unsafe extern "C" fn(CuDevicePtr, *const c_void, usize, CuStream) -> CuResult;
pub(crate) type CuMemcpyDtoHAsyncV2 = unsafe extern "C" fn(*mut c_void, CuDevicePtr, usize, CuStream) -> CuResult;
pub(crate) type CuMemcpyDtoDAsyncV2 = unsafe extern "C" fn(CuDevicePtr, CuDevicePtr, usize, CuStream) -> CuResult;
pub(crate) type CuLaunchKernel = unsafe extern "C" fn(
	CuFunction, c_uint, c_uint, c_uint, c_uint, c_uint, c_uint, c_uint, CuStream, *mut *mut c_void, *mut *mut c_void,
) -> CuResult;
pub(crate) type CuModuleGetLoadingMode = unsafe extern "C" fn(*mut c_int) -> CuResult;
pub(crate) type CuGetErrorName = unsafe extern "C" fn(CuResult, *mut *const c_char) -> CuResult;
pub(crate) type CuGetErrorString = unsafe extern "C" fn(CuResult, *mut *const c_char) -> CuResult;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum DriverSymbol { Init, DriverGetVersion, DeviceGetCount, DeviceGet, DeviceGetName, DeviceGetUuid,
	DeviceGetUuidV2, DeviceGetPciBusId, DeviceTotalMemV2, DeviceGetAttribute, CtxCreateV2, CtxDestroyV2, CtxPushCurrentV2,
	CtxPopCurrentV2, ModuleLoadData, ModuleUnload, ModuleGetFunction, MemAllocV2, MemFreeV2, MemGetInfoV2, MemHostAlloc,
	MemFreeHost, StreamCreate, StreamDestroyV2, StreamQuery, EventCreate, EventRecord, EventQuery, EventDestroyV2,
	MemcpyHtoDAsyncV2, MemcpyDtoHAsyncV2, MemcpyDtoDAsyncV2, LaunchKernel, ModuleGetLoadingMode, GetErrorName,
	GetErrorString, }

impl DriverSymbol {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Init => "cuInit",
			Self::DriverGetVersion => "cuDriverGetVersion",
			Self::DeviceGetCount => "cuDeviceGetCount",
			Self::DeviceGet => "cuDeviceGet",
			Self::DeviceGetName => "cuDeviceGetName",
			Self::DeviceGetUuid => "cuDeviceGetUuid",
			Self::DeviceGetUuidV2 => "cuDeviceGetUuid_v2",
			Self::DeviceGetPciBusId => "cuDeviceGetPCIBusId",
			Self::DeviceTotalMemV2 => "cuDeviceTotalMem_v2",
			Self::DeviceGetAttribute => "cuDeviceGetAttribute",
			Self::CtxCreateV2 => "cuCtxCreate_v2",
			Self::CtxDestroyV2 => "cuCtxDestroy_v2",
			Self::CtxPushCurrentV2 => "cuCtxPushCurrent_v2",
			Self::CtxPopCurrentV2 => "cuCtxPopCurrent_v2",
			Self::ModuleLoadData => "cuModuleLoadData",
			Self::ModuleUnload => "cuModuleUnload",
			Self::ModuleGetFunction => "cuModuleGetFunction",
			Self::MemAllocV2 => "cuMemAlloc_v2",
			Self::MemFreeV2 => "cuMemFree_v2",
			Self::MemGetInfoV2 => "cuMemGetInfo_v2",
			Self::MemHostAlloc => "cuMemHostAlloc",
			Self::MemFreeHost => "cuMemFreeHost",
			Self::StreamCreate => "cuStreamCreate",
			Self::StreamDestroyV2 => "cuStreamDestroy_v2",
			Self::StreamQuery => "cuStreamQuery",
			Self::EventCreate => "cuEventCreate",
			Self::EventRecord => "cuEventRecord",
			Self::EventQuery => "cuEventQuery",
			Self::EventDestroyV2 => "cuEventDestroy_v2",
			Self::MemcpyHtoDAsyncV2 => "cuMemcpyHtoDAsync_v2",
			Self::MemcpyDtoHAsyncV2 => "cuMemcpyDtoHAsync_v2",
			Self::MemcpyDtoDAsyncV2 => "cuMemcpyDtoDAsync_v2",
			Self::LaunchKernel => "cuLaunchKernel",
			Self::ModuleGetLoadingMode => "cuModuleGetLoadingMode",
			Self::GetErrorName => "cuGetErrorName",
			Self::GetErrorString => "cuGetErrorString",
		} }

	fn c_name(self) -> &'static CStr {
		match self {
			Self::Init => c"cuInit",
			Self::DriverGetVersion => c"cuDriverGetVersion",
			Self::DeviceGetCount => c"cuDeviceGetCount",
			Self::DeviceGet => c"cuDeviceGet",
			Self::DeviceGetName => c"cuDeviceGetName",
			Self::DeviceGetUuid => c"cuDeviceGetUuid",
			Self::DeviceGetUuidV2 => c"cuDeviceGetUuid_v2",
			Self::DeviceGetPciBusId => c"cuDeviceGetPCIBusId",
			Self::DeviceTotalMemV2 => c"cuDeviceTotalMem_v2",
			Self::DeviceGetAttribute => c"cuDeviceGetAttribute",
			Self::CtxCreateV2 => c"cuCtxCreate_v2",
			Self::CtxDestroyV2 => c"cuCtxDestroy_v2",
			Self::CtxPushCurrentV2 => c"cuCtxPushCurrent_v2",
			Self::CtxPopCurrentV2 => c"cuCtxPopCurrent_v2",
			Self::ModuleLoadData => c"cuModuleLoadData",
			Self::ModuleUnload => c"cuModuleUnload",
			Self::ModuleGetFunction => c"cuModuleGetFunction",
			Self::MemAllocV2 => c"cuMemAlloc_v2",
			Self::MemFreeV2 => c"cuMemFree_v2",
			Self::MemGetInfoV2 => c"cuMemGetInfo_v2",
			Self::MemHostAlloc => c"cuMemHostAlloc",
			Self::MemFreeHost => c"cuMemFreeHost",
			Self::StreamCreate => c"cuStreamCreate",
			Self::StreamDestroyV2 => c"cuStreamDestroy_v2",
			Self::StreamQuery => c"cuStreamQuery",
			Self::EventCreate => c"cuEventCreate",
			Self::EventRecord => c"cuEventRecord",
			Self::EventQuery => c"cuEventQuery",
			Self::EventDestroyV2 => c"cuEventDestroy_v2",
			Self::MemcpyHtoDAsyncV2 => c"cuMemcpyHtoDAsync_v2",
			Self::MemcpyDtoHAsyncV2 => c"cuMemcpyDtoHAsync_v2",
			Self::MemcpyDtoDAsyncV2 => c"cuMemcpyDtoDAsync_v2",
			Self::LaunchKernel => c"cuLaunchKernel",
			Self::ModuleGetLoadingMode => c"cuModuleGetLoadingMode",
			Self::GetErrorName => c"cuGetErrorName",
			Self::GetErrorString => c"cuGetErrorString",
		} } }

pub const REQUIRED_DRIVER_SYMBOLS: &[DriverSymbol] = &[ DriverSymbol::Init, DriverSymbol::DriverGetVersion,
	DriverSymbol::DeviceGetCount, DriverSymbol::DeviceGet, DriverSymbol::DeviceGetName, DriverSymbol::DeviceGetUuid,
	DriverSymbol::DeviceGetPciBusId, DriverSymbol::DeviceTotalMemV2, DriverSymbol::DeviceGetAttribute,
	DriverSymbol::CtxCreateV2, DriverSymbol::CtxDestroyV2, DriverSymbol::CtxPushCurrentV2, DriverSymbol::CtxPopCurrentV2,
	DriverSymbol::ModuleLoadData, DriverSymbol::ModuleUnload, DriverSymbol::ModuleGetFunction, DriverSymbol::MemAllocV2,
	DriverSymbol::MemFreeV2, DriverSymbol::MemGetInfoV2, DriverSymbol::MemHostAlloc, DriverSymbol::MemFreeHost,
	DriverSymbol::StreamCreate, DriverSymbol::StreamDestroyV2, DriverSymbol::StreamQuery, DriverSymbol::EventCreate,
	DriverSymbol::EventRecord, DriverSymbol::EventQuery, DriverSymbol::EventDestroyV2, DriverSymbol::MemcpyHtoDAsyncV2,
	DriverSymbol::MemcpyDtoHAsyncV2, DriverSymbol::MemcpyDtoDAsyncV2, DriverSymbol::LaunchKernel, ];

pub const OPTIONAL_DRIVER_SYMBOLS: &[DriverSymbol] = &[ DriverSymbol::DeviceGetUuidV2,
	DriverSymbol::ModuleGetLoadingMode, DriverSymbol::GetErrorName, DriverSymbol::GetErrorString, ];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverCapabilities { available: BTreeSet<DriverSymbol>, }

impl DriverCapabilities { pub fn supports(&self, symbol: DriverSymbol) -> bool { self.available.contains(&symbol) }


	pub(crate) fn from_available(available: BTreeSet<DriverSymbol>) -> Self { Self { available } } }

pub(crate) struct Api { pub(crate) init: CuInit, pub(crate) driver_get_version: CuDriverGetVersion,
	pub(crate) device_get_count: CuDeviceGetCount, pub(crate) device_get: CuDeviceGet,
	pub(crate) device_get_name: CuDeviceGetName, pub(crate) device_get_uuid: CuDeviceGetUuid,
	pub(crate) device_get_uuid_v2: Option<CuDeviceGetUuid>, pub(crate) device_get_pci_bus_id: CuDeviceGetPciBusId,
	pub(crate) device_total_mem_v2: CuDeviceTotalMemV2, pub(crate) device_get_attribute: CuDeviceGetAttribute,
	pub(crate) ctx_create_v2: CuCtxCreateV2, pub(crate) ctx_destroy_v2: CuCtxDestroyV2,
	pub(crate) ctx_push_current_v2: CuCtxPushCurrentV2, pub(crate) ctx_pop_current_v2: CuCtxPopCurrentV2,
	pub(crate) module_load_data: CuModuleLoadData, pub(crate) module_unload: CuModuleUnload,
	pub(crate) module_get_function: CuModuleGetFunction, pub(crate) mem_alloc_v2: CuMemAllocV2,
	pub(crate) mem_free_v2: CuMemFreeV2, pub(crate) mem_get_info_v2: CuMemGetInfoV2,
	pub(crate) mem_host_alloc: CuMemHostAlloc, pub(crate) mem_free_host: CuMemFreeHost,
	pub(crate) stream_create: CuStreamCreate, pub(crate) stream_destroy_v2: CuStreamDestroyV2,
	pub(crate) stream_query: CuStreamQuery, pub(crate) event_create: CuEventCreate, pub(crate) event_record: CuEventRecord,
	pub(crate) event_query: CuEventQuery, pub(crate) event_destroy_v2: CuEventDestroyV2,
	pub(crate) memcpy_htod_async_v2: CuMemcpyHtoDAsyncV2, pub(crate) memcpy_dtoh_async_v2: CuMemcpyDtoHAsyncV2,
	pub(crate) memcpy_dtod_async_v2: CuMemcpyDtoDAsyncV2, pub(crate) launch_kernel: CuLaunchKernel,
	pub(crate) get_error_name: Option<CuGetErrorName>, pub(crate) get_error_string: Option<CuGetErrorString>,
	capabilities: DriverCapabilities, }

trait SymbolResolver {
	unsafe fn lookup(&self, symbol: DriverSymbol) -> core::result::Result<Option<NonNull<c_void>>, String>; }

impl Api { fn load(resolver: &impl SymbolResolver) -> Result<Self> { let init = required(resolver, DriverSymbol::Init)?;
		let driver_get_version = required(resolver, DriverSymbol::DriverGetVersion)?;
		let device_get_count = required(resolver, DriverSymbol::DeviceGetCount)?;
		let device_get = required(resolver, DriverSymbol::DeviceGet)?;
		let device_get_name = required(resolver, DriverSymbol::DeviceGetName)?;
		let device_get_uuid = required(resolver, DriverSymbol::DeviceGetUuid)?;
		let device_get_pci_bus_id = required(resolver, DriverSymbol::DeviceGetPciBusId)?;
		let device_total_mem_v2 = required(resolver, DriverSymbol::DeviceTotalMemV2)?;
		let device_get_attribute = required(resolver, DriverSymbol::DeviceGetAttribute)?;
		let ctx_create_v2 = required(resolver, DriverSymbol::CtxCreateV2)?;
		let ctx_destroy_v2 = required(resolver, DriverSymbol::CtxDestroyV2)?;
		let ctx_push_current_v2 = required(resolver, DriverSymbol::CtxPushCurrentV2)?;
		let ctx_pop_current_v2 = required(resolver, DriverSymbol::CtxPopCurrentV2)?;
		let module_load_data = required(resolver, DriverSymbol::ModuleLoadData)?;
		let module_unload = required(resolver, DriverSymbol::ModuleUnload)?;
		let module_get_function = required(resolver, DriverSymbol::ModuleGetFunction)?;
		let mem_alloc_v2 = required(resolver, DriverSymbol::MemAllocV2)?;
		let mem_free_v2 = required(resolver, DriverSymbol::MemFreeV2)?;
		let mem_get_info_v2 = required(resolver, DriverSymbol::MemGetInfoV2)?;
		let mem_host_alloc = required(resolver, DriverSymbol::MemHostAlloc)?;
		let mem_free_host = required(resolver, DriverSymbol::MemFreeHost)?;
		let stream_create = required(resolver, DriverSymbol::StreamCreate)?;
		let stream_destroy_v2 = required(resolver, DriverSymbol::StreamDestroyV2)?;
		let stream_query = required(resolver, DriverSymbol::StreamQuery)?;
		let event_create = required(resolver, DriverSymbol::EventCreate)?;
		let event_record = required(resolver, DriverSymbol::EventRecord)?;
		let event_query = required(resolver, DriverSymbol::EventQuery)?;
		let event_destroy_v2 = required(resolver, DriverSymbol::EventDestroyV2)?;
		let memcpy_htod_async_v2 = required(resolver, DriverSymbol::MemcpyHtoDAsyncV2)?;
		let memcpy_dtoh_async_v2 = required(resolver, DriverSymbol::MemcpyDtoHAsyncV2)?;
		let memcpy_dtod_async_v2 = required(resolver, DriverSymbol::MemcpyDtoDAsyncV2)?;
		let launch_kernel = required(resolver, DriverSymbol::LaunchKernel)?;

		let device_get_uuid_v2 = optional(resolver, DriverSymbol::DeviceGetUuidV2);
		let module_get_loading_mode: Option<CuModuleGetLoadingMode> = optional(resolver, DriverSymbol::ModuleGetLoadingMode);
		let get_error_name = optional(resolver, DriverSymbol::GetErrorName);
		let get_error_string = optional(resolver, DriverSymbol::GetErrorString);

		let mut available = BTreeSet::from_iter(REQUIRED_DRIVER_SYMBOLS.iter().copied()); for (symbol, present) in [
			(DriverSymbol::DeviceGetUuidV2, device_get_uuid_v2.is_some()), ( DriverSymbol::ModuleGetLoadingMode,
				module_get_loading_mode.is_some(), ), (DriverSymbol::GetErrorName, get_error_name.is_some()),
			(DriverSymbol::GetErrorString, get_error_string.is_some()), ] { if present { available.insert(symbol); } }

		Ok(Self { init, driver_get_version, device_get_count, device_get, device_get_name, device_get_uuid,
			device_get_uuid_v2, device_get_pci_bus_id, device_total_mem_v2, device_get_attribute, ctx_create_v2, ctx_destroy_v2,
			ctx_push_current_v2, ctx_pop_current_v2, module_load_data, module_unload, module_get_function, mem_alloc_v2,
			mem_free_v2, mem_get_info_v2, mem_host_alloc, mem_free_host, stream_create, stream_destroy_v2, stream_query,
			event_create, event_record, event_query, event_destroy_v2, memcpy_htod_async_v2, memcpy_dtoh_async_v2,
			memcpy_dtod_async_v2, launch_kernel, get_error_name, get_error_string,
			capabilities: DriverCapabilities::from_available(available), }) }

	pub(crate) fn capabilities(&self) -> &DriverCapabilities { &self.capabilities } }

fn required<T: Copy>(resolver: &impl SymbolResolver, symbol: DriverSymbol) -> Result<T> {
	let lookup = unsafe { resolver.lookup(symbol) }; match lookup {
		Ok(Some(pointer)) => Ok(unsafe { pointer_as_function(pointer) }), Ok(None) => { Err(CudaError::MissingRequiredSymbol {
				symbol: symbol.as_str(), detail: None, }) }
		Err(detail) => { Err(CudaError::MissingRequiredSymbol { symbol: symbol.as_str(), detail: Some(detail), }) } } }

fn optional<T: Copy>(resolver: &impl SymbolResolver, symbol: DriverSymbol) -> Option<T> {
	let pointer = unsafe { resolver.lookup(symbol) }.ok().flatten()?; Some(unsafe { pointer_as_function(pointer) }) }

unsafe fn pointer_as_function<T: Copy>(pointer: NonNull<c_void>) -> T {
	assert_eq!(mem::size_of::<T>(), mem::size_of::<*mut c_void>()); unsafe { mem::transmute_copy(&pointer.as_ptr()) } }

pub(crate) struct DynamicLibrary { handle: NonNull<c_void>, name: String, }

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary { pub(crate) fn open_default() -> Result<Self> { let mut attempts = Vec::new();
		for candidate in ["libcuda.so.1", "libcuda.so"] {
			match Self::open(candidate) { Ok(library) => return Ok(library), Err(CudaError::LibraryOpen {
					attempts: candidate_attempts, }) => attempts.extend(candidate_attempts), Err(error) => return Err(error), } }
		Err(CudaError::LibraryOpen { attempts }) }

	pub(crate) fn open(path: &str) -> Result<Self> { let path_c = CString::new(path).map_err(|_| {
			CudaError::InvalidLibraryPath { path: path.to_owned(), } })?; unsafe { clear_dlerror();
			let handle = libc::dlopen(path_c.as_ptr(), libc::RTLD_NOW | libc::RTLD_LOCAL);
			let Some(handle) = NonNull::new(handle) else {
				let detail = take_dlerror().unwrap_or_else(|| "unknown dlopen error".to_owned());
				return Err(CudaError::LibraryOpen {
					attempts: vec![format!("{path}: {detail}")],
				}); }; Ok(Self { handle, name: path.to_owned(), }) } }

	pub(crate) fn load_api(&self) -> Result<Api> { Api::load(self) }

	pub(crate) fn name(&self) -> &str { &self.name } }

impl SymbolResolver for DynamicLibrary {
	unsafe fn lookup(&self, symbol: DriverSymbol) -> core::result::Result<Option<NonNull<c_void>>, String> { unsafe {
			clear_dlerror(); let pointer = libc::dlsym(self.handle.as_ptr(), symbol.c_name().as_ptr());
			let detail = take_dlerror(); if let Some(detail) = detail { return Err(detail); }
			Ok(NonNull::new(pointer)) } } }

impl Drop for DynamicLibrary { fn drop(&mut self) { unsafe { let _ = libc::dlclose(self.handle.as_ptr()); } } }

unsafe fn clear_dlerror() { unsafe { let _ = libc::dlerror(); } }

unsafe fn take_dlerror() -> Option<String> { let pointer = unsafe { libc::dlerror() }; if pointer.is_null() {
		return None; }
	Some(unsafe { CStr::from_ptr(pointer) } .to_string_lossy() .into_owned()) }
