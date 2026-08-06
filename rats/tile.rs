use super::*;

#[cfg(feature = "amd")]
const RDNA3_GFX1101: u32 = 110001;
#[cfg(feature = "amd")]
const RDNA3_VGPRS_PER_SIMD: u32 = 1536;
#[cfg(feature = "amd")]
const RDNA3_VGPR_GRANULE: u32 = 24;
const DOUBLE_BUFFER_VALUES: u32 = 2;

#[derive(Clone, Copy)]
pub(super) struct Resources {
	pub registers: u32,
	pub shared: u32,
	pub max_block: u32,
}

#[derive(Clone, Copy)]
pub(super) struct Geometry {
	pub groups: u32,
	pub block: u32,
}

impl Geometry {
	pub fn threads(self) -> Result<u32> {
		self.groups
			.checked_mul(self.block)
			.filter(|value| *value != 0)
			.ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	}
}

#[cfg(feature = "amd")]
fn property(text: &str, name: &str) -> Result<u32> {
	text.lines()
		.find_map(|line| line.split_once(' ').filter(|value| value.0 == name))
		.ok_or_else(|| RecipeError::new(format!("KFD property {name:?} is absent")))?
		.1
		.parse::<u32>()
		.map_err(|error| RecipeError::new(format!("KFD property {name:?} is invalid: {error}")))
}

fn geometry(
	cus: u32,
	wave: u32,
	workgroup: u32,
	lds: u32,
	groups_per_cu: u32,
	resources: Resources,
) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= resources.max_block, "GPU wave exceeds the kernel workgroup")?;
	let waves = groups_per_cu.min(workgroup / wave).min(resources.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let tile = block
		.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("GPU tile size overflows"))?;
	let shared = resources.shared.max(tile);
	require(shared != 0 && shared <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
}

#[cfg(feature = "amd")]
pub(super) fn amd(
	cus: u32,
	wave: u32,
	workgroup: u32,
	waves_per_cu: u32,
	simds_per_cu: u32,
	node: u32,
	resources: Resources,
) -> Result<Geometry> {
	let path = format!("/sys/class/kfd/kfd/topology/nodes/{node}/properties");
	let text = fs::read_to_string(&path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
	let gfx = property(&text, "gfx_target_version")?;
	require(gfx == RDNA3_GFX1101, format!("GPU target {gfx} does not match the compiled gfx1101 kernel"))?;
	let lds = property(&text, "lds_size_in_kb")?
		.checked_mul(1024)
		.ok_or_else(|| RecipeError::new("AMD LDS size overflows"))?;
	let registers = resources.registers.div_ceil(RDNA3_VGPR_GRANULE) * RDNA3_VGPR_GRANULE;
	require(registers != 0, "AMD kernel register count is absent")?;
	let register_waves = RDNA3_VGPRS_PER_SIMD / registers * simds_per_cu;
	geometry(cus, wave, workgroup, lds, waves_per_cu.min(register_waves), resources)
}

#[cfg(feature = "nvidia")]
pub(super) fn nvidia(
	cus: u32,
	wave: u32,
	workgroup: u32,
	block_lds: u32,
	sm_lds: u32,
	waves_per_cu: u32,
	resources: Resources,
) -> Result<Geometry> {
	require(resources.registers != 0, "Nvidia kernel register count is absent")?;
	let tile = wave
		.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("Nvidia tile size overflows"))?;
	require(resources.shared.max(tile) <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, waves_per_cu, resources)
}

pub(super) struct Buffer {
	runtime: &'static Gpu, 	pub(super) pointer: u64, }
impl Buffer { 	pub(super) fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
		Ok(Self { runtime, pointer: runtime.allocate(bytes)? }) 	}
	pub(super) fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
		let buffer = Self::new(runtime, size_of_val(values))?;
		runtime.upload(buffer.pointer, values.as_ptr().cast(), size_of_val(values))?; 		Ok(buffer) 	}
	pub(super) fn download<T: Copy + Default>(&self, count: usize) -> Result<Vec<T>> { 		self.runtime.synchronize()?;
		let mut values = std::iter::repeat_n(T::default(), count).collect::<Vec<_>>();
		self.runtime.download(values.as_mut_ptr().cast(), self.pointer, size_of_val(&*values))?; 		Ok(values) 	} }
