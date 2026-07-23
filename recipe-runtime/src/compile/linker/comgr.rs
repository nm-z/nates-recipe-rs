use crate::compile::{SourceUnit, Target};
use anyhow::Context;
use std::ffi::CString;
use std::ptr;

// ─

type ComgrStatus = i32;
type ComgrDataSet = u64;
type ComgrData = u64;
type ComgrActionInfo = u64;

const STATUS_SUCCESS: i32 = 0;

const DATA_KIND_SOURCE: i32 = 0x0;
const DATA_KIND_INCLUDE: i32 = 0x1;
const DATA_KIND_RELOCATABLE: i32 = 0x3;
const DATA_KIND_EXECUTABLE: i32 = 0x4;
const DATA_KIND_BC: i32 = 0x10;

const ACTION_COMPILE_SOURCE_TO_BC: i32 = 0x3;
const ACTION_LINK_BC_TO_BC: i32 = 0x5;
const ACTION_CODEGEN_BC_TO_RELOCATABLE: i32 = 0x8;
const ACTION_LINK_RELOCATABLE_TO_EXECUTABLE: i32 = 0xB;
const ACTION_COMPILE_SOURCE_TO_RELOCATABLE: i32 = 0xD;

unsafe extern "C" {
      fn amd_comgr_create_data_set(ds: *mut ComgrDataSet) -> ComgrStatus;
      fn amd_comgr_destroy_data_set(ds: ComgrDataSet) -> ComgrStatus;
      fn amd_comgr_create_data(kind: i32, data: *mut ComgrData) -> ComgrStatus;
      fn amd_comgr_release_data(data: ComgrData) -> ComgrStatus;
      fn amd_comgr_set_data(data: ComgrData, size: usize, bytes: *const u8) -> ComgrStatus;
      fn amd_comgr_set_data_name(data: ComgrData, name: *const i8) -> ComgrStatus;
      fn amd_comgr_data_set_add(ds: ComgrDataSet, data: ComgrData) -> ComgrStatus;
      fn amd_comgr_create_action_info(info: *mut ComgrActionInfo) -> ComgrStatus;
      fn amd_comgr_destroy_action_info(info: ComgrActionInfo) -> ComgrStatus;
      fn amd_comgr_action_info_set_isa_name(
            info: ComgrActionInfo,
            isa: *const i8,
      ) -> ComgrStatus;
      fn amd_comgr_action_info_set_language(info: ComgrActionInfo, lang: i32) -> ComgrStatus;
      fn amd_comgr_action_info_set_option_list(
            info: ComgrActionInfo,
            options: *const *const i8,
            count: usize,
      ) -> ComgrStatus;
      fn amd_comgr_do_action(
            action: i32,
            info: ComgrActionInfo,
            input: ComgrDataSet,
            output: ComgrDataSet,
      ) -> ComgrStatus;
      fn amd_comgr_action_data_count(ds: ComgrDataSet, kind: i32, count: *mut usize)
            -> ComgrStatus;
      fn amd_comgr_action_data_get_data(
            ds: ComgrDataSet,
            kind: i32,
            index: usize,
            data: *mut ComgrData,
      ) -> ComgrStatus;
      fn amd_comgr_get_data(
            data: ComgrData,
            size: *mut usize,
            bytes: *mut u8,
      ) -> ComgrStatus;
}

const LANGUAGE_HIP: i32 = 3;

// ─

fn check(rc: ComgrStatus, ctx: &str) -> anyhow::Result<()> {
      if rc != STATUS_SUCCESS {
            return Err(anyhow::anyhow!("comgr {ctx} failed: {rc}"));
      }
      return Ok(());
}

