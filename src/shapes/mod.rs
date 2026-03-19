pub mod butterfly;
pub mod harmonograph;
pub mod lissajous;
pub mod lorenz;
pub mod rose;
pub mod spirograph;
pub mod torusknot;
pub mod wireframe;

use rand::Rng;

use butterfly::Butterfly;
use harmonograph::Harmonograph;
use lissajous::Lissajous;
use lorenz::Lorenz;
use rose::Rose;
use spirograph::Spirograph;
use torusknot::TorusKnot;
use wireframe::Wireframe;

/// Names of all available shapes (order matches enum discriminant).
pub const SHAPE_NAMES: &[&str] = &[
    "harmonograph",
    "spirograph",
    "lissajous",
    "rose",
    "butterfly",
    "lorenz",
    "wireframe",
    "torusknot",
];

pub enum Shape {
    Harmonograph(Harmonograph),
    Spirograph(Spirograph),
    Lissajous(Lissajous),
    Rose(Rose),
    Butterfly(Butterfly),
    Lorenz(Lorenz),
    Wireframe(Wireframe),
    TorusKnot(TorusKnot),
}

macro_rules! dispatch {
    ($self:expr, $method:ident) => {
        match $self {
            Shape::Harmonograph(s) => s.$method(),
            Shape::Spirograph(s) => s.$method(),
            Shape::Lissajous(s) => s.$method(),
            Shape::Rose(s) => s.$method(),
            Shape::Butterfly(s) => s.$method(),
            Shape::Lorenz(s) => s.$method(),
            Shape::Wireframe(s) => s.$method(),
            Shape::TorusKnot(s) => s.$method(),
        }
    };
    ($self:expr, $method:ident, $($arg:expr),+) => {
        match $self {
            Shape::Harmonograph(s) => s.$method($($arg),+),
            Shape::Spirograph(s) => s.$method($($arg),+),
            Shape::Lissajous(s) => s.$method($($arg),+),
            Shape::Rose(s) => s.$method($($arg),+),
            Shape::Butterfly(s) => s.$method($($arg),+),
            Shape::Lorenz(s) => s.$method($($arg),+),
            Shape::Wireframe(s) => s.$method($($arg),+),
            Shape::TorusKnot(s) => s.$method($($arg),+),
        }
    };
}

impl Shape {
    /// Create a new shape by name. Returns None if the name is unknown.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "harmonograph" => Some(Shape::Harmonograph(Harmonograph::new())),
            "spirograph" => Some(Shape::Spirograph(Spirograph::new())),
            "lissajous" => Some(Shape::Lissajous(Lissajous::new())),
            "rose" => Some(Shape::Rose(Rose::new())),
            "butterfly" => Some(Shape::Butterfly(Butterfly::new())),
            "lorenz" => Some(Shape::Lorenz(Lorenz::new())),
            "wireframe" => Some(Shape::Wireframe(Wireframe::new())),
            "torusknot" => Some(Shape::TorusKnot(TorusKnot::new())),
            _ => None,
        }
    }

    /// Create a random shape.
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let name = SHAPE_NAMES[rng.gen_range(0..SHAPE_NAMES.len())];
        Self::from_name(name).unwrap()
    }

    pub fn name(&self) -> &'static str {
        match self {
            Shape::Harmonograph(_) => Harmonograph::name(),
            Shape::Spirograph(_) => Spirograph::name(),
            Shape::Lissajous(_) => Lissajous::name(),
            Shape::Rose(_) => Rose::name(),
            Shape::Butterfly(_) => Butterfly::name(),
            Shape::Lorenz(_) => Lorenz::name(),
            Shape::Wireframe(_) => Wireframe::name(),
            Shape::TorusKnot(_) => TorusKnot::name(),
        }
    }

    /// Cycle to the next shape type (wraps around).
    pub fn next_name(&self) -> &'static str {
        let current = self.name();
        let idx = SHAPE_NAMES.iter().position(|&n| n == current).unwrap_or(0);
        SHAPE_NAMES[(idx + 1) % SHAPE_NAMES.len()]
    }

    pub fn step(&mut self) -> Option<(f64, f64)> {
        dispatch!(self, step)
    }

    #[allow(dead_code)]
    pub fn randomize(&mut self) {
        dispatch!(self, randomize)
    }

    pub fn reset(&mut self) {
        dispatch!(self, reset)
    }

    #[allow(dead_code)]
    pub fn get_param(&self, name: &str) -> Option<f64> {
        dispatch!(self, get_param, name)
    }

    pub fn set_param(&mut self, name: &str, value: f64) -> bool {
        dispatch!(self, set_param, name, value)
    }

    pub fn all_params(&self) -> Vec<(&'static str, f64)> {
        dispatch!(self, all_params)
    }
}

// ---------------------------------------------------------------------------
// CurveDrawer — ring buffer + catmull-rom + triangle strip generation
// ---------------------------------------------------------------------------

pub struct CurveDrawer {
    pub shape: Shape,
    ring: [(f64, f64); 4],
    ring_count: u32,
}

impl CurveDrawer {
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            ring: [(0.0, 0.0); 4],
            ring_count: 0,
        }
    }

    #[allow(dead_code)]
    pub fn new_random() -> Self {
        Self::new(Shape::random())
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
    #[allow(dead_code)]
    pub fn randomize(&mut self) {
        self.shape.randomize();
        self.ring_count = 0;
    }

    /// Switch to a completely new random shape.
    pub fn randomize_new_shape(&mut self) {
        self.shape = Shape::random();
        self.ring_count = 0;
    }

    /// Switch to a specific named shape with random params.
    pub fn switch_shape(&mut self, name: &str) -> bool {
        match Shape::from_name(name) {
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
