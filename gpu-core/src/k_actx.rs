use crate::HipError;
use crate::callspy::{GET_LAST_ERROR, LAUNCH, tick};
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::memory::GpuBuffer;
use core::ffi::c_void;
use core::ptr;

/// Ticks the launch counters and returns the pending HIP error, if any.
///
/// # Errors
/// Returns [`HipError`] when the preceding kernel launch reported a failure.
fn e() -> Result<(), HipError> {
	tick(&LAUNCH);
	tick(&GET_LAST_ERROR);
	// SAFETY: hipGetLastError only reads and clears thread-local HIP error state.
	return check(unsafe { hipGetLastError() });
}

/// Defines parameterless activation launchers over `(x, n) -> out`.
macro_rules! a0 {
    ($($name:ident => $launch:ident),* $(,)?) => {
        unsafe extern "C" { $( fn $launch(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void); )* }
        $(
            /// Launches the activation kernel over `n` elements.
            ///
            /// # Errors
            /// Returns [`HipError`] if `n` overflows [`i32`] or the launch fails.
            #[inline]
            pub fn $name(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
                let n = ci(n)?;
                // SAFETY: FFI launch with device pointers from live buffers and a range-checked length.
                unsafe { $launch(x.ptr_raw().cast_const(), out.ptr_raw(), n, ptr::null_mut()); }
                return e();
            }
        )*
    };
}
/// Defines one-parameter activation launchers over `(x, p, n) -> out`.
macro_rules! a1 {
    ($($name:ident => $launch:ident),* $(,)?) => {
        unsafe extern "C" { $( fn $launch(x: *const c_void, out: *mut c_void, n: i32, p: *const c_void, s: *mut c_void); )* }
        $(
            /// Launches the parameterized activation kernel over `n` elements.
            ///
            /// # Errors
            /// Returns [`HipError`] if `n` overflows [`i32`] or the launch fails.
            #[inline]
            pub fn $name(x: &GpuBuffer, p: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
                let n = ci(n)?;
                // SAFETY: FFI launch with device pointers from live buffers and a range-checked length.
                unsafe { $launch(x.ptr_raw().cast_const(), out.ptr_raw(), n, p.ptr_raw().cast_const(), ptr::null_mut()); }
                return e();
            }
        )*
    };
}

a0! {
    gpu_relu6 => launch_actx_relu6,
    gpu_hardsigmoid => launch_actx_hardsigmoid,
    gpu_hardtanh => launch_actx_hardtanh,
    gpu_softsign => launch_actx_softsign,
    gpu_tanhshrink => launch_actx_tanhshrink,
    gpu_logsigmoid => launch_actx_logsigmoid,
    gpu_gelu_exact => launch_actx_gelu_exact,
    gpu_softshrink => launch_actx_softshrink,
}
a1! {
    gpu_celu => launch_actx_celu,
    gpu_hardshrink => launch_actx_hardshrink,
    gpu_softshrink_p => launch_actx_softshrink_p,
    gpu_thresholdedrelu => launch_actx_thresholdedrelu,
}
