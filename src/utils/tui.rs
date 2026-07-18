use ratatui::Frame;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use recipe_infer::{Metric, Pt, pt};
use std::fs::OpenOptions;
use std::io::{self, IsTerminal};
use std::mem;
use std::os::unix::io::AsRawFd as _;
use std::path::Path;

#[derive(Clone, Copy)]
struct Rgb {
	r: u8,
	g: u8,
	b: u8,
}
enum CellFix {
	Fill,
	Corner,
	Leave,
}
struct Found {
	cx: u16,
	style: Style,
}
const PALETTE: [Rgb; 12] = [
	Rgb {
		r: 242,
		g: 40,
		b: 60,
	},
	Rgb {
		r: 39,
		g: 125,
		b: 255,
	},
	Rgb {
		r: 0,
		g: 174,
		b: 107,
	},
	Rgb {
		r: 255,
		g: 194,
		b: 0,
	},
	Rgb {
		r: 215,
		g: 46,
		b: 130,
	},
	Rgb {
		r: 135,
		g: 90,
		b: 251,
	},
	Rgb {
		r: 255,
		g: 122,
		b: 0,
	},
	Rgb {
		r: 91,
		g: 192,
		b: 235,
	},
	Rgb {
		r: 157,
		g: 121,
		b: 188,
	},
	Rgb {
		r: 46,
		g: 83,
		b: 57,
	},
	Rgb {
		r: 3,
		g: 252,
		b: 186,
	},
	Rgb {
		r: 194,
		g: 1,
		b: 20,
	},
];
fn palette(i: usize) -> Rgb {
	PALETTE[i % PALETTE.len()]
}
fn symlog(y: f64) -> f64 {
	Some(())
		.filter(|_probe| y.abs() <= 1.0)
		.map(|_probe| y)
		.unwrap_or_else(|| y.signum() * (1.0 + y.abs().log10()))
}
fn inv_symlog(v: f64) -> f64 {
	Some(())
		.filter(|_probe| v.abs() <= 1.0)
		.map(|_probe| v)
		.unwrap_or_else(|| v.signum() * 10f64.powf(v.abs() - 1.0))
}
fn fmt_time_axis(secs: f64) -> String {
	Some(())
		.filter(|_probe| secs >= 3600.0)
		.map(|_probe| format!("{:.1}h", secs / 3600.0))
		.or_else(|| {
			Some(())
				.filter(|_probe| secs >= 60.0)
				.map(|_probe| format!("{:.1}m", secs / 60.0))
		})
		.unwrap_or_else(|| format!("{secs:.0}s"))
}
fn fmt_axis(v: f64) -> String {
	let a = v.abs();
	Some(())
		.filter(|_probe| a >= 1000.0 || (a > 0.0 && a < 0.01))
		.map(|_probe| format!("{v:.1e}"))
		.or_else(|| {
			Some(())
				.filter(|_probe| a >= 1.0)
				.map(|_probe| format!("{v:.1}"))
		})
		.unwrap_or_else(|| format!("{v:.3}"))
}
pub fn dashboard(frame: &mut Frame, summary: &str, rows: &[Vec<f64>], ys: &[Metric]) {
	let header_h = summary.lines().count() as u16;
	let mut constraints = vec![Constraint::Length(header_h)];
	constraints.extend(ys.iter().map(|_m| Constraint::Fill(1)));
	let areas = Layout::vertical(constraints).split(frame.area());
	frame.render_widget(Paragraph::new(summary), areas[0]);
	let xmax = rows.last().map_or(1.0, |r| r[0]).max(1.0);
	let lxmax = xmax.log10().max(1e-9);
	for j in 0..ys.len() {
		let m = ys[j];
		let lo = rows
			.iter()
			.map(|r| symlog(r[1 + j]))
			.filter(|y| y.is_finite())
			.fold(f64::INFINITY, f64::min);
		let hi = rows
			.iter()
			.map(|r| symlog(r[1 + j]))
			.filter(|y| y.is_finite())
			.fold(f64::NEG_INFINITY, f64::max);
		let pts: Vec<Pt> = rows
			.iter()
			.map(|r| pt(r[0].max(1.0).log10(), symlog(r[1 + j])))
			.collect();
		let mut ymin = 0.0;
		let mut ymax = 1.0;
		match Some(()).filter(|_probe| hi > lo) {
			Some(_spread) => {
				let pad = (hi - lo) * 0.05;
				ymin = lo - pad;
				ymax = hi + pad;
			}
			None => {
				let finite = Some(()).filter(|_probe| lo.is_finite());
				ymin = finite.map(|_probe| lo - 1.0).unwrap_or(ymin);
				ymax = finite.map(|_probe| lo + 1.0).unwrap_or(ymax);
			}
		}
		let real_lo = Some(())
			.filter(|_probe| lo.is_finite())
			.map(|_probe| inv_symlog(lo))
			.unwrap_or(0.0);
		let real_hi = Some(())
			.filter(|_probe| hi.is_finite())
			.map(|_probe| inv_symlog(hi))
			.unwrap_or(1.0);
		let c = palette(j);
		let color = Color::Rgb(c.r, c.g, c.b);
		let ds = ChartDataset::default()
			.marker(Marker::Braille)
			.graph_type(GraphType::Line)
			.style(Style::default().fg(color))
			.data(&pts);
		let cur = rows.last().map_or(f64::NAN, |r| r[1 + j]);
		let title = Span::styled(
			format!("{} = {}", m.fmt().label, fmt_axis(cur)),
			Style::default().fg(color),
		);
		let chart = Chart::new(vec![ds])
			.block(Block::default().title(title))
			.x_axis(Axis::default().bounds([0.0, lxmax]).labels([
				String::new(),
				String::new(),
				fmt_time_axis(10f64.powf(lxmax)),
			]))
			.y_axis(Axis::default().bounds([ymin, ymax]).labels([
				format!("{:>12}", fmt_axis(real_lo)),
				format!("{:>12}", fmt_axis(real_hi)),
			]));
		frame.render_widget(chart, areas[j + 1]);
	}
	let Some(first) = areas.get(1).copied() else {
		return;
	};
	let last = areas[areas.len() - 1];
	let buf = frame.buffer_mut();
	let mut found: Option<Found> = None;
	'find: for x in first.left()..first.right() {
		for y in first.top()..first.bottom() {
			let Some(c) = buf.cell(Position::new(x, y)) else {
				continue;
			};
			let Some(_vert) = Some(()).filter(|_probe| c.symbol() == symbols::line::VERTICAL)
			else {
				continue;
			};
			found = Some(Found {
				cx: x,
				style: c.style(),
			});
			break 'find;
		}
	}
	let Some(f) = found else {
		return;
	};
	for y in first.top()..last.bottom().saturating_sub(1) {
		let Some(c) = buf.cell_mut(Position::new(f.cx, y)) else {
			continue;
		};
		let sym = c.symbol();
		let blank = sym == " " || sym.is_empty();
		let corner = sym == symbols::line::BOTTOM_LEFT && y < last.top();
		let fix = Some(())
			.filter(|_probe| blank)
			.map(|_probe| CellFix::Fill)
			.or_else(|| {
				Some(())
					.filter(|_probe| corner)
					.map(|_probe| CellFix::Corner)
			})
			.unwrap_or(CellFix::Leave);
		match fix {
			CellFix::Fill => {
				c.set_symbol(symbols::line::VERTICAL);
				c.set_style(f.style);
			}
			CellFix::Corner => {
				c.set_symbol(symbols::line::VERTICAL_RIGHT);
			}
			CellFix::Leave => continue,
		}
	}
}
pub fn show(summary: &str, rows: &[Vec<f64>], ys: &[Metric]) {
	let mut term = ratatui::init();
	let _guard = TermRestore::new();
	let _drawn = term.draw(|frame| {
		dashboard(frame, summary, rows, ys);
	});
	Some(())
		.filter(|_probe| io::stdin().is_terminal())
		.map(|_probe| {
			loop {
				match event::read() {
					Err(_err) => break,
					Ok(ev) => match ev {
						Event::Key(_key) => break,
						_other => continue,
					},
				}
			}
		})
		.unwrap_or(());
}
pub use recipe_infer::llm::{Tok, TokStatus, render_toks, toks_line};

