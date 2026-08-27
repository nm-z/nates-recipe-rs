# experiment/rat-lut

```
vna.rs   an ordinary training script: fill percentage from a VNA sweep
rat.rs   the RAT: R/P/M/B/T/L over an m/n/k lookup table, per node, per direction
```

`vna.rs` is just training. The RAT runs it untouched, times it from the outside,
and reads the model it saves, so nothing is instrumented into it.

## Why it exists

#378 and #93

## How it dispatches tiles



## How it checks numerics

`rat.rs` compares the saved model.

## The loop

https://github.com/nm-z/recipe-dev/issues/93#issuecomment-5365779577

## Run

```bash
cargo build --release
./target/release/recipe experiment/rat.rs
```

```
RECIPE_BIN    target/release/recipe   driver binary
RAT_SCRIPT    experiment/vna.rs       workload
RAT_MODEL     vna.ogdl                model the workload saves
RAT_BUDGET    120                     measurements
RAT_REPEATS   3                       timings per candidate, min taken
RAT_EXPLORE   0.25                    random exploration rate
RAT_HIDDEN    24                      hidden units in B and T
RAT_RATE      0.02                    learning rate for B and T
RAT_SEED      17                      seed for B, T and exploration
```

## First result

72 measurements, `RAT_REPEATS=2`, on gfx1101, against the four contraction nodes
of `vna.rs`:

```
71 cells proposed, 1 repeat skipped
63 rejected by the compiled resource bounds   (89 %)
 8 dispatched and timed
 0 faster than Recipe's own selection
 0 changed the trained model
baseline 1.716 s, selected 1.716 s
```

The eight that dispatched ran between 1.717 s and 3.130 s against a 1.716 s
baseline, so the reachable part of the grid is not merely no better, it is
mostly worse.

The LUT is mostly unreachable, and not for a subtle reason. `native_extent_valid`
requires `m % register_m == 0` and `n % register_n == 0`, both 8 here, so every
`n = 4` cell is invalid on arrival; it requires `k % chunk_k == 0` unless `k`
equals the shape's own K, and `chunk_k` is 64, so of `k` in
`{8, 16, 32, 64, 128}` only 64 and 128 are generally legal; and it caps
`(m / 8) * (n / 8)` at the 64-lane workgroup, which removes the large `m` by
large `n` corner. Every one of the eight cells that did dispatch has `n` a
multiple of 8 and `k` in `{64, 128}`, except one whose `k = 16` happened to
equal that direction's K exactly. The measured rejections land precisely where
those three rules predict.

So the grid worth searching on this kernel is not the one in the issue. It is
closer to:

```
m in {16, 32, 64, 128, 256}      already multiples of register_m
n in {8, 16, 32, 64}             drop 4
k in {64, 128} or the shape's K  multiples of chunk_k
```

Note also that the heuristic extents this workload starts from -- K of 5, 48,
150, 320 -- are not on the proposed grid at all, so an LUT search cannot even
express the schedule it is trying to beat.

## Known limits

The candidate space is bounded by the heuristic, not by the hardware.
`shared_values`, `chunk_values` and `chunk_bias_values` are computed as maxima
over the heuristic assignment and baked into the artifact, so any cell needing
more is rejected before it can be timed. That is why nothing here beat the
formula: the search is confined to the neighbourhood the formula already chose,
and inside that neighbourhood the formula is already at a local optimum.
Reserving over the candidate space rather than the heuristic is the prerequisite
for an LUT-shaped search to mean anything.

Timing resolution is the other limit. Each measurement is the wall time of a
whole `recipe experiment/vna.rs` invocation, so it carries a constant of process
startup, script compilation and data preparation. That constant shifts every
candidate equally and does not favour one schedule, but it does compress
relative differences, and run-to-run spread on an unchanged binary is still tens
of percent. Raise `RAT_REPEATS`, or lengthen the training in `vna.rs`, before
believing a small margin.

The numerical check found nothing here only because nothing was accepted. On a
dense model, tuner-selectable schedules do change the trained result, so the
check stays.
