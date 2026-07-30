pub fn checkpoint_plumbing_is_not_public(report: &recipe::TrainingReport) {
	let _ = report.checkpoint_manifest();
}
