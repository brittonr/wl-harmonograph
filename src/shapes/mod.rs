pub mod butterfly;
pub mod clifford;
pub mod dejong;
pub mod dopendulum;
pub mod guilloche;
pub mod harmonograph;
pub mod lissajous;
pub mod lorenz;
pub mod rose;
pub mod rossler;
pub mod spirograph;
pub mod superformula;
pub mod surface;
pub mod torusknot;
pub mod wireframe;

use rand::Rng;

use butterfly::Butterfly;

use clifford::Clifford;
use dejong::DeJong;
use dopendulum::DoPendulum;
use guilloche::Guilloche;
use harmonograph::Harmonograph;
use lissajous::Lissajous;
use lorenz::Lorenz;
use rose::Rose;
use rossler::Rossler;
use spirograph::Spirograph;
use superformula::Superformula;
use surface::Surface;
use torusknot::TorusKnot;
use wireframe::Wireframe;

/// Names of all available shapes (order matches creation order).
pub const SHAPE_NAMES: &[&str] = &[
    "harmonograph",
    "spirograph",
    "lissajous",
    "rose",
    "butterfly",
    "lorenz",
    "wireframe",
    "torusknot",
    "clifford",
    "dejong",
    "superformula",
    "guilloche",
    "dopendulum",
    "rossler",
    "surface",
];

/// Shape trait for all parametric curve generators.
pub trait Shape {
    /// Get the shape's name.
    fn name(&self) -> &'static str;
    
    /// Advance to the next point in the curve.
    /// Returns None when the curve is finished.
    fn step(&mut self) -> Option<(f64, f64)>;
    
    /// Randomize the shape's parameters.
    fn randomize(&mut self);
    
    /// Reset the shape's time/iteration state without changing parameters.
    ///
    /// For deterministic shapes (harmonograph, spirograph, lissajous, etc.)
    /// this simply rewinds the time counter. For chaotic systems (lorenz,
    /// rossler, clifford, etc.) this may also pick new initial conditions,
    /// since different starting points produce visually distinct trajectories.
    fn reset(&mut self);
    
    /// Get a parameter value by name.
    fn get_param(&self, name: &str) -> Option<f64>;
    
    /// Set a parameter value by name.
    /// Returns true if the parameter was found and set.
    fn set_param(&mut self, name: &str, value: f64) -> bool;
    
    /// Get all parameters as (name, value) pairs.
    fn all_params(&self) -> Vec<(&'static str, f64)>;
}

/// Create a new shape by name. Returns None if the name is unknown.
pub fn shape_from_name(name: &str) -> Option<Box<dyn Shape>> {
    match name {
        "harmonograph" => Some(Box::new(Harmonograph::new())),
        "spirograph" => Some(Box::new(Spirograph::new())),
        "lissajous" => Some(Box::new(Lissajous::new())),
        "rose" => Some(Box::new(Rose::new())),
        "butterfly" => Some(Box::new(Butterfly::new())),
        "lorenz" => Some(Box::new(Lorenz::new())),
        "wireframe" => Some(Box::new(Wireframe::new())),
        "torusknot" => Some(Box::new(TorusKnot::new())),
        "clifford" => Some(Box::new(Clifford::new())),
        "dejong" => Some(Box::new(DeJong::new())),
        "superformula" => Some(Box::new(Superformula::new())),
        "guilloche" => Some(Box::new(Guilloche::new())),
        "dopendulum" => Some(Box::new(DoPendulum::new())),
        "rossler" => Some(Box::new(Rossler::new())),
        "surface" => Some(Box::new(Surface::new())),
        _ => None,
    }
}

/// Create a random shape.
pub fn random_shape() -> Box<dyn Shape> {
    let mut rng = rand::thread_rng();
    let name = SHAPE_NAMES[rng.gen_range(0..SHAPE_NAMES.len())];
    shape_from_name(name).unwrap()
}

/// Get the next shape name in the list (wraps around).
pub fn next_shape_name(current: &str) -> &'static str {
    let idx = SHAPE_NAMES.iter().position(|&n| n == current).unwrap_or(0);
    SHAPE_NAMES[(idx + 1) % SHAPE_NAMES.len()]
}

// ---------------------------------------------------------------------------
// Shared 3D helpers
// ---------------------------------------------------------------------------

/// Rotate a 3D point around X, Y, and Z axes (extrinsic Euler rotation).
pub fn rotate_xyz(p: [f64; 3], angle_x: f64, angle_y: f64, angle_z: f64) -> [f64; 3] {
    let (sx, cx) = angle_x.sin_cos();
    let (sy, cy) = angle_y.sin_cos();
    let (sz, cz) = angle_z.sin_cos();
    let [x, y, z] = p;

    // Rx
    let (x1, y1, z1) = (x, cx * y - sx * z, sx * y + cx * z);
    // Ry
    let (x2, y2, z2) = (cy * x1 + sy * z1, y1, -sy * x1 + cy * z1);
    // Rz
    [cz * x2 - sz * y2, sz * x2 + cz * y2, z2]
}

