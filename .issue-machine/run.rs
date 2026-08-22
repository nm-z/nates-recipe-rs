use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const RED: &str = "\x1b[38;2;255;92;122m";
const YELLOW: &str = "\x1b[38;2;229;192;123m";
const GREEN: &str = "\x1b[38;2;86;214;169m";
const BLUE: &str = "\x1b[38;2;97;175;239m";
const TIME: &str = "\x1b[38;2;255;194;0m";
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
    kimi_agent: PathBuf,
    kimi_skills: PathBuf,
    agy_binary: PathBuf,
    agy_models: Vec<String>,
    opencode_binary: PathBuf,
    opencode_config: PathBuf,
    opencode_models: Vec<String>,
    opencode_concurrency: BTreeMap<String, usize>,
    opencode_agent: String,
    copilot_binary: PathBuf,
    copilot_model: String,
    claude_binary: PathBuf,
    ollama_binary: PathBuf,
    ollama_model: String,
    resolver_enabled: bool,
    resolver_model: String,
    resolver_effort: String,
    resolver_base: String,
    resolver_worktree_root: PathBuf,
    resolver_poll_seconds: u64,
    resolver_concurrency: usize,
    resolver_memory_mib: u64,
    review_concurrency: usize,
    provider_poll_seconds: u64,
    slow_cursor_seconds: u64,
    trial_memory_mib: u64,
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

fn limits(text: &str, name: &str) -> BTreeMap<String, usize> {
    values(text, name)
        .into_iter()
        .map(|entry| {
            let (model, limit) = entry
                .rsplit_once(':')
                .unwrap_or_else(|| panic!("{name} entry has no concurrency limit: {entry}"));
            let limit = limit
                .parse()
                .unwrap_or_else(|_| panic!("{name} entry has an invalid concurrency limit: {entry}"));
            assert!(limit > 0, "{name} entry must have a positive concurrency limit: {entry}");
            (model.to_owned(), limit)
        })
        .collect()
}

fn config(path: &Path) -> Config {
    let text = std::fs::read_to_string(path).expect("cannot read machine.toml");
    let opencode_models = values(&text, "opencode_models");
    let opencode_concurrency = limits(&text, "opencode_concurrency");
    assert_eq!(
        opencode_models.iter().cloned().collect::<BTreeSet<_>>(),
        opencode_concurrency.keys().cloned().collect(),
        "opencode_concurrency must define every configured OpenCode model exactly once",
    );
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
        kimi_agent: value(&text, "kimi_agent").into(),
        kimi_skills: value(&text, "kimi_skills").into(),
        agy_binary: value(&text, "agy_binary").into(),
        agy_models: values(&text, "agy_models"),
        opencode_binary: value(&text, "opencode_binary").into(),
        opencode_config: value(&text, "opencode_config").into(),
        opencode_models,
        opencode_concurrency,
        opencode_agent: value(&text, "opencode_agent"),
        copilot_binary: value(&text, "copilot_binary").into(),
        copilot_model: value(&text, "copilot_model"),
        claude_binary: value(&text, "claude_binary").into(),
        ollama_binary: value(&text, "ollama_binary").into(),
        ollama_model: value(&text, "ollama_model"),
        resolver_enabled: value(&text, "resolver_enabled")
            .parse()
            .expect("resolver_enabled must be true or false"),
        resolver_model: value(&text, "resolver_model"),
        resolver_effort: value(&text, "resolver_effort"),
        resolver_base: value(&text, "resolver_base"),
        resolver_worktree_root: value(&text, "resolver_worktree_root").into(),
        resolver_poll_seconds: value(&text, "resolver_poll_seconds")
            .parse()
            .expect("resolver_poll_seconds must be an unsigned integer"),
        resolver_concurrency: value(&text, "resolver_concurrency")
            .parse()
            .expect("resolver_concurrency must be an unsigned integer"),
        resolver_memory_mib: value(&text, "resolver_memory_mib")
            .parse()
            .expect("resolver_memory_mib must be an unsigned integer"),
        review_concurrency: value(&text, "review_concurrency")
            .parse()
            .expect("review_concurrency must be an unsigned integer"),
        provider_poll_seconds: value(&text, "provider_poll_seconds")
            .parse()
            .expect("provider_poll_seconds must be an unsigned integer"),
        slow_cursor_seconds: value(&text, "slow_cursor_seconds")
            .parse()
            .expect("slow_cursor_seconds must be an unsigned integer"),
        trial_memory_mib: value(&text, "trial_memory_mib")
            .parse()
            .expect("trial_memory_mib must be an unsigned integer"),
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
    timed_out: bool,
}

struct SlowTrial {
    config: Config,
    device: String,
    cursor: u64,
    reproduction: PathBuf,
    elapsed_seconds: u64,
}

enum Discovery {
    Start { device: String, cursor: u64 },
    Slow(SlowTrial),
    Complete(Trial),
}

enum Review {
    Done,
    Stop,
}

struct Active {
    cursor: u64,
    started: Instant,
}

struct ResolverNode {
    issue: u64,
    model: String,
    started: Instant,
}

struct ReviewNode {
    device: String,
    cursor: u64,
    elapsed: String,
    status: &'static str,
    model: Option<String>,
    started: Option<Instant>,
}

#[derive(Default)]
struct Display {
    active: BTreeMap<String, Active>,
    reviews: BTreeMap<String, ReviewNode>,
    resolvers: BTreeMap<u64, ResolverNode>,
    rows: usize,
}