impl Drop for Buffer { 	fn drop(&mut self) { 		self.runtime.free(self.pointer); 	} } #[derive(Clone, Copy)]
pub(super) struct Kernel { 	object: u64, 	#[cfg(feature = "amd")] 	kernarg: usize, 	#[cfg(feature = "amd")] 	group: u32,
	#[cfg(feature = "amd")] 	private: u32, 	#[cfg(feature = "amd")] 	layout: &'static [u8], }
const FORWARD_ARGS: &[u8] = b"888888444";
const EPOCH_ARGS: &[u8] = b"8888888888888888444488888888844"; #[cfg(feature = "nvidia")] struct Cuda {
	allocate: unsafe extern "C" fn(*mut u64, usize) -> i32, 	free: unsafe extern "C" fn(u64) -> i32,
	upload: unsafe extern "C" fn(u64, *const c_void, usize) -> i32,
	download: unsafe extern "C" fn(Ptr, u64, usize) -> i32, 	synchronize: unsafe extern "C" fn() -> i32,
	launch: unsafe extern "C" fn(usize, u32, u32, u32, u32, u32, u32, u32, Ptr, *mut Ptr) -> i32, }
#[cfg(feature = "nvidia")] impl Kernel { 	const fn cuda(object: usize, _layout: &'static [u8]) -> Self { 		Self {
			object: object as u64, 			#[cfg(feature = "amd")] 			kernarg: 0, 			#[cfg(feature = "amd")] 			group: 0,
			#[cfg(feature = "amd")] 			private: 0, 			#[cfg(feature = "amd")] 			layout: _layout, 		} 	} }
#[cfg(feature = "amd")] #[allow(dead_code)] struct Hsa {
	allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32, 	free: unsafe extern "C" fn(Ptr) -> i32,
	allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32,
	copy: unsafe extern "C" fn(Ptr, *const c_void, usize) -> i32, 	store: unsafe extern "C" fn(u64, i64),
	wait: unsafe extern "C" fn(u64, i32, i64, u64, i32) -> i64, 	write: unsafe extern "C" fn(*const HsaQueue, u64) -> u64,
	queue: Ptr, 	signal: u64, 	cpu_agent: u64, 	vram_pool: u64, 	kernarg_pool: u64, 	kernarg_size: usize, 	kernarg: Ptr,
	_code: fs::File, } enum Driver { 	#[cfg(feature = "amd")] 	Hsa(Hsa), 	#[cfg(feature = "nvidia")] 	Cuda(Cuda), }
#[allow(dead_code)] pub(super) struct Gpu {
	backend: Backend, 	driver: Driver, 	geometry: Geometry, 	pub(super) forward: Kernel,
	pub(super) epoch: Kernel, 	dispatch: Mutex<()>, }
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}
#[cfg(feature = "amd")]
#[repr(C)] struct HsaQueue { 	kind: u32, 	features: u32, 	base: Ptr, 	doorbell: u64, 	size: u32, 	reserved: u32,
	id: u64, } #[cfg(feature = "amd")] #[repr(C)] struct HsaPacket { 	header: u16, 	setup: u16, 	workgroup_x: u16,
	workgroup_y: u16, 	workgroup_z: u16, 	reserved0: u16, 	grid_x: u32, 	grid_y: u32, 	grid_z: u32, 	private: u32,
	group: u32, 	object: u64, 	kernarg: Ptr, 	reserved1: u64, 	completion: u64, } #[cfg(feature = "nvidia")]
