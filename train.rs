use recipe::*;
fn number(text: &str, role: &str) -> Result<f64> {
	let value = text.parse::<f64>().map_err(|error| RecipeError(format!("invalid {role}: {error}")))?;
	(value.is_finite() && value > 0.0)
		.then_some(value)
		.ok_or_else(|| RecipeError(format!("{role} must be finite and positive")))
}
fn near(role: &str, actual: &[f64], expected: &[f64], tolerance: f64) -> Result<()> {
	if actual.len() != expected.len() {
		return Err(RecipeError(format!("{role} length mismatch")));
	}
	for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
		if !actual.is_finite() || (actual - expected).abs() > tolerance {
			return Err(RecipeError(format!(
				"{role}[{index}] is {actual}, expected {expected} within {tolerance}"
			)));
		}
	}
	Ok(())
}
fn verify(
	name: &str,
	model: Model,
	data: Data,
	expected_output: &[f64],
	expected_gradient: &[f64],
	backend: Backend,
	tolerances: [f64; 3],
) -> Result<TrainingReport> {
	let host_report = recipe.train(Backend::Host).run(&model, &data)?;
	near(name, &host_report.initial_predictions, expected_output, tolerances[0])?;
	near(name, &host_report.gradient, expected_gradient, tolerances[1])?;
	if backend != Backend::Host {
		let device_report = recipe.train(backend).run(&model, &data)?;
		near(name, &device_report.initial_predictions, &host_report.initial_predictions, tolerances[2])?;
		near(name, &device_report.gradient, &host_report.gradient, tolerances[2])?;
		near(name, &device_report.predictions, &host_report.predictions, tolerances[2])?;
	}
	Ok(host_report)
}
fn estimator_data() -> Data {
	recipe.data(([[0.0], [2.0], [10.0], [12.0]], [[0.0], [2.0], [10.0], [12.0]]))
}
fn estimator(name: &str, model: Model, expected: &[f64], tolerance: f64) -> Result<()> {
	let report = recipe.train(Backend::Host).run(&model, &estimator_data())?;
	near(name, &report.initial_predictions, expected, tolerance)
}
fn selected_backend() -> Result<Backend> {
	match std::env::args().nth(1).as_deref() {
		Some("host") => Ok(Backend::Host),
		Some("amd") => Ok(Backend::Amd),
		Some("nvidia") => Ok(Backend::Nvidia),
		_ => Err(RecipeError("usage: train host|amd|nvidia".into())),
	}
}
fn unsupported(backend: Backend) -> Result<()> {
	let expected = match backend {
		Backend::Amd => "Amd does not support standalone estimators",
		Backend::Nvidia => "Nvidia does not support standalone estimators",
		Backend::Host => return Err(RecipeError("host estimators are supported".into())),
	};
	for model in [recipe.model().kmeans(2), recipe.model().knn(2), recipe.model().forest(1)] {
		match recipe.train(backend).run(&model, &estimator_data()) {
			Err(error) if error.0 == expected => {}
			Err(error) => return Err(RecipeError(format!("unexpected unsupported error: {}", error.0))),
			Ok(_) => return Err(RecipeError(format!("{backend:?} estimator unexpectedly executed"))),
		}
	}
	Ok(())
}
fn main() -> Result<()> {
	let backend = selected_backend()?;
	let tolerances = [
		number(env!("RECIPE_OUTPUT_TOLERANCE"), "output tolerance")?,
		number(env!("RECIPE_GRADIENT_TOLERANCE"), "gradient tolerance")?,
		number(env!("RECIPE_BACKEND_TOLERANCE"), "backend tolerance")?,
	];
	let layer_report = verify(
		"layer",
		recipe.model().layer(1).parameters([3.0]),
		recipe.data(([[2.0]], [[0.0]])),
		&[6.0],
		&[24.0],
		backend,
		tolerances,
	)?;
	near("trained layer", &layer_report.predictions, &[5.52], tolerances[0])?;
	verify(
		"convolution",
		recipe.model().conv(1, 2).parameters([1.0, 2.0]),
		recipe.data(([[1.0, 2.0, 4.0]], [[0.0, 0.0]])),
		&[5.0, 10.0],
		&[25.0, 50.0],
		backend,
		tolerances,
	)?;
	verify(
		"pool",
		recipe.model().conv(1, 1).pool(2).parameters([2.0]),
		recipe.data(([[1.0, 2.0, 4.0]], [[0.0, 0.0]])),
		&[4.0, 8.0],
		&[40.0],
		backend,
		tolerances,
	)?;
	verify(
		"embedding",
		recipe.model().embed(1, 2).parameters([2.0, 3.0]),
		recipe.data(([[0.0, 1.0]], [[0.0, 0.0]])),
		&[2.0, 3.0],
		&[2.0, 3.0],
		backend,
		tolerances,
	)?;
	verify(
		"attention",
		recipe.model().attn(1).parameters([1.0, 1.0, 1.0]),
		recipe.data(([[1.0, 2.0]], [[0.0, 0.0]])),
		&[1.7310585786300048, 1.8807970779778822],
		&[0.7352900309653699, 0.7352900309653699, 6.533961451178673],
		backend,
		tolerances,
	)?;
	verify(
		"rnn",
		recipe.model().rnn(1).parameters([1.0, 1.0, 0.0]),
		recipe.data(([[1.0], [2.0]], [[0.0], [0.0]])),
		&[0.7615941559557649, 0.9920455700292888],
		&[0.35789089826972664, 0.011971913128950188, 0.34217135389648234],
		backend,
		tolerances,
	)?;
	verify(
		"gru",
		recipe.model().gru(1).parameters([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0]),
		recipe.data(([[1.0], [2.0]], [[0.0], [0.0]])),
		&[0.3807970779778824, 0.6780378586752849],
		&[
			-0.3401684261659197,
			-0.038373017214536106,
			-0.23939817493840138,
			0.0031520151543081285,
			0.0006001390802512698,
			0.0015760075771540643,
			0.18599993461909897,
			0.0031520151543081285,
			0.16944510614695818,
		],
		backend,
		tolerances,
	)?;
	verify(
		"lstm",
		recipe.model().lstm(1).parameters([0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0]),
		recipe.data(([[1.0], [2.0]], [[0.0], [0.0]])),
		&[0.18169974219452625, 0.29505136205146726],
		&[
			0.07123239423766844,
			0.004257945167883522,
			0.04779843051016353,
			0.018307601616424952,
			0.0016632432469522531,
			0.009153800808212476,
			0.10377963968051537,
			0.007908963351002269,
			0.060251986556302366,
			0.03164682519170416,
			0.00043387055062141423,
			0.029258981679208058,
		],
		backend,
		tolerances,
	)?;
	verify(
		"residual",
		recipe.model().residual([Residual::Layer(1), Residual::Relu]).parameters([-1.5]),
		recipe.data(([[2.0], [-2.0]], [[0.0], [0.0]])),
		&[2.0, 1.0],
		&[-2.0],
		backend,
		tolerances,
	)?;
	verify(
		"ReLU",
		recipe.model().layer(1).relu()?.parameters([1.0]),
		recipe.data(([[2.0], [-1.0]], [[0.0], [0.0]])),
		&[2.0, 0.0],
		&[4.0],
		backend,
		tolerances,
	)?;
	estimator("kmeans", recipe.model().kmeans(2), &[0.0, 0.0, 1.0, 1.0], tolerances[0])?;
	estimator("knn", recipe.model().knn(2), &[1.0, 1.0, 11.0, 11.0], tolerances[0])?;
	estimator("forest", recipe.model().forest(1), &[1.0, 1.0, 11.0, 11.0], tolerances[0])?;
	if backend != Backend::Host {
		unsupported(backend)?;
	}
	println!("{backend:?} public operations verified");
	Ok(())
}
