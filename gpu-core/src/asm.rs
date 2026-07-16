//! Runtime dispatch of hand-written gfx assembly.
//!
//! Loads a `.s` (or pre-assembled `.hsaco`) AMDGPU code object, freezes it into
//! an HSA executable, and dispatches its kernels by hand-writing the AQL packet
//! and ringing the queue doorbell. This is the raw-HSA path: no HIP module API,
//! no vendor launcher. A `.s` is assembled to a `.hsaco` once (cached beside the
//! source by mtime) via the ROCm LLVM `clang`, then loaded through the runtime.
//!
//! ```no_run
//! let prog = gpu_core::asm::load("/path/diffusion_step.gfx1101.entropy_fast.s")?;
//! let mut args = gpu_core::asm::KernArgs::new();
//! args.ptr(d_logits).ptr(d_entropy).i32(n).i32(classes);
//! prog.launch("diffusionx_entropy_gated_step_kernel", [n as u32, 1, 1], [256, 1, 1], &args)?;
//! # Ok::<(), gpu_core::asm::AsmError>(())
//! ```

use crate::gate;
use core::error::Error;
use core::ffi::c_void;
use core::fmt;
use core::ptr;
use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// A failure anywhere in the assemble/load/dispatch path.
#[derive(Debug)]
pub struct AsmError(pub String);

impl fmt::Display for AsmError {
      #[inline]
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let Self(msg) = self;
            return write!(f, "{msg}");
      }
}
impl Error for AsmError {}

type Result<T> = core::result::Result<T, AsmError>;

fn err<T>(msg: impl Into<String>) -> Result<T> {
      return Err(AsmError(msg.into()));
}

