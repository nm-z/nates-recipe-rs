# TODO

Surfaced by the 2026-07-08 spec-vs-code audit — **37 / 79 behavioral requirement
lines done** (19 partial, 23 absent; the other 151 spec lines are headers or
hardware values). Each item is anchored to ground truth. "Target" = design goal,
not a quick fix.

## Decision needed (blocked on you)

- [ ] **OGDL file format.** The `ogdl` crate parses `key=value` only
  (`ogdl/src/lib.rs:39`); real OGDL and `config.ogdl` use whitespace/indentation
  `name value`, so `ogdl::Node` can't read `config.ogdl` at all. Unifying lets one
  crate serve both config + model files, but switching off `=` breaks `model.ogdl`
  (772 KB of `key=value` weights). Pick: conform crate to ogdl.org (migrate
  model.ogdl), standardize config on `=`, or keep the two parsers.

## OGDL crate — spec fidelity

- [ ] **Port the path expression language** (ogdl.org/spec/path) into `get()`
  (`ogdl/src/lib.rs:60`), which today only does `.split('.')` + first-match-by-name.
  Add: `[n]` positional index, `{n}` n-1'th same-name selector, `{}` all same-name.
  Tests against the spec examples: `a.b`, `a.1`, `a.b{1}`, `a.b{}`, `a[1]`.

## config.ogdl — per-machine ordering

- [ ] **Sort peers alphabetically** in `rewrite_config` (`wire.rs:321-326`).
  Rule: self first (already done, `wire.rs:318-320`), then every other machine A→Z.
  Peers currently come from a `HashMap` (`wire.rs:233`) in arbitrary order — add
  `sort_by(|a, b| a.host.cmp(&b.host))` before push. Pairs with `[n]` indexing so
  position = identity (`machines[0]` = self). Rule per host:
  engi → `engi, archy, sentry`; sentry → `sentry, archy, engi`; archy → `archy, engi, sentry`.

## Cleanups

- [ ] **Drop the dead `_config_ogdl` arg and "converter" comment** on `service_unit`
  (`probe.rs:669`). Per ruling: `recipe install` *generates* both files; the unit is
  a fixed template (identical on every machine), NOT derived from the OGDL. Template
  is correct — only the ".ogdl→.service converter" framing is wrong.
- [ ] **Doc:** the spec's config block lists `engi, sentry, archy`, contradicting the
  self+alphabetical rule (`engi, archy, sentry` on engi).

## Target / design gaps (larger)

- [ ] **AOT scheduler + static computation DAG** — absent; system is definitively
  pre-AOT (no `Schedule`/`Dag`, `SAT_ENFORCE=false`). Cost model exists only as
  post-hoc roofline (size/bw, FLOPs/dev-FLOPs never consumed to schedule). This is
  the entire post-AOT table (0/22) and both `design:` blocks.
- [ ] **`parse_config` has no runtime consumer** (`probe.rs:522`) — `config.ogdl` is
  write-only; nothing reads it back to drive placement/scheduling. This is the bridge
  to the AOT scheduler above.
- [ ] **Master/Worker roles** — absent; nodes are peer-symmetric. No role type drives
  behavior; remote compute (RUN) is declared-only, only STORE/FETCH/FREE data
  transport is wired.
- [ ] **Duplex concurrency model** — absent. bidirectional (PCIe SDMA, NVMe, SAS, eth
  full-duplex) vs unidirectional (SATA, wlan half-duplex) is not represented in any
  cost/scheduling code.
- [ ] **`init alloc 0x`** — not met; init still does ≥2 device allocs (arena
  `hipMallocAsync` + host pin). The one-claim arena is the intended endpoint.
- [ ] **1 GB RAM reserve in the gemma waterfall** — `waterfall.rs:67` uses
  `mem_available()/10` (90% guard), not `USER_GB`. The 1 GB-per-tier law holds for
  VRAM/DISK and the fit path, but not gemma's RAM tier.

## Enforcement

- [ ] **No fp64 compile-time scanner.** The referenced `gpu-core/build.rs` fp64 scanner
  does not exist — fp64-only is convention in the live path; f32/f16/bf16 primitives
  remain declared + callable in gpu-core. (Root `build.rs` bans sync alloc;
  `gpu-core/build.rs` enforces mem chokepoints + no direct BLAS — neither checks
  precision.)
