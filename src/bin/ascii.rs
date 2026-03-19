//! Terminal ASCII renderer for harmonograph patterns.
//!
//! Draws the same mathematical curves as the Wayland wallpaper, rendered as
//! density-mapped ASCII art. Each terminal cell accumulates intensity as
//! curves pass through it, mapped to a character from a density ramp
//! (space through @). Older trails fade each frame.
//!
//! Reads the same HARMONOGRAPH_* env vars as the wallpaper (shape, colors,
//! fps, fade). Speed defaults higher (50) since each step is a single
//! point rather than a GPU-interpolated segment.
//!
//! Keys:
//!   q/Esc   quit
//!   r       randomize (new shape + params)
//!   s       next shape type
//!   c       next color
//!   space   restart (clear + new params, same shape type)

use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use rand::Rng;
use ratatui::backend::CrosstermBackend;
use ratatui::style::{Color as TermColor, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Terminal;

use wl_harmonograph::shapes::{Shape, SHAPE_NAMES};
use wl_harmonograph::{colors_from_env, parse_env_f32, parse_env_u32, Color};

/// ASCII density ramp from empty to full.
const RAMP: &[char] = &[' ', '.', ',', ':', ';', '=', '!', '*', '#', '$', '@'];

/// Terminal cells are roughly 2× taller than wide.
const CELL_RATIO: f64 = 2.1;

fn main() {
    let (fg_colors, bg_color) = colors_from_env();
    let mut rng = rand::thread_rng();
    let current_color = fg_colors[rng.gen_range(0..fg_colors.len())];

    let fps = parse_env_u32("HARMONOGRAPH_FPS", 30).clamp(1, 144);
    let fade = parse_env_f32("HARMONOGRAPH_FADE", 0.03);
    let steps = parse_env_u32("HARMONOGRAPH_SPEED", 50).max(1);

    let shape_env = std::env::var("HARMONOGRAPH_SHAPE").unwrap_or_default();
    let (shape_lock, initial_shape) = match shape_env.to_lowercase().as_str() {
        "" | "random" => (None, Shape::random()),
        name => match Shape::from_name(name) {
            Some(s) => (Some(name.to_string()), s),
            None => {
                eprintln!(
                    "unknown shape '{}', using random (available: {})",
                    name,
                    SHAPE_NAMES.join(", ")
                );
                (None, Shape::random())
            }
        },
    };

    if let Err(e) = terminal::enable_raw_mode() {
        eprintln!("error: requires a terminal ({})", e);
        std::process::exit(1);
    }
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).unwrap();

    let size = terminal.size().unwrap();
    let mut app = AsciiApp::new(
        size.width as usize,
        size.height.saturating_sub(1) as usize,
        initial_shape,
        shape_lock,
        fg_colors,
        bg_color,
        current_color,
        fade as f64,
        steps,
    );

    let frame_interval = Duration::from_millis((1000 / fps as u64).max(1));

    loop {
        let frame_start = Instant::now();

        while event::poll(Duration::ZERO).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        execute!(
                            terminal.backend_mut(),
                            LeaveAlternateScreen,
                            cursor::Show
                        )
                        .ok();
                        terminal::disable_raw_mode().ok();
                        return;
                    }
                    KeyCode::Char('r') => app.randomize(),
                    KeyCode::Char('s') => app.next_shape(),
                    KeyCode::Char('c') => app.next_color(),
                    KeyCode::Char(' ') => app.restart(),
                    _ => {}
                }
            }
        }

        let size = terminal.size().unwrap();
        let w = size.width as usize;
        let h = size.height.saturating_sub(1) as usize;
        if w != app.width || h != app.height {
            app.resize(w, h);
        }

        app.tick();
        app.render(&mut terminal);

        let elapsed = frame_start.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct AsciiApp {
    shape: Shape,
    shape_lock: Option<String>,

    grid: Vec<f64>,
    width: usize,
    height: usize,

    /// Previous grid-space point for line interpolation.
    prev: Option<(f64, f64)>,

    fg_colors: Vec<Color>,
    bg_color: Color,
    current_color: Color,

    fade: f64,
    steps_per_frame: u32,
    plot_intensity: f64,
}

