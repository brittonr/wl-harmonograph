use std::f64::consts::PI;

use rand::Rng;

/// Double pendulum — a chaotic mechanical system where a second pendulum
/// hangs from the end of the first.
///
/// The system has four state variables (θ1, θ2, ω1, ω2) and exhibits
/// sensitive dependence on initial conditions. We trace the position
/// of the lower bob, producing wild, looping trajectories.
///
/// Equations of motion (Lagrangian mechanics):
///
///   α1 = (-g(2m1+m2)sin(θ1) - m2*g*sin(θ1-2θ2)
///         - 2sin(θ1-θ2)*m2*(ω2²L2 + ω1²L1cos(θ1-θ2)))
///        / (L1*(2m1 + m2 - m2*cos(2θ1-2θ2)))
///
///   α2 = (2sin(θ1-θ2)*(ω1²L1(m1+m2) + g(m1+m2)cos(θ1)
///         + ω2²L2*m2*cos(θ1-θ2)))
///        / (L2*(2m1 + m2 - m2*cos(2θ1-2θ2)))
///
/// We use RK4 integration and track the lower bob's (x2, y2) position.
pub struct DoPendulum {
    theta1: f64,
    theta2: f64,
    omega1: f64,
    omega2: f64,
    l1: f64,
    l2: f64,
    m1: f64,
    m2: f64,
    g: f64,
    dt: f64,
    steps_done: u64,
    max_steps: u64,
}

impl DoPendulum {
    pub fn new() -> Self {
        let mut s = Self {
            theta1: PI / 2.0,
            theta2: PI / 2.0,
            omega1: 0.0,
            omega2: 0.0,
            l1: 0.5,
            l2: 0.5,
            m1: 1.0,
            m2: 1.0,
            g: 9.81,
            dt: 0.005,
            steps_done: 0,
            max_steps: 80000,
        };
        s.randomize();
        s
    }

    /// Compute angular accelerations (α1, α2) from current state.
    fn accelerations(
        theta1: f64,
        theta2: f64,
        omega1: f64,
        omega2: f64,
        l1: f64,
        l2: f64,
        m1: f64,
        m2: f64,
        g: f64,
    ) -> (f64, f64) {
        let dt = theta1 - theta2;
        let sin_dt = dt.sin();
        let cos_dt = dt.cos();
        let denom1 = l1 * (2.0 * m1 + m2 - m2 * (2.0 * dt).cos());
        let denom2 = l2 * (2.0 * m1 + m2 - m2 * (2.0 * dt).cos());

        let alpha1 = (-g * (2.0 * m1 + m2) * theta1.sin()
            - m2 * g * (theta1 - 2.0 * theta2).sin()
            - 2.0 * sin_dt * m2
                * (omega2 * omega2 * l2 + omega1 * omega1 * l1 * cos_dt))
            / denom1;

        let alpha2 = (2.0
            * sin_dt
            * (omega1 * omega1 * l1 * (m1 + m2)
                + g * (m1 + m2) * theta1.cos()
                + omega2 * omega2 * l2 * m2 * cos_dt))
            / denom2;

        (alpha1, alpha2)
    }