use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::Modifier;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Borders, Wrap};
use tui_textarea::TextArea;

/// Owns the terminal for a TUI session: while alive, fd 2 is redirected to a log
/// file so `Write::*` diagnostics from generation cannot corrupt the ratatui
/// screen. Dropping restores the terminal and fd 2, then reports the log path.
struct TermRestore {
	saved_stderr: i32,
}
impl TermRestore {
	fn new() -> Self {
		let log_path = std::env::var("RECIPE_TUI_LOG")
			.unwrap_or_else(|_e| "/tmp/recipe-tui.log".to_owned());
		let saved_stderr = redirect_stderr(&log_path);
		drop(ratatui::crossterm::execute!(
			io::stdout(),
			event::EnableBracketedPaste
		));
		Self { saved_stderr }
	}
}
impl Drop for TermRestore {
	fn drop(&mut self) {
		drop(ratatui::crossterm::execute!(
			io::stdout(),
			event::DisableBracketedPaste
		));
		ratatui::restore();
		if self.saved_stderr >= 0 {
			// SAFETY: saved_stderr is a live dup of the original fd 2 from redirect_stderr; dup2 restores it and close releases the dup.
			unsafe {
				libc::dup2(self.saved_stderr, 2);
				libc::close(self.saved_stderr);
			}
		}
	}
}

