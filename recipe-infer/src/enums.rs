use recipe_ir::Metric;

pub struct MetricFmt {
	pub label: &'static str,
	pub width: usize,
}

pub fn metric_fmt(m: Metric) -> MetricFmt {
	match m {
		Metric::Epoch => MetricFmt {
			label: "epoch",
			width: 5,
		},
		Metric::Lr => MetricFmt {
			label: "lr",
			width: 7,
		},
		Metric::Time => MetricFmt {
			label: "time",
			width: 9,
		},
		Metric::Loss => MetricFmt {
			label: "loss",
			width: 7,
		},
		Metric::Accuracy => MetricFmt {
			label: "acc",
			width: 6,
		},
		Metric::R2 => MetricFmt {
			label: "r2",
			width: 8,
		},
	}
}

pub fn metric_pinned_slot(m: Metric) -> Option<usize> {
	return match m {
		Metric::Loss => Some(0),
		Metric::Accuracy => Some(1),
		Metric::R2 => Some(2),
		Metric::Epoch | Metric::Lr | Metric::Time => None,
	};
}

pub fn metric_render(m: Metric, v: f64) -> String {
	let w = metric_fmt(m).width;
	v.partial_cmp(&v)
		.map_or(format!("{:>w$}", "N/A"), |_finite| match m {
			Metric::Epoch => format!("{:>w$}", v as usize),
			Metric::Time => format!("{:>w$}", fmt_time(v)),
			Metric::Loss | Metric::Accuracy | Metric::Lr | Metric::R2 => {
				format!("{v:>w$.4}")
			}
		})
}

pub fn fmt_time(secs: f64) -> String {
	let s = secs as u64;
	let h = s / 3600;
	let m = (s % 3600) / 60;
	let sec = s % 60;
	match h.checked_sub(1) {
		Some(_hh) => format!("{h}h {m:02}m {sec:02}s"),
		None => match m.checked_sub(1) {
			Some(_mm) => format!("{m}m {sec:02}s"),
			None => format!("{secs:.1}s"),
		},
	}
}
