# Recipe replacement cutover map

`main` now contains only the replacement workspace. The old implementation is
retained on the `legacy` branch as behavioral evidence; it is neither a Cargo
feature nor a dependency of the root package.

Historical source paths remain in `operation-surface.txt` and the operation
registry as stable provenance keys. Those strings identify accepted behavior;
they do not import, compile, link, or dispatch the retired implementation.

| Retired surface | Replacement owner on `main` | Cutover state |
|---|---|---|
| root `src/api/*` and CLI | `src/api.rs`, `src/cli.rs`, `src/facade.rs`, and `src/training.rs` | replacement-only facade with native dense-binary training and measured probing |
| `recipe-ir` declarations | `recipe-core` and `recipe-language` | retired from the workspace |
| runtime planning and machine models | `recipe-planner`, `recipe-scheduler`, `recipe-probe`, and `recipe-cluster` | replaced with measured, typed inputs |
| runtime compilation | `recipe-kernel`, `recipe-primitives`, `recipe-cuda`, and `recipe-hsa` | HIP/HIPRTC paths retired |
| runtime execution and memory | `recipe-prepare`, `recipe-executor`, `recipe-native-executor`, and `recipe-host` | fixed-point preparation and typestate runtime are replacement-owned |
| runtime transport and spill | `recipe-transport` and `recipe-remote` | explicit bounded transport and worker execution |
| `gpu-core` native interfaces | `recipe-hsa` and `recipe-cuda` | HIP compatibility layer retired |
| `gpu-core` kernels and operation modules | `recipe-ops`, `recipe-language`, `recipe-primitives`, and `recipe-kernel` | every normative entry is classified by an owned lowering or lifecycle recipe |
| model parsing | `recipe-ingest` | bounded dependency-clean framing |
| tokenization and chat rendering | `recipe-text` | bounded pre-init host transformation |
| training, inference, tree, and boosting behavior | the operation registry plus planner/executor pipeline | dense-binary training is end-to-end; remaining workflow families retain owned declarations and operation contracts |
| legacy tests and examples | public real-data workloads and hardware acceptance commands | retired tests remain on `legacy`; only end-to-end native acceptance evidence is authoritative on `main` |

## Main-branch invariant

The `recipe` package and its normal dependency closure may contain no retired
crate, HIP implementation, CUDA Runtime integration, or prohibited AMD/NVIDIA
operation library. Source strings used by the audit policy, behavioral
provenance, and negative fixtures are non-executable evidence and must remain
distinguishable from a dependency, linker input, generated call, or binary
symbol.
