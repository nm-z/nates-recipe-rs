use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

const RED: &str = "\x1b[38;2;255;92;122m";
const YELLOW: &str = "\x1b[38;2;229;192;123m";
const GREEN: &str = "\x1b[38;2;86;214;169m";
const BLUE: &str = "\x1b[38;2;97;175;239m";
const MUTED: &str = "\x1b[38;2;125;133;144m";
const RESET: &str = "\x1b[0m";
const SHELL_SIGNAL_OFFSET: i32 = 128;

#[derive(Clone)]
struct Config {
    repository: PathBuf,
    log_path: PathBuf,
    queue_path: PathBuf,
    reproduction_directory: PathBuf,
    decision_schema: PathBuf,
    spark_model: String,
    spark_effort: String,
    kimi_binary: PathBuf,
    kimi_k3_model: String,
    kimi_deepseek_model: String,
    kimi_agent: PathBuf,
    kimi_skills: PathBuf,
    agy_binary: PathBuf,
    agy_models: Vec<String>,
    provider_poll_seconds: u64,
    slow_cursor_seconds: u64,
    discovery_devices: Vec<String>,
    seed: u64,
    cursor: u64,
    compositions_per_batch: u64,
    batches: u64,
    publish: bool,
    debug: bool,
}

struct Decision {
    provider: &'static str,
    model: String,
    effort: String,
    json: String,
}

