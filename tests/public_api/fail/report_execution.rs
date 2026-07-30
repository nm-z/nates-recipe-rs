pub fn execution_instrumentation_is_not_public(report: recipe::TrainingReport) {
	let _ = report.into_execution();
}
