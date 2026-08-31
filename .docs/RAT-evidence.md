# RAT evidence

## 2026-08-31: real-workload online tuning

RAT now runs through the normal AMD training path. It queries the device and the supplied workload, predicts a configuration autoregressively, compiles that configuration, and measures actual training epochs. Both learned models update from those measurements. The synthetic matrix runner, truth-table enumerator, CSV reader and writer, and `recipe rat` alias are removed. Existing CSV artifacts are unchanged.

The current policy is defined in `Cargo.toml`: 10 bootstrap observations for new models, two online configuration observations for initialized models, eight actual workload epochs per observation, and 16 shuffled model proposals ranked by the bench model. The remaining workload epochs use the fastest configuration measured during that run. The bench model predicts positive log-time. Both models use Recipe's CPU backend; the user's training workload remains on AMD.

### Controlled comparison

The original `train-temp-fill.rs` from `/home/nate/Desktop/vna-models-repro.zip` is unchanged. Both branches train all 10,000 epochs with seed 29, AdamW, FP32, the original split, and the original logging and checkpoint behavior. The forward load is 36 samples, two outputs, and 2,400 inputs. The latest fetched `minimal` commit is `9098de8711552004bb9eabd84c6d8d9c120d4c58`.

Measured process wall time, including CLI compilation, online model updates, kernel compilation, training, logging, checkpointing, and evaluation:

- Minimal: 3.45 seconds.
- Cleaned RAT from the fixed pretrained state: 2.33 seconds, 32% less time.
- Both: held-out R-squared 0.9602 and Huber loss 3.2063.
- Both saved workload models: SHA-256 `6e912c535d04980261c72b2b12341fe1bef0a0e0d798ed7f1988beee66ef906b`.

These measurements use the installed runtime without an `LD_LIBRARY_PATH` override. The CLI runs under `systemd-run --wait --pipe --collect --uid=nate`, with `/usr/bin/time` measuring the executable. Output is drained continuously. No epoch count, model, dataset, or logging change is used to obtain the speedup.

The pretrained starting weights are preserved at `/home/nate/.local/share/recipe/rat/ranked-repeat-start`:

- `bench.ogdl`: SHA-256 `8328d31ccf86df5f92ee9081df86a9ebdd84227e95d0651c323f0d87021ed10a`.
- `knob.ogdl`: SHA-256 `9520e31c9c6078956c38c8e88f6d74d823a68462df7203649aaceeb796a8da96`.

This is a measured result for that pretrained state, not a guarantee for every learned configuration. A later continuously updated state took 4.76 seconds. Comparisons of implementation changes therefore restore the same pretrained starting weights, then include the online updates in the timed run. The updated weights are saved normally.

Run the workload from its extracted directory:

```sh
/home/nate/Desktop/recipe-dev/target/release/recipe --device amd0 train-temp-fill.rs
```

### Correctness and compiler repair

A repeated run exposed an unreachable 1,960-element vector load in the gradient kernel. LLVM allocated a 171,904-byte private frame and rejected it against its 131,068-byte limit. The selected tile extent was only 1,349. The shared fragment predicate now includes that compile-time extent, so the impossible vector branch is eliminated without changing the selected knobs or restricting the search space. The captured IR is in `rat-evidence/compiler-stack-failure.ll` and `rat-evidence/compiler-stack-optimized.ll`.

The exact formerly failing configuration now compiles and completes a real training epoch. Its 48 held-out predictions differ from the independently evaluated CPU path by at most `0.00012969970703125`. The 72 initial training predictions match. A fresh-model startup also completes that same public resume-and-train path and saves both learned models. Separate saved-model evaluation checks all 120 outputs against CPU.

The cleanup reduces the combined production source count for `recipe.rs`, `build.rs`, `amd-nv-cpu.ll`, `cli.rs`, and the removed `experiment/bench.rs` from 19,038 to 18,327 lines. `cargo check` completes without warnings. Earlier sections below describe superseded experiments, not the current execution path.

## 2026-08-30: total workgroup count overflow

Status: Recipe implementation defect. No RAT search-space constraint was added.

Observed command:

```text
RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 rat
```

The saved-model path skipped initial collection and made 10 online attempts. All 10 attempts were inside the imposed search space and failed before dispatch. The complete rows are in `knob-configs-online-failure-test.csv`.

One measured attempt used these values:

```text
M=3 N=2 K=1
gx=1193046723 gy=36408 gz=36408
wg=2779321882594752
attempt=knob.inside-ss.fail.invalid
time=999999999999 ms
error=knob wg does not fit the dispatch workgroup count: out of range integral type conversion attempted
```

The local failure occurs in `Geometry::from_knobs`, which converts `Knobs::wg` from `u64` to the internal `Geometry::groups` `u32` field. The AMD launch path uses that field for Recipe's whole-grid synchronization state. The AQL packet itself contains three `u32` grid dimensions and no scalar total-workgroup field.

