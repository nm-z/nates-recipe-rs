mod cluster;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::{fs::MetadataExt, process::{CommandExt, ExitStatusExt}};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering}, mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RED: &str = "\x1b[38;2;255;92;122m";
const YELLOW: &str = "\x1b[38;2;229;192;123m";
const GREEN: &str = "\x1b[38;2;86;214;169m";
const BLUE: &str = "\x1b[38;2;97;175;239m";
const TIME: &str = "\x1b[38;2;255;194;0m";
const RESET: &str = "\x1b[0m";
const SHELL_SIGNAL_OFFSET: i32 = 128;
const TM_YEAR_BASE: i32 = 1900;
const TM_MONTH_BASE: i32 = 1;
const RLIMIT_NOFILE: i32 = 7;

#[repr(C)]
struct Limit {
    current: u64,
    maximum: u64,
}

#[repr(C)]
struct Calendar {
    second: i32,
    minute: i32,
    hour: i32,
    day: i32,
    month: i32,
    year: i32,
    weekday: i32,
    yearday: i32,
    daylight: i32,
    offset: i64,
    zone: *const std::ffi::c_char,
}

unsafe extern "C" {
    fn localtime_r(time: *const i64, result: *mut Calendar) -> *mut Calendar;
    fn getrlimit(resource: i32, limit: *mut Limit) -> i32;
    fn setrlimit(resource: i32, limit: *const Limit) -> i32;
}

fn timestamp(second: u64) -> String {
    let time = second as i64;
    let mut calendar = std::mem::MaybeUninit::<Calendar>::uninit();
    assert!(!unsafe { localtime_r(&time, calendar.as_mut_ptr()) }.is_null(), "cannot read local time");
    let calendar = unsafe { calendar.assume_init() };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", calendar.year + TM_YEAR_BASE, calendar.month + TM_MONTH_BASE, calendar.day, calendar.hour, calendar.minute, calendar.second)
}

