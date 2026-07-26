pub fn banned(report: &recipe::TrainingReport) {
	let _ = report.try_save("model.ogdl");
}