static DISPLAY: OnceLock<Mutex<Display>> = OnceLock::new();
static SPARK: OnceLock<Mutex<usize>> = OnceLock::new();
static KIMI: OnceLock<Mutex<usize>> = OnceLock::new();
static AGY: OnceLock<Mutex<usize>> = OnceLock::new();
static OPENCODE: OnceLock<Mutex<BTreeMap<String, usize>>> = OnceLock::new();
static COPILOT: OnceLock<Mutex<usize>> = OnceLock::new();
static OLLAMA: OnceLock<Mutex<usize>> = OnceLock::new();

fn display() -> &'static Mutex<Display> { DISPLAY.get_or_init(|| Mutex::new(Display::default())) }

struct Provider {
    slots: &'static Mutex<usize>,
}

impl Drop for Provider {
    fn drop(&mut self) {
        *self.slots.lock().expect("provider lock is poisoned") -= 1;
    }
}

fn provider(slot: &'static OnceLock<Mutex<usize>>, limit: usize) -> Option<Provider> {
    let slots = slot.get_or_init(|| Mutex::new(0));
    let mut active = slots.lock().expect("provider lock is poisoned");
    if *active >= limit {
        return None;
    }
    *active += 1;
    drop(active);
    Some(Provider { slots })
}

struct ModelProvider {
    slots: &'static Mutex<BTreeMap<String, usize>>,
    model: String,
}

impl Drop for ModelProvider {
    fn drop(&mut self) {
        let mut slots = self.slots.lock().expect("model provider lock is poisoned");
        let active = slots.get_mut(&self.model).expect("active model has no provider slot");
        *active -= 1;
        if *active == 0 {
            slots.remove(&self.model);
        }
    }
}

fn model_provider(
    slot: &'static OnceLock<Mutex<BTreeMap<String, usize>>>,
    model: &str,
    limit: usize,
) -> Option<ModelProvider> {
    let slots = slot.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut active = slots.lock().expect("model provider lock is poisoned");
    let count = active.entry(model.to_owned()).or_default();
    if *count >= limit {
        return None;
    }
    *count += 1;
    drop(active);
    Some(ModelProvider { slots, model: model.to_owned() })
}

fn identity(provider: &str, model: &str) -> String { format!("{provider}/{model}") }

fn display_clear(display: &mut Display) {
    if display.rows == 0 {
        return;
    }
    eprint!("\x1b[{}A", display.rows);
    for row in 0..display.rows {
        eprint!("\r\x1b[2K");
        if row + 1 != display.rows {
            eprint!("\x1b[1B");
        }
    }
    if display.rows != 1 {
        eprint!("\x1b[{}A", display.rows - 1);
    }
    display.rows = 0;
}

fn elapsed(started: Instant) -> String {
    let seconds = started.elapsed().as_secs_f64();
    if seconds < 1.0 {
        format!("{:>7.3} ms", seconds * 1000.0)
    } else {
        format!("{seconds:>8.4} s")
    }
}

fn fit(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        return value.to_owned();
    }
    if width <= 2 {
        return value.chars().take(width).collect();
    }
    format!("{}..", value.chars().take(width - 2).collect::<String>())
}

fn trial_line(device: &str, cursor: u64, elapsed: &str, status: &str) -> String {
    let device_color = if device == "cpu" { GREEN } else { BLUE };
    let status_color = match status {
        "PASS" => GREEN,
        "FAIL" => RED,
        "SLOW" => YELLOW,
        _ => RESET,
    };
    format!("{device_color}{device:<4}{RESET}  cursor {cursor:<6}  time {TIME}{elapsed}{RESET}  status {status_color}{status:<4}{RESET}")
}

fn pane_size() -> (usize, usize) {
    let pane = std::env::var("TMUX_PANE").expect("machine display is not running in tmux");
    let size = output(Command::new("tmux").args(["display-message", "-p", "-t", &pane, "#{pane_width} #{pane_height}"]), None);
    assert!(size.status.success(), "cannot read tmux pane size");
    let text = String::from_utf8(size.stdout).expect("tmux pane size is not UTF-8");
    let mut values = text.split_whitespace().map(|value| value.parse::<usize>().expect("tmux pane size is invalid"));
    (
        values.next().expect("tmux pane width is absent").saturating_sub(1),
        values.next().expect("tmux pane height is absent"),
    )
}