type Count = unsafe extern "C" fn(*mut i32) -> i32; #[cfg(feature = "nvidia")]
type Attribute = unsafe extern "C" fn(*mut i32, i32, i32) -> i32; #[cfg(feature = "nvidia")]
type Device = unsafe extern "C" fn(*mut i32, i32) -> i32; #[cfg(feature = "nvidia")]
type Context = unsafe extern "C" fn(*mut Ptr, u32, i32) -> i32; #[cfg(feature = "nvidia")]
type Module = unsafe extern "C" fn(*mut Ptr, *const u8) -> i32; #[cfg(feature = "nvidia")]
type Function = unsafe extern "C" fn(*mut usize, Ptr, *const u8) -> i32;
#[cfg(feature = "nvidia")]
type FunctionAttribute = unsafe extern "C" fn(*mut i32, i32, usize) -> i32;
#[cfg(feature = "nvidia")]
type Occupancy = unsafe extern "C" fn(*mut i32, usize, i32, usize) -> i32;
#[cfg(any(feature = "amd", feature = "nvidia"))] struct Library(Ptr); #[cfg(any(feature = "amd", feature = "nvidia"))]
impl Library { 	fn open(name: &str) -> Result<Self> { 		let name = format!("{name}\0");
		let handle = unsafe { dlopen(name.as_ptr().cast(), 2) };
		require(!handle.is_null(), format!("cannot load {name:?}"))?; 		Ok(Self(handle)) 	}
	fn function<F: Copy>(&self, name: &[u8]) -> Result<F> { 		let pointer = unsafe { dlsym(self.0, name.as_ptr().cast()) };
		require(!pointer.is_null(), format!("runtime symbol {:?} is absent", name))?;
		Ok(unsafe { std::mem::transmute_copy(&pointer) }) 	} }
fn driver_status(backend: Backend, status: i32, action: &str) -> Result<()> {
	(status == 0).then_some(()).ok_or_else(|| RecipeError::new(format!("{backend:?} {action} failed: {status}"))) }