/// Random sign: +1.0 or -1.0 with equal probability.
pub fn random_sign(rng: &mut impl Rng) -> f64 {
    if rng.gen_bool(0.5) { 1.0 } else { -1.0 }
}

// ---------------------------------------------------------------------------
// CurveDrawer — ring buffer + catmull-rom + triangle strip generation
// ---------------------------------------------------------------------------

pub struct CurveDrawer {
    pub shape: Box<dyn Shape>,
    ring: [(f64, f64); 4],
    ring_count: u32,
}

impl CurveDrawer {
    pub fn new(shape: Box<dyn Shape>) -> Self {
        Self {
            shape,
            ring: [(0.0, 0.0); 4],
            ring_count: 0,
        }
    }

    /// Advance the shape by one step, feeding the point into the ring buffer.
    /// Returns false when the shape is finished.
    #[inline]
    pub fn advance(&mut self) -> bool {
        let pt = match self.shape.step() {
            Some(pt) => pt,
            None => return false,
        };
        if self.ring_count >= 4 {
            self.ring[0] = self.ring[1];
            self.ring[1] = self.ring[2];
            self.ring[2] = self.ring[3];
            self.ring[3] = pt;
        } else {
            self.ring[self.ring_count as usize] = pt;
        }
        self.ring_count += 1;
        true
    }

    #[inline]
    pub fn catmull_rom_points(&self) -> Option<&[(f64, f64); 4]> {
        if self.ring_count >= 4 {
            Some(&self.ring)
        } else {
            None
        }
    }

    /// Randomize the current shape (keeps same shape type).
    pub fn randomize(&mut self) {
        self.shape.randomize();
        self.ring_count = 0;
    }

    /// Switch to a completely new random shape.
    pub fn randomize_new_shape(&mut self) {
        self.shape = random_shape();
        self.ring_count = 0;
    }

    /// Switch to a specific named shape with random params.
    pub fn switch_shape(&mut self, name: &str) -> bool {
        match shape_from_name(name) {
            Some(s) => {
                self.shape = s;
                self.ring_count = 0;
                true
            }
            None => false,
        }
    }

    /// Reset time without changing shape parameters.
    pub fn reset_time(&mut self) {
        self.shape.reset();
        self.ring_count = 0;
    }

    /// Append triangle-strip vertices for the current Catmull-Rom segment.
    ///
    /// Each vertex is `[x, y, cross]` where `cross` is +1.0 or -1.0 indicating
    /// the side of the line center (used for shader-based edge antialiasing).
    pub fn append_catmull_rom_strip(
        &self,
        scale_x: f64,
        scale_y: f64,
        line_width: f64,
        n_subdivisions: usize,
        verts: &mut Vec<[f32; 3]>,
    ) -> bool {
        let pts = match self.catmull_rom_points() {
            Some(p) => p,
            None => return false,
        };

        let p0 = (pts[0].0 * scale_x, pts[0].1 * scale_y);
        let p1 = (pts[1].0 * scale_x, pts[1].1 * scale_y);
        let p2 = (pts[2].0 * scale_x, pts[2].1 * scale_y);
        let p3 = (pts[3].0 * scale_x, pts[3].1 * scale_y);

        // Catmull-Rom → cubic Bezier control points
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);

        let hw = line_width * 0.5;

        // When appending to an existing strip, skip t=0 because the previous
        // segment already emitted those vertices (they share the same point).
        let start = if verts.is_empty() { 0 } else { 1 };

        for i in start..=n_subdivisions {
            let t = i as f64 / n_subdivisions as f64;
            let mt = 1.0 - t;
            let mt2 = mt * mt;
            let t2 = t * t;
            let x =
                mt2 * mt * p1.0 + 3.0 * mt2 * t * c1.0 + 3.0 * mt * t2 * c2.0 + t2 * t * p2.0;
            let y =
                mt2 * mt * p1.1 + 3.0 * mt2 * t * c1.1 + 3.0 * mt * t2 * c2.1 + t2 * t * p2.1;

            // Tangent for normal computation
            let dx = -3.0 * mt2 * p1.0
                + 3.0 * (mt2 - 2.0 * mt * t) * c1.0
                + 3.0 * (2.0 * mt * t - t2) * c2.0
                + 3.0 * t2 * p2.0;
            let dy = -3.0 * mt2 * p1.1
                + 3.0 * (mt2 - 2.0 * mt * t) * c1.1
                + 3.0 * (2.0 * mt * t - t2) * c2.1
                + 3.0 * t2 * p2.1;
            let len = (dx * dx + dy * dy).sqrt().max(1e-10);
            let nx = -dy / len * hw;
            let ny = dx / len * hw;

            verts.push([(x + nx) as f32, (y + ny) as f32, 1.0]);
            verts.push([(x - nx) as f32, (y - ny) as f32, -1.0]);
        }

        true
    }
}