fn display_render(display: &mut Display) {
    display_clear(display);
    let width = pane_size().0;
    let trials = display.active.iter().map(|(device, active)| {
        format!("{device:<4}  cursor {:<6}  time {}", active.cursor, elapsed(active.started))
    }).collect::<Vec<_>>();
    let mut reviews = BTreeMap::<String, Vec<(String, u64, String)>>::new();
    for review in display.reviews.values() {
        let Some(identity) = review.model.as_deref() else { continue };
        let Some(started) = review.started else { continue };
        let (provider, model) = identity.split_once('/').unwrap_or((identity, identity));
        reviews.entry(provider.to_owned()).or_default().push((model.to_owned(), review.cursor, elapsed(started)));
    }
    let queued = display.reviews.values().filter(|review| review.model.is_none()).count();
    let resolving = display.resolvers.values().map(|resolver| {
        format!("{:<28}  issue #{:<5}  time {}", resolver.model, resolver.issue, elapsed(resolver.started))
    }).collect::<Vec<_>>();
    if queued != 0 {
        eprintln!("{YELLOW}queued{RESET}");
        eprintln!("└─ {queued} reviews");
        display.rows += 2;
    }
    if !trials.is_empty() || !reviews.is_empty() || !resolving.is_empty() {
        eprintln!("{BLUE}live{RESET}");
        display.rows += 1;
    }
    if !trials.is_empty() {
        let branch = if reviews.is_empty() && resolving.is_empty() { "└─" } else { "├─" };
        eprintln!("{branch} trial");
        display.rows += 1;
        for (index, line) in trials.iter().enumerate() {
            let branch = if index + 1 == trials.len() { "└─" } else { "├─" };
            let trunk = if reviews.is_empty() && resolving.is_empty() { "   " } else { "│  " };
            eprintln!("{}", fit(&format!("{trunk}{branch} {line}"), width));
            display.rows += 1;
        }
    }
    if !reviews.is_empty() {
        let review_branch = if resolving.is_empty() { "└─" } else { "├─" };
        let review_trunk = if resolving.is_empty() { "   " } else { "│  " };
        eprintln!("{review_branch} review");
        display.rows += 1;
        let provider_count = reviews.len();
        for (provider_index, (provider, models)) in reviews.into_iter().enumerate() {
            let provider_last = provider_index + 1 == provider_count;
            let provider_branch = if provider_last { "└─" } else { "├─" };
            if models.len() == 1 {
                let (model, cursor, elapsed) = &models[0];
                eprintln!("{}", fit(&format!("{review_trunk}{provider_branch} {provider}/{model:<28}  cursor {cursor:<6}  time {elapsed}"), width));
                display.rows += 1;
                continue;
            }
            eprintln!("{review_trunk}{provider_branch} {provider}");
            display.rows += 1;
            for (model_index, (model, cursor, elapsed)) in models.iter().enumerate() {
                let branch = if model_index + 1 == models.len() { "└─" } else { "├─" };
                let trunk = if provider_last { format!("{review_trunk}   ") } else { format!("{review_trunk}│  ") };
                eprintln!("{}", fit(&format!("{trunk}{branch} {model:<28}  cursor {cursor:<6}  time {elapsed}"), width));
                display.rows += 1;
            }
        }
    }
    if !resolving.is_empty() {
        eprintln!("└─ resolve");
        display.rows += 1;
        for (index, resolver) in resolving.iter().enumerate() {
            let branch = if index + 1 == resolving.len() { "└─" } else { "├─" };
            eprintln!("{}", fit(&format!("   {branch} {resolver}"), width));
            display.rows += 1;
        }
    }
    std::io::stderr().flush().expect("cannot draw machine status");
}

fn display_start(device: &str, cursor: u64) {
    let mut display = display().lock().expect("display lock is poisoned");
    display.active.insert(device.to_owned(), Active { cursor, started: Instant::now() });
    display_render(&mut display);
}

fn display_finish(device: &str, failed: bool) {
    let mut display = display().lock().expect("display lock is poisoned");
    display_clear(&mut display);
    if let Some(active) = display.active.remove(device) {
        let status = if failed { "FAIL" } else { "PASS" };
        eprintln!("{}", trial_line(device, active.cursor, &elapsed(active.started), status));
    }
    display_render(&mut display);
}

fn display_queue(packet: &str) {
    let key = packet_key(packet);
    let cursor = packet_cursor(packet);
    let device = packet_backend(packet).to_owned();
    let mut display = display().lock().expect("display lock is poisoned");
    let active = display.active.get(&device).is_some_and(|active| active.cursor == cursor)
        .then(|| display.active.remove(&device).expect("matched active trial disappeared"));
    let duration = active.map(|active| elapsed(active.started)).unwrap_or_else(|| {
        packet.lines().find_map(|line| {
            line.strip_prefix("measurement=elapsed_seconds:")
                .and_then(|value| value.split_whitespace().next())
                .and_then(|seconds| seconds.parse::<f64>().ok())
                .map(|seconds| format!("{seconds:>8.4} s"))
        }).unwrap_or_else(|| "     ...".to_owned())
    });
    display.reviews.entry(key).or_insert(ReviewNode {
        device,
        cursor,
        elapsed: duration,
        status: if packet.starts_with("kind=performance\n") { "SLOW" } else { "FAIL" },
        model: None,
        started: None,
    });
    display_render(&mut display);
}

fn display_reviewing(packet: &str, model: &str) {
    let key = packet_key(packet);
    let mut display = display().lock().expect("display lock is poisoned");
    if let Some(review) = display.reviews.get_mut(&key) {
        if model == "queued" || model.starts_with("waiting for ") {
            review.model = None;
            review.started = None;
        } else if review.model.as_deref() != Some(model) {
            review.model = Some(model.to_owned());
            review.started = Some(Instant::now());
        }
    }
    display_render(&mut display);
}

fn display_reviewed(packet: &str, model: &str, result: &str, url: Option<&str>) {
    let key = packet_key(packet);
    let mut display = display().lock().expect("display lock is poisoned");
    display_clear(&mut display);
    if let Some(review) = display.reviews.remove(&key) {
        eprintln!("{}", trial_line(&review.device, review.cursor, &review.elapsed, review.status));
        eprintln!("├─ review  {model}");
        if let Some(url) = url {
            eprintln!("└─ {result:<7} {url}");
        } else {
            eprintln!("└─ result  {result}");
        }
    }
    display_render(&mut display);
}

fn display_resolving(issue: u64, model: &str) {
    let mut display = display().lock().expect("display lock is poisoned");
    display.resolvers.insert(issue, ResolverNode { issue, model: model.to_owned(), started: Instant::now() });
    display_render(&mut display);
}

