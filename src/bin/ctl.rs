//! Live control TUI for wl-walls.
//!
//! Interactive mode (no args): full-screen parameter editor with sliders.
//! CLI mode (with args): send a single command and print the response.
//!
//!   wl-walls-ctl                     # interactive TUI
//!   wl-walls-ctl get                 # dump all params
//!   wl-walls-ctl set alpha 0.5       # set a param
//!   wl-walls-ctl set shape lorenz    # switch shape
//!   wl-walls-ctl restart             # clear + redraw
//!   wl-walls-ctl randomize           # new random pattern
//!   wl-walls-ctl next-color          # cycle color
//!   wl-walls-ctl next-shape          # cycle shape

use std::f64::consts::TAU;
use std::io::{self, Read, Write as _};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use rat_widgets::Slider;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

// ---------------------------------------------------------------------------
// Socket IPC
// ---------------------------------------------------------------------------

fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(dir).join("wl-walls.sock")
}

fn send_command(cmd: &str) -> Result<String, String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "cannot connect to {} — is wl-walls running?\n({})",
            path.display(),
            e
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_millis(2000)))
        .ok();
    stream.write_all(cmd.as_bytes()).map_err(|e| e.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    Ok(response)
}

fn send_set(key: &str, value: f64) {
    let _ = send_command(&format!("set {} {}", key, value));
}

fn send_action(action: &str) {
    let _ = send_command(action);
}

// ---------------------------------------------------------------------------
// Parameter definitions
// ---------------------------------------------------------------------------

struct Param {
    key: String,
    label: String,
    min: f64,
    max: f64,
    step: f64,
    fine: f64,
    decimals: usize,
    value: f64,
}

struct Section {
    name: String,
    start: usize,
    count: usize,
}