The [HSA Platform System Architecture Specification 1.2](https://hsafoundation.com/wp-content/uploads/2021/02/HSA-SysArch-1.2.pdf), Appendix A, permits each grid dimension through `2^32 - 1` and the total grid size through `2^64 - 1`. Therefore, `wg <= 2^32 - 1` would incorrectly narrow the hardware-valid RAT space.

Required correction: remove the internal `u32` total-workgroup dependency from ordinary dispatch. Handle Recipe's whole-grid synchronization through the AOT kernel-boundary design instead of converting this implementation limit into a RAT constraint.

## 2026-08-30: 65,535 ordinary workgroups after terminal inference barrier removal

Status: valid RAT configuration. No RAT search-space constraint was added.

Observed command:

```text
RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 rat 32,1,1,1,1,65535,3,65535,1,1,65535,1,65535,1,1,65535,32,1,65535,1,1,1,1,1,1,1,1,1,1
```

Observed result:

```text
measurement loss 0.000000013385522156180812 seconds 0.000077642
elapsed=0.24
```

This configuration uses wave32, `gx=65,535`, `tpwgx=1`, and therefore `wg=65,535`. `RatMatrix::measure` checked every GPU output against `K * (row + 1) * (column + 1)` before returning the measured time. The command completed normally, so the exact matrix comparison passed.

The compiled one-layer `recipe_model_forward` in `recipe-native-0ebb0fe2b23708d3/artifact.hsaco` contained none of `global_atomic`, `s_sleep`, `ds_gws_barrier`, or `__ockl_grid_sync`. The inference emitter omits only the barrier following its final node. Training, reverse, and nonterminal node barriers remain.

Conclusion: `wg=65,535` is not inherently invalid or dangerous for this ordinary dispatch. The earlier residency concern came from Recipe's terminal whole-grid synchronization, not the hardware-valid workgroup count.

## 2026-08-30: inference tape allocated training state

Status: Recipe implementation defect. No RAT search-space constraint was added.

The first 10-level coupled-load sweep queried live free VRAM, applied the configured 80% budget, and failed at this row:

```text
M=2 N=1 K=194608848 wvmd=wave32
time=999999999999 ms
attempt=knob.inside-ss.fail.invalid
error=device amd0 Amd: Amd allocation failed: 4104
```

The complete interrupted run is preserved in `knob-configs-load-allocation-failure.csv`. HSA status 4104 is `HSA_STATUS_ERROR_OUT_OF_RESOURCES` (`0x1008`) in `/opt/rocm/include/hsa/hsa.h`.

`RatLoads` budgeted the three inference matrices, but `NativeTape::new` allocated training-only input adjoints, graph adjoints, optimizer moments, optimizer variances, frozen flags, and gradient storage even when `loss=None`. These allocations were not part of the modeled load and exhausted VRAM.

The correction keeps the existing `NativeTape` implementation and allocates those buffers at their one-element inactive size when `loss=None`. `retune_inference` no longer grows unused gradient storage. Training with a loss retains the full allocations.

The replay queried 11,330,265,088 free VRAM bytes and set its budget to `floor(free VRAM * 0.80) = 9,064,212,070` bytes. Because live free VRAM changed, its corresponding sixth load was larger than the failed load:

```text
M=2 N=1 K=209819723 wvmd=wave32
time=127639.68785599999 ms
attempt=knob.inside-ss.success
```

The same larger load also passed under wave64 in 130,101.687982 ms. Both rows passed the exact output comparison. This proves the allocation failure was caused by unused training state, not by an invalid M,N,K combination.

## 2026-08-30: five-second execution limit

Status: user-requested measurement policy. No search-space constraint or row limit was added.

`rat-row-timeout-seconds = 5` in `Cargo.toml` limits measured GPU execution. Compilation, allocation, and output verification are outside that measurement. Expired execution records `time=inf`, `valid=no`, and `attempt=knob.inside-ss.fail.timeout`. This does not assert that the configuration is invalid.

The AMD dispatch path inactivates and destroys the expired queue before creating the queue for subsequent dispatches. [HSA queue inactivation](https://rocm.docs.amd.com/projects/ROCR-Runtime/en/docs-7.14.0/api-reference/api.html) aborts pending executions; [ROCr's implementation](https://github.com/ROCm/ROCR-Runtime/blob/amd-staging/runtime/hsa-runtime/core/runtime/amd_aql_queue.cpp) destroys the driver queue in `AqlQueue::Inactivate`. A failed queue transition stops collection instead of continuing with a stale queue.

The public source entrypoint ran these two dense loads consecutively in one process:

```text
RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 experiment/rat-timeout.rs '32,1,1,1,1,1,3,1,1,1,1,1,1,1,1,1,32,1,1,1,1,1,1,1,1,1,1,1,1;100000;24'

M=107 N=100000 K=1: inf, GPU execution exceeded 5 s
M=107 N=45 K=1: 2.598106 ms, all 4815 outputs match
```

Both loads use wave32 with one thread. The second load verifies queue reuse after cancellation against the independently calculated matrix result. The CSV keeps `inf`; subsequent bench-model training uses the existing configured finite invalid-time penalty instead of an infinite regression target.

The earlier reduced sweep stopped at 29 completed rows. Its CSV is preserved in `knob-configs-before-five-second-limit.csv`. Requested numeric sampling counts are restored. Load sampling now updates N's bound after choosing M and K's bound after choosing M,N, using `bytes * (M*K + K*N + M*N + N) <= floor(free_vram * 0.80)`.

The same public sequence under wave64 also recorded `inf` for the large load, followed by a successful 2.721899 ms small load. Its 14.66 s total wall time includes setup and the explicit-configuration command's CPU benchmark epochs. It is not a collection throughput measurement.

## 2026-08-30: full requested sweep blocked by output capacity

The normal command `RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 rat` counted 509,913,600,000 configurations and stopped before collection. It did not lower any sampling count or overwrite the existing CSV.

```text
free VRAM bytes: 10119131136
load budget bytes: 8095304908
conditional M,N,K combinations: 178
complete knob configurations: 509913600000
minimum CSV bytes: 41303001600101
available output bytes: 81955879784
```

An independent integer calculation from those probed bounds reproduced the count exactly: 70 sampled workgroup-dimension triples, 1,000 grid-dimension triples per workgroup triple, both wave modes, and all sampled load, tile, register, and reduction combinations. All 178 loads satisfy the memory budget; 73 have M,N,K simultaneously above their minimum values.

The CSV alone requires at least 41.3 TB. The output filesystem has about 82 GB available. No full-sweep per-row rate has been measured because the disk check prevented it from starting.

## 2026-08-30: three-step sampling and compiler resource failure

The user changed numeric sampling to three interior positions: one quarter, one half, and three quarters of each conditional range. One-step fields still select their minimum. Both probed wave modes remain included. The public `rat query` command and an independent integer calculation each counted 1,064,340 configurations, requiring at least 136,235,621 CSV bytes.

The first full run reached LLVM with `M=121936180 N=2 K=2`, `m=30484045 n=1 k=1`, and `rm=30484045 rn=1`. Its staging allocation alone requires `(30484045 + 1) * 1 * 8 = 243872368` shared-memory bytes per workgroup. The device query reports `lds=65536`. LLVM received `<30484045 x double>` vectors and `[30484045 x double]` private arrays. The compiler consumed about 166 CPU seconds and 6,399,248 KiB RSS without completing the first row.

The generated source is archived at `rat-evidence/initial-three-step-vector.ll`, SHA-256 `f664558f8540350b54f0d0aa309224ed57b0ac63934087b4155446271df2b93f`. The compiler was manually terminated after interrupting the runtime. `rat-evidence/initial-three-step-interrupted.csv` preserves that resulting row; its termination message is not evidence of an LLVM or HSA rejection of the configuration.

Recipe already checked the requested shared-memory size, but only after compiling and loading the kernel. The correction checks the known tile allocation before compilation and retains the post-load check for the compiled kernel's additional requirements. It reports the requested and available byte counts. No sampled domain, step count, or search-space constraint changed.

The limit is per workgroup: [`HSA_REGION_INFO_ALLOC_MAX_SIZE`](https://rocm.docs.amd.com/projects/ROCR-Runtime/en/docs-7.14.0/api-reference/api.html) specifies the maximum group-segment allocation for a dispatch. The installed `/opt/rocm/include/hsa/hsa.h` documents this at lines 3299 through 3311. This is a resource failure of Recipe's current lowering, not proof that a different lowering could never implement the same abstract tile.

The unchanged rerun again counted 1,064,340 configurations. Its first row uses the new live load snapshot `M=125591194 N=2 K=2`, requests 251,182,400 shared-memory bytes per workgroup, and now records that resource failure without compiling the oversized vector.

That sweep completed all 1,064,340 rows. An independent CSV count found 1,064,340 `valid=no` rows, all reporting a tile shared-memory request above the queried limit. No row produced a valid GPU time. The complete 246,570,581-byte CSV is archived at `rat-evidence/three-step-shared-memory.csv`. Training was interrupted after collection; no bench or knob model weights were saved.

A small public dispatch with `M=107 N=45 K=1`, wave32, `tpwgx=gx=256`, `tpwgy=tpwgz=gy=gz=1`, and `m=n=k=1` passed the independent comparison of all 4,815 outputs with `rm=rn=4` in 7.015955 ms. Changing only `rm=rn=8` reached the compiled model resource check and reported 147,456 required shared-memory bytes against 65,536 available. The public entrypoint was `experiment/rat-timeout.rs` with width 24. Neither check changed a search-space domain.

## 2026-08-30: one-GB row budget

The user replaced the percentage budget with a maximum of 1 GB per row. `rat-vram-budget-bytes = 1000000000` is the authoritative TOML setting; the load generator uses the smaller of this cap and live free VRAM. All sampling counts and interior sample positions remain unchanged. The proposed additional shared-memory domain constraints were not applied.

An independent integer calculation gives 13 sampled M,N,K loads under this cap. The largest input, weights, bias, and output allocation total is 875,000,184 bytes, from `M=15625001 N=4 K=3` at eight bytes per value.

The public `RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 rat` rerun recorded all 1,064,340 configurations. Independent CSV arithmetic confirmed 13 load combinations, a maximum matrix allocation of 875,000,184 bytes, and zero rows above the 1,000,000,000-byte budget. Every row still failed Recipe's per-workgroup shared-memory check before GPU execution. The first row requested 31,250,016 bytes against the queried 65,536-byte limit.

The complete CSV is `rat-evidence/one-gb-three-step.csv` (242,225,741 bytes). A live KFD reading for runtime PID 825132 reported 198,729,728 VRAM bytes; this was a sample, not a peak measurement. Training was interrupted after collection, and neither model's weights were saved. The one-GB load budget did not resolve the separate shared-memory resource failure.

## 2026-08-30: shared-allocation corrections and coupled domains

The allocation audit found two obsolete reservations. The host added a full per-thread reduction buffer to the shared-memory requirement, although the kernel exchanges chunk partials in the existing contraction tile. The chunk-size calculation also reserved bias partials, although `contraction_bias_accumulate` keeps those sums in private memory. Both reservations were removed from the common allocation path and its local and remote consumers.

The unchanged `M=107 N=45 K=1`, `tpwg=256`, `m=n=k=1`, `rm=rn=8` configuration that had been rejected for requesting 147,456 bytes then executed correctly under wave32 and wave64. All 4,815 outputs matched, in 19.397342 ms and 20.584779 ms, respectively.

A boundary replay used `M=107 N=128 K=1`, `tpwg=256`, `m=rm=64`, `n=rn=128`, and `k=1`. Removing the unused bias reservation reduced its request from 66,560 bytes to 65,536 bytes. Wave32 then executed and all 13,696 outputs matched in 7.422287 ms. Wave64 reached LLVM, which rejected a 167,680-byte private stack frame against its reported 131,068-byte limit. That is a separate compiler-resource failure, not an LDS failure. The five raw replay records, including the complete compiler diagnostic, are archived in `rat-evidence/shared-allocation-replays.csv`.

Shared-memory arithmetic now uses wide integer intermediates and is checked against the queried device limit before narrowing the allocation size. A one-thread workgroup has no cross-thread chunk exchange and reserves no chunk-partial region.

The search-space domain calculation now calls the same `native_contraction_shared_values` implementation used by the allocator. It accounts for staged operands and chunk partials using the configured model width and accumulator width. The per-workgroup byte limit is documented by [`HSA_REGION_INFO_ALLOC_MAX_SIZE`](https://rocm.docs.amd.com/projects/ROCR-Runtime/en/docs-7.14.0/api-reference/api.html). No private-stack limit was added to the domains from the single compiler rejection.

The counter and enumerator share one component ordering. Initial enumeration selects the workgroup dimensions before the tile and register dimensions, so those domains incorporate the selected workgroup. The one-GB cap, three interior samples, and both wave modes are unchanged. The public `rat query` command reports 714,096 configurations; an independent integer calculation reproduces that count exactly.

## 2026-08-30: whole-row deadline

The first configuration after the LDS correction used `M=15625001 N=2 K=2`, `tpwgx=256`, and `m=rm=2048`. It fits LDS, but its LLVM compilation was still running after three minutes. The existing five-second limit applied only after GPU dispatch. The exact source is `rat-evidence/shared-bounds-first-row.ll`. The manually interrupted attempt is `rat-evidence/shared-bounds-interrupted.csv`; its termination message is not a compiler-resource rejection.

The configured row limit now starts before load setup and travels through compilation, dispatch, and output verification. The compiler runs in its own [Unix process group](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.process_group). On expiry, Recipe terminates that group and reaps the compiler before recording `inf`. The [group-directed signal](https://man7.org/linux/man-pages/man2/kill.2.html) does not target Recipe or its parent services. Timeout messages name the phase that exceeded the deadline. Finite CSV times still measure GPU execution.

The unchanged public full-sweep command advanced through 14 configurations in about 70 seconds. All 14 timed out during compilation and are archived in `rat-evidence/whole-row-compiler-timeouts.csv`. Process inspection confirmed no compiler from those attempts remained. Collection was then interrupted to test the GPU deadline through the public Recipe source interface:

```sh
RECIPE_DEBUG=1 ./target/debug/recipe --device amd0 experiment/rat-timeout.rs '32,1,1,1,1,1,3,1,1,1,1,1,1,1,1,1,32,1,1,1,1,1,1,1,1,1,1,1,1;100000;24'
```

In that same process, the `107x1` by `1x100000` load timed out during GPU execution. The following `107x1` by `1x45` load completed in 2.359301 ms and all 4,815 downloaded outputs matched the independently calculated products. Both records follow the 14 compiler attempts in `rat-evidence/whole-row-deadline-replays.csv`. This verifies cancellation and continued execution; it does not complete the 714,096-row dataset or classify timeout configurations as invalid.

## 2026-08-30: register-vector lowering

A profile of the real public sweep attributed 70.13% of sampled cycles to LLVM's `SelectionDAGISel::SelectAllBasicBlocks`, including 17.23% in DAG legalization and 16.74% in DAG combining. The profile is archived losslessly as `rat-evidence/compiler-deadline-profile.data.zst`; its rows are `rat-evidence/compiler-profile-rows.csv`. These profiled attempts are diagnostic runs, not uninstrumented runtime measurements.

Removing the identity conversion from the large vectors did not resolve the deadline failure. The 18 attempts from that iteration are archived in `rat-evidence/identity-vector-replay.csv`. That change was superseded: the contraction now uses one scalar register-accumulation loop for both its local and chunk-partial paths. The loop retains the selected register dimensions and scalar fused multiply-add order. The obsolete wide-vector fragment loads, conversion loop, and vector arithmetic generator were removed. No search-space bound changed.

The unchanged full-sweep path then compiled the large-register kernels and reached 2,574 configurations before interruption. Every recorded row failed Recipe's 32-bit launch-size product, before dispatch. Those records are `rat-evidence/scalar-register-grid-overflow.csv`. This is a host representation failure, not an HSA rejection of the packet.

Public manual replays used `M=107 N=45 K=1`, `tpwgx=gx=256`, `tpwgy=tpwgz=gy=gz=1`, `m=64 n=k=1`, and `rm=2048 rn=1`. Wave32 completed in 41.952776 ms; wave64 completed in 54.618642 ms. All 4,815 downloaded outputs matched the independent products in each run. These manual register values exercise the compiler and GPU; they are not rows from the stepped load space.

The next boundary replay used `M=107 N=128 K=1`, wave64, `m=rm=64`, `n=rn=128`, and `tpwgx=gx=256`. LLVM now reports no register spills, but rejects a 131,080-byte private frame against its reported 131,068-byte limit. The two explicit private arrays account for 131,072 bytes before compiler overhead. The complete diagnostic and both successful replays are appended in `rat-evidence/scalar-register-replays.csv`. No private-stack bound has been added. The remaining work starts with that allocation failure, followed by the wide-grid representation failure; the full dataset and RAT-versus-minimal benchmark remain incomplete.

## 2026-08-30: single-chunk storage

The second private array held chunk partials even when the tile contained one chunk. The existing local accumulation path now handles a single chunk as well as a single K lane. The compiled allocation is zero when no selectable tile needs chunk exchange. The unused bias-partial reservation was also removed from the schedule. Shared-memory allocation and the coupled domains use the same condition.

The unchanged `M=107 N=128 K=1`, `m=rm=64`, `n=rn=128`, `tpwgx=gx=256` manual replay then completed under wave64 in 5.262926 ms and wave32 in 5.331887 ms. All 13,696 downloaded outputs matched independently calculated products in both cases. The records are `rat-evidence/single-chunk-replays.csv`.

Removing the unused exchange requirement restored 1,064,340 stepped configurations. The rerun recorded 3,391 failures before interruption, archived in `rat-evidence/single-chunk-register-overflow.csv`. Its first row selected `m=2048` and `rm=3906251`. LLVM reported a 31,250,016-byte frame, with no register spills, against its 262,136-byte wave32 limit. The main accumulator still allocated every selected register position, including positions beyond the tile. This is a separate storage-layout failure; no private-stack bound has been added.

## 2026-08-30: compact register storage and wide dispatch indexing

The scalar accumulator now stores only positions that its selectable tiles can contain. The selected `rm` and `rn` still determine ownership. Physical row stride, column capacity, initialization, accumulation, and output addressing use the compact storage dimensions. Chunk-partial allocation uses those same dimensions. The matrix-instruction path retains its existing register layout.

Public manual replays completed with `rm=3906251`: `M=107 N=45 K=1`, `m=64 n=1 rn=1`, wave32, in 1.559622 ms; and `M=107 N=128 K=1`, `m=64 n=4 rn=128`, wave64, in 4.272490 ms. All 4,815 and 13,696 outputs, respectively, matched the independent products. These records are in `rat-evidence/compact-register-replays.csv`. Their register values exceed the stepped load domains. The current manual CSV exporter incorrectly labels them `inside-ss`; the downloaded-output comparisons do not establish search-space membership.

The full rerun then compiled and recorded 2,613 host grid-product failures before interruption, archived in `rat-evidence/compact-register-grid-overflow.csv`. Grid volume, flattened thread and workgroup IDs, and parallel loop induction now use 64 bits. Data indices are narrowed separately, and the loop termination comparisons retain the full dispatch index. The kernel argument layouts and CPU function signatures use the same 64-bit thread count. Workgroup traversal obtains the actual product of per-axis group counts, not a truncated grid-volume division. The duplicate CPU epoch argument lists were replaced with one shared invocation.

A real wave32 dispatch with `tpwgx=1024`, `gx=16777216`, `gy=256`, and `gz=1` launched 4,294,967,296 threads. It completed in 54.289506 ms, with all 4,815 outputs matching. The record is in `rat-evidence/wide-grid-replays.csv`.

The next full rerun recorded 1,945 failures while initializing unused OCKL state, archived in `rat-evidence/wide-grid-unused-sync.csv`. The compiled forward kernel reports a 48-byte KERNARG segment containing only its explicit arguments. Recipe now initializes the implicit synchronization state only when the compiled KERNARG segment includes implicit arguments. This follows the [LLVM code-object metadata contract](https://llvm.org/docs/AMDGPUUsage.html#amdgpu-amdhsa-code-object-kernel-argument-metadata-map-table-v5); it does not remove synchronization from a kernel that uses it.

After that correction, a dispatch with 4,294,967,296 one-thread workgroups completed in 1,655.357810 ms and all 4,815 outputs matched. A manual replay with `gx=536871103`, `gy=gz=16384`, and `tpwgx=256` reached GPU execution and returned `inf` at the five-second row deadline. Both records are in `rat-evidence/unused-sync-and-timeout-replays.csv`.

## 2026-08-30: first total-memory failure

The next full rerun reached the first real load, `M=15625001 N=2 K=2`, with `m=2048 n=k=1`, `rm=3906251 rn=1`, and wave32. Its matrix allocation formula gives 500,000,080 bytes. Live KFD accounting at `/sys/class/kfd/kfd/proc/1557458/vram_30381` reported **1,610,571,776 bytes**. The observer sent SIGINT to that runtime PID immediately. Its active dispatch reached the existing deadline, and the process exited. The row is archived in `rat-evidence/one-gb-scratch-overrun.csv`.

The configured one-GB cap currently covers matrix buffers, not total VRAM. The compiled first-row kernel also requires a 16,392-byte private frame per thread. KFD reports 54 CUs and 32 scratch slots per CU. ROCr sizes scratch from the compiled private-frame size, wave size, and device or dispatch scratch slots, with alignment and allocation-class handling. The installed ROCr 7.2.4 implementation is [AqlQueue::HandleInsufficientScratch](https://github.com/ROCm/rocm-systems/blob/rocm-7.2.4/projects/rocr-runtime/runtime/hsa-runtime/core/runtime/amd_aql_queue.cpp#L873-L1076) and [GpuAgent::AcquireQueueMainScratch](https://github.com/ROCm/rocm-systems/blob/rocm-7.2.4/projects/rocr-runtime/runtime/hsa-runtime/core/runtime/amd_gpu_agent.cpp#L1831-L2007).

The [ROCr scratch-limit settings](https://rocm.docs.amd.com/en/latest/reference/environment-variables/index.html) control reclamation thresholds, not a hard total-memory quota. No such setting was changed. The full sweep is stopped until its budget includes actual runtime and scratch allocations. No complete new dataset, trained RAT models, performance comparison, commit, push, or PR is claimed.

## 2026-08-30: total one-GB VRAM limit

Linux 7.1 exposes AMDGPU VRAM through the [`dmem` cgroup controller](https://docs.kernel.org/admin-guide/cgroup-v2.html#dmem). The rerun uses `dmem.max` with `drm/0000:03:00.0/vram 1000000000`, so the kernel charges each TTM VRAM allocation before AMDGPU accepts it. The same cgroup's `dmem.current` counter observed 678,797,312 bytes while the first three rows ran.

The first capped cooperative dispatch exited with status 134 when ROCr could not allocate scratch. ROCr 7.2.4 [returns its shared cooperative queue without installing the caller's event callback](https://github.com/ROCm/rocm-systems/blob/rocm-7.2.4/projects/rocr-runtime/runtime/hsa-runtime/core/runtime/amd_gpu_agent.cpp#L1720-L1733). Recipe now selects the queue from the compiled KERNARG contract. A kernel with implicit grid-synchronization arguments uses the cooperative queue. This RAT forward kernel has no implicit arguments, so it uses a normal queue with Recipe's HSA error callback. Timeout and asynchronous-error recovery share the same queue replacement operation.

The exact public sweep then advanced without terminating. At the 38-row snapshot, 34 rows reached their GPU execution deadline and four reached their output-verification deadline. KFD reported 905,134,080 bytes for process 1634132 while `dmem.current` reported 905,760,768 bytes for the complete service. Both independently measured values remained below the configured one-GB limit. The CSV prefix is archived as `rat-evidence/one-gb-dmem-prefix.csv`.

## 2026-08-30: deterministic collection resume

At its observed rate, the 1,064,340-row sweep requires about 62 days. A restart previously truncated the CSV and restarted enumeration at row one. Collection now reads every existing row, reconstructs its queried load and knobs, and compares it with the next deterministic search-space configuration. It stops on a schema, value, ordering, duplicate, or cardinality mismatch. When the prefix matches, the enumerator and valid-row count continue after that prefix and the CSV opens for append.

The real capped process stopped at 112 rows with CSV SHA-256 `5958cfd5739e46985b145b733fe352e1cfcc04412bd0d3bbfbf7f992b92b8118`. The rebuilt public command reported `collect 112/1064340 configurations, 0 valid`, retained the previous final row, and appended a distinct 113th configuration. The resumed process remained under `dmem.max`. A 122-row post-resume snapshot is `rat-evidence/one-gb-dmem-resume.csv`, SHA-256 `1ec97f395dee194eacd71f38cde6527e676246f6095f05d1285be4293eb27b0f`.

An earlier capped run used `Restart=on-failure`. A scoped `SIGKILL` terminated only Recipe at row 139. Systemd replaced PID 1669600 with PID 1670821 and reported `NRestarts=1`. The replacement process reported `collect 139/1064340 configurations, 0 valid` and appended row 140 without duplicating row 139. The service still reported the same 1,000,000,000-byte `dmem.max`. A 143-row snapshot after that supervised restart is `rat-evidence/one-gb-supervised-resume.csv`, SHA-256 `d99aa82b62374598023453181f77fb2c3f7c05bac832176eec7fb3c4ca2c3e3a`.

Automatic restarts are now disabled by `/run/systemd/system/recipe-rat-vram.service.d/no-restart.conf` with `Restart=no`, following the user's no-retry instruction. Reloading the setting kept PID 1698869 and its start time unchanged. The process remained active with the same 1,000,000,000-byte `dmem.max`.

Row 190 required more scratch than the one-GB policy allowed. HSA returned `0x1008` after Recipe had already replaced the failed queue. Treating that recovered row error as process-fatal made supervision restart the process seven times at the same deterministic row. Recipe now records it as `knob.inside-ss.fail.invalid` with the HSA status as evidence and continues enumeration.

The first continuation reused GPU buffers after the queue error. Row 191 then downloaded 31,250,002 zeros instead of the expected products. Rebuilding after the queue error alone was insufficient: a later GPU-execution timeout replaced the queue, and row 193 showed the same zero-output corruption. The complete failed prefixes are `rat-evidence/one-gb-queue-state-failure.csv`, SHA-256 `9015ca4830e60d25c92244ac49e3edeb29f9851ce5d6fa02bc2a2b722cc9e447`, and `rat-evidence/one-gb-timeout-state-failure.csv`, SHA-256 `c91bfbfb3ac26c2bfe75beb2ad9b7423ef3fa148d571777172c8a6a795ee1914`.

Collection now discards the current `RatMatrix` after either an HSA queue error or a GPU-execution timeout. It recreates the real graph, buffers, and compiled program before measuring the next configuration on the already-replaced queue. The contaminated suffixes were retained in the evidence files and removed from the active CSV. Replaying from the clean 189-row prefix advanced through row 207 with 11 recorded `0x1008` failures, 196 timeouts, and no incorrect outputs. The replay snapshot is `rat-evidence/one-gb-queue-state-replay.csv`, SHA-256 `9c8f6aa24dc143fab7feef600441f73829514a98ff9e675d88ced73eb30f40cb`.

## 2026-08-30: prediction domains and collection sampling

The shared domain constructor previously applied the collection step counts before prediction. `RatModels::choose` therefore selected only sampled values, and a numeric field with one sampling level was treated as a singleton even when its valid range contained more values.

The constraint constructor now returns full valid domains. The existing collection helper samples those domains for counting and enumeration, retaining every queried wave mode. Numeric range membership uses the bounds directly. No bound, configured sampling count, or sample-position formula changed. The production backend decreased by five lines.

`cargo build` and `cargo check` completed. The public `./target/debug/recipe --device amd0 rat query` command still returned 1,064,340 configurations. Collection was stopped after row 850; its complete prefix is `rat-evidence/full-domain-collection-prefix.csv` (144,920 bytes), SHA-256 `800706964134bf0dd5c981aa0cd0d190a44e6450d36a7d89d429bbc9c85f8ce8`.

The rebuilt public sweep validated that prefix and continued from row 851 under the unchanged one-GB quota, with automatic restarts disabled. At row 868, all 850 earlier rows remained byte-identical, and all 868 configurations were distinct. A 20-second observation sampled a peak of 679,116,800 VRAM bytes. This verifies collection continuity. Full-domain prediction through trained models remains untested because the configured saved model files are absent.

## 2026-08-30: kernel quota denials

The running public sweep remains in `recipe-rat-vram.service`, with CLI PID 1841405 and GPU client PID 1841436. Its `dmem.max` remains `drm/0000:03:00.0/vram 1000000000`, and automatic restarts remain disabled. Twenty one-second samples of `dmem.current` ranged from 490,004,480 to 993,361,920 bytes. This kernel does not expose `dmem.peak`, so the sampled maximum is not a lifetime peak.

A 20-second return probe on the real client's `dmem_cgroup_try_charge` calls recorded 92 negative returns, all `-11`. Requested allocations included 1,812,381,696 and 2,720,563,200 bytes. The [Linux 7.1 implementation](https://github.com/torvalds/linux/blob/v7.1/kernel/cgroup/dmem.c#L652-L695) returns `-EAGAIN` when its quota charge fails. These are allocation-denial events, not 92 failed rows. The capture is `rat-evidence/dmem-denials.data`, SHA-256 `11a0ca719d13153d08f0a8333c3f364ca2d99d0ed5b9d283084ca51f25cd34b5`; read it with `perf script -i .docs/rat-evidence/dmem-denials.data`. The probe was removed after collection.

At 2,280 completed rows, the CSV contained 999 HSA resource errors, 1,050 GPU-execution timeouts, 231 output-verification timeouts, and no valid timing. The archived 850-row prefix remained byte-identical. The cap is enforced and collection is advancing; successful execution of this sampled space is not established.

## 2026-08-30: immediate verification and scratch-count overflow

The verifier previously accumulated mismatches until it checked every output. A later row deadline could therefore replace an already-detected mismatch with an output-verification timeout. It now reports the first incorrect downloaded value immediately. Cancellation before native dispatch also has a distinct `RecipeError::Interrupted` value and propagates out of measurement instead of becoming a penalized configuration. These changes remove three net backend lines. `cargo build --locked`, `cargo check --locked`, and the whitespace check completed.

The pre-change collection stopped at 2,979 rows. Its archive is `rat-evidence/pre-immediate-verification.csv`, SHA-256 `97a30046a5251136122d709554f923b8c6aa9ffdbb1d5b9fa42434f87379444f`. The rebuilt public sweep preserved that prefix and exposed six zero-output failures among its next 47 rows. The first was row 2,987: output 0 was 0 instead of 2 for `M=15625001 N=2 K=2`. The complete 3,026-row file, including the old incorrectly penalized interruption, is `rat-evidence/immediate-verification-failure.csv`, SHA-256 `006aa5292a2729142f0a2556a2576e8cbdf08de424d80b2675d4a28e48c6f81c`.

For a fresh-process replay, the active CSV retained the first 2,986 rows from that archive. A new public `recipe --device amd0 rat` process, GPU client PID 2395245, reproduced the same zero output on its first configuration. This rules out a prior configuration's queue failure as a necessary trigger. The replay ran under the same one-GB quota and stopped with 3,046 unique rows. Its 60 new rows contain no penalized interruption. The complete file is `rat-evidence/fresh-process-zero-output.csv`, SHA-256 `7923b67ab17771b7603ff1363352a4f9afffbd6c4f0d985d0582d823da5134de`.

The installed packages report ROCr 7.2.4. Its [scratch-allocation code](https://github.com/ROCm/rocm-systems/blob/rocm-7.2.4/projects/rocr-runtime/runtime/hsa-runtime/core/runtime/amd_aql_queue.cpp#L1002-L1007) narrows the group count to `uint32_t`, multiplies the wave count in `uint32_t`, and only then applies the device-slot limit. Live KFD properties give 54 CUs and three shader engines. Applying the source calculation to row 2,987 gives 3,377,700,525,834,240 workgroups and 16 waves per workgroup. The narrowed scratch-slot product is zero. All six observed mismatches in the 47-row run have a zero wrapped slot count; one additional zero-slot row timed out.

The source overflow and fresh-process failure are established. A corrected-runtime replay is still needed to establish that the overflow causes the incorrect output. The full sweep is paused for that investigation. No new knob constraint or system-driver change has been made.

## 2026-08-30: controlled ROCr replay under the one-GB quota

The controlled replay identifies the narrowed scratch-count arithmetic as the cause of the zero-output result for row 2,987. Two private builds use the same ROCr 7.2.4 source commit, `97f5574fe2fdc7bef44fb01545347912ee9f1779`, compiler, build directory, Recipe binary, row, and `dmem.max` quota of 1,000,000,000 bytes. The only source difference is [rocr-scratch-count.patch](rat-evidence/rocr-scratch-count.patch): retain the group count and wave multiplication in 64 bits, then clamp to the existing device-slot limit before narrowing.

The patched runtime reported `AMD queue failed with HSA status 0x1008` for that row. All 153 new rows in its replay reported that resource error. Restoring the upstream arithmetic and rebuilding reproduced `output 0 is 0, expected 2` on the first replayed row. This does not establish a successful execution under the quota: the correction exposes resource exhaustion instead of returning an incorrect output. No search-space bound changed.

The patched replay is [private-wide-scratch.csv](rat-evidence/private-wide-scratch.csv), with 3,139 total rows, SHA-256 `565b39260181eef85ad1b650245548135558114638e1cacf553e00d52a447057`. The control is [private-narrow-scratch.csv](rat-evidence/private-narrow-scratch.csv), with 2,995 total rows, SHA-256 `e3ccdf8aa84a54e248f1c83fa1f8df90d05da485f0e894e14390d92cf2011742`. Each contains the unchanged 2,986-row prefix; only the following rows were measured in that replay. The patched GPU client was PID 2463142; the control GPU client was PID 2496697. Their process mappings identified the private library, and their live cgroup quota read `drm/0000:03:00.0/vram 1000000000`.

The patched library SHA-256 is `f24fdb6a14e70022f2e2606a5755543d2577089a094eaa9f64952ac0477b9bcb`; the control library SHA-256 is `96974031eb930826106036bc5407a0baf44b771cf5ee1ea0174c11d54d15b74b`. The private library resides at `/home/nate/.local/share/recipe/rat/rocr-overflow-build/rocr/lib/libhsa-runtime64.so.1.18.0`. Recipe selects it with the unit's `LD_LIBRARY_PATH`; `/opt/rocm` remains unchanged. The control CSV was moved to its archive so the corrected full sweep can start at row 1 without mixing earlier runtime results into the new dataset.

The restored patched library has the same SHA-256 as the first patched build, and the archived patch passes a reverse-application check against its source. The full 1,064,340-row sweep restarted at 22:20:43 PDT through `recipe --device amd0 rat`. Its unit is `recipe-rat-vram.service`, CLI PID 2506715, GPU client PID 2506778, with `Restart=no`. At 56 seconds, the live quota was 1,000,000,000 bytes and charged VRAM was 879,837,184 bytes. This is a sampled current allocation, not a lifetime peak. The first 11 saved rows each exceeded the five-second GPU deadline and have `time=inf`; none is a successful timing. The active output is the new repository-root `knob-configs.csv`. Model training and the VNA comparison have not run.

## 2026-08-31: packet diagnostics and search-space attempt labels

The existing dispatch diagnostic now records the submitted grid, workgroup dimensions, private bytes per work-item, shared bytes per workgroup, and cooperative mode in `recipe.log`. This replaces one log line without increasing backend LOC. A public resume retained all 136 completed rows, with prefix SHA-256 `864db17d447839655329fe2875830da6058795cab45da4acaf3fb864728da8ca`. The captured packet used `grid=[536871103,16384,32768]`, `workgroup=[256,1,1]`, `private=16392`, `shared=16392`, and `cooperative=false`. The first original row's smaller grid, `[536871103,16384,16384]`, already contains 144,115,239,347,027,968 work-items. These observations identify the launch geometry and queue mode; they do not establish a successful kernel execution.

The CSV writer incorrectly labeled initial collection as `knob.inside-ss`. Its existing callers now provide the origin: `config.ss` for initial collection and its storage estimate, and `knob.inside-ss` for explicit knob attempts and the surrogate loop. Outcome suffixes and every measurement field remain unchanged. `cargo build --locked` and `cargo check --locked` completed; backend LOC remains 18,850 across `recipe.rs`, `build.rs`, and `amd-nv-cpu.ll`.

The 5,347-row file before the label correction is archived as [initial-collection-origin-before.csv](rat-evidence/initial-collection-origin-before.csv), SHA-256 `c9f4c704d71590f0f2ef3b73322c0b8bcc9c21f103038f0370bef5aa1c11e3d7`. After changing only the attempt prefix, the active prefix has SHA-256 `3badd232535b1452e7758d2e91717d50298c1781b8c7c4ddd47083122e0715ae`. Excluding only that column gives the same SHA-256 before and after: `e9b0a4178dbcc5344551139b289f25018159309399d2d1afb21f6973ffb2b9ad`. No time, M/N/K, knob, validity, or error value changed.

The public capped sweep resumed at 01:40:55 PDT, CLI PID 3702933 and GPU client PID 3702978. At 5,363 rows, the corrected 5,347-row prefix remained byte-identical. Newly measured rows use `config.ss.fail.timeout`. Totals at that snapshot were 1,556 GPU-execution timeouts, 3,807 HSA resource errors, and zero successful timings. The live quota remained 1,000,000,000 bytes; sampled current VRAM was 993,206,272 bytes. All earlier results remain preserved. No domain, sampling level, timeout, or training phase changed.
