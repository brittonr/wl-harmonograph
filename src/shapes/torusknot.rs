use rand::Rng;

/// Torus knot — a closed curve winding p times around the axis of a
/// torus and q times through its hole.
///
///   x(t) = (R + r·cos(q·t)) · cos(p·t)
///   y(t) = (R + r·cos(q·t)) · sin(p·t)
///   z(t) = r·sin(q·t)
///
/// where (p, q) are coprime integers defining the knot type. A (2,3)
/// knot is the classic trefoil. Larger values produce denser, more
/// intricate patterns.
///
/// The shape slowly rotates in 3D, so each pass over the knot projects
/// at a slightly different angle. With trail accumulation this creates
/// dense, layered patterns that look convincingly three-dimensional.
pub struct TorusKnot {
    p: f64,
    q: f64,
    big_r: f64,
    small_r: f64,

    // Rotation
    angle_x: f64,
    angle_y: f64,
    rot_speed_x: f64,
    rot_speed_y: f64,

    // Projection
    perspective: f64,

    // Output
    output_scale: f64,
    damping: f64,

    // Lifecycle
    t: f64,
    max_t: f64,
    dt: f64,
}

/// Coprime (p, q) pairs that produce visually distinct knots.
const KNOT_TYPES: [(f64, f64); 8] = [
    (2.0, 3.0), // trefoil
    (2.0, 5.0), // cinquefoil
    (2.0, 7.0),
    (3.0, 4.0),
    (3.0, 5.0),
    (3.0, 7.0),
    (4.0, 5.0),
    (5.0, 7.0),
];

impl TorusKnot {
    pub fn new() -> Self {
        let mut k = Self {
            p: 2.0,
            q: 3.0,
            big_r: 1.0,
            small_r: 0.4,
            angle_x: 0.0,
            angle_y: 0.0,
            rot_speed_x: 0.001,
            rot_speed_y: 0.0013,
            perspective: 3.5,
            output_scale: 1.3,
            damping: 0.00003,
            t: 0.0,
            max_t: 150.0,
            dt: 0.008,
        };
        k.randomize();
        k
    }

    fn rotate(&self, x: f64, y: f64, z: f64) -> [f64; 3] {
        let (sx, cx) = self.angle_x.sin_cos();
        let (sy, cy) = self.angle_y.sin_cos();

        // Rx then Ry (two axes is enough for full coverage over time)
        let (x1, y1, z1) = (x, cx * y - sx * z, sx * y + cx * z);
        [cy * x1 + sy * z1, y1, -sy * x1 + cy * z1]
    }

    fn project(&self, p: [f64; 3]) -> (f64, f64) {
        let s = self.perspective / (self.perspective + p[2]);
        (p[0] * s, p[1] * s)
    }

    pub fn name() -> &'static str {
        "torusknot"
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        let (p, q) = KNOT_TYPES[rng.gen_range(0..KNOT_TYPES.len())];
        // Randomly swap p and q — same knot, different orientation
        if rng.gen_bool(0.5) {
            self.p = p;
            self.q = q;
        } else {
            self.p = q;
            self.q = p;
        }

        self.big_r = rng.gen_range(0.8..1.2);
        self.small_r = rng.gen_range(0.25..0.5);

        self.angle_x = rng.gen_range(0.0..std::f64::consts::TAU);
        self.angle_y = rng.gen_range(0.0..std::f64::consts::TAU);

        let sign = |rng: &mut rand::rngs::ThreadRng| {
            if rng.gen_bool(0.5) {
                1.0
            } else {
                -1.0
            }
        };
        self.rot_speed_x = rng.gen_range(0.0005..0.003) * sign(&mut rng);
        self.rot_speed_y = rng.gen_range(0.0005..0.003) * sign(&mut rng);

        self.perspective = rng.gen_range(3.0..5.0);
        self.output_scale = rng.gen_range(1.2..1.6);
        self.damping = rng.gen_range(0.00002..0.00005);
        self.dt = rng.gen_range(0.005..0.012);

        self.t = 0.0;
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        if self.t > self.max_t {
            return None;
        }

        let pt = self.p * self.t;
        let qt = self.q * self.t;

        let x = (self.big_r + self.small_r * qt.cos()) * pt.cos();
        let y = (self.big_r + self.small_r * qt.cos()) * pt.sin();
        let z = self.small_r * qt.sin();

        let rotated = self.rotate(x, y, z);
        let (px, py) = self.project(rotated);

        // Decay based on step count (not raw t) so damping rate is
        // independent of dt.
        let step_count = self.t / self.dt;
        let decay = (-self.damping * step_count).exp();

        self.t += self.dt;
        self.angle_x += self.rot_speed_x;
        self.angle_y += self.rot_speed_y;

        Some((
            px * decay * self.output_scale,
            py * decay * self.output_scale,
        ))
    }

    pub fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "knot.p" => Some(self.p),
            "knot.q" => Some(self.q),
            "knot.big_r" => Some(self.big_r),
            "knot.small_r" => Some(self.small_r),
            "knot.rot_x" => Some(self.rot_speed_x),
            "knot.rot_y" => Some(self.rot_speed_y),
            "knot.perspective" => Some(self.perspective),
            "knot.scale" => Some(self.output_scale),
            "knot.damping" => Some(self.damping),
            "knot.dt" => Some(self.dt),
            "knot.max_t" => Some(self.max_t),
            _ => None,
        }
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "knot.p" => self.p = value,
            "knot.q" => self.q = value,
            "knot.big_r" => self.big_r = value,
            "knot.small_r" => self.small_r = value,
            "knot.rot_x" => self.rot_speed_x = value,
            "knot.rot_y" => self.rot_speed_y = value,
            "knot.perspective" => self.perspective = value,
            "knot.scale" => self.output_scale = value,
            "knot.damping" => self.damping = value,
            "knot.dt" => self.dt = value,
            "knot.max_t" => self.max_t = value,
            _ => return false,
        }
        true
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("knot.p", self.p),
            ("knot.q", self.q),
            ("knot.big_r", self.big_r),
            ("knot.small_r", self.small_r),
            ("knot.rot_x", self.rot_speed_x),
            ("knot.rot_y", self.rot_speed_y),
            ("knot.perspective", self.perspective),
            ("knot.scale", self.output_scale),
            ("knot.damping", self.damping),
            ("knot.dt", self.dt),
            ("knot.max_t", self.max_t),
        ]
    }
}