impl Gpu { 	fn status(&self, status: i32, action: &str) -> Result<()> { 		driver_status(self.backend, status, action) 	}
	pub(super) fn threads(&self) -> Result<u32> { 		self.geometry.threads() 	}
	fn allocate(&self, bytes: usize) -> Result<u64> {
		unsafe { 			match &self.driver { 				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => {
					let mut pointer = 0;
					self.status((driver.allocate)(&mut pointer, bytes), "allocation")?; 					Ok(pointer) 				}
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => { 					let mut pointer = ptr::null_mut();
					self.status((driver.allocate)(driver.vram_pool, bytes, 0, &mut pointer), "allocation")?; 					self.status(
						(driver.allow)(1, &driver.cpu_agent, ptr::null(), pointer), 						"CPU allocation access", 					)?;
					Ok(pointer as u64) 				} 			} 		} 	} 	fn free(&self, pointer: u64) { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => { 					(driver.free)(pointer); 				}
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => { 					(driver.free)(pointer as Ptr); 				} 			} 		} 	}
	fn upload(&self, dst: u64, src: *const c_void, bytes: usize) -> Result<()> { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => self.status((driver.upload)(dst, src, bytes), "upload"),
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => self.status((driver.copy)(dst as Ptr, src, bytes), "upload"),
			} 		} 	} 	fn download(&self, dst: Ptr, src: u64, bytes: usize) -> Result<()> { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(cuda) => self.status((cuda.download)(dst, src, bytes), "download"),
				#[cfg(feature = "amd")]
				Driver::Hsa(driver) => self.status((driver.copy)(dst, src as *const c_void, bytes), "download"), 			} 		} 	}
	fn synchronize(&self) -> Result<()> { 		unsafe { 			match &self.driver { 				#[cfg(feature = "nvidia")]
				Driver::Cuda(driver) => self.status((driver.synchronize)(), "synchronization"), 				#[cfg(feature = "amd")]
				Driver::Hsa(driver) => require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD synchronization failed"),
			} 		} 	} 	pub(super) fn launch(&self, kernel: Kernel, arguments: &mut [Ptr]) -> Result<()> {
		require(!INTERRUPTED.load(Ordering::Acquire), "interrupted before GPU dispatch")?;
		let _guard = self.dispatch.lock().map_err(|_| RecipeError::new("GPU dispatch lock is poisoned"))?; 		unsafe {
			match &self.driver { 				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => { 					let stream = ptr::null_mut();
					self.status( 						(driver.launch)( 							kernel.object as usize,
							self.geometry.groups, 							1, 							1, 							self.geometry.block,
							1, 							1, 							0, 							stream, 							arguments.as_mut_ptr(), 						),
						"dispatch", 					) 				} 				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => {
					require(arguments.len() == kernel.layout.len(), "kernel argument count is invalid")?;
					ptr::write_bytes(driver.kernarg.cast::<u8>(), 0, driver.kernarg_size);
					let mut offset = 0; 					for (argument, kind) in arguments.iter().zip(kernel.layout) {
						let bytes = usize::from(*kind - b'0'); 						ptr::copy_nonoverlapping( 							(*argument).cast::<u8>(),
							driver.kernarg.cast::<u8>().add(offset), 							bytes, 						);
						offset += bytes; 					} 					require( 						offset <= kernel.kernarg && kernel.kernarg <= driver.kernarg_size,
						"kernarg layout is invalid", 					)?;
					(driver.store)(driver.signal, 1);
					let queue = &mut *(driver.queue as *mut HsaQueue);
					let index = (driver.write)(queue, 1); 					let packet =
						queue.base.cast::<HsaPacket>().add(index as usize & (queue.size as usize - 1)); 					packet.write(HsaPacket {
						header: 0, 						setup: 1, 						workgroup_x: self.geometry.block as u16,
						workgroup_y: 1, 						workgroup_z: 1,
						reserved0: 0, 						grid_x: self.threads()?, 						grid_y: 1, 						grid_z: 1, 						private: kernel.private,
						group: kernel.group, 						object: kernel.object, 						kernarg: driver.kernarg, 						reserved1: 0,
						completion: driver.signal, 					});
					std::sync::atomic::fence(Ordering::Release);
					let header = &*(&mut (*packet).header as *mut u16 as *mut std::sync::atomic::AtomicU16);
					header.store(2 | 2 << 9 | 2 << 11, Ordering::Release);
					(driver.store)(queue.doorbell, index as i64);
					require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD dispatch failed") 				} 			} 		} 	} }
static AMD: OnceLock<Result<Gpu>> = OnceLock::new();
static NVIDIA: OnceLock<Result<Gpu>> = OnceLock::new(); pub(super) fn device_backend() -> Result<Backend> {
	let mut failures = Vec::new(); 	for backend in [Backend::Amd, Backend::Nvidia] { 		match gpu(backend) {
			Ok(_) => return Ok(backend), 			Err(error) => failures.push(error.to_string()), 		} 	}
	Err(RecipeError::new(failures.join("; "))) } pub(super) fn gpu(backend: Backend) -> Result<&'static Gpu> {
	let loaded = match backend { 		Backend::Amd => AMD.get_or_init(load_amd),
		Backend::Nvidia => NVIDIA.get_or_init(load_nvidia), 	}; 	loaded.as_ref().map_err(Clone::clone) }
#[cfg(feature = "nvidia")] fn discrete(count: i32, mut probe: impl FnMut(i32) -> Result<Option<i32>>) -> Result<i32> {
	(0..count) 		.map(&mut probe) 		.find_map(|result| result.transpose()) 		.transpose()?
		.ok_or_else(|| RecipeError::new("Nvidia has no discrete GPU")) } #[cfg(feature = "amd")]
type HsaInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32; #[cfg(feature = "amd")] struct HsaQuery { 	info: HsaInfo,
	attribute: i32, 	expected: u32, 	secondary: i32, 	mask: u32, 	found: u64, } #[cfg(feature = "amd")]
extern "C" fn collect_hsa(handle: u64, pointer: Ptr) -> i32 { 	unsafe { 		let query = &mut *pointer.cast::<HsaQuery>();
		let mut value = 0;
		let mut status = (query.info)(handle, query.attribute, (&mut value as *mut u32).cast());
		if status != 0 || value != query.expected { 			return status; 		} 		if query.secondary >= 0 {
			status = (query.info)(handle, query.secondary, (&mut value as *mut u32).cast());
			if status != 0 || value & query.mask == 0 { 				return status; 			} 		} 		if query.found == 0 {
			query.found = handle; 		} 		0 	} } #[cfg(feature = "amd")] struct HsaGpuQuery { 	info: HsaInfo, 	found: u64, }
#[cfg(feature = "amd")] extern "C" fn collect_discrete_hsa(handle: u64, pointer: Ptr) -> i32 { 	unsafe {
		let query = &mut *pointer.cast::<HsaGpuQuery>();
		let mut device = 0_u32;
		let mut status = (query.info)(handle, 17, (&mut device as *mut u32).cast()); 		if status != 0 || device != 1 {
			return status; 		}
		let mut properties = 0_u64;
		status = (query.info)(handle, 0xA114, (&mut properties as *mut u64).cast()); 		if status != 0 || properties & 1 != 0 {
			return status; 		} 		if query.found == 0 {
			query.found = handle; 		} 		0 	} } #[cfg(feature = "amd")]
