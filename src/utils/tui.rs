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
pub struct Tok {
	pub text: String,
	pub status: TokStatus,
	pub age: u8,
	pub heat: f32,
}
pub enum TokStatus {
	Draft,
	Accepted,
	Recent,
}
pub fn render_toks(toks: &[Tok]) -> String {
	let mut out = String::new();
	for t in toks {
		out.push_str(&t.text);
	}
	out
}
pub fn toks_line(toks: &[Tok]) -> String {
	let mut out = String::new();
	for t in toks {
		let Some(_keep) = Some(()).filter(|_probe| !matches!(t.status, TokStatus::Draft)) else {
			continue;
		};
		out.push_str(&t.text);
	}
	out
}