/// Redirects fd 2 to `path` (truncating it), returning a dup of the original fd 2
/// to restore later, or `-1` if the redirect could not be set up.
fn redirect_stderr(path: &str) -> i32 {
	let Ok(file) = OpenOptions::new().create(true).write(true).truncate(true).open(path) else {
		return -1;
	};
	// SAFETY: dup/dup2 on fd 2 and the open file's fd; on success fd 2 aliases the file and `saved` keeps the original open.
	let saved = unsafe {
		let saved = libc::dup(2);
		if saved >= 0 {
			libc::dup2(file.as_raw_fd(), 2);
		}
		saved
	};
	return saved;
}

fn new_input() -> TextArea<'static> {
	let mut ta = TextArea::default();
	ta.set_block(
		Block::default()
			.borders(Borders::ALL)
			.title("message  (Enter send, Esc quit)"),
	);
	ta.set_cursor_line_style(Style::default());
	ta
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
	let t = t.clamp(0.0, 1.0);
	let v = a as f32 + (b as f32 - a as f32) * t;
	v.round().clamp(0.0, 255.0) as u8
}

fn tok_style(t: &Tok) -> Style {
	match t.status {
		TokStatus::Draft => Style::default()
			.fg(Color::DarkGray)
			.add_modifier(Modifier::DIM),
		TokStatus::Recent => {
			let h = t.heat.clamp(0.0, 1.0);
			let g = lerp_u8(140, 245, h);
			let b = lerp_u8(30, 120, h);
			Style::default()
				.fg(Color::Rgb(255, g, b))
				.add_modifier(Modifier::BOLD)
		}
		TokStatus::Accepted => {
			let a = t.age.min(6) as f32 / 6.0;
			let r = lerp_u8(90, 190, a);
			let g = lerp_u8(170, 190, a);
			let b = lerp_u8(255, 190, a);
			Style::default().fg(Color::Rgb(r, g, b))
		}
	}
}

fn toks_to_lines(toks: &[Tok]) -> Vec<Line<'static>> {
	let mut lines: Vec<Line> = Vec::new();
	let mut cur: Vec<Span> = Vec::new();
	for t in toks {
		let st = tok_style(t);
		let segs: Vec<&str> = t.text.split('\n').collect();
		for (i, seg) in segs.iter().enumerate() {
			let _break = Some(())
				.filter(|_probe| i > 0)
				.map(|_probe| lines.push(Line::from(mem::take(&mut cur))));
			let _push = Some(())
				.filter(|_probe| !seg.is_empty())
				.map(|_probe| cur.push(Span::styled((*seg).to_string(), st)));
		}
	}
	lines.push(Line::from(cur));
	lines
}