/// Known parameter ranges. Returns (min, max, step, fine, decimals).
fn param_spec(key: &str) -> (f64, f64, f64, f64, usize) {
    match key {
        // Drawing
        "line_width" => (0.5, 20.0, 0.5, 0.1, 1),
        "alpha" => (0.01, 1.0, 0.05, 0.01, 2),
        "fade" => (0.0, 0.1, 0.005, 0.001, 4),
        "speed" => (1.0, 50.0, 1.0, 1.0, 0),
        // Dithering
        "dither" => (0.0, 1.0, 0.05, 0.01, 2),
        "dither_levels" => (2.0, 256.0, 4.0, 1.0, 0),
        "dither_scale" => (1.0, 8.0, 0.5, 0.1, 1),
        // Harmonograph pendulum params
        k if k.ends_with(".freq") => (0.1, 16.0, 0.1, 0.01, 3),
        k if k.ends_with(".amp") || k.ends_with(".amplitude") => (0.0, 2.0, 0.05, 0.01, 3),
        k if k.ends_with(".phase") => (0.0, TAU, 0.1, 0.01, 3),
        k if k.ends_with(".damping") => (0.0, 0.05, 0.001, 0.0001, 4),
        // Spirograph
        "spiro.big_r" => (0.1, 3.0, 0.1, 0.01, 3),
        "spiro.small_r" => (0.01, 2.0, 0.05, 0.005, 3),
        "spiro.offset" => (0.01, 2.0, 0.05, 0.01, 3),
        "spiro.inner" => (0.0, 1.0, 1.0, 1.0, 0),
        // Lissajous
        "liss.freq_a" | "liss.freq_b" => (0.1, 16.0, 0.1, 0.01, 3),
        "liss.delta" => (0.0, TAU, 0.1, 0.01, 3),
        // Rose
        "rose.k" => (0.1, 12.0, 0.1, 0.01, 3),
        "rose.k2" => (0.0, 12.0, 0.5, 0.1, 3),
        "rose.mix" => (0.0, 1.0, 0.05, 0.01, 3),
        // Butterfly
        "bfly.wing_freq" => (1.0, 10.0, 0.5, 0.1, 1),
        "bfly.tail_freq" => (4.0, 48.0, 2.0, 0.5, 1),
        // Lorenz
        "lorenz.sigma" => (1.0, 30.0, 0.5, 0.1, 1),
        "lorenz.rho" => (1.0, 60.0, 1.0, 0.1, 1),
        "lorenz.beta" => (0.1, 10.0, 0.2, 0.05, 2),
        "lorenz.dt" => (0.001, 0.02, 0.001, 0.0001, 4),
        "lorenz.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // Wireframe
        k if k.starts_with("wire.rot_") => (-0.01, 0.01, 0.0005, 0.0001, 4),
        "wire.perspective" => (1.5, 8.0, 0.5, 0.1, 1),
        "wire.scale" => (0.5, 4.0, 0.2, 0.05, 2),
        "wire.damping" => (0.0, 0.0002, 0.00001, 0.000002, 5),
        "wire.steps_per_edge" => (10.0, 200.0, 10.0, 5.0, 0),
        "wire.max_steps" => (5000.0, 200000.0, 5000.0, 1000.0, 0),
        // Torus knot
        "knot.p" | "knot.q" => (1.0, 12.0, 1.0, 1.0, 0),
        "knot.big_r" => (0.3, 2.0, 0.1, 0.02, 2),
        "knot.small_r" => (0.05, 1.0, 0.05, 0.01, 2),
        k if k.starts_with("knot.rot_") => (-0.01, 0.01, 0.0005, 0.0001, 4),
        "knot.perspective" => (1.5, 8.0, 0.5, 0.1, 1),
        "knot.scale" => (0.5, 3.0, 0.1, 0.02, 2),
        "knot.damping" => (0.0, 0.0002, 0.00001, 0.000002, 5),
        "knot.dt" => (0.001, 0.05, 0.002, 0.0005, 3),
        "knot.max_t" => (20.0, 500.0, 20.0, 5.0, 0),
        // Clifford attractor
        "cliff.a" | "cliff.b" => (-3.0, 3.0, 0.1, 0.01, 2),
        "cliff.c" | "cliff.d" => (-2.0, 2.0, 0.1, 0.01, 2),
        "cliff.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // De Jong attractor
        "dj.a" | "dj.b" | "dj.c" | "dj.d" => (-3.14, 3.14, 0.1, 0.01, 2),
        "dj.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // Superformula
        "sf.m" => (1.0, 20.0, 1.0, 0.1, 1),
        "sf.n1" => (0.1, 100.0, 1.0, 0.1, 1),
        "sf.n2" | "sf.n3" => (0.1, 100.0, 1.0, 0.1, 1),
        "sf.a" | "sf.b" => (0.1, 3.0, 0.1, 0.01, 2),
        // Guilloché
        "guil.r0" => (0.1, 1.0, 0.05, 0.01, 2),
        k if k.starts_with("guil.amp") => (0.0, 0.5, 0.02, 0.005, 3),
        k if k.starts_with("guil.freq") => (1.0, 60.0, 1.0, 0.1, 1),
        k if k.starts_with("guil.phase") => (0.0, TAU, 0.1, 0.01, 3),
        // Double pendulum
        "dpend.l1" | "dpend.l2" => (0.1, 0.9, 0.05, 0.01, 2),
        "dpend.m2" => (0.1, 5.0, 0.2, 0.05, 2),
        "dpend.g" => (1.0, 20.0, 1.0, 0.1, 1),
        "dpend.dt" => (0.001, 0.02, 0.001, 0.0001, 4),
        "dpend.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // Rössler attractor
        "ross.a" => (0.05, 0.5, 0.02, 0.005, 3),
        "ross.b" => (0.05, 0.5, 0.02, 0.005, 3),
        "ross.c" => (2.0, 18.0, 0.5, 0.1, 1),
        "ross.dt" => (0.001, 0.05, 0.002, 0.0005, 3),
        "ross.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // 3D Surface
        "surf.kind" => (0.0, 5.0, 1.0, 1.0, 0),
        "surf.big_r" => (0.1, 1.5, 0.05, 0.01, 2),
        "surf.small_r" => (0.05, 0.5, 0.02, 0.005, 3),
        "surf.wraps" => (5.0, 200.0, 5.0, 1.0, 0),
        "surf.height" => (0.5, 3.0, 0.1, 0.02, 2),
        "surf.dt" => (0.001, 0.02, 0.001, 0.0001, 4),
        k if k.starts_with("surf.rot_") => (-0.01, 0.01, 0.0005, 0.0001, 4),
        "surf.perspective" => (1.5, 8.0, 0.5, 0.1, 1),
        "surf.scale" => (0.5, 4.0, 0.2, 0.05, 2),
        "surf.max_steps" => (1000.0, 200000.0, 5000.0, 1000.0, 0),
        // Common
        "max_t" => (10.0, 2000.0, 50.0, 10.0, 0),
        "step" => (0.001, 0.1, 0.005, 0.001, 3),
        // Fallback
        _ => (0.0, 100.0, 1.0, 0.1, 3),
    }
}