// ---- HSA FFI ----------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct Agent {
      handle: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Signal {
      handle: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Pool {
      handle: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CodeObjectReader {
      handle: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Executable {
      handle: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Symbol {
      handle: u64,
}

/// AMD queue header, 64-bit (`HSA_LARGE_MODEL`) layout. Only the fields the
/// doorbell path reads are named; the runtime owns the rest.
#[repr(C)]
struct Queue {
      queue_type: u32,
      features: u32,
      base_address: *mut c_void,
      doorbell_signal: Signal,
      size: u32,
      reserved1: u32,
      id: u64,
}

/// The 64-byte kernel-dispatch AQL packet (`HSA_LARGE_MODEL`).
#[repr(C)]
struct DispatchPacket {
      header: u16,
      setup: u16,
      workgroup_size_x: u16,
      workgroup_size_y: u16,
      workgroup_size_z: u16,
      reserved0: u16,
      grid_size_x: u32,
      grid_size_y: u32,
      grid_size_z: u32,
      private_segment_size: u32,
      group_segment_size: u32,
      kernel_object: u64,
      kernarg_address: *mut c_void,
      reserved2: u64,
      completion_signal: Signal,
}

const HSA_STATUS_SUCCESS: i32 = 0;
const HSA_STATUS_INFO_BREAK: i32 = 1;
const HSA_AGENT_INFO_DEVICE: i32 = 17;
const HSA_DEVICE_TYPE_CPU: u32 = 0;
const HSA_DEVICE_TYPE_GPU: u32 = 1;
const HSA_QUEUE_TYPE_SINGLE: u32 = 1;
const HSA_PROFILE_FULL: i32 = 1;
const HSA_ROUNDING_MODE_DEFAULT: i32 = 0;
const HSA_SYMBOL_KERNEL_OBJECT: i32 = 22;
const HSA_SYMBOL_KERNARG_SEGMENT_SIZE: i32 = 11;
const HSA_SYMBOL_GROUP_SEGMENT_SIZE: i32 = 13;
const HSA_SYMBOL_PRIVATE_SEGMENT_SIZE: i32 = 14;
const HSA_POOL_INFO_SEGMENT: i32 = 0;
const HSA_POOL_INFO_GLOBAL_FLAGS: i32 = 1;
const HSA_POOL_INFO_RUNTIME_ALLOC_ALLOWED: i32 = 5;
const HSA_SEGMENT_GLOBAL: u32 = 0;
const HSA_POOL_FLAG_KERNARG: u32 = 1;
const HSA_POOL_FLAG_COARSE: u32 = 4;
const HSA_PACKET_TYPE_KERNEL_DISPATCH: u16 = 2;
const HSA_FENCE_SCOPE_SYSTEM: u16 = 2;
const HSA_SIGNAL_CONDITION_EQ: i32 = 0;
const HSA_WAIT_STATE_BLOCKED: i32 = 0;

unsafe extern "C" {
      fn hsa_init() -> i32;
      fn hsa_iterate_agents(
            cb: extern "C" fn(Agent, *mut c_void) -> i32,
            data: *mut c_void,
      ) -> i32;
      fn hsa_agent_get_info(agent: Agent, attribute: i32, value: *mut c_void) -> i32;
      fn hsa_queue_create(
            agent: Agent,
            size: u32,
            queue_type: u32,
            callback: *const c_void,
            data: *const c_void,
            private_segment_size: u32,
            group_segment_size: u32,
            queue: *mut *mut Queue,
      ) -> i32;
      fn hsa_queue_load_write_index_relaxed(queue: *const Queue) -> u64;
      fn hsa_queue_store_write_index_relaxed(queue: *const Queue, value: u64);
      fn hsa_signal_create(
            initial_value: i64,
            num_consumers: u32,
            consumers: *const Agent,
            signal: *mut Signal,
      ) -> i32;
      fn hsa_signal_destroy(signal: Signal) -> i32;
      fn hsa_signal_store_screlease(signal: Signal, value: i64);
      fn hsa_signal_wait_scacquire(
            signal: Signal,
            condition: i32,
            compare_value: i64,
            timeout_hint: u64,
            wait_state_hint: i32,
      ) -> i64;
      fn hsa_code_object_reader_create_from_file(
            file: i32,
            reader: *mut CodeObjectReader,
      ) -> i32;
      fn hsa_executable_create_alt(
            profile: i32,
            default_float_rounding_mode: i32,
            options: *const i8,
            executable: *mut Executable,
      ) -> i32;
      fn hsa_executable_load_agent_code_object(
            executable: Executable,
            agent: Agent,
            reader: CodeObjectReader,
            options: *const i8,
            loaded: *mut c_void,
      ) -> i32;
      fn hsa_executable_freeze(executable: Executable, options: *const i8) -> i32;
      fn hsa_executable_get_symbol_by_name(
            executable: Executable,
            symbol_name: *const i8,
            agent: *const Agent,
            symbol: *mut Symbol,
      ) -> i32;
      fn hsa_executable_symbol_get_info(symbol: Symbol, attribute: i32, value: *mut c_void)
      -> i32;
      fn hsa_amd_agent_iterate_memory_pools(
            agent: Agent,
            cb: extern "C" fn(Pool, *mut c_void) -> i32,
            data: *mut c_void,
      ) -> i32;
      fn hsa_amd_memory_pool_get_info(pool: Pool, attribute: i32, value: *mut c_void) -> i32;
      fn hsa_amd_memory_pool_allocate(
            pool: Pool,
            size: usize,
            flags: u32,
            ptr: *mut *mut c_void,
      ) -> i32;
      fn hsa_amd_memory_pool_free(ptr: *mut c_void) -> i32;
      fn hsa_amd_agents_allow_access(
            num_agents: u32,
            agents: *const Agent,
            flags: *const u32,
            ptr: *const c_void,
      ) -> i32;
}

fn hsa_check(status: i32, what: &str) -> Result<()> {
      if status == HSA_STATUS_SUCCESS {
            return Ok(());
      }
      return err(format!("{what}: HSA status {status}"));
}

// ---- process-global HSA context ---------------------------------------------

struct Ctx {
      gpu: Agent,
      queue: *mut Queue,
      kernarg_pool: Pool,
      vram_pool: Pool,
}

// SAFETY: the queue pointer and pool/agent handles are stable for process life;
// every dispatch that touches them holds `gate::acquire`, serializing GPU entry.
unsafe impl Send for Ctx {}
// SAFETY: see the Send note; access is funneled through the process GPU gate.
unsafe impl Sync for Ctx {}

static CTX: OnceLock<Ctx> = OnceLock::new();

struct AgentSearch {
      want: u32,
      found: Option<Agent>,
}

extern "C" fn pick_agent(agent: Agent, data: *mut c_void) -> i32 {
      // SAFETY: data is the &mut AgentSearch handed to hsa_iterate_agents below.
      let search = unsafe { &mut *data.cast::<AgentSearch>() };
      let mut device_type: u32 = 0;
      // SAFETY: HSA_AGENT_INFO_DEVICE writes a u32 into device_type.
      let st = unsafe {
            hsa_agent_get_info(
                  agent,
                  HSA_AGENT_INFO_DEVICE,
                  (&raw mut device_type).cast::<c_void>(),
            )
      };
      if st == HSA_STATUS_SUCCESS && device_type == search.want {
            search.found = Some(agent);
            return HSA_STATUS_INFO_BREAK;
      }
      return HSA_STATUS_SUCCESS;
}

struct PoolSearch {
      want_flags: u32,
      found: Option<Pool>,
}

extern "C" fn pick_pool(pool: Pool, data: *mut c_void) -> i32 {
      // SAFETY: data is the &mut PoolSearch handed to the iterate call below.
      let search = unsafe { &mut *data.cast::<PoolSearch>() };
      let mut segment: u32 = 0;
      let mut flags: u32 = 0;
      let mut alloc_ok: u8 = 0;
      // SAFETY: each query writes its documented scalar into the local it targets.
      unsafe {
            if hsa_amd_memory_pool_get_info(
                  pool,
                  HSA_POOL_INFO_SEGMENT,
                  (&raw mut segment).cast::<c_void>(),
            ) != HSA_STATUS_SUCCESS
            {
                  return HSA_STATUS_SUCCESS;
            }
            hsa_amd_memory_pool_get_info(
                  pool,
                  HSA_POOL_INFO_GLOBAL_FLAGS,
                  (&raw mut flags).cast::<c_void>(),
            );
            hsa_amd_memory_pool_get_info(
                  pool,
                  HSA_POOL_INFO_RUNTIME_ALLOC_ALLOWED,
                  (&raw mut alloc_ok).cast::<c_void>(),
            );
      }
      if segment == HSA_SEGMENT_GLOBAL
            && alloc_ok != 0
            && (flags & search.want_flags) == search.want_flags
      {
            search.found = Some(pool);
            return HSA_STATUS_INFO_BREAK;
      }
      return HSA_STATUS_SUCCESS;
}

fn find_agent(want: u32) -> Result<Agent> {
      let mut search = AgentSearch { want, found: None };
      // SAFETY: pick_agent only writes through the &mut AgentSearch pointer passed here.
      unsafe {
            hsa_iterate_agents(pick_agent, (&raw mut search).cast::<c_void>());
      }
      return search.found.ok_or_else(|| AsmError(format!("no HSA agent of device type {want}")));
}

fn find_pool(agent: Agent, want_flags: u32) -> Result<Pool> {
      let mut search = PoolSearch { want_flags, found: None };
      // SAFETY: pick_pool only writes through the &mut PoolSearch pointer passed here.
      unsafe {
            hsa_amd_agent_iterate_memory_pools(agent, pick_pool, (&raw mut search).cast::<c_void>());
      }
      return search
            .found
            .ok_or_else(|| AsmError(format!("no global pool with flags {want_flags:#x}")));
}

fn context() -> Result<&'static Ctx> {
      if let Some(ctx) = CTX.get() {
            return Ok(ctx);
      }
      static INIT_LOCK: Mutex<()> = Mutex::new(());
      let _guard = INIT_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
      if let Some(ctx) = CTX.get() {
            return Ok(ctx);
      }
      gate::acquire();
      // SAFETY: hsa_init is reference-counted and safe to call after HIP has already initialized the runtime.
      hsa_check(unsafe { hsa_init() }, "hsa_init")?;
      let gpu = find_agent(HSA_DEVICE_TYPE_GPU)?;
      let cpu = find_agent(HSA_DEVICE_TYPE_CPU)?;
      let kernarg_pool = find_pool(cpu, HSA_POOL_FLAG_KERNARG)?;
      let vram_pool = find_pool(gpu, HSA_POOL_FLAG_COARSE)?;
      let mut queue: *mut Queue = ptr::null_mut();
      // SAFETY: a single-consumer dispatch queue of 1024 slots on the GPU agent; queue receives the runtime-owned pointer.
      hsa_check(
            unsafe {
                  hsa_queue_create(
                        gpu,
                        1024,
                        HSA_QUEUE_TYPE_SINGLE,
                        ptr::null(),
                        ptr::null(),
                        u32::MAX,
                        u32::MAX,
                        &raw mut queue,
                  )
            },
            "hsa_queue_create",
      )?;
      if queue.is_null() {
            return err("hsa_queue_create returned a null queue");
      }
      let ctx = Ctx { gpu, queue, kernarg_pool, vram_pool };
      // Only this thread holds INIT_LOCK and the double-check above saw None, so
      // set cannot fail and no queue leaks.
      if CTX.set(ctx).is_err() {
            return err("asm context init raced despite lock");
      }
      return CTX.get().ok_or_else(|| AsmError("asm context vanished after set".to_owned()));
}

/// Allocates `bytes` of coarse-grained device VRAM for use as a kernel argument
/// buffer, granting the GPU agent access. Free it with [`free_device`].
///
/// This is the raw-HSA analogue of the arena; tensors that already live in the
/// ledgered arena should pass their own device pointers to [`KernArgs::ptr`]
/// instead of allocating here.
///
/// # Errors
/// Returns [`AsmError`] if the HSA pool allocation or access grant fails.
pub fn alloc_device(bytes: usize) -> Result<*mut c_void> {
      let ctx = context()?;
      gate::acquire();
      let mut ptr_out: *mut c_void = ptr::null_mut();
      // SAFETY: allocates `bytes` from the device coarse pool; ptr_out receives the pointer.
      hsa_check(
            unsafe {
                  hsa_amd_memory_pool_allocate(ctx.vram_pool, bytes.max(1), 0, &raw mut ptr_out)
            },
            "hsa_amd_memory_pool_allocate(vram)",
      )?;
      if ptr_out.is_null() {
            return err("device allocation returned null");
      }
      // SAFETY: grants the GPU agent access to the buffer it will read and write.
      let allow =
            unsafe { hsa_amd_agents_allow_access(1, &raw const ctx.gpu, ptr::null(), ptr_out) };
      if allow != HSA_STATUS_SUCCESS {
            // SAFETY: ptr_out is a live allocation; free it before surfacing the error.
            unsafe {
                  hsa_amd_memory_pool_free(ptr_out);
            }
            return err(format!("hsa_amd_agents_allow_access(vram): HSA status {allow}"));
      }
      return Ok(ptr_out);
}

/// Frees a buffer returned by [`alloc_device`].
///
/// # Errors
/// Returns [`AsmError`] if the HSA free fails.
///
/// # Safety
/// `ptr` must be a pointer previously returned by [`alloc_device`] and not yet
/// freed.
pub unsafe fn free_device(ptr: *mut c_void) -> Result<()> {
      gate::acquire();
      // SAFETY: caller guarantees ptr came from alloc_device and is freed exactly once.
      return hsa_check(unsafe { hsa_amd_memory_pool_free(ptr) }, "hsa_amd_memory_pool_free(vram)");
}

// ---- kernel arguments -------------------------------------------------------

/// A packed kernarg buffer, built left to right in declaration order. Each value
/// is placed at its natural alignment, matching the AMDGPU kernarg ABI.
#[derive(Default)]
pub struct KernArgs {
      buf: Vec<u8>,
}

impl KernArgs {
      #[inline]
      #[must_use]
      pub fn new() -> Self {
            return Self { buf: Vec::new() };
      }

      fn align_to(&mut self, align: usize) {
            while self.buf.len() % align != 0 {
                  self.buf.push(0);
            }
      }

      /// Appends a device pointer argument (8 bytes, 8-aligned).
      #[inline]
      pub fn ptr(&mut self, p: *const c_void) -> &mut Self {
            return self.u64(p as u64);
      }

      /// Appends a `u64` argument (8 bytes, 8-aligned).
      #[inline]
      pub fn u64(&mut self, v: u64) -> &mut Self {
            self.align_to(8);
            self.buf.extend_from_slice(&v.to_ne_bytes());
            return self;
      }

      /// Appends an `i32` argument (4 bytes, 4-aligned).
      #[inline]
      pub fn i32(&mut self, v: i32) -> &mut Self {
            self.align_to(4);
            self.buf.extend_from_slice(&v.to_ne_bytes());
            return self;
      }

      /// Appends a `u32` argument (4 bytes, 4-aligned).
      #[inline]
      pub fn u32(&mut self, v: u32) -> &mut Self {
            self.align_to(4);
            self.buf.extend_from_slice(&v.to_ne_bytes());
            return self;
      }

      /// Appends an `f64` argument (8 bytes, 8-aligned).
      #[inline]
      pub fn f64(&mut self, v: f64) -> &mut Self {
            self.align_to(8);
            self.buf.extend_from_slice(&v.to_ne_bytes());
            return self;
      }

      #[inline]
      #[must_use]
      pub fn as_bytes(&self) -> &[u8] {
            return &self.buf;
      }
}

// ---- assemble + load --------------------------------------------------------

struct Kernel {
      kernel_object: u64,
      kernarg_size: u32,
      group_size: u32,
      private_size: u32,
}

/// A frozen HSA executable and its dispatchable kernels, keyed by name.
pub struct Program {
      kernels: HashMap<String, Kernel>,
      source: PathBuf,
}

/// Assembles `path` if it is a `.s` file, loads the code object, and freezes it.
///
/// A `.s` source is compiled to `<stem>.hsaco` beside it, reusing the cached
/// object when it is newer than the source. A `.hsaco`/`.co` path is loaded
/// directly.
///
/// # Errors
/// Returns [`AsmError`] if assembly, HSA load, or symbol resolution fails.
pub fn load(path: impl AsRef<Path>) -> Result<Program> {
      let path = path.as_ref();
      let object = assemble(path)?;
      let ctx = context()?;
      gate::acquire();
      let fd = open_ro(&object)?;
      let mut reader = CodeObjectReader { handle: 0 };
      // SAFETY: fd is a live read-only descriptor for the code object; reader receives the handle.
      let st = unsafe { hsa_code_object_reader_create_from_file(fd, &raw mut reader) };
      // SAFETY: fd is owned here and closed exactly once now that the reader has consumed it.
      unsafe {
            libc::close(fd);
      }
      hsa_check(st, "hsa_code_object_reader_create_from_file")?;

      let mut exe = Executable { handle: 0 };
      // SAFETY: full profile, default rounding; exe receives the new executable handle.
      hsa_check(
            unsafe {
                  hsa_executable_create_alt(
                        HSA_PROFILE_FULL,
                        HSA_ROUNDING_MODE_DEFAULT,
                        ptr::null(),
                        &raw mut exe,
                  )
            },
            "hsa_executable_create_alt",
      )?;
      // SAFETY: exe, ctx.gpu, and reader are live handles from the calls above.
      hsa_check(
            unsafe {
                  hsa_executable_load_agent_code_object(
                        exe,
                        ctx.gpu,
                        reader,
                        ptr::null(),
                        ptr::null_mut(),
                  )
            },
            "hsa_executable_load_agent_code_object",
      )?;
      // SAFETY: exe is a live, loaded executable ready to freeze.
      hsa_check(unsafe { hsa_executable_freeze(exe, ptr::null()) }, "hsa_executable_freeze")?;

      let mut kernels = HashMap::new();
      for name in kernel_names(&object)? {
            let kernel = resolve(exe, ctx.gpu, &name)?;
            kernels.insert(name, kernel);
      }
      if kernels.is_empty() {
            return err(format!("no kernels found in {}", object.display()));
      }
      return Ok(Program { kernels, source: path.to_path_buf() });
}

fn resolve(exe: Executable, gpu: Agent, name: &str) -> Result<Kernel> {
      let symbol_name = CString::new(format!("{name}.kd"))
            .map_err(|_e| AsmError(format!("kernel name has interior NUL: {name}")))?;
      let mut symbol = Symbol { handle: 0 };
      // SAFETY: symbol_name is a valid C string; &gpu is a live agent; symbol receives the handle.
      hsa_check(
            unsafe {
                  hsa_executable_get_symbol_by_name(
                        exe,
                        symbol_name.as_ptr(),
                        &raw const gpu,
                        &raw mut symbol,
                  )
            },
            &format!("hsa_executable_get_symbol_by_name({name}.kd)"),
      )?;
      let mut kernel_object: u64 = 0;
      let mut kernarg_size: u32 = 0;
      let mut group_size: u32 = 0;
      let mut private_size: u32 = 0;
      // SAFETY: each symbol query writes its documented scalar type into the matching local.
      unsafe {
            hsa_check(
                  hsa_executable_symbol_get_info(
                        symbol,
                        HSA_SYMBOL_KERNEL_OBJECT,
                        (&raw mut kernel_object).cast::<c_void>(),
                  ),
                  "symbol KERNEL_OBJECT",
            )?;
            hsa_check(
                  hsa_executable_symbol_get_info(
                        symbol,
                        HSA_SYMBOL_KERNARG_SEGMENT_SIZE,
                        (&raw mut kernarg_size).cast::<c_void>(),
                  ),
                  "symbol KERNARG_SEGMENT_SIZE",
            )?;
            hsa_check(
                  hsa_executable_symbol_get_info(
                        symbol,
                        HSA_SYMBOL_GROUP_SEGMENT_SIZE,
                        (&raw mut group_size).cast::<c_void>(),
                  ),
                  "symbol GROUP_SEGMENT_SIZE",
            )?;
            hsa_check(
                  hsa_executable_symbol_get_info(
                        symbol,
                        HSA_SYMBOL_PRIVATE_SEGMENT_SIZE,
                        (&raw mut private_size).cast::<c_void>(),
                  ),
                  "symbol PRIVATE_SEGMENT_SIZE",
            )?;
      }
      return Ok(Kernel { kernel_object, kernarg_size, group_size, private_size });
}

impl Program {
      /// The kernel names this program exposes.
      #[must_use]
      pub fn kernels(&self) -> Vec<&str> {
            return self.kernels.keys().map(String::as_str).collect();
      }

      /// The source path this program was loaded from.
      #[must_use]
      pub fn source(&self) -> &Path {
            return &self.source;
      }

      /// Dispatches `name` over `grid` work-items in `block`-sized work-groups,
      /// with `args` copied into the kernarg segment. Blocks until the kernel
      /// signals completion.
      ///
      /// `grid` and `block` are `[x, y, z]`; the packet dimension count is the
      /// number of dimensions whose grid extent is greater than 1.
      ///
      /// # Errors
      /// Returns [`AsmError`] if the kernel is unknown, `args` overflows the
      /// kernarg segment, or an HSA allocation/dispatch step fails.
      pub fn launch(
            &self,
            name: &str,
            grid: [u32; 3],
            block: [u16; 3],
            args: &KernArgs,
      ) -> Result<()> {
            let kernel = self
                  .kernels
                  .get(name)
                  .ok_or_else(|| AsmError(format!("kernel {name} not in {}", self.source.display())))?;
            let bytes = args.as_bytes();
            if bytes.len() > kernel.kernarg_size as usize {
                  return err(format!(
                        "kernarg overflow for {name}: {} bytes given, segment is {}",
                        bytes.len(),
                        kernel.kernarg_size
                  ));
            }
            let ctx = context()?;
            gate::acquire();

            let kernarg = alloc_kernarg(ctx, kernel.kernarg_size, bytes)?;
            let result = self.dispatch(ctx, kernel, grid, block, kernarg);
            // SAFETY: kernarg came from hsa_amd_memory_pool_allocate above; freed once, after the wait completes.
            unsafe {
                  hsa_amd_memory_pool_free(kernarg);
            }
            return result;
      }

      fn dispatch(
            &self,
            ctx: &Ctx,
            kernel: &Kernel,
            grid: [u32; 3],
            block: [u16; 3],
            kernarg: *mut c_void,
      ) -> Result<()> {
            let mut signal = Signal { handle: 0 };
            // SAFETY: creates a completion signal initialized to 1; signal receives the handle.
            hsa_check(
                  unsafe { hsa_signal_create(1, 0, ptr::null(), &raw mut signal) },
                  "hsa_signal_create",
            )?;

            let dims: u16 = if grid[2] > 1 {
                  3
            } else if grid[1] > 1 {
                  2
            } else {
                  1
            };
            let header = HSA_PACKET_TYPE_KERNEL_DISPATCH
                  | (HSA_FENCE_SCOPE_SYSTEM << 9)
                  | (HSA_FENCE_SCOPE_SYSTEM << 11);

            // SAFETY: ctx.queue is a live single-consumer queue; base_address holds queue.size packet slots.
            let (base, qsize, doorbell) = unsafe {
                  ((*ctx.queue).base_address, (*ctx.queue).size, (*ctx.queue).doorbell_signal)
            };
            // SAFETY: reads the current write index of the live queue.
            let index = unsafe { hsa_queue_load_write_index_relaxed(ctx.queue) };
            let slot = index % u64::from(qsize);
            let packet = base.cast::<DispatchPacket>().wrapping_add(slot as usize);

            // SAFETY: packet points at slot `slot` of the queue's packet ring, sized for DispatchPacket.
            unsafe {
                  ptr::write(
                        packet,
                        DispatchPacket {
                              header: 0,
                              setup: dims,
                              workgroup_size_x: block[0],
                              workgroup_size_y: block[1],
                              workgroup_size_z: block[2],
                              reserved0: 0,
                              grid_size_x: grid[0],
                              grid_size_y: grid[1],
                              grid_size_z: grid[2],
                              private_segment_size: kernel.private_size,
                              group_segment_size: kernel.group_size,
                              kernel_object: kernel.kernel_object,
                              kernarg_address: kernarg,
                              reserved2: 0,
                              completion_signal: signal,
                        },
                  );
            }
            // SAFETY: publishes the packet header with release ordering so the packet
            // processor observes a fully written packet before the type flips to dispatch.
            unsafe {
                  let header_ptr = &raw mut (*packet).header;
                  core::sync::atomic::AtomicU16::from_ptr(header_ptr)
                        .store(header, core::sync::atomic::Ordering::Release);
            }
            // SAFETY: advances the queue write index and rings the doorbell for the packet just published.
            unsafe {
                  hsa_queue_store_write_index_relaxed(ctx.queue, index + 1);
                  hsa_signal_store_screlease(doorbell, index as i64);
            }

            // SAFETY: blocks the host until the completion signal reaches 0.
            unsafe {
                  hsa_signal_wait_scacquire(
                        signal,
                        HSA_SIGNAL_CONDITION_EQ,
                        0,
                        u64::MAX,
                        HSA_WAIT_STATE_BLOCKED,
                  );
                  hsa_signal_destroy(signal);
            }
            return Ok(());
      }
}

fn alloc_kernarg(ctx: &Ctx, size: u32, bytes: &[u8]) -> Result<*mut c_void> {
      let size = size.max(1) as usize;
      let mut kernarg: *mut c_void = ptr::null_mut();
      // SAFETY: allocates `size` bytes from the kernarg pool; kernarg receives the pointer.
      hsa_check(
            unsafe {
                  hsa_amd_memory_pool_allocate(ctx.kernarg_pool, size, 0, &raw mut kernarg)
            },
            "hsa_amd_memory_pool_allocate(kernarg)",
      )?;
      if kernarg.is_null() {
            return err("kernarg allocation returned null");
      }
      // SAFETY: grants the GPU agent access to the host-side kernarg buffer.
      let allow = unsafe {
            hsa_amd_agents_allow_access(1, &raw const ctx.gpu, ptr::null(), kernarg)
      };
      if allow != HSA_STATUS_SUCCESS {
            // SAFETY: kernarg is a live allocation from the pool; free it before returning the error.
            unsafe {
                  hsa_amd_memory_pool_free(kernarg);
            }
            return err(format!("hsa_amd_agents_allow_access(kernarg): HSA status {allow}"));
      }
      // SAFETY: kernarg points to `size` writable bytes; zero it, then copy the args prefix.
      unsafe {
            ptr::write_bytes(kernarg.cast::<u8>(), 0, size);
            ptr::copy_nonoverlapping(bytes.as_ptr(), kernarg.cast::<u8>(), bytes.len());
      }
      return Ok(kernarg);
}

fn open_ro(path: &Path) -> Result<i32> {
      let c = CString::new(path.as_os_str().as_bytes())
            .map_err(|_e| AsmError(format!("path has interior NUL: {}", path.display())))?;
      // SAFETY: c is a valid NUL-terminated path; O_RDONLY (0) opens it read-only.
      let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
      if fd < 0 {
            return err(format!("open {}: {}", path.display(), std::io::Error::last_os_error()));
      }
      return Ok(fd);
}

fn assemble(path: &Path) -> Result<PathBuf> {
      let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
      if ext == "hsaco" || ext == "co" {
            return Ok(path.to_path_buf());
      }
      if ext != "s" {
            return err(format!("unrecognized code-object extension: {}", path.display()));
      }
      let object = path.with_extension("hsaco");
      if fresh(&object, path) {
            return Ok(object);
      }
      let rocm = env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_owned());
      let clang = format!("{rocm}/llvm/bin/clang");
      let arch = env::var("GPU_ARCH").unwrap_or_else(|_e| "gfx1101".to_owned());
      let out = Command::new(&clang)
            .arg("-target")
            .arg("amdgcn-amd-amdhsa")
            .arg(format!("-mcpu={arch}"))
            .arg(path)
            .arg("-o")
            .arg(&object)
            .output()
            .map_err(|e| AsmError(format!("spawn {clang}: {e}")))?;
      if !out.status.success() {
            return err(format!(
                  "assemble {} for {arch} failed:\n{}",
                  path.display(),
                  String::from_utf8_lossy(&out.stderr)
            ));
      }
      return Ok(object);
}

fn fresh(object: &Path, source: &Path) -> bool {
      let (Ok(obj), Ok(src)) = (fs::metadata(object), fs::metadata(source)) else {
            return false;
      };
      return obj.mtime() >= src.mtime();
}

fn kernel_names(object: &Path) -> Result<Vec<String>> {
      let rocm = env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_owned());
      let readelf = format!("{rocm}/llvm/bin/llvm-readelf");
      let out = Command::new(&readelf)
            .arg("-s")
            .arg(object)
            .output()
            .map_err(|e| AsmError(format!("spawn {readelf}: {e}")))?;
      if !out.status.success() {
            return err(format!(
                  "llvm-readelf {}: {}",
                  object.display(),
                  String::from_utf8_lossy(&out.stderr)
            ));
      }
      let text = String::from_utf8_lossy(&out.stdout);
      let mut names = Vec::new();
      for line in text.lines() {
            let Some(sym) = line.split_whitespace().last() else {
                  continue;
            };
            if let Some(base) = sym.strip_suffix(".kd") {
                  if !base.is_empty() && !names.iter().any(|n: &String| n == base) {
                        names.push(base.to_owned());
                  }
            }
      }
      return Ok(names);
}
