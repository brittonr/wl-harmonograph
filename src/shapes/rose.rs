use std::f64::consts::PI;

use rand::Rng;

/// Rose (rhodonea) curves — polar flowers.
///
///   r(θ) = cos(k * θ)
///   x = r * cos(θ)
///   y = r * sin(θ)
///
/// When k is an integer:
///   - odd k  → k petals
///   - even k → 2k petals
///
/// When k = p/q (rational), the curve closes after q*π radians
/// (or 2*q*π if p*q is even), producing intricate multi-layered petals.
///
/// A secondary frequency is added for more complex petal structures.
pub struct Rose {
    k: f64,
    k2: f64,
    mix: f64,
    damping: f64,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Rose {
    pub fn new() -> Self {
        let mut r = Self {
            k: 3.0,
            k2: 0.0,
            mix: 0.0,
            damping: 0.003,
            t: 0.0,
            max_t: 300.0,
            step: 0.005,
        };
        r.randomize();
        r
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Pick from interesting k values (integer and rational)
        let k_choices: &[(f64, f64)] = &[
            // (numerator, denominator) — k = n/d
            (2.0, 1.0),
            (3.0, 1.0),
            (4.0, 1.0),
            (5.0, 1.0),
            (7.0, 1.0),
            (2.0, 3.0),
            (3.0, 2.0),
            (4.0, 3.0),
            (5.0, 3.0),
            (5.0, 2.0),
            (7.0, 3.0),
            (7.0, 4.0),
            (8.0, 5.0),
        ];
        let (n, d) = k_choices[rng.gen_range(0..k_choices.len())];
        self.k = n / d + rng.gen_range(-0.01..0.01);

        // Sometimes add a secondary frequency for compound petals
        if rng.gen_bool(0.4) {
            self.k2 = rng.gen_range(2.0..8.0);
            self.mix = rng.gen_range(0.1..0.3);
        } else {
            self.k2 = 0.0;
            self.mix = 0.0;
        }

        self.damping = rng.gen_range(0.001..0.005);
        self.t = 0.0;

        // Enough rotations to trace the full pattern
        self.max_t = d * PI * 4.0 + 80.0;
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
    }

    pub fn name() -> &'static str {
        "rose"
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }
        let decay = (-self.damping * self.t).exp();
        let mut r = (self.k * self.t).cos();
        if self.mix > 0.0 {
            r = r * (1.0 - self.mix) + (self.k2 * self.t).cos() * self.mix;
        }
        r *= decay;
        let x = r * self.t.cos();
        let y = r * self.t.sin();
        self.t += self.step;
        Some((x, y))
    }

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "rose.k" => Some(self.k),
            "rose.k2" => Some(self.k2),
            "rose.mix" => Some(self.mix),
            "rose.damping" => Some(self.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "rose.k" => self.k = value,
            "rose.k2" => self.k2 = value,
            "rose.mix" => self.mix = value,
            "rose.damping" => self.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("rose.k", self.k),
            ("rose.k2", self.k2),
            ("rose.mix", self.mix),
            ("rose.damping", self.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
