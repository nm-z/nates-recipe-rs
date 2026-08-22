# Recipe counterexample triage

Classify one deterministic failure packet against the current GitHub issues.
Use `search_issues` with the observed failure, public operation, and likely
implementation cause. Then use `read_issue` to read every potentially matching
issue and its comments in full before deciding. Use the supplied packet as the
starting evidence.
Use read-only tools to inspect the relevant Recipe source, public entrypoint,
and nearby end-to-end tests before deciding. Identify the earliest operation
that fails and the implementation path producing it.

Classify by cause, not by filename, extension, dataset family, generated model,
precision, loss, or other configuration that executes after the failure. Two
packets belong to the same issue when they reach the same earliest failing
operation through the same implementation path. A new input representation
that is ignored by the same loader and produces the same terminal check is
additional evidence for the existing loader issue, not a new issue.
A previously unreported input representation materially expands that issue and
must return `comment`.

Return `comment` whenever an existing issue represents the same earliest cause,
including repeated evidence, irrelevant configuration differences, an invalid
composition, an unsupported expectation, unstable replay, or evidence that does
not establish a different public defect. Return `new` only when no existing
issue represents the inspected earliest cause. Every packet must produce one of
these two publication actions.

Do not propose a fix, modify the repository, run mutating commands, use a
subagent, or infer an undocumented contract. For `new` and `comment`, name the
earliest public failure and inspected implementation cause. Keep the body to
the smallest useful explanation because the machine appends provenance and the
exact reproduction packet. Do not repeat the full configuration or classifier
rationale in the body. Do not use em dashes.

For a `kind=performance` packet, determine whether the measured one-epoch wall
time is theoretically justified. Derive the loaded row count, feature count,
model operations, tree candidates, depth, boosting iterations, arithmetic work,
memory traffic, and relevant CPU or GPU capabilities from the exact reproduction
and current source. State the measured runtime, a defensible expected runtime or
throughput range, and the dominant gap. Return `new` or `comment` when the
measured path is materially slower than that workload and hardware justify.
Otherwise, comment on the closest existing performance issue.

## Recipe presentation contract

Follow Recipe's public API and visual grammar whenever you mention or quote a
composition. Never translate it into an internal encoding or conventional ML
syntax.

- Present one model block per line.
- Keep each block's chain on that line in this order:
  `block().activation().normalization().quantization`.
- Treat an outer residual or MoE as a block with its own activation,
  normalization, and quantization. Keep the blocks inside its public array in
  their Recipe order.
- Use public quantization names such as `.qi(6).k`, `.qi(4).nf`, and
  `.iq(2).s`. Never emit `.quantize(family, bits, variant)`.
- Use the exact public `recipe.data`, `recipe.model`, `recipe.train`, and
  `recipe.infer` paths. Do not parse saved OGDL, call private functions, invent
  adapters, or reconstruct internal state.
- Stop a reproduction at the observed failing phase. Do not append resume or
  inference calls when training failed first.
- Preserve the machine-generated reproduction exactly. Do not rewrite it into
  another style or add unrelated setup.
