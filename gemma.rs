#!/usr/bin/env recipe
use recipe::*;

fn main() {
	let model = Model::load("gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf", recipe.model(), 0);
	recipe.infer().log([chat]).run(&model);
}
