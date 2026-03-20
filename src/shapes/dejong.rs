use rand::Rng;
use super::Shape;

/// Bounding box estimate: output bounded in [-2, 2] on each axis.
const ATTRACTOR_SCALE: f64 = 2.0;

/// De Jong attractor — Peter de Jong's iterated 2D map.
///
///   x_{n+1} = sin(a * y_n) - cos(b * x_n)
///   y_{n+1} = sin(c * x_n) - cos(d * y_n)
///
/// Produces symmetric star-like, floral, and web structures depending
/// on the four parameters. The output is bounded in roughly [-2, 2].
pub struct DeJong {
    x: f64,
    y: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    steps_done: u64,
    max_steps: u64,
}

impl DeJong {
    pub fn new() -> Self {
        let mut s = Self {
            x: 0.1,
            y: 0.1,
            a: -2.0,
            b: -2.0,
            c: -1.2,
            d: 2.0,
            steps_done: 0,
            max_steps: 80000,
        };
        s.randomize();
        s
    }
}

impl Shape for DeJong {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        let presets: &[(f64, f64, f64, f64)] = &[
            (-2.0, -2.0, -1.2, 2.0),
            (1.4, -2.3, 2.4, -2.1),
            (2.01, -2.53, 1.61, -0.33),
            (-2.7, -0.09, -0.86, -2.2),
            (-0.827, -1.637, 1.659, -0.943),
            (2.462, -2.544, 2.284, -2.229),
            (-2.24, 0.43, -0.65, -2.43),
        ];

        if rng.gen_bool(0.4) {
            let p = presets[rng.gen_range(0..presets.len())];
            self.a = p.0 + rng.gen_range(-0.1..0.1);
            self.b = p.1 + rng.gen_range(-0.1..0.1);
            self.c = p.2 + rng.gen_range(-0.1..0.1);
            self.d = p.3 + rng.gen_range(-0.1..0.1);
        } else {
            self.a = rng.gen_range(-3.0..3.0);
            self.b = rng.gen_range(-3.0..3.0);
            self.c = rng.gen_range(-3.0..3.0);
            self.d = rng.gen_range(-3.0..3.0);
        }

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
        "dejong"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }

        let nx = (self.a * self.y).sin() - (self.b * self.x).cos();
        let ny = (self.c * self.x).sin() - (self.d * self.y).cos();
        self.x = nx;
        self.y = ny;
        self.steps_done += 1;

        // Output bounded in [-2, 2]; scale to [-1, 1]
        Some((self.x / ATTRACTOR_SCALE, self.y / ATTRACTOR_SCALE))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "dj.a" => Some(self.a),
            "dj.b" => Some(self.b),
            "dj.c" => Some(self.c),
            "dj.d" => Some(self.d),
            "dj.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "dj.a" => self.a = value,
            "dj.b" => self.b = value,
            "dj.c" => self.c = value,
            "dj.d" => self.d = value,
            "dj.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("dj.a", self.a),
            ("dj.b", self.b),
            ("dj.c", self.c),
            ("dj.d", self.d),
            ("dj.max_steps", self.max_steps as f64),
        ]
    }
}
