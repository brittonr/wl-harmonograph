//! Live control TUI for wl-harmonograph.
//!
//! Interactive mode (no args): full-screen parameter editor with sliders.
//! CLI mode (with args): send a single command and print the response.
//!
//!   wl-harmonograph-ctl                     # interactive TUI
//!   wl-harmonograph-ctl get                 # dump all params
//!   wl-harmonograph-ctl set alpha 0.5       # set a param
//!   wl-harmonograph-ctl restart             # clear + redraw
//!   wl-harmonograph-ctl randomize           # new random pattern
//!   wl-harmonograph-ctl next-color          # cycle color

use std::collections::HashMap;
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
    PathBuf::from(dir).join("wl-harmonograph.sock")
}

fn send_command(cmd: &str) -> Result<String, String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "cannot connect to {} — is wl-harmonograph running?\n({})",
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
    key: &'static str,
    label: &'static str,
    min: f64,
    max: f64,
    step: f64,
    fine: f64,
    decimals: usize,
    value: f64,
}

struct Section {
    name: &'static str,
    start: usize,
    count: usize,
}

fn build_params() -> (Vec<Param>, Vec<Section>) {
    let p = |key, label, min, max, step, fine, decimals| Param {
        key,
        label,
        min,
        max,
        step,
        fine,
        decimals,
        value: 0.0,
    };

    let params = vec![
        // Drawing
        p("line_width", "Line Width", 0.5, 20.0, 0.5, 0.1, 1),
        p("alpha", "Alpha", 0.01, 1.0, 0.05, 0.01, 2),
        p("fade", "Fade", 0.0, 0.1, 0.005, 0.001, 4),
        p("speed", "Speed", 1.0, 50.0, 1.0, 1.0, 0),
        // Dithering
        p("dither", "Strength", 0.0, 1.0, 0.05, 0.01, 2),
        p("dither_levels", "Levels", 2.0, 256.0, 4.0, 1.0, 0),
        p("dither_scale", "Scale", 1.0, 8.0, 0.5, 0.1, 1),
        // Pendulum X1
        p("x1.freq", "Frequency", 0.1, 16.0, 0.1, 0.01, 3),
        p("x1.amp", "Amplitude", 0.0, 2.0, 0.05, 0.01, 3),
        p("x1.phase", "Phase", 0.0, 6.283, 0.1, 0.01, 3),
        p("x1.damping", "Damping", 0.0, 0.05, 0.001, 0.0001, 4),
        // Pendulum X2
        p("x2.freq", "Frequency", 0.1, 16.0, 0.1, 0.01, 3),
        p("x2.amp", "Amplitude", 0.0, 2.0, 0.05, 0.01, 3),
        p("x2.phase", "Phase", 0.0, 6.283, 0.1, 0.01, 3),
        p("x2.damping", "Damping", 0.0, 0.05, 0.001, 0.0001, 4),
        // Pendulum Y1
        p("y1.freq", "Frequency", 0.1, 16.0, 0.1, 0.01, 3),
        p("y1.amp", "Amplitude", 0.0, 2.0, 0.05, 0.01, 3),
        p("y1.phase", "Phase", 0.0, 6.283, 0.1, 0.01, 3),
        p("y1.damping", "Damping", 0.0, 0.05, 0.001, 0.0001, 4),
        // Pendulum Y2
        p("y2.freq", "Frequency", 0.1, 16.0, 0.1, 0.01, 3),
        p("y2.amp", "Amplitude", 0.0, 2.0, 0.05, 0.01, 3),
        p("y2.phase", "Phase", 0.0, 6.283, 0.1, 0.01, 3),
        p("y2.damping", "Damping", 0.0, 0.05, 0.001, 0.0001, 4),
    ];

    let sections = vec![
        Section { name: "Drawing", start: 0, count: 4 },
        Section { name: "Dithering", start: 4, count: 3 },
        Section { name: "Pendulum X1", start: 7, count: 4 },
        Section { name: "Pendulum X2", start: 11, count: 4 },
        Section { name: "Pendulum Y1", start: 15, count: 4 },
        Section { name: "Pendulum Y2", start: 19, count: 4 },
    ];

    (params, sections)
}

/// Parse the `get` response into a key→value map.
fn parse_get_response(response: &str) -> HashMap<String, f64> {
    let mut map = HashMap::new();
    for line in response.lines() {
        if let Some((key, val_str)) = line.split_once('=') {
            if let Ok(v) = val_str.parse::<f64>() {
                map.insert(key.to_string(), v);
            }
        }
    }
    map
}