pub fn compile_and_link(unit: &SourceUnit, target: &Target) -> anyhow::Result<Vec<u8>> {
      let isa = CString::new(format!("amdgcn-amd-amdhsa--{}", target.arch)).context("isa")?;

      let mut input_ds: ComgrDataSet = 0;
      // SAFETY: comgr data set creation, input_ds is valid out-pointer
      unsafe {
            check(amd_comgr_create_data_set(&mut input_ds), "create input set")?;
      }

      let src_c = CString::new(unit.hip_source.as_str()).context("source null byte")?;
      let src_name = CString::new("recipe_gen.hip").context("name")?;
      let mut src_data: ComgrData = 0;
      // SAFETY: creating a source data object; all CStrings are live
      unsafe {
            check(
                  amd_comgr_create_data(DATA_KIND_SOURCE, &mut src_data),
                  "create source data",
            )?;
            check(
                  amd_comgr_set_data(src_data, unit.hip_source.len(), src_c.as_ptr() as *const u8),
                  "set source data",
            )?;
            check(
                  amd_comgr_set_data_name(src_data, src_name.as_ptr()),
                  "set source name",
            )?;
            check(
                  amd_comgr_data_set_add(input_ds, src_data),
                  "add source to set",
            )?;
      }

      for (name, content) in &unit.headers {
            let hdr_c = CString::new(content.as_str()).unwrap_or_default();
            let hdr_name = CString::new(name.as_str()).unwrap_or_default();
            let mut hdr_data: ComgrData = 0;
            // SAFETY: creating header data objects; CStrings are live
            unsafe {
                  check(
                        amd_comgr_create_data(DATA_KIND_INCLUDE, &mut hdr_data),
                        "create header",
                  )?;
                  check(
                        amd_comgr_set_data(hdr_data, content.len(), hdr_c.as_ptr() as *const u8),
                        "set header data",
                  )?;
                  check(
                        amd_comgr_set_data_name(hdr_data, hdr_name.as_ptr()),
                        "set header name",
                  )?;
                  check(
                        amd_comgr_data_set_add(input_ds, hdr_data),
                        "add header to set",
                  )?;
            }
      }

      let mut action_info: ComgrActionInfo = 0;
      // SAFETY: creating action info; isa CString is live
      unsafe {
            check(
                  amd_comgr_create_action_info(&mut action_info),
                  "create action info",
            )?;
            check(
                  amd_comgr_action_info_set_isa_name(action_info, isa.as_ptr()),
                  "set isa",
            )?;
            check(
                  amd_comgr_action_info_set_language(action_info, LANGUAGE_HIP),
                  "set language",
            )?;
      }

      let mut opts: Vec<CString> = vec![
            CString::new("-O3").unwrap_or_default(),
            CString::new("-std=c++17").unwrap_or_default(),
      ];
      for (key, val) in &unit.definitions {
            opts.push(CString::new(format!("-D{key}={val}")).unwrap_or_default());
      }
      let opt_ptrs: Vec<*const i8> = opts.iter().map(|c| c.as_ptr()).collect();
      // SAFETY: opt_ptrs are valid CString pointers
      unsafe {
            check(
                  amd_comgr_action_info_set_option_list(
                        action_info,
                        opt_ptrs.as_ptr(),
                        opt_ptrs.len(),
                  ),
                  "set options",
            )?;
      }

      let mut reloc_ds: ComgrDataSet = 0;
      // SAFETY: comgr data set creation
      unsafe {
            check(amd_comgr_create_data_set(&mut reloc_ds), "create reloc set")?;
      }

      // SAFETY: performing the compile-to-relocatable action
      unsafe {
            let rc = amd_comgr_do_action(
                  ACTION_COMPILE_SOURCE_TO_RELOCATABLE,
                  action_info,
                  input_ds,
                  reloc_ds,
            );
            if rc != STATUS_SUCCESS {
                  amd_comgr_destroy_action_info(action_info);
                  amd_comgr_destroy_data_set(input_ds);
                  amd_comgr_destroy_data_set(reloc_ds);
                  return Err(anyhow::anyhow!("comgr compile-to-relocatable failed: {rc}"));
            }
      }

      let mut exec_ds: ComgrDataSet = 0;
      // SAFETY: creating output set and performing link action
      unsafe {
            check(amd_comgr_create_data_set(&mut exec_ds), "create exec set")?;
            let rc = amd_comgr_do_action(
                  ACTION_LINK_RELOCATABLE_TO_EXECUTABLE,
                  action_info,
                  reloc_ds,
                  exec_ds,
            );
            if rc != STATUS_SUCCESS {
                  amd_comgr_destroy_action_info(action_info);
                  amd_comgr_destroy_data_set(input_ds);
                  amd_comgr_destroy_data_set(reloc_ds);
                  amd_comgr_destroy_data_set(exec_ds);
                  return Err(anyhow::anyhow!("comgr link-to-executable failed: {rc}"));
            }
      }

      let mut count: usize = 0;
      // SAFETY: querying the output set for executable data objects
      unsafe {
            check(
                  amd_comgr_action_data_count(exec_ds, DATA_KIND_EXECUTABLE, &mut count),
                  "count executables",
            )?;
      }
      if count == 0 {
            // SAFETY: cleanup
            unsafe {
                  amd_comgr_destroy_action_info(action_info);
                  amd_comgr_destroy_data_set(input_ds);
                  amd_comgr_destroy_data_set(reloc_ds);
                  amd_comgr_destroy_data_set(exec_ds);
            }
            return Err(anyhow::anyhow!("comgr produced no executable"));
      }

      let mut exec_data: ComgrData = 0;
      let mut size: usize = 0;
      // SAFETY: extracting the executable code object bytes
      unsafe {
            check(
                  amd_comgr_action_data_get_data(exec_ds, DATA_KIND_EXECUTABLE, 0, &mut exec_data),
                  "get executable data",
            )?;
            check(
                  amd_comgr_get_data(exec_data, &mut size, ptr::null_mut()),
                  "get data size",
            )?;
      }
      let mut code = vec![0u8; size];
      // SAFETY: code buffer sized correctly; reading into it
      unsafe {
            check(
                  amd_comgr_get_data(exec_data, &mut size, code.as_mut_ptr()),
                  "get data bytes",
            )?;
            amd_comgr_release_data(exec_data);
            amd_comgr_destroy_action_info(action_info);
            amd_comgr_destroy_data_set(input_ds);
            amd_comgr_destroy_data_set(reloc_ds);
            amd_comgr_destroy_data_set(exec_ds);
      }

      return Ok(code);
}