fn render_chat(
	frame: &mut Frame,
	input: &TextArea,
	scrollback: &[(String, String)],
	pending: Option<&str>,
	live: &[Tok],
	generating: bool,
) {
	let outer = Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(frame.area());
	let head = Style::default()
		.fg(Color::Rgb(120, 200, 255))
		.add_modifier(Modifier::BOLD);
	let mut lines: Vec<Line> = Vec::new();
	for (p, r) in scrollback {
		lines.push(Line::from(Span::styled(format!("> {p}"), head)));
		for seg in r.split('\n') {
			lines.push(Line::from(seg.to_string()));
		}
		lines.push(Line::from(String::new()));
	}
	let _pend = pending.map(|p| lines.push(Line::from(Span::styled(format!("> {p}"), head))));
	for l in toks_to_lines(live) {
		lines.push(l);
	}
	let _gen = Some(()).filter(|_probe| generating).map(|_probe| {
		lines.push(Line::from(Span::styled(
			"  generating (model load + rounds, this takes minutes)".to_string(),
			Style::default()
				.fg(Color::Yellow)
				.add_modifier(Modifier::ITALIC),
		)))
	});
	let text = Text::from(lines);
	let total = text.lines.len();
	let scroll = total.saturating_sub(outer[0].height as usize) as u16;
	let para = Paragraph::new(text)
		.wrap(Wrap { trim: false })
		.scroll((scroll, 0));
	frame.render_widget(para, outer[0]);
	frame.render_widget(input, outer[1]);
}

pub struct PeerRow {
	pub host: String,
	pub detail: String,
	pub selected: bool,
	pub local: bool,
}

fn render_peers(frame: &mut Frame, rows: &[PeerRow], cur: usize) {
	let mut lines: Vec<Line> = Vec::new();
	lines.push(Line::from(Span::styled(
		"pool  (arrows move, space toggle, enter save, q quit)",
		Style::default()
			.fg(Color::Rgb(120, 200, 255))
			.add_modifier(Modifier::BOLD),
	)));
	lines.push(Line::from(""));
	for (i, r) in rows.iter().enumerate() {
		let mark = match r.selected {
			true => "[x]",
			false => "[ ]",
		};
		let tag = match r.local {
			true => "  (this machine)",
			false => "",
		};
		let base = match r.selected {
			true => Style::default(),
			false => Style::default().fg(Color::DarkGray),
		};
		let style = match i == cur {
			true => base.add_modifier(Modifier::REVERSED),
			false => base,
		};
		lines.push(Line::from(Span::styled(
			format!(" {mark} {}{tag}  {}", r.host, r.detail),
			style,
		)));
	}
	let para = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
	frame.render_widget(para, frame.area());
}

