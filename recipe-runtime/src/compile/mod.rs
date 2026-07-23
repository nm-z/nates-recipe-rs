use recipe_ir::graph::SemanticGraph;

pub mod compiler;
pub mod linker;

// ─

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
      Amd,
      Nvidia,
}

#[derive(Debug, Clone)]
pub struct Target {
      pub vendor: Vendor,
      pub arch: String,
}

#[derive(Debug)]
pub struct SourceUnit {
      pub hip_source: String,
      pub entry_symbol: String,
      pub headers: Vec<(String, String)>,
      pub definitions: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Artifact {
      pub code_object: Vec<u8>,
      pub target: Target,
      pub entry_symbol: String,
      pub route: Route,
      pub diagnostics: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
      RuntimeLink,
      RuntimeCompile,
      ExternalCompile,
}

#[derive(Debug)]
pub struct RouteFailure {
      pub route: Route,
      pub diagnostics: String,
}

// ─

pub fn detect_target() -> anyhow::Result<Target> {
      let arch = std::env::var("GPU_ARCH").unwrap_or_else(|_e| "gfx1101".to_string());
      let vendor = match arch.starts_with("gfx") || arch.starts_with("amdgcn") {
            true => Vendor::Amd,
            false => Vendor::Nvidia,
      };
      return Ok(Target { vendor, arch });
}

pub fn compile(unit: &SourceUnit, target: &Target) -> anyhow::Result<Artifact> {
      let mut failures: Vec<RouteFailure> = Vec::new();

      match try_route(Route::RuntimeLink, unit, target) {
            Ok(artifact) => return Ok(artifact),
            Err(f) => failures.push(f),
      }

      match try_route(Route::RuntimeCompile, unit, target) {
            Ok(artifact) => return Ok(artifact),
            Err(f) => failures.push(f),
      }

      match try_route(Route::ExternalCompile, unit, target) {
            Ok(artifact) => return Ok(artifact),
            Err(f) => failures.push(f),
      }

      let mut msg = String::from("all compilation routes failed:\n");
      for f in &failures {
            msg.push_str(&format!("  {:?}: {}\n", f.route, f.diagnostics));
      }
      return Err(anyhow::anyhow!("{msg}"));
}

fn try_route(
      route: Route,
      unit: &SourceUnit,
      target: &Target,
) -> Result<Artifact, RouteFailure> {
      let result = match (route, target.vendor) {
            (Route::RuntimeLink, Vendor::Amd) => {
                  linker::comgr::compile_and_link(unit, target)
            }
            (Route::RuntimeLink, Vendor::Nvidia) => {
                  linker::nvjitlink::compile_and_link(unit, target)
            }
            (Route::RuntimeCompile, Vendor::Amd) => {
                  compiler::hiprtc::compile(unit, target)
            }
            (Route::RuntimeCompile, Vendor::Nvidia) => {
                  compiler::hiprtc::compile(unit, target)
            }
            (Route::ExternalCompile, Vendor::Amd) => {
                  compiler::hipcc::compile(unit, target)
            }
            (Route::ExternalCompile, Vendor::Nvidia) => {
                  compiler::hipcc::compile(unit, target)
            }
      };
      return match result {
            Ok(code_object) => Ok(Artifact {
                  code_object,
                  target: target.clone(),
                  entry_symbol: unit.entry_symbol.clone(),
                  route,
                  diagnostics: String::new(),
            }),
            Err(e) => Err(RouteFailure {
                  route,
                  diagnostics: format!("{e:#}"),
            }),
      };
}

// ─

pub fn generate_train_source(
      graph: &SemanticGraph,
      dims: &[recipe_ir::LayerDims],
      loss: recipe_ir::Loss,
      n: usize,
      k: usize,
      lr: f64,
) -> SourceUnit {
      let mut src = String::with_capacity(8192);
      src.push_str("#include <hip/hip_runtime.h>\n\n");

      emit_train_kernel(&mut src, graph, dims, loss, n, k, lr);

      return SourceUnit {
            hip_source: src,
            entry_symbol: "recipe_train".to_string(),
            headers: Vec::new(),
            definitions: Vec::new(),
      };
}

fn emit_train_kernel(
      src: &mut String,
      _graph: &SemanticGraph,
      dims: &[recipe_ir::LayerDims],
      loss: recipe_ir::Loss,
      n: usize,
      _k: usize,
      lr: f64,
) {
      src.push_str("extern \"C\" __global__ void recipe_train(\n");
      src.push_str("      double* __restrict__ arena,\n");
      src.push_str("      const double* __restrict__ x,\n");
      src.push_str("      const double* __restrict__ y,\n");
      src.push_str("      double* __restrict__ metric_out,\n");
      src.push_str("      int epochs,\n");
      src.push_str("      int epoch_bound\n");
      src.push_str(") {\n");

      let mut offset: usize = 0;
      for (i, ld) in dims.iter().enumerate() {
            let w_elems = ld.in_dim * ld.out_dim;
            let b_elems = ld.out_dim;
            src.push_str(&format!(
                  "      double* w{i} = arena + {offset};\n"
            ));
            offset += w_elems;
            src.push_str(&format!(
                  "      double* b{i} = arena + {offset};\n"
            ));
            offset += b_elems;
      }

      src.push_str(&format!(
            "      double* acts = arena + {offset};\n"
      ));

      src.push_str("\n      for (int e = 0; e < epoch_bound && e < epochs; e++) {\n");

      let mut prev_buf = "x";
      for (i, ld) in dims.iter().enumerate() {
            let _act_offset = i * n * ld.out_dim;
            src.push_str(&format!(
                  "            // layer {i}: {prev_buf}[{n}x{}] @ w{i}[{}x{}] + b{i}\n",
                  ld.in_dim, ld.in_dim, ld.out_dim
            ));
            match ld.kind {
                  recipe_ir::LayerKind::Dense => {
                        emit_dense_fwd(src, i, prev_buf, n, ld);
                  }
                  _other => {
                        src.push_str(&format!(
                              "            // non-dense layer {i} placeholder\n"
                        ));
                  }
            }
            prev_buf = "acts";
      }

      emit_loss_bwd(src, loss, dims, n, _k, lr);

      src.push_str("      }\n");
      src.push_str("}\n");
}

fn emit_dense_fwd(src: &mut String, idx: usize, input: &str, n: usize, ld: &recipe_ir::LayerDims) {
      src.push_str(&format!(
            "            for (int r = 0; r < {n}; r++) {{\n"
      ));
      src.push_str(&format!(
            "                  for (int c = 0; c < {}; c++) {{\n",
            ld.out_dim
      ));
      src.push_str(&format!(
            "                        double sum = b{idx}[c];\n"
      ));
      src.push_str(&format!(
            "                        for (int j = 0; j < {}; j++) {{\n",
            ld.in_dim
      ));
      src.push_str(&format!(
            "                              sum += {input}[r * {} + j] * w{idx}[j * {} + c];\n",
            ld.in_dim, ld.out_dim
      ));
      src.push_str("                        }\n");

      match ld.act {
            recipe_ir::Activation::Relu => {
                  src.push_str("                        sum = sum > 0.0 ? sum : 0.0;\n");
            }
            recipe_ir::Activation::LeakyRelu => {
                  src.push_str("                        sum = sum > 0.0 ? sum : 0.01 * sum;\n");
            }
            recipe_ir::Activation::Sigmoid => {
                  src.push_str("                        sum = 1.0 / (1.0 + exp(-sum));\n");
            }
            recipe_ir::Activation::Tanh => {
                  src.push_str("                        sum = tanh(sum);\n");
            }
            recipe_ir::Activation::Gelu => {
                  src.push_str("                        sum = 0.5 * sum * (1.0 + tanh(0.7978845608 * (sum + 0.044715 * sum * sum * sum)));\n");
            }
            recipe_ir::Activation::Silu => {
                  src.push_str("                        sum = sum / (1.0 + exp(-sum));\n");
            }
            _linear => {}
      }

      src.push_str(&format!(
            "                        acts[r * {} + c] = sum;\n",
            ld.out_dim
      ));
      src.push_str("                  }\n");
      src.push_str("            }\n");
}

fn emit_loss_bwd(
      src: &mut String,
      loss: recipe_ir::Loss,
      dims: &[recipe_ir::LayerDims],
      n: usize,
      _k: usize,
      lr: f64,
) {
      let last = match dims.last() {
            Some(d) => d,
            None => return,
      };
      let total = n * last.out_dim;

      src.push_str(&format!(
            "\n            // loss gradient ({:?})\n",
            loss
      ));
      src.push_str(&format!(
            "            double inv_n = 1.0 / {n}.0;\n"
      ));

      match loss {
            recipe_ir::Loss::Mse => {
                  src.push_str(&format!(
                        "            for (int j = 0; j < {total}; j++) {{\n"
                  ));
                  src.push_str(
                        "                  acts[j] = 2.0 * inv_n * (acts[j] - y[j]);\n",
                  );
                  src.push_str("            }\n");
            }
            recipe_ir::Loss::Mae => {
                  src.push_str(&format!(
                        "            for (int j = 0; j < {total}; j++) {{\n"
                  ));
                  src.push_str(
                        "                  double d = acts[j] - y[j];\n",
                  );
                  src.push_str(
                        "                  acts[j] = (d > 0.0 ? 1.0 : -1.0) * inv_n;\n",
                  );
                  src.push_str("            }\n");
            }
            _other => {
                  src.push_str(&format!(
                        "            for (int j = 0; j < {total}; j++) {{\n"
                  ));
                  src.push_str(
                        "                  acts[j] = 2.0 * inv_n * (acts[j] - y[j]);\n",
                  );
                  src.push_str("            }\n");
            }
      }

      src.push_str("\n            // SGD update\n");
      for i in (0..dims.len()).rev() {
            let ld = &dims[i];
            src.push_str(&format!(
                  "            for (int p = 0; p < {}; p++) {{\n",
                  ld.in_dim * ld.out_dim
            ));
            src.push_str(&format!(
                  "                  w{i}[p] -= {lr} * w{i}[p];\n"
            ));
            src.push_str("            }\n");
      }

      src.push_str(&format!(
            "\n            metric_out[0] = 0.0;\n"
      ));
      src.push_str(&format!(
            "            for (int j = 0; j < {total}; j++) {{\n"
      ));
      src.push_str(
            "                  double d = acts[j] - y[j];\n",
      );
      src.push_str(
            "                  metric_out[0] += d * d;\n",
      );
      src.push_str("            }\n");
      src.push_str(&format!(
            "            metric_out[0] *= inv_n;\n"
      ));
}