/// Pretty label for a param key.
fn param_label(key: &str) -> String {
    match key {
        "line_width" => "Line Width".into(),
        "alpha" => "Alpha".into(),
        "fade" => "Fade".into(),
        "speed" => "Speed".into(),
        "dither" => "Strength".into(),
        "dither_levels" => "Levels".into(),
        "dither_scale" => "Scale".into(),
        "max_t" => "Max Time".into(),
        "step" => "Step Size".into(),
        _ => {
            // Strip prefix, capitalize: "x1.freq" -> "Frequency", "spiro.big_r" -> "Big R"
            let suffix = key.rsplit('.').next().unwrap_or(key);
            let mut label = String::new();
            let mut capitalize = true;
            for ch in suffix.chars() {
                if ch == '_' {
                    label.push(' ');
                    capitalize = true;
                } else if capitalize {
                    label.push(ch.to_ascii_uppercase());
                    capitalize = false;
                } else {
                    label.push(ch);
                }
            }
            // Special cases
            match label.as_str() {
                "Freq" => "Frequency".into(),
                "Amp" => "Amplitude".into(),
                "Freq a" | "Freq A" => "Freq A".into(),
                "Freq b" | "Freq B" => "Freq B".into(),
                "Dt" => "Time Step".into(),
                "Big r" | "Big R" => "Outer Radius".into(),
                "Small r" | "Small R" => "Inner Radius".into(),
                "Offset" => "Pen Offset".into(),
                "Wing freq" => "Wing Freq".into(),
                "Tail freq" => "Tail Freq".into(),
                "Max steps" => "Max Steps".into(),
                "Rot x" | "Rot X" => "Rotation X".into(),
                "Rot y" | "Rot Y" => "Rotation Y".into(),
                "Rot z" | "Rot Z" => "Rotation Z".into(),
                "Steps per edge" => "Steps/Edge".into(),
                "P" => "Winding P".into(),
                "Q" => "Winding Q".into(),
                "A" => "Param A".into(),
                "B" => "Param B".into(),
                "C" => "Param C".into(),
                "D" => "Param D".into(),
                "M" => "Symmetry M".into(),
                "N1" => "Exponent N1".into(),
                "N2" => "Exponent N2".into(),
                "N3" => "Exponent N3".into(),
                "R0" => "Base Radius".into(),
                "Amp1" => "Amplitude 1".into(),
                "Amp2" => "Amplitude 2".into(),
                "Amp3" => "Amplitude 3".into(),
                "Freq1" => "Frequency 1".into(),
                "Freq2" => "Frequency 2".into(),
                "Freq3" => "Frequency 3".into(),
                "Phase1" => "Phase 1".into(),
                "Phase2" => "Phase 2".into(),
                "Phase3" => "Phase 3".into(),
                "L1" => "Arm 1 Length".into(),
                "L2" => "Arm 2 Length".into(),
                "M2" => "Mass 2".into(),
                "G" => "Gravity".into(),
                "Kind" => "Surface Type".into(),
                "Wraps" => "Spiral Wraps".into(),
                "Height" => "Height".into(),
                _ => label,
            }
        }
    }
}

/// Section name from a group of param key prefixes.
fn section_name(prefix: &str) -> String {
    match prefix {
        "drawing" => "Drawing".into(),
        "dithering" => "Dithering".into(),
        "x1" => "Pendulum X1".into(),
        "x2" => "Pendulum X2".into(),
        "y1" => "Pendulum Y1".into(),
        "y2" => "Pendulum Y2".into(),
        "spiro" => "Spirograph".into(),
        "liss" => "Lissajous".into(),
        "rose" => "Rose".into(),
        "bfly" => "Butterfly".into(),
        "lorenz" => "Lorenz".into(),
        "wire" => "Wireframe".into(),
        "knot" => "Torus Knot".into(),
        "cliff" => "Clifford Attractor".into(),
        "dj" => "De Jong Attractor".into(),
        "sf" => "Superformula".into(),
        "guil" => "Guilloché".into(),
        "dpend" => "Double Pendulum".into(),
        "ross" => "Rössler Attractor".into(),
        "surf" => "3D Surface".into(),
        "sim" => "Simulation".into(),
        _ => prefix.to_string(),
    }
}