pub fn peers_picker(rows: &mut [PeerRow]) -> bool {
	let mut term = ratatui::init();
	let _guard = TermRestore::new();
	let mut cur = 0usize;
	loop {
		let _drawn = term.draw(|f| render_peers(f, rows, cur));
		let ev = match event::read() {
			Ok(e) => e,
			Err(_e) => return false,
		};
		let Event::Key(k) = ev else { continue };
		let KeyEventKind::Press = k.kind else {
			continue;
		};
		match (k.code, k.modifiers) {
			(KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => return false,
			(KeyCode::Esc, _mods) | (KeyCode::Char('q'), _mods) => return false,
			(KeyCode::Enter, _mods) => return true,
			(KeyCode::Up, _mods) | (KeyCode::Char('k'), _mods) => {
				cur = cur.saturating_sub(1);
			}
			(KeyCode::Down, _mods) | (KeyCode::Char('j'), _mods) => {
				cur = (cur + 1).min(rows.len().saturating_sub(1));
			}
			(KeyCode::Char(' '), _mods) => {
				for r in rows.get_mut(cur).into_iter() {
					r.selected = !r.selected;
				}
			}
			_other => {}
		}
	}
}

fn render_models(frame: &mut Frame, names: &[String], cur: usize) {
	let mut lines: Vec<Line> = Vec::new();
	lines.push(Line::from(Span::styled(
		"models  (arrows move, enter load, q quit)",
		Style::default()
			.fg(Color::Rgb(120, 200, 255))
			.add_modifier(Modifier::BOLD),
	)));
	lines.push(Line::from(""));
	for (i, name) in names.iter().enumerate() {
		let style = match i == cur {
			true => Style::default().add_modifier(Modifier::REVERSED),
			false => Style::default(),
		};
		lines.push(Line::from(Span::styled(format!(" {name}"), style)));
	}
	let para = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
	frame.render_widget(para, frame.area());
}

/// Single-select dropdown over the gguf.toml model names; returns the chosen
/// index, or `None` if the user quits without selecting. Mirrors the
/// [`peers_picker`] ratatui style.
pub fn model_picker(names: &[String]) -> Option<usize> {
	if !io::stdin().is_terminal() {
		gpu_core::log::Write::error("run: needs a tty");
		return None;
	}
	let mut term = ratatui::init();
	let _guard = TermRestore::new();
	let mut cur = 0usize;
	loop {
		let _drawn = term.draw(|f| render_models(f, names, cur));
		let ev = match event::read() {
			Ok(e) => e,
			Err(_e) => return None,
		};
		let Event::Key(k) = ev else { continue };
		let KeyEventKind::Press = k.kind else {
			continue;
		};
		match (k.code, k.modifiers) {
			(KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => return None,
			(KeyCode::Esc, _mods) | (KeyCode::Char('q'), _mods) => return None,
			(KeyCode::Enter, _mods) => return Some(cur),
			(KeyCode::Up, _mods) | (KeyCode::Char('k'), _mods) => {
				cur = cur.saturating_sub(1);
			}
			(KeyCode::Down, _mods) | (KeyCode::Char('j'), _mods) => {
				cur = (cur + 1).min(names.len().saturating_sub(1));
			}
			_other => {}
		}
	}
}

/// The one paste path: CRLF/CR normalized to LF, trailing newlines stripped so a
/// paste never submits, interior newlines kept as textarea line breaks.
fn insert_paste(textarea: &mut TextArea, s: &str) {
	let text = s.replace("\r\n", "\n").replace('\r', "\n");
	let _ins = textarea.insert_str(text.trim_end_matches('\n'));
}

/// Non-blocking input drain, waiting up to `wait_ms` for the first event (frame
/// pacing): Esc or Ctrl-C sets `cancel`, a bracketed paste lands in `textarea`
/// via [`insert_paste`] so text pasted mid-reply survives into the next message,
/// and other keys are ignored while a reply streams.
fn drain_input(cancel: &std::sync::atomic::AtomicBool, textarea: &mut TextArea, wait_ms: u64) {
	let mut ready = event::poll(std::time::Duration::from_millis(wait_ms)).unwrap_or(false);
	while ready {
		match event::read() {
			Ok(Event::Paste(s)) => insert_paste(textarea, &s),
			Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
				let ctrl_c =
					k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL);
				if ctrl_c || k.code == KeyCode::Esc {
					cancel.store(true, std::sync::atomic::Ordering::Relaxed);
				}
			}
			Ok(_e) => {}
			Err(_e) => break,
		}
		ready = event::poll(std::time::Duration::from_millis(0)).unwrap_or(false);
	}
}

pub fn render_once(gguf: &str, prompt: &str) {
	if !io::stdin().is_terminal() {
		gpu_core::log::Write::error("render: needs a tty");
		return;
	}
	let mut term = ratatui::init();
	let _guard = TermRestore::new();
	let mut input = new_input();
	let mut scrollback: Vec<(String, String)> = Vec::new();
	let cancel = std::sync::atomic::AtomicBool::new(false);
	let res = {
		let sb = &scrollback;
		let mut on_round = |toks: &[Tok]| -> bool {
			let _round = term.draw(|f| render_chat(f, &input, sb, Some(prompt), toks, true));
			drain_input(&cancel, &mut input, 0);
			!cancel.load(std::sync::atomic::Ordering::Relaxed)
		};
		let _first = on_round(&[]);
		recipe_infer::llm::generate(Path::new(gguf), prompt, &mut on_round)
	};
	match res {
		Ok(resp) => scrollback.push((prompt.to_string(), resp)),
		Err(e) => {
			drop(_guard);
			gpu_core::log::Write::error(format!("render: {e:#}"));
			return;
		}
	}
	let _final = term.draw(|f| render_chat(f, &input, &scrollback, None, &[], false));
	loop {
		match event::read() {
			Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => break,
			Ok(_e) => {}
			Err(_e) => break,
		}
	}
}

/// Events streamed from the session worker to the chat UI thread: live token
/// snapshots during load or decode, the load terminal signals, and the finished
/// reply for a message (Ok body or a formatted error string).
enum FromWorker {
	Snap(Vec<Tok>),
	Loaded,
	LoadEnded(Option<String>),
	Reply(std::result::Result<String, String>),
}

