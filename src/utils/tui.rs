use ratatui::Frame;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Style};
use ratatui::symbols::{self, Marker};
use ratatui::text::Span;
use ratatui::widgets::{Axis, Block, Chart, Dataset as ChartDataset, GraphType, Paragraph};
use recipe_infer::{Metric, Pt, pt};
use std::io::IsTerminal;

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
			let Some(_vert) =
				Some(()).filter(|_probe| c.symbol() == symbols::line::VERTICAL)
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
	let _drawn = term.draw(|frame| {
		dashboard(frame, summary, rows, ys);
	});
	Some(())
		.filter(|_probe| std::io::stdin().is_terminal())
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
	ratatui::restore();
}
pub use recipe_infer::llm::{Tok, TokStatus, render_toks, toks_line};

use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::Modifier;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Borders, Wrap};
use tui_textarea::TextArea;

struct TermRestore;
impl Drop for TermRestore {
	fn drop(&mut self) {
		ratatui::restore();
	}
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
				.map(|_probe| lines.push(Line::from(std::mem::take(&mut cur))));
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
	let outer =
		Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).split(frame.area());
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
	let _gen = Some(())
		.filter(|_probe| generating)
		.map(|_probe| {
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
	let _guard = TermRestore;
	let mut cur = 0usize;
	loop {
		let _drawn = term.draw(|f| render_peers(f, rows, cur));
		let ev = match event::read() {
			Ok(e) => e,
			Err(_e) => return false,
		};
		let Event::Key(k) = ev else { continue };
		let KeyEventKind::Press = k.kind else { continue };
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

pub fn chat(gguf: &str) {
	crate::some_or_die(std::io::stdin().is_terminal().then_some(()), "chat: needs a tty");
	let mut term = ratatui::init();
	let _guard = TermRestore;
	let mut textarea = new_input();
	let mut scrollback: Vec<(String, String)> = Vec::new();
	let mut err_out: Option<String> = None;
	loop {
		let _idle = term.draw(|f| render_chat(f, &textarea, &scrollback, None, &[], false));
		let ev = match event::read() {
			Ok(e) => e,
			Err(_e) => break,
		};
		match ev {
			Event::Key(k) if k.kind == KeyEventKind::Press => match (k.code, k.modifiers) {
				(KeyCode::Esc, _mods) => break,
				(KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => break,
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
					let res;
					{
						let sb = &scrollback;
						let ta = &textarea;
						let pr = prompt.as_str();
						let mut on_round = |toks: &[Tok]| {
							let _round =
								term.draw(|f| render_chat(f, ta, sb, Some(pr), toks, true));
						};
						on_round(&[]);
						res = recipe_infer::llm::generate(
							std::path::Path::new(gguf),
							&prompt,
							&mut on_round,
						);
					}
					match res {
						Ok(resp) => scrollback.push((prompt, resp)),
						Err(e) => {
							err_out = Some(format!("{e:#}"));
							break;
						}
					}
				}
				_other => {
					let _typed = textarea.input(Event::Key(k));
				}
			},
			Event::Key(_k) => {}
			other => {
				let _typed = textarea.input(other);
			}
		}
	}
	drop(_guard);
	err_out.map(|e| drop(gpu_core::log::Write::err(format!("chat: {e}"))));
}
