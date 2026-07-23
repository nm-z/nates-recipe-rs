# Recipe legacy-to-next migration map

Status values are `legacy`, `adapting`, `next`, and `retired`. A legacy module
remains buildable only in the mutually exclusive `legacy` artifact. It may
serve as behavioral evidence, but it is never linked into a `next` artifact.

| Existing surface | Next owner | Initial status | Rule |
|---|---|---|---|
| `src/lib.rs`, `src/api/*` | clean root facade | adapting | facade selection happens before data, model, train, or infer construction |
| `recipe-ir` semantic declarations | `recipe-core` adapters | legacy | preserve user intent, then lower into typed scalar and scheduled IR |
| `recipe-runtime/src/plan.rs` | `recipe-scheduler` | legacy | port formulas only after typed FLOP/byte review |
| `recipe-runtime/src/machine.rs` | `recipe-probe` plus `recipe-core` profiles | legacy | port useful benchmark methods without HIP, permissive zero defaults, or manual device/rate configuration |
| `recipe-runtime/src/compile/*` | artifact build pipeline | legacy | generated HIP, HIPRTC, and COMGR-HIP paths do not migrate |
| `recipe-runtime/src/execute.rs` | `recipe-executor` | legacy | replace dynamic compilation, placement, allocation, and synchronization |
| `recipe-runtime` transport/spill | remote executor and explicit transfers | legacy | spill is not remote calculation execution |
| `gpu-core/src/asm.rs` | `recipe-hsa` | legacy | port reviewed ABI/AQL concepts, not globals or blocking lifecycle |
| `gpu-core/src/hip.rs` | `recipe-hsa` and `recipe-cuda` | legacy | no HIP compatibility layer in next |
| `gpu-core/src/memory.rs` | executor arenas and backend allocations | legacy | exact decimal reservation and static arena replace admission headroom |
| `gpu-core/src/kernels.rs` and operation modules | owned kernel templates/catalog | legacy | migrate every entry in `operation-surface.txt` to f32/int32 primitives |
| `gpu-core/src/kernels/*.hip` | Zig/LLVM/ISA artifact sources | legacy | no source translation or HIP headers in next |
| `gpu-core/build.rs` | artifact builder and prohibition audit | legacy | no hipcc, CUDA Runtime, or vendor-math links |
| `recipe-infer` parsing/model formats | dependency-clean format crates | legacy | extract pure parsing; numerical transforms become GPU calculations |
| `recipe-infer` tokenization/chat | `recipe-text` | adapting | bounded pre-init host metadata transformation; token scoring remains a GPU calculation |
| `recipe-infer` execution/KV/sampling | next operation catalog/executor | legacy | f32/int32 payloads and static resource bounds |
| `pantry` parsing and framing | `recipe-ingest` plus GPU preprocessing operations | adapting | split bounded byte framing and decimal representation from GPU-backed detection and preprocessing |
| `catboost-rs` | next tree/boosting operation batch | legacy | owned histogram/tree primitives |
| `xgboost-rs-broken` | next tree/boosting operation batch | legacy | no compatibility promise beyond accepted shipped behavior |
| `lightgbm-rs` | next tree/boosting operation batch | legacy | owned histogram/tree primitives |
| existing tests | legacy oracle plus independent next tests | adapting | tests are not relabeled; next conformance is package-scoped |

## Cutover invariant

A row moves to `next` only when its accepted public behavior is reachable from
the clean facade, its calculation payload is f32/int32, its operations are in
the owned catalog, and its normal dependency/artifact audits contain no legacy
runtime, HIP, CUDA Runtime, or prohibited vendor operation library.

The legacy default is removed only after every accepted entry in
`operation-surface.txt` has a next owner or an explicit compatibility decision.
