// E2E for attention + RoPE on real text: cookbook LLM scenario (embed → attn →
// dense), few epochs, foreground. If the full-batch attention scores exceed VRAM
// the preflight aborts cleanly (a dataset-size limit, not a RoPE bug).
use recipe::*;

#[allow(unused_variables)] // llm/data resolved via the live registry, not textually read
fn main() {
	let llm = Model::new()
		.loss(ce)
		.layer(embed(16))
		.layer(attn(4))
		.layer(32)
		.leak()
		.layer(3)
		.lr(0.001);
	let data = Data::load()
		.set("datasets/llm-classification-finetuning/train.csv")
		.split(0.8)
		.exclude("id")
		.target(["winner_model_a", "winner_model_b", "winner_tie"]);
	let train = Train::new().epochs(3).log([Loss, Accuracy]);
	train.run(());
}
