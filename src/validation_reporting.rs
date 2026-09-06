use recipe_training::{ValidationMetricFamily, ValidationMetricStatus, ValidationUnavailableReason};

pub fn unavailable_validation_line(status: ValidationMetricStatus) -> Option<String> {
	let ValidationMetricStatus::Unavailable {
		family,
		reason,
		split_rows,
	} = status
	else {
		return None;
	};
	let family = match family {
		ValidationMetricFamily::Binary => "binary",
		ValidationMetricFamily::Multiclass => "multiclass",
		ValidationMetricFamily::Regression => "regression",
	};
	let (reason, known_rows) = match reason {
		ValidationUnavailableReason::NoKnownTargets => ("no-known-targets", 0),
		ValidationUnavailableReason::SingleKnownClass { known_rows } => ("single-known-class", known_rows.get()),
	};
	return Some(format!(
		"validation unavailable  family {family}  reason {reason}  known_rows {known_rows}  split_rows {split_rows}"
	));
}
