use crate::compile::{SourceUnit, Target};

pub fn compile_and_link(_unit: &SourceUnit, target: &Target) -> anyhow::Result<Vec<u8>> {
      return Err(anyhow::anyhow!(
            "nvJitLink route: NVIDIA runtime linking not available on this vendor ({})",
            target.arch
      ));
}
