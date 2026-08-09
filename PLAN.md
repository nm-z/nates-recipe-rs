# Platform Support Plan

| OS | CPU | AMD | NV |
|---|---|---|---|
| Linux | Yes | Yes | Yes |
| macOS | Yes | No | No |
| WinOS | Yes | No | Yes |

- Add no more than 20 net production LOC for this support.
- Implement the platform support in a separate commit.

## Precision and Quantization

Floating-point families:

| Prefix | Meaning |
|---|---|
| `fp` | floating point |
| `bf` | brain float |
| `nf` | normal float |
| `tf` | tensor float |

Quantized families:

| Prefix | Meaning |
|---|---|
| `qi` | quantized integer |
| `iq` | importance quantized |

Floating compute precision belongs in `recipe.train()`:

```text
.fp(8|16|32|64)
.bf(16)
.nf(4)
.tf(32)
```

Quantization belongs on individual `blck.atvn.norm` model chains:

```text
.qi(2|3|4|5|6|7|8).(0|1)|K.iff(S|M|L)
.iq(1|2|3|4).XXS|XS|S|M|NL
```

Quantization before the first block is the model-wide inherited default. Quantization after a block overrides only that block. A later block without an override continues to inherit the model-wide default.

```rust
let all = recipe.model().qi(4).k.m
	.layer(64).gelu()
	.layer(32).gelu()
	.layer(1)
	.loss(mae);

let mixed = recipe.model().qi(4).k
	.layer(64).gelu()
	.layer(32).gelu().qi(6).k.m
	.layer(1)
	.loss(mae);
```

Default dereference forms:

```text
.qi(n) -> Qn_0, no suffix needed
.iq(n) -> IQn, no suffix needed
```

Example:

```rust
let model = recipe.model()
	.conv(16, 5).pool(64).gelu().qi(6).k
	.layer(32).gelu().qi(4).k.m
	.layer(1)
	.loss(mae);

recipe.train()
	.fp(32)
	.epochs(1000)
	.lr(0.001)
	.save("model.ogdl")
	.run(&model, &data);
```

`fp` means floating point: the decimal remains and the selected precision is used for training computation. `qi` means quantized integer: the decimal is removed and the selected model weights are stored approximately in integer buckets. `iq` means importance quantization: importance is represented before quantization.

Model quantization controls how each selected model region is stored. Training floating-point precision controls how the GPU computes while training. Users may vary them independently, for example `.fp(32)` to train with decimals and `.qi(4)` to save selected weights without them.

Quantized saves contain the packed model rather than duplicate full-precision best weights and optimizer state. Resuming a quantized model reconstructs its approximate weights and starts a fresh optimizer at epoch zero.

Precision lowering must be trait-driven rather than a match arm for every format and suffix. Floating families provide their bit width and LLVM IR type. Integer families own compression, decompression, bit width, and format metadata. Format suffixes configure the integer family without creating a separate lowering path for every public spelling.

If a requested floating precision is unavailable for the active hardware and backend, return an error that lists the precisions available for that execution situation.
