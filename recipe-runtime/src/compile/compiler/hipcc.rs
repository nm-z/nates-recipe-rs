use crate::compile::{SourceUnit, Target};
use anyhow::Context;
use std::io::Write as IoWrite;
use std::process::Command;

pub fn compile(unit: &SourceUnit, target: &Target) -> anyhow::Result<Vec<u8>> {
      let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_e| "/opt/rocm".to_string());
      let hipcc = std::env::var("HIPCC").unwrap_or_else(|_e| format!("{rocm}/bin/hipcc"));

      let tmp = std::env::temp_dir().join("recipe_hipcc_gen");
      std::fs::create_dir_all(&tmp).context("create hipcc tmp dir")?;

      let src_path = tmp.join("recipe_gen.hip");
      let out_path = tmp.join("recipe_gen.co");

      {
            let mut f = std::fs::File::create(&src_path).context("write source")?;
            f.write_all(unit.hip_source.as_bytes()).context("write source bytes")?;
      }

      for (name, content) in &unit.headers {
            let hdr_path = tmp.join(name);
            if let Some(parent) = hdr_path.parent() {
                  std::fs::create_dir_all(parent).ok();
            }
            std::fs::write(&hdr_path, content).context("write header")?;
      }

      let mut cmd = Command::new(&hipcc);
      cmd.arg("-c")
            .arg("-fPIC")
            .arg("--genco")
            .arg(&format!("--offload-arch={}", target.arch))
            .arg("-O3")
            .arg("-std=c++17")
            .arg("-o")
            .arg(&out_path)
            .arg(&src_path);

      for (key, val) in &unit.definitions {
            cmd.arg(format!("-D{key}={val}"));
      }

      cmd.arg(format!("-I{tmp}", tmp = tmp.display()));

      let output = cmd.output().context("hipcc spawn")?;

      let stderr = String::from_utf8_lossy(&output.stderr);
      if !output.status.success() {
            return Err(anyhow::anyhow!(
                  "hipcc failed (exit {}):\n{stderr}",
                  output.status.code().unwrap_or(-1)
            ));
      }

      if !stderr.is_empty() {
            ogdl::log::Write::line(ogdl::log::gpu, &format!("hipcc: {stderr}"));
      }

      let code = std::fs::read(&out_path).context("read compiled code object")?;
      return Ok(code);
}
