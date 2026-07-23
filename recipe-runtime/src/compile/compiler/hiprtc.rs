use crate::compile::{SourceUnit, Target};
use anyhow::Context;
use std::ffi::CString;
use std::ptr;

// ─

type HiprtcResult = i32;
type HiprtcProgram = *mut std::ffi::c_void;

const HIPRTC_SUCCESS: i32 = 0;

unsafe extern "C" {
      fn hiprtcCreateProgram(
            prog: *mut HiprtcProgram,
            src: *const i8,
            name: *const i8,
            num_headers: i32,
            headers: *const *const i8,
            include_names: *const *const i8,
      ) -> HiprtcResult;

      fn hiprtcCompileProgram(
            prog: HiprtcProgram,
            num_options: i32,
            options: *const *const i8,
      ) -> HiprtcResult;

      fn hiprtcGetCodeSize(prog: HiprtcProgram, size: *mut usize) -> HiprtcResult;

      fn hiprtcGetCode(prog: HiprtcProgram, code: *mut u8) -> HiprtcResult;

      fn hiprtcGetProgramLogSize(prog: HiprtcProgram, size: *mut usize) -> HiprtcResult;

      fn hiprtcGetProgramLog(prog: HiprtcProgram, log: *mut u8) -> HiprtcResult;

      fn hiprtcDestroyProgram(prog: *mut HiprtcProgram) -> HiprtcResult;
}

// ─

fn get_log(prog: HiprtcProgram) -> String {
      let mut log_size: usize = 0;
      // SAFETY: prog is a live hiprtcProgram handle; log_size is a valid pointer
      unsafe {
            if hiprtcGetProgramLogSize(prog, &mut log_size) != HIPRTC_SUCCESS || log_size <= 1 {
                  return String::new();
            }
            let mut buf = vec![0u8; log_size];
            hiprtcGetProgramLog(prog, buf.as_mut_ptr());
            buf.truncate(log_size.saturating_sub(1));
            return String::from_utf8_lossy(&buf).into_owned();
      }
}

pub fn compile(unit: &SourceUnit, target: &Target) -> anyhow::Result<Vec<u8>> {
      let src_c = CString::new(unit.hip_source.as_str()).context("source contains null byte")?;
      let name_c = CString::new("recipe_gen.hip").context("name")?;

      let header_srcs: Vec<CString> = unit
            .headers
            .iter()
            .map(|(_name, content)| CString::new(content.as_str()).unwrap_or_default())
            .collect();
      let header_names: Vec<CString> = unit
            .headers
            .iter()
            .map(|(name, _content)| CString::new(name.as_str()).unwrap_or_default())
            .collect();
      let header_src_ptrs: Vec<*const i8> = header_srcs.iter().map(|c| c.as_ptr()).collect();
      let header_name_ptrs: Vec<*const i8> = header_names.iter().map(|c| c.as_ptr()).collect();

      let mut prog: HiprtcProgram = ptr::null_mut();

      // SAFETY: hiprtcCreateProgram takes source + headers as C strings; all CStrings are live
      unsafe {
            let rc = hiprtcCreateProgram(
                  &mut prog,
                  src_c.as_ptr(),
                  name_c.as_ptr(),
                  header_srcs.len() as i32,
                  if header_src_ptrs.is_empty() {
                        ptr::null()
                  } else {
                        header_src_ptrs.as_ptr()
                  },
                  if header_name_ptrs.is_empty() {
                        ptr::null()
                  } else {
                        header_name_ptrs.as_ptr()
                  },
            );
            if rc != HIPRTC_SUCCESS {
                  return Err(anyhow::anyhow!("hiprtcCreateProgram failed: {rc}"));
            }
      }

      let arch_opt = format!("--gpu-architecture={}", target.arch);
      let mut opts: Vec<CString> = vec![CString::new(arch_opt).context("arch opt")?];
      for (key, val) in &unit.definitions {
            opts.push(CString::new(format!("-D{key}={val}")).context("def")?);
      }
      let opt_ptrs: Vec<*const i8> = opts.iter().map(|c| c.as_ptr()).collect();

      // SAFETY: prog is live; opt_ptrs are valid CString pointers
      let compile_rc = unsafe {
            hiprtcCompileProgram(prog, opt_ptrs.len() as i32, opt_ptrs.as_ptr())
      };

      let log = get_log(prog);

      if compile_rc != HIPRTC_SUCCESS {
            // SAFETY: destroying a failed-but-created program
            unsafe {
                  hiprtcDestroyProgram(&mut prog);
            }
            return Err(anyhow::anyhow!(
                  "hiprtcCompileProgram failed (rc={compile_rc}):\n{log}"
            ));
      }

      let mut code_size: usize = 0;
      // SAFETY: prog compiled successfully; code_size is valid pointer
      unsafe {
            let rc = hiprtcGetCodeSize(prog, &mut code_size);
            if rc != HIPRTC_SUCCESS {
                  hiprtcDestroyProgram(&mut prog);
                  return Err(anyhow::anyhow!("hiprtcGetCodeSize failed: {rc}"));
            }
      }

      let mut code = vec![0u8; code_size];
      // SAFETY: code buffer is correctly sized from hiprtcGetCodeSize
      unsafe {
            let rc = hiprtcGetCode(prog, code.as_mut_ptr());
            hiprtcDestroyProgram(&mut prog);
            if rc != HIPRTC_SUCCESS {
                  return Err(anyhow::anyhow!("hiprtcGetCode failed: {rc}"));
            }
      }

      if !log.is_empty() {
            ogdl::log::Write::line(ogdl::log::gpu, &format!("hiprtc: {log}"));
      }

      return Ok(code);
}
