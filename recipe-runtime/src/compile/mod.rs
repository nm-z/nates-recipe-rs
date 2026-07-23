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

pub fn arena_workspace(dims: &[recipe_ir::LayerDims], n: usize) -> usize {
      let wp: usize = dims.iter().map(|ld| ld.in_dim * ld.out_dim + ld.out_dim).sum();
      let fwd: usize = dims.iter().map(|ld| 2 * n * ld.out_dim).sum();
      let max_out = dims.iter().map(|ld| ld.out_dim).max().unwrap_or(1);
      let grad = 2 * n * max_out;
      return wp + fwd + grad;
}

pub fn generate_train_source(
      graph: &SemanticGraph,
      dims: &[recipe_ir::LayerDims],
      loss: recipe_ir::Loss,
      n: usize,
      k: usize,
      lr: f64,
) -> SourceUnit {
      let mut src = String::with_capacity(16384);
      src.push_str("#include <hip/hip_runtime.h>\n");
      src.push_str("#include <math.h>\n\n");

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
            src.push_str(&format!("      double* w{i} = arena + {offset};\n"));
            offset += ld.in_dim * ld.out_dim;
            src.push_str(&format!("      double* b{i} = arena + {offset};\n"));
            offset += ld.out_dim;
      }

      for (i, ld) in dims.iter().enumerate() {
            let sz = n * ld.out_dim;
            src.push_str(&format!("      double* z{i} = arena + {offset};\n"));
            offset += sz;
            src.push_str(&format!("      double* a{i} = arena + {offset};\n"));
            offset += sz;
      }

      let max_out = dims.iter().map(|ld| ld.out_dim).max().unwrap_or(1);
      let grad_sz = n * max_out;
      src.push_str(&format!("      double* delta = arena + {offset};\n"));
      offset += grad_sz;
      src.push_str(&format!("      double* delta2 = arena + {offset};\n"));

      src.push_str("\n      for (int e = 0; e < epoch_bound && e < epochs; e++) {\n");

      for (i, ld) in dims.iter().enumerate() {
            let input = if i == 0 { "x".to_string() } else { format!("a{}", i - 1) };
            emit_dense_fwd(src, i, &input, ld.in_dim, ld.out_dim, n, ld.act);
      }

      let last = dims.len() - 1;
      let last_out = dims[last].out_dim;
      emit_loss_grad(src, loss, last, n, last_out);

      for ri in 0..dims.len() {
            let i = dims.len() - 1 - ri;
            let ld = &dims[i];
            let input = if i == 0 { "x".to_string() } else { format!("a{}", i - 1) };
            let cur = if ri % 2 == 0 { "delta" } else { "delta2" };
            let nxt = if ri % 2 == 0 { "delta2" } else { "delta" };
            emit_layer_bwd(src, i, &input, ld.in_dim, ld.out_dim, n, lr, ld.act, cur, nxt, i > 0);
      }

      emit_metric(src, last, n, last_out);

      src.push_str("      }\n");
      src.push_str("}\n");
}

