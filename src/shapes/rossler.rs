use rand::Rng;

/// Rössler attractor — Otto Rössler's chaotic system (1976).
///
///   dx/dt = -y - z
///   dy/dt = x + a*y
///   dz/dt = b + z*(x - c)
///
/// Simpler than Lorenz but produces a distinctive folded-band structure:
/// a flat spiral on the xy-plane with occasional z-axis excursions.
/// Classic parameters: a=0.2, b=0.2, c=5.7.
///
/// We project the (x, y) plane and scale to fit [-1, 1].
pub struct Rossler {
    x: f64,
    y: f64,
    z: f64,
    a: f64,
    b: f64,
    c: f64,
    dt: f64,
    steps_done: u64,
    max_steps: u64,
}

impl Rossler {
    pub fn new() -> Self {
        let mut s = Self {
            x: 1.0,
            y: 1.0,
            z: 0.0,
            a: 0.2,
            b: 0.2,
            c: 5.7,
            dt: 0.01,
            steps_done: 0,
            max_steps: 60000,
        };
        s.randomize();
        s
    }

    fn derivatives(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        (
            -y - z,
            x + self.a * y,
            self.b + z * (x - self.c),
        )
    }

    fn rk4_step(&mut self) {
        let dt = self.dt;
        let (x, y, z) = (self.x, self.y, self.z);

        let (k1x, k1y, k1z) = self.derivatives(x, y, z);
        let (k2x, k2y, k2z) = self.derivatives(
            x + 0.5 * dt * k1x,
            y + 0.5 * dt * k1y,
            z + 0.5 * dt * k1z,
        );
        let (k3x, k3y, k3z) = self.derivatives(
            x + 0.5 * dt * k2x,
            y + 0.5 * dt * k2y,
            z + 0.5 * dt * k2z,
        );
        let (k4x, k4y, k4z) =
            self.derivatives(x + dt * k3x, y + dt * k3y, z + dt * k3z);

        self.x = x + dt / 6.0 * (k1x + 2.0 * k2x + 2.0 * k3x + k4x);
        self.y = y + dt / 6.0 * (k1y + 2.0 * k2y + 2.0 * k3y + k4y);
        self.z = z + dt / 6.0 * (k1z + 2.0 * k2z + 2.0 * k3z + k4z);
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Stay near the chaotic regime but allow variation
        self.a = rng.gen_range(0.1..0.3);
        self.b = rng.gen_range(0.1..0.3);
        self.c = rng.gen_range(4.0..9.0);

        // Initial conditions near the attractor
        self.x = rng.gen_range(-2.0..2.0);
        self.y = rng.gen_range(-2.0..2.0);
        self.z = rng.gen_range(0.0..1.0);

        self.steps_done = 0;
    }

    pub fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(-2.0..2.0);
        self.y = rng.gen_range(-2.0..2.0);
        self.z = rng.gen_range(0.0..1.0);
        self.steps_done = 0;
    }

    pub fn name() -> &'static str {
        "rossler"
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }
        self.rk4_step();
        self.steps_done += 1;

        // The attractor extends roughly x∈[-12,12], y∈[-12,12] depending
        // on c. Scale adaptively based on c.
        let scale = if self.c > 6.0 { 15.0 } else { 10.0 };
        let px = self.x / scale;
        let py = self.y / scale;
        Some((px, py))
    }

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "ross.a" => Some(self.a),
            "ross.b" => Some(self.b),
            "ross.c" => Some(self.c),
            "ross.dt" => Some(self.dt),
            "ross.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "ross.a" => self.a = value,
            "ross.b" => self.b = value,
            "ross.c" => self.c = value,
            "ross.dt" => self.dt = value,
            "ross.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("ross.a", self.a),
            ("ross.b", self.b),
            ("ross.c", self.c),
            ("ross.dt", self.dt),
            ("ross.max_steps", self.max_steps as f64),
        ]
    }
}
