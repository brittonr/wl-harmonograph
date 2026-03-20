use rand::Rng;
use super::Shape;

/// Lorenz strange attractor — Edward Lorenz's chaotic system (1963).
///
///   dx/dt = σ (y − x)
///   dy/dt = x (ρ − z) − y
///   dz/dt = x y − β z
///
/// Classic parameters: σ=10, ρ=28, β=8/3.
///
/// The attractor lives in roughly x∈[-20,20], y∈[-30,30], z∈[0,50].
/// We project to 2D using (x, z−25) and scale to fit [-1, 1].
///
/// Unlike the parametric curves, this is an ODE integrated with RK4.
/// No damping needed — the attractor is bounded by nature. Instead,
/// we run for a fixed number of integration steps.
pub struct Lorenz {
    // ODE state
    x: f64,
    y: f64,
    z: f64,
    // Parameters
    sigma: f64,
    rho: f64,
    beta: f64,
    dt: f64,
    // Iteration tracking
    steps_done: u64,
    max_steps: u64,
}

impl Lorenz {
    pub fn new() -> Self {
        let mut l = Self {
            x: 1.0,
            y: 1.0,
            z: 1.0,
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
            dt: 0.005,
            steps_done: 0,
            max_steps: 60000,
        };
        l.randomize();
        l
    }

    /// Runge-Kutta 4th order integration step for the Lorenz equations:
    ///   dx/dt = σ (y − x)
    ///   dy/dt = x (ρ − z) − y
    ///   dz/dt = x y − β z
    fn rk4_step(&mut self) {
        let dt = self.dt;
        
        // k1
        let dx1 = self.sigma * (self.y - self.x);
        let dy1 = self.x * (self.rho - self.z) - self.y;
        let dz1 = self.x * self.y - self.beta * self.z;
        
        // k2
        let x2 = self.x + 0.5 * dt * dx1;
        let y2 = self.y + 0.5 * dt * dy1;
        let z2 = self.z + 0.5 * dt * dz1;
        let dx2 = self.sigma * (y2 - x2);
        let dy2 = x2 * (self.rho - z2) - y2;
        let dz2 = x2 * y2 - self.beta * z2;
        
        // k3
        let x3 = self.x + 0.5 * dt * dx2;
        let y3 = self.y + 0.5 * dt * dy2;
        let z3 = self.z + 0.5 * dt * dz2;
        let dx3 = self.sigma * (y3 - x3);
        let dy3 = x3 * (self.rho - z3) - y3;
        let dz3 = x3 * y3 - self.beta * z3;
        
        // k4
        let x4 = self.x + dt * dx3;
        let y4 = self.y + dt * dy3;
        let z4 = self.z + dt * dz3;
        let dx4 = self.sigma * (y4 - x4);
        let dy4 = x4 * (self.rho - z4) - y4;
        let dz4 = x4 * y4 - self.beta * z4;
        
        // Update state
        self.x += dt * (dx1 + 2.0 * dx2 + 2.0 * dx3 + dx4) / 6.0;
        self.y += dt * (dy1 + 2.0 * dy2 + 2.0 * dy3 + dy4) / 6.0;
        self.z += dt * (dz1 + 2.0 * dz2 + 2.0 * dz3 + dz4) / 6.0;
    }
}

impl Shape for Lorenz {

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Vary parameters near the classic chaotic regime
        self.sigma = rng.gen_range(8.0..14.0);
        self.rho = rng.gen_range(24.0..35.0);
        self.beta = rng.gen_range(2.0..3.5);

        // Random initial condition near the attractor
        self.x = rng.gen_range(-5.0..5.0);
        self.y = rng.gen_range(-5.0..5.0);
        self.z = rng.gen_range(20.0..30.0);

        self.steps_done = 0;
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(-5.0..5.0);
        self.y = rng.gen_range(-5.0..5.0);
        self.z = rng.gen_range(20.0..30.0);
        self.steps_done = 0;
    }

    fn name(&self) -> &'static str {
        "lorenz"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }
        self.rk4_step();
        self.steps_done += 1;

        // Project to 2D: use (x, z-center) view, scaled to ~[-1, 1]
        let px = self.x / 25.0;
        let py = (self.z - 25.0) / 25.0;
        Some((px, py))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "lorenz.sigma" => Some(self.sigma),
            "lorenz.rho" => Some(self.rho),
            "lorenz.beta" => Some(self.beta),
            "lorenz.dt" => Some(self.dt),
            "lorenz.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "lorenz.sigma" => self.sigma = value,
            "lorenz.rho" => self.rho = value,
            "lorenz.beta" => self.beta = value,
            "lorenz.dt" => self.dt = value,
            "lorenz.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("lorenz.sigma", self.sigma),
            ("lorenz.rho", self.rho),
            ("lorenz.beta", self.beta),
            ("lorenz.dt", self.dt),
            ("lorenz.max_steps", self.max_steps as f64),
        ]
    }
}
