# Measured tile RAT

`bench.rs` is the complete experiment surface. It declares the real workload,
the benchmark model B, and the tile model T. `recipe.rs` owns hardware
validation, fused-epoch measurement, numerical comparison, score assignment,
and T backpropagation through frozen B.

T proposes three log2 ratios from the analytic tile, which spans every positive
`u32` extent without a lookup table. Recipe assigns `-99_999_999` without
dispatch when a proposal violates the compiled hardware constraints or changes
the resulting epoch state. A valid, repeatable proposal receives its negative
median epoch time. B preserves valid timing ratios by scaling them against the
largest valid magnitude and maps the exact invalid score below that valid band.

Run it through the public Recipe entrypoint:

```bash
cargo build --release
RECIPE_DEVICE=amd0 target/release/recipe experiment/bench.rs
```
