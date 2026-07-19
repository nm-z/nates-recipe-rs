#!/usr/bin/env recipe
use recipe::*;

fn main() {
    const HP: &str = "datasets/house-prices/train.csv";

    let d = Data::load(HP).split(0.8).exclude("Id").target("SalePrice");
    let m = Model::new().layer(64).relu().layer(1).loss(mse).lr(0.05);
    Train::new().epochs(20).log([Loss, R2]).run(&d, &m);
}
