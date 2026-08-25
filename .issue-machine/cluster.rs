use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Client {
    address: SocketAddr,
    secret: String,
    node: String,
    digest: String,
    devices: String,
    timeout: Duration,
    exhausted: Arc<AtomicBool>,
}

#[derive(Clone, Copy)]
pub struct Lease {
    pub id: u64,
    pub cursor: u64,
}

pub struct Completion {
    pub node: String,
    pub device: String,
    pub cursor: u64,
    pub packets: Vec<String>,
}

pub struct ServerConfig {
    pub listen: SocketAddr,
    pub state_path: PathBuf,
    pub secret: String,
    pub digest: String,
    pub initial_cursor: u64,
    pub cursor_step: u64,
    pub batches: u64,
    pub node_limit: usize,
    pub node_timeout: Duration,
    pub cursor_timeout: Duration,
}

struct Node {
    digest: String,
    devices: String,
    deadline: u64,
}

struct Held {
    cursor: u64,
    node: String,
    device: String,
    deadline: u64,
}

struct State {
    frontier: u64,
    next: u64,
    next_lease: u64,
    returned: BTreeSet<u64>,
    completed: BTreeMap<u64, u64>,
    leases: BTreeMap<u64, Held>,
    nodes: BTreeMap<String, Node>,
}

struct Coordinator {
    config: ServerConfig,
    state: State,
    completions: mpsc::Sender<Completion>,
}

fn hex(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(value: &str) -> Result<String, String> {
    if value.len() % 2 != 0 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("cluster packet encoding is invalid".to_owned());
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                b'A'..=b'F' => Ok(byte - b'A' + 10),
                _ => Err("cluster packet encoding is invalid".to_owned()),
            };
            Ok(digit(pair[0])? * 16 + digit(pair[1])?)
        })
        .collect::<Result<Vec<_>, String>>()?;
    String::from_utf8(bytes).map_err(|_| "cluster packet is not UTF-8".to_owned())
}

fn packet_bundle(packets: &[String]) -> String {
    packets.iter().map(|packet| hex(packet)).collect::<Vec<_>>().join(",")
}

fn unpack_bundle(bundle: &str) -> Result<Vec<String>, String> {
    if bundle.is_empty() {
        return Ok(Vec::new());
    }
    bundle.split(',').map(unhex).collect()
}

fn epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time precedes the Unix epoch")
        .as_secs()
}

fn token(value: &str, name: &str) {
    assert!(
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)),
        "cluster {name} contains an unsupported character"
    );
}

pub fn binary_digest(binary: &Path, sha256_binary: &Path) -> String {
    let output = Command::new(sha256_binary)
        .arg(binary)
        .output()
        .expect("cannot start the configured SHA-256 command");
    assert!(output.status.success(), "cannot hash the machine binary");
    let digest = String::from_utf8(output.stdout)
        .expect("SHA-256 output is not UTF-8")
        .split_whitespace()
        .next()
        .expect("SHA-256 output is empty")
        .to_owned();
    assert!(digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()), "SHA-256 output is invalid");
    digest
}

fn content_digest(content: &[u8], sha256_binary: &Path) -> String {
    let mut child = Command::new(sha256_binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("cannot start the configured SHA-256 command");
    child.stdin.take().expect("SHA-256 stdin is absent").write_all(content).expect("cannot write SHA-256 content");
    let output = child.wait_with_output().expect("cannot collect the SHA-256 command");
    assert!(output.status.success(), "cannot hash cluster content");
    let digest = String::from_utf8(output.stdout)
        .expect("SHA-256 output is not UTF-8")
        .split_whitespace()
        .next()
        .expect("SHA-256 output is empty")
        .to_owned();
    assert!(digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()), "SHA-256 output is invalid");
    digest
}