fn display_resolved(issue: u64, result: &str, url: Option<&str>) {
    let mut display = display().lock().expect("display lock is poisoned");
    display_clear(&mut display);
    if let Some(resolver) = display.resolvers.remove(&issue) {
        eprintln!("resolve  issue #{:<5}  time {}  model {}", resolver.issue, elapsed(resolver.started), resolver.model);
        if let Some(url) = url {
            eprintln!("└─ {result:<7} {url}");
        } else {
            eprintln!("└─ result  {result}");
        }
    }
    display_render(&mut display);
}

fn display_clock() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(1));
        display_render(&mut display().lock().expect("display lock is poisoned"));
    });
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

fn log(config: &Config, message: &str) {
    let time = Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .expect("cannot read event time");
    let line = format!("{} {message}\n", String::from_utf8_lossy(&time.stdout).trim());
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_path)
        .expect("cannot open machine log")
        .write_all(line.as_bytes())
        .expect("cannot write machine log");
}

fn event(config: &Config, message: &str) { log(config, message) }

fn trace(config: &Config, message: &str) {
    if config.debug { log(config, &format!("DEBUG {message}")) }
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

fn packet_backend(packet: &str) -> &str {
    packet.lines().find_map(|line| line.strip_prefix("backend=")).unwrap_or("unrecorded")
}

fn packet_composition(packet: &str) -> &str {
    field(packet, "cursor=")
        .split_whitespace()
        .find_map(|value| value.strip_prefix("composition:"))
        .expect("failure cursor has no composition")
}

fn packet_key(packet: &str) -> String {
    format!("{}:{}", field(packet, "id="), packet_backend(packet))
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
        && packet_backend(left) == packet_backend(right)
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
) -> (std::process::Output, bool) {
    let memory_bytes = config
        .trial_memory_mib
        .checked_mul(1024 * 1024)
        .expect("trial_memory_mib exceeds the supported address-space limit");
    let mut command = Command::new("prlimit");
    command
        .args([
            &format!("--as={memory_bytes}"),
            "--",
            "cargo",
            "run",
            "--bin",
            "recipe",
            "--",
            "test.rs",
        ])
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
    command.process_group(0);
    let mut child = command.spawn().expect("cannot start memory-bounded Recipe traversal");
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
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait().expect("cannot inspect Recipe traversal") {
            break (status, false);
        }
        if started.elapsed() >= Duration::from_secs(config.slow_cursor_seconds) {
            let terminated = Command::new("kill")
                .args(["-KILL", &format!("-{}", child.id())])
                .status()
                .expect("cannot terminate slow Recipe traversal");
            assert!(terminated.success(), "cannot terminate slow Recipe traversal process group");
            let status = child.wait().expect("cannot collect slow Recipe traversal");
            let _ = send.send(Discovery::Slow(SlowTrial {
                config: config.clone(),
                device: device.to_owned(),
                cursor,
                reproduction: reproduction.to_owned(),
                elapsed_seconds: started.elapsed().as_secs(),
            }));
            break (status, true);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    (std::process::Output {
        status,
        stdout: stdout.join().expect("Recipe traversal stdout reader failed"),
        stderr: stderr.join().expect("Recipe traversal stderr reader failed"),
    }, timed_out)
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
    if !matches!(verdict.as_str(), "new" | "comment") {
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
                identity("codex", &config.spark_model)
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
        provider: "codex",
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
            &format!("model={} repairing structured output session={session}", identity(provider, model)),
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
            &format!("model={} repairing structured output conversation={conversation}", identity("agy", model)),
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
        provider: "agy",
        model: model.to_owned(),
        effort: effort.to_owned(),
        json,
    })
}

fn opencode(config: &Config, model: &str, prompt: &str) -> std::result::Result<Decision, String> {
    let permissions = r#"{"*":"deny","read":"allow","glob":"allow","grep":"allow","list":"allow","recipe_issues_*":"allow"}"#;
    let directory = config.repository.to_str().expect("repository path is not UTF-8");
    let result = output(
        Command::new(&config.opencode_binary)
            .args([
                "run",
                "--model",
                model,
                "--agent",
                &config.opencode_agent,
                "--dir",
                directory,
                "--format",
                "json",
                "--title",
                "Recipe issue review",
                prompt,
            ])
            .env("OPENCODE_PERMISSION", permissions)
            .env("OPENCODE_CONFIG", &config.opencode_config)
            .current_dir(&config.repository),
        None,
    );
    let stream = String::from_utf8_lossy(&result.stdout);
    let response = jq_lines(&stream, "[.[] | select(.type == \"text\") | .part.text][-1]");
    if !result.status.success() && response.is_err() {
        return Err(failure(&result));
    }
    let mut json = response.and_then(object);
    if json.is_err() {
        let session = jq_lines(&stream, "[.[] | .sessionID][-1]")?;
        trace(
            config,
            &format!("model={model} repairing structured output session={session}"),
        );
        let repair = output(
            Command::new(&config.opencode_binary)
                .args([
                    "run",
                    "--session",
                    &session,
                    "--model",
                    model,
                    "--agent",
                    &config.opencode_agent,
                    "--dir",
                    directory,
                    "--format",
                    "json",
                    "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.",
                ])
                .env("OPENCODE_PERMISSION", permissions)
                .env("OPENCODE_CONFIG", &config.opencode_config)
                .current_dir(&config.repository),
            None,
        );
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq_lines(&repaired, "[.[] | select(.type == \"text\") | .part.text][-1]")
            .and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    let (provider, model) = model.split_once('/').expect("OpenCode model has no provider");
    Ok(Decision {
        provider: if provider == "openrouter" { "openrouter" } else { "opencode" },
        model: model.to_owned(),
        effort: "default".to_owned(),
        json,
    })
}

fn copilot_command(
    config: &Config,
    packet: &str,
    prompt: &str,
    session: Option<&str>,
) -> std::process::Output {
    let mut command = Command::new(&config.copilot_binary);
    command
        .arg("-C")
        .arg(&config.repository)
        .args(["-p", prompt, "--output-format", "json", "--available-tools=view,grep,glob,recipe_issues", "--allow-all-tools", "--disable-builtin-mcps", "--disallow-temp-dir", "--no-custom-instructions", "--no-ask-user", "--no-remote", "--no-remote-export", "--no-auto-update", "--log-level", "none"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(session) = session {
        command.arg(format!("--resume={session}"));
    } else {
        command.args(["--model", &config.copilot_model]);
    }
    let mut child = command.spawn().expect("cannot start GitHub Copilot");
    let mut stdout = BufReader::new(child.stdout.take().expect("GitHub Copilot stdout is absent"));
    let mut stderr = child.stderr.take().expect("GitHub Copilot stderr is absent");
    let stderr = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).expect("cannot read GitHub Copilot stderr");
        bytes
    });
    let mut bytes = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if stdout.read_line(&mut line).expect("cannot read GitHub Copilot output") == 0 {
            break;
        }
        bytes.extend_from_slice(line.as_bytes());
        if line.contains(r#""type":"session.auto_mode_resolved""#) {
            if let Ok(model) = jq(&line, ".data.chosenModel") {
                display_reviewing(packet, &identity("copilot", &model));
            }
        }
    }
    std::process::Output {
        status: child.wait().expect("cannot collect GitHub Copilot"),
        stdout: bytes,
        stderr: stderr.join().expect("GitHub Copilot stderr reader failed"),
    }
}