/// Parse the `get` response into (shape_name, key→value map).
fn parse_get_response(response: &str) -> (String, Vec<(String, f64)>) {
    let mut shape = String::new();
    let mut params = Vec::new();
    for line in response.lines() {
        if let Some((key, val_str)) = line.split_once('=') {
            if key == "shape" {
                shape = val_str.to_string();
            } else if key == "bg" || key == "color" {
                // skip non-numeric compound values
            } else if let Ok(v) = val_str.parse::<f64>() {
                params.push((key.to_string(), v));
            }
        }
    }
    (shape, params)
}

/// Build the param list and sections from the daemon's current state.
fn build_params_from_daemon() -> Option<(String, Vec<Param>, Vec<Section>)> {
    let resp = send_command("get").ok()?;
    let (shape, raw_params) = parse_get_response(&resp);

    // Classify params into groups
    let drawing_keys = ["line_width", "alpha", "fade", "speed"];
    let dither_keys = ["dither", "dither_levels", "dither_scale"];
    let sim_keys = ["max_t", "step"];

    let mut params = Vec::new();
    let mut sections = Vec::new();

    // Drawing section
    let start = params.len();
    let mut count = 0;
    for &dk in &drawing_keys {
        if let Some((_, v)) = raw_params.iter().find(|(k, _)| k == dk) {
            let (min, max, step, fine, decimals) = param_spec(dk);
            params.push(Param {
                key: dk.to_string(),
                label: param_label(dk),
                min, max, step, fine, decimals,
                value: *v,
            });
            count += 1;
        }
    }
    if count > 0 {
        sections.push(Section { name: "Drawing".into(), start, count });
    }

    // Dithering section
    let start = params.len();
    let mut count = 0;
    for &dk in &dither_keys {
        if let Some((_, v)) = raw_params.iter().find(|(k, _)| k == dk) {
            let (min, max, step, fine, decimals) = param_spec(dk);
            params.push(Param {
                key: dk.to_string(),
                label: param_label(dk),
                min, max, step, fine, decimals,
                value: *v,
            });
            count += 1;
        }
    }
    if count > 0 {
        sections.push(Section { name: "Dithering".into(), start, count });
    }

    // Shape-specific params (grouped by prefix before '.')
    let shape_params: Vec<&(String, f64)> = raw_params.iter()
        .filter(|(k, _)| {
            !drawing_keys.contains(&k.as_str())
                && !dither_keys.contains(&k.as_str())
                && !sim_keys.contains(&k.as_str())
        })
        .collect();

    // Collect unique prefixes in order
    let mut seen_prefixes = Vec::new();
    for (k, _) in &shape_params {
        let prefix = k.split('.').next().unwrap_or(k);
        if !seen_prefixes.contains(&prefix.to_string()) {
            seen_prefixes.push(prefix.to_string());
        }
    }

    for prefix in &seen_prefixes {
        let start = params.len();
        let mut count = 0;
        for (k, v) in &shape_params {
            let p = k.split('.').next().unwrap_or(k);
            if p == prefix {
                let (min, max, step, fine, decimals) = param_spec(k);
                params.push(Param {
                    key: k.clone(),
                    label: param_label(k),
                    min, max, step, fine, decimals,
                    value: *v,
                });
                count += 1;
            }
        }
        if count > 0 {
            sections.push(Section { name: section_name(prefix), start, count });
        }
    }

    // Simulation params (max_t, step)
    let start = params.len();
    let mut count = 0;
    for &sk in &sim_keys {
        if let Some((_, v)) = raw_params.iter().find(|(k, _)| k == sk) {
            let (min, max, step, fine, decimals) = param_spec(sk);
            params.push(Param {
                key: sk.to_string(),
                label: param_label(sk),
                min, max, step, fine, decimals,
                value: *v,
            });
            count += 1;
        }
    }
    if count > 0 {
        sections.push(Section { name: "Simulation".into(), start, count });
    }

    Some((shape, params, sections))
}

// ---------------------------------------------------------------------------
// Display line mapping (for scroll tracking)
// ---------------------------------------------------------------------------

enum DisplayLine {
    Header(usize),
    Param(usize),
    Gap,
}

fn build_display_lines(sections: &[Section]) -> Vec<DisplayLine> {
    let mut lines = Vec::new();
    for (si, section) in sections.iter().enumerate() {
        lines.push(DisplayLine::Header(si));
        for i in section.start..(section.start + section.count) {
            lines.push(DisplayLine::Param(i));
        }
        if si + 1 < sections.len() {
            lines.push(DisplayLine::Gap);
        }
    }
    lines
}

