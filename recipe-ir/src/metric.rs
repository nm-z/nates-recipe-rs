#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
	Loss,
	Accuracy,
	Epoch,
	Lr,
	Time,
	R2,
}

#[derive(Clone, Copy, PartialEq)]
pub enum LogItem {
	Metric(Metric),
	Device,
}
