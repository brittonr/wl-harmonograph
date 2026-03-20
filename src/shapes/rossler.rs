use rand::Rng;
use super::Shape;

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

    fn rk4_step(&mut self) {
        let dt = self.dt;
        
        // k1
        let dx1 = -self.y - self.z;
        let dy1 = self.x + self.a * self.y;
        let dz1 = self.b + self.z * (self.x - self.c);
        
        // k2
        let x2 = self.x + 0.5 * dt * dx1;
        let y2 = self.y + 0.5 * dt * dy1;
        let z2 = self.z + 0.5 * dt * dz1;
        let dx2 = -y2 - z2;
        let dy2 = x2 + self.a * y2;
        let dz2 = self.b + z2 * (x2 - self.c);
        
        // k3
        let x3 = self.x + 0.5 * dt * dx2;
        let y3 = self.y + 0.5 * dt * dy2;
        let z3 = self.z + 0.5 * dt * dz2;
        let dx3 = -y3 - z3;
        let dy3 = x3 + self.a * y3;
        let dz3 = self.b + z3 * (x3 - self.c);
        
        // k4
        let x4 = self.x + dt * dx3;
        let y4 = self.y + dt * dy3;
        let z4 = self.z + dt * dz3;
        let dx4 = -y4 - z4;
        let dy4 = x4 + self.a * y4;
        let dz4 = self.b + z4 * (x4 - self.c);
        
        // Update state
        self.x += dt * (dx1 + 2.0 * dx2 + 2.0 * dx3 + dx4) / 6.0;
        self.y += dt * (dy1 + 2.0 * dy2 + 2.0 * dy3 + dy4) / 6.0;
        self.z += dt * (dz1 + 2.0 * dz2 + 2.0 * dz3 + dz4) / 6.0;
    }
}

impl Shape for Rossler {

    fn randomize(&mut self) {
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

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(-2.0..2.0);
        self.y = rng.gen_range(-2.0..2.0);
        self.z = rng.gen_range(0.0..1.0);
        self.steps_done = 0;
    }

    fn name(&self) -> &'static str {
        "rossler"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
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

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "ross.a" => Some(self.a),
            "ross.b" => Some(self.b),
            "ross.c" => Some(self.c),
            "ross.dt" => Some(self.dt),
            "ross.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
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

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("ross.a", self.a),
            ("ross.b", self.b),
            ("ross.c", self.c),
            ("ross.dt", self.dt),
            ("ross.max_steps", self.max_steps as f64),
        ]
    }
}