    fn rk4_step(&mut self) {
        let dt = self.dt;
        let (t1, t2, w1, w2) = (self.theta1, self.theta2, self.omega1, self.omega2);
        let (l1, l2, m1, m2, g) = (self.l1, self.l2, self.m1, self.m2, self.g);

        // k1
        let (a1_1, a2_1) = Self::accelerations(t1, t2, w1, w2, l1, l2, m1, m2, g);
        let k1 = (w1, w2, a1_1, a2_1);

        // k2
        let (a1_2, a2_2) = Self::accelerations(
            t1 + 0.5 * dt * k1.0,
            t2 + 0.5 * dt * k1.1,
            w1 + 0.5 * dt * k1.2,
            w2 + 0.5 * dt * k1.3,
            l1, l2, m1, m2, g,
        );
        let k2 = (
            w1 + 0.5 * dt * k1.2,
            w2 + 0.5 * dt * k1.3,
            a1_2,
            a2_2,
        );

        // k3
        let (a1_3, a2_3) = Self::accelerations(
            t1 + 0.5 * dt * k2.0,
            t2 + 0.5 * dt * k2.1,
            w1 + 0.5 * dt * k2.2,
            w2 + 0.5 * dt * k2.3,
            l1, l2, m1, m2, g,
        );
        let k3 = (
            w1 + 0.5 * dt * k2.2,
            w2 + 0.5 * dt * k2.3,
            a1_3,
            a2_3,
        );

        // k4
        let (a1_4, a2_4) = Self::accelerations(
            t1 + dt * k3.0,
            t2 + dt * k3.1,
            w1 + dt * k3.2,
            w2 + dt * k3.3,
            l1, l2, m1, m2, g,
        );
        let k4 = (
            w1 + dt * k3.2,
            w2 + dt * k3.3,
            a1_4,
            a2_4,
        );

        self.theta1 = t1 + dt / 6.0 * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0);
        self.theta2 = t2 + dt / 6.0 * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1);
        self.omega1 = w1 + dt / 6.0 * (k1.2 + 2.0 * k2.2 + 2.0 * k3.2 + k4.2);
        self.omega2 = w2 + dt / 6.0 * (k1.3 + 2.0 * k2.3 + 2.0 * k3.3 + k4.3);
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        // Arm lengths (total ≈ 1.0 for good scaling)
        self.l1 = rng.gen_range(0.3..0.7);
        self.l2 = 1.0 - self.l1;

        // Mass ratio
        self.m1 = 1.0;
        self.m2 = rng.gen_range(0.5..2.0);

        // Start from a high angle for dramatic motion
        self.theta1 = rng.gen_range(PI * 0.4..PI * 0.9);
        if rng.gen_bool(0.5) {
            self.theta1 = -self.theta1;
        }
        self.theta2 = rng.gen_range(PI * 0.3..PI * 0.8);
        if rng.gen_bool(0.5) {
            self.theta2 = -self.theta2;
        }

        // Start at rest or with slight angular velocity
        self.omega1 = rng.gen_range(-1.0..1.0);
        self.omega2 = rng.gen_range(-1.0..1.0);

        self.steps_done = 0;
    }

    pub fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.theta1 = rng.gen_range(PI * 0.4..PI * 0.9);
        self.theta2 = rng.gen_range(PI * 0.3..PI * 0.8);
        self.omega1 = rng.gen_range(-1.0..1.0);
        self.omega2 = rng.gen_range(-1.0..1.0);
        self.steps_done = 0;
    }

    pub fn name() -> &'static str {
        "dopendulum"
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }

        self.rk4_step();
        self.steps_done += 1;

        // Position of the lower bob, normalized to roughly [-1, 1]
        let total_l = self.l1 + self.l2;
        let x2 = self.l1 * self.theta1.sin() + self.l2 * self.theta2.sin();
        let y2 = -(self.l1 * self.theta1.cos() + self.l2 * self.theta2.cos());
        Some((x2 / total_l, y2 / total_l))
    }

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "dpend.l1" => Some(self.l1),
            "dpend.l2" => Some(self.l2),
            "dpend.m2" => Some(self.m2),
            "dpend.g" => Some(self.g),
            "dpend.dt" => Some(self.dt),
            "dpend.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "dpend.l1" => self.l1 = value,
            "dpend.l2" => self.l2 = value,
            "dpend.m2" => self.m2 = value,
            "dpend.g" => self.g = value,
            "dpend.dt" => self.dt = value,
            "dpend.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("dpend.l1", self.l1),
            ("dpend.l2", self.l2),
            ("dpend.m2", self.m2),
            ("dpend.g", self.g),
            ("dpend.dt", self.dt),
            ("dpend.max_steps", self.max_steps as f64),
        ]
    }
}
