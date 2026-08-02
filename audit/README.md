# recipe-audit

`recipe-audit` is the deterministic prohibition gate for Recipe's replacement
runtime. It rejects HIP, the CUDA Runtime API, direct KFD ownership, and the
listed AMD/NVIDIA operation libraries across source, build inputs, Cargo
dependency closure, LLVM declarations/calls, ELF `DT_NEEDED` entries, and
undefined artifact symbols.

The native interfaces intentionally accepted by policy are ROCr/HSA and an
exact reviewed CUDA Driver API symbol list. The auditor does not use substring
allowlists.

The CLI never chooses the current directory or runs `cargo metadata`
implicitly. A production invocation supplies an absolute scope and any facts
to inspect:

```text
recipe-audit --mode next --scope /absolute/next/source \
  --metadata /absolute/cargo-metadata.json --package-id 'recipe 0.1.1 (path+...)' \
  --link-input=-Wl,-Bstatic,-lowned \
  --elf /absolute/next/source/target/release/recipe
```

`legacy` mode additionally requires a JSON grant file. Each grant names one
exact category, normalized path, line, and symbol:

```json
[
  {
    "category": "source-api",
    "path": "src/legacy.rs",
    "line": 12,
    "symbol": "hipMalloc"
  }
]
```

Wildcards, blanket path exceptions, duplicate grants, and unused/stale grants
are rejected. Findings have stable `category`, `path`, `line`, `symbol`, and
`disposition` fields. Line zero denotes graph or binary evidence.