pub fn repository_digest(repository: &Path, paths: &[String], git_binary: &Path, sha256_binary: &Path) -> String {
    assert!(!paths.is_empty(), "cluster workload paths are empty");
    let listed = Command::new(git_binary)
        .args(["ls-files", "-z", "--"])
        .args(paths)
        .current_dir(repository)
        .output()
        .expect("cannot start workload file discovery");
    assert!(listed.status.success(), "cannot discover cluster workload files");
    let mut files = listed.stdout.split(|byte| *byte == 0).filter(|path| !path.is_empty()).collect::<Vec<_>>();
    files.sort_unstable();
    assert!(!files.is_empty(), "cluster workload contains no tracked files");
    let mut manifest = Vec::new();
    for path in files {
        let path = std::str::from_utf8(path).expect("cluster workload path is not UTF-8");
        let hashed = Command::new(git_binary)
            .args(["hash-object", "--", path])
            .current_dir(repository)
            .output()
            .expect("cannot start workload hashing");
        assert!(hashed.status.success(), "cannot hash cluster workload path {path}");
        manifest.extend_from_slice(path.as_bytes());
        manifest.push(b'\t');
        manifest.extend_from_slice(hashed.stdout.strip_suffix(b"\n").expect("workload hash has no newline"));
        manifest.push(b'\n');
    }
    content_digest(&manifest, sha256_binary)
}

fn map(values: &BTreeMap<u64, u64>) -> String {
    values
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn set(values: &BTreeSet<u64>) -> String {
    values.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

fn load_map(value: &str) -> BTreeMap<u64, u64> {
    value
        .split(',')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let (key, value) = entry.split_once(':').expect("cluster state map entry has no value");
            (
                key.parse().expect("cluster state map key is invalid"),
                value.parse().expect("cluster state map value is invalid"),
            )
        })
        .collect()
}

fn load_set(value: &str) -> BTreeSet<u64> {
    value
        .split(',')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.parse().expect("cluster state set entry is invalid"))
        .collect()
}

impl State {
    fn new(cursor: u64) -> Self {
        Self {
            frontier: cursor,
            next: cursor,
            next_lease: 1,
            returned: BTreeSet::new(),
            completed: BTreeMap::new(),
            leases: BTreeMap::new(),
            nodes: BTreeMap::new(),
        }
    }

    fn load(path: &Path, cursor: u64) -> Self {
        if !path.exists() {
            return Self::new(cursor);
        }
        let text = std::fs::read_to_string(path).expect("cannot read cluster state");
        let value = |name: &str| {
            text.lines()
                .find_map(|line| line.split_once('=').filter(|(key, _)| *key == name).map(|(_, value)| value))
                .unwrap_or_else(|| panic!("cluster state has no {name}"))
        };
        let mut state = Self {
            frontier: value("frontier").parse().expect("cluster frontier is invalid"),
            next: value("next").parse().expect("cluster next cursor is invalid"),
            next_lease: value("next_lease").parse().expect("cluster next lease is invalid"),
            returned: load_set(value("returned")),
            completed: load_map(value("completed")),
            leases: BTreeMap::new(),
            nodes: BTreeMap::new(),
        };
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("lease=") {
                let fields = value.split('|').collect::<Vec<_>>();
                assert_eq!(fields.len(), 5, "cluster lease state has the wrong width");
                state.leases.insert(
                    fields[0].parse().expect("cluster lease id is invalid"),
                    Held {
                        cursor: fields[1].parse().expect("cluster lease cursor is invalid"),
                        node: fields[2].to_owned(),
                        device: fields[3].to_owned(),
                        deadline: fields[4].parse().expect("cluster lease deadline is invalid"),
                    },
                );
            }
            if let Some(value) = line.strip_prefix("node=") {
                let fields = value.split('|').collect::<Vec<_>>();
                assert_eq!(fields.len(), 4, "cluster node state has the wrong width");
                state.nodes.insert(
                    fields[0].to_owned(),
                    Node {
                        digest: fields[1].to_owned(),
                        devices: fields[2].to_owned(),
                        deadline: fields[3].parse().expect("cluster node deadline is invalid"),
                    },
                );
            }
        }
        state
    }

    fn text(&self) -> String {
        let mut text = format!(
            "frontier={}\nnext={}\nnext_lease={}\nreturned={}\ncompleted={}\n",
            self.frontier,
            self.next,
            self.next_lease,
            set(&self.returned),
            map(&self.completed),
        );
        for (id, lease) in &self.leases {
            text.push_str(&format!(
                "lease={id}|{}|{}|{}|{}\n",
                lease.cursor, lease.node, lease.device, lease.deadline
            ));
        }
        for (id, node) in &self.nodes {
            text.push_str(&format!(
                "node={id}|{}|{}|{}\n",
                node.digest, node.devices, node.deadline
            ));
        }
        text
    }
}