fn emit_dense_fwd(
      src: &mut String,
      idx: usize,
      input: &str,
      in_dim: usize,
      out_dim: usize,
      n: usize,
      act: recipe_ir::Activation,
) {
      src.push_str(&format!("            for (int r = 0; r < {n}; r++) {{\n"));
      src.push_str(&format!("                  for (int c = 0; c < {out_dim}; c++) {{\n"));
      src.push_str(&format!("                        double sum = b{idx}[c];\n"));
      src.push_str(&format!("                        for (int j = 0; j < {in_dim}; j++) {{\n"));
      src.push_str(&format!("                              sum += {input}[r * {in_dim} + j] * w{idx}[j * {out_dim} + c];\n"));
      src.push_str("                        }\n");
      src.push_str(&format!("                        z{idx}[r * {out_dim} + c] = sum;\n"));

      match act {
            recipe_ir::Activation::Relu =>
                  src.push_str("                        sum = sum > 0.0 ? sum : 0.0;\n"),
            recipe_ir::Activation::LeakyRelu =>
                  src.push_str("                        sum = sum > 0.0 ? sum : 0.01 * sum;\n"),
            recipe_ir::Activation::PRelu =>
                  src.push_str("                        sum = sum > 0.0 ? sum : 0.25 * sum;\n"),
            recipe_ir::Activation::Elu =>
                  src.push_str("                        sum = sum > 0.0 ? sum : exp(sum) - 1.0;\n"),
            recipe_ir::Activation::Selu => {
                  src.push_str("                        sum = sum > 0.0 ? 1.0507009873554805 * sum : 1.0507009873554805 * 1.6732632423543772 * (exp(sum) - 1.0);\n");
            }
            recipe_ir::Activation::Sigmoid =>
                  src.push_str("                        sum = 1.0 / (1.0 + exp(-sum));\n"),
            recipe_ir::Activation::Tanh =>
                  src.push_str("                        sum = tanh(sum);\n"),
            recipe_ir::Activation::Gelu =>
                  src.push_str("                        sum = 0.5 * sum * (1.0 + tanh(0.7978845608 * (sum + 0.044715 * sum * sum * sum)));\n"),
            recipe_ir::Activation::Silu =>
                  src.push_str("                        sum = sum / (1.0 + exp(-sum));\n"),
            recipe_ir::Activation::Linear => {}
      }

      src.push_str(&format!("                        a{idx}[r * {out_dim} + c] = sum;\n"));
      src.push_str("                  }\n");
      src.push_str("            }\n");
}

fn emit_loss_grad(
      src: &mut String,
      loss: recipe_ir::Loss,
      last: usize,
      n: usize,
      last_out: usize,
) {
      let total = n * last_out;
      src.push_str(&format!("            double inv_n = 1.0 / {n}.0;\n"));
      match loss {
            recipe_ir::Loss::Mse => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  delta[j] = 2.0 * inv_n * (a{last}[j] - y[j]);\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Loss::Mae => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  double d = a{last}[j] - y[j];\n"));
                  src.push_str("                  delta[j] = (d > 0.0 ? 1.0 : -1.0) * inv_n;\n");
                  src.push_str("            }\n");
            }
            recipe_ir::Loss::Bce => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  double p = a{last}[j];\n"));
                  src.push_str("                  double pc = fmax(fmin(p, 1.0 - 1e-15), 1e-15);\n");
                  src.push_str("                  delta[j] = inv_n * (-y[j] / pc + (1.0 - y[j]) / (1.0 - pc));\n");
                  src.push_str("            }\n");
            }
            recipe_ir::Loss::Ce => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  delta[j] = inv_n * (a{last}[j] - y[j]);\n"));
                  src.push_str("            }\n");
            }
            _other => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  delta[j] = 2.0 * inv_n * (a{last}[j] - y[j]);\n"));
                  src.push_str("            }\n");
            }
      }
}

