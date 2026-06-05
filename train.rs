#!/usr/bin/env -S cargo +nightly -Zscript
---
[dependencies]
nates-recipe = { path = "/home/nate/nates-recipe-rs" }
---
use nates_recipe::*;

fn main() {
    let data = Data::load("german_credit.arff")
        .target("class")
        .split(0.8);

    let model = Model::new()
        .layer((128, relu))
        .layer(1)
        .loss(mae)
        .lr(0.001)
        .log(&[Loss, Accuracy, Epoch, Lr, Time, R2])
        .plot(&[Loss, Accuracy, Epoch, Lr, R2, Time]);

    model.train(&data.train, 500);
    model.eval(&data.test);
}