enum LoadOutcome {
	Loaded,
	Cancelled,
	Failed(String),
}

/// Owns the resident [`ChatSession`] for the chat's lifetime on its own thread:
/// loads the weights once (streaming snapshots + honoring the cancel flag), then
/// serves each prompt from `prompts` by calling `generate_in` against those same
/// weights — no reload between messages. Exits when `prompts` closes (chat quit),
/// dropping the session so the weights free exactly once.
fn session_worker(
	gguf: &Path,
	prompts: std::sync::mpsc::Receiver<String>,
	ev: std::sync::mpsc::Sender<FromWorker>,
	cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
	let opened = {
		let mut load_round = |toks: &[Tok]| -> bool {
			let _snap = ev.send(FromWorker::Snap(toks.to_vec()));
			return !cancel.load(std::sync::atomic::Ordering::Relaxed);
		};
		recipe_infer::llm::ChatSession::open(gguf, &mut load_round)
	};
	let mut session = match opened {
		Err(e) => {
			let _sent = ev.send(FromWorker::LoadEnded(Some(format!("{e:#}"))));
			return;
		}
		Ok(None) => {
			let _sent = ev.send(FromWorker::LoadEnded(None));
			return;
		}
		Ok(Some(s)) => s,
	};
	let _ready = ev.send(FromWorker::Loaded);
	loop {
		let prompt = match prompts.recv() {
			Ok(p) => p,
			Err(_closed) => return,
		};
		cancel.store(false, std::sync::atomic::Ordering::Relaxed);
		let mut on_round = |toks: &[Tok]| -> bool {
			let _snap = ev.send(FromWorker::Snap(toks.to_vec()));
			return !cancel.load(std::sync::atomic::Ordering::Relaxed);
		};
		let r = session.generate_in(&prompt, &mut on_round);
		let _reply = ev.send(FromWorker::Reply(r.map_err(|e| format!("{e:#}"))));
	}
}

/// Renders the load screen and polls keys until the worker reports the load
/// finished, was cancelled (Esc/Ctrl-C), or failed. A cancel sets the shared
/// flag so `ChatSession::open` returns cleanly.
fn wait_load(
	term: &mut ratatui::DefaultTerminal,
	textarea: &mut TextArea,
	scrollback: &[(String, String)],
	ev_rx: &std::sync::mpsc::Receiver<FromWorker>,
	cancel: &std::sync::atomic::AtomicBool,
) -> LoadOutcome {
	let mut latest: Vec<Tok> = Vec::new();
	loop {
		loop {
			match ev_rx.try_recv() {
				Ok(FromWorker::Snap(t)) => latest = t,
				Ok(FromWorker::Loaded) => return LoadOutcome::Loaded,
				Ok(FromWorker::LoadEnded(None)) => return LoadOutcome::Cancelled,
				Ok(FromWorker::LoadEnded(Some(e))) => return LoadOutcome::Failed(e),
				Ok(FromWorker::Reply(_r)) => {}
				Err(std::sync::mpsc::TryRecvError::Empty) => break,
				Err(std::sync::mpsc::TryRecvError::Disconnected) => {
					return LoadOutcome::Failed("worker thread ended during load".to_string());
				}
			}
		}
		let _live = term
			.draw(|f| render_chat(f, textarea, scrollback, Some("(loading model…)"), &latest, true));
		drain_input(cancel, textarea, 50);
	}
}

/// Sends one already-templated prompt's decode span through the UI: streams
/// token snapshots and polls keys every 50ms so Esc/Ctrl-C cancels the
/// generation (setting the flag) while leaving the session alive for the next
/// message. Returns the worker's reply, or `None` if the worker channel closed.
fn run_message(
	term: &mut ratatui::DefaultTerminal,
	textarea: &mut TextArea,
	scrollback: &[(String, String)],
	prompt: &str,
	ev_rx: &std::sync::mpsc::Receiver<FromWorker>,
	cancel: &std::sync::atomic::AtomicBool,
) -> Option<std::result::Result<String, String>> {
	let mut latest: Vec<Tok> = Vec::new();
	loop {
		loop {
			match ev_rx.try_recv() {
				Ok(FromWorker::Snap(t)) => latest = t,
				Ok(FromWorker::Reply(r)) => return Some(r),
				Ok(_other) => {}
				Err(std::sync::mpsc::TryRecvError::Empty) => break,
				Err(std::sync::mpsc::TryRecvError::Disconnected) => return None,
			}
		}
		let _live =
			term.draw(|f| render_chat(f, textarea, scrollback, Some(prompt), &latest, true));
		drain_input(cancel, textarea, 50);
	}
}

