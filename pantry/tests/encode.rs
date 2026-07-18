use ndarray::{Array1, Array2};
use pantry::encode::{Dataset, clean_dataset};

fn ds(n: usize, cols: usize, k: usize, y: Vec<f64>) -> Dataset {
	return Dataset {
		x: Array2::zeros((n, cols)),
		y: Array1::from_vec(y),
		source: "t".into(),
		n_targets: k,
		has_target: k > 0,
		text_cols: Vec::new(),
		onehot_groups: Vec::new(),
	};
}

#[test]
fn no_target_survives_cleaning() -> anyhow::Result<()> {
	let mut d = ds(4, 3, 0, Vec::new());
	clean_dataset(&mut d)?;
	assert_eq!((d.x.nrows(), d.x.ncols()), (4, 3));
	assert!(d.y.is_empty());
	return Ok(());
}

#[test]
fn missing_target_drops_row_features_impute() -> anyhow::Result<()> {
	let mut d = ds(3, 2, 1, vec![1.0, f64::NAN, 3.0]);
	d.x[(0, 1)] = f64::NAN;
	clean_dataset(&mut d)?;
	assert_eq!(d.y.to_vec(), vec![1.0, 3.0]);
	assert!(d.x.iter().all(|v| return v.is_finite()));
	return Ok(());
}
