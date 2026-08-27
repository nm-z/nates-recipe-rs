# experiment/rat-lut

Experimental branch. Nothing here is part of the library, nothing here is
proposed for `minimal`, and `recipe.rs` is unmodified from #378.

Two scripts:

```
rat.rs   the RAT: R/P/M/B/T/L over an m/n/k lookup table, per node, per direction
vna.rs   the workload it tunes: fill percentage from a VNA sweep
```

## Why it exists

#378 resolves #93 by measuring candidate schedules, but its selector is a
regressor plus an argmin. Against the design in the issue thread it has B and
nothing else:

| piece | #378 | here |
| --- | --- | --- |
| B, benchmark surrogate | `fit_xgboost` over schedule features | one-hidden-layer tanh net |
| T, tile proposer | absent, plain argmin | one-hidden-layer tanh net over the LUT |
| L, lookup state | absent, flat vectors refit each pick | per-slot selected neurons and stored measurements |
| LUT | absent, extents derived from register multiples | m/n/k grid, 125 cells |
| selected-neuron updates | absent | per-slot masks on both nets |
| continuous learning | absent, one shot then frozen in cache | loop persists across the budget |

This is that loop written out, so the design can be measured rather than argued
about.

## How it dispatches tiles

Recipe has no public tile knob. #378 reads a schedule cache beside the native
artifact before the first epoch, and a cache hit short-circuits its own tuner,
so writing that file dispatches an arbitrary assignment through the ordinary
public `train().run()` path. No library changes, no internal state read as
proof.

The same channel is the validity oracle. When an extent violates a resource
bound the compiled artifact reserved, Recipe rejects the whole file and
retunes, which overwrites it. `rat.rs` reads the file back after every run: a
file that survived was the file that ran.

## How it checks numerics

#378 accepts a candidate when the epoch loss is unchanged. That check is
computed before the reverse pass, so it cannot observe a gradient or previous
tile at all, and for a forward tile it is a mean over every row, which absorbs
per-element differences. Measured on a dense model, a tile that costs 32 % of
final accuracy passes it.

`rat.rs` compares the trained model instead: a digest over prediction bit
patterns after a long enough run for a difference to surface. Any cell that
measures faster but changes that digest is rejected and reported by name.

## The loop

```
state -> L                        selected B neurons, selected T neurons, memory
L + state -> T -> action          argmax over the LUT, with random exploration
L + state + action -> B -> P      predicted seconds
state + action -> benchmark -> M  measured seconds, real fused epochs
M -> L
difference(P, M) -> backward      updates the selected B neurons
objective(P) -> backward          through frozen B, updates the selected T neurons
```

T's choice is discrete, so it is made differentiable the way Recipe's own RAT
does it in `lower_estimator`: the softmax over LUT cells forms an expected
action embedding, B scores that embedding, and the gradient of the score
reaches T's logits. The argmax cell is what actually gets benchmarked.

## Running

```bash
cargo build --release
unzip vna-temp-fill-data-v2.zip -d ~/Desktop/vna-temp-fill-data-v2
./target/release/recipe experiment/rat.rs
```

| variable | default | meaning |
| --- | --- | --- |
| `RECIPE_BIN` | `target/release/recipe` | driver binary |
| `RAT_SCRIPT` | `experiment/vna.rs` | workload |
| `VNA_DATA` | `~/Desktop/vna-temp-fill-data-v2` | data root |
| `RAT_BUDGET` | 120 | measurements |
| `RAT_MEASURE_EPOCHS` | 40 | epochs per timing |
| `RAT_REPEATS` | 3 | timings per candidate, min taken |
| `RAT_CHECK_EPOCHS` | 300 | epochs for the numerical check |
| `RAT_EXPLORE` | 0.25 | random exploration rate |
| `RAT_HIDDEN` | 24 | hidden units in B and T |
| `RAT_SEED` | 17 | |

## First result

72 measurements, `RAT_REPEATS=2`, `RAT_MEASURE_EPOCHS=20`, `RAT_CHECK_EPOCHS=150`,
on gfx1101, against the four contraction nodes of `vna.rs`:

```
65 of 72 cells rejected by the compiled resource bounds   (90 %)
 7 cells dispatched and timed
 0 cells faster than the heuristic
 0 cells changed the trained model
baseline 0.132004 s, selected 0.132004 s over 20 epochs
```

The LUT is mostly unreachable, and not for a subtle reason. `native_extent_valid`
requires `m % register_m == 0` and `n % register_n == 0`, both 8 here, so every
`n = 4` cell is invalid on arrival; it requires `k % chunk_k == 0` unless `k`
equals the shape's own K, and `chunk_k` is 64, so of `k` in
`{8, 16, 32, 64, 128}` only 64 and 128 are generally legal; and it caps
`(m / 8) * (n / 8)` at the 64-lane workgroup, which removes the large `m` by
large `n` corner. Every one of the seven cells that did dispatch has `n` a
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

Timing resolution is the other limit. A 20-epoch measurement on this workload is
around 130 ms and run-to-run spread on an unchanged binary is tens of percent,
so a single-digit-percent difference is not separable from noise at small repeat
counts. Raise `RAT_MEASURE_EPOCHS` and `RAT_REPEATS` before believing a margin.

The numerical check found nothing here only because nothing was accepted. On a
dense model, tuner-selectable schedules do change the trained result, so the
check stays.