impl Coordinator {
    fn persist(&self) {
        if let Some(parent) = self.config.state_path.parent() {
            std::fs::create_dir_all(parent).expect("cannot create cluster state directory");
        }
        let temporary = self.config.state_path.with_extension("next");
        std::fs::write(&temporary, self.state.text()).expect("cannot write cluster state");
        std::fs::rename(temporary, &self.config.state_path).expect("cannot publish cluster state");
    }

    fn expire(&mut self, now: u64) {
        self.state.nodes.retain(|_, node| node.deadline > now);
        let expired = self
            .state
            .leases
            .iter()
            .filter_map(|(id, lease)| (lease.deadline <= now).then_some((*id, lease.cursor)))
            .collect::<Vec<_>>();
        for (id, cursor) in expired {
            self.state.leases.remove(&id);
            assert!(self.state.returned.insert(cursor), "an expired cursor lease was already returned");
        }
    }

    fn register(&mut self, node: &str, digest: &str, devices: &str, now: u64) -> Result<(), String> {
        token(node, "node id");
        token(digest, "binary digest");
        for device in devices.split(',') {
            token(device, "device id");
        }
        if digest != self.config.digest {
            return Err("machine binary digest differs from the coordinator".to_owned());
        }
        self.expire(now);
        if !self.state.nodes.contains_key(node) && self.state.nodes.len() >= self.config.node_limit {
            return Err(format!("cluster already has {} live nodes", self.config.node_limit));
        }
        self.state.nodes.insert(
            node.to_owned(),
            Node {
                digest: digest.to_owned(),
                devices: devices.to_owned(),
                deadline: now + self.config.node_timeout.as_secs(),
            },
        );
        Ok(())
    }

    fn claim(&mut self, node: &str, digest: &str, devices: &str, device: &str) -> Result<Option<Lease>, String> {
        let now = epoch();
        self.register(node, digest, devices, now)?;
        if !devices.split(',').any(|candidate| candidate == device) {
            return Err(format!("node {node} did not register device {device}"));
        }
        let cursor = if let Some(cursor) = self.state.returned.pop_first() {
            cursor
        } else {
            let end = self
                .config
                .batches
                .checked_mul(self.config.cursor_step)
                .and_then(|span| self.config.initial_cursor.checked_add(span))
                .expect("cluster cursor range overflows");
            if self.config.batches != 0 && self.state.next >= end {
                self.persist();
                return Ok(None);
            }
            let cursor = self.state.next;
            self.state.next = self
                .state
                .next
                .checked_add(self.config.cursor_step)
                .expect("cluster cursor overflows");
            cursor
        };
        let id = self.state.next_lease;
        self.state.next_lease = self.state.next_lease.checked_add(1).expect("cluster lease id overflows");
        self.state.leases.insert(
            id,
            Held {
                cursor,
                node: node.to_owned(),
                device: device.to_owned(),
                deadline: now + self.config.cursor_timeout.as_secs(),
            },
        );
        self.persist();
        Ok(Some(Lease { id, cursor }))
    }

    fn held(&mut self, node: &str, id: u64, cursor: u64) -> Result<&mut Held, String> {
        let lease = self.state.leases.get_mut(&id).ok_or_else(|| format!("cursor lease {id} is absent"))?;
        if lease.node != node || lease.cursor != cursor {
            return Err(format!("cursor lease {id} ownership differs"));
        }
        Ok(lease)
    }

    fn renew(&mut self, node: &str, id: u64, cursor: u64) -> Result<(), String> {
        let deadline = epoch() + self.config.cursor_timeout.as_secs();
        self.held(node, id, cursor)?.deadline = deadline;
        if let Some(node) = self.state.nodes.get_mut(node) {
            node.deadline = epoch() + self.config.node_timeout.as_secs();
        }
        self.persist();
        Ok(())
    }

