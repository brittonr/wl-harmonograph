pub mod shapes;

use std::env;

pub type Color = (f64, f64, f64);

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
