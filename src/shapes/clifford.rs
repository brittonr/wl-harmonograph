use rand::Rng;
use super::Shape;

/// Bounding box estimate: the attractor fits within ~[-2.5, 2.5] on each axis.
const ATTRACTOR_SCALE: f64 = 2.5;

/// Clifford attractor — a 2D iterated map that produces swirling,
/// feathery structures from four parameters.
///
///   x_{n+1} = sin(a * y_n) + c * cos(a * x_n)
///   y_{n+1} = sin(b * x_n) + d * cos(b * y_n)
///
/// Discovered by Clifford Pickover. The attractor is bounded (roughly
/// [-2, 2] on each axis for most parameter choices) and exhibits rich
/// spiral/swirl structure depending on a, b, c, d.
pub struct Clifford {
    x: f64,
    y: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    steps_done: u64,
    max_steps: u64,
}

impl Clifford {
    pub fn new() -> Self {
        let mut s = Self {
            x: 0.1,
            y: 0.1,
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
            steps_done: 0,
            max_steps: 80000,
        };
        s.randomize();
        s
    }
}

impl Shape for Clifford {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Parameters in [-3, 3] produce the best visual variety.
        // Some known beautiful presets mixed with randomness.
        let presets: &[(f64, f64, f64, f64)] = &[
            (-1.4, 1.6, 1.0, 0.7),
            (1.7, 1.7, 0.6, 1.2),
            (-1.7, 1.3, -0.1, -1.2),
            (-1.8, -2.0, -0.5, -0.9),
            (1.5, -1.8, 1.6, 0.9),
            (-1.4, 1.7, 1.3, 0.5),
        ];

        if rng.gen_bool(0.4) {
            // Use a known preset with slight perturbation
            let p = presets[rng.gen_range(0..presets.len())];
            self.a = p.0 + rng.gen_range(-0.15..0.15);
            self.b = p.1 + rng.gen_range(-0.15..0.15);
            self.c = p.2 + rng.gen_range(-0.15..0.15);
            self.d = p.3 + rng.gen_range(-0.15..0.15);
        } else {
            self.a = rng.gen_range(-2.5..2.5);
            self.b = rng.gen_range(-2.5..2.5);
            self.c = rng.gen_range(-1.5..1.5);
            self.d = rng.gen_range(-1.5..1.5);
        }

        // Start near origin
        self.x = rng.gen_range(-0.1..0.1);
        self.y = rng.gen_range(-0.1..0.1);
        self.steps_done = 0;
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(-0.1..0.1);
        self.y = rng.gen_range(-0.1..0.1);
        self.steps_done = 0;
    }

    fn name(&self) -> &'static str {
        "clifford"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }

        let nx = (self.a * self.y).sin() + self.c * (self.a * self.x).cos();
        let ny = (self.b * self.x).sin() + self.d * (self.b * self.y).cos();
        self.x = nx;
        self.y = ny;
        self.steps_done += 1;

        // The attractor fits roughly in [-2.5, 2.5]; scale to [-1, 1]
        Some((self.x / ATTRACTOR_SCALE, self.y / ATTRACTOR_SCALE))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "cliff.a" => Some(self.a),
            "cliff.b" => Some(self.b),
            "cliff.c" => Some(self.c),
            "cliff.d" => Some(self.d),
            "cliff.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "cliff.a" => self.a = value,
            "cliff.b" => self.b = value,
            "cliff.c" => self.c = value,
            "cliff.d" => self.d = value,
            "cliff.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("cliff.a", self.a),
            ("cliff.b", self.b),
            ("cliff.c", self.c),
            ("cliff.d", self.d),
            ("cliff.max_steps", self.max_steps as f64),
        ]
    }
}
