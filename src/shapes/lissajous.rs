use std::f64::consts::{PI, TAU};

use rand::Rng;
use super::Shape;

/// Lissajous curves — two sinusoids with different frequencies.
///
///   x(t) = sin(a * t + delta)
///   y(t) = sin(b * t)
///
/// Integer frequency ratios produce clean closed curves:
///   a=1, b=2  → figure-8
///   a=3, b=2  → pretzel
///   a=3, b=4  → complex knot
///
/// Slight detuning off integer ratios makes the pattern slowly rotate
/// and fill out a band, producing richer visuals.
pub struct Lissajous {
    freq_a: f64,
    freq_b: f64,
    delta: f64,
    damping: f64,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Lissajous {
    pub fn new() -> Self {
        let mut l = Self {
            freq_a: 3.0,
            freq_b: 2.0,
            delta: PI / 2.0,
            damping: 0.003,
            t: 0.0,
            max_t: 400.0,
            step: 0.01,
        };
        l.randomize();
        l
    }
}

impl Shape for Lissajous {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Pick from nice integer ratios, then detune slightly
        let pairs = [
            (1.0, 2.0),
            (1.0, 3.0),
            (2.0, 3.0),
            (3.0, 2.0),
            (3.0, 4.0),
            (4.0, 3.0),
            (3.0, 5.0),
            (5.0, 4.0),
            (4.0, 5.0),
            (5.0, 6.0),
            (7.0, 6.0),
            (5.0, 8.0),
        ];
        let (a, b) = pairs[rng.gen_range(0..pairs.len())];
        self.freq_a = a + rng.gen_range(-0.02..0.02);
        self.freq_b = b + rng.gen_range(-0.02..0.02);
        self.delta = rng.gen_range(0.0..TAU);
        self.damping = rng.gen_range(0.002..0.006);
        self.t = 0.0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn name(&self) -> &'static str {
        "lissajous"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }
        let decay = (-self.damping * self.t).exp();
        let x = (self.freq_a * self.t + self.delta).sin() * decay;
        let y = (self.freq_b * self.t).sin() * decay;
        self.t += self.step;
        Some((x, y))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "liss.freq_a" => Some(self.freq_a),
            "liss.freq_b" => Some(self.freq_b),
            "liss.delta" => Some(self.delta),
            "liss.damping" => Some(self.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "liss.freq_a" => self.freq_a = value,
            "liss.freq_b" => self.freq_b = value,
            "liss.delta" => self.delta = value,
            "liss.damping" => self.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("liss.freq_a", self.freq_a),
            ("liss.freq_b", self.freq_b),
            ("liss.delta", self.delta),
            ("liss.damping", self.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
