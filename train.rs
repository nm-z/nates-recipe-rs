#!/usr/bin/env -S cargo +nightly -Zscript -v
---
[package]
edition = "2024"

[dependencies]
nates-recipe = { path = "/home/nate/nates-recipe-rs" }
---
use nates_recipe::*;

fn main() {
    let data = Data::load()
    .set("/home/nate/Desktop/march-machine-learning-mania-2026/")
    .split(0.5)
    .target("Pred");
    
    let model = Model::new()
    .loss(mse)
    .layer(6)
    .layer(2)
    .layer(1)
    .lr(0.0000000000000000001);
    
    let train = Train::new()
    .epochs(10)
    .log(&[Loss, Accuracy, R2, Lr])
    //        .resume("~/Desktop/model.ogdl")
    .save(&[w, b], "~/Desktop/model.ogdl");
    
    train.run(&model, &data);
    
}











