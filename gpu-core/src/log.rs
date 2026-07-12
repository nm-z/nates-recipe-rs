use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

macro_rules! buckets {
	($($name:ident),+ $(,)?) => {
		#[derive(Clone, Copy, PartialEq, Debug)]
		pub enum Log {
			$($name,)+
		}
	};
}

buckets!(Error, Info, Metrics, Hip);

pub const RUN_LOG_DIR: &str = "/tmp/recipe";
pub const RUN_LOG_PATH: &str = concat!("/tmp/recipe/run", env!("RECIPE_GIT_HASH"), ".log");

static SINK: Mutex<Option<File>> = Mutex::new(None);
static SUBSCRIBED: Mutex<Vec<Log>> = Mutex::new(Vec::new());

pub fn subscribe(buckets: &[Log]) {
	*SUBSCRIBED.lock().unwrap_or_else(|p| p.into_inner()) = buckets.to_vec();
}

fn strip_ansi(s: &str) -> String {
	let mut out = String::with_capacity(s.len());
	let mut it = s.chars();
	while let Some(c) = it.next() {
		match c {
			'\u{1b}' => {
				for c2 in it.by_ref() {
					if c2 == 'm' {
						break;
					}
				}
			}
			_other => out.push(c),
		}
	}
	out
}

fn file_line(bucket: Log, msg: &str) {
	let mut g = SINK.lock().unwrap_or_else(|p| p.into_inner());
	if g.is_none() {
		std::fs::create_dir_all(RUN_LOG_DIR).expect("log: create /tmp/recipe");
		let f = OpenOptions::new()
			.create(true)
			.append(true)
			.open(RUN_LOG_PATH)
			.expect("log: open run log");
		*g = Some(f);
	}
	let f = g.as_mut().expect("log: sink");
	let tag = format!("{bucket:?}").to_lowercase();
	writeln!(f, "[{tag}] {}", strip_ansi(msg)).expect("log: write run log");
}

pub fn line(bucket: Log, msg: &str) {
	match bucket {
		Log::Error => eprintln!("\x1b[1;31m{msg}\x1b[0m"),
		_other => {
			let sub = SUBSCRIBED
				.lock()
				.unwrap_or_else(|p| p.into_inner())
				.contains(&bucket);
			if sub {
				println!("{msg}");
			}
		}
	}
	file_line(bucket, msg);
}
