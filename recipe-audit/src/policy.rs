//! Exact interface and library policy.
//!
//! Rules operate on complete lexical tokens or normalized library basenames.
//! They never search arbitrary substrings.

/// Native interface family identified by policy.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeInterface {
	RocrHsa,
	CudaDriver,
	Hip,
	CudaRuntime,
	DirectKfd,
	RocBlas,
	RocSolver,
	RocFft,
	MiOpen,
	Rccl,
	CuBlas,
	CuSolver,
	CuFft,
	CuDnn,
	Nccl,
	/// A Driver-style `cuXxx` symbol not in the reviewed exact allowlist.
	CudaDriverOutsideAllowlist,
}

/// Result of classifying one complete symbol or library basename.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterfaceClassification {
	Allowed(NativeInterface),
	Prohibited(NativeInterface),
	Unknown,
}

impl InterfaceClassification {
	#[must_use]
	pub const fn is_prohibited(self) -> bool { matches!(self, Self::Prohibited(_)) }
}

/// Reviewed host-call surface. This is deliberately an exact symbol list.
///
/// Versioned ABI spellings are separate entries. Adding a new CUDA Driver call
/// therefore requires a visible policy change rather than widening a prefix.
pub const CUDA_DRIVER_API_ALLOWLIST: &[&str] = &[
	"cuCtxCreate",
	"cuCtxCreate_v2",
	"cuCtxDestroy",
	"cuCtxDestroy_v2",
	"cuCtxGetCurrent",
	"cuCtxGetDevice",
	"cuCtxPopCurrent",
	"cuCtxPopCurrent_v2",
	"cuCtxPushCurrent",
	"cuCtxPushCurrent_v2",
	"cuCtxSetCurrent",
	"cuDeviceCanAccessPeer",
	"cuDeviceComputeCapability",
	"cuDeviceGet",
	"cuDeviceGetAttribute",
	"cuDeviceGetCount",
	"cuDeviceGetName",
	"cuDeviceGetPCIBusId",
	"cuDeviceGetUuid",
	"cuDeviceGetUuid_v2",
	"cuDevicePrimaryCtxRelease",
	"cuDevicePrimaryCtxRelease_v2",
	"cuDevicePrimaryCtxRetain",
	"cuDeviceTotalMem",
	"cuDeviceTotalMem_v2",
	"cuDriverGetVersion",
	"cuEventCreate",
	"cuEventDestroy",
	"cuEventDestroy_v2",
	"cuEventElapsedTime",
	"cuEventQuery",
	"cuEventRecord",
	"cuEventSynchronize",
	"cuFuncGetAttribute",
	"cuFuncSetAttribute",
	"cuGetErrorName",
	"cuGetErrorString",
	"cuGraphAddDependencies",
	"cuGraphAddEmptyNode",
	"cuGraphAddEventRecordNode",
	"cuGraphAddEventWaitNode",
	"cuGraphAddKernelNode",
	"cuGraphAddMemcpyNode",
	"cuGraphDestroy",
	"cuGraphExecDestroy",
	"cuGraphInstantiate",
	"cuGraphInstantiate_v2",
	"cuGraphLaunch",
	"cuInit",
	"cuLaunchKernel",
	"cuMemAlloc",
	"cuMemAllocHost",
	"cuMemAllocHost_v2",
	"cuMemAlloc_v2",
	"cuMemFree",
	"cuMemFreeHost",
	"cuMemFree_v2",
	"cuMemGetInfo",
	"cuMemGetInfo_v2",
	"cuMemHostAlloc",
	"cuMemHostGetDevicePointer",
	"cuMemHostGetDevicePointer_v2",
	"cuMemHostRegister",
	"cuMemHostRegister_v2",
	"cuMemHostUnregister",
	"cuMemcpyDtoDAsync",
	"cuMemcpyDtoDAsync_v2",
	"cuMemcpyDtoHAsync",
	"cuMemcpyDtoHAsync_v2",
	"cuMemcpyHtoDAsync",
	"cuMemcpyHtoDAsync_v2",
	"cuMemcpyPeerAsync",
	"cuModuleGetFunction",
	"cuModuleGetGlobal",
	"cuModuleGetGlobal_v2",
	"cuModuleGetLoadingMode",
	"cuModuleLoad",
	"cuModuleLoadData",
	"cuModuleLoadDataEx",
	"cuModuleUnload",
	"cuPointerGetAttribute",
	"cuStreamCreate",
	"cuStreamCreateWithPriority",
	"cuStreamDestroy",
	"cuStreamDestroy_v2",
	"cuStreamGetPriority",
	"cuStreamQuery",
	"cuStreamSynchronize",
	"cuStreamWaitEvent",
];