fn copilot(config: &Config, packet: &str, prompt: &str) -> std::result::Result<Decision, String> {
    let result = copilot_command(config, packet, prompt, None);
    let stream = String::from_utf8_lossy(&result.stdout);
    let response = jq_lines(&stream, "[.[] | select(.type == \"assistant.message\") | .data.content][-1]");
    if !result.status.success() && response.is_err() {
        return Err(failure(&result));
    }
    let model = jq_lines(&stream, "[.[] | select(.type == \"assistant.message\") | .data.model][-1]")
        .unwrap_or_else(|_| config.copilot_model.clone());
    let mut json = response.and_then(object);
    if json.is_err() {
        let session = jq_lines(&stream, "[.[] | select(.type == \"result\") | .sessionId][-1]")?;
        trace(
            config,
            &format!("model={} repairing structured output session={session}", identity("copilot", &model)),
        );
        let repair = copilot_command(
            config,
            packet,
            "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.",
            Some(&session),
        );
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq_lines(&repaired, "[.[] | select(.type == \"assistant.message\") | .data.content][-1]")
            .and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    Ok(Decision {
        provider: "copilot",
        model,
        effort: "auto".to_owned(),
        json,
    })
}

fn ollama_command(config: &Config, prompt: &str, session: Option<&str>) -> std::process::Output {
    let tools = "Read,Glob,Grep,mcp__recipe_issues__search_issues,mcp__recipe_issues__read_issue";
    let mcp = config.repository.join(".mcp.json");
    let mut command = Command::new(&config.ollama_binary);
    command.args(["launch", "claude", "--model", &config.ollama_model, "--yes", "--"]);
    command.args([
        "-p",
        prompt,
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "dontAsk",
        "--disable-slash-commands",
        "--strict-mcp-config",
        "--mcp-config",
        mcp.to_str().expect("MCP config path is not UTF-8"),
        "--tools",
        tools,
        "--allowed-tools",
        tools,
    ]);
    if let Some(session) = session {
        command.args(["--resume", session]);
    }
    output(command.current_dir(&config.repository), None)
}

fn ollama(config: &Config, prompt: &str) -> std::result::Result<Decision, String> {
    let result = ollama_command(config, prompt, None);
    let stream = String::from_utf8_lossy(&result.stdout);
    let response = jq_lines(&stream, "[.[] | select(.type == \"assistant\") | .message.content[]? | select(.type == \"text\") | .text][-1]");
    if !result.status.success() && response.is_err() {
        return Err(failure(&result));
    }
    let model = jq_lines(&stream, "[.[] | select(.type == \"system\" and .subtype == \"init\") | .model][-1]")
        .unwrap_or_else(|_| config.ollama_model.clone());
    let mut json = response.and_then(object);
    if json.is_err() {
        let session = jq_lines(&stream, "[.[] | select(.session_id != null) | .session_id][-1]")?;
        trace(config, &format!("model={} repairing structured output session={session}", identity("ollama", &model)));
        let repair = ollama_command(
            config,
            "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.",
            Some(&session),
        );
        let repaired = String::from_utf8_lossy(&repair.stdout);
        json = jq_lines(&repaired, "[.[] | select(.type == \"assistant\") | .message.content[]? | select(.type == \"text\") | .text][-1]")
            .and_then(object);
    }
    let json = json.map_err(|error| format!("{}; {error}", failure(&result)))?;
    Ok(Decision {
        provider: "ollama",
        model,
        effort: "thinking".to_owned(),
        json,
    })
}

fn classify(config: &Config, prompt: &str, packet: &str) -> Decision {
    loop {
        if let Some(_provider) = provider(&SPARK, config.review_concurrency) {
            let classifier = identity("codex", &config.spark_model);
            display_reviewing(packet, &classifier);
            match spark(config, prompt) {
                Ok(decision) => return decision,
                Err(error) => trace(
                    config,
                    &format!("model={classifier} unavailable error={error}"),
                ),
            }
            display_reviewing(packet, "queued");
        }
        if let Some(_provider) = provider(&KIMI, config.review_concurrency) {
            let classifier = identity("kimi", &config.kimi_k3_model);
            display_reviewing(packet, &classifier);
            match kimi(config, "kimi", &config.kimi_k3_model, prompt) {
                Ok(decision) => return decision,
                Err(error) => trace(
                    config,
                    &format!("model={classifier} unavailable error={error}"),
                ),
            }
            display_reviewing(packet, "queued");
        }
        for model in &config.opencode_models {
            let limit = *config.opencode_concurrency.get(model).expect("OpenCode model has no concurrency limit");
            if let Some(_provider) = model_provider(&OPENCODE, model, limit) {
                let classifier = model.to_owned();
                display_reviewing(packet, &classifier);
                match opencode(config, model, prompt) {
                    Ok(decision) => return decision,
                    Err(error) => trace(config, &format!("model={classifier} unavailable error={error}")),
                }
                display_reviewing(packet, "queued");
            }
        }
        if let Some(_provider) = provider(&COPILOT, config.review_concurrency) {
            let classifier = identity("copilot", &config.copilot_model);
            display_reviewing(packet, &classifier);
            match copilot(config, packet, prompt) {
                Ok(decision) => return decision,
                Err(error) => trace(config, &format!("model={classifier} unavailable error={error}")),
            }
            display_reviewing(packet, "queued");
        }
        if let Some(_provider) = provider(&OLLAMA, config.review_concurrency) {
            let classifier = identity("ollama", &config.ollama_model);
            display_reviewing(packet, &classifier);
            match ollama(config, prompt) {
                Ok(decision) => return decision,
                Err(error) => trace(config, &format!("model={classifier} unavailable error={error}")),
            }
            display_reviewing(packet, "queued");
        }
        if let Some(_provider) = provider(&AGY, config.review_concurrency) {
            for model in &config.agy_models {
                let classifier = identity("agy", model);
                display_reviewing(packet, &classifier);
                match agy(config, model, prompt) {
                    Ok(decision) => return decision,
                    Err(error) => trace(config, &format!("model={classifier} unavailable error={error}")),
                }
            }
            display_reviewing(packet, "queued");
        }
        trace(
            config,
            &format!(
                "all classifiers unavailable poll={}s",
                config.provider_poll_seconds
            ),
        );
        display_reviewing(packet, "waiting for provider");
        std::thread::sleep(std::time::Duration::from_secs(config.provider_poll_seconds));
    }
}

fn triage(config: &Config, instructions: &str, packet: &str) -> Review {
    let composition = packet_composition(packet);
    let schema =
        std::fs::read_to_string(&config.decision_schema).expect("cannot read decision schema");
    let prompt = format!("{instructions}\n\n## Failure packet\n\n{packet}\n\n## Required decision schema\n\n{schema}");
    let decision = classify(config, &prompt, packet);
    let classifier = identity(decision.provider, &decision.model);
    event(
        config,
        &format!(
            "CLASSIFY model={classifier} composition={composition}"
        ),
    );
    let verdict = jq(&decision.json, ".verdict").expect("validated decision lost its verdict");
    let issue = jq(&decision.json, ".issue").expect("validated decision lost its issue");
    let title = jq(&decision.json, ".title").expect("validated decision lost its title");
    let mut body = jq(&decision.json, ".body").expect("validated decision lost its body");
    if !config.publish {
        event(
            config,
            &format!(
                "DECISION model={} verdict={verdict} issue={issue} title={title}",
                classifier
            ),
        );
        display_reviewed(packet, &classifier, &verdict, None);
        return Review::Stop;
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
    event(
        config,
        &format!(
            "ISSUE model={} {action} issue=#{published_issue} url={url}",
            classifier
        ),
    );
    display_reviewed(packet, &classifier, if verdict == "new" { "issue" } else { "comment" }, Some(&url));
    Review::Done
}

fn dispatch_review(
    work: Arc<Mutex<Work>>,
    path: PathBuf,
    instructions: Arc<String>,
    packet: String,
) {
    display_queue(&packet);
    std::thread::spawn(move || {
        let current = config(&path);
        match triage(&current, &instructions, &packet) {
            Review::Done => {
                let mut state = work.lock().expect("failure queue lock is poisoned");
                let reviewed = field(&packet, "id=");
                let backend = packet_backend(&packet);
                if let Some(index) = state.packets.iter().position(|queued| {
                    field(queued, "id=") == reviewed && packet_backend(queued) == backend
                }) {
                    state.packets.remove(index);
                    persist_queue(&current.queue_path, &state.packets);
                }
            }
            Review::Stop => work.lock().expect("failure queue lock is poisoned").halted = true,
        }
    });
}

fn enqueue_review(
    work: &Arc<Mutex<Work>>,
    config: &Config,
    packet: &str,
) -> Option<usize> {
    let depth = {
        let mut state = work.lock().expect("failure queue lock is poisoned");
        if state.packets.iter().any(|queued| same_failure(queued, &packet)) {
            None
        } else {
            if packet.starts_with("kind=performance\n") {
                state.packets.push_front(packet.to_owned());
            } else {
                state.packets.push_back(packet.to_owned());
            }
            persist_queue(&config.queue_path, &state.packets);
            Some(state.packets.len())
        }
    };
    if depth.is_none() { display_queue(packet) }
    depth
}

fn resolver_issue(config: &Config, active: &BTreeSet<u64>) -> std::result::Result<Option<(u64, String)>, String> {
    let issues = output(
        Command::new("gh")
            .args(["issue", "list", "--state", "open", "--limit", "100", "--json", "number,url", "--jq", ".[] | [.number, .url] | @tsv"])
            .current_dir(&config.repository),
        None,
    );
    if !issues.status.success() {
        return Err(failure(&issues));
    }
    let pull_requests = output(
        Command::new("gh")
            .args(["pr", "list", "--state", "open", "--limit", "100", "--json", "closingIssuesReferences", "--jq", ".[].closingIssuesReferences[].number"])
            .current_dir(&config.repository),
        None,
    );
    if !pull_requests.status.success() {
        return Err(failure(&pull_requests));
    }
    let claimed = String::from_utf8_lossy(&pull_requests.stdout)
        .lines()
        .filter_map(|number| number.parse::<u64>().ok())
        .collect::<BTreeSet<_>>();
    Ok(String::from_utf8_lossy(&issues.stdout).lines().find_map(|line| {
        let (number, url) = line.split_once('\t')?;
        let number = number.parse::<u64>().ok()?;
        (!claimed.contains(&number) && !active.contains(&number)).then(|| (number, url.to_owned()))
    }))
}

struct ResolverClaim {
    active: Arc<Mutex<BTreeSet<u64>>>,
    number: u64,
    url: String,
}

impl Drop for ResolverClaim {
    fn drop(&mut self) {
        self.active.lock().expect("resolver claim lock is poisoned").remove(&self.number);
    }
}

fn resolver_claim(config: &Config, active: &Arc<Mutex<BTreeSet<u64>>>) -> std::result::Result<Option<ResolverClaim>, String> {
    let mut issues = active.lock().expect("resolver claim lock is poisoned");
    let Some((number, url)) = resolver_issue(config, &issues)? else { return Ok(None) };
    issues.insert(number);
    drop(issues);
    Ok(Some(ResolverClaim { active: Arc::clone(active), number, url }))
}

fn resolver_pr(config: &Config, issue: u64) -> std::result::Result<Option<String>, String> {
    let pull_requests = output(
        Command::new("gh")
            .args(["pr", "list", "--state", "open", "--limit", "100", "--json", "closingIssuesReferences,url", "--jq", ".[] | [.url, ([.closingIssuesReferences[].number] | map(tostring) | join(\",\"))] | @tsv"])
            .current_dir(&config.repository),
        None,
    );
    if !pull_requests.status.success() {
        return Err(failure(&pull_requests));
    }
    Ok(String::from_utf8_lossy(&pull_requests.stdout).lines().find_map(|line| {
        let (url, issues) = line.split_once('\t')?;
        issues.split(',').any(|number| number.parse::<u64>() == Ok(issue)).then(|| url.to_owned())
    }))
}

fn resolver_loop(path: PathBuf, active: Arc<Mutex<BTreeSet<u64>>>) {
    loop {
        let current = config(&path);
        let issue = match resolver_claim(&current, &active) {
            Ok(Some(issue)) => issue,
            Ok(None) => {
                std::thread::sleep(Duration::from_secs(current.resolver_poll_seconds));
                continue;
            }
            Err(error) => {
                trace(&current, &format!("resolver issue selection failed error={error}"));
                std::thread::sleep(Duration::from_secs(current.resolver_poll_seconds));
                continue;
            }
        };
        let model = format!("claude/{}-{}", current.resolver_model.strip_prefix("claude-").unwrap_or(&current.resolver_model), current.resolver_effort);
        let goal = format!(r#"/goal Read GitHub issue #{number} and every comment in full, then resolve that one issue completely and create one pull request that closes it.

Use unrestricted tools and make every required repository and GitHub change yourself. Never edit the current issue-machine worktree. Work in one separate Git worktree under {root}, based on the latest origin/{base}. Reuse coherent existing work for issue #{number} if it exists. Reproduce the exact public failure first, identify the earliest cause in current origin/{base}, implement the root fix without a fallback or parallel implementation, and validate the exact public Recipe path end to end. Test reachable edge cases one at a time through the same public entrypoint. Preserve unrelated user changes.

Commit and push the coherent fix, then create one pull request targeting {base}. The pull request body must contain `Fixes #{number}`, the exact reproduction, the root cause, the change, and measured end-to-end evidence. Do not stop until the pull request exists and its URL is visible. Target issue: {url}"#, number = issue.number, root = current.resolver_worktree_root.display(), base = current.resolver_base, url = issue.url);
        display_resolving(issue.number, &model);
        event(&current, &format!("RESOLVE model={model} issue=#{} url={}", issue.number, issue.url));
        let memory = format!("MemoryMax={}M", current.resolver_memory_mib);
        let unit = format!("recipe-resolve-{}", issue.number);
        let result = output(
            Command::new("systemd-run")
                .args([
                    "--user",
                    "--wait",
                    "--pipe",
                    "--collect",
                    "--quiet",
                    "--unit",
                    &unit,
                    "--working-directory",
                    current.repository.to_str().expect("repository path is not UTF-8"),
                    "--property",
                    &memory,
                    "--",
                    current.claude_binary.to_str().expect("Claude binary path is not UTF-8"),
                    "-p",
                    &goal,
                    "--model",
                    &current.resolver_model,
                    "--effort",
                    &current.resolver_effort,
                    "--dangerously-skip-permissions",
                    "--output-format",
                    "json",
                    "--name",
                    &format!("Resolve Recipe issue #{}", issue.number),
                ])
                .current_dir(&current.repository),
            None,
        );
        if !result.status.success() {
            event(&current, &format!("RESOLVE model={model} issue=#{} failed error={}", issue.number, failure(&result)));
            display_resolved(issue.number, "FAIL", None);
            return;
        }
        match resolver_pr(&current, issue.number) {
            Ok(Some(url)) => {
                event(&current, &format!("PR model={model} issue=#{} url={url}", issue.number));
                display_resolved(issue.number, "PR", Some(&url));
            }
            Ok(None) => {
                event(&current, &format!("RESOLVE model={model} issue=#{} failed error=goal completed without an open pull request", issue.number));
                display_resolved(issue.number, "FAIL", None);
                return;
            }
            Err(error) => {
                event(&current, &format!("RESOLVE model={model} issue=#{} failed error={error}", issue.number));
                display_resolved(issue.number, "FAIL", None);
                return;
            }
        }
    }
}

fn main() {
    let directory = std::env::current_exe()
        .expect("cannot locate machine executable")
        .parent()
        .expect("machine has no directory")
        .to_owned();
    let path = directory.join("machine.toml");
    let initial = config(&path);
    display_clock();
    let pending = std::fs::read_to_string(&initial.queue_path)
        .map(|text| queued(&text))
        .unwrap_or_default();
    let work = Arc::new(Mutex::new(Work { packets: pending, halted: false }));
    let instructions = Arc::new(
        std::fs::read_to_string(directory.join("triage.md")).expect("cannot read triage.md"),
    );
    work
        .lock()
        .expect("failure queue lock is poisoned")
        .packets
        .iter()
        .cloned()
        .for_each(|packet| dispatch_review(Arc::clone(&work), path.clone(), Arc::clone(&instructions), packet));
    if initial.resolver_enabled {
        let active = Arc::new(Mutex::new(BTreeSet::new()));
        for _ in 0..initial.resolver_concurrency {
            let resolver_path = path.clone();
            let resolver_active = Arc::clone(&active);
            std::thread::spawn(move || resolver_loop(resolver_path, resolver_active));
        }
    }
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
            if worker_send.send(Discovery::Start { device: device.clone(), cursor: start }).is_err() {
                break;
            }
            let (result, timed_out) = trial(&current, &device, start, &reproduction, &worker_send);
            if worker_send
                .send(Discovery::Complete(Trial {
                    config: current,
                    device: device.clone(),
                    cursor: start,
                    reproduction,
                    output: result,
                    timed_out,
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
            Discovery::Start { device, cursor } => {
                display_start(&device, cursor);
                continue;
            }
            Discovery::Slow(trial) => {
                let packet = slow_packet(&trial);
                let composition = packet_composition(&packet);
                if let Some(depth) = enqueue_review(&work, &trial.config, &packet) {
                    event(&trial.config, &format!("SLOW device={} cursor={} composition={composition} elapsed={}s depth={depth}", trial.device, trial.cursor, trial.elapsed_seconds));
                    dispatch_review(Arc::clone(&work), path.clone(), Arc::clone(&instructions), packet);
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
        let signal = (!trial.timed_out).then(|| termination_signal(trial.output.status)).flatten();
        assert!(
            trial.timed_out || trial.output.status.success() || signal.is_some(),
            "Recipe traversal failed outside a failure packet on {} with status {:?}: {}",
            trial.device,
            trial.output.status,
            failure(&trial.output)
        );
        let mut failures = if trial.timed_out {
            Vec::new()
        } else {
            packets(&text)
                .into_iter()
                .map(|packet| packet_for_device(packet, &trial.device))
                .collect::<Vec<_>>()
        };
        if let Some(signal) = signal {
            failures.push(packet_for_device(&crash_packet(&trial, &text, signal), &trial.device));
        }
        for (offset, composition) in text
            .lines()
            .filter_map(|line| {
                if trial.timed_out {
                    return None;
                }
                let (composition, _) = line.strip_prefix("composition ")?.split_once(':')?;
                composition.parse::<u64>().ok().map(|_| composition)
            })
            .enumerate()
        {
            let analyzed = trial.cursor + offset as u64;
            let failed = failures
                .iter()
                .any(|packet| packet_cursor(packet) == analyzed);
            let status = if failed { "FAIL" } else { "PASS" };
            event(
                &trial.config,
                &format!(
                    "{status} device={} cursor={analyzed} composition={composition}",
                    trial.device
                ),
            );
        }
        for packet in &failures {
            let packet_cursor = packet_cursor(packet);
            let composition = packet_composition(packet);
            if let Some(depth) = enqueue_review(&work, &trial.config, packet) {
                event(&trial.config, &format!("QUEUE device={} cursor={packet_cursor} composition={composition} depth={depth}", trial.device));
                dispatch_review(Arc::clone(&work), path.clone(), Arc::clone(&instructions), packet.clone());
            }
        }
        display_finish(&trial.device, !failures.is_empty());
        let next = if trial.timed_out || signal.is_some() {
            trial.cursor + trial.config.compositions_per_batch
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
}
