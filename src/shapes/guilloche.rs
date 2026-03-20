use std::f64::consts::{PI, TAU};

use rand::Rng;
use super::Shape;

/// Guilloché patterns — the intricate engraving found on banknotes,
/// certificates, and watch dials.
///
/// Built by combining multiple harmonic oscillations in polar form:
///
///   r(θ) = r0 + a1*sin(f1*θ + p1) + a2*sin(f2*θ + p2) + a3*sin(f3*θ + p3)
///   x = r * cos(θ)
///   y = r * sin(θ)
///
/// The interplay of three frequency layers creates moiré-like interference
/// patterns with intricate detail at multiple scales.
pub struct Guilloche {
    r0: f64,
    amp1: f64,
    freq1: f64,
    phase1: f64,
    amp2: f64,
    freq2: f64,
    phase2: f64,
    amp3: f64,
    freq3: f64,
    phase3: f64,
    damping: f64,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Guilloche {
    pub fn new() -> Self {
        let mut g = Self {
            r0: 0.5,
            amp1: 0.3,
            freq1: 6.0,
            phase1: 0.0,
            amp2: 0.15,
            freq2: 13.0,
            phase2: 0.0,
            amp3: 0.08,
            freq3: 31.0,
            phase3: 0.0,
            damping: 0.002,
            t: 0.0,
            max_t: 400.0,
            step: 0.005,
        };
        g.randomize();
        g
    }
}

impl Shape for Guilloche {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        self.r0 = rng.gen_range(0.3..0.7);

        // Layer 1: large structure (low frequency, high amplitude)
        self.freq1 = rng.gen_range(3.0f64..10.0).round();
        self.amp1 = rng.gen_range(0.15..0.4);
        self.phase1 = rng.gen_range(0.0..TAU);

        // Layer 2: medium detail (higher frequency)
        self.freq2 = self.freq1 * rng.gen_range(1.5..4.0);
        // Slight detuning for visual richness
        self.freq2 += rng.gen_range(-0.3..0.3);
        self.amp2 = rng.gen_range(0.08..0.25);
        self.phase2 = rng.gen_range(0.0..TAU);

        // Layer 3: fine detail (highest frequency)
        self.freq3 = self.freq2 * rng.gen_range(1.5..3.5);
        self.freq3 += rng.gen_range(-0.2..0.2);
        self.amp3 = rng.gen_range(0.03..0.12);
        self.phase3 = rng.gen_range(0.0..TAU);

        self.damping = rng.gen_range(0.001..0.004);
        self.t = 0.0;

        // Run long enough for the pattern to develop and fade
        let max_freq = self.freq1.max(self.freq2).max(self.freq3);
        self.max_t = max_freq * PI * 2.0 + 300.0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn name(&self) -> &'static str {
        "guilloche"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }

        let decay = (-self.damping * self.t).exp();
        let r = self.r0
            + self.amp1 * (self.freq1 * self.t + self.phase1).sin()
            + self.amp2 * (self.freq2 * self.t + self.phase2).sin()
            + self.amp3 * (self.freq3 * self.t + self.phase3).sin();
        let r = r * decay;

        let x = r * self.t.cos();
        let y = r * self.t.sin();
        self.t += self.step;
        Some((x, y))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "guil.r0" => Some(self.r0),
            "guil.amp1" => Some(self.amp1),
            "guil.freq1" => Some(self.freq1),
            "guil.phase1" => Some(self.phase1),
            "guil.amp2" => Some(self.amp2),
            "guil.freq2" => Some(self.freq2),
            "guil.phase2" => Some(self.phase2),
            "guil.amp3" => Some(self.amp3),
            "guil.freq3" => Some(self.freq3),
            "guil.phase3" => Some(self.phase3),
            "guil.damping" => Some(self.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "guil.r0" => self.r0 = value,
            "guil.amp1" => self.amp1 = value,
            "guil.freq1" => self.freq1 = value,
            "guil.phase1" => self.phase1 = value,
            "guil.amp2" => self.amp2 = value,
            "guil.freq2" => self.freq2 = value,
            "guil.phase2" => self.phase2 = value,
            "guil.amp3" => self.amp3 = value,
            "guil.freq3" => self.freq3 = value,
            "guil.phase3" => self.phase3 = value,
            "guil.damping" => self.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("guil.r0", self.r0),
            ("guil.amp1", self.amp1),
            ("guil.freq1", self.freq1),
            ("guil.phase1", self.phase1),
            ("guil.amp2", self.amp2),
            ("guil.freq2", self.freq2),
            ("guil.phase2", self.phase2),
            ("guil.amp3", self.amp3),
            ("guil.freq3", self.freq3),
            ("guil.phase3", self.phase3),
            ("guil.damping", self.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