    fn complete(&mut self, node: &str, id: u64, cursor: u64, next: u64, device: &str, packets: Vec<String>) -> Result<u64, String> {
        let held = self.held(node, id, cursor)?;
        if held.device != device {
            return Err(format!("cursor lease {id} device differs"));
        }
        if next <= cursor {
            return Err("completed cursor did not advance".to_owned());
        }
        self.state.leases.remove(&id);
        if self.state.completed.insert(cursor, next).is_some() {
            return Err(format!("cursor {cursor} completed twice"));
        }
        while let Some(next) = self.state.completed.remove(&self.state.frontier) {
            self.state.frontier = next;
        }
        self.persist();
        self.completions
            .send(Completion { node: node.to_owned(), device: device.to_owned(), cursor, packets })
            .map_err(|_| "cluster completion receiver is absent".to_owned())?;
        Ok(self.state.frontier)
    }

    fn release(&mut self, node: &str, id: u64, cursor: u64) -> Result<(), String> {
        self.held(node, id, cursor)?;
        self.state.leases.remove(&id);
        if !self.state.returned.insert(cursor) {
            return Err(format!("cursor {cursor} was already returned"));
        }
        self.persist();
        Ok(())
    }

    fn handle(&mut self, request: &str) -> Result<String, String> {
        let fields = request.trim_end().split('\t').collect::<Vec<_>>();
        let command = fields.first().copied().unwrap_or_default();
        let secret = fields.get(1).copied().unwrap_or_default();
        if secret != self.config.secret {
            return Err("cluster secret differs".to_owned());
        }
        match command {
            "REGISTER" if fields.len() == 5 => {
                self.register(fields[2], fields[3], fields[4], epoch())?;
                self.persist();
                Ok("OK".to_owned())
            }
            "CLAIM" if fields.len() == 6 => self
                .claim(fields[2], fields[3], fields[4], fields[5])
                .map(|lease| lease.map_or_else(|| "EMPTY".to_owned(), |lease| format!("LEASE\t{}\t{}", lease.id, lease.cursor))),
            "RENEW" if fields.len() == 5 => {
                self.renew(
                    fields[2],
                    fields[3].parse().map_err(|_| "lease id is invalid")?,
                    fields[4].parse().map_err(|_| "lease cursor is invalid")?,
                )?;
                Ok("OK".to_owned())
            }
            "COMPLETE" if fields.len() == 7 || fields.len() == 8 => self
                .complete(
                    fields[2],
                    fields[3].parse().map_err(|_| "lease id is invalid")?,
                    fields[4].parse().map_err(|_| "lease cursor is invalid")?,
                    fields[5].parse().map_err(|_| "next cursor is invalid")?,
                    fields[6],
                    unpack_bundle(fields.get(7).copied().unwrap_or_default())?,
                )
                .map(|frontier| format!("FRONTIER\t{frontier}")),
            "RETURN" if fields.len() == 5 => {
                self.release(
                    fields[2],
                    fields[3].parse().map_err(|_| "lease id is invalid")?,
                    fields[4].parse().map_err(|_| "lease cursor is invalid")?,
                )?;
                Ok("OK".to_owned())
            }
            _ => Err("cluster request is invalid".to_owned()),
        }
    }
}

fn serve(stream: &mut TcpStream, coordinator: &Arc<Mutex<Coordinator>>) {
    let mut request = String::new();
    if BufReader::new(&mut *stream).read_line(&mut request).is_err() {
        return;
    }
    let response = coordinator
        .lock()
        .expect("cluster coordinator lock is poisoned")
        .handle(&request)
        .unwrap_or_else(|error| format!("ERROR\t{error}"));
    let _ = writeln!(stream, "{response}");
}

