use std::f64::consts::PI;

use rand::Rng;
use super::Shape;

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

    /// Compute angular accelerations α1 and α2 for the double pendulum system.
    /// Based on the Lagrangian equations of motion.
    fn accelerations(&self, theta1: f64, theta2: f64, omega1: f64, omega2: f64) -> (f64, f64) {
        let dt = theta1 - theta2;
        let den = 2.0 * self.m1 + self.m2 - self.m2 * (2.0 * dt).cos();
        
        let num1 = -self.m2 * self.g * (theta1 - 2.0 * theta2).sin()
                   - 2.0 * dt.sin() * self.m2
                     * (omega2 * omega2 * self.l2 + omega1 * omega1 * self.l1 * dt.cos())
                   - self.g * (2.0 * self.m1 + self.m2) * theta1.sin();
        
        let num2 = 2.0 * dt.sin()
                   * (omega1 * omega1 * self.l1 * (self.m1 + self.m2)
                      + self.g * (self.m1 + self.m2) * theta1.cos()
                      + omega2 * omega2 * self.l2 * self.m2 * dt.cos());
        
        let alpha1 = num1 / (self.l1 * den);
        let alpha2 = num2 / (self.l2 * den);
        
        (alpha1, alpha2)
    }

    /// Runge-Kutta 4th order integration step for the double pendulum.
    /// State vector: (θ1, θ2, ω1, ω2)
    fn rk4_step(&mut self) {
        let dt = self.dt;
        
        // k1
        let (alpha1_1, alpha2_1) = self.accelerations(self.theta1, self.theta2, self.omega1, self.omega2);
        let dtheta1_1 = self.omega1;
        let dtheta2_1 = self.omega2;
        let domega1_1 = alpha1_1;
        let domega2_1 = alpha2_1;
        
        // k2
        let theta1_2 = self.theta1 + 0.5 * dt * dtheta1_1;
        let theta2_2 = self.theta2 + 0.5 * dt * dtheta2_1;
        let omega1_2 = self.omega1 + 0.5 * dt * domega1_1;
        let omega2_2 = self.omega2 + 0.5 * dt * domega2_1;
        let (alpha1_2, alpha2_2) = self.accelerations(theta1_2, theta2_2, omega1_2, omega2_2);
        let dtheta1_2 = omega1_2;
        let dtheta2_2 = omega2_2;
        let domega1_2 = alpha1_2;
        let domega2_2 = alpha2_2;
        
        // k3
        let theta1_3 = self.theta1 + 0.5 * dt * dtheta1_2;
        let theta2_3 = self.theta2 + 0.5 * dt * dtheta2_2;
        let omega1_3 = self.omega1 + 0.5 * dt * domega1_2;
        let omega2_3 = self.omega2 + 0.5 * dt * domega2_2;
        let (alpha1_3, alpha2_3) = self.accelerations(theta1_3, theta2_3, omega1_3, omega2_3);
        let dtheta1_3 = omega1_3;
        let dtheta2_3 = omega2_3;
        let domega1_3 = alpha1_3;
        let domega2_3 = alpha2_3;
        
        // k4
        let theta1_4 = self.theta1 + dt * dtheta1_3;
        let theta2_4 = self.theta2 + dt * dtheta2_3;
        let omega1_4 = self.omega1 + dt * domega1_3;
        let omega2_4 = self.omega2 + dt * domega2_3;
        let (alpha1_4, alpha2_4) = self.accelerations(theta1_4, theta2_4, omega1_4, omega2_4);
        let dtheta1_4 = omega1_4;
        let dtheta2_4 = omega2_4;
        let domega1_4 = alpha1_4;
        let domega2_4 = alpha2_4;
        
        // Update state
        self.theta1 += dt * (dtheta1_1 + 2.0 * dtheta1_2 + 2.0 * dtheta1_3 + dtheta1_4) / 6.0;
        self.theta2 += dt * (dtheta2_1 + 2.0 * dtheta2_2 + 2.0 * dtheta2_3 + dtheta2_4) / 6.0;
        self.omega1 += dt * (domega1_1 + 2.0 * domega1_2 + 2.0 * domega1_3 + domega1_4) / 6.0;
        self.omega2 += dt * (domega2_1 + 2.0 * domega2_2 + 2.0 * domega2_3 + domega2_4) / 6.0;
    }
}

impl Shape for DoPendulum {

    fn randomize(&mut self) {
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

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.theta1 = rng.gen_range(PI * 0.4..PI * 0.9);
        self.theta2 = rng.gen_range(PI * 0.3..PI * 0.8);
        self.omega1 = rng.gen_range(-1.0..1.0);
        self.omega2 = rng.gen_range(-1.0..1.0);
        self.steps_done = 0;
    }

    fn name(&self) -> &'static str {
        "dopendulum"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
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

    fn get_param(&self, name: &str) -> Option<f64> {
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

    fn set_param(&mut self, name: &str, value: f64) -> bool {
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

    fn all_params(&self) -> Vec<(&'static str, f64)> {
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