fn param_display_row(lines: &[DisplayLine], param_idx: usize) -> usize {
    for (i, line) in lines.iter().enumerate() {
        if let DisplayLine::Param(idx) = line {
            if *idx == param_idx {
                return i;
            }
        }
    }
    0
}

// ---------------------------------------------------------------------------
// TUI rendering
// ---------------------------------------------------------------------------

fn draw(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    params: &[Param],
    sections: &[Section],
    display_lines: &[DisplayLine],
    selected: usize,
    scroll: usize,
    shape_name: &str,
) {
    terminal
        .draw(|frame| {
            let area = frame.area();

            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

            // --- Header ---
            let header = Line::from(vec![
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(
                    "wl-walls",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  shape: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    shape_name,
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            frame.render_widget(Paragraph::new(header), chunks[0]);

            // --- Separator ---
            frame.render_widget(
                Paragraph::new(""),
                chunks[1],
            );

            // --- Body: scrollable param list ---
            let body = chunks[2];
            let visible_rows = body.height as usize;

            for (vi, line) in display_lines
                .iter()
                .enumerate()
                .skip(scroll)
                .take(visible_rows)
            {
                let row_y = body.y + (vi - scroll) as u16;
                if row_y >= body.y + body.height {
                    break;
                }
                let row_area = Rect::new(body.x, row_y, body.width, 1);

                match line {
                    DisplayLine::Header(si) => {
                        let name = &sections[*si].name;
                        let hdr = Paragraph::new(Line::from(Span::styled(
                            format!(" {}", name),
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )));
                        frame.render_widget(hdr, row_area);
                    }
                    DisplayLine::Param(idx) => {
                        let p = &params[*idx];
                        let is_sel = *idx == selected;

                        let label_w = 18u16;
                        let value_w = 10u16;
                        let bar_w = row_area.width.saturating_sub(label_w + value_w + 1);

                        let parts = Layout::horizontal([
                            Constraint::Length(label_w),
                            Constraint::Length(bar_w),
                            Constraint::Length(value_w),
                        ])
                        .split(row_area);

                        let marker = if is_sel { " ▸ " } else { "   " };
                        let label_style = if is_sel {
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        frame.render_widget(
                            Paragraph::new(Span::styled(
                                format!("{}{}", marker, p.label),
                                label_style,
                            )),
                            parts[0],
                        );

                        let frac = if p.max > p.min {
                            (p.value - p.min) / (p.max - p.min)
                        } else {
                            0.0
                        };
                        let (filled, empty, thumb) = if is_sel {
                            (
                                Style::default().fg(Color::Cyan),
                                Style::default().fg(Color::Rgb(40, 40, 40)),
                                Style::default().fg(Color::White),
                            )
                        } else {
                            (
                                Style::default().fg(Color::DarkGray),
                                Style::default().fg(Color::Rgb(40, 40, 40)),
                                Style::default().fg(Color::DarkGray),
                            )
                        };
                        let slider = Slider::new(frac)
                            .with_filled_style(filled)
                            .with_empty_style(empty)
                            .with_thumb_style(thumb);
                        slider.render(frame, parts[1]);

                        let val_style = if is_sel {
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        frame.render_widget(
                            Paragraph::new(Span::styled(
                                format!(" {:>8.*}", p.decimals, p.value),
                                val_style,
                            )),
                            parts[2],
                        );
                    }
                    DisplayLine::Gap => {}
                }
            }

            // --- Footer ---
            let footer = Line::from(vec![
                Span::styled(" ↑↓", Style::default().fg(Color::DarkGray)),
                Span::styled(" select  ", Style::default().fg(Color::Gray)),
                Span::styled("←→", Style::default().fg(Color::DarkGray)),
                Span::styled(" adjust  ", Style::default().fg(Color::Gray)),
                Span::styled("shift+←→", Style::default().fg(Color::DarkGray)),
                Span::styled(" fine  ", Style::default().fg(Color::Gray)),
                Span::styled("r", Style::default().fg(Color::Yellow)),
                Span::styled(" random  ", Style::default().fg(Color::Gray)),
                Span::styled("s", Style::default().fg(Color::Yellow)),
                Span::styled(" shape  ", Style::default().fg(Color::Gray)),
                Span::styled("c", Style::default().fg(Color::Yellow)),
                Span::styled(" color  ", Style::default().fg(Color::Gray)),
                Span::styled("space", Style::default().fg(Color::Yellow)),
                Span::styled(" restart  ", Style::default().fg(Color::Gray)),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::styled(" quit", Style::default().fg(Color::Gray)),
            ]);
            frame.render_widget(Paragraph::new(footer), chunks[3]);
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// TUI event loop
// ---------------------------------------------------------------------------

fn run_tui() -> Result<(), String> {
    let (mut shape_name, mut params, mut sections) =
        build_params_from_daemon().ok_or("cannot connect — is wl-walls running?")?;
    let mut display_lines = build_display_lines(&sections);

    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).map_err(|e| e.to_string())?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let mut selected: usize = 0;
    let mut scroll: usize = 0;

    loop {
        let total = params.len();

        // Keep selection visible
        let visible = terminal.size().map(|s| s.height as usize).unwrap_or(24).saturating_sub(4);
        let sel_row = param_display_row(&display_lines, selected);
        if sel_row < scroll + 1 {
            scroll = sel_row.saturating_sub(1);
        } else if sel_row >= scroll + visible.saturating_sub(1) {
            scroll = sel_row + 2 - visible;
        }

        draw(&mut terminal, &params, &sections, &display_lines, selected, scroll, &shape_name);

        if !event::poll(Duration::from_millis(100)).unwrap_or(false) {
            continue;
        }

        match event::read() {
            Ok(Event::Key(KeyEvent {
                code, modifiers, ..
            })) => match code {
                KeyCode::Char('q') | KeyCode::Esc => break,

                // Navigation
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected + 1 < total {
                        selected += 1;
                    }
                }
                KeyCode::Tab => {
                    for section in &sections {
                        if section.start > selected {
                            selected = section.start;
                            break;
                        }
                    }
                }
                KeyCode::BackTab => {
                    for section in sections.iter().rev() {
                        if section.start < selected {
                            selected = section.start;
                            break;
                        }
                    }
                }

                // Adjust value
                KeyCode::Right | KeyCode::Char('l')
                    if !modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value + p.step).min(p.max);
                        send_set(&p.key, p.value);
                    }
                }
                KeyCode::Left | KeyCode::Char('h')
                    if !modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value - p.step).max(p.min);
                        send_set(&p.key, p.value);
                    }
                }

                // Fine adjust
                KeyCode::Right if modifiers.contains(KeyModifiers::SHIFT) => {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value + p.fine).min(p.max);
                        send_set(&p.key, p.value);
                    }
                }
                KeyCode::Left if modifiers.contains(KeyModifiers::SHIFT) => {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value - p.fine).max(p.min);
                        send_set(&p.key, p.value);
                    }
                }
                KeyCode::Char('H') => {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value - p.fine).max(p.min);
                        send_set(&p.key, p.value);
                    }
                }
                KeyCode::Char('L') => {
                    if selected < total {
                        let p = &mut params[selected];
                        p.value = (p.value + p.fine).min(p.max);
                        send_set(&p.key, p.value);
                    }
                }

                // Actions
                KeyCode::Char('r') => {
                    send_action("randomize");
                    // Shape may have changed — rebuild param list
                    if let Some((sn, p, s)) = build_params_from_daemon() {
                        shape_name = sn;
                        params = p;
                        sections = s;
                        display_lines = build_display_lines(&sections);
                        selected = selected.min(params.len().saturating_sub(1));
                    }
                }
                KeyCode::Char('s') => {
                    send_action("next-shape");
                    // Rebuild for the new shape's params
                    if let Some((sn, p, s)) = build_params_from_daemon() {
                        shape_name = sn;
                        params = p;
                        sections = s;
                        display_lines = build_display_lines(&sections);
                        selected = selected.min(params.len().saturating_sub(1));
                    }
                }
                KeyCode::Char('c') => {
                    send_action("next-color");
                }
                KeyCode::Char(' ') => {
                    send_action("restart");
                }

                _ => {}
            },
            Ok(Event::Resize(_, _)) => {}
            _ => {}
        }
    }

    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        cursor::Show
    )
    .ok();
    terminal::disable_raw_mode().ok();

    Ok(())
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let cmd = args[1..].join(" ");
        match send_command(&cmd) {
            Ok(resp) => print!("{}", resp),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        if let Err(e) = run_tui() {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