pub fn start_server(config: ServerConfig, completions: mpsc::Sender<Completion>) {
    assert!(config.cursor_step > 0, "cluster cursor step must be positive");
    assert!(config.node_limit > 0, "cluster node limit must be positive");
    assert!(config.node_timeout.as_secs() > 0, "cluster node timeout must be positive");
    assert!(config.cursor_timeout.as_secs() > 0, "cluster cursor timeout must be positive");
    let listener = TcpListener::bind(config.listen).expect("cannot bind the cluster coordinator");
    let state = State::load(&config.state_path, config.initial_cursor);
    let coordinator = Arc::new(Mutex::new(Coordinator { config, state, completions }));
    coordinator.lock().expect("cluster coordinator lock is poisoned").persist();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("cannot accept a cluster connection");
            let coordinator = Arc::clone(&coordinator);
            std::thread::spawn(move || serve(&mut stream, &coordinator));
        }
    });
}

impl Client {
    pub fn new(
        address: SocketAddr,
        secret: String,
        node: String,
        digest: String,
        devices: &[String],
        timeout: Duration,
    ) -> Self {
        token(&node, "node id");
        token(&digest, "binary digest");
        assert!(!devices.is_empty(), "cluster node has no devices");
        for device in devices {
            token(device, "device id");
        }
        Self {
            address,
            secret,
            node,
            digest,
            devices: devices.join(","),
            timeout,
            exhausted: Arc::new(AtomicBool::new(false)),
        }
    }

    fn request(&self, command: &str) -> String {
        let mut stream = TcpStream::connect_timeout(&self.address, self.timeout)
            .expect("cannot connect to the cluster coordinator");
        stream.set_read_timeout(Some(self.timeout)).expect("cannot set cluster read timeout");
        stream.set_write_timeout(Some(self.timeout)).expect("cannot set cluster write timeout");
        writeln!(stream, "{command}").expect("cannot send cluster request");
        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response).expect("cannot read cluster response");
        let response = response.trim_end();
        if let Some(error) = response.strip_prefix("ERROR\t") {
            panic!("cluster coordinator rejected the request: {error}");
        }
        response.to_owned()
    }

    pub fn register(&self) {
        let response = self.request(&format!(
            "REGISTER\t{}\t{}\t{}\t{}",
            self.secret, self.node, self.digest, self.devices
        ));
        assert_eq!(response, "OK", "cluster registration response is invalid");
    }

    pub fn claim(&self, device: &str) -> Option<Lease> {
        let response = self.request(&format!(
            "CLAIM\t{}\t{}\t{}\t{}\t{device}",
            self.secret, self.node, self.digest, self.devices
        ));
        if response == "EMPTY" {
            self.exhausted.store(true, Ordering::Relaxed);
            return None;
        }
        self.exhausted.store(false, Ordering::Relaxed);
        let fields = response.split('\t').collect::<Vec<_>>();
        assert!(fields.len() == 3 && fields[0] == "LEASE", "cluster lease response is invalid");
        Some(Lease {
            id: fields[1].parse().expect("cluster lease id is invalid"),
            cursor: fields[2].parse().expect("cluster lease cursor is invalid"),
        })
    }

    pub fn exhausted(&self) -> bool {
        self.exhausted.load(Ordering::Relaxed)
    }

    pub fn renew(&self, lease: Lease) {
        let response = self.request(&format!(
            "RENEW\t{}\t{}\t{}\t{}",
            self.secret, self.node, lease.id, lease.cursor
        ));
        assert_eq!(response, "OK", "cluster renewal response is invalid");
    }

    pub fn complete(&self, lease: Lease, next: u64, device: &str, packets: &[String]) -> u64 {
        let response = self.request(&format!(
            "COMPLETE\t{}\t{}\t{}\t{}\t{next}\t{device}\t{}",
            self.secret, self.node, lease.id, lease.cursor, packet_bundle(packets)
        ));
        let fields = response.split('\t').collect::<Vec<_>>();
        assert!(fields.len() == 2 && fields[0] == "FRONTIER", "cluster completion response is invalid");
        fields[1].parse().expect("cluster frontier is invalid")
    }

    pub fn release(&self, lease: Lease) {
        let response = self.request(&format!(
            "RETURN\t{}\t{}\t{}\t{}",
            self.secret, self.node, lease.id, lease.cursor
        ));
        assert_eq!(response, "OK", "cluster return response is invalid");
    }
}
