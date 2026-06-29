// E2E for the argument-free run-arg forms (the `.run()` ergonomics). On the small
// house-prices set it exercises every form the spec defines:
//   (1) one model + one data in scope        → train.run(())
//   (2) model explicit, one data in scope     → train.run(model)
//   (3) both explicit (the loop form)         → train.run((model, data))
//   (4) two models in scope                    → train.run(()) panics (ambiguous)
// Each `{ }` scope drops its bindings so the live registry is empty between forms.
use recipe::*;

#[allow(unused_variables)] // models/data are resolved through the live registry
fn main() {
	{
		let data = Data::load()
			.set("datasets/house-prices/train.csv")
			.split(0.8)
			.exclude("Id")
			.target("SalePrice");
		let mlp = Model::new().loss(mse).layer(32).leak().layer(1).lr(0.0001);
		let train = Train::new().epochs(5).log([Loss, R2]);

		eprintln!("=== form .run(()): one model + one data in scope ===");
		train.run(());
		eprintln!("=== form .run(model): model explicit, data from scope ===");
		train.run(&mlp);
		eprintln!("=== form .run((model, data)): both explicit ===");
		train.run((&mlp, &data));
	}

	{
		let data = Data::load()
			.set("datasets/house-prices/train.csv")
			.split(0.8)
			.exclude("Id")
			.target("SalePrice");
		let a = Model::new().loss(mse).layer(8).leak().layer(1).lr(0.0001);
		let b = Model::new().loss(mse).layer(8).leak().layer(1).lr(0.0001);
		let train = Train::new().epochs(1);
		eprintln!("=== form .run(()) with 2 models in scope: expect ambiguity panic ===");
		let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| train.run(())));
		assert!(r.is_err(), "two models in scope must make .run(()) panic");
		eprintln!("correctly refused to guess (ambiguous)");
	}

	eprintln!("all run-arg forms OK");
}
