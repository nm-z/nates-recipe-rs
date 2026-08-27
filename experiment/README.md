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

...

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

Each measured row reports:

```
step     position in the budget
node     contraction node index, the same one the schedule cache lines carry
dir      0 forward, 1 weight gradient, 2 input gradient
m n k    the LUT cell being tried in that slot
P        seconds B predicted before anything ran
M        seconds measured, NaN when the extent never dispatched
verdict  rejected    the compiled resource bounds refused the extent, nothing was timed
         slower      dispatched and timed, but no faster than the running best
         accepted    faster, and the trained model is unchanged, so it is adopted
         changes the trained model    the saved model differs from the reference, so it is refused whatever it timed
```

## First result

71 cells proposed
63 rejected
 8 dispatched
baseline 1.716 s, selected 1.716 s
```

```
m in {16, 32, 64, 128, 256}
n in {8, 16, 32, 64}
k in {64, 128}
```