/// Fetch current values from the daemon and populate params.
fn fetch_values(params: &mut [Param]) -> bool {
    match send_command("get") {
        Ok(resp) => {
            let map = parse_get_response(&resp);
            for p in params.iter_mut() {
                if let Some(&v) = map.get(p.key) {
                    p.value = v;
                }
            }
            true
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Display line mapping (for scroll tracking)
// ---------------------------------------------------------------------------

/// Each visible row is either a section header, a param slider, or a gap.
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

/// Find the display-line index for a given param index.
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
) {
    terminal
        .draw(|frame| {
            let area = frame.area();

            // Layout: header (1) | body (flex) | footer (1)
            let chunks = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

            // --- Header ---
            let header = Line::from(vec![
                Span::styled("● ", Style::default().fg(Color::Green)),
                Span::styled(
                    "wl-harmonograph",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            frame.render_widget(Paragraph::new(header), chunks[0]);

            // --- Body: scrollable param list ---
            let body = chunks[1];
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
                        let name = sections[*si].name;
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

                        // Split row: marker+label | bar | value
                        let label_w = 18u16;
                        let value_w = 10u16;
                        let bar_w = row_area.width.saturating_sub(label_w + value_w + 1);

                        let parts = Layout::horizontal([
                            Constraint::Length(label_w),
                            Constraint::Length(bar_w),
                            Constraint::Length(value_w),
                        ])
                        .split(row_area);

                        // Label
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

                        // Slider (rat-widgets)
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

                        // Value
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
                Span::styled("c", Style::default().fg(Color::Yellow)),
                Span::styled(" color  ", Style::default().fg(Color::Gray)),
                Span::styled("space", Style::default().fg(Color::Yellow)),
                Span::styled(" restart  ", Style::default().fg(Color::Gray)),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::styled(" quit", Style::default().fg(Color::Gray)),
            ]);
            frame.render_widget(Paragraph::new(footer), chunks[2]);
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// TUI event loop
// ---------------------------------------------------------------------------

fn run_tui() -> Result<(), String> {
    let (mut params, sections) = build_params();
    let total = params.len();
    let display_lines = build_display_lines(&sections);

    let connected = fetch_values(&mut params);
    if !connected {
        return Err("cannot connect — is wl-harmonograph running?".into());
    }

    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).map_err(|e| e.to_string())?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    let mut selected: usize = 0;
    let mut scroll: usize = 0;

    loop {
        // Keep selection visible
        let visible = terminal.size().map(|s| s.height as usize).unwrap_or(24).saturating_sub(3);
        let sel_row = param_display_row(&display_lines, selected);
        if sel_row < scroll + 1 {
            scroll = sel_row.saturating_sub(1);
        } else if sel_row >= scroll + visible.saturating_sub(1) {
            scroll = sel_row + 2 - visible;
        }

        draw(&mut terminal, &params, &sections, &display_lines, selected, scroll);

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
                    let p = &mut params[selected];
                    p.value = (p.value + p.step).min(p.max);
                    send_set(p.key, p.value);
                }
                KeyCode::Left | KeyCode::Char('h')
                    if !modifiers.contains(KeyModifiers::SHIFT) =>
                {
                    let p = &mut params[selected];
                    p.value = (p.value - p.step).max(p.min);
                    send_set(p.key, p.value);
                }

                // Fine adjust (shift+arrow or H/L)
                KeyCode::Right if modifiers.contains(KeyModifiers::SHIFT) => {
                    let p = &mut params[selected];
                    p.value = (p.value + p.fine).min(p.max);
                    send_set(p.key, p.value);
                }
                KeyCode::Left if modifiers.contains(KeyModifiers::SHIFT) => {
                    let p = &mut params[selected];
                    p.value = (p.value - p.fine).max(p.min);
                    send_set(p.key, p.value);
                }
                KeyCode::Char('H') => {
                    let p = &mut params[selected];
                    p.value = (p.value - p.fine).max(p.min);
                    send_set(p.key, p.value);
                }
                KeyCode::Char('L') => {
                    let p = &mut params[selected];
                    p.value = (p.value + p.fine).min(p.max);
                    send_set(p.key, p.value);
                }

                // Actions
                KeyCode::Char('r') => {
                    send_action("randomize");
                    fetch_values(&mut params);
                }
                KeyCode::Char('c') => {
                    send_action("next-color");
                }
                KeyCode::Char(' ') => {
                    send_action("restart");
                }

                _ => {}
            },
            Ok(Event::Resize(_, _)) => {} // redraw next loop
            _ => {}
        }
    }

    // Cleanup
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
        // CLI mode: join remaining args as the command
        let cmd = args[1..].join(" ");
        match send_command(&cmd) {
            Ok(resp) => print!("{}", resp),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Interactive TUI
        if let Err(e) = run_tui() {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