type HsaSymbol = unsafe extern "C" fn(u64, *const u8, *const u64, *mut u64) -> i32; #[cfg(feature = "amd")]
type HsaSymbolInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32; #[cfg(feature = "amd")] unsafe fn hsa_kernel(
	symbol: HsaSymbol, 	info: HsaSymbolInfo, 	executable: u64, 	agent: u64, 	name: &'static [u8], 	layout: &'static [u8],
) -> Result<Kernel> { 	let mut handle = 0;
	driver_status(Backend::Amd, unsafe { symbol(executable, name.as_ptr(), &agent, &mut handle) }, "kernel lookup")?;
	let mut kernel = Kernel { object: 0, kernarg: 0, group: 0, private: 0, layout }; 	for (attribute, output) in [
		(22, (&mut kernel.object as *mut u64).cast()), 		(11, (&mut kernel.kernarg as *mut usize).cast()),
		(13, (&mut kernel.group as *mut u32).cast()), 		(14, (&mut kernel.private as *mut u32).cast()), 	] {
		driver_status(Backend::Amd, unsafe { info(handle, attribute, output) }, "kernel metadata")?; 	} 	Ok(kernel) }
fn load_amd() -> Result<Gpu> { 	#[cfg(not(feature = "amd"))]
	return Err(RecipeError::new("AMD support is not compiled into this build")); 	#[cfg(feature = "amd")] 	unsafe {
		let runtime = Library::open(env!("RECIPE_HSA_RUNTIME"))?;
		let init: unsafe extern "C" fn() -> i32 = runtime.function(b"hsa_init\0")?;
		let iterate: unsafe extern "C" fn(extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_iterate_agents\0")?;
		let info: HsaInfo = runtime.function(b"hsa_agent_get_info\0")?;
		driver_status(Backend::Amd, init(), "initialization")?;
		let mut cpu = HsaQuery { info, attribute: 17, expected: 0, secondary: -1, mask: 0, found: 0 };
		let mut gpu = HsaGpuQuery { info, found: 0 };
		driver_status(Backend::Amd, iterate(collect_hsa, (&mut cpu as *mut HsaQuery).cast()), "CPU agent")?; 		driver_status(
			Backend::Amd, 			iterate(collect_discrete_hsa, (&mut gpu as *mut HsaGpuQuery).cast()), 			"GPU agent", 		)?;
		require(cpu.found != 0 && gpu.found != 0, "AMD CPU or discrete GPU agent is absent")?;
		let pool_info: HsaInfo = runtime.function(b"hsa_amd_memory_pool_get_info\0")?;
		let pool_iterate: unsafe extern "C" fn(u64, extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_amd_agent_iterate_memory_pools\0")?;
		let mut vram = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 4, found: 0 };
		let mut kernarg = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 1, found: 0 };
		driver_status( 			Backend::Amd, 			pool_iterate(gpu.found, collect_hsa, (&mut vram as *mut HsaQuery).cast()),
			"VRAM pools", 		)?; 		driver_status( 			Backend::Amd,
			pool_iterate(cpu.found, collect_hsa, (&mut kernarg as *mut HsaQuery).cast()), 			"KERNARG pools", 		)?;
		require(vram.found != 0 && kernarg.found != 0, "AMD VRAM or KERNARG pool is absent")?;
		let (mut wave, mut workgroup, mut available, mut node, mut waves, mut simds, mut cus) = (0, 0, 0, 0, 0, 0, 0);
		for (attribute, output, action) in [
			(6, (&mut wave as *mut u32).cast(), "wave query"),
			(8, (&mut workgroup as *mut u32).cast(), "workgroup query"),
			(0xA002, (&mut available as *mut u32).cast(), "CU query"),
			(0xA004, (&mut node as *mut u32).cast(), "KFD node query"),
			(0xA00A, (&mut waves as *mut u32).cast(), "wave occupancy query"),
			(0xA00B, (&mut simds as *mut u32).cast(), "SIMD query"),
			(0xA014, (&mut cus as *mut u32).cast(), "cooperative CU query"),
		] {
			driver_status(Backend::Amd, info(gpu.found, attribute, output), action)?;
		}
		require(cus <= available, "AMD cooperative CU count exceeds available CUs")?;
		let code = fs::File::open(env!("RECIPE_HSA_CODE_OBJECT"))
			.map_err(|error| RecipeError::new(format!("cannot open HSA code object: {error}")))?;
		let reader_create: unsafe extern "C" fn(i32, *mut u64) -> i32 =
			runtime.function(b"hsa_code_object_reader_create_from_file\0")?;
		let executable_create: unsafe extern "C" fn(i32, i32, Ptr, *mut u64) -> i32 =
			runtime.function(b"hsa_executable_create_alt\0")?;
		let executable_load: unsafe extern "C" fn(u64, u64, u64, Ptr, Ptr) -> i32 =
			runtime.function(b"hsa_executable_load_agent_code_object\0")?;
		let executable_freeze: unsafe extern "C" fn(u64, Ptr) -> i32 = 			runtime.function(b"hsa_executable_freeze\0")?;
		let symbol: HsaSymbol = runtime.function(b"hsa_executable_get_symbol_by_name\0")?;
		let symbol_info: HsaSymbolInfo = runtime.function(b"hsa_executable_symbol_get_info\0")?;
		let (mut reader, mut executable) = (0, 0);
		let descriptor = std::os::fd::AsRawFd::as_raw_fd(&code);
		driver_status(Backend::Amd, reader_create(descriptor, &mut reader), "code-object reader")?; 		driver_status(
			Backend::Amd, 			executable_create(1, 0, ptr::null_mut(), &mut executable), 			"executable creation", 		)?;
		driver_status( 			Backend::Amd, 			executable_load(executable, gpu.found, reader, ptr::null_mut(), ptr::null_mut()),
			"code-object load", 		)?;
		driver_status(Backend::Amd, executable_freeze(executable, ptr::null_mut()), "executable freeze")?;
		let forward = hsa_kernel(symbol, symbol_info, executable, gpu.found, b"forward_graph.kd\0", FORWARD_ARGS)?;
		let epoch = hsa_kernel(symbol, symbol_info, executable, gpu.found, b"tape_epoch_graph.kd\0", EPOCH_ARGS)?;
		let compiled = |name, text| -> Result<u32> { Ok(narrow(natural(name, text)?, name)? as u32) };
		let resources = Resources {
			registers: compiled("HSA forward VGPRs", env!("RECIPE_HSA_FORWARD_VGPRS"))?
				.max(compiled("HSA epoch VGPRs", env!("RECIPE_HSA_EPOCH_VGPRS"))?),
			shared: forward.group.max(epoch.group),
			max_block: compiled("HSA forward workgroup", env!("RECIPE_HSA_FORWARD_MAX_BLOCK"))?
				.min(compiled("HSA epoch workgroup", env!("RECIPE_HSA_EPOCH_MAX_BLOCK"))?),
		};
		let geometry = amd(cus, wave, workgroup, waves, simds, node, resources)?;
		let queue_create: unsafe extern "C" fn(u64, u32, u32, Ptr, Ptr, u32, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_queue_create\0")?;
		let signal_create: unsafe extern "C" fn(i64, u32, *const u64, *mut u64) -> i32 =
			runtime.function(b"hsa_signal_create\0")?; 		let allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_amd_memory_pool_allocate\0")?;
		let allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32 =
			runtime.function(b"hsa_amd_agents_allow_access\0")?;
		let (ka_size, mut ka) = (forward.kernarg.max(epoch.kernarg), ptr::null_mut());
		let (mut queue, mut completion) = (ptr::null_mut(), 0); 		driver_status( 			Backend::Amd,
			queue_create(gpu.found, 256, 2, ptr::null_mut(), ptr::null_mut(), u32::MAX, u32::MAX, &mut queue),
			"queue creation", 		)?;
		driver_status(Backend::Amd, signal_create(1, 0, ptr::null(), &mut completion), "signal creation")?;
		driver_status(Backend::Amd, allocate(kernarg.found, ka_size, 0, &mut ka), "KERNARG allocation")?;
		driver_status(Backend::Amd, allow(1, &gpu.found, ptr::null(), ka), "GPU KERNARG access")?;
		eprintln!("AMD grid {} block {}", geometry.groups, geometry.block); 		Ok(Gpu {
			backend: Backend::Amd, 			driver: Driver::Hsa(Hsa {
				allocate, 				free: runtime.function(b"hsa_amd_memory_pool_free\0")?, 				allow,
				copy: runtime.function(b"hsa_memory_copy\0")?, 				store: runtime.function(b"hsa_signal_store_screlease\0")?,
				wait: runtime.function(b"hsa_signal_wait_scacquire\0")?,
				write: runtime.function(b"hsa_queue_add_write_index_scacq_screl\0")?, 				queue, 				signal: completion,
				cpu_agent: cpu.found, 				vram_pool: vram.found, 				kernarg_pool: kernarg.found, 				kernarg_size: ka_size,
				kernarg: ka, 				_code: code, 			}), 			geometry, 			forward, 			epoch,
			dispatch: Mutex::new(()), 		}) 	} }
