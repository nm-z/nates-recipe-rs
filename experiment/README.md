# Measured tile RAT

`bench.rs` is the complete experiment surface. It declares the initial tile
LUT and exactly two models: benchmark model B and tile model T. It does not load
or train on VNA data.

`.loss(&benchmark)` composes T through frozen B. Recipe fits B to the measured
native epoch score for each proposed tile, then trains T against zero through
B. The LUT's target values are therefore not user data: the real native epoch
supplies them.

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
