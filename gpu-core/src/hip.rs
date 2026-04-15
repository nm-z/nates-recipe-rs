use std::ffi::c_void;
use std::fmt;

#[derive(Debug)]
pub struct HipError(pub i32);

impl fmt::Display for HipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HIP error code {}", self.0)
    }
}
impl std::error::Error for HipError {}

pub fn check(code: i32) -> Result<(), HipError> {
    if code == 0 { Ok(()) } else { Err(HipError(code)) }
}

pub const HIP_MEMCPY_H2D: i32 = 1;
pub const HIP_MEMCPY_D2H: i32 = 2;
pub const HIP_MEMCPY_D2D: i32 = 3;

unsafe extern "C" {
    pub fn hipMalloc(ptr: *mut *mut c_void, size: usize) -> i32;
    pub fn hipFree(ptr: *mut c_void) -> i32;
    pub fn hipMemcpy(dst: *mut c_void, src: *const c_void, size: usize, kind: i32) -> i32;
    pub fn hipMemset(dst: *mut c_void, value: i32, size: usize) -> i32;
    pub fn hipGetLastError() -> i32;
    pub fn hipDeviceSynchronize() -> i32;
    pub fn hipEventCreate(event: *mut *mut c_void) -> i32;
    pub fn hipEventDestroy(event: *mut c_void) -> i32;
    pub fn hipEventRecord(event: *mut c_void, stream: *mut c_void) -> i32;
    pub fn hipEventSynchronize(event: *mut c_void) -> i32;
    pub fn hipEventElapsedTime(ms: *mut f32, start: *mut c_void, stop: *mut c_void) -> i32;
    pub fn hipSetDevice(device: i32) -> i32;
    pub fn hipStreamCreate(stream: *mut *mut c_void) -> i32;
    pub fn hipStreamSynchronize(stream: *mut c_void) -> i32;
    pub fn hipStreamDestroy(stream: *mut c_void) -> i32;
    pub fn hipMemGetInfo(free: *mut usize, total: *mut usize) -> i32;
}

pub fn mem_info() -> Result<(usize, usize), HipError> {
    let mut free: usize = 0;
    let mut total: usize = 0;
    check(unsafe { hipMemGetInfo(&mut free, &mut total) })?;
    Ok((free, total))
}

pub fn device_synchronize() -> Result<(), HipError> {
    check(unsafe { hipDeviceSynchronize() })
}

pub fn set_device(device: i32) -> Result<(), HipError> {
    check(unsafe { hipSetDevice(device) })
}
