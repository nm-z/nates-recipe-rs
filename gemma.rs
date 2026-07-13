#!/usr/bin/env recipe
use recipe::*;

fn main() {
	if let Some(code) = recipe_infer::llm::vram_probe_ask() {
		std::process::exit(code);
	}
	let model = Model::load("gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf", recipe.model(), 0);
	recipe.infer().log([chat]).run(&model);
}
