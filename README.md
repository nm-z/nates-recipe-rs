# recipe

GPU/CPU ML training and inference in Rust.

```rust
let data = recipe.data("measurements/")
	.target(["temperature"])
	.norm(z_score)
	.split(0.8);

let model = recipe.model()
	.conv(16, 5).pool(64).gelu()
	.layer(32).gelu()
	.layer(1)
	.loss(mae);

recipe.train()
	.fp(32)
	.lr(0.0001)
	.stop(0.1)
	.epochs(100000)
	.save("model.ogdl")
	.run(&model, &data);

let prediction = recipe.infer("model.ogdl", &input);
```

## devices

Select the local or host-qualified device to train on with `--device`:

```text
recipe --device amd0 model.rs
recipe --device engi:amd0 model.rs
```

Recipe trains one device, so `--device` is given once.

`recipe --worker <device>` serves one local GPU over stdin and stdout. Recipe starts this transport entrypoint through SSH for a host-qualified selector. It is a protocol endpoint, not a model script invocation.

## RAT

AMD training tunes the supplied workload through two models. The knob model selects a configuration from the queried device's search space. The bench model learns from measured training time and supplies gradients to the knob model. Training retains the fastest measured configuration after the configured observation budget.

Run the normal entrypoint, such as `recipe --device amd0 model.rs`. There is no CSV collection phase. `Cargo.toml` defines the tuning policy and the paths for both saved models. New models bootstrap from real workload observations; initialized models continue online. Decisions and measurements go to `recipe.log`. Set `RECIPE_DEBUG=1` for additional diagnostics in that same file.

The [RAT evidence](.docs/RAT-evidence.md) records the controlled VNA comparison and the pretrained state used. Performance depends on that learned state; new or subsequently updated models are not guaranteed to beat the untuned schedule.

## files

```bash
recipe.rs       runtime
amd-nv-cpu.ll   kernels
build.rs        compiler
cli.rs          cli options
test.rs         combo testing
```

## 18 thingys:
```rust
weights:
	layer(neurons)
	conv(filters, kernel)
	attn(heads)
	perc(width)
	rnn(hidden)
	gru(hidden)
	lstm(hidden)

blocks:
	moe(topk, [...])
	res([...])

feature reduction:
	pool(size)
	kmeans(clusters)
	knn(neighbors)

trees:
	forest(trees)
	cbst()
	xgbst()
	lgbm()

estimators:
	svm()
	bayes()
```
Feature generation is banned.

## 15 activations

```
relu  leak  sigmoid  tanh   selu   gelu   silu   elu
prelu cos   exp      log    ln     huber  tan
```

## compute precisions

```rust
.fp(8|16|32|64)
.int(1|4|8)
.bf(16)
.tf(32)
.f(exp, mantissa)
```

## 32 quantizations

```rust
quantized integer:
	.qi(4|5|8).(0|1)
	.qi(2|6|8).k
	.qi(3).k[.s|m|l]
	.qi(4|5).k[.s|m]
	.qi(4).nf
importance quantized:
	.iq(1).(s|m)
	.iq(2|3).(xxs|xs|s|m)
	.iq(4).(xs|nl)
```
##### **reporting:**

```rust
let report = recipe.train()
	.run(&model, &data);

report.initial_loss();
report.final_loss();
report.initial_predictions();
report.predictions();
report.r2();
report.tile();
report.epoch_seconds();
```