#[derive(Clone)]
struct Config {
    repository: PathBuf,
    log_path: PathBuf,
    queue_path: PathBuf,
    trial_directory: PathBuf,
    native_cache_directory: PathBuf,
    decision_schema: PathBuf,
    triage_path: PathBuf,
    spark_model: String,
    spark_effort: String,
    spark_issue_reader: PathBuf,
    kimi_binary: PathBuf,
    kimi_k3_model: String,
    kimi_agent: PathBuf,
    kimi_skills: PathBuf,
    agy_binary: PathBuf,
    agy_models: Vec<String>,
    opencode_binary: PathBuf,
    opencode_config: PathBuf,
    opencode_server: String,
    opencode_request_seconds: u64,
    opencode_server_start_seconds: u64,
    opencode_server_poll_milliseconds: u64,
    opencode_status_poll_milliseconds: u64,
    opencode_models: Vec<String>,
    opencode_agent: String,
    copilot_binary: PathBuf,
    copilot_model: String,
    claude_binary: PathBuf,
    ollama_binary: PathBuf,
    ollama_model: String,
    ollama_session_name: String,
    ollama_disable_nonessential_traffic: String,
    resolver_enabled: bool,
    resolver_model: String,
    resolver_effort: String,
    resolver_base: String,
    resolver_worktree_root: PathBuf,
    resolver_poll_seconds: u64,
    resolver_concurrency: usize,
    resolver_reset_unix: u64,
    resolver_memory_mib: u64,
    resolver_memory_budget_mib: u64,
    resolver_environment: String,
    review_poll_milliseconds: u64,
    review_recovery: f64,
    review_overload_divisor: usize,
    issue_history_limit: usize,
    slow_cursor_seconds: u64,
    discovery_devices: Vec<String>,
    filesystem_stat_binary: PathBuf,
    nvidia_smi_binary: PathBuf,
    timeout_binary: PathBuf,
    hardware_query_seconds: u64,
    amd_drm_root: PathBuf,
    cluster_coordinator: bool,
    cluster_address: std::net::SocketAddr,
    cluster_listen: std::net::SocketAddr,
    cluster_state_path: PathBuf,
    cluster_secret: String,
    cluster_node_id: String,
    cluster_node_limit: usize,
    cluster_node_timeout_seconds: u64,
    cursor_lease_seconds: u64,
    cursor_renew_milliseconds: u64,
    cluster_request_seconds: u64,
    sha256_binary: PathBuf,
    git_binary: PathBuf,
    cluster_workload_paths: Vec<String>,
    memory_setpoint: f64,
    memory_maximum: f64,
    vram_setpoint: f64,
    vram_maximum: f64,
    cpu_setpoint: f64,
    gpu_setpoint: f64,
    disk_maximum: f64,
    queue_setpoint: usize,
    controller_sample_milliseconds: u64,
    controller_slew_workers_per_second: f64,
    cpu_ultimate_gain: f64,
    cpu_ultimate_period_seconds: f64,
    gpu_ultimate_gain: f64,
    gpu_ultimate_period_seconds: f64,
    queue_ultimate_gain: f64,
    queue_ultimate_period_seconds: f64,
    memory_ultimate_gain: f64,
    memory_ultimate_period_seconds: f64,
    vram_ultimate_gain: f64,
    vram_ultimate_period_seconds: f64,
    trial_limit: usize,
    device_worker_floor: usize,
    review_limit: usize,
    review_worker_floor: usize,
    disk_headroom: f64,
    convergence_tolerance: f64,
    queue_tolerance: usize,
    trial_grace_seconds: u64,
    review_grace_seconds: u64,
    model_cooldown_seconds: u64,
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

fn number<T: std::str::FromStr>(text: &str, name: &str) -> T {
    value(text, name).parse().unwrap_or_else(|_| panic!("machine.toml has invalid {name}"))
}


// Configuration is loaded once per controller period, so every numeric field is rejected
// here rather than producing a nonfinite command later.
fn ratio(text: &str, name: &str) -> f64 {
    let value: f64 = number(text, name);
    assert!(value.is_finite() && (0.0..=1.0).contains(&value), "machine.toml has invalid {name}");
    value
}

fn rate(text: &str, name: &str) -> f64 {
    let value: f64 = number(text, name);
    assert!(value.is_finite() && value > 0.0, "machine.toml has invalid {name}");
    value
}

fn positive<T: std::str::FromStr + PartialOrd + Default>(text: &str, name: &str) -> T {
    let value: T = number(text, name);
    assert!(value > T::default(), "machine.toml has invalid {name}");
    value
}

fn values(text: &str, name: &str) -> Vec<String> {
    value(text, name)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn discovered_devices(nvidia_topology_root: &Path, amd_topology_root: &Path) -> Vec<String> {
    let mut devices = vec!["cpu".to_owned()];
    if amd_topology_root.is_dir() {
        let mut nodes = std::fs::read_dir(amd_topology_root)
            .expect("cannot read AMD topology")
            .map(|entry| entry.expect("cannot read AMD topology entry").path())
            .collect::<Vec<_>>();
        nodes.sort();
        let count = nodes
            .iter()
            .filter(|node| std::fs::read_to_string(node.join("gpu_id")).ok().and_then(|value| value.trim().parse::<u64>().ok()).is_some_and(|gpu| gpu != 0))
            .count();
        devices.extend((0..count).map(|index| format!("amd{index}")));
    }
    if nvidia_topology_root.is_dir() {
        let mut nvidia = std::fs::read_dir(nvidia_topology_root)
            .expect("cannot read NVIDIA topology")
            .map(|entry| entry.expect("cannot read NVIDIA topology entry").path())
            .map(|device| {
                let information = std::fs::read_to_string(device.join("information")).expect("cannot read NVIDIA device information");
                information
                    .lines()
                    .find_map(|line| line.strip_prefix("Device Minor:").map(str::trim))
                    .expect("NVIDIA device has no minor")
                    .parse::<usize>()
                    .expect("NVIDIA device minor is invalid")
            })
            .collect::<Vec<_>>();
        nvidia.sort_unstable();
        devices.extend(nvidia.into_iter().map(|index| format!("nv{index}")));
    }
    assert_eq!(devices.iter().collect::<BTreeSet<_>>().len(), devices.len(), "hardware discovery returned a duplicate device");
    devices
}

fn config(path: &Path) -> Config {
    let text = std::fs::read_to_string(path).expect("cannot read machine.toml");
    let opencode_models = values(&text, "opencode_models");
    let nvidia_smi_binary = PathBuf::from(value(&text, "nvidia_smi_binary"));
    let nvidia_topology_root = PathBuf::from(value(&text, "nvidia_topology_root"));
    let amd_topology_root = PathBuf::from(value(&text, "amd_topology_root"));
    let discovery_devices = discovered_devices(&nvidia_topology_root, &amd_topology_root);
    let configuration = Config {
        repository: value(&text, "repository").into(),
        log_path: value(&text, "log_path").into(),
        queue_path: value(&text, "queue_path").into(),
        trial_directory: value(&text, "trial_directory").into(),
        native_cache_directory: value(&text, "native_cache_directory").into(),
        decision_schema: value(&text, "decision_schema").into(),
        triage_path: value(&text, "triage_path").into(),
        spark_model: value(&text, "spark_model"),
        spark_effort: value(&text, "spark_effort"),
        spark_issue_reader: value(&text, "spark_issue_reader").into(),
        kimi_binary: value(&text, "kimi_binary").into(),
        kimi_k3_model: value(&text, "kimi_k3_model"),
        kimi_agent: value(&text, "kimi_agent").into(),
        kimi_skills: value(&text, "kimi_skills").into(),
        agy_binary: value(&text, "agy_binary").into(),
        agy_models: values(&text, "agy_models"),
        opencode_binary: value(&text, "opencode_binary").into(),
        opencode_config: value(&text, "opencode_config").into(),
        opencode_server: value(&text, "opencode_server"),
        opencode_request_seconds: number(&text, "opencode_request_seconds"),
        opencode_server_start_seconds: number(&text, "opencode_server_start_seconds"),
        opencode_server_poll_milliseconds: number(&text, "opencode_server_poll_milliseconds"),
        opencode_status_poll_milliseconds: number(&text, "opencode_status_poll_milliseconds"),
        opencode_models,
        opencode_agent: value(&text, "opencode_agent"),
        copilot_binary: value(&text, "copilot_binary").into(),
        copilot_model: value(&text, "copilot_model"),
        claude_binary: value(&text, "claude_binary").into(),
        ollama_binary: value(&text, "ollama_binary").into(),
        ollama_model: value(&text, "ollama_model"),
        ollama_session_name: value(&text, "ollama_session_name"),
        ollama_disable_nonessential_traffic: value(&text, "ollama_disable_nonessential_traffic"),
        resolver_enabled: number(&text, "resolver_enabled"),
        resolver_model: value(&text, "resolver_model"),
        resolver_effort: value(&text, "resolver_effort"),
        resolver_base: value(&text, "resolver_base"),
        resolver_worktree_root: value(&text, "resolver_worktree_root").into(),
        resolver_poll_seconds: number(&text, "resolver_poll_seconds"),
        resolver_concurrency: number(&text, "resolver_concurrency"),
        resolver_reset_unix: number(&text, "resolver_reset_unix"),
        resolver_memory_mib: number(&text, "resolver_memory_mib"),
        resolver_memory_budget_mib: number(&text, "resolver_memory_budget_mib"),
        resolver_environment: value(&text, "resolver_environment"),
        review_poll_milliseconds: number(&text, "review_poll_milliseconds"),
        review_recovery: ratio(&text, "review_recovery"),
        review_overload_divisor: positive(&text, "review_overload_divisor"),
        issue_history_limit: number(&text, "issue_history_limit"),
        slow_cursor_seconds: number(&text, "slow_cursor_seconds"),
        discovery_devices,
        filesystem_stat_binary: value(&text, "filesystem_stat_binary").into(),
        nvidia_smi_binary,
        timeout_binary: value(&text, "timeout_binary").into(),
        hardware_query_seconds: positive(&text, "hardware_query_seconds"),
        amd_drm_root: value(&text, "amd_drm_root").into(),
        cluster_coordinator: number(&text, "cluster_coordinator"),
        cluster_address: value(&text, "cluster_address").parse().expect("cluster_address is invalid"),
        cluster_listen: value(&text, "cluster_listen").parse().expect("cluster_listen is invalid"),
        cluster_state_path: value(&text, "cluster_state_path").into(),
        cluster_secret: value(&text, "cluster_secret"),
        cluster_node_id: value(&text, "cluster_node_id"),
        cluster_node_limit: positive(&text, "cluster_node_limit"),
        cluster_node_timeout_seconds: positive(&text, "cluster_node_timeout_seconds"),
        cursor_lease_seconds: positive(&text, "cursor_lease_seconds"),
        cursor_renew_milliseconds: positive(&text, "cursor_renew_milliseconds"),
        cluster_request_seconds: positive(&text, "cluster_request_seconds"),
        sha256_binary: value(&text, "sha256_binary").into(),
        git_binary: value(&text, "git_binary").into(),
        cluster_workload_paths: values(&text, "cluster_workload_paths"),
        memory_setpoint: ratio(&text, "memory_setpoint"),
        memory_maximum: ratio(&text, "memory_maximum"),
        vram_setpoint: ratio(&text, "vram_setpoint"),
        vram_maximum: ratio(&text, "vram_maximum"),
        cpu_setpoint: ratio(&text, "cpu_setpoint"),
        gpu_setpoint: ratio(&text, "gpu_setpoint"),
        disk_maximum: ratio(&text, "disk_maximum"),
        queue_setpoint: number(&text, "queue_setpoint"),
        controller_sample_milliseconds: positive(&text, "controller_sample_milliseconds"),
        controller_slew_workers_per_second: rate(&text, "controller_slew_workers_per_second"),
        cpu_ultimate_gain: rate(&text, "cpu_ultimate_gain"),
        cpu_ultimate_period_seconds: rate(&text, "cpu_ultimate_period_seconds"),
        gpu_ultimate_gain: rate(&text, "gpu_ultimate_gain"),
        gpu_ultimate_period_seconds: rate(&text, "gpu_ultimate_period_seconds"),
        queue_ultimate_gain: rate(&text, "queue_ultimate_gain"),
        queue_ultimate_period_seconds: rate(&text, "queue_ultimate_period_seconds"),
        memory_ultimate_gain: rate(&text, "memory_ultimate_gain"),
        memory_ultimate_period_seconds: rate(&text, "memory_ultimate_period_seconds"),
        vram_ultimate_gain: rate(&text, "vram_ultimate_gain"),
        vram_ultimate_period_seconds: rate(&text, "vram_ultimate_period_seconds"),
        trial_limit: positive(&text, "trial_limit"),
        device_worker_floor: positive(&text, "device_worker_floor"),
        review_limit: positive(&text, "review_limit"),
        review_worker_floor: positive(&text, "review_worker_floor"),
        disk_headroom: ratio(&text, "disk_headroom"),
        convergence_tolerance: ratio(&text, "convergence_tolerance"),
        queue_tolerance: number(&text, "queue_tolerance"),
        trial_grace_seconds: positive(&text, "trial_grace_seconds"),
        review_grace_seconds: positive(&text, "review_grace_seconds"),
        model_cooldown_seconds: positive(&text, "model_cooldown_seconds"),
        seed: number(&text, "seed"),
        cursor: number(&text, "cursor"),
        compositions_per_batch: number(&text, "compositions_per_batch"),
        batches: number(&text, "batches"),
        publish: number(&text, "publish"),
        debug: number(&text, "debug"),
    };
    assert!(configuration.memory_setpoint <= configuration.memory_maximum, "memory_setpoint exceeds memory_maximum");
    assert!(configuration.vram_setpoint <= configuration.vram_maximum, "vram_setpoint exceeds vram_maximum");
    assert!(configuration.disk_headroom < configuration.disk_maximum, "disk_headroom is not below disk_maximum");
    assert!(configuration.cursor_renew_milliseconds < configuration.cursor_lease_seconds * 1000, "cursor renewal must precede lease expiry");
    configuration
}

struct Work {
    packets: VecDeque<String>,
    active: BTreeSet<String>,
    reviewing: BTreeSet<String>,
    ready: BTreeSet<String>,
    halted: bool,
}

struct Admission {
    trials: BTreeMap<String, AtomicI64>,
    reviews: AtomicI64,
    live: Mutex<LiveWorkers>,
    next_worker: AtomicUsize,
}

// A retiring worker keeps its slot until it has actually released its resources, so a
// lowered command can never be satisfied by accounting alone.
#[derive(Default)]
struct LiveWorkers {
    trials: BTreeMap<usize, LiveTrial>,
    reviews: BTreeMap<usize, LiveReview>,
}

struct LiveTrial {
    device: String,
    started: Instant,
    retire: Arc<AtomicBool>,
}

struct LiveReview {
    retired: Option<Instant>,
    started: Instant,
    cancel: Cancel,
}

// The handle a review worker publishes so its real OpenCode session can be aborted.
#[derive(Clone)]
struct Cancel {
    retire: Arc<AtomicBool>,
    session: Arc<Mutex<Option<String>>>,
}

impl Cancel {
    fn new() -> Self {
        Self { retire: Arc::new(AtomicBool::new(false)), session: Arc::new(Mutex::new(None)) }
    }

    fn retiring(&self) -> bool { self.retire.load(Ordering::Relaxed) }

    fn opened(&self, session: &str) {
        *self.session.lock().expect("review cancellation lock is poisoned") = Some(session.to_owned());
    }

    fn session(&self) -> Option<String> {
        self.session.lock().expect("review cancellation lock is poisoned").clone()
    }
}

// There is no declared endpoint capacity. The controller's lease command is the authority
// and the endpoint only derates it by what transport failures and completions actually
// show, so measured behaviour governs concurrency instead of a configured starting count.
// A starting count is a ceiling wearing measurement's clothes: with a floor of one lease
// that only grows on completion, an endpoint that has completed nothing can never earn the
// concurrency it needs to complete anything.
struct ReviewEndpoint {
    derating: Mutex<f64>,
    recovery: f64,
    overload_divisor: f64,
}

impl ReviewEndpoint {
    fn new(config: &Config) -> Self {
        Self {
            derating: Mutex::new(1.0),
            recovery: config.review_recovery,
            overload_divisor: config.review_overload_divisor as f64,
        }
    }

    // One lease survives any derating while the controller is asking for work, because an
    // endpoint carrying nothing produces no measurement to recover from.
    fn admissible(&self, command: usize) -> usize {
        if command == 0 { return 0 }
        let derating = *self.derating.lock().expect("review endpoint lock is poisoned");
        (((command as f64) * derating).round() as usize).max(1)
    }

    fn derating(&self) -> f64 { *self.derating.lock().expect("review endpoint lock is poisoned") }

    // Provider unavailability is endpoint state: a model still inside its measured reset
    // window serves no lease this period and the next period observes it again.
    fn claim(self: &Arc<Self>, config: &Config) -> Option<ModelLease> {
        let first = OPENCODE_NEXT.fetch_add(1, Ordering::Relaxed);
        (0..config.opencode_models.len()).find_map(|offset| {
            let model = &config.opencode_models[(first + offset) % config.opencode_models.len()];
            available(model).then(|| ModelLease { endpoint: Arc::clone(self), model: model.clone() })
        })
    }

    fn reachable(&self, config: &Config) -> usize {
        config.opencode_models.iter().filter(|model| available(model)).count()
    }

    fn completed(&self) {
        let mut derating = self.derating.lock().expect("review endpoint lock is poisoned");
        *derating = (*derating + self.recovery).min(1.0);
    }

    fn overloaded(&self) {
        let mut derating = self.derating.lock().expect("review endpoint lock is poisoned");
        *derating /= self.overload_divisor;
    }
}

struct ModelLease {
    endpoint: Arc<ReviewEndpoint>,
    model: String,
}

#[derive(Clone, Copy)]
struct Gains {
    proportional: f64,
    integral: f64,
    derivative: f64,
}

const ZN_PROPORTIONAL: f64 = 0.60;
const ZN_INTEGRAL: f64 = 1.20;
const ZN_DERIVATIVE: f64 = 3.0 / 40.0;

impl Gains {
    fn ziegler_nichols(name: &str, ultimate_gain: f64, ultimate_period_seconds: f64) -> Self {
        assert!(ultimate_gain.is_finite() && ultimate_gain > 0.0, "{name}_ultimate_gain must be finite and positive");
        assert!(ultimate_period_seconds.is_finite() && ultimate_period_seconds > 0.0, "{name}_ultimate_period_seconds must be finite and positive");
        Self {
            proportional: ZN_PROPORTIONAL * ultimate_gain,
            integral: ZN_INTEGRAL * ultimate_gain / ultimate_period_seconds,
            derivative: ZN_DERIVATIVE * ultimate_gain * ultimate_period_seconds,
        }
    }
}

// One measured process variable producing one nonnegative worker command.
struct Loop {
    gains: Gains,
    integral: f64,
    previous_error: Option<f64>,
    ceiling: bool,
}

impl Loop {
    fn new(gains: Gains) -> Self {
        Self { gains, integral: 0.0, previous_error: None, ceiling: false }
    }

    // Conditional-integration anti-windup: the integral is frozen only when advancing it
    // would push further into the saturated direction. A gate that freezes it in both
    // directions strands the integral at its pre-saturation value, and the command then
    // snaps back to that stale value as soon as the error changes sign, which is a limit
    // cycle rather than convergence.
    fn command(&mut self, error: f64, seconds: f64, upper: f64) -> f64 {
        assert!(error.is_finite(), "controller error is not measurable");
        assert!(seconds.is_finite() && seconds > 0.0, "controller period is not measurable");
        assert!(upper.is_finite() && upper >= 0.0, "controller command limit is invalid");
        let derivative = self.previous_error.map_or(0.0, |previous| (error - previous) / seconds);
        self.previous_error = Some(error);
        let held = self.gains.proportional * error + self.gains.integral * self.integral + self.gains.derivative * derivative;
        let winding = (held >= upper && error > 0.0) || (held <= 0.0 && error < 0.0);
        if !winding { self.integral += error * seconds }
        let command = self.gains.proportional * error + self.gains.integral * self.integral + self.gains.derivative * derivative;
        self.ceiling = command >= upper;
        command.clamp(0.0, upper)
    }
}

// The measured plant, sampled once per controller period.
struct Plant {
    memory: f64,
    vram: f64,
    cpu: f64,
    gpu: f64,
    disk: f64,
    queue: usize,
}

// Live worker counts observed at the same instant as the plant.
struct Occupancy {
    trials: BTreeMap<String, usize>,
    reviews: usize,
}

#[derive(Clone, Default)]
struct Control {
    memory: f64,
    vram: f64,
    cpu: f64,
    gpu: f64,
    disk: f64,
    queue: usize,
    memory_setpoint: f64,
    vram_setpoint: f64,
    cpu_setpoint: f64,
    gpu_setpoint: f64,
    disk_maximum: f64,
    queue_setpoint: usize,
    actuators: Vec<(String, i64, usize)>,
    residual: String,
}

impl Control {
    fn summary(&self) -> String {
        let actuators = self.actuators.iter().map(|(name, command, live)| format!("{name}={command}/{live}")).collect::<Vec<_>>().join(" ");
        format!(
            "CONTROL {actuators} memory={:.3}/{:.3} vram={:.3}/{:.3} cpu={:.3}/{:.3} gpu={:.3}/{:.3} disk={:.3}<{:.3} ready={}/{} residual={}",
            self.memory, self.memory_setpoint,
            self.vram, self.vram_setpoint,
            self.cpu, self.cpu_setpoint,
            self.gpu, self.gpu_setpoint,
            self.disk, self.disk_maximum,
            self.queue, self.queue_setpoint,
            self.residual,
        )
    }
}

// The intermediate quantities of one control step, kept so the residual report names the
// measured limit that actually held a command back.
struct Demand {
    gpu_request: f64,
    vram_request: f64,
    queue_request: f64,
    cpu_budget: f64,
    memory_budget: f64,
    reviews: f64,
    trial_budget: f64,
    accelerator_total: f64,
    disk_blocked: bool,
}

// Three actuators serve five measured resources, so the coupling is explicit: the GPU and
// its memory are private to accelerator trials, the ready queue is private to review
// leases, and host compute, host memory and storage are shared budgets that cap the total.
struct Regulator {
    memory: Loop,
    vram: Loop,
    cpu: Loop,
    gpu: Loop,
    queue: Loop,
    trials: BTreeMap<String, f64>,
    reviews: f64,
}

impl Regulator {
    fn new(config: &Config) -> Self {
        let mut regulator = Self {
            memory: Loop::new(Gains::ziegler_nichols("memory", config.memory_ultimate_gain, config.memory_ultimate_period_seconds)),
            vram: Loop::new(Gains::ziegler_nichols("vram", config.vram_ultimate_gain, config.vram_ultimate_period_seconds)),
            cpu: Loop::new(Gains::ziegler_nichols("cpu", config.cpu_ultimate_gain, config.cpu_ultimate_period_seconds)),
            gpu: Loop::new(Gains::ziegler_nichols("gpu", config.gpu_ultimate_gain, config.gpu_ultimate_period_seconds)),
            queue: Loop::new(Gains::ziegler_nichols("queue", config.queue_ultimate_gain, config.queue_ultimate_period_seconds)),
            trials: BTreeMap::new(),
            reviews: 0.0,
        };
        regulator.trials = config.discovery_devices.iter().map(|device| (device.clone(), 0.0)).collect();
        regulator
    }

    fn retune(&mut self, config: &Config) {
        self.memory.gains = Gains::ziegler_nichols("memory", config.memory_ultimate_gain, config.memory_ultimate_period_seconds);
        self.vram.gains = Gains::ziegler_nichols("vram", config.vram_ultimate_gain, config.vram_ultimate_period_seconds);
        self.cpu.gains = Gains::ziegler_nichols("cpu", config.cpu_ultimate_gain, config.cpu_ultimate_period_seconds);
        self.gpu.gains = Gains::ziegler_nichols("gpu", config.gpu_ultimate_gain, config.gpu_ultimate_period_seconds);
        self.queue.gains = Gains::ziegler_nichols("queue", config.queue_ultimate_gain, config.queue_ultimate_period_seconds);
    }

    fn step(&mut self, config: &Config, plant: &Plant, occupancy: &Occupancy, seconds: f64) -> Control {
        self.retune(config);
        let accelerators = config.discovery_devices.iter().filter(|device| device.as_str() != "cpu").count();
        let device_limit = config.trial_limit as f64;
        let trial_ceiling = device_limit * config.discovery_devices.len() as f64;
        let review_ceiling = config.review_limit as f64;
        // Private objectives: only accelerator trials move the GPU and its memory, and only
        // review leases drain the ready queue.
        let gpu_request = (accelerators != 0).then(|| self.gpu.command(config.gpu_setpoint - plant.gpu, seconds, device_limit * accelerators as f64)).unwrap_or(0.0);
        let vram_request = (accelerators != 0).then(|| self.vram.command(config.vram_setpoint - plant.vram, seconds, device_limit * accelerators as f64)).unwrap_or(0.0);
        let queue_request = self.queue.command(plant.queue as f64 - config.queue_setpoint as f64, seconds, review_ceiling);
        // Shared budgets: host compute is consumed by every trial, host memory by every worker.
        let cpu_budget = self.cpu.command(config.cpu_setpoint - plant.cpu, seconds, trial_ceiling);
        let memory_budget = self.memory.command(config.memory_setpoint - plant.memory, seconds, trial_ceiling + review_ceiling);
        // Trials are the only actuator that can move host compute, the GPU or its memory,
        // so they claim the shared host-memory budget first and review leases absorb the
        // remainder. The reverse order lets a deep ready queue saturate the budget and hold
        // every trial at zero, which leaves cpu, gpu and vram permanently unreachable.
        let review_floor = if plant.queue > config.queue_setpoint { config.review_worker_floor as f64 } else { 0.0 };
        let trial_budget = cpu_budget.min((memory_budget - review_floor).max(0.0));
        let reviews = queue_request
            .min((memory_budget - trial_budget).max(0.0))
            .max(review_floor.min(memory_budget));
        // Accelerator trials are the only actuator that can serve the GPU setpoints, so they
        // take the shared budget first and host trials absorb the remainder.
        let device_floor = config.device_worker_floor as f64;
        let accelerator_floor = device_floor * accelerators as f64;
        let accelerator_ceiling = (trial_budget - device_floor).max(0.0);
        let accelerator_total = gpu_request
            .min(vram_request)
            .min(accelerator_ceiling)
            .max(accelerator_floor.min(accelerator_ceiling));
        let host_total = trial_budget - accelerator_total;
        let slew = config.controller_slew_workers_per_second * seconds;
        assert!(slew.is_finite() && slew > 0.0, "controller_slew_workers_per_second must be finite and positive");
        // Storage never recovers on its own, so new disk-producing work stops one headroom
        // before the configured maximum and retires once the maximum is reached.
        let disk_blocked = plant.disk >= config.disk_maximum - config.disk_headroom;
        let disk_exceeded = plant.disk >= config.disk_maximum;
        let mut actuators = Vec::new();
        for device in &config.discovery_devices {
            let live = *occupancy.trials.get(device).expect("occupancy lost a discovery device");
            let requested = if device == "cpu" { host_total } else { accelerator_total / accelerators as f64 };
            let previous = *self.trials.get(device).expect("regulator lost a discovery device");
            let mut command = requested.clamp(previous - slew, previous + slew).max(0.0);
            if plant.memory >= config.memory_maximum { command = command.min(retire_below(live)) }
            if device != "cpu" && plant.vram >= config.vram_maximum { command = command.min(retire_below(live)) }
            if disk_blocked { command = command.min(live as f64) }
            if disk_exceeded { command = command.min(retire_below(live)) }
            self.trials.insert(device.clone(), command);
            actuators.push((device.clone(), command.round() as i64, live));
        }
        let mut review_command = reviews.clamp(self.reviews - slew, self.reviews + slew).max(0.0);
        if plant.memory >= config.memory_maximum { review_command = review_command.min(retire_below(occupancy.reviews)) }
        self.reviews = review_command;
        actuators.push(("review".to_owned(), review_command.round() as i64, occupancy.reviews));
        let residual = self.residual(config, plant, &Demand { gpu_request, vram_request, queue_request, cpu_budget, memory_budget, reviews, trial_budget, accelerator_total, disk_blocked });
        Control {
            memory: plant.memory,
            vram: plant.vram,
            cpu: plant.cpu,
            gpu: plant.gpu,
            disk: plant.disk,
            queue: plant.queue,
            memory_setpoint: config.memory_setpoint,
            vram_setpoint: config.vram_setpoint,
            cpu_setpoint: config.cpu_setpoint,
            gpu_setpoint: config.gpu_setpoint,
            disk_maximum: config.disk_maximum,
            queue_setpoint: config.queue_setpoint,
            actuators,
            residual,
        }
    }

    // A setpoint outside tolerance is reported as infeasible only when the actuator that
    // would close it is already held at an admissible bound by another measured limit.
    fn residual(&self, config: &Config, plant: &Plant, demand: &Demand) -> String {
        let mut residuals = Vec::new();
        let mut record = |name: &str, error: f64, tolerance: f64, binding: Option<&str>| {
            if let Some(binding) = binding.filter(|_| error.abs() > tolerance) {
                residuals.push(format!("{name}:{error:+.3}<{binding}"));
            }
        };
        record("memory", config.memory_setpoint - plant.memory, config.convergence_tolerance,
            (demand.trial_budget + demand.reviews < demand.memory_budget).then(|| if self.cpu.ceiling { "trial-limit" } else { "cpu-budget" }));
        record("cpu", config.cpu_setpoint - plant.cpu, config.convergence_tolerance,
            if demand.trial_budget < demand.cpu_budget { Some("memory-budget") } else if self.cpu.ceiling { Some("trial-limit") } else { None });
        record("gpu", config.gpu_setpoint - plant.gpu, config.convergence_tolerance,
            if demand.accelerator_total < demand.gpu_request {
                Some(if demand.vram_request < demand.gpu_request { "vram-request" } else { "trial-budget" })
            } else if self.gpu.ceiling { Some("trial-limit") } else { None });
        record("vram", config.vram_setpoint - plant.vram, config.convergence_tolerance,
            if demand.accelerator_total < demand.vram_request { Some("trial-budget") } else if self.vram.ceiling { Some("trial-limit") } else { None });
        record("queue", config.queue_setpoint as f64 - plant.queue as f64, config.queue_tolerance as f64,
            if demand.reviews < demand.queue_request { Some("memory-budget") } else if self.queue.ceiling { Some("review-limit") } else { None });
        if demand.disk_blocked { residuals.push(format!("disk:{:+.3}<headroom", config.disk_maximum - plant.disk)) }
        if residuals.is_empty() { "none".to_owned() } else { residuals.join(",") }
    }
}

fn retire_below(live: usize) -> f64 { (live as f64 - 1.0).max(0.0) }

impl Admission {
    fn new(config: &Config) -> Self {
        Self {
            trials: config.discovery_devices.iter().cloned().map(|device| (device, AtomicI64::new(0))).collect(),
            reviews: AtomicI64::new(0),
            live: Mutex::new(LiveWorkers::default()),
            next_worker: AtomicUsize::new(0),
        }
    }

    fn occupancy(&self) -> Occupancy {
        let live = self.live.lock().expect("live worker lock is poisoned");
        let mut trials = self.trials.keys().cloned().map(|device| (device, 0)).collect::<BTreeMap<_, _>>();
        for trial in live.trials.values() {
            *trials.get_mut(&trial.device).expect("a live trial has no discovery device") += 1;
        }
        Occupancy { trials, reviews: live.reviews.len() }
    }

    fn apply(&self, control: &Control) {
        for (name, command, _) in &control.actuators {
            if name == "review" {
                self.reviews.store(*command, Ordering::Relaxed);
            } else {
                self.trials.get(name).expect("admission lost a discovery device").store(*command, Ordering::Relaxed);
            }
        }
    }

    fn command(&self, actuator: &str) -> usize {
        let command = if actuator == "review" { &self.reviews } else { self.trials.get(actuator).expect("admission lost a discovery device") };
        command.load(Ordering::Relaxed).max(0) as usize
    }

    // A newly admitted worker occupies its slot before its thread starts, so the next
    // control period already observes it.
    fn admit_trial(&self, device: &str) -> (usize, Arc<AtomicBool>) {
        let worker = self.next_worker.fetch_add(1, Ordering::Relaxed);
        let retire = Arc::new(AtomicBool::new(false));
        self.live.lock().expect("live worker lock is poisoned").trials.insert(worker, LiveTrial { device: device.to_owned(), started: Instant::now(), retire: Arc::clone(&retire) });
        (worker, retire)
    }

    fn release_trial(&self, worker: usize) {
        self.live.lock().expect("live worker lock is poisoned").trials.remove(&worker).expect("a released trial was never admitted");
    }

    fn admit_review(&self) -> (usize, Cancel) {
        let worker = self.next_worker.fetch_add(1, Ordering::Relaxed);
        let cancel = Cancel::new();
        self.live.lock().expect("live worker lock is poisoned").reviews.insert(worker, LiveReview { retired: None, started: Instant::now(), cancel: cancel.clone() });
        (worker, cancel)
    }

    fn release_review(&self, worker: usize) {
        self.live.lock().expect("live worker lock is poisoned").reviews.remove(&worker).expect("a released review was never admitted");
    }

    // Retirement removes the newest workers first, so the least completed work is lost.
    fn retire_trials(&self, device: &str, excess: usize) {
        let live = self.live.lock().expect("live worker lock is poisoned");
        let mut candidates = live.trials.values().filter(|trial| trial.device == device && !trial.retire.load(Ordering::Relaxed)).collect::<Vec<_>>();
        candidates.sort_by_key(|trial| std::cmp::Reverse(trial.started));
        for trial in candidates.iter().take(excess) { trial.retire.store(true, Ordering::Relaxed) }
    }

    fn retire_reviews(&self, excess: usize) -> Vec<Cancel> {
        let mut live = self.live.lock().expect("live worker lock is poisoned");
        let mut candidates = live.reviews.iter().filter(|(_, review)| review.retired.is_none()).map(|(worker, review)| (*worker, review.started)).collect::<Vec<_>>();
        candidates.sort_by_key(|(_, started)| std::cmp::Reverse(*started));
        let now = Instant::now();
        candidates.iter().take(excess).map(|(worker, _)| {
            let review = live.reviews.get_mut(worker).expect("a retiring review disappeared");
            review.retired = Some(now);
            review.cancel.retire.store(true, Ordering::Relaxed);
            review.cancel.clone()
        }).collect()
    }

    // A retiring worker keeps its slot until its session is really gone, so an overrun is
    // reported and keeps applying back pressure instead of being counted as released.
    fn audit_reviews(&self, config: &Config) -> usize {
        let live = self.live.lock().expect("live worker lock is poisoned");
        let overrun = live.reviews.values().filter(|review| review.retired.is_some_and(|retired| retired.elapsed() >= Duration::from_secs(config.review_grace_seconds))).count();
        if overrun != 0 { event(config, &format!("RETIRE review overrun={overrun} grace={}s", config.review_grace_seconds)) }
        overrun
    }
}

// Releasing the worker slot through a guard keeps the live count exact even when a worker
// thread unwinds.
struct TrialSlot {
    admission: Arc<Admission>,
    worker: usize,
}

impl Drop for TrialSlot {
    fn drop(&mut self) { self.admission.release_trial(self.worker) }
}

struct ReviewSlot {
    admission: Arc<Admission>,
    worker: usize,
}

impl Drop for ReviewSlot {
    fn drop(&mut self) { self.admission.release_review(self.worker) }
}

fn control_lines(control: &Control, width: usize) -> Vec<String> {
    let commands = control.actuators.iter().map(|(name, command, live)| format!("{name} {command}/{live}")).collect::<Vec<_>>().join("  ");
    vec![
        fit(&format!("├─ command   {commands}"), width),
        fit(&format!(
            "├─ resource  memory {:.3}/{:.3}  vram {:.3}/{:.3}  cpu {:.3}/{:.3}  gpu {:.3}/{:.3}  disk {:.3}<{:.3}  ready {}/{}",
            control.memory, control.memory_setpoint,
            control.vram, control.vram_setpoint,
            control.cpu, control.cpu_setpoint,
            control.gpu, control.gpu_setpoint,
            control.disk, control.disk_maximum,
            control.queue, control.queue_setpoint,
        ), width),
        fit(&format!("└─ residual  {}", control.residual), width),
    ]
}

fn display_control(control: Control) {
    display().lock().expect("display lock is poisoned").control = Some(control);
}

struct CursorLease {
    client: Arc<cluster::Client>,
    lease: cluster::Lease,
    stop: Option<mpsc::Sender<()>>,
    heartbeat: Option<std::thread::JoinHandle<()>>,
}

impl CursorLease {
    fn new(client: Arc<cluster::Client>, lease: cluster::Lease, renewal: Duration) -> Self {
        let (stop, receive) = mpsc::channel();
        let heartbeat_client = Arc::clone(&client);
        let heartbeat = std::thread::spawn(move || {
            while receive.recv_timeout(renewal).is_err() {
                heartbeat_client.renew(lease);
            }
        });
        Self { client, lease, stop: Some(stop), heartbeat: Some(heartbeat) }
    }

    fn stop(&mut self) {
        if let Some(stop) = self.stop.take() { let _ = stop.send(()); }
        if let Some(heartbeat) = self.heartbeat.take() { heartbeat.join().expect("cursor heartbeat failed"); }
    }

    fn complete(mut self, next: u64, device: &str, packets: &[String]) -> u64 {
        self.stop();
        self.client.complete(self.lease, next, device, packets)
    }

    fn release(mut self) {
        self.stop();
        self.client.release(self.lease)
    }
}

impl Drop for CursorLease {
    fn drop(&mut self) { self.stop() }
}

struct TrialDirectory(PathBuf);

impl TrialDirectory {
    fn create(config: &Config, worker: usize, cursor: u64) -> Self {
        let path = config.trial_directory.join(format!("recipe-trial-{worker}-{cursor}"));
        std::fs::create_dir(&path).expect("cannot create trial directory");
        Self(path)
    }

    fn reproduction(&self) -> PathBuf { self.0.join("reproduction.rs") }
}

impl Drop for TrialDirectory {
    fn drop(&mut self) {
        if self.0.exists() { std::fs::remove_dir_all(&self.0).expect("cannot remove trial directory") }
    }
}

struct Trial {
    config: Arc<Config>,
    worker: usize,
    device: String,
    cursor: u64,
    _directory: TrialDirectory,
    reproduction: PathBuf,
    output: std::process::Output,
    elapsed: Duration,
    timed_out: bool,
    crash: Option<(i32, i32)>,
    replay: Option<String>,
    lease: Option<CursorLease>,
}

// A retired trial reports no failure and no crash packet: its work simply did not happen.
enum Termination {
    Completed,
    Slow,
    Retired,
}

enum Discovery {
    Start { worker: usize, device: String, cursor: u64 },
    Cluster(cluster::Completion),
    Complete(Trial),
    Retired { worker: usize, device: String, cursor: u64, replay: Option<String>, lease: Option<CursorLease> },
}

enum Review {
    Retired,
    Done,
    Stop,
    Unavailable,
    Transport,
}

struct Active {
    device: String,
    cursor: u64,
    started: Instant,
    completed: Option<Duration>,
}

struct ResolverNode {
    issue: u64,
    model: String,
    started: Instant,
}

struct ReviewNode {
    model: Option<String>,
    started: Option<Instant>,
}

#[derive(Default)]
struct Display {
    active: BTreeMap<usize, Active>,
    reviews: BTreeMap<String, ReviewNode>,
    resolvers: BTreeMap<u64, ResolverNode>,
    history: Vec<String>,
    control: Option<Control>,
    rows: usize,
}

static DISPLAY: OnceLock<Mutex<Display>> = OnceLock::new();
static OPENCODE_NEXT: AtomicUsize = AtomicUsize::new(0);
static OPENCODE_SESSION: Mutex<()> = Mutex::new(());
static PUBLICATION: Mutex<()> = Mutex::new(());
static CONFIGURATION: Mutex<()> = Mutex::new(());
static LOGGER: OnceLock<mpsc::Sender<String>> = OnceLock::new();
static COOLDOWNS: OnceLock<Mutex<BTreeMap<String, Instant>>> = OnceLock::new();

fn display() -> &'static Mutex<Display> { DISPLAY.get_or_init(|| Mutex::new(Display::default())) }

fn identity(provider: &str, model: &str) -> String { format!("{provider}/{model}") }
fn epoch() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).expect("system time precedes Unix epoch").as_secs() }

