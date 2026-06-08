use nates_recipe::*;

fn main() {
      // 1. binary classification — telecom customer churn (Yes/No)
      let data = Data::load()
            .set("examples/playground-series-s6e3/train.csv")
            .split(0.8)
            .exclude("id")
            .target("Churn");
      let model = Model::new().loss(bce).layer(64).leak().layer(1).sigmoid().lr(0.001);
      let train = Train::new().epochs(5000).log([Loss, Accuracy]);
      train.run(&model, &data);

      // 2. multi-class classification — handwriting recognition (36 classes: 1-9, A-Z)
      // let data = Data::load()
      //       .set("examples/predict-the-handwriting-images/")
      //       .split(0.8)
      //       .target("label");
      // let model = Model::new().loss(ce).layer(128).leak().layer(64).leak().layer(36).lr(0.001);
      // let train = Train::new().epochs(5000).log([Loss, Accuracy]);
      // train.run(&model, &data);

      // 3. regression — house sale prices
      let data = Data::load()
            .set("examples/house-prices/train.csv")
            .split(0.8)
            .exclude("Id")
            .target("SalePrice");
      let model = Model::new().loss(mse).layer(128).leak().layer(64).leak().layer(1).lr(0.0001);
      let train = Train::new().epochs(10000).log([Loss, R2]);
      train.run(&model, &data);

      // 4. text classification — LLM arena judge (3-way: model_a wins, model_b wins, tie)
      let data = Data::load()
            .set("examples/llm-classification-finetuning/train.csv")
            .split(0.8)
            .exclude("id")
            .target(["winner_model_a", "winner_model_b", "winner_tie"]);
      let model = Model::new()
            .loss(ce)
            .layer(embed(16))
            .layer(attn(4))
            .layer(32).leak()
            .layer(3)
            .lr(0.001);
      let train = Train::new().epochs(5000).log([Loss, Accuracy]);
      train.run(&model, &data);

      // 5. image classification — handwriting digits + letters
      // let data = Data::load()
      //       .images("examples/predict-the-handwriting-images/train_images/")
      //       .split(0.8)
      //       .target("label");
      // let model = Model::new().loss(ce).layer(conv(32, 3)).layer(pool(2)).layer(conv(64, 3)).layer(128).leak().layer(36).lr(0.001);
      // let train = Train::new().epochs(100).log([Loss, Accuracy]);
      // train.run(&model, &data);

      // 6. time series — web traffic forecasting
      // let data = Data::load()
      //       .set("examples/web-traffic-time-series-forecasting/train_1.csv")
      //       .window(24)
      //       .split(0.8)
      //       .target("visits");
      // let model = Model::new().loss(mse).layer(gru(64)).layer(1).lr(0.001);
      // let train = Train::new().epochs(5000).log([Loss, R2]);
      // train.run(&model, &data);

      // 7. boosted trees — churn prediction (same data, tree model)
      // let data = Data::load()
      //       .set("examples/playground-series-s6e3/train.csv")
      //       .split(0.8)
      //       .exclude("id")
      //       .target("Churn");
      // let model = Model::new().loss(bce).trees(500).depth(6).lr(0.1);
      // let train = Train::new();
      // train.run(&model, &data);

      // 8. clustering — churn customers, unsupervised
      // let data = Data::load()
      //       .set("examples/playground-series-s6e3/train.csv")
      //       .exclude("id")
      //       .exclude("Churn");
      // let model = Model::new().kmeans(5);
      // let train = Train::new();
      // train.run(&model, &data);

      // 9. ensemble — NN + trees on churn
      // let data = Data::load()
      //       .set("examples/playground-series-s6e3/train.csv")
      //       .split(0.8)
      //       .exclude("id")
      //       .target("Churn");
      // let nn = Model::new().loss(bce).layer(64).leak().layer(1).sigmoid().lr(0.001);
      // let trees = Model::new().loss(bce).trees(500).depth(6).lr(0.1);
      // let model = Model::new().ensemble(&[nn, trees]);
      // let train = Train::new().epochs(1000);
      // train.run(&model, &data);

      // 10. competition submission — house prices with test set
      // let data = Data::load()
      //       .set("examples/house-prices/train.csv")
      //       .test("examples/house-prices/test.csv")
      //       .exclude("Id")
      //       .target("SalePrice");
      // let model = Model::new().loss(mse).layer(128).leak().layer(64).leak().layer(1).lr(0.0001);
      // let train = Train::new().epochs(10000).log([Loss, R2]);
      // train.run(&model, &data);
      // train.save([w, b], "model.ogdl");
      // train.run(&model, &data.test);
      // train.save(["Id", data.target], "submission.csv");
}
