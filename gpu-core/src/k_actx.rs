use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::{HipError, check};

fn e() -> Result<(), HipError> { check(unsafe { crate::hip::hipGetLastError() }) }

macro_rules! a0 {
    ($($name:ident => $launch:ident),* $(,)?) => {
        unsafe extern "C" { $( fn $launch(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void); )* }
        $(
            pub fn $name(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
                let o = GpuBuffer::alloc(n)?;
                unsafe { $launch(x.ptr_raw() as *const c_void, o.ptr_raw(), n as i32, std::ptr::null_mut()); }
                e()?; Ok(o)
            }
        )*
    };
}
macro_rules! a1 {
    ($($name:ident => $launch:ident),* $(,)?) => {
        unsafe extern "C" { $( fn $launch(x: *const c_void, out: *mut c_void, n: i32, p: f64, s: *mut c_void); )* }
        $(
            pub fn $name(x: &GpuBuffer, n: usize, p: f64) -> Result<GpuBuffer, HipError> {
                let o = GpuBuffer::alloc(n)?;
                unsafe { $launch(x.ptr_raw() as *const c_void, o.ptr_raw(), n as i32, p, std::ptr::null_mut()); }
                e()?; Ok(o)
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