fn value(text: &str, name: &str) -> String {
    text.lines()
        .find_map(|line| {
            line.split_once('=')
                .filter(|(key, _)| key.trim() == name)
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| panic!("machine.toml has no {name}"))
}

fn values(text: &str, name: &str) -> Vec<String> {
    value(text, name)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn config(path: &Path) -> Config {
    let text = std::fs::read_to_string(path).expect("cannot read machine.toml");
    Config {
        repository: value(&text, "repository").into(),
        log_path: value(&text, "log_path").into(),
        queue_path: value(&text, "queue_path").into(),
        reproduction_directory: value(&text, "reproduction_directory").into(),
        decision_schema: value(&text, "decision_schema").into(),
        spark_model: value(&text, "spark_model"),
        spark_effort: value(&text, "spark_effort"),
        kimi_binary: value(&text, "kimi_binary").into(),
        kimi_k3_model: value(&text, "kimi_k3_model"),
        kimi_deepseek_model: value(&text, "kimi_deepseek_model"),
        kimi_agent: value(&text, "kimi_agent").into(),
        kimi_skills: value(&text, "kimi_skills").into(),
        agy_binary: value(&text, "agy_binary").into(),
        agy_models: values(&text, "agy_models"),
        provider_poll_seconds: value(&text, "provider_poll_seconds")
            .parse()
            .expect("provider_poll_seconds must be an unsigned integer"),
        slow_cursor_seconds: value(&text, "slow_cursor_seconds")
            .parse()
            .expect("slow_cursor_seconds must be an unsigned integer"),
        discovery_devices: values(&text, "discovery_devices"),
        seed: value(&text, "seed")
            .parse()
            .expect("seed must be an unsigned integer"),
        cursor: value(&text, "cursor")
            .parse()
            .expect("cursor must be an unsigned integer"),
        compositions_per_batch: value(&text, "compositions_per_batch")
            .parse()
            .expect("compositions_per_batch must be an unsigned integer"),
        batches: value(&text, "batches")
            .parse()
            .expect("batches must be an unsigned integer"),
        publish: value(&text, "publish")
            .parse()
            .expect("publish must be true or false"),
        debug: value(&text, "debug")
            .parse()
            .expect("debug must be true or false"),
    }
}

struct Work {
    packets: VecDeque<String>,
    discovery_done: bool,
    halted: bool,
}

struct Allocation {
    next: u64,
    claimed: u64,
}

struct Trial {
    config: Config,
    device: String,
    cursor: u64,
    reproduction: PathBuf,
    output: std::process::Output,
}

struct SlowTrial {
    config: Config,
    device: String,
    cursor: u64,
    reproduction: PathBuf,
    elapsed_seconds: u64,
}

enum Discovery {
    Slow(SlowTrial),
    Complete(Trial),
}

enum Review {
    Done,
    Deferred,
    Stop,
}

fn queued(text: &str) -> VecDeque<String> {
    let mut packets = text.split("RECIPE QUEUED FAILURE BEGIN\n")
        .skip(1)
        .filter_map(|tail| {
            tail.split_once("\nRECIPE QUEUED FAILURE END")
                .map(|(packet, _)| packet.to_owned())
        })
        .collect::<VecDeque<_>>();
    if let Some(index) = packets.iter().position(|packet| packet.starts_with("kind=performance\n")) {
        let packet = packets.remove(index).expect("performance packet disappeared from the queue");
        packets.push_front(packet);
    }
    packets
}

fn persist_queue(path: &Path, packets: &VecDeque<String>) {
    let text = packets
        .iter()
        .map(|packet| format!("RECIPE QUEUED FAILURE BEGIN\n{packet}\nRECIPE QUEUED FAILURE END\n"))
        .collect::<String>();
    let temporary = path.with_extension("next");
    std::fs::write(&temporary, text).expect("cannot write the failure queue");
    std::fs::rename(temporary, path).expect("cannot publish the failure queue");
}

fn cursor(path: &Path, next: u64) {
    let text = std::fs::read_to_string(path).expect("cannot read machine.toml");
    let updated = text
        .lines()
        .map(|line| {
            if line
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "cursor")
            {
                format!("cursor = {next}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let temporary = path.with_extension("next");
    std::fs::write(&temporary, updated).expect("cannot write the next cursor");
    std::fs::rename(temporary, path).expect("cannot publish the next cursor");
}

fn output(command: &mut Command, input: Option<&str>) -> std::process::Output {
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot start command");
    if let Some(input) = input {
        child
            .stdin
            .take()
            .expect("command has no input")
            .write_all(input.as_bytes())
            .expect("cannot write command input");
    }
    child
        .wait_with_output()
        .expect("cannot read command output")
}

fn event(config: &Config, color: &str, message: &str) {
    let time = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .expect("cannot read event time");
    let timestamp = String::from_utf8_lossy(&time.stdout);
    let timestamp = timestamp.trim();
    let line = format!("{timestamp} {message}\n");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
        .expect("cannot open machine log")
        .write_all(line.as_bytes())
        .expect("cannot write machine log");
    let (event, details) = message.split_once(' ').unwrap_or((message, ""));
    eprintln!("{MUTED}{}{RESET}  {color}{event:<8}{RESET} {details}", &timestamp[11..]);
}

fn trace(config: &Config, message: &str) {
    if !config.debug {
        return;
    }
    let time = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .expect("cannot read event time");
    let line = format!(
        "{} DEBUG {message}\n",
        String::from_utf8_lossy(&time.stdout).trim()
    );
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
        .expect("cannot open machine log")
        .write_all(line.as_bytes())
        .expect("cannot write machine log");
}

fn failure(result: &std::process::Output) -> String {
    let error = String::from_utf8_lossy(&result.stderr);
    if !error.trim().is_empty() {
        return error.trim().to_owned();
    }
    String::from_utf8_lossy(&result.stdout)
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("no diagnostic")
        .trim()
        .to_owned()
}

fn packets(text: &str) -> Vec<&str> {
    text.split("RECIPE FAILURE BEGIN\n")
        .skip(1)
        .filter_map(|tail| {
            tail.split_once("RECIPE FAILURE END")
                .map(|(packet, _)| packet.trim())
        })
        .collect()
}

fn field<'a>(packet: &'a str, name: &str) -> &'a str {
    packet
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .unwrap_or_else(|| panic!("failure packet has no {name}"))
}

fn packet_cursor(packet: &str) -> u64 {
    field(packet, "cursor=")
        .split_whitespace()
        .find_map(|value| value.strip_prefix("cursor:"))
        .expect("failure packet has no cursor")
        .parse()
        .expect("failure packet cursor is invalid")
}

fn packet_for_device(packet: &str, device: &str) -> String {
    let selector = if device == "cpu" {
        "RECIPE_FORCE_CPU=1".to_owned()
    } else {
        format!("RECIPE_DEVICE={device}")
    };
    packet
        .lines()
        .map(|line| {
            if let Some(command) = line.strip_prefix("command=") {
                format!("command={selector} {command}")
            } else {
                line.to_owned()
            }
        })
        .chain(std::iter::once(format!("backend={device}")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn same_failure(left: &str, right: &str) -> bool {
    field(left, "id=") == field(right, "id=")
        && left.lines().find_map(|line| line.strip_prefix("backend="))
            == right.lines().find_map(|line| line.strip_prefix("backend="))
}

fn reproduction_path(config: &Config, device: &str, cursor: u64) -> PathBuf {
    config.reproduction_directory.join(format!("recipe-composition-repro-{device}-{cursor}.rs"))
}

fn trial(
    config: &Config,
    device: &str,
    cursor: u64,
    reproduction: &Path,
    send: &mpsc::Sender<Discovery>,
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command
        .args(["run", "--bin", "recipe", "--", "test.rs"])
        .env("RECIPE_COMPOSITION_SEED", config.seed.to_string())
        .env("RECIPE_COMPOSITION_CURSOR", cursor.to_string())
        .env(
            "RECIPE_COMPOSITION_COUNT",
            config.compositions_per_batch.to_string(),
        )
        .env("RECIPE_COMPOSITION_REPRO", reproduction)
        .env_remove("RECIPE_DEVICE")
        .env_remove("RECIPE_FORCE_CPU")
        .current_dir(&config.repository);
    if device == "cpu" {
        command.env("RECIPE_FORCE_CPU", "1");
    } else {
        command.env("RECIPE_DEVICE", device);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("cannot start Recipe traversal");
    let mut child_stdout = child.stdout.take().expect("Recipe traversal stdout is absent");
    let mut child_stderr = child.stderr.take().expect("Recipe traversal stderr is absent");
    let stdout = std::thread::spawn(move || {
        let mut output = Vec::new();
        child_stdout.read_to_end(&mut output).expect("cannot read Recipe traversal stdout");
        output
    });
    let stderr = std::thread::spawn(move || {
        let mut output = Vec::new();
        child_stderr.read_to_end(&mut output).expect("cannot read Recipe traversal stderr");
        output
    });
    let started = Instant::now();
    let mut reported = false;
    let status = loop {
        if let Some(status) = child.try_wait().expect("cannot inspect Recipe traversal") {
            break status;
        }
        if !reported && started.elapsed() >= Duration::from_secs(config.slow_cursor_seconds) {
            reported = true;
            let _ = send.send(Discovery::Slow(SlowTrial {
                config: config.clone(),
                device: device.to_owned(),
                cursor,
                reproduction: reproduction.to_owned(),
                elapsed_seconds: started.elapsed().as_secs(),
            }));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    std::process::Output {
        status,
        stdout: stdout.join().expect("Recipe traversal stdout reader failed"),
        stderr: stderr.join().expect("Recipe traversal stderr reader failed"),
    }
}

fn termination_signal(status: std::process::ExitStatus) -> Option<i32> {
    status.signal().or_else(|| status.code().filter(|code| *code > SHELL_SIGNAL_OFFSET).map(|code| code - SHELL_SIGNAL_OFFSET))
}

fn repository_base(repository: &Path) -> String {
    let commit = output(Command::new("git").args(["rev-parse", "HEAD"]).current_dir(repository), None);
    let status = output(Command::new("git").args(["status", "--porcelain", "--untracked-files=no"]).current_dir(repository), None);
    assert!(commit.status.success() && status.status.success(), "cannot inspect the composition base");
    format!("commit={} tracked_tree={}", String::from_utf8_lossy(&commit.stdout).trim(), if status.stdout.is_empty() { "clean" } else { "modified" })
}

fn crash_packet(trial: &Trial, text: &str, signal: i32) -> String {
    let line = text.lines().find(|line| line.starts_with("composition ") && line.contains(':')).expect("signaled traversal emitted no composition");
    let (case, configuration) = line.strip_prefix("composition ").expect("composition prefix disappeared").split_once(':').expect("composition description has no separator");
    let step = text.lines().find_map(|line| line.split_whitespace().find_map(|value| value.strip_prefix("step="))).expect("signaled traversal emitted no permutation step");
    let source = std::fs::read_to_string(&trial.reproduction).expect("signaled traversal staged no reproduction");
    let mut fingerprint = 1_469_598_103_934_665_603_u64;
    for byte in format!("{}:{signal}:{case}", trial.device).bytes().chain(source.bytes()) {
        fingerprint ^= u64::from(byte);
        fingerprint = fingerprint.wrapping_mul(1_099_511_628_211);
    }
    let message = format!("device {} terminated by signal {signal}", trial.device);
    format!("id={fingerprint:016x}\nbase={}\ncursor=seed:{} cursor:{} next:{} step:{step} composition:{case}\nconfiguration={}\nexpected=training, optional resume, and inference produce finite numerical results through the public Recipe API\nobserved=phase:process message:{message}\noutput=phase:process message:{message}\nreplay=phase:process message:the process terminated before in-process replay stable:unknown\ncommand=cargo run --bin recipe -- {}\nreproduction:\n```rust\n{source}```", repository_base(&trial.config.repository), trial.config.seed, trial.cursor, trial.cursor + 1, configuration.trim(), trial.reproduction.display())
}

fn slow_packet(trial: &SlowTrial) -> String {
    let source = std::fs::read_to_string(&trial.reproduction).expect("slow traversal staged no reproduction");
    let composition = source
        .lines()
        .find_map(|line| line.trim().strip_prefix(".seed(")?.strip_suffix(')'))
        .expect("slow traversal reproduction has no composition seed");
    let mut fingerprint = 1_469_598_103_934_665_603_u64;
    for byte in format!("performance:{}:{composition}", trial.device).bytes().chain(source.bytes()) {
        fingerprint ^= u64::from(byte);
        fingerprint = fingerprint.wrapping_mul(1_099_511_628_211);
    }
    format!("kind=performance\nid={fingerprint:016x}\nbase={}\ncursor=seed:{} cursor:{} next:{} composition:{composition}\nexpected=one training epoch uses throughput proportionate to its operations, data volume, memory traffic, and available hardware\nobserved=the public Recipe run remained active after {} seconds\nmeasurement=elapsed_seconds:{} backend:{}\nreplay=the configured slow-runtime threshold was crossed stable:true\ncommand=cargo run --bin recipe -- {}\nreproduction:\n```rust\n{source}```\nbackend={}", repository_base(&trial.config.repository), trial.config.seed, trial.cursor, trial.cursor + 1, trial.elapsed_seconds, trial.elapsed_seconds, trial.device, trial.reproduction.display(), trial.device)
}

fn jq(json: &str, filter: &str) -> std::result::Result<String, String> {
    let result = output(Command::new("jq").args(["-er", filter]), Some(json));
    if !result.status.success() {
        return Err(failure(&result));
    }
    String::from_utf8(result.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| error.to_string())
}

fn jq_lines(json: &str, filter: &str) -> std::result::Result<String, String> {
    let result = output(Command::new("jq").args(["-ser", filter]), Some(json));
    if !result.status.success() {
        return Err(failure(&result));
    }
    String::from_utf8(result.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| error.to_string())
}

fn valid(json: String) -> std::result::Result<String, String> {
    let verdict = jq(&json, ".verdict")?;
    if !matches!(verdict.as_str(), "new" | "comment" | "reject") {
        return Err(format!("invalid verdict {verdict}"));
    }
    for field in [".issue", ".title", ".body", ".rationale"] {
        jq(&json, field)?;
    }
    Ok(json)
}

fn object(text: String) -> std::result::Result<String, String> {
    let start = text
        .find('{')
        .ok_or_else(|| "classifier returned no JSON object".to_owned())?;
    let end = text
        .rfind('}')
        .ok_or_else(|| "classifier returned an incomplete JSON object".to_owned())?;
    valid(text[start..=end].to_owned())
}

fn spark(config: &Config, prompt: &str) -> std::result::Result<Decision, String> {
    let effort = format!("model_reasoning_effort=\"{}\"", config.spark_effort);
    let result = output(
        Command::new("codex")
            .args([
                "exec",
                "--ignore-user-config",
                "--sandbox",
                "read-only",
                "--model",
                &config.spark_model,
                "--config",
                &effort,
                "--output-schema",
                config
                    .decision_schema
                    .to_str()
                    .expect("decision schema path is not UTF-8"),
                "--json",
                "-",
            ])
            .current_dir(&config.repository),
        Some(prompt),
    );
    let stream = String::from_utf8_lossy(&result.stdout);
    let response = jq_lines(&stream, "[.[] | select(.type == \"item.completed\" and .item.type == \"agent_message\") | .item.text][-1]");
    if !result.status.success() && response.is_err() {
        return Err(failure(&result));
    }
    let mut json = response.and_then(object);
    if json.is_err() {
        let thread = jq_lines(
            &stream,
            "[.[] | select(.type == \"thread.started\") | .thread_id][-1]",
        )?;
        trace(
            config,
            &format!(
                "model={} repairing structured output thread={thread}",
                config.spark_model
            ),
        );
        let repair = output(Command::new("codex").args([
			"exec", "resume", &thread, "--sandbox", "read-only", "--output-schema",
			config.decision_schema.to_str().expect("decision schema path is not UTF-8"), "--json", "-",
		]).current_dir(&config.repository), Some("Return only one corrected JSON object matching the required schema. Do not repeat the investigation."));
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq_lines(&repaired, "[.[] | select(.type == \"item.completed\" and .item.type == \"agent_message\") | .item.text][-1]").and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    Ok(Decision {
        provider: "OpenAI Codex",
        model: config.spark_model.clone(),
        effort: config.spark_effort.clone(),
        json,
    })
}

fn kimi(
    config: &Config,
    provider: &'static str,
    model: &str,
    prompt: &str,
) -> std::result::Result<Decision, String> {
    let result = output(
        Command::new(&config.kimi_binary)
            .args([
                "--model",
                model,
                "--prompt",
                prompt,
                "--output-format",
                "stream-json",
                "--agent-file",
                config
                    .kimi_agent
                    .to_str()
                    .expect("Kimi agent path is not UTF-8"),
                "--skills-dir",
                config
                    .kimi_skills
                    .to_str()
                    .expect("Kimi skills path is not UTF-8"),
            ])
            .current_dir(&config.repository),
        None,
    );
    let stream = String::from_utf8_lossy(&result.stdout);
    let response = jq_lines(&stream, "[.[] | select(.role == \"assistant\" and .content != null) | .content | if type == \"string\" then . else [.[] | select(.type == \"text\") | .text] | join(\"\") end][-1]");
    if !result.status.success() && response.is_err() {
        return Err(failure(&result));
    }
    let mut json = response.and_then(object);
    if json.is_err() {
        let session = jq_lines(
            &stream,
            "[.[] | select(.type == \"session.resume_hint\") | .session_id][-1]",
        )?;
        trace(
            config,
            &format!("model={model} repairing structured output session={session}"),
        );
        let repair = output(Command::new(&config.kimi_binary).args([
			"--session", &session, "--prompt", "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.",
			"--output-format", "stream-json",
		]).current_dir(&config.repository), None);
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq_lines(&repaired, "[.[] | select(.role == \"assistant\" and .content != null) | .content | if type == \"string\" then . else [.[] | select(.type == \"text\") | .text] | join(\"\") end][-1]").and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    Ok(Decision {
        provider,
        model: model.to_owned(),
        effort: "max".to_owned(),
        json,
    })
}

fn agy(config: &Config, model: &str, prompt: &str) -> std::result::Result<Decision, String> {
    let result = output(
        Command::new(&config.agy_binary)
            .args([
                "--print",
                prompt,
                "--output-format",
                "json",
                "--model",
                model,
                "--sandbox",
                "--disable-slash-commands",
                "--json-schema",
                config
                    .decision_schema
                    .to_str()
                    .expect("decision schema path is not UTF-8"),
            ])
            .current_dir(&config.repository),
        None,
    );
    let text = String::from_utf8_lossy(&result.stdout);
    let response = jq(&text, ".structured_output | tojson").or_else(|_| jq(&text, ".response"));
    let mut json = response.and_then(object);
    if json.is_err() {
        let conversation = jq(&text, ".conversation_id")?;
        trace(
            config,
            &format!("model={model} repairing structured output conversation={conversation}"),
        );
        let repair = output(Command::new(&config.agy_binary).args([
			"--conversation", &conversation, "--print", "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.",
			"--output-format", "json", "--sandbox", "--disable-slash-commands", "--json-schema",
			config.decision_schema.to_str().expect("decision schema path is not UTF-8"),
		]).current_dir(&config.repository), None);
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq(&repaired, ".structured_output | tojson")
            .or_else(|_| jq(&repaired, ".response"))
            .and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    let effort = if model.contains("high") {
        "high"
    } else if model.contains("medium") {
        "medium"
    } else {
        "thinking"
    };
    Ok(Decision {
        provider: "Google Antigravity",
        model: model.to_owned(),
        effort: effort.to_owned(),
        json,
    })
}

fn classify(config: &Config, prompt: &str) -> Decision {
    loop {
        match spark(config, prompt) {
            Ok(decision) => return decision,
            Err(error) => trace(
                config,
                &format!("model={} unavailable error={error}", config.spark_model),
            ),
        }
        match kimi(config, "Kimi managed", &config.kimi_k3_model, prompt) {
            Ok(decision) => return decision,
            Err(error) => trace(
                config,
                &format!("model={} unavailable error={error}", config.kimi_k3_model),
            ),
        }
        for model in &config.agy_models {
            match agy(config, model, prompt) {
                Ok(decision) => return decision,
                Err(error) => trace(config, &format!("model={model} unavailable error={error}")),
            }
        }
        match kimi(
            config,
            "DeepSeek through Kimi",
            &config.kimi_deepseek_model,
            prompt,
        ) {
            Ok(decision) => return decision,
            Err(error) => trace(
                config,
                &format!(
                    "model={} unavailable error={error}",
                    config.kimi_deepseek_model
                ),
            ),
        }
        trace(
            config,
            &format!(
                "all classifiers unavailable poll={}s",
                config.provider_poll_seconds
            ),
        );
        std::thread::sleep(std::time::Duration::from_secs(config.provider_poll_seconds));
    }
}

fn triage(config: &Config, instructions: &str, packet: &str) -> Review {
    if field(packet, "replay=").ends_with("stable:false") {
        event(
            config,
            YELLOW,
            &format!(
                "REJECT composition=unstable fingerprint={}",
                field(packet, "id=")
            ),
        );
        return Review::Done;
    }
    let cursor = field(packet, "cursor=");
    let composition = cursor
        .split_whitespace()
        .find_map(|value| value.strip_prefix("composition:"))
        .expect("failure cursor has no composition");
    let schema =
        std::fs::read_to_string(&config.decision_schema).expect("cannot read decision schema");
    let prompt = format!("{instructions}\n\n## Failure packet\n\n{packet}\n\n## Required decision schema\n\n{schema}");
    let performance = packet.starts_with("kind=performance\n");
    let decision = if performance {
        match kimi(config, "Kimi managed", &config.kimi_k3_model, &prompt) {
            Ok(decision) => decision,
            Err(error) => {
                trace(config, &format!("model={} unavailable error={error}", config.kimi_k3_model));
                return Review::Deferred;
            }
        }
    } else {
        classify(config, &prompt)
    };
    event(
        config,
        RED,
        &format!(
            "CLASSIFY model={} composition={composition}",
            decision.model
        ),
    );
    let verdict = jq(&decision.json, ".verdict").expect("validated decision lost its verdict");
    let issue = jq(&decision.json, ".issue").expect("validated decision lost its issue");
    let title = jq(&decision.json, ".title").expect("validated decision lost its title");
    let mut body = jq(&decision.json, ".body").expect("validated decision lost its body");
    if !config.publish {
        event(
            config,
            YELLOW,
            &format!(
                "DECISION model={} verdict={verdict} issue={issue} title={title}",
                decision.model
            ),
        );
        return Review::Stop;
    }
    if verdict == "reject" {
        event(
            config,
            YELLOW,
            &format!("REJECT model={} composition={composition}", decision.model),
        );
        return Review::Done;
    }
    assert!(
        !title.is_empty() && !body.is_empty(),
        "classifier returned empty issue text"
    );
    let fingerprint = field(packet, "id=");
    let provenance = format!(
        r#"## Machine provenance

- Discoverer: Recipe exhaustive Cartesian traversal
- Failure fingerprint: `{fingerprint}`
- Classifier provider: `{}`
- Classifier model: `{}`
- Classifier effort: `{}`

### Classifier decision

```json
{}
```"#,
        decision.provider, decision.model, decision.effort, decision.json
    );
    body = format!("<!-- recipe-failure:{fingerprint} -->\n\n{body}\n\n{provenance}\n\n## Deterministic failure packet\n\n{packet}");
    let published = match verdict.as_str() {
        "new" => output(
            Command::new("gh")
                .args([
                    "issue", "create", "--label", "bug", "--title", &title, "--body", &body,
                ])
                .current_dir(&config.repository),
            None,
        ),
        "comment" => output(
            Command::new("gh")
                .args(["issue", "comment", &issue, "--body", &body])
                .current_dir(&config.repository),
            None,
        ),
        _ => unreachable!(),
    };
    assert!(
        published.status.success(),
        "GitHub publication failed: {}",
        failure(&published)
    );
    let url = String::from_utf8_lossy(&published.stdout).trim().to_owned();
    let published_issue = if verdict == "new" {
        url.rsplit('/').next().unwrap_or("unknown")
    } else {
        &issue
    };
    let action = if verdict == "new" {
        "created"
    } else {
        "commented on"
    };
    let color = if verdict == "new" { GREEN } else { BLUE };
    event(
        config,
        color,
        &format!(
            "ISSUE model={} {action} issue=#{published_issue} url={url}",
            decision.model
        ),
    );
    Review::Done
}

fn main() {
    let directory = std::env::current_exe()
        .expect("cannot locate machine executable")
        .parent()
        .expect("machine has no directory")
        .to_owned();
    let path = directory.join("machine.toml");
    let initial = config(&path);
    let pending = std::fs::read_to_string(&initial.queue_path)
        .map(|text| queued(&text))
        .unwrap_or_default();
    let work = Arc::new((
        Mutex::new(Work {
            packets: pending,
            discovery_done: false,
            halted: false,
        }),
        Condvar::new(),
    ));
    let reviewer_work = Arc::clone(&work);
    let reviewer_path = path.clone();
    let instructions = Arc::new(
        std::fs::read_to_string(directory.join("triage.md")).expect("cannot read triage.md"),
    );
    let reviewer_instructions = Arc::clone(&instructions);
    let reviewer = std::thread::spawn(move || loop {
        let packet = {
            let (lock, ready) = &*reviewer_work;
            let mut state = lock.lock().expect("failure queue lock is poisoned");
            while state.packets.is_empty() && !state.discovery_done && !state.halted {
                state = ready.wait(state).expect("failure queue lock is poisoned");
            }
            if state.halted || state.packets.is_empty() && state.discovery_done {
                break;
            }
            state
                .packets
                .front()
                .expect("notified failure queue is empty")
                .clone()
        };
        let current = config(&reviewer_path);
        match triage(&current, &reviewer_instructions, &packet) {
            Review::Done => {}
            Review::Deferred => break,
            Review::Stop => {
                let (lock, ready) = &*reviewer_work;
                let mut state = lock.lock().expect("failure queue lock is poisoned");
                state.halted = true;
                ready.notify_all();
                break;
            }
        }
        let (lock, _) = &*reviewer_work;
        let mut state = lock.lock().expect("failure queue lock is poisoned");
        let reviewed = field(&packet, "id=");
        let index = state
            .packets
            .iter()
            .position(|queued| field(queued, "id=") == reviewed)
            .expect("reviewed packet disappeared from the failure queue");
        state.packets.remove(index);
        persist_queue(&current.queue_path, &state.packets);
    });
    assert!(
        !initial.discovery_devices.is_empty(),
        "discovery_devices must contain at least one device"
    );
    let allocation = Arc::new(Mutex::new(Allocation {
        next: initial.cursor,
        claimed: 0,
    }));
    let (send, receive) = mpsc::channel();
    let mut discoverers = Vec::new();
    for device in initial.discovery_devices.clone() {
        let worker_path = path.clone();
        let worker_work = Arc::clone(&work);
        let worker_allocation = Arc::clone(&allocation);
        let worker_send = send.clone();
        discoverers.push(std::thread::spawn(move || loop {
            if worker_work
                .0
                .lock()
                .expect("failure queue lock is poisoned")
                .halted
            {
                break;
            }
            let current = config(&worker_path);
            let start = {
                let mut allocation = worker_allocation
                    .lock()
                    .expect("cursor allocator lock is poisoned");
                if current.batches != 0 && allocation.claimed == current.batches {
                    break;
                }
                let start = allocation.next;
                allocation.next = allocation
                    .next
                    .saturating_add(current.compositions_per_batch);
                allocation.claimed += 1;
                start
            };
            let reproduction = reproduction_path(&current, &device, start);
            let result = trial(&current, &device, start, &reproduction, &worker_send);
            if worker_send
                .send(Discovery::Complete(Trial {
                    config: current,
                    device: device.clone(),
                    cursor: start,
                    reproduction,
                    output: result,
                }))
                .is_err()
            {
                break;
            }
        }));
    }
    drop(send);
    let mut frontier = initial.cursor;
    let mut completed = BTreeMap::new();
    for discovery in receive {
        let trial = match discovery {
            Discovery::Slow(trial) => {
                let packet = slow_packet(&trial);
                let composition = field(&packet, "cursor=")
                    .split_whitespace()
                    .find_map(|value| value.strip_prefix("composition:"))
                    .expect("slow cursor has no composition")
                    .to_owned();
                let (lock, ready) = &*work;
                let mut state = lock.lock().expect("failure queue lock is poisoned");
                if !state.packets.iter().any(|queued| same_failure(queued, &packet)) {
                    state.packets.push_front(packet);
                    persist_queue(&trial.config.queue_path, &state.packets);
                    event(&trial.config, YELLOW, &format!("SLOW device={} cursor={} composition={composition} elapsed={}s depth={}", trial.device, trial.cursor, trial.elapsed_seconds, state.packets.len()));
                    ready.notify_one();
                }
                continue;
            }
            Discovery::Complete(trial) => trial,
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&trial.output.stderr),
            String::from_utf8_lossy(&trial.output.stdout)
        );
        let signal = termination_signal(trial.output.status);
        assert!(
            trial.output.status.success() || signal.is_some(),
            "Recipe traversal failed outside a failure packet on {} with status {:?}: {}",
            trial.device,
            trial.output.status,
            failure(&trial.output)
        );
        let mut failures = packets(&text)
            .into_iter()
            .map(|packet| packet_for_device(packet, &trial.device))
            .collect::<Vec<_>>();
        if let Some(signal) = signal {
            failures.push(packet_for_device(&crash_packet(&trial, &text, signal), &trial.device));
        }
        for (offset, composition) in text
            .lines()
            .filter_map(|line| {
                let (composition, _) = line.strip_prefix("composition ")?.split_once(':')?;
                composition.parse::<u64>().ok().map(|_| composition)
            })
            .enumerate()
        {
            let analyzed = trial.cursor + offset as u64;
            let failed = failures
                .iter()
                .any(|packet| packet_cursor(packet) == analyzed);
            let (color, status) = if failed {
                (RED, "FAIL")
            } else {
                (GREEN, "PASS")
            };
            event(
                &trial.config,
                color,
                &format!(
                    "{status} device={} cursor={analyzed} composition={composition}",
                    trial.device
                ),
            );
        }
        for packet in &failures {
            let packet_cursor = packet_cursor(packet);
            let composition = field(packet, "cursor=")
                .split_whitespace()
                .find_map(|value| value.strip_prefix("composition:"))
                .expect("failure cursor has no composition");
            let (lock, ready) = &*work;
            let mut state = lock.lock().expect("failure queue lock is poisoned");
            if !state
                .packets
                .iter()
                .any(|queued| same_failure(queued, packet))
            {
                state.packets.push_back(packet.clone());
                persist_queue(&trial.config.queue_path, &state.packets);
                event(
                    &trial.config,
                    BLUE,
                    &format!(
                        "QUEUE device={} cursor={packet_cursor} composition={composition} depth={}",
                        trial.device,
                        state.packets.len()
                    ),
                );
                ready.notify_one();
            }
        }
        let next = if signal.is_some() {
            trial.cursor + 1
        } else {
            text.lines()
                .find_map(|line| line.strip_prefix("composition cursor="))
                .expect("Recipe traversal emitted no next cursor")
                .parse()
                .expect("Recipe next cursor is invalid")
        };
        completed.insert(trial.cursor, next);
        while let Some(next) = completed.remove(&frontier) {
            cursor(&path, next);
            frontier = next;
        }
    }
    for discoverer in discoverers {
        discoverer.join().expect("cursor discoverer failed");
    }
    {
        let (lock, ready) = &*work;
        let mut state = lock.lock().expect("failure queue lock is poisoned");
        state.discovery_done = true;
        ready.notify_all();
    }
    reviewer.join().expect("reviewer thread failed");
}
