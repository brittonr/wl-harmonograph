use rand::Rng;

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

    fn derivatives(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        (
            self.sigma * (y - x),
            x * (self.rho - z) - y,
            x * y - self.beta * z,
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

    pub fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(-5.0..5.0);
        self.y = rng.gen_range(-5.0..5.0);
        self.z = rng.gen_range(20.0..30.0);
        self.steps_done = 0;
    }

    pub fn name() -> &'static str {
        "lorenz"
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
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

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "lorenz.sigma" => Some(self.sigma),
            "lorenz.rho" => Some(self.rho),
            "lorenz.beta" => Some(self.beta),
            "lorenz.dt" => Some(self.dt),
            "lorenz.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
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

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("lorenz.sigma", self.sigma),
            ("lorenz.rho", self.rho),
            ("lorenz.beta", self.beta),
            ("lorenz.dt", self.dt),
            ("lorenz.max_steps", self.max_steps as f64),
        ]
    }
}