fn memory_used() -> f64 {
    let text = std::fs::read_to_string("/proc/meminfo").expect("cannot read /proc/meminfo");
    let kib = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_else(|| panic!("/proc/meminfo has no {name}"))
    };
    1.0 - kib("MemAvailable:") / kib("MemTotal:")
}

fn cpu_time() -> (u64, u64) {
    let text = std::fs::read_to_string("/proc/stat").expect("cannot read /proc/stat");
    let values = text.lines().next().expect("/proc/stat has no aggregate CPU row").split_whitespace().skip(1).map(|value| value.parse::<u64>().expect("/proc/stat has an invalid CPU value")).collect::<Vec<_>>();
    assert!(values.len() >= 8, "/proc/stat aggregate CPU row is incomplete");
    (values[..8].iter().sum(), values[3] + values[4])
}

fn accelerator_used(config: &Config) -> (f64, f64) {
    let mut gpu = 0.0_f64;
    let mut vram = 0.0_f64;
    for device in config.discovery_devices.iter().filter_map(|device| device.strip_prefix("nv")) {
        let result = output(Command::new(&config.timeout_binary).arg(config.hardware_query_seconds.to_string()).arg(&config.nvidia_smi_binary).args([
            "-i",
            device,
            "--query-gpu=utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ]), None);
        assert!(result.status.success(), "cannot inspect NVIDIA device nv{device}: {}", failure(&result));
        let text = String::from_utf8(result.stdout).expect("NVIDIA resource output is not UTF-8");
        let values = text.trim().split(',').map(|value| value.trim().parse::<f64>().expect("NVIDIA resource value is invalid")).collect::<Vec<_>>();
        assert_eq!(values.len(), 3, "NVIDIA resource output has the wrong width");
        gpu = gpu.max(values[0] / 100.0);
        vram = vram.max(values[1] / values[2]);
    }
    if config.discovery_devices.iter().any(|device| device.starts_with("amd")) {
        let mut measured = 0_usize;
        for entry in std::fs::read_dir(&config.amd_drm_root).expect("cannot read AMD DRM resources") {
            let path = entry.expect("cannot read AMD DRM resource entry").path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else { continue };
            if !name.strip_prefix("card").is_some_and(|index| !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())) { continue }
            let device = path.join("device");
            let busy = device.join("gpu_busy_percent");
            let used = device.join("mem_info_vram_used");
            let total = device.join("mem_info_vram_total");
            if !(busy.is_file() && used.is_file() && total.is_file()) { continue }
            let number = |path: &Path| std::fs::read_to_string(path).expect("cannot read AMD resource").trim().parse::<f64>().expect("AMD resource value is invalid");
            gpu = gpu.max(number(&busy) / 100.0);
            vram = vram.max(number(&used) / number(&total));
            measured += 1;
        }
        assert!(measured != 0, "AMD devices were discovered without measurable DRM resources");
    }
    (gpu, vram)
}

fn disk_used(config: &Config) -> f64 {
    let result = output(Command::new(&config.filesystem_stat_binary).args(["-f", "-c", "%b %a"]).arg(&config.repository), None);
    assert!(result.status.success(), "cannot inspect repository filesystem: {}", failure(&result));
    let values = String::from_utf8(result.stdout).expect("filesystem resource output is not UTF-8").split_whitespace().map(|value| value.parse::<f64>().expect("filesystem resource value is invalid")).collect::<Vec<_>>();
    assert_eq!(values.len(), 2, "filesystem resource output has the wrong width");
    1.0 - values[1] / values[0]
}

// Only packets explicitly held Ready and not already claimed count as review queue depth.
fn ready_packets(state: &Work) -> usize {
    state.packets.iter().filter(|packet| {
        let key = packet_key(packet);
        state.ready.contains(&key) && !state.active.contains(&key)
    }).count()
}

fn regulate(config: Arc<Config>, work: Arc<Mutex<Work>>, admission: Arc<Admission>) {
    std::thread::spawn(move || {
        let mut regulator = Regulator::new(&config);
        let mut previous_sample = Instant::now();
        let mut previous_cpu = cpu_time();
        loop {
            std::thread::sleep(Duration::from_millis(config.controller_sample_milliseconds));
            let now = Instant::now();
            let seconds = now.duration_since(previous_sample).as_secs_f64();
            previous_sample = now;
            let cpu = cpu_time();
            let period_ticks = cpu.0.checked_sub(previous_cpu.0).expect("aggregate CPU time went backwards");
            let idle_ticks = cpu.1.checked_sub(previous_cpu.1).expect("idle CPU time went backwards");
            assert!(period_ticks != 0, "the controller period observed no CPU time");
            previous_cpu = cpu;
            let (gpu, vram) = accelerator_used(&config);
            let plant = Plant {
                memory: memory_used(),
                vram,
                cpu: 1.0 - idle_ticks as f64 / period_ticks as f64,
                gpu,
                disk: disk_used(&config),
                queue: ready_packets(&work.lock().expect("failure queue lock is poisoned")),
            };
            let occupancy = admission.occupancy();
            let control = regulator.step(&config, &plant, &occupancy, seconds);
            admission.apply(&control);
            event(&config, &control.summary());
            display_control(control);
        }
    });
}

fn reset_after(error: &str) -> Option<Duration> {
    let lower = error.to_ascii_lowercase();
    if let Some(tail) = lower.split_once("\"resetsat\":").map(|(_, tail)| tail) {
        let reset_epoch = tail.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u64>().ok()?;
        return reset_epoch.checked_sub(epoch()).map(Duration::from_secs);
    }
    let tail = ["resets in ", "refreshes in "].into_iter().find_map(|marker| lower.split_once(marker).map(|(_, tail)| tail))?;
    let mut seconds = 0_u64;
    for token in tail.split_whitespace() {
        let token = token.trim_matches(|character: char| !character.is_ascii_alphanumeric());
        let Some((value, unit)) = token.split_at_checked(token.len().saturating_sub(1)) else { break };
        let Ok(value) = value.parse::<u64>() else { break };
        seconds = seconds.saturating_add(value.saturating_mul(match unit { "d" => 86_400, "h" => 3_600, "m" => 60, "s" => 1, _ => break }));
    }
    (seconds != 0).then(|| Duration::from_secs(seconds))
}

fn usage_exhausted(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    ["usage limit", "session limit", "quota", "out of credits", "credit balance", "rate limit", "rate-limit", "rate_limit"].iter().any(|message| error.contains(message))
}

fn available(model: &str) -> bool {
    let cooldowns = COOLDOWNS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut cooldowns = cooldowns.lock().expect("cooldown lock is poisoned");
    match cooldowns.get(model).copied() {
        Some(reset) if Instant::now() >= reset => { cooldowns.remove(model); true }
        Some(_) => false,
        None => true,
    }
}

// An unmeasured reset is not an infinite one. Without a bounded re-measure interval a
// single limit message that carries no reset time removes the model for the whole machine
// run, and the measured endpoint capacity never recovers from one transient refusal.
fn cooldown(config: &Config, model: &str, error: &str) -> Option<u64> {
    let measured = reset_after(error);
    let reset = measured.unwrap_or_else(|| Duration::from_secs(config.model_cooldown_seconds));
    COOLDOWNS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().expect("cooldown lock is poisoned").insert(model.to_owned(), Instant::now() + reset);
    trace(config, &format!("model={model} unavailable reset_seconds={} measured={} error={error}", reset.as_secs(), measured.is_some()));
    measured.map(|duration| epoch().saturating_add(duration.as_secs()))
}

struct OpenCodeServer(Child);

impl Drop for OpenCodeServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn opencode_server(config: &Config) -> OpenCodeServer {
    let address = config.opencode_server.strip_prefix("http://").expect("opencode_server must use http://");
    let (host, port) = address.split_once(':').expect("opencode_server must contain host:port");
    let mut child = Command::new(&config.opencode_binary)
        .args(["serve", "--hostname", host, "--port", port, "--log-level", "ERROR"])
        .env("OPENCODE_CONFIG", &config.opencode_config)
        .current_dir(&config.repository)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("cannot start OpenCode server");
    let stderr = child.stderr.take().expect("OpenCode server stderr is absent");
    let log_config = config.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            log(&log_config, &format!("OPENCODE {line}"));
        }
    });
    let deadline = Instant::now() + Duration::from_secs(config.opencode_server_start_seconds);
    while std::net::TcpStream::connect(address).is_err() {
        assert!(Instant::now() < deadline, "OpenCode server did not start at {}", config.opencode_server);
        std::thread::sleep(Duration::from_millis(config.opencode_server_poll_milliseconds));
    }
    OpenCodeServer(child)
}

