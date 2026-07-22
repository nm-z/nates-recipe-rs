use recipe_infer::{fmt_time, metric_fmt, metric_render};
use recipe_ir::metric as li;
use recipe_ir::{LogItem, Metric};
use recipe_runtime::execute::metrics_line;

#[test]
fn loss_acc_line_bytes() {
	let line = metrics_line(&[Metric::Loss, Metric::Accuracy], &[0.7451, 0.5528]);
	assert_eq!(
		line.as_bytes(),
		"loss \u{1b}[38;2;242;40;60m 0.7451\u{1b}[0m  acc \u{1b}[38;2;39;125;255m0.5528\u{1b}[0m"
			.as_bytes()
	);
	return;
}

#[test]
fn seven_user_metrics_line_bytes() {
	let _routed = recipe::Train::new().log([
		li::Loss,
		li::Accuracy,
		li::R2,
		li::Lr,
		li::Epoch,
		li::Time,
		li::hip,
	]);
	assert!(matches!(li::hip, LogItem::Device));
	let ms = [
		Metric::Loss,
		Metric::Accuracy,
		Metric::R2,
		Metric::Lr,
		Metric::Epoch,
		Metric::Time,
	];
	let line = metrics_line(&ms, &[0.7451, 0.5528, 0.9123, 0.001, 42.0, 125.0]);
	assert_eq!(
		line.as_bytes(),
		"loss \u{1b}[38;2;242;40;60m 0.7451\u{1b}[0m  acc \u{1b}[38;2;39;125;255m0.5528\u{1b}[0m  r2 \u{1b}[38;2;0;174;107m  0.9123\u{1b}[0m  lr \u{1b}[38;2;255;194;0m 0.0010\u{1b}[0m  epoch \u{1b}[38;2;215;46;130m   42\u{1b}[0m  time \u{1b}[38;2;135;90;251m   2m 05s\u{1b}[0m"
			.as_bytes()
	);
	return;
}

#[test]
fn nan_line_bytes() {
	let line = metrics_line(&[Metric::Loss, Metric::Accuracy], &[f64::NAN, f64::NAN]);
	assert_eq!(
		line.as_bytes(),
		"loss \u{1b}[38;2;242;40;60m    N/A\u{1b}[0m  acc \u{1b}[38;2;39;125;255m   N/A\u{1b}[0m"
			.as_bytes()
	);
	return;
}

#[test]
fn empty_line_bytes() {
	assert_eq!(metrics_line(&[], &[]), "");
	return;
}

#[test]
fn metric_fmt_labels_and_widths() {
	let expect: [(Metric, &str, usize); 6] = [
		(Metric::Epoch, "epoch", 5),
		(Metric::Lr, "lr", 7),
		(Metric::Time, "time", 9),
		(Metric::Loss, "loss", 7),
		(Metric::Accuracy, "acc", 6),
		(Metric::R2, "r2", 8),
	];
	for (m, label, width) in expect {
		let f = metric_fmt(m);
		assert_eq!(f.label, label);
		assert_eq!(f.width, width);
	}
	return;
}

#[test]
fn metric_render_bytes() {
	assert_eq!(metric_render(Metric::Epoch, 42.9), "   42");
	assert_eq!(metric_render(Metric::Epoch, 0.0), "    0");
	assert_eq!(metric_render(Metric::Lr, 0.001), " 0.0010");
	assert_eq!(metric_render(Metric::Loss, 0.7451), " 0.7451");
	assert_eq!(metric_render(Metric::Loss, 123456.789), "123456.7890");
	assert_eq!(metric_render(Metric::Accuracy, 1.0), "1.0000");
	assert_eq!(metric_render(Metric::R2, -0.5), " -0.5000");
	assert_eq!(metric_render(Metric::Time, 125.0), "   2m 05s");
	assert_eq!(metric_render(Metric::Loss, f64::NAN), "    N/A");
	assert_eq!(metric_render(Metric::Accuracy, f64::NAN), "   N/A");
	assert_eq!(metric_render(Metric::R2, f64::NAN), "     N/A");
	assert_eq!(metric_render(Metric::Time, f64::NAN), "      N/A");
	assert_eq!(metric_render(Metric::Epoch, f64::NAN), "  N/A");
	assert_eq!(metric_render(Metric::Lr, f64::NAN), "    N/A");
	return;
}

#[test]
fn fmt_time_bytes() {
	assert_eq!(fmt_time(0.0), "0.0s");
	assert_eq!(fmt_time(5.3), "5.3s");
	assert_eq!(fmt_time(59.9), "59.9s");
	assert_eq!(fmt_time(60.0), "1m 00s");
	assert_eq!(fmt_time(119.0), "1m 59s");
	assert_eq!(fmt_time(3599.0), "59m 59s");
	assert_eq!(fmt_time(3600.0), "1h 00m 00s");
	assert_eq!(fmt_time(7325.0), "2h 02m 05s");
	assert_eq!(fmt_time(90061.0), "25h 01m 01s");
	return;
}

#[test]
fn palette_slot_bytes() {
	let expect: [(u8, u8, u8); 12] = [
		(242, 40, 60),
		(39, 125, 255),
		(0, 174, 107),
		(255, 194, 0),
		(215, 46, 130),
		(135, 90, 251),
		(255, 122, 0),
		(91, 192, 235),
		(157, 121, 188),
		(46, 83, 57),
		(3, 252, 186),
		(194, 1, 20),
	];
	for (i, (r, g, b)) in expect.into_iter().enumerate() {
		let c = ogdl::log::palette(i);
		assert_eq!((c.r, c.g, c.b), (r, g, b));
		let w = ogdl::log::palette(i + 12);
		assert_eq!((w.r, w.g, w.b), (r, g, b));
	}
	return;
}

#[test]
fn device_block_serialize_bytes() {
	let base = [0u64; gpu_core::callspy::N];
	let mut end = [0u64; gpu_core::callspy::N];
	end[0] = 2;
	end[2] = 5;
	end[3] = 1;
	end[7] = 340;
	end[17] = 3;
	end[21] = 1;
	end[41] = 57;
	let text = gpu_core::callspy::report_between(&base, &end).serialize();
	assert_eq!(
		text.as_bytes(),
		"sync\n\tallocations\t2\nasync\n\ttransfers\t5\n\tallocations\t1\nkernel launch\n\thipLaunchKernelGGL\t340\nsyncs\n\thipDeviceSynchronize\t3\ndevice/settings\n\thipMemGetInfo\t1\nother\n\thipBLAS\t57\n"
			.as_bytes()
	);
	return;
}
