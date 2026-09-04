# Online kernel tuning: measured behaviour on current minimal

## What the feature does

An AMD training run whose placed route resolves to a single device tunes its own
contraction kernel while it trains. A knob model proposes a configuration from
the search space the device reports; a bench model learns epoch time from the
epochs the run already performs and supplies the knob model's gradient. Both
control models run on the CPU backend. The workload stays on the GPU.

`Cargo.toml` sets the observation budget, the learning rate, the per-proposal
compile budget, and the path of each saved model. New models bootstrap from real
observations; saved models continue online. A route placed across several
devices keeps the schedule it was planned with, because one measured epoch time
does not describe any single device in that route.

## Workload

The unchanged `train-temp-fill.rs` from `vna-models-repro.zip`, SHA-256
`d1644713ed4ebe3bca84c63aa590d5ac39c47edec8896648ef875effe3801030`. It trains
10,000 epochs with seed 29, FP32, AdamW, and the original split, logging, and
checkpoint behaviour. The forward load is 36 rows, two outputs, 2,400 inputs.

All numbers are `recipe --device amd0 train-temp-fill.rs` on engi amd0
(gfx1030), under `nice -n 10` with a 12 GB virtual-memory cap, native artifact
caches warm, one job at a time. The base is `minimal` at `10148e25`.

## Untuned behaviour is unchanged

Untuned, this branch reproduces the base byte for byte: held-out R-squared
`0.9602`, Huber `3.2063`, saved-model SHA-256
`6e912c535d04980261c72b2b12341fe1bef0a0e0d798ed7f1988beee66ef906b`.

That is the check on the kernel changes. The fragment-extent gate and the
output-lane striding loop added to the forward, gradient, and previous-input
contraction bodies leave every schedule the base can pick untouched, and the
four schedule parameters moved from build time to run time resolve to the values
`Cargo.toml` baked before.

## The control models were refit

The state this branch previously recorded was fitted against the kernel at
`9098de87`. Minimal has since rewritten the contraction inner loop and changed
how a schedule's local-memory footprint is counted, so those weights name
schedules that are legal but slow here. They were discarded and both models were
bootstrapped again against the current kernel, twelve real observations over the
workload above, no CSV and no synthetic benchmark.

The refit state is preserved at `~/.local/share/recipe/rat/refit-2026-09-04`:

- `bench.ogdl`: `1a39af4926850ee7c0bfa731752cd869345c88691fdf9216da29ac76620991be`
- `knob.ogdl`: `af0e245be58e088afef4e17a3d68fdd277b7efdad709caa770177eb2f8591710`

## Timing after the refit

Two alternating pairs, base then head, each head run restarted from the refit
state:

| pair | base `10148e25` | this branch |
| --- | --- | --- |
| 1 | 5.679 s | 15.454 s |
| 2 | 5.313 s | 15.158 s |

The tuner keeps a schedule it measured at about 0.98 ms per epoch, against about
0.30 ms per epoch for the schedule the base picks without tuning. The refit
improves on the stale state, which measured 1.5 ms and worse, but it does not
reach the untuned schedule.

Continued online updating does not converge toward the untuned schedule either.
Three further runs from the refit state measured 15.6 s, 21.9 s and 23.2 s, with
the kept proposal drifting from 1.52 ms to 2.07 ms per epoch.

## Tuning changes the trained bytes

The base saves
`6e912c535d04980261c72b2b12341fe1bef0a0e0d798ed7f1988beee66ef906b` with
R-squared `0.9602` and Huber `3.2063`. The refit head saves
`07a3132ddfc17de4925f6a273ca50bceb56903d4e83dd8d59b6a3019a13ad3fc` with
R-squared `0.9569` and Huber `3.6070`.

Both are valid fits. They differ because the contraction tile fixes the order in
which the K extent is summed, and the tuner changes the tile, so a tuned run is
not bit-comparable with an untuned one. Any claim that tuning leaves the saved
model identical holds only for the schedules that happen to preserve that order.

Inference agrees across backends. Evaluating a saved model through the public
`recipe.infer` path over thirty inputs, sixty outputs in all, the largest
absolute difference between amd0 and the CPU backend is `0.0249` on values near
`7.9e3`, about three parts in a million in FP32.

## Two admissions the tuned path needs

A tuned choice states the tile outright instead of deriving it, so it has to be
held to the rules the untuned schedule satisfies by construction.

The staged tile and its partials must fit the per-wave local-memory budget. A
choice that overruns it used to reach the GPU and return wrong numbers, a
negative Huber loss among them; it is now reported as unusable, and the tuner
scores it and proposes another.

A proposal whose kernel the compiler cannot produce within `rat-compile-budget-ms`
is likewise unusable. Without that bound one exploratory proposal had not reached
its first measured epoch after twelve minutes, against 5.3 s for the whole
untuned run, because the staging helpers carry the reduction fragment as a vector
value of the selected width.

The searched grid and workgroup are the X extents. The contraction kernels index
one dimension, so reporting the device's Y and Z maxima enumerates launches the
kernel cannot distinguish, and the search then costs far more than the epochs it
saves.