fn emit_act_deriv_mul(
      src: &mut String,
      idx: usize,
      out_dim: usize,
      n: usize,
      act: recipe_ir::Activation,
      delta_buf: &str,
) {
      let total = n * out_dim;
      match act {
            recipe_ir::Activation::Linear => {}
            recipe_ir::Activation::Relu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= (z{idx}[j] > 0.0) ? 1.0 : 0.0;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::LeakyRelu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= (z{idx}[j] > 0.0) ? 1.0 : 0.01;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::PRelu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= (z{idx}[j] > 0.0) ? 1.0 : 0.25;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Elu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= (z{idx}[j] > 0.0) ? 1.0 : a{idx}[j] + 1.0;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Selu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= (z{idx}[j] > 0.0) ? 1.0507009873554805 : a{idx}[j] + 1.0507009873554805 * 1.6732632423543772;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Sigmoid => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= a{idx}[j] * (1.0 - a{idx}[j]);\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Tanh => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  {delta_buf}[j] *= 1.0 - a{idx}[j] * a{idx}[j];\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Gelu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  double zv = z{idx}[j];\n"));
                  src.push_str("                  double kk = 0.7978845608 * (zv + 0.044715 * zv * zv * zv);\n");
                  src.push_str("                  double tk = tanh(kk);\n");
                  src.push_str("                  double gprime = 0.5 * (1.0 + tk) + 0.5 * zv * (1.0 - tk * tk) * 0.7978845608 * (1.0 + 3.0 * 0.044715 * zv * zv);\n");
                  src.push_str(&format!("                  {delta_buf}[j] *= gprime;\n"));
                  src.push_str("            }\n");
            }
            recipe_ir::Activation::Silu => {
                  src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
                  src.push_str(&format!("                  double zv = z{idx}[j];\n"));
                  src.push_str("                  double sig = 1.0 / (1.0 + exp(-zv));\n");
                  src.push_str(&format!("                  {delta_buf}[j] *= sig * (1.0 + zv * (1.0 - sig));\n"));
                  src.push_str("            }\n");
            }
      }
}

fn emit_layer_bwd(
      src: &mut String,
      i: usize,
      input: &str,
      in_dim: usize,
      out_dim: usize,
      n: usize,
      lr: f64,
      act: recipe_ir::Activation,
      cur: &str,
      nxt: &str,
      propagate: bool,
) {
      emit_act_deriv_mul(src, i, out_dim, n, act, cur);

      if propagate {
            src.push_str(&format!("            for (int r = 0; r < {n}; r++) {{\n"));
            src.push_str(&format!("                  for (int j = 0; j < {in_dim}; j++) {{\n"));
            src.push_str("                        double dp = 0.0;\n");
            src.push_str(&format!("                        for (int c = 0; c < {out_dim}; c++) {{\n"));
            src.push_str(&format!("                              dp += {cur}[r * {out_dim} + c] * w{i}[j * {out_dim} + c];\n"));
            src.push_str("                        }\n");
            src.push_str(&format!("                        {nxt}[r * {in_dim} + j] = dp;\n"));
            src.push_str("                  }\n");
            src.push_str("            }\n");
      }

      src.push_str(&format!("            for (int j = 0; j < {in_dim}; j++) {{\n"));
      src.push_str(&format!("                  for (int c = 0; c < {out_dim}; c++) {{\n"));
      src.push_str("                        double dw = 0.0;\n");
      src.push_str(&format!("                        for (int r = 0; r < {n}; r++) {{\n"));
      src.push_str(&format!("                              dw += {input}[r * {in_dim} + j] * {cur}[r * {out_dim} + c];\n"));
      src.push_str("                        }\n");
      src.push_str(&format!("                        w{i}[j * {out_dim} + c] -= {lr} * dw;\n"));
      src.push_str("                  }\n");
      src.push_str("            }\n");

      src.push_str(&format!("            for (int c = 0; c < {out_dim}; c++) {{\n"));
      src.push_str("                  double db = 0.0;\n");
      src.push_str(&format!("                  for (int r = 0; r < {n}; r++) {{\n"));
      src.push_str(&format!("                        db += {cur}[r * {out_dim} + c];\n"));
      src.push_str("                  }\n");
      src.push_str(&format!("                  b{i}[c] -= {lr} * db;\n"));
      src.push_str("            }\n");
}

fn emit_metric(src: &mut String, last: usize, n: usize, last_out: usize) {
      let total = n * last_out;
      src.push_str("            metric_out[0] = 0.0;\n");
      src.push_str(&format!("            for (int j = 0; j < {total}; j++) {{\n"));
      src.push_str(&format!("                  double d = a{last}[j] - y[j];\n"));
      src.push_str("                  metric_out[0] += d * d;\n");
      src.push_str("            }\n");
      src.push_str(&format!("            metric_out[0] /= {total}.0;\n"));
}
