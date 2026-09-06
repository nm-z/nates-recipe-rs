//! Standalone Hugging Face tokenization for Recipe input preparation.

use std::{
	env,
	error::Error,
	fmt, fs,
	io::{self, Read},
	path::PathBuf,
	process::ExitCode,
};

use tokenizers::Tokenizer;
use write::{always, err};

fn main() -> ExitCode {
	let arguments = env::args_os().skip(1).collect::<Vec<_>>();
	if matches!(arguments.as_slice(), [argument] if argument == "-h" || argument == "--help") {
		always(usage());
		return ExitCode::SUCCESS;
	}
	match run(arguments.into_iter()) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			drop(err(format!("tokenize: {error}\n{}", usage())));
			ExitCode::FAILURE
		}
	}
}

fn run(arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<(), TokenizeError> {
	let input = Input::parse(arguments)?;
	let tokenizer = Tokenizer::from_file(&input.model).map_err(|error| {
		TokenizeError::new(format!(
			"load tokenizer model {}: {error}",
			input.model.display()
		))
	})?;
	let text = read_text(&input.text)?;
	let encoding = tokenizer
		.encode(text, input.add_special_tokens)
		.map_err(|error| TokenizeError::new(format!("tokenize input text: {error}")))?;
	let mut output = Vec::with_capacity(encoding.len().saturating_mul(core::mem::size_of::<i32>()));
	for id in encoding.get_ids() {
		let id = i32::try_from(*id).map_err(|error| TokenizeError::new(format!("token ID {id} is outside int32: {error}")))?;
		output.extend_from_slice(&id.to_le_bytes());
	}
	write_output(&input.output, &output)?;
	Ok(())
}

#[derive(Debug)]
struct Input {
	model: PathBuf,
	text: Source,
	output: Destination,
	add_special_tokens: bool,
}

impl Input {
	fn parse(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> Result<Self, TokenizeError> {
		let mut positional = Vec::new();
		let mut add_special_tokens = false;
		for argument in arguments.by_ref() {
			if argument == "--add-special-tokens" {
				if add_special_tokens {
					return Err(TokenizeError::new(
						"--add-special-tokens may be supplied only once",
					));
				}
				add_special_tokens = true;
			} else {
				positional.push(argument);
			}
		}
		if positional.len() != 3 {
			return Err(TokenizeError::new(
				"expected TOKENIZER_JSON, INPUT_TEXT, and OUTPUT_IDS",
			));
		}
		let output = positional.pop().expect("three positional arguments");
		let text = positional.pop().expect("three positional arguments");
		let model = positional.pop().expect("three positional arguments");
		Ok(Self {
			model: PathBuf::from(model),
			text: Source::new(text),
			output: Destination::new(output),
			add_special_tokens,
		})
	}
}

#[derive(Debug)]
enum Source {
	Stdin,
	File(PathBuf),
}

impl Source {
	fn new(value: std::ffi::OsString) -> Self {
		if value == "-" {
			Self::Stdin
		} else {
			Self::File(PathBuf::from(value))
		}
	}
}

#[derive(Debug)]
enum Destination {
	Stdout,
	File(PathBuf),
}

impl Destination {
	fn new(value: std::ffi::OsString) -> Self {
		if value == "-" {
			Self::Stdout
		} else {
			Self::File(PathBuf::from(value))
		}
	}
}

fn read_text(source: &Source) -> Result<String, TokenizeError> {
	match source {
		Source::Stdin => {
			let mut text = String::new();
			io::stdin()
				.read_to_string(&mut text)
				.map_err(|error| TokenizeError::new(format!("read input text from stdin: {error}")))?;
			Ok(text)
		}
		Source::File(path) => fs::read_to_string(path).map_err(|error| TokenizeError::new(format!("read input text {}: {error}", path.display()))),
	}
}

fn write_output(destination: &Destination, bytes: &[u8]) -> Result<(), TokenizeError> {
	match destination {
		Destination::Stdout => write::stdout(bytes).map_err(|error| TokenizeError::new(format!("write token IDs: {error}"))),
		Destination::File(path) => fs::write(path, bytes).map_err(|error| TokenizeError::new(format!("write token output {}: {error}", path.display()))),
	}
}

#[derive(Debug)]
struct TokenizeError {
	detail: String,
}

impl TokenizeError {
	fn new(detail: impl Into<String>) -> Self {
		Self {
			detail: detail.into(),
		}
	}
}

impl fmt::Display for TokenizeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { self.detail.fmt(formatter) }
}

impl Error for TokenizeError {}

fn usage() -> &'static str {
	"usage: tokenize TOKENIZER_JSON INPUT_TEXT OUTPUT_IDS [--add-special-tokens]\n\
	 INPUT_TEXT and OUTPUT_IDS may be - for stdin or stdout; output is raw little-endian int32"
}