impl AsciiApp {
    fn new(
        width: usize,
        height: usize,
        shape: Shape,
        shape_lock: Option<String>,
        fg_colors: Vec<Color>,
        bg_color: Color,
        current_color: Color,
        fade: f64,
        steps_per_frame: u32,
    ) -> Self {
        Self {
            shape,
            shape_lock,
            grid: vec![0.0; width * height],
            width,
            height,
            prev: None,
            fg_colors,
            bg_color,
            current_color,
            fade,
            steps_per_frame,
            plot_intensity: 0.08,
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.grid = vec![0.0; width * height];
        self.prev = None;
    }

    fn pick_color(&mut self) {
        let mut rng = rand::thread_rng();
        if self.fg_colors.len() <= 1 {
            self.current_color = self.fg_colors[0];
            return;
        }
        loop {
            let c = self.fg_colors[rng.gen_range(0..self.fg_colors.len())];
            if c != self.current_color {
                self.current_color = c;
                return;
            }
        }
    }

    fn restart(&mut self) {
        self.grid.fill(0.0);
        self.prev = None;
        if let Some(ref lock) = self.shape_lock {
            let name = lock.clone();
            self.shape = Shape::from_name(&name).unwrap_or_else(Shape::random);
        } else {
            self.shape = Shape::random();
        }
        self.pick_color();
    }

    fn randomize(&mut self) {
        self.grid.fill(0.0);
        self.prev = None;
        self.shape = Shape::random();
        self.pick_color();
    }

    fn next_shape(&mut self) {
        let next = self.shape.next_name().to_string();
        self.grid.fill(0.0);
        self.prev = None;
        self.shape = Shape::from_name(&next).unwrap_or_else(Shape::random);
        self.pick_color();
    }

    fn next_color(&mut self) {
        self.pick_color();
    }

    // -----------------------------------------------------------------------
    // Coordinate mapping
    // -----------------------------------------------------------------------

    /// Map shape coordinates (roughly [-1.5, 1.5]) to grid cell coordinates,
    /// preserving aspect ratio against the terminal's non-square cells.
    fn shape_to_grid(&self, sx: f64, sy: f64) -> (f64, f64) {
        let w = self.width as f64;
        let h = self.height as f64;
        let eff_h = h * CELL_RATIO;
        let min_dim = w.min(eff_h);
        let scale = min_dim * 0.4;

        let gx = sx * scale + w / 2.0;
        let gy = -sy * scale / CELL_RATIO + h / 2.0;
        (gx, gy)
    }

    // -----------------------------------------------------------------------
    // Plotting
    // -----------------------------------------------------------------------

    /// Add intensity to a grid cell (bounds-checked).
    fn add_cell(&mut self, x: i32, y: i32, amount: f64) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.grid[y as usize * self.width + x as usize] += amount;
        }
    }

    /// Plot a single point with bilinear sub-cell distribution.
    fn plot_point(&mut self, gx: f64, gy: f64) {
        let ix = gx.floor() as i32;
        let iy = gy.floor() as i32;
        let fx = gx - ix as f64;
        let fy = gy - iy as f64;
        let i = self.plot_intensity;

        self.add_cell(ix, iy, i * (1.0 - fx) * (1.0 - fy));
        self.add_cell(ix + 1, iy, i * fx * (1.0 - fy));
        self.add_cell(ix, iy + 1, i * (1.0 - fx) * fy);
        self.add_cell(ix + 1, iy + 1, i * fx * fy);
    }

    /// Plot a line between two grid-space points, interpolating at ~2×
    /// the grid resolution so curves don't have gaps.
    fn plot_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = (dx * dx + dy * dy).sqrt();
        let steps = (dist * 2.0).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.plot_point(x0 + dx * t, y0 + dy * t);
        }
    }

    // -----------------------------------------------------------------------
    // Tick + render
    // -----------------------------------------------------------------------

    fn tick(&mut self) {
        // Fade existing intensity
        let retain = 1.0 - self.fade;
        for v in &mut self.grid {
            *v *= retain;
        }

        // Advance shape and plot
        for _ in 0..self.steps_per_frame {
            match self.shape.step() {
                Some((sx, sy)) => {
                    let (gx, gy) = self.shape_to_grid(sx, sy);
                    if let Some((px, py)) = self.prev {
                        self.plot_line(px, py, gx, gy);
                    } else {
                        self.plot_point(gx, gy);
                    }
                    self.prev = Some((gx, gy));
                }
                None => {
                    self.restart();
                    return;
                }
            }
        }
    }

    fn render(&self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
        let max_val = self.grid.iter().cloned().fold(0.0f64, f64::max).max(0.001);
        let (cr, cg, cb) = self.current_color;
        let (br, bg, bb) = self.bg_color;
        let bg_term = TermColor::Rgb(
            (br * 255.0) as u8,
            (bg * 255.0) as u8,
            (bb * 255.0) as u8,
        );

        terminal
            .draw(|frame| {
                let area = frame.area();
                let rows = self.height.min(area.height.saturating_sub(1) as usize);
                let cols = self.width.min(area.width as usize);

                let mut lines: Vec<Line> = Vec::with_capacity(rows + 1);

                for y in 0..rows {
                    let mut spans: Vec<Span> = Vec::with_capacity(cols);
                    for x in 0..cols {
                        let val = self.grid[y * self.width + x];
                        let normalized = (val / max_val).sqrt();
                        let idx = (normalized * (RAMP.len() - 1) as f64).round() as usize;
                        let idx = idx.min(RAMP.len() - 1);
                        let ch = RAMP[idx];

                        if idx == 0 {
                            spans.push(Span::styled(" ", Style::default().bg(bg_term)));
                        } else {
                            let brightness = normalized.clamp(0.3, 1.0);
                            let fg = TermColor::Rgb(
                                (cr * 255.0 * brightness) as u8,
                                (cg * 255.0 * brightness) as u8,
                                (cb * 255.0 * brightness) as u8,
                            );
                            spans.push(Span::styled(
                                String::from(ch),
                                Style::default().fg(fg).bg(bg_term),
                            ));
                        }
                    }
                    lines.push(Line::from(spans));
                }

                // Status bar
                let status_fg = TermColor::Rgb(
                    (cr * 180.0) as u8,
                    (cg * 180.0) as u8,
                    (cb * 180.0) as u8,
                );
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(" {} ", self.shape.name()),
                        Style::default().fg(status_fg).bg(bg_term),
                    ),
                    Span::styled(
                        " r:random s:shape c:color ␣:restart q:quit",
                        Style::default()
                            .fg(TermColor::Rgb(80, 80, 80))
                            .bg(bg_term),
                    ),
                ]));

                frame.render_widget(
                    Paragraph::new(lines).style(Style::default().bg(bg_term)),
                    area,
                );
            })
            .unwrap();
    }
}