fn load_nvidia() -> Result<Gpu> { 	#[cfg(not(feature = "nvidia"))]
	return Err(RecipeError::new("NVIDIA support is not compiled into this build")); 	#[cfg(feature = "nvidia")] 	unsafe {
		const MAX_BLOCK: i32 = 1;
		const BLOCK_LDS: i32 = 8;
		const WAVE: i32 = 10;
		const CUS: i32 = 16;
		const INTEGRATED: i32 = 18;
		const THREADS_PER_SM: i32 = 39;
		const SM_LDS: i32 = 81;
		const REGISTERS_PER_SM: i32 = 82;
		const COOPERATIVE: i32 = 95;
		let runtime = Library::open(env!("RECIPE_NV_RUNTIME"))?;
		let init: unsafe extern "C" fn(u32) -> i32 = runtime.function(b"cuInit\0")?;
		let count_devices: Count = runtime.function(b"cuDeviceGetCount\0")?;
		let get_device: Device = runtime.function(b"cuDeviceGet\0")?;
		let attribute: Attribute = runtime.function(b"cuDeviceGetAttribute\0")?;
		let create: Context = runtime.function(b"cuCtxCreate_v2\0")?;
		let load: Module = runtime.function(b"cuModuleLoad\0")?;
		let function: Function = runtime.function(b"cuModuleGetFunction\0")?;
		let function_attribute: FunctionAttribute = runtime.function(b"cuFuncGetAttribute\0")?;
		let occupancy: Occupancy = runtime.function(b"cuOccupancyMaxActiveBlocksPerMultiprocessor\0")?;
		let (mut count, mut forward, mut epoch) = (0, 0, 0);
		let (mut context, mut module) = (ptr::null_mut(), ptr::null_mut());
		driver_status(Backend::Nvidia, init(0), "initialization")?;
		driver_status(Backend::Nvidia, count_devices(&mut count), "device enumeration")?;
		let device = discrete(count, |ordinal| { 			let (mut device, mut integrated) = (0, 0);
			driver_status(Backend::Nvidia, get_device(&mut device, ordinal), "device enumeration")?;
			driver_status(Backend::Nvidia, attribute(&mut integrated, INTEGRATED, device), "device probe")?;
			Ok((integrated == 0).then_some(device)) 		})?;
		let (mut cus, mut wave, mut workgroup, mut block_lds, mut sm_lds, mut registers, mut threads, mut cooperative) =
			(0, 0, 0, 0, 0, 0, 0, 0);
		for (kind, output, action) in [
			(CUS, &mut cus, "SM query"), 			(WAVE, &mut wave, "warp query"),
			(MAX_BLOCK, &mut workgroup, "workgroup query"), 			(BLOCK_LDS, &mut block_lds, "workgroup LDS query"),
			(SM_LDS, &mut sm_lds, "SM LDS query"), 			(REGISTERS_PER_SM, &mut registers, "register query"),
			(THREADS_PER_SM, &mut threads, "resident thread query"),
			(COOPERATIVE, &mut cooperative, "cooperative launch query"),
		] {
			driver_status(Backend::Nvidia, attribute(output, kind, device), action)?;
		}
		require(cooperative != 0, "Nvidia device does not support cooperative launch")?;
		driver_status(Backend::Nvidia, create(&mut context, 0, device), "context creation")?; 		driver_status(
			Backend::Nvidia, 			load(&mut module, concat!(env!("RECIPE_NV_MODULE"), "\0").as_ptr()), 			"module load", 		)?;
		driver_status( 			Backend::Nvidia, 			function(&mut forward, module, b"forward_graph\0".as_ptr()), 			"forward load",
		)?;
		driver_status(Backend::Nvidia, function(&mut epoch, module, b"tape_epoch_graph\0".as_ptr()), "epoch load")?;
		let resource = |kernel| -> Result<Resources> {
			let (mut max_block, mut shared, mut used_registers) = (0, 0, 0);
			for (kind, output, action) in [
				(0, &mut max_block, "kernel workgroup query"), 				(1, &mut shared, "kernel LDS query"),
				(4, &mut used_registers, "kernel register query"),
			] {
				driver_status(Backend::Nvidia, function_attribute(output, kind, kernel), action)?;
			}
			require(max_block > 0 && shared >= 0 && used_registers > 0, "Nvidia kernel resources are invalid")?;
			Ok(Resources { registers: used_registers as u32, shared: shared as u32, max_block: max_block as u32 })
		};
		let forward_resource = resource(forward)?;
		let epoch_resource = resource(epoch)?;
		let resources = Resources {
			registers: forward_resource.registers.max(epoch_resource.registers),
			shared: forward_resource.shared.max(epoch_resource.shared),
			max_block: forward_resource.max_block.min(epoch_resource.max_block),
		};
		let register_wave = resources.registers.checked_mul(wave as u32)
			.ok_or_else(|| RecipeError::new("Nvidia wave register count overflows"))?;
		let observed = (registers as u32 / register_wave).min(threads as u32 / wave as u32);
		let geometry = nvidia(
			cus as u32, 			wave as u32, 			workgroup as u32, 			block_lds as u32, 			sm_lds as u32,
			observed, 			resources, 		)?;
		for (kernel, action) in [(forward, "forward occupancy"), (epoch, "epoch occupancy")] {
			let mut active = 0;
			driver_status(Backend::Nvidia, occupancy(&mut active, kernel, geometry.block as i32, 0), action)?;
			require(active > 0, format!("Nvidia {action} has no resident workgroup"))?;
		}
		let cuda = Cuda { 			allocate: runtime.function(b"cuMemAlloc_v2\0")?,
			free: runtime.function(b"cuMemFree_v2\0")?, 			upload: runtime.function(b"cuMemcpyHtoD_v2\0")?,
			download: runtime.function(b"cuMemcpyDtoH_v2\0")?, 			synchronize: runtime.function(b"cuCtxSynchronize\0")?,
			launch: runtime.function(b"cuLaunchCooperativeKernel\0")?, 		};
		eprintln!("Nvidia grid {} block {}", geometry.groups, geometry.block); 		Ok(Gpu {
			backend: Backend::Nvidia, 			driver: Driver::Cuda(cuda), 			geometry,
			forward: Kernel::cuda(forward, FORWARD_ARGS), 			epoch: Kernel::cuda(epoch, EPOCH_ARGS),
			dispatch: Mutex::new(()), 		}) 	} } #[cfg(any(feature = "amd", feature = "nvidia"))] #[link(name = "dl")]
unsafe extern "C" { 	fn dlopen(name: *const std::ffi::c_char, flags: i32) -> Ptr;
	fn dlsym(handle: Ptr, name: *const std::ffi::c_char) -> Ptr; }
