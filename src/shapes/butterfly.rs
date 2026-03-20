use std::f64::consts::PI;

use rand::Rng;
use super::Shape;

/// Maximum radius of the raw butterfly curve (~e^1 + 2 + 1 ≈ 4.3, padded).
const MAX_RADIUS: f64 = 4.5;

/// Temple H. Fay's butterfly curve (1989).
///
///   r(θ) = e^sin(θ) − 2·cos(4θ) + sin⁵((2θ − π) / 24)
///
/// Produces a striking butterfly-shaped figure. The raw curve has a
/// max radius of about 4.3, so we normalize to fit [-1, 1].
///
/// Randomization varies the internal frequency multipliers and adds
/// damping for the fade-out effect.
pub struct Butterfly {
    /// Multiplier on the cos term (default 4.0 → 4 wing lobes)
    wing_freq: f64,
    /// Multiplier on the sin^5 tail term (default 1/24 period)
    tail_freq: f64,
    /// Overall scale
    amplitude: f64,
    damping: f64,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Butterfly {
    pub fn new() -> Self {
        let mut b = Self {
            wing_freq: 4.0,
            tail_freq: 24.0,
            amplitude: 1.0,
            damping: 0.003,
            t: 0.0,
            max_t: 300.0,
            step: 0.005,
        };
        b.randomize();
        b
    }
}

impl Shape for Butterfly {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // The classic curve uses wing_freq=4, tail_freq=24.
        // Vary these slightly for different wing shapes.
        let wing_choices = [3.0, 4.0, 5.0, 6.0];
        self.wing_freq = wing_choices[rng.gen_range(0..wing_choices.len())]
            + rng.gen_range(-0.1..0.1);
        self.tail_freq = rng.gen_range(16.0..32.0);
        self.amplitude = rng.gen_range(0.8..1.0);
        self.damping = rng.gen_range(0.002..0.005);
        self.t = 0.0;
        // Several full rotations
        self.max_t = 24.0 * PI + 100.0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn name(&self) -> &'static str {
        "butterfly"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }
        let decay = (-self.damping * self.t).exp();

        let r = self.t.sin().exp()
            - 2.0 * (self.wing_freq * self.t).cos()
            + ((2.0 * self.t - PI) / self.tail_freq).sin().powi(5);

        // Raw max r ≈ 4.3, normalize to ~1.0
        let scale = self.amplitude / MAX_RADIUS;
        let r = r * scale * decay;
        let x = r * self.t.cos();
        let y = r * self.t.sin();
        self.t += self.step;
        Some((x, y))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "bfly.wing_freq" => Some(self.wing_freq),
            "bfly.tail_freq" => Some(self.tail_freq),
            "bfly.amplitude" => Some(self.amplitude),
            "bfly.damping" => Some(self.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "bfly.wing_freq" => self.wing_freq = value,
            "bfly.tail_freq" => self.tail_freq = value,
            "bfly.amplitude" => self.amplitude = value,
            "bfly.damping" => self.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("bfly.wing_freq", self.wing_freq),
            ("bfly.tail_freq", self.tail_freq),
            ("bfly.amplitude", self.amplitude),
            ("bfly.damping", self.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
