---
name: project_run_argfree
description: "Train::run is argument-free — .run(())/.run(model)/.run((model,data)) resolve model+data from a live registry"
metadata: 
  node_type: memory
  type: project
  originSessionId: 7f101d10-b1f0-41a2-a4ca-9033a1ae79e8
---

SHIPPED (f702cc9). `Train::run` takes no model/data args when one of each is in scope.
Rust can't overload one method across 0/1/2 args, so the forms collapse onto a single
`RunArgs` argument (same `()`-means-no-args convention as the existing `SavePath` for
`save`/`resume`):

- `train.run(())` — the one live Model + one live Data
- `train.run(model)` — model explicit, the one live Data
- `train.run((model, data))` — both explicit (the only form usable when several are in scope, e.g. a loop)

**Why boxing was required:** the builder moves the struct by value at every step
(`Model::new().layer()…`), so a raw `*const` captured at construction dangles. Fix:
`Model { inner: Box<ModelInner> }` / `Data { inner: Box<DataInner> }` — the heap inner
is address-stable across the outer's moves. Deref/DerefMut keep `self.field`/method
access transparent (field access auto-derefs through user Deref). A thread-local registry
holds `*const ModelInner` / `*const dyn RunData` into that box; register at `new()`/`load()`,
deregister on `Drop` ("in scope" ≈ alive). 0 or >1 live → clear panic to pass it explicitly.
`run` resolves `&ModelInner` + `&dyn RunData`; the run-path methods moved `impl Model`→`impl
ModelInner` (train.rs) but stay callable on `Model` via Deref. `RunData` is impl'd on
`DataInner` (registry) AND delegated on `Data` (explicit `(&Model,&Data)`). See [[project_data_lazy]].

**Wart:** `.run(())` makes the `model`/`data` bindings look textually unused (the registry
reads them via raw pointer + Drop, invisible to the compiler) → `unused_variables` warning
in user code. Examples use `#[allow(unused_variables)]`. Inherent to the design, accepted.

Verified all four behaviors on GPU via `examples/e2e_runforms.rs`. The e2e_* examples now
use `.run(())`; cookbook/train_detector use the explicit tuple.