const OPERATION_FAMILIES: &[(&str, NativeInterface)] = &[
	("rocblaslt", NativeInterface::RocBlas),
	("rocblas", NativeInterface::RocBlas),
	("rocsolver", NativeInterface::RocSolver),
	("rocfft", NativeInterface::RocFft),
	("miopen", NativeInterface::MiOpen),
	("rccl", NativeInterface::Rccl),
	("cublas", NativeInterface::CuBlas),
	("cusolver", NativeInterface::CuSolver),
	("cufft", NativeInterface::CuFft),
	("cudnn", NativeInterface::CuDnn),
	("nccl", NativeInterface::Nccl),
];

const HIP_INTERFACE_FAMILIES: &[&str] = &[
	"hip",
	"hipblas",
	"hipcub",
	"hipfft",
	"hiprand",
	"hiprtc",
	"hipsolver",
	"hipsparse",
];

/// Classify one complete source/IR/artifact symbol.
#[must_use]
pub fn classify_interface_symbol(symbol: &str) -> InterfaceClassification {
	let symbol = strip_symbol_decoration(symbol);

	if symbol.starts_with("hsaKmt") || symbol.starts_with("kfd_") {
		return InterfaceClassification::Prohibited(NativeInterface::DirectKfd);
	}
	if symbol.starts_with("hsa_") {
		return InterfaceClassification::Allowed(NativeInterface::RocrHsa);
	}

	if CUDA_DRIVER_API_ALLOWLIST.contains(&symbol) {
		return InterfaceClassification::Allowed(NativeInterface::CudaDriver);
	}

	if is_cuda_driver_shape(symbol) {
		return InterfaceClassification::Prohibited(NativeInterface::CudaDriverOutsideAllowlist);
	}
	if is_hip_symbol(symbol) {
		return InterfaceClassification::Prohibited(NativeInterface::Hip);
	}
	if is_cuda_runtime_symbol(symbol) {
		return InterfaceClassification::Prohibited(NativeInterface::CudaRuntime);
	}

	classify_operation_symbol(symbol).map_or(
		InterfaceClassification::Unknown,
		InterfaceClassification::Prohibited,
	)
}

/// Classify one complete linker or dynamic-library name.
#[must_use]
pub fn classify_library(name: &str) -> InterfaceClassification {
	let Some(stem) = normalized_library_stem(name) else {
		return InterfaceClassification::Unknown;
	};

	if matches!(stem.as_str(), "cuda" | "nvcuda") {
		return InterfaceClassification::Allowed(NativeInterface::CudaDriver);
	}
	if matches!(stem.as_str(), "hsa-runtime64" | "hsa-runtime") {
		return InterfaceClassification::Allowed(NativeInterface::RocrHsa);
	}
	if matches!(stem.as_str(), "hsakmt" | "kfd") {
		return InterfaceClassification::Prohibited(NativeInterface::DirectKfd);
	}
	if is_named_family(&stem, "amdhip64")
		|| HIP_INTERFACE_FAMILIES
			.iter()
			.any(|family| is_named_family(&stem, family))
	{
		return InterfaceClassification::Prohibited(NativeInterface::Hip);
	}
	if is_named_family(&stem, "cudart") {
		return InterfaceClassification::Prohibited(NativeInterface::CudaRuntime);
	}

	operation_library_family(&stem).map_or(
		InterfaceClassification::Unknown,
		InterfaceClassification::Prohibited,
	)
}

pub(crate) fn classify_dependency(name: &str) -> InterfaceClassification {
	let normalized = name.to_ascii_lowercase().replace('_', "-");

	if matches!(
		normalized.as_str(),
		"cuda-driver" | "cuda-driver-sys" | "hsa" | "hsa-sys" | "rocr" | "rocr-sys"
	) {
		return if normalized.starts_with("cuda") {
			InterfaceClassification::Allowed(NativeInterface::CudaDriver)
		} else {
			InterfaceClassification::Allowed(NativeInterface::RocrHsa)
		};
	}
	if HIP_INTERFACE_FAMILIES
		.iter()
		.any(|family| has_package_family(&normalized, family))
	{
		return InterfaceClassification::Prohibited(NativeInterface::Hip);
	}
	if has_package_family(&normalized, "cudart") || has_package_family(&normalized, "cuda-runtime") {
		return InterfaceClassification::Prohibited(NativeInterface::CudaRuntime);
	}
	if has_package_family(&normalized, "hsakmt") || has_package_family(&normalized, "kfd") {
		return InterfaceClassification::Prohibited(NativeInterface::DirectKfd);
	}

	OPERATION_FAMILIES
		.iter()
		.find_map(|(family, interface)| has_package_family(&normalized, family).then_some(*interface))
		.map_or(
			InterfaceClassification::Unknown,
			InterfaceClassification::Prohibited,
		)
}

