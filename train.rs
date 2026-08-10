use recipe::*;

const DATASET_ROOT: &str = "/home/nate/Desktop/nates-recipe-rs/examples/datasets";
const REGRESSION_LOSSES: &[(&str, Loss)] = &[("mse", mse), ("rmse", rmse), ("huber", huber), ("mae", mae)];
const BINARY_LOSSES: &[(&str, Loss)] = &[("bce", bce), ("ce", ce), ("focal", focal)];

#[derive(Clone, Copy)]
enum Task {
	Regression,
	Binary,
}

impl Task {
	const fn losses(self) -> &'static [(&'static str, Loss)] {
		match self {
			Self::Regression => REGRESSION_LOSSES,
			Self::Binary => BINARY_LOSSES,
		}
	}
}

struct Dataset {
	name: &'static str,
	data: Data,
	task: Task,
}

fn datasets() -> Vec<Dataset> {
	let regression = Task::Regression;
	let binary = Task::Binary;
	vec![
		Dataset {
			name: "D7",
			data: recipe
				.data("/home/nate/Desktop/D7-data")
				.target("temper1")
				.exclude([
					"Frequency(Hz)",
					"sample",
					"time",
					"scan",
					"temperx",
					"am1",
					"am2",
					"max6675",
					"minivna",
					"am1rh",
					"am2rh",
				])
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
		Dataset {
			name: "VNA",
			data: recipe
				.data("/home/nate/Desktop/odroid-vna-data/MAX6675-VNA-D8_260807_232354")
				.target("temperature")
				.exclude(["Frequency(Hz)"])
				.norm(z_score)
				.split(1.0),
			task: regression,
		},
		Dataset {
			name: "MAX",
			data: recipe
				.data("/home/nate/Desktop/MAX6675-live-20")
				.target("max6675")
				.exclude(["Frequency(Hz)", "sample", "time", "scan"])
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
		Dataset {
			name: "house",
			data: recipe
				.data(format!("{DATASET_ROOT}/house-prices/train.csv"))
				.target("SalePrice")
				.exclude(["Id"])
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
		Dataset {
			name: "appointments",
			data: recipe
				.data(format!("{DATASET_ROOT}/no-show-appointments/KaggleV2-May-2016.csv"))
				.target("No-show")
				.exclude(["AppointmentID", "PatientId", "ScheduledDay", "AppointmentDay"])
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "sonar",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-sonar/sonar.all-data"))
				.target("col61")
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "ionosphere",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-ionosphere/ionosphere.data"))
				.target("col35")
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "wdbc",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-wdbc/wdbc.data"))
				.target("col2")
				.exclude(["col1"])
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "spambase",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-spambase/spambase.data"))
				.target("col58")
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "magic",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-magic/magic04.data"))
				.target("col11")
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "bank",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-bank-semicolon/bank.csv"))
				.target("y")
				.norm(z_score)
				.split(0.2),
			task: binary,
		},
		Dataset {
			name: "airfoil",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-airfoil/airfoil_self_noise.dat"))
				.target("col6")
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
		Dataset {
			name: "abalone",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-abalone/abalone.data"))
				.target("col9")
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
		Dataset {
			name: "wine",
			data: recipe
				.data(format!("{DATASET_ROOT}/uci-winequality-semicolon/winequality-red.csv"))
				.target("quality")
				.norm(z_score)
				.split(0.2),
			task: regression,
		},
	]
}

fn main() {
	let mut arguments = std::env::args().skip(1);
	let selected = arguments.next();
	assert!(
		arguments.next().is_none(),
		"usage: train [dataset/block/activation/normalization/loss]"
	);
	let blocks: &[(&str, fn(Model) -> Model)] = &[
		("layer", |model| model.layer(8)),
		("residual", |model| model.layer(8).residual([layer(8), relu()])),
		("conv", |model| model.conv(4, 3).pool(8)),
		("attn", |model| model.attn(1)),
		("embed", |model| model.embed(4, 8).pool(8)),
		("rnn", |model| model.rnn(4)),
		("gru", |model| model.gru(4)),
		("lstm", |model| model.lstm(4)),
		("perc", |model| model.perc(8)),
		("moe", |model| model.moe(2, [layer(4), layer(4), layer(4), layer(4)])),
		("svm", |model| model.svm([tanh, relu, gelu, sigmoid])),
		("kmeans", |model| model.kmeans(4)),
		("knn", |model| model.knn(5)),
	];
	let activations: &[(&str, fn(Model) -> Model)] = &[
		("linear", |model| model),
		("relu", |model| model.relu()),
		("gelu", |model| model.gelu()),
		("silu", |model| model.silu()),
		("tanh", |model| model.tanh()),
		("sigmoid", |model| model.sigmoid()),
		("elu", |model| model.elu()),
		("selu", |model| model.selu()),
		("prelu", |model| model.prelu()),
		("leak", |model| model.leak()),
		("exp", |model| model.exp()),
		("ln", |model| model.ln()),
		("log", |model| model.log()),
		("cos", |model| model.cos()),
		("tan", |model| model.tan()),
		("huber", |model| model.huber()),
	];
	let normalizations: &[(&str, Option<Norm>)] = &[("none", None), ("batch", Some(batch)), ("layer", Some(layer))];
	let (mut pass, mut fail) = (0_usize, 0_usize);
	for dataset in datasets() {
		for (block_name, block) in blocks {
			for (activation_name, activate) in activations {
				for (normalization_name, normalization) in normalizations {
					for (loss_name, loss) in dataset.task.losses() {
						let label = format!(
							"{}/{block_name}/{activation_name}/{normalization_name}/{loss_name}",
							dataset.name
						);
						if selected.as_ref().is_some_and(|selected| selected != &label) {
							continue;
						}
						let model = activate(block(recipe.model()));
						let model = match normalization {
							Some(normalization) => model.norm(*normalization),
							None => model,
						};
						let model = model.layer(1).loss(*loss);
						let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
							recipe
								.train()
								.fp(32)
								.seed(1)
								.epochs(1)
								.lr(0.01)
								.run(&model, &dataset.data)
						}));
						match result {
							Ok(report) => {
								pass += 1;
								eprintln!("{label}: R2 {:.4}", report.r2());
							}
							Err(_) => {
								fail += 1;
								eprintln!("{label}: FAILED");
							}
						}
					}
				}
			}
		}
	}
	assert!(pass + fail != 0, "selected matrix combination does not exist");
	assert_eq!(fail, 0, "{fail} matrix combinations failed");
	eprintln!("{pass} passed");
}
