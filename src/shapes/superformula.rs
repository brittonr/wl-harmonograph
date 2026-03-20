use rand::Rng;
use super::Shape;

/// Superformula — Johan Gielis's generalization of the superellipse (2003).
///
///   r(θ) = ( |cos(m*θ/4)/a|^n2 + |sin(m*θ/4)/b|^n3 )^(-1/n1)
///
/// By varying m, n1, n2, n3, a, b this single formula produces circles,
/// ellipses, stars, flowers, gear shapes, organic blobs, and polygons.
///
/// We trace r(θ) as a polar curve with optional damping for fade-out.
pub struct Superformula {
    m: f64,
    n1: f64,
    n2: f64,
    n3: f64,
    a: f64,
    b: f64,
    damping: f64,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Superformula {
    pub fn new() -> Self {
        let mut s = Self {
            m: 6.0,
            n1: 1.0,
            n2: 1.0,
            n3: 1.0,
            a: 1.0,
            b: 1.0,
            damping: 0.003,
            t: 0.0,
            max_t: 300.0,
            step: 0.005,
        };
        s.randomize();
        s
    }

    fn radius(&self, theta: f64) -> f64 {
        let angle = self.m * theta / 4.0;
        let cos_term = (angle.cos() / self.a).abs().powf(self.n2);
        let sin_term = (angle.sin() / self.b).abs().powf(self.n3);
        (cos_term + sin_term).powf(-1.0 / self.n1)
    }
}

impl Shape for Superformula {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Curated presets that produce distinctive shapes, with randomization.
        let presets: &[(f64, f64, f64, f64, f64, f64)] = &[
            // (m, n1, n2, n3, a, b)
            // Star shapes
            (5.0, 2.0, 7.0, 7.0, 1.0, 1.0),
            (7.0, 3.0, 4.0, 17.0, 1.0, 1.0),
            (8.0, 0.5, 0.5, 8.0, 1.0, 1.0),
            // Flower / petal shapes
            (6.0, 1.0, 1.0, 1.0, 1.0, 1.0),
            (3.0, 5.0, 18.0, 18.0, 1.0, 1.0),
            (12.0, 15.0, 20.0, 3.0, 1.0, 1.0),
            // Gear / polygon
            (4.0, 100.0, 100.0, 100.0, 1.0, 1.0),
            (6.0, 60.0, 55.0, 55.0, 1.0, 1.0),
            // Organic / amoeba
            (3.0, 0.5, 1.7, 1.7, 1.0, 1.0),
            (5.0, 0.3, 0.3, 0.3, 1.0, 1.0),
            (7.0, 0.2, 1.7, 1.7, 1.0, 1.0),
            // Asymmetric
            (6.0, 1.0, 7.0, 8.0, 1.0, 1.2),
            (5.0, 2.0, 6.0, 6.0, 0.8, 1.0),
        ];

        if rng.gen_bool(0.5) {
            let p = presets[rng.gen_range(0..presets.len())];
            self.m = p.0 + rng.gen_range(-0.3..0.3);
            self.n1 = (p.1 + rng.gen_range(-0.2..0.2)).max(0.1);
            self.n2 = (p.2 + rng.gen_range(-0.5..0.5)).max(0.1);
            self.n3 = (p.3 + rng.gen_range(-0.5..0.5)).max(0.1);
            self.a = (p.4 + rng.gen_range(-0.1..0.1)).max(0.1);
            self.b = (p.5 + rng.gen_range(-0.1..0.1)).max(0.1);
        } else {
            self.m = rng.gen_range(2.0f64..12.0).round();
            self.n1 = rng.gen_range(0.2..20.0);
            self.n2 = rng.gen_range(0.2..20.0);
            self.n3 = rng.gen_range(0.2..20.0);
            self.a = rng.gen_range(0.5..1.5);
            self.b = rng.gen_range(0.5..1.5);
        }

        self.damping = rng.gen_range(0.001..0.005);
        self.t = 0.0;

        // Need enough revolutions to trace the full shape and let it fade
        self.max_t = self.m.max(4.0) * std::f64::consts::TAU + 150.0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
    }

    fn name(&self) -> &'static str {
        "superformula"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }

        let decay = (-self.damping * self.t).exp();
        let r = self.radius(self.t) * decay;
        let x = r * self.t.cos();
        let y = r * self.t.sin();
        self.t += self.step;
        Some((x, y))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "sf.m" => Some(self.m),
            "sf.n1" => Some(self.n1),
            "sf.n2" => Some(self.n2),
            "sf.n3" => Some(self.n3),
            "sf.a" => Some(self.a),
            "sf.b" => Some(self.b),
            "sf.damping" => Some(self.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "sf.m" => self.m = value,
            "sf.n1" => self.n1 = value,
            "sf.n2" => self.n2 = value,
            "sf.n3" => self.n3 = value,
            "sf.a" => self.a = value,
            "sf.b" => self.b = value,
            "sf.damping" => self.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("sf.m", self.m),
            ("sf.n1", self.n1),
            ("sf.n2", self.n2),
            ("sf.n3", self.n3),
            ("sf.a", self.a),
            ("sf.b", self.b),
            ("sf.damping", self.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
