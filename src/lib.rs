pub mod ascii;
pub mod shapes;

use std::env;

use rand::Rng;

pub type Color = (f64, f64, f64);

/// Pick a random color from the palette, different from `current` if possible.
///
/// Bounded to avoid spinning forever on palettes with all-identical entries.
pub fn pick_random_color(palette: &[Color], current: Color) -> Color {
    if palette.len() <= 1 {
        return palette[0];
    }
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let c = palette[rng.gen_range(0..palette.len())];
        if c != current {
            return c;
        }
    }
    palette[rng.gen_range(0..palette.len())]
}

/// Resolve the `HARMONOGRAPH_SHAPE` env var into an initial shape and
/// optional lock name (Some = locked to that shape type on restart).
pub fn resolve_shape_env() -> (Option<String>, Box<dyn shapes::Shape>) {
    let shape_env = env::var("HARMONOGRAPH_SHAPE").unwrap_or_default();
    match shape_env.to_lowercase().as_str() {
        "" | "random" => (None, shapes::random_shape()),
        name => match shapes::shape_from_name(name) {
            Some(s) => (Some(name.to_string()), s),
            None => {
                eprintln!(
                    "Unknown shape '{}', using random. Available: {}",
                    name,
                    shapes::SHAPE_NAMES.join(", ")
                );
                (None, shapes::random_shape())
            }
        },
    }
}

pub fn parse_hex_color(hex: &str) -> Option<Color> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f64 / 255.0;
    Some((r, g, b))
}

pub fn colors_from_env() -> (Vec<Color>, Color) {
    let default_fg: Vec<Color> = vec![
        (0.984, 0.286, 0.204),
        (0.596, 0.592, 0.102),
        (0.988, 0.694, 0.349),
        (0.514, 0.647, 0.596),
        (0.827, 0.525, 0.608),
        (0.557, 0.753, 0.486),
        (0.894, 0.827, 0.529),
    ];
    let default_bg: Color = (0.114, 0.122, 0.137);

    let fg = env::var("HARMONOGRAPH_FG")
        .ok()
        .and_then(|s| {
            let c: Vec<Color> = s.split(',').filter_map(parse_hex_color).collect();
            if c.is_empty() {
                None
            } else {
                Some(c)
            }
        })
        .unwrap_or(default_fg);

    let bg = env::var("HARMONOGRAPH_BG")
        .ok()
        .and_then(|s| parse_hex_color(&s))
        .unwrap_or(default_bg);

    (fg, bg)
}

pub fn parse_env_f32(name: &str, default: f32) -> f32 {
    env::var(name)
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .unwrap_or(default)
}

pub fn parse_env_f64(name: &str, default: f64) -> f64 {
    env::var(name)
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

pub fn parse_env_u32(name: &str, default: u32) -> u32 {
    env::var(name)
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(default)
}