fn opencode_request(config: &Config, method: &str, path: &str, body: &str) -> std::result::Result<String, String> {
    let timeout = config.opencode_request_seconds.to_string();
    let result = output(
        Command::new("curl").args([
            "--fail-with-body", "--silent", "--show-error", "--max-time", &timeout,
            "--request", method, "--header", "Content-Type: application/json", "--data-binary", body,
            &format!("{}{path}", config.opencode_server),
        ]),
        None,
    );
    result.status.success().then(|| String::from_utf8_lossy(&result.stdout).into_owned()).ok_or_else(|| failure(&result))
}

fn opencode_turn(config: &Config, model: &str, prompt: &str, session: &str, cancel: &Cancel) -> std::result::Result<Option<String>, String> {
    let (provider, model) = model.split_once('/').expect("OpenCode model has no provider");
    let payload = output(Command::new("jq").args(["-cn", "--arg", "provider", provider, "--arg", "model", model, "--arg", "agent", &config.opencode_agent, "--arg", "prompt", prompt, r#"{model:{providerID:$provider,modelID:$model},agent:$agent,parts:[{type:"text",text:$prompt}]}"#]), None);
    if !payload.status.success() { return Err(failure(&payload)) }
    opencode_request(config, "POST", &format!("/session/{session}/prompt_async"), &String::from_utf8_lossy(&payload.stdout))?;
    loop {
        if cancel.retiring() {
            opencode_request(config, "POST", &format!("/session/{session}/abort"), "")?;
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(config.opencode_status_poll_milliseconds));
        let sessions = opencode_request(config, "GET", "/session/status", "")?;
        let filter = format!(r#".["{session}"] // {{"type":"idle"}}"#);
        let state = jq(&sessions, &filter)?;
        match jq(&state, ".type")?.as_str() {
            "busy" => continue,
            "retry" => {
                let message = jq(&state, ".message").unwrap_or_else(|_| "OpenCode usage is unavailable".to_owned());
                opencode_request(config, "POST", &format!("/session/{session}/abort"), "")?;
                return Err(message);
            }
            "idle" => {
                let messages = opencode_request(config, "GET", &format!("/session/{session}/message"), "")?;
                if let Ok(response) = jq(&messages, r#"[.[] | select(.info.role == "assistant") | .parts[]? | select(.type == "text") | .text][-1]"#) { return Ok(Some(response)) }
                if let Ok(message) = jq(&messages, r#"[.[] | select(.info.role == "assistant" and .info.error != null) | .info.error.data.message][-1]"#) {
                    let reset = jq(&messages, r#"[.[] | select(.info.role == "assistant" and .info.error != null) | .info.error.data.responseHeaders["x-ratelimit-reset"]][-1]"#).ok().and_then(|value| value.parse::<u64>().ok()).map(|next| {
                        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("system time precedes Unix epoch").as_millis() as u64;
                        next.saturating_sub(now).saturating_add(999) / 1000
                    });
                    if let Some(reset) = reset { return Err(format!("usage limit; refreshes in {reset}s; {message}")) }
                    if usage_exhausted(&message) { return Err(message) }
                    return Err(message);
                }
                if jq(&messages, r#"[.[] | select(.info.role == "assistant") | .info.id][-1]"#).is_ok() { return Err("OpenCode returned a completed assistant turn with no response".to_owned()) }
            }
            _ => continue,
        }
    }
}

fn display_line(frame: &mut String, display: &mut Display, line: &str) {
    frame.push_str(line);
    frame.push('\n');
    display.rows += 1;
}

fn elapsed(started: Instant) -> String {
    duration(started.elapsed())
}

fn duration(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs_f64();
    if seconds < 1.0 {
        format!("{:>7.3} ms", seconds * 1000.0)
    } else {
        format!("{seconds:>8.4} s")
    }
}

fn active_elapsed(active: &Active) -> String {
    active.completed.map_or_else(|| elapsed(active.started), duration)
}

fn fit(value: &str, width: usize) -> String {
    if value.chars().count() <= width { return value.to_owned() }
    if width <= 2 { value.chars().take(width).collect() } else { format!("{}..", value.chars().take(width - 2).collect::<String>()) }
}

fn review_line(prefix: &str, model: &str, count: usize, elapsed: &str, width: usize) -> String {
    let suffix = format!("  time {elapsed}");
    let model = format!("{count}x {model}");
    format!("{prefix}{}{suffix}", fit(&model, width.saturating_sub(prefix.chars().count() + suffix.chars().count())))
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
    let mut frame = String::new();
    display.rows = 0;
    let (width, height) = pane_size();
    let mut trials = BTreeMap::<String, usize>::new();
    for active in display.active.values() { *trials.entry(active.device.clone()).or_default() += 1 }
    let mut reviews = BTreeMap::<String, BTreeMap<String, (usize, Instant)>>::new();
    for review in display.reviews.values() {
        let Some(identity) = review.model.as_deref() else { continue };
        let Some(started) = review.started else { continue };
        let (provider, model) = identity.split_once('/').unwrap_or((identity, identity));
        reviews.entry(provider.to_owned()).or_default().entry(model.to_owned())
            .and_modify(|(count, oldest)| { *count += 1; if started < *oldest { *oldest = started } })
            .or_insert((1, started));
    }
    let queued = display.reviews.values().filter(|review| review.model.is_none()).count();
    let resolving = display.resolvers.values().map(|resolver| {
        format!("{:<28}  issue #{:<5}  time {}", resolver.model, resolver.issue, elapsed(resolver.started))
    }).collect::<Vec<_>>();
    if let Some(control) = display.control.clone() {
        display_line(&mut frame, display, &format!("{BLUE}control{RESET}"));
        for line in control_lines(&control, width) { display_line(&mut frame, display, &line) }
    }
    if queued != 0 {
        display_line(&mut frame, display, &format!("{YELLOW}queued{RESET}"));
        display_line(&mut frame, display, &format!("└─ {queued} reviews"));
    }
    if !trials.is_empty() || !reviews.is_empty() || !resolving.is_empty() {
        display_line(&mut frame, display, &format!("{BLUE}live{RESET}"));
    }
    if !trials.is_empty() {
        let branch = if reviews.is_empty() && resolving.is_empty() { "└─" } else { "├─" };
        display_line(&mut frame, display, &format!("{branch} trials"));
        for (index, (device, count)) in trials.iter().enumerate() {
            let branch = if index + 1 == trials.len() { "└─" } else { "├─" };
            let trunk = if reviews.is_empty() && resolving.is_empty() { "   " } else { "│  " };
            display_line(&mut frame, display, &format!("{trunk}{branch} {device} selected {count}"));
        }
    }
    if !reviews.is_empty() {
        let review_branch = if resolving.is_empty() { "└─" } else { "├─" };
        let review_trunk = if resolving.is_empty() { "   " } else { "│  " };
        display_line(&mut frame, display, &format!("{review_branch} review"));
        let provider_count = reviews.len();
        for (provider_index, (provider, models)) in reviews.into_iter().enumerate() {
            let provider_last = provider_index + 1 == provider_count;
            let provider_branch = if provider_last { "└─" } else { "├─" };
            if models.len() == 1 {
                let (model, (count, started)) = models.iter().next().expect("single-model provider is empty");
                display_line(&mut frame, display, &review_line(&format!("{review_trunk}{provider_branch} "), &format!("{provider}/{model}"), *count, &elapsed(*started), width));
                continue;
            }
            display_line(&mut frame, display, &format!("{review_trunk}{provider_branch} {provider}"));
            for (model_index, (model, (count, started))) in models.iter().enumerate() {
                let branch = if model_index + 1 == models.len() { "└─" } else { "├─" };
                let trunk = if provider_last { format!("{review_trunk}   ") } else { format!("{review_trunk}│  ") };
                display_line(&mut frame, display, &review_line(&format!("{trunk}{branch} "), model, *count, &elapsed(*started), width));
            }
        }
    }
    if !resolving.is_empty() {
        display_line(&mut frame, display, "└─ resolve");
        for (index, resolver) in resolving.iter().enumerate() {
            let branch = if index + 1 == resolving.len() { "└─" } else { "├─" };
            display_line(&mut frame, display, &fit(&format!("   {branch} {resolver}"), width));
        }
    }
    let content_height = height.saturating_sub(1);
    let live = frame.lines().take(content_height).collect::<Vec<_>>();
    let history_rows = content_height.saturating_sub(live.len());
    let history_start = display.history.len().saturating_sub(history_rows);
    let mut screen = String::from("\x1b[H\x1b[2J\x1b[3J");
    for line in &display.history[history_start..] { screen.push_str(line); screen.push('\n') }
    for line in live { screen.push_str(line); screen.push('\n') }
    if display.history.len() > content_height {
        display.history.drain(..display.history.len() - content_height);
    }
    let mut stderr = std::io::stderr().lock();
    stderr.write_all(screen.as_bytes()).expect("cannot draw machine status");
    stderr.flush().expect("cannot flush machine status");
}

fn display_start(worker: usize, device: String, cursor: u64) {
    let mut display = display().lock().expect("display lock is poisoned");
    display.active.insert(worker, Active { device, cursor, started: Instant::now(), completed: None });
}

fn display_complete(worker: usize, elapsed: Duration) {
    if let Some(active) = display().lock().expect("display lock is poisoned").active.get_mut(&worker) {
        active.completed = Some(elapsed);
    }
}

fn display_finish(worker: usize, status: &str) {
    let mut display = display().lock().expect("display lock is poisoned");
    if let Some(active) = display.active.remove(&worker) {
        display.history.push(trial_line(&active.device, active.cursor, &active_elapsed(&active), status));
    }
}

fn display_queue(packet: &str) {
    let key = packet_key(packet);
    let mut display = display().lock().expect("display lock is poisoned");
    display.reviews.entry(key).or_insert(ReviewNode {
        model: None,
        started: None,
    });
}

fn display_drop(packet: &str) {
    display().lock().expect("display lock is poisoned").reviews.remove(&packet_key(packet));
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
}

fn display_reviewed(packet: &str, model: &str, result: &str, url: Option<&str>) {
    let key = packet_key(packet);
    let mut display = display().lock().expect("display lock is poisoned");
    if display.reviews.remove(&key).is_some() {
        display.history.push(format!("├─ review  {model}"));
        if let Some(url) = url {
            display.history.push(format!("└─ {result:<7} {url}"));
        } else {
            display.history.push(format!("└─ result  {result}"));
        }
    }
}

fn display_resolved(issue: u64, result: &str, url: Option<&str>) {
    let mut display = display().lock().expect("display lock is poisoned");
    let Some(resolver) = display.resolvers.remove(&issue) else { return };
    display.history.push(format!("resolve  issue #{:<5}  time {}  model {}", resolver.issue, elapsed(resolver.started), resolver.model));
    display.history.push(url.map_or_else(|| format!("└─ result  {result}"), |url| format!("└─ {result:<7} {url}")));
}

fn display_clock(model: String) {
    if std::env::var_os("TMUX_PANE").is_none() { return }
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let running = resolver_units().expect("cannot inspect active resolver units");
        let mut display = display().lock().expect("display lock is poisoned");
        display.resolvers.retain(|issue, _| running.contains_key(issue));
        for (issue, started) in running { display.resolvers.entry(issue).or_insert_with(|| ResolverNode { issue, model: model.clone(), started }); }
        display_render(&mut display);
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

fn setting(path: &Path, key: &str, value: impl std::fmt::Display) {
    let _configuration = CONFIGURATION.lock().expect("configuration lock is poisoned");
    let text = std::fs::read_to_string(path).expect("cannot read machine.toml");
    let updated = text
        .lines()
        .map(|line| {
            if line
                .split_once('=')
                .is_some_and(|(candidate, _)| candidate.trim() == key)
            {
                format!("{key} = {value}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let temporary = path.with_extension("next");
    std::fs::write(&temporary, updated).expect("cannot write machine configuration");
    std::fs::rename(temporary, path).expect("cannot publish machine configuration");
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
    let sender = LOGGER.get_or_init(|| {
        let path = config.log_path.clone();
        let (send, receive) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).expect("cannot open machine log");
            let (mut second, mut cached_time) = (u64::MAX, String::new());
            for message in receive {
                let now = epoch();
                if now != second {
                    second = now;
                    cached_time = timestamp(now);
                }
                writeln!(file, "{cached_time} {message}").expect("cannot write machine log");
            }
        });
        send
    });
    sender.send(message.to_owned()).expect("machine log writer stopped");
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

fn prepare_storage(config: &Config) {
    std::fs::create_dir_all(&config.trial_directory).expect("cannot create trial root");
    std::fs::create_dir_all(&config.native_cache_directory).expect("cannot create native cache root");
    let repository = config.repository.canonicalize().expect("cannot locate repository");
    let trial = config.trial_directory.canonicalize().expect("cannot locate trial root");
    let cache = config.native_cache_directory.canonicalize().expect("cannot locate native cache root");
    assert!(trial.starts_with(repository.join("target")), "trial_directory must be below the repository target directory");
    assert!(cache.starts_with(repository.join("target")), "native_cache_directory must be below the repository target directory");
    assert_ne!(trial, cache, "trial and native cache directories must differ");
    assert_eq!(trial.metadata().expect("cannot inspect trial root").dev(), cache.metadata().expect("cannot inspect native cache root").dev(), "trial and native cache directories must share one filesystem");
    for entry in std::fs::read_dir(&config.trial_directory).expect("cannot read trial root") {
        let entry = entry.expect("cannot read reproduction entry");
        let valid = entry.file_type().is_ok_and(|kind| kind.is_dir())
            && entry.file_name().to_str().and_then(|name| name.strip_prefix("recipe-trial-")).and_then(|name| name.split_once('-')).is_some_and(|(worker, cursor)| worker.parse::<usize>().is_ok() && cursor.parse::<u64>().is_ok());
        if valid { std::fs::remove_dir_all(entry.path()).expect("cannot remove interrupted trial") }
    }
}

fn trial(
    config: &Config,
    device: &str,
    cursor: u64,
    directory: &Path,
    reproduction: &Path,
    retire: &AtomicBool,
) -> (std::process::Output, Termination, Duration) {
    let started = Instant::now();
    let mut command = Command::new("cargo");
    command
        .args([
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
        .env("RECIPE_TRIAL_DIRECTORY", directory)
        .env("RECIPE_NATIVE_CACHE_DIRECTORY", &config.native_cache_directory)
        .env("TMPDIR", directory)
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
    let mut child = command.spawn().expect("cannot start Recipe traversal");
    let group = child.id();
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
    let collect = |child: &mut Child| -> std::process::ExitStatus {
        if let Some(status) = child.try_wait().expect("cannot inspect Recipe traversal") { return status }
        let terminated = signal_process_group(group, "-KILL");
        assert!(terminated || !signal_process_group(group, "-0"), "cannot terminate Recipe traversal process group");
        child.wait().expect("cannot collect terminated Recipe traversal")
    };
    let (status, termination) = loop {
        // A retired trial releases its resources before the next control period, so the
        // retirement check precedes the slow-cursor deadline.
        if retire.load(Ordering::Relaxed) {
            break (collect(&mut child), Termination::Retired);
        }
        if started.elapsed() >= Duration::from_secs(config.slow_cursor_seconds) {
            break (collect(&mut child), Termination::Slow);
        }
        if let Some(status) = child.try_wait().expect("cannot inspect Recipe traversal") {
            break (status, Termination::Completed);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    // The worker slot is only released once the whole group is gone. A group that survives
    // SIGKILL is reported every grace period and keeps applying back pressure, because
    // counting it as released would understate the machine's real occupancy.
    let mut deadline = Instant::now() + Duration::from_secs(config.trial_grace_seconds);
    while signal_process_group(group, "-0") {
        if Instant::now() >= deadline {
            event(config, &format!("RETIRE trial device={device} cursor={cursor} group={group} overrun grace={}s", config.trial_grace_seconds));
            deadline = Instant::now() + Duration::from_secs(config.trial_grace_seconds);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = std::process::Output {
            status,
            stdout: stdout.join().expect("Recipe traversal stdout reader failed"),
            stderr: stderr.join().expect("Recipe traversal stderr reader failed"),
        };
    let elapsed = started.elapsed();
    let termination = match termination {
        Termination::Completed if elapsed >= Duration::from_secs(config.slow_cursor_seconds) => Termination::Slow,
        termination => termination,
    };
    (output, termination, elapsed)
}

fn signal_process_group(group: u32, signal: &str) -> bool {
    Command::new("kill")
        .args([signal, &format!("-{group}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("cannot inspect Recipe traversal process group")
        .success()
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

fn packet_current(repository: &Path, packet: &str) -> bool {
    let commit = field(packet, "base=").split_whitespace().find_map(|value| value.strip_prefix("commit=")).expect("failure packet base has no commit");
    output(Command::new("git").args(["diff", "--quiet", commit, "HEAD", "--", "Cargo.lock", "Cargo.toml", "amd-nv-cpu.ll", "build.rs", "cli.rs", "data", "recipe.rs", "test.rs"]).current_dir(repository), None).status.success()
}

fn crash_packet(trial: &Trial, text: &str, observed: i32, replayed: i32) -> String {
    let line = text.lines().find(|line| line.starts_with("composition ") && line.contains(':')).expect("signaled traversal emitted no composition");
    let (case, configuration) = line.strip_prefix("composition ").expect("composition prefix disappeared").split_once(':').expect("composition description has no separator");
    let step = text.lines().find_map(|line| line.split_whitespace().find_map(|value| value.strip_prefix("step="))).expect("signaled traversal emitted no permutation step");
    let source = std::fs::read_to_string(&trial.reproduction).expect("signaled traversal staged no reproduction");
    let mut fingerprint = 1_469_598_103_934_665_603_u64;
    for byte in format!("{}:{observed}:{replayed}:{case}", trial.device).bytes().chain(source.bytes()) {
        fingerprint ^= u64::from(byte);
        fingerprint = fingerprint.wrapping_mul(1_099_511_628_211);
    }
    let observed = format!("device {} terminated by signal {observed}", trial.device);
    let replayed = format!("device {} terminated by signal {replayed}", trial.device);
    format!("id={fingerprint:016x}\nbase={}\ncursor=seed:{} cursor:{} next:{} step:{step} composition:{case}\nconfiguration={}\nexpected=training, optional resume, and inference produce finite numerical results through the public Recipe API\nobserved=phase:process message:{observed}\noutput=phase:process message:{observed}\nreplay=phase:process message:{replayed} stable:true\ncommand=cargo run --bin recipe -- {}\nreproduction:\n```rust\n{source}```", repository_base(&trial.config.repository), trial.config.seed, trial.cursor, trial.cursor + 1, configuration.trim(), trial.reproduction.display())
}

fn slow_packet(trial: &Trial) -> String {
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
    format!("kind=performance\nid={fingerprint:016x}\nbase={}\ncursor=seed:{} cursor:{} next:{} composition:{composition}\nexpected=one training epoch uses throughput proportionate to its operations, data volume, memory traffic, and available hardware\nobserved=the public Recipe run remained active after {} seconds\nmeasurement=elapsed_seconds:{} backend:{}\nreplay=the configured slow-runtime threshold was crossed stable:true\ncommand=cargo run --bin recipe -- {}\nreproduction:\n```rust\n{source}```\nbackend={}", repository_base(&trial.config.repository), trial.config.seed, trial.cursor, trial.cursor + 1, trial.elapsed.as_secs(), trial.elapsed.as_secs(), trial.device, trial.reproduction.display(), trial.device)
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
    let issue_reader = format!("mcp_servers.recipe_issues.command=\"{}\"", config.spark_issue_reader.display());
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
                "--config",
                &issue_reader,
                "--config",
                "mcp_servers.recipe_issues.enabled_tools=[\"search_issues\",\"read_issue\"]",
                "--config",
                "mcp_servers.recipe_issues.default_tools_approval_mode=\"approve\"",
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
    if jq(&text, ".status").as_deref() == Ok("ERROR") { return Err(failure(&result)) }
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
// Returns Ok(None) when the lease was retired mid-review, so the caller can return the
// packet to Ready untouched.
fn opencode(config: &Config, model: &str, prompt: &str, cancel: &Cancel) -> std::result::Result<Option<Decision>, String> {
    let session = {
        let _creation = OPENCODE_SESSION.lock().expect("OpenCode session lock is poisoned");
        jq(&opencode_request(config, "POST", "/session", r#"{"title":"Recipe issue review"}"#)?, ".id")?
    };
    cancel.opened(&session);
    let Some(response) = opencode_turn(config, model, prompt, &session, cancel)? else { return Ok(None) };
    let mut json = object(response);
    if json.is_err() {
        trace(
            config,
            &format!("model={model} repairing structured output session={session}"),
        );
        let Some(repaired) = opencode_turn(config, model, "Return only one corrected JSON object matching the required schema. Do not repeat the investigation.", &session, cancel)? else { return Ok(None) };
        json = object(repaired);
    }
    let json = json?;
    let (provider, model) = model.split_once('/').expect("OpenCode model has no provider");
    Ok(Some(Decision {
        provider: if provider == "openrouter" { "openrouter" } else { "opencode" },
        model: model.to_owned(),
        effort: "default".to_owned(),
        json,
    }))
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
    command.env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", &config.ollama_disable_nonessential_traffic);
    command.args(["launch", "claude", "--model", &config.ollama_model, "--yes", "--"]);
    command.args([
        "-p",
        prompt,
        "--output-format",
        "stream-json",
        "--verbose",
        "--name",
        &config.ollama_session_name,
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

fn classify(config: &Config, prompt: &str, packet: &str, model: &str, cancel: &Cancel) -> std::result::Result<Decision, Review> {
    display_reviewing(packet, model);
    match opencode(config, model, prompt, cancel) {
        Ok(None) => {
            display_reviewing(packet, "queued");
            Err(Review::Retired)
        }
        Ok(Some(decision)) => Ok(decision),
        Err(error) if usage_exhausted(&error) => {
            let _ = cooldown(config, model, &error);
            display_reviewing(packet, "deferred");
            Err(Review::Unavailable)
        }
        Err(error) => {
            trace(config, &format!("review endpoint transport failure model={model} error={error}"));
            display_reviewing(packet, "deferred");
            Err(Review::Transport)
        }
    }
}

fn triage(config: &Config, instructions: &str, packet: &str, model: &str, cancel: &Cancel) -> Review {
    let composition = packet_composition(packet);
    let schema =
        std::fs::read_to_string(&config.decision_schema).expect("cannot read decision schema");
    let prompt = format!("{instructions}\n\n## Failure packet\n\n{packet}\n\n## Required decision schema\n\n{schema}");
    let decision = match classify(config, &prompt, packet, model, cancel) {
        Ok(decision) => decision,
        Err(result) => return result,
    };
    let classifier = identity(decision.provider, &decision.model);
    event(
        config,
        &format!(
            "CLASSIFY model={classifier} composition={composition}"
        ),
    );
    let mut verdict = jq(&decision.json, ".verdict").expect("validated decision lost its verdict");
    let mut issue = jq(&decision.json, ".issue").expect("validated decision lost its issue");
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
    let _publication = PUBLICATION.lock().expect("publication lock is poisoned");
    if verdict == "new" {
        let limit = config.issue_history_limit.to_string();
        let existing = output(
            Command::new("gh")
                .args(["issue", "list", "--state", "all", "--limit", &limit, "--json", "number,title,body"])
                .current_dir(&config.repository),
            None,
        );
        assert!(existing.status.success(), "GitHub duplicate check failed: {}", failure(&existing));
        let existing = String::from_utf8(existing.stdout).expect("GitHub issue history is not UTF-8");
        let cause = packet.lines().find(|line| line.starts_with("observed=phase:process message:")).unwrap_or("");
        let duplicate = output(Command::new("jq").args([
            "-er", "--arg", "title", &title, "--arg", "cause", cause,
            r#"[.[] | select((.title | ascii_downcase) == ($title | ascii_downcase) or ($cause != "" and ((.body // "") | contains($cause))))] | sort_by(.number) | .[0].number // empty"#,
        ]), Some(&existing));
        if duplicate.status.success() {
            verdict = "comment".to_owned();
            issue = String::from_utf8(duplicate.stdout).expect("duplicate issue number is not UTF-8").trim().to_owned();
        }
    }
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

fn review_loop(
    work: Arc<Mutex<Work>>,
    config: Arc<Config>,
    instructions: Arc<String>,
    packet: String,
    lease: ModelLease,
    cancel: Cancel,
) {
    let result = triage(&config, &instructions, &packet, &lease.model, &cancel);
    match result {
        Review::Done => lease.endpoint.completed(),
        Review::Transport => lease.endpoint.overloaded(),
        Review::Retired | Review::Stop | Review::Unavailable => {}
    }
    let key = packet_key(&packet);
    let mut state = work.lock().expect("failure queue lock is poisoned");
    match result {
        Review::Done => {
            let reviewed = field(&packet, "id=");
            let backend = packet_backend(&packet);
            if let Some(index) = state.packets.iter().position(|queued| field(queued, "id=") == reviewed && packet_backend(queued) == backend) {
                state.packets.remove(index);
                persist_queue(&config.queue_path, &state.packets);
            }
            state.ready.remove(&key);
        }
        Review::Stop => state.halted = true,
        // A retired review returns the same packet to Ready. It keeps its queue entry and
        // its Ready mark, and only releases the claim, so exactly one later lease reviews it.
        Review::Retired => {
            assert!(state.ready.contains(&key), "a retired review lost its Ready packet");
            event(&config, &format!("RETIRE review packet={key} model={}", lease.model));
            display_reviewing(&packet, "queued");
        }
        Review::Unavailable | Review::Transport => { state.ready.remove(&key); }
    }
    state.active.remove(&key);
    state.reviewing.remove(&key);
}

fn enqueue_review(
    work: &Arc<Mutex<Work>>,
    config: &Config,
    packet: &str,
) -> Option<usize> {
    let depth = {
        let mut state = work.lock().expect("failure queue lock is poisoned");
        if let Some(index) = state.packets.iter().position(|queued| same_failure(queued, &packet)) {
            let key = packet_key(packet);
            if state.ready.insert(key) {
                state.packets[index] = packet.to_owned();
                persist_queue(&config.queue_path, &state.packets);
                Some(state.packets.len())
            } else {
                None
            }
        } else {
            if packet.starts_with("kind=performance\n") {
                state.packets.push_front(packet.to_owned());
            } else {
                state.packets.push_back(packet.to_owned());
            }
            state.ready.insert(packet_key(packet));
            persist_queue(&config.queue_path, &state.packets);
            Some(state.packets.len())
        }
    };
    display_queue(packet);
    depth
}

fn resolver_units() -> std::result::Result<BTreeMap<u64, Instant>, String> {
    let resolvers = output(Command::new("systemctl").args(["--user", "show", "recipe-resolve-*", "--property=Id", "--property=ActiveState", "--property=ActiveEnterTimestampMonotonic", "--value"]), None);
    if !resolvers.status.success() { return Err(failure(&resolvers)) }
    let uptime = std::fs::read_to_string("/proc/uptime").map_err(|error| error.to_string())?.split_whitespace().next().expect("system uptime is absent").parse::<f64>().map_err(|error| error.to_string())?;
    let now = Instant::now();
    Ok(String::from_utf8_lossy(&resolvers.stdout).split("\n\n").filter_map(|unit| {
        let mut values = unit.lines();
        let issue = values.next()?.strip_prefix("recipe-resolve-")?.strip_suffix(".service")?.parse::<u64>().ok()?;
        (values.next()? == "active").then_some(())?;
        let entered = values.next()?.parse::<u64>().ok()?;
        let elapsed = Duration::from_secs_f64(uptime).saturating_sub(Duration::from_micros(entered));
        Some((issue, now.checked_sub(elapsed).expect("resolver start precedes process clock")))
    }).collect())
}

fn resolver_issue(config: &Config, active: &BTreeSet<u64>, running: &BTreeMap<u64, Instant>) -> std::result::Result<Option<(u64, String)>, String> {
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
    let mut claimed = String::from_utf8_lossy(&pull_requests.stdout)
        .lines()
        .filter_map(|number| number.parse::<u64>().ok())
        .collect::<BTreeSet<_>>();
    claimed.extend(running.keys().copied());
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
    let limit = config.resolver_memory_budget_mib / config.resolver_memory_mib;
    assert!(limit != 0, "resolver_memory_budget_mib must hold at least one resolver");
    let running = resolver_units()?;
    if issues.len() + running.keys().filter(|issue| !issues.contains(issue)).count() >= limit as usize { return Ok(None) }
    let Some((number, url)) = resolver_issue(config, &issues, &running)? else { return Ok(None) };
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

fn resolver_loop(config: Arc<Config>, path: PathBuf, active: Arc<Mutex<BTreeSet<u64>>>) {
    loop {
        let model = format!("claude/{}-{}", config.resolver_model.strip_prefix("claude-").unwrap_or(&config.resolver_model), config.resolver_effort);
        if !available(&model) {
            std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
            continue;
        }
        let issue = match resolver_claim(&config, &active) {
            Ok(Some(issue)) => issue,
            Ok(None) => {
                std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
                continue;
            }
            Err(error) => {
                trace(&config, &format!("resolver issue selection failed error={error}"));
                std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
                continue;
            }
        };
        let goal = format!(r#"/goal Read GitHub issue #{number} and every comment in full, then resolve that one issue completely and create one pull request that closes it.

Use unrestricted tools and make every required repository and GitHub change yourself. Never edit the current issue-machine worktree. Work in one separate Git worktree under {root}, based on the latest origin/{base}. Reuse coherent existing work for issue #{number} if it exists. Reproduce the exact public failure first, identify the earliest cause in current origin/{base}, implement the root fix without a fallback or parallel implementation, and validate the exact public Recipe path end to end. Test reachable edge cases one at a time through the same public entrypoint. Preserve unrelated user changes.

Do not read, inspect, execute, edit, create, install, or delegate Python code, files, tooling, dependencies, documentation, or artifacts. Do not create or redirect output into additional log files. The public Recipe entrypoint may write only the repository-root recipe.log. Use Google developer documentation style and never use em dashes.

Commit and push the coherent fix, then create one pull request targeting {base}. Keep the pull request body concise and include only `Fixes #{number}`, the exact reproduction, the root cause, the change, and measured end-to-end evidence. Omit issue history, implementation tours, alternatives, tables, and exhaustive edge-case narratives. Do not stop until the pull request exists and its URL is visible. Target issue: {url}"#, number = issue.number, root = config.resolver_worktree_root.display(), base = config.resolver_base, url = issue.url);
        event(&config, &format!("RESOLVE model={model} issue=#{} url={}", issue.number, issue.url));
        let memory = format!("MemoryMax={}M", config.resolver_memory_mib);
        let environment = format!("Environment={}", config.resolver_environment);
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
                    config.repository.to_str().expect("repository path is not UTF-8"),
                    "--property",
                    &memory,
                    "--property",
                    &environment,
                    "--",
                    config.claude_binary.to_str().expect("Claude binary path is not UTF-8"),
                    "-p",
                    &goal,
                    "--model",
                    &config.resolver_model,
                    "--effort",
                    &config.resolver_effort,
                    "--dangerously-skip-permissions",
                    "--strict-mcp-config",
                    "--output-format",
                    "json",
                    "--name",
                    &format!("Resolve Recipe issue #{}", issue.number),
                ])
                .current_dir(&config.repository),
            None,
        );
        let transcript = format!("{}{}", String::from_utf8_lossy(&result.stderr), String::from_utf8_lossy(&result.stdout));
        if usage_exhausted(&transcript) {
            if let Some(reset) = cooldown(&config, &model, &transcript) { setting(&path, "resolver_reset_unix", reset) }
            display().lock().expect("display lock is poisoned").resolvers.remove(&issue.number);
            std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
            continue;
        }
        if !result.status.success() {
            let error = failure(&result);
            event(&config, &format!("RESOLVE model={model} issue=#{} failed", issue.number));
            trace(&config, &format!("resolver issue #{} failed error={error}", issue.number));
            display_resolved(issue.number, "FAIL", None);
            std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
            continue;
        }
        match resolver_pr(&config, issue.number) {
            Ok(Some(url)) => {
                event(&config, &format!("PR model={model} issue=#{} url={url}", issue.number));
                display_resolved(issue.number, "PR", Some(&url));
            }
            Ok(None) => {
                event(&config, &format!("RESOLVE model={model} issue=#{} failed", issue.number));
                trace(&config, &format!("resolver issue #{} completed without an open pull request", issue.number));
                display_resolved(issue.number, "FAIL", None);
                std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
                continue;
            }
            Err(error) => {
                event(&config, &format!("RESOLVE model={model} issue=#{} failed", issue.number));
                trace(&config, &format!("resolver issue #{} pull request lookup failed error={error}", issue.number));
                display_resolved(issue.number, "FAIL", None);
                std::thread::sleep(Duration::from_secs(config.resolver_poll_seconds));
                continue;
            }
        }
    }
}

fn discovery_loop(
    worker: usize,
    device: String,
    config: Arc<Config>,
    work: Arc<Mutex<Work>>,
    cluster: Arc<cluster::Client>,
    send: mpsc::Sender<Discovery>,
    retire: Arc<AtomicBool>,
) {
    if work.lock().expect("failure queue lock is poisoned").halted { return }
    let replay = {
        let mut state = work.lock().expect("failure queue lock is poisoned");
        let packet = state.packets.iter().find(|packet| packet_backend(packet) == device && !state.ready.contains(&packet_key(packet)) && !state.active.contains(&packet_key(packet))).cloned();
        if let Some(packet) = &packet { state.active.insert(packet_key(packet)); }
        packet
    };
    let (start, lease) = if let Some(packet) = replay.as_ref() {
        (packet_cursor(packet), None)
    } else {
        let Some(lease) = cluster.claim(&device) else { return };
        let start = lease.cursor;
        let renewal = Duration::from_millis(config.cursor_renew_milliseconds);
        (start, Some(CursorLease::new(Arc::clone(&cluster), lease, renewal)))
    };
    if retire.load(Ordering::Relaxed) {
        let _ = send.send(Discovery::Retired { worker, device, cursor: start, replay, lease });
        return;
    }
    let directory = TrialDirectory::create(&config, worker, start);
    let reproduction = directory.reproduction();
    if send.send(Discovery::Start { worker, device: device.clone(), cursor: start }).is_err() { return }
    let (mut result, mut termination, mut elapsed) = trial(&config, &device, start, &directory.0, &reproduction, &retire);
    if matches!(termination, Termination::Completed) {
        if let Some(observed) = termination_signal(result.status) {
            let replayed;
            (result, termination, replayed) = trial(&config, &device, start, &directory.0, &reproduction, &retire);
            elapsed += replayed;
            if matches!(termination, Termination::Completed) {
                let crash = termination_signal(result.status).map(|replayed| (observed, replayed));
                let completed = Trial { config, worker, device: device.clone(), cursor: start, _directory: directory, reproduction, output: result, elapsed, timed_out: false, crash, replay, lease };
                let _ = send.send(Discovery::Complete(completed));
                return;
            }
        }
    }
    if matches!(termination, Termination::Retired) {
        // Dropping the directory here removes the complete trial directory before the
        // worker slot is released, so no retired trial leaves storage behind.
        drop(directory);
        let _ = send.send(Discovery::Retired { worker, device, cursor: start, replay, lease });
        return;
    }
    let timed_out = matches!(termination, Termination::Slow);
    let completed = Trial { config, worker, device: device.clone(), cursor: start, _directory: directory, reproduction, output: result, elapsed, timed_out, crash: None, replay, lease };
    let _ = send.send(Discovery::Complete(completed));
}

// The review manager tracks the commanded lease count in both directions: it admits new
// leases while the command exceeds the live count, and retires the newest live leases
// while the command falls below it.
fn manage_reviews(config: Arc<Config>, work: Arc<Mutex<Work>>, admission: Arc<Admission>, endpoint: Arc<ReviewEndpoint>, instructions: Arc<String>) {
    std::thread::spawn(move || {
        loop {
            let live = admission.occupancy().reviews;
            let target = endpoint.admissible(admission.command("review"));
            for cancel in admission.retire_reviews(live.saturating_sub(target)) {
                // Abort the real session immediately so the endpoint stops working on a
                // review the machine no longer has capacity for.
                if let Some(session) = cancel.session() {
                    let _ = opencode_request(&config, "POST", &format!("/session/{session}/abort"), "");
                }
            }
            admission.audit_reviews(&config);
            let mut admitted = live;
            for _ in live..target {
                let Some(lease) = endpoint.claim(&config) else { break };
                let packet = {
                    let mut state = work.lock().expect("failure queue lock is poisoned");
                    if state.halted { None } else {
                        let packet = state.packets.iter().find(|packet| state.ready.contains(&packet_key(packet)) && !state.active.contains(&packet_key(packet))).cloned();
                        if let Some(packet) = &packet {
                            let key = packet_key(packet);
                            state.active.insert(key.clone());
                            state.reviewing.insert(key);
                        }
                        packet
                    }
                };
                let Some(packet) = packet else { break };
                let (worker, cancel) = admission.admit_review();
                admitted += 1;
                let worker_config = Arc::clone(&config);
                let worker_work = Arc::clone(&work);
                let worker_admission = Arc::clone(&admission);
                let worker_instructions = Arc::clone(&instructions);
                std::thread::spawn(move || {
                    let _slot = ReviewSlot { admission: worker_admission, worker };
                    review_loop(worker_work, worker_config, worker_instructions, packet, lease, cancel);
                });
            }
            // Endpoint reach is state, so an unfilled command is reported once per period
            // rather than retried.
            if admitted < target {
                event(&config, &format!("ENDPOINT leases={admitted}/{target} derating={:.3} reachable={}/{}", endpoint.derating(), endpoint.reachable(&config), config.opencode_models.len()));
            }
            std::thread::sleep(Duration::from_millis(config.review_poll_milliseconds));
        }
    });
}

fn manage_trials(
    config: Arc<Config>,
    work: Arc<Mutex<Work>>,
    cluster: Arc<cluster::Client>,
    admission: Arc<Admission>,
    send: mpsc::Sender<Discovery>,
) {
    std::thread::spawn(move || {
        loop {
            let occupancy = admission.occupancy();
            for device in admission.trials.keys() {
                let live = *occupancy.trials.get(device).expect("occupancy lost a discovery device");
                let target = admission.command(device);
                admission.retire_trials(device, live.saturating_sub(target));
                for _ in live..if cluster.exhausted() { live } else { target } {
                    let (worker, retire) = admission.admit_trial(device);
                    let worker_config = Arc::clone(&config);
                    let worker_work = Arc::clone(&work);
                    let worker_cluster = Arc::clone(&cluster);
                    let worker_admission = Arc::clone(&admission);
                    let worker_send = send.clone();
                    let worker_device = device.clone();
                    std::thread::spawn(move || {
                        let _slot = TrialSlot { admission: worker_admission, worker };
                        discovery_loop(worker, worker_device, worker_config, worker_work, worker_cluster, worker_send, retire);
                    });
                }
            }
            std::thread::sleep(Duration::from_millis(config.controller_sample_milliseconds));
        }
    });
}

fn main() {
    let mut limit = std::mem::MaybeUninit::<Limit>::uninit();
    assert_eq!(unsafe { getrlimit(RLIMIT_NOFILE, limit.as_mut_ptr()) }, 0, "cannot read the file descriptor limit");
    let limit = unsafe { limit.assume_init() };
    let limit = Limit { current: limit.maximum, maximum: limit.maximum };
    assert_eq!(unsafe { setrlimit(RLIMIT_NOFILE, &limit) }, 0, "cannot remove the file descriptor ceiling");
    let executable = std::env::current_exe().expect("cannot locate machine executable");
    let arguments = std::env::args().collect::<Vec<_>>();
    assert_eq!(arguments.len(), 2, "usage: recipe-machine /absolute/path/to/machine.toml");
    let path = PathBuf::from(&arguments[1]);
    assert!(path.is_absolute(), "machine configuration path must be absolute");
    let initial = Arc::new(config(&path));
    assert!(initial.discovery_devices.iter().any(|device| device == "cpu"), "discovery_devices must contain cpu");
    assert_eq!(initial.discovery_devices.iter().collect::<BTreeSet<_>>().len(), initial.discovery_devices.len(), "discovery_devices contains a duplicate device");
    assert!(!initial.opencode_models.is_empty(), "opencode_models must contain at least one model");
    let digest = format!(
        "{}{}",
        cluster::binary_digest(&executable, &initial.sha256_binary),
        cluster::repository_digest(&initial.repository, &initial.cluster_workload_paths, &initial.git_binary, &initial.sha256_binary),
    );
    let (cluster_send, cluster_receive) = mpsc::channel();
    if initial.cluster_coordinator {
        cluster::start_server(cluster::ServerConfig {
            listen: initial.cluster_listen,
            state_path: initial.cluster_state_path.clone(),
            secret: initial.cluster_secret.clone(),
            digest: digest.clone(),
            initial_cursor: initial.cursor,
            cursor_step: initial.compositions_per_batch,
            batches: initial.batches,
            node_limit: initial.cluster_node_limit,
            node_timeout: Duration::from_secs(initial.cluster_node_timeout_seconds),
            cursor_timeout: Duration::from_secs(initial.cursor_lease_seconds),
        }, cluster_send);
    }
    let cluster = Arc::new(cluster::Client::new(
        initial.cluster_address,
        initial.cluster_secret.clone(),
        initial.cluster_node_id.clone(),
        digest.clone(),
        &initial.discovery_devices,
        Duration::from_secs(initial.cluster_request_seconds),
    ));
    cluster.register();
    let panic_config = Arc::clone(&initial);
    std::panic::set_hook(Box::new(move |panic| log(&panic_config, &format!("PANIC {panic}"))));
    prepare_storage(&initial);
    if initial.cluster_coordinator {
        if let Some(seconds) = initial.resolver_reset_unix.checked_sub(epoch()).filter(|seconds| *seconds != 0) {
        let model = format!("claude/{}-{}", initial.resolver_model.strip_prefix("claude-").unwrap_or(&initial.resolver_model), initial.resolver_effort);
        COOLDOWNS.get_or_init(|| Mutex::new(BTreeMap::new())).lock().expect("cooldown lock is poisoned").insert(model, Instant::now() + Duration::from_secs(seconds));
        }
    }
    let _opencode_server = initial.cluster_coordinator.then(|| opencode_server(&initial));
    display_clock(format!("claude/{}-{}", initial.resolver_model.strip_prefix("claude-").unwrap_or(&initial.resolver_model), initial.resolver_effort));
    let pending = if initial.cluster_coordinator {
        std::fs::read_to_string(&initial.queue_path).map(|text| queued(&text)).unwrap_or_default()
    } else {
        VecDeque::new()
    };
    let ready = pending.iter().filter(|packet| packet_current(&initial.repository, packet)).map(|packet| packet_key(packet)).collect();
    let work = Arc::new(Mutex::new(Work { packets: pending, active: BTreeSet::new(), reviewing: BTreeSet::new(), ready, halted: false }));
    work.lock().expect("failure queue lock is poisoned").packets.iter().for_each(|packet| display_queue(packet));
    let admission = Arc::new(Admission::new(&initial));
    if initial.cluster_coordinator {
        let instructions = Arc::new(std::fs::read_to_string(&initial.triage_path).expect("cannot read triage.md"));
        let endpoint = Arc::new(ReviewEndpoint::new(&initial));
        manage_reviews(Arc::clone(&initial), Arc::clone(&work), Arc::clone(&admission), endpoint, Arc::clone(&instructions));
        if initial.resolver_enabled {
            let active = Arc::new(Mutex::new(BTreeSet::new()));
            for _ in 0..initial.resolver_concurrency {
                let resolver_config = Arc::clone(&initial);
                let resolver_path = path.clone();
                let resolver_active = Arc::clone(&active);
                std::thread::spawn(move || resolver_loop(resolver_config, resolver_path, resolver_active));
            }
        }
    }
    regulate(Arc::clone(&initial), Arc::clone(&work), Arc::clone(&admission));
    let (send, receive) = mpsc::channel();
    if initial.cluster_coordinator {
        let completion_send = send.clone();
        std::thread::spawn(move || {
            for completion in cluster_receive {
                if completion_send.send(Discovery::Cluster(completion)).is_err() {
                    return;
                }
            }
        });
    }
    manage_trials(Arc::clone(&initial), Arc::clone(&work), Arc::clone(&cluster), Arc::clone(&admission), send);
    for discovery in receive {
        let mut trial = match discovery {
            Discovery::Start { worker, device, cursor } => {
                display_start(worker, device, cursor);
                continue;
            }
            Discovery::Cluster(completion) => {
                for packet in completion.packets {
                    let composition = packet_composition(&packet);
                    if let Some(depth) = enqueue_review(&work, &initial, &packet) {
                        event(&initial, &format!("QUEUE node={} device={} cursor={} composition={composition} depth={depth}", completion.node, completion.device, completion.cursor));
                    }
                }
                continue;
            }
            // A retired trial produces no packet and no cursor advance. Its cursor returns
            // to the allocator so exactly one later trial replays it, and a retired replay
            // releases its claim so the queued packet stays reviewable.
            Discovery::Retired { worker, device, cursor, replay, lease } => {
                match replay {
                    Some(packet) => { work.lock().expect("failure queue lock is poisoned").active.remove(&packet_key(&packet)); }
                    None => lease.expect("a new cursor has no cluster lease").release(),
                }
                display_finish(worker, "STOP");
                event(&initial, &format!("RETIRE trial device={device} cursor={cursor}"));
                continue;
            }
            Discovery::Complete(trial) => {
                display_complete(trial.worker, trial.elapsed);
                trial
            }
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
            vec![slow_packet(&trial)]
        } else {
            packets(&text)
                .into_iter()
                .map(|packet| packet_for_device(packet, &trial.device))
                .collect::<Vec<_>>()
        };
        if let Some((observed, replayed)) = trial.crash {
            failures.push(packet_for_device(&crash_packet(&trial, &text, observed, replayed), &trial.device));
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
        if trial.replay.is_some() {
            for packet in &failures {
                let packet_cursor = packet_cursor(packet);
                let composition = packet_composition(packet);
                if let Some(depth) = enqueue_review(&work, &trial.config, packet) {
                    event(&trial.config, &format!("QUEUE device={} cursor={packet_cursor} composition={composition} depth={depth}", trial.device));
                }
            }
        }
        let status = if trial.timed_out { "SLOW" } else if failures.is_empty() { "PASS" } else { "FAIL" };
        display_finish(trial.worker, status);
        if let Some(replayed) = &trial.replay {
            let key = packet_key(replayed);
            let removed = {
                let mut state = work.lock().expect("failure queue lock is poisoned");
                state.active.remove(&key);
                if state.ready.contains(&key) {
                    false
                } else {
                    state.packets.retain(|packet| packet_key(packet) != key);
                    persist_queue(&trial.config.queue_path, &state.packets);
                    true
                }
            };
            if removed { display_drop(replayed) }
            continue;
        }
        let next = if trial.timed_out || signal.is_some() {
            trial.cursor + trial.config.compositions_per_batch
        } else {
            text.lines()
                .find_map(|line| line.strip_prefix("composition cursor="))
                .expect("Recipe traversal emitted no next cursor")
                .parse()
                .expect("Recipe next cursor is invalid")
        };
        let frontier = trial.lease.take().expect("a completed new cursor has no cluster lease").complete(next, &trial.device, &failures);
        event(&trial.config, &format!("CLUSTER node={} device={} cursor={} frontier={frontier}", trial.config.cluster_node_id, trial.device, trial.cursor));
    }
}
