use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

macro_rules! mx {
    ($($name:ident => $launch:ident),* $(,)?) => {
        unsafe extern "C" { $( fn $launch(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void); )* }
        $(
            pub fn $name(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
                let o = GpuBuffer::alloc(n)?;
                unsafe { $launch(x.ptr_raw() as *const c_void, o.ptr_raw(), n as i32, std::ptr::null_mut()); }
                crate::callspy::tick(&crate::callspy::LAUNCH);
                crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
                check(unsafe { crate::hip::hipGetLastError() })?;
                Ok(o)
            }
        )*
    };
}

mx! {
    gpu_square  => launch_mx_square,
    gpu_exp2    => launch_mx_exp2,
    gpu_log2    => launch_mx_log2,
    gpu_log10   => launch_mx_log10,
    gpu_cbrt    => launch_mx_cbrt,
    gpu_sinh    => launch_mx_sinh,
    gpu_cosh    => launch_mx_cosh,
    gpu_asin    => launch_mx_asin,
    gpu_acos    => launch_mx_acos,
    gpu_atan    => launch_mx_atan,
    gpu_asinh   => launch_mx_asinh,
    gpu_acosh   => launch_mx_acosh,
    gpu_atanh   => launch_mx_atanh,
    gpu_erf     => launch_mx_erf,
    gpu_erfc    => launch_mx_erfc,
    gpu_tgamma  => launch_mx_tgamma,
    gpu_lgamma  => launch_mx_lgamma,
    gpu_deg2rad => launch_mx_deg2rad,
    gpu_rad2deg => launch_mx_rad2deg,
}