fn strip_symbol_decoration(symbol: &str) -> &str {
	let symbol = symbol.strip_prefix('\u{1}').unwrap_or(symbol);
	symbol.split('@').next().unwrap_or(symbol)
}

fn is_cuda_driver_shape(symbol: &str) -> bool {
	let bytes = symbol.as_bytes();
	bytes.len() > 2 && bytes.starts_with(b"cu") && bytes[2].is_ascii_uppercase() && !symbol.starts_with("cuda")
}

fn is_hip_symbol(symbol: &str) -> bool {
	if matches!(
		symbol,
		"hip" | "hipcc" | "hipconfig" | "hiprtc" | "amdhip64"
	) {
		return true;
	}
	let plain = symbol.strip_prefix("__").unwrap_or(symbol);
	HIP_INTERFACE_FAMILIES
		.iter()
		.any(|family| has_identifier_family(plain, family))
}

fn is_cuda_runtime_symbol(symbol: &str) -> bool {
	let plain = symbol.strip_prefix("__").unwrap_or(symbol);
	matches!(plain, "cudart" | "cuda_runtime" | "cuda_runtime_api")
		|| plain
			.strip_prefix("cuda")
			.and_then(|suffix| suffix.as_bytes().first())
			.is_some_and(u8::is_ascii_uppercase)
}

fn classify_operation_symbol(symbol: &str) -> Option<NativeInterface> {
	OPERATION_FAMILIES.iter().find_map(|(family, interface)| {
		has_identifier_family_case_insensitive(symbol, family).then_some(*interface)
	})
}

fn operation_library_family(stem: &str) -> Option<NativeInterface> {
	OPERATION_FAMILIES
		.iter()
		.find_map(|(family, interface)| is_named_family(stem, family).then_some(*interface))
}

fn has_identifier_family(value: &str, family: &str) -> bool {
	value.strip_prefix(family).is_some_and(|suffix| {
		suffix.is_empty()
			|| suffix
				.as_bytes()
				.first()
				.is_some_and(|byte| byte.is_ascii_uppercase() || *byte == b'_')
	})
}

fn has_identifier_family_case_insensitive(value: &str, family: &str) -> bool {
	let Some(prefix) = value.get(..family.len()) else {
		return false;
	};
	if !prefix.eq_ignore_ascii_case(family) {
		return false;
	}
	let Some(suffix) = value.get(family.len()..) else {
		return false;
	};
	suffix.is_empty()
		|| suffix
			.as_bytes()
			.first()
			.is_some_and(|byte| byte.is_ascii_uppercase() || *byte == b'_' || byte.is_ascii_digit())
}

fn normalized_library_stem(value: &str) -> Option<String> {
	let token = value
		.trim()
		.trim_matches(|character| matches!(character, '"' | '\'' | '(' | ')' | '[' | ']'));
	if token.is_empty() {
		return None;
	}
	let token = token.rsplit(['/', '\\']).next().unwrap_or(token);
	let token = token
		.strip_prefix("-l")
		.or_else(|| token.strip_prefix("/DEFAULTLIB:"))
		.unwrap_or(token);
	let token = token.strip_prefix("lib").unwrap_or(token);
	let lowercase = token.to_ascii_lowercase();
	let end = [".so", ".dylib", ".dll", ".a", ".lib"]
		.iter()
		.filter_map(|suffix| lowercase.find(suffix))
		.min()
		.unwrap_or(lowercase.len());
	let stem = lowercase[..end].trim_end_matches("-static");
	(!stem.is_empty()).then(|| stem.to_owned())
}

fn is_named_family(stem: &str, family: &str) -> bool {
	let Some(suffix) = stem.strip_prefix(family) else {
		return false;
	};
	suffix.is_empty()
		|| matches!(suffix, "_static" | "-static" | "lt" | "lt_static")
		|| suffix
			.strip_prefix("64_")
			.is_some_and(|version| !version.is_empty() && version.bytes().all(|byte| byte.is_ascii_digit()))
		|| suffix
			.strip_prefix('_')
			.is_some_and(|rest| rest.bytes().all(|byte| byte.is_ascii_digit()))
}

fn has_package_family(name: &str, family: &str) -> bool {
	let Some(suffix) = name.strip_prefix(family) else {
		return false;
	};
	suffix.is_empty()
		|| matches!(
			suffix,
			"-sys" | "-src" | "-bindings" | "-runtime" | "-runtime-sys" | "-static" | "-wrapper"
		)
}
