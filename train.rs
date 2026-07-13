#!/usr/bin/env recipe
use recipe::*;

const GGUF: &str = "gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf";

fn main() {
	let model = recipe.model().load(GGUF);
	recipe.infer().log(chat).run(&model);
}
