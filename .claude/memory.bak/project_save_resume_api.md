---
name: project_save_resume_api
description: "Train save/resume API collapsed to two methods — save(path)/resume(path) via SavePath trait, default \"model.ogdl\". save_as/resume_from/SaveItem/w/b consts REMOVED."
metadata: 
  node_type: memory
  type: project
  originSessionId: c896e4fe-80e3-430b-84b6-81eb86fb789e
---

**2026-06-29 (working tree, uncommitted).** Train's checkpoint API is now two methods, each taking an optional path via the `SavePath` trait (model.rs):
- `.save(())` / `.resume(())` → default `"model.ogdl"` in cwd.
- `.save("custom.ogdl")` / `.resume("custom.ogdl")` → explicit path (accepts `&str` or `String`).

`impl SavePath for ()`/`&str`/`String`; `or_default()` maps `()`→"model.ogdl". Internals unchanged: `save(p)` → `save_ogdl(None, &p.or_default())` (always writes ALL params, best-only guard); `resume(p)` → `self.resume = Some(p.or_default())`. Behavior is identical to the old methods for any given path — pure call-signature refactor.

**REMOVED:** `save_as(items, path)` (incl. the prediction-CSV-column save feature and the param-subset filter), `resume_from(path)`, the `SaveItem` enum + its `From` impls, and the crate-root `w`/`b` consts + `Param` re-export (they were only `save_as` arguments). `Param` stays internal in model.rs as the `save_ogdl` filter type. Callers updated: train_detector `.resume_from(x)`→`.resume(x)`, `.save_as([w,b],x)`→`.save(x)`; lib.rs doc + re-export.

**Arity caveat (the one deviation from the literal spec):** the user's spec wrote `.resume()`/`.save()` (empty parens) for the default. Rust has NO zero-arg/one-arg method overload, so `.resume()` (0 args) and `.resume("x")` (1 arg) cannot share a name. Default is therefore spelled `.resume(())`/`.save(())`, not `.resume()`. Trivially switchable to `Into<Option<&str>>` (`.resume(None)`) if that reads better. Verified: `cargo build --release --all-targets` clean. See [[project_data_lazy]] (Prepared/RunData lives in the same Train API).
