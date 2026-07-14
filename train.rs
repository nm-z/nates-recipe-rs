#!/usr/bin/env recipe
use recipe::*;

// const GGUF: &str = "gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf";

fn main() {
	// recipe.model().load(&GGUF);
	// recipe.infer().log(chat).run(&model);

	set_opt(Opt {
		chat: true,
		..opt()
	});

	ogdl!(a.r"a	b	c");

	Write::block(chat, ogdl!(a));
}