pub fn chat(gguf: &str) {
	if !io::stdin().is_terminal() {
		gpu_core::log::Write::error("chat: needs a tty");
		return;
	}
	let mut term = ratatui::init();
	let _guard = TermRestore::new();
	let mut textarea = new_input();
	let mut scrollback: Vec<(String, String)> = Vec::new();

	let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
	let (prompt_tx, prompt_rx) = std::sync::mpsc::channel::<String>();
	let (ev_tx, ev_rx) = std::sync::mpsc::channel::<FromWorker>();
	let gguf_owned = std::path::PathBuf::from(gguf);
	let flag = std::sync::Arc::clone(&cancel);
	let worker = std::thread::spawn(move || session_worker(&gguf_owned, prompt_rx, ev_tx, flag));

	match wait_load(&mut term, &mut textarea, &scrollback, &ev_rx, &cancel) {
		LoadOutcome::Loaded => {}
		LoadOutcome::Cancelled => {
			drop(prompt_tx);
			let _joined = worker.join();
			drop(_guard);
			return;
		}
		LoadOutcome::Failed(e) => {
			drop(prompt_tx);
			let _joined = worker.join();
			drop(_guard);
			gpu_core::log::Write::error(format!("chat: load failed: {e}"));
			return;
		}
	}

	loop {
		let _idle = term.draw(|f| render_chat(f, &textarea, &scrollback, None, &[], false));
		let ev = match event::read() {
			Ok(e) => e,
			Err(_e) => break,
		};
		match ev {
			Event::Key(k) if k.kind == KeyEventKind::Press => match (k.code, k.modifiers) {
				(KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break,
				(KeyCode::Esc, _mods) => {
					let Some(_clear) = Some(()).filter(|_probe| !textarea.lines().join("").trim().is_empty()) else {
						break;
					};
					textarea = new_input();
				}
				(KeyCode::Enter, _mods) => {
					let joined = textarea.lines().join(" ");
					let prompt: String = joined
						.chars()
						.filter(|c| *c != '\r' && *c != '\n')
						.collect::<String>()
						.trim()
						.to_string();
					textarea = new_input();
					let Some(_run) = Some(()).filter(|_probe| !prompt.is_empty()) else {
						continue;
					};
					let mut history: Vec<recipe_infer::chat::Msg> = Vec::new();
					for (u, r) in &scrollback {
						let keep = !u.is_empty() && !r.starts_with("error: ") && !r.starts_with("note: ");
						let Some(_ok) = Some(()).filter(|_probe| keep) else {
							continue;
						};
						history.push(recipe_infer::chat::Msg::new("user", u.clone()));
						history.push(recipe_infer::chat::Msg::new("assistant", r.clone()));
					}
					history.push(recipe_infer::chat::Msg::new("user", prompt.clone()));
					let templated = recipe_infer::chat::render_chat(Path::new(gguf), &history, true);
					let (send, note): (String, Option<&str>) = match templated {
						Ok(s) => (s, None),
						Err(_e) => (prompt.clone(), Some("note: no chat template in gguf; multi-turn history disabled")),
					};
					if prompt_tx.send(send).is_err() {
						break;
					}
					match run_message(&mut term, &mut textarea, &scrollback, &prompt, &ev_rx, &cancel) {
						Some(Ok(resp)) => {
							let shown = note.map(|n| format!("{n}\n{resp}")).unwrap_or(resp);
							scrollback.push((prompt, shown));
						}
						Some(Err(e)) => scrollback.push((prompt, format!("error: {e}"))),
						None => break,
					}
				}
				_other => {
					let _typed = textarea.input(Event::Key(k));
				}
			},
			Event::Key(_k) => {}
			Event::Paste(s) => insert_paste(&mut textarea, &s),
			other => {
				let _typed = textarea.input(other);
			}
		}
	}
	drop(prompt_tx);
	let _joined = worker.join();
	drop(_guard);
}
