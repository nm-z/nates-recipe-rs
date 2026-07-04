---
name: reference_nates_gpu_ruby_require
description: How to load the nates-gpu Ruby extension from a standalone script (require path + symlink gotcha)
metadata: 
  node_type: memory
  type: reference
  originSessionId: 9ad35a6f-c740-41a6-b5ff-c2d3e6f52f4a
---

To use `NatesGpu` from a standalone Ruby script, require the **named** symlink, not the raw lib:

```ruby
require File.expand_path("nates-gpu-ruby/target/release/nates_gpu.so", __dir__)
```

`target/release/nates_gpu.so` is an absolute symlink → `libnates_gpu.so`. You must require a file named `nates_gpu.so` (not `libnates_gpu.so`) so Ruby looks up the `Init_nates_gpu` entry symbol — requiring `libnates_gpu.so` directly fails (`Init_libnates_gpu` not found).

Gotcha (fixed 2026-06): the convenience symlink at the repo's `nates-gpu-ruby/nates_gpu.so` was **broken** — it pointed to the relative `nates-gpu-ruby/target/release/libnates_gpu.so`, which from inside `nates-gpu-ruby/` resolves to a doubled `nates-gpu-ruby/nates-gpu-ruby/...` path. Fix was `ln -sf target/release/libnates_gpu.so nates_gpu.so`.

Verified API shape for a linear-regression loop (all f64, row-major up/download): `upload(flat, rows, cols)`, `download`, `zeros(r,c)`, `reduce_mean_cols`/`reduce_var_cols`/`sqrt` + broadcast operators (`a - v`, `a / v` when `v.rows==1`) for z-scoring, `linear(x,w,b)` forward (needs `x.cols==w.rows`, `w` is `(feat,out)`, `b` is `(1,out)`), `gemm(x, grad, "T", "N")` = `xᵀ·grad` for the weight gradient, `reduce_sum_cols` for the bias gradient, and `sgd_update(w, g, lr)` in-place `w -= lr·g`. Working example: `linreg_gd_penguins.rb` at repo root. See [[feedback_ruby_gem_build]], [[feedback_rbs_updated]].

**Bare calls via `include NatesGpu` (no prefix).** The magnus bindings are registered with `define_module_function`, which makes each fn BOTH a singleton method (`NatesGpu.linear`) AND a private instance method. A top-level `include NatesGpu` mixes the instance methods into `Object`, so you can call `upload(...)`, `linear(...)`, `gemm(...)`, `sgd_update(...)` bare — and even top-level `def`s (e.g. a `zscore` helper) see them. Both styles work; `kaggle_s6e4/solve_v2.rb` uses the explicit `NatesGpu.` prefix, `linreg_gd_penguins.rb` uses `include`. Caveat: `include` also pulls short names (`sum`, `mean`, `max`, `min`, `log`, `exp`, `pow`, `sqrt`) into `Object`, so a bare `sum(...)` would hit the GPU fn — keep an explicit receiver (`arr.sum`) for Array/Math intent. Confirmed working against the built `.so` on the live GPU.
