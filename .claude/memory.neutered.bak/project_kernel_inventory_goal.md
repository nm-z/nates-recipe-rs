---
name: project_kernel_inventory_goal
description: "The real completion condition for the gpu-core kernel expansion — prove each kernel_inventory JSON item operates the same, implement missing"
metadata: 
  node_type: memory
  type: project
  originSessionId: cea6d039-8630-495c-9a35-b344784ca5fe
---

The gpu-core kernel expansion's **completion condition** (stated by user 2026-06-01, via stop-hook): "functionally match the [items] in the json. for each item in the json prove it can operate the same. for missing ops, implement those."

**Goal is NOT a kernel count** — it's per-item operational proof against `kernel_inventory/*.json`. Each JSON item = `{name, category, signature, description, dtypes, fused, library, url, trivial}`. The `description`+`signature`+`category` define expected behavior. "Prove operates the same" = run the gpu-core op on input, assert output matches the item's semantics (CPU/library oracle) within tolerance, ON the live gfx1101 GPU.

Inventory size: ~12,871 named entries across amd(1339)/nvidia(1612)/pytorch(1536)/rapids(1275)/transformers(785)/tjx(6469). Heavily inflated by dtype variants (sscal/dscal/cscal/zscal) and aliases (abs/absolute) — real unique-op count is far lower after canonicalization.

Right architecture: a **data-driven proof harness** that loads the JSON and, per item, dispatches the mapped gpu-core `gpu_*` op vs a category/name oracle — NOT hand-written per-item tests. Coverage report = proven / unmapped / missing. Implement gaps until all green.

Context: a concurrent swarm (session aee38bf3) already built ~16 modules (263 `launch_*`, 378 `gpu_*`) into gpu-core; `cargo build -p gpu-core` is green. This session's job = the gaps + the proofs. See [[project_gpu_core_expansion]]. GPU IS present now (gfx1101 RX 7700 XT) — runtime proof is possible, unlike when [[project_gpu_core_expansion]] was written.

**Progress (2026-06-01):** Harness lives at `gpu-core/tests/inventory_proof.rs` (run: `cargo test -p gpu-core --test inventory_proof -- --nocapture`; needs dev-deps serde_json + libm). **Unified dedup coverage 2223/13016 (17.1%), green.** PARALLEL-AGENT RECIPE THAT WORKS: Workflow fan-out, one agent per category, each writes ONLY `kernels/<cat>x.hip` + `tests/prove_<cat>.rs` (own FFI + public GpuBuffer + authoritative oracle), no shared-file edits. CRITICAL: server rate-limits >2 concurrent agents → run in **chunks of 2** (sequential waves) with retry; and DROP the agent `schema` (forced StructuredOutput makes them fail) — return plain text `RESULT <cat>: proven=N ...`. Done so far: mathx/gapact/actx (me) + agent categories elementwise_unary/binary, special, loss, pool, norm(groupnorm+instancenorm!), reduction, shape, indexing, sort, distance, padding, scan. In flight/next: quantized, optimizer, conv, activation, creation, foreach, embedding, histogram, search, set, then linalg(1750)/attention/fft/rnn/sparse/graph. Unified count = per-category max(inventory_proof_cat, prove_<cat>_count), + disjoint-union for elementwise_unary. The OLD line below is superseded: 7 proof shapes: unary/binary/reduce/scan registries + `complex_proofs()` (gemm/dot/scal/softmax/log_softmax/gemv/ger/cholesky/transpose). `canon()` strips vendor prefixes (rocblas_/cublas_…) + BLAS dtype letter so sgemm/dgemm→gemm (gemm alone = 150 items). **To extend coverage:** add a `gpu_*` op to the matching registry with a CPU/libm oracle + a `canon` alias; or implement a gap kernel (new `.hip` + `k_*.rs` + lib.rs `pub mod`) then register it. **Gotcha:** after adding a NEW .hip, `touch` an existing .hip so build.rs re-discovers it. Implemented this session: k_mathx (19 unary math), k_gapact (elu/selu/mish/softplus/hardswish/swiglu/geglu), k_actx (relu6/hardsigmoid/hardtanh/softsign/tanhshrink/logsigmoid/gelu_exact/celu/softshrink/hardshrink/thresholdedrelu). Deferred (ambiguous layout, task #3): syrk, svd/qr/lu/eigh reconstruction proofs. Build-break fix: see [[reference_gpucore_build_gotcha]].
