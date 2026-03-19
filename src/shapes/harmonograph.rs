use std::f64::consts::PI;

use rand::Rng;

#[derive(Clone, Copy)]
struct Pendulum {
    amplitude: f64,
    frequency: f64,
    phase: f64,
    damping: f64,
}

impl Pendulum {
    #[inline(always)]
    fn eval(&self, t: f64) -> f64 {
        self.amplitude * (t * self.frequency + self.phase).sin() * (-self.damping * t).exp()
    }
}

pub struct Harmonograph {
    x1: Pendulum,
    x2: Pendulum,
    y1: Pendulum,
    y2: Pendulum,
    t: f64,
    max_t: f64,
    step: f64,
}

impl Harmonograph {
    pub fn new() -> Self {
        let mut h = Self {
            x1: Pendulum {
                amplitude: 0.0,
                frequency: 0.0,
                phase: 0.0,
                damping: 0.0,
            },
            x2: Pendulum {
                amplitude: 0.0,
                frequency: 0.0,
                phase: 0.0,
                damping: 0.0,
            },
            y1: Pendulum {
                amplitude: 0.0,
                frequency: 0.0,
                phase: 0.0,
                damping: 0.0,
            },
            y2: Pendulum {
                amplitude: 0.0,
                frequency: 0.0,
                phase: 0.0,
                damping: 0.0,
            },
            t: 0.0,
            max_t: 400.0,
            step: 0.01,
        };
        h.randomize();
        h
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        let base_freq = rng.gen_range(1.0..2.0);
        let ratios: [f64; 4] = [1.0, 2.0, 3.0, 4.0];

        let pick_freq = |rng: &mut rand::rngs::ThreadRng| -> f64 {
            let ratio = ratios[rng.gen_range(0..ratios.len())];
            base_freq * ratio + rng.gen_range(-0.03..0.03)
        };

        let pendulum = |rng: &mut rand::rngs::ThreadRng, freq: f64, primary: bool| -> Pendulum {
            Pendulum {
                amplitude: if primary {
                    rng.gen_range(0.6..1.0)
                } else {
                    rng.gen_range(0.1..0.35)
                },
                frequency: freq,
                phase: rng.gen_range(0.0..2.0 * PI),
                damping: rng.gen_range(0.002..0.006),
            }
        };

        let (f_x, f_y, f_x2, f_y2) = (
            pick_freq(&mut rng),
            pick_freq(&mut rng),
            pick_freq(&mut rng),
            pick_freq(&mut rng),
        );

        self.x1 = pendulum(&mut rng, f_x, true);
        self.x2 = pendulum(&mut rng, f_x2, false);
        self.y1 = pendulum(&mut rng, f_y, true);
        self.y2 = pendulum(&mut rng, f_y2, false);
        self.t = 0.0;
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
    }

    pub fn name() -> &'static str {
        "harmonograph"
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }
        let x = self.x1.eval(self.t) + self.x2.eval(self.t);
        let y = self.y1.eval(self.t) + self.y2.eval(self.t);
        self.t += self.step;
        Some((x, y))
    }

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "x1.freq" => Some(self.x1.frequency),
            "x1.amp" => Some(self.x1.amplitude),
            "x1.phase" => Some(self.x1.phase),
            "x1.damping" => Some(self.x1.damping),
            "x2.freq" => Some(self.x2.frequency),
            "x2.amp" => Some(self.x2.amplitude),
            "x2.phase" => Some(self.x2.phase),
            "x2.damping" => Some(self.x2.damping),
            "y1.freq" => Some(self.y1.frequency),
            "y1.amp" => Some(self.y1.amplitude),
            "y1.phase" => Some(self.y1.phase),
            "y1.damping" => Some(self.y1.damping),
            "y2.freq" => Some(self.y2.frequency),
            "y2.amp" => Some(self.y2.amplitude),
            "y2.phase" => Some(self.y2.phase),
            "y2.damping" => Some(self.y2.damping),
            "max_t" => Some(self.max_t),
            "step" => Some(self.step),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "x1.freq" => self.x1.frequency = value,
            "x1.amp" => self.x1.amplitude = value,
            "x1.phase" => self.x1.phase = value,
            "x1.damping" => self.x1.damping = value,
            "x2.freq" => self.x2.frequency = value,
            "x2.amp" => self.x2.amplitude = value,
            "x2.phase" => self.x2.phase = value,
            "x2.damping" => self.x2.damping = value,
            "y1.freq" => self.y1.frequency = value,
            "y1.amp" => self.y1.amplitude = value,
            "y1.phase" => self.y1.phase = value,
            "y1.damping" => self.y1.damping = value,
            "y2.freq" => self.y2.frequency = value,
            "y2.amp" => self.y2.amplitude = value,
            "y2.phase" => self.y2.phase = value,
            "y2.damping" => self.y2.damping = value,
            "max_t" => self.max_t = value,
            "step" => self.step = value,
            _ => return false,
        }
        true
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("x1.freq", self.x1.frequency),
            ("x1.amp", self.x1.amplitude),
            ("x1.phase", self.x1.phase),
            ("x1.damping", self.x1.damping),
            ("x2.freq", self.x2.frequency),
            ("x2.amp", self.x2.amplitude),
            ("x2.phase", self.x2.phase),
            ("x2.damping", self.x2.damping),
            ("y1.freq", self.y1.frequency),
            ("y1.amp", self.y1.amplitude),
            ("y1.phase", self.y1.phase),
            ("y1.damping", self.y1.damping),
            ("y2.freq", self.y2.frequency),
            ("y2.amp", self.y2.amplitude),
            ("y2.phase", self.y2.phase),
            ("y2.damping", self.y2.damping),
            ("max_t", self.max_t),
            ("step", self.step),
        ]
    }
}
