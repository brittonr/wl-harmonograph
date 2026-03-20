use rand::Rng;
use super::Shape;

/// Hypotrochoid / epitrochoid curves — the math behind a Spirograph toy.
///
/// Hypotrochoid (inner = true):
///   x(t) = (R - r) * cos(t) + d * cos((R - r) / r * t)
///   y(t) = (R - r) * sin(t) - d * sin((R - r) / r * t)
///
/// Epitrochoid (inner = false):
///   x(t) = (R + r) * cos(t) - d * cos((R + r) / r * t)
///   y(t) = (R + r) * sin(t) - d * sin((R + r) / r * t)
pub struct Spirograph {
    big_r: f64,
    small_r: f64,
    offset: f64,
    damping: f64,
    inner: bool,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Spirograph {
    pub fn new() -> Self {
        let mut s = Self {
            big_r: 1.0,
            small_r: 0.4,
            offset: 0.3,
            damping: 0.003,
            inner: true,
            t: 0.0,
            max_t: 300.0,
            step: 0.01,
        };
        s.randomize();
        s
    }
}

impl Shape for Spirograph {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        self.big_r = 1.0;
        // Pick nice ratios for interesting patterns
        let ratios = [
            (1.0, 3.0),
            (1.0, 4.0),
            (1.0, 5.0),
            (2.0, 5.0),
            (2.0, 7.0),
            (3.0, 7.0),
            (3.0, 8.0),
            (4.0, 9.0),
            (5.0, 12.0),
            (3.0, 5.0),
            (4.0, 7.0),
            (5.0, 9.0),
        ];
        let (n, d) = ratios[rng.gen_range(0..ratios.len())];
        self.small_r = n / d + rng.gen_range(-0.005..0.005);
        self.offset = rng.gen_range(0.2..0.9) * self.small_r;
        self.damping = rng.gen_range(0.001..0.005);
        self.inner = rng.gen_bool(0.7); // hypotrochoid more often
        self.t = 0.0;

        // Enough time to trace the full pattern several times with decay
        self.max_t = d * std::f64::consts::TAU * 3.0 + 100.0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn name(&self) -> &'static str {
        "spirograph"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }

        let decay = (-self.damping * self.t).exp();
        let (x, y) = if self.inner {
            let diff = self.big_r - self.small_r;
            let ratio = diff / self.small_r;
            (
                diff * self.t.cos() + self.offset * (ratio * self.t).cos(),
                diff * self.t.sin() - self.offset * (ratio * self.t).sin(),
            )
        } else {
            let sum = self.big_r + self.small_r;
            let ratio = sum / self.small_r;
            (
                sum * self.t.cos() - self.offset * (ratio * self.t).cos(),
                sum * self.t.sin() - self.offset * (ratio * self.t).sin(),
            )
        };

        // Normalize so max radius maps to ~1.0
        let max_r = if self.inner {
            (self.big_r - self.small_r).abs() + self.offset
        } else {
            self.big_r + self.small_r + self.offset
        };
        let scale = 1.0 / max_r.max(0.01);

        self.t += self.step;
        Some((x * scale * decay, y * scale * decay))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "spiro.big_r" => Some(self.big_r),
            "spiro.small_r" => Some(self.small_r),
            "spiro.offset" => Some(self.offset),
            "spiro.damping" => Some(self.damping),
            "spiro.inner" => Some(if self.inner { 1.0 } else { 0.0 }),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "spiro.big_r" => self.big_r = value,
            "spiro.small_r" => self.small_r = value,
            "spiro.offset" => self.offset = value,
            "spiro.damping" => self.damping = value,
            "spiro.inner" => self.inner = value > 0.5,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("spiro.big_r", self.big_r),
            ("spiro.small_r", self.small_r),
            ("spiro.offset", self.offset),
            ("spiro.damping", self.damping),
            ("spiro.inner", if self.inner { 1.0 } else { 0.0 }),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
