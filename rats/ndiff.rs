use super::*;
fn distance(left: &[f64], right: &[f64]) -> f64 { left.iter().zip(right).map(|(a, b)| (a - b).powi(2)).sum() }
fn nearest(query: &[f64], state: &[f64], features: usize) -> (usize, f64) {
	state.chunks_exact(features).enumerate().map(|(i, row)| (i, distance(query, row))).min_by(|a, b| a.1.total_cmp(&b.1)).unwrap_or((0, f64::INFINITY))
}
fn kmeans(data: &Prepared, rows: usize, clusters: usize, iterations: usize) -> Result<Vec<f64>> {
	require(clusters != 0 && clusters <= rows, "kmeans cluster count is invalid")?; let mut centers = data.samples[..clusters * data.features].to_vec();
	let mut assigned = vec![0; rows];
	let mut distances = vec![0.0; rows];
	for _ in 0..iterations {
		for (row, sample) in data.samples[..rows * data.features].chunks_exact(data.features).enumerate() {
			let selected = nearest(sample, &centers, data.features);
			(assigned[row], distances[row]) = selected;
		}
		for cluster in 0..clusters {
			let members = assigned.iter().enumerate().filter(|(_, value)| **value == cluster).map(|(row, _)| row).collect::<Vec<_>>();
			if members.is_empty() {
				let worst = distances.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|v| v.0)
					.ok_or_else(|| RecipeError::new("kmeans has no training row"))?;
				centers[cluster * data.features..(cluster + 1) * data.features]
					.copy_from_slice(&data.samples[worst * data.features..(worst + 1) * data.features]);
				distances[worst] = -1.0;
			} else {
				for feature in 0..data.features {
					centers[cluster * data.features + feature] = members.iter()
						.map(|row| data.samples[row * data.features + feature]).sum::<f64>() / members.len() as f64;
				}
			}
		}
	}
	Ok(data.samples[rows * data.features..].chunks_exact(data.features)
		.map(|sample| nearest(sample, &centers, data.features).0 as f64).collect())
}
fn knn(data: &Prepared, rows: usize, count: usize, exclude_self: bool) -> Result<Vec<f64>> {
	let maximum = rows - usize::from(exclude_self); require(count != 0 && count <= maximum, "knn neighbor count is invalid")?;
	let training = &data.samples[..rows * data.features]; Ok(data.samples[rows * data.features..].chunks_exact(data.features).enumerate().map(|(row, query)| {
		let mut nearest = training.chunks_exact(data.features).enumerate()
			.filter(|(index, _)| !exclude_self || *index != row)
			.map(|(index, sample)| (distance(query, sample), data.targets[index])).collect::<Vec<_>>();
		nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
		nearest.iter().take(count).map(|value| value.1).sum::<f64>() / count as f64
	}).collect())
}
pub(super) fn estimator_predict(operation: &Operation, data: &Prepared, rows: usize, config: Config, exclude_self: bool) -> Result<Vec<f64>> {
	require(data.rows > rows, "estimator split must retain test rows")?; match operation {
		Operation::KMeans(clusters) => kmeans(data, rows, *clusters, config.kmeans_iterations),
		Operation::Knn(neighbors) => knn(data, rows, *neighbors, exclude_self),
		_ => Err(RecipeError::new("operation is not a supported estimator")),
	}
}
