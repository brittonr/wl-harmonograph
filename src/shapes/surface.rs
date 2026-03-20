use std::f64::consts::{PI, TAU};

use rand::Rng;
use super::Shape;

/// Spinning 3D surface shapes projected to 2D.
///
/// Traces a continuous spiral path across a 3D surface (torus, sphere,
/// cylinder, cone, Möbius strip, spring) while slowly rotating the object.
/// The spiral fills in the surface over time, producing a pointillist
/// rendering of the solid form.
///
/// Each step returns one (x, y) point — the 3D surface point after
/// rotation and perspective projection.
pub struct Surface {
    kind: SurfaceKind,

    // Surface parameters
    big_r: f64,
    small_r: f64,
    wraps: f64,
    height: f64,

    // Walking state
    t: f64,
    dt: f64,

    // Rotation
    angle_x: f64,
    angle_y: f64,
    angle_z: f64,
    rot_speed_x: f64,
    rot_speed_y: f64,
    rot_speed_z: f64,

    // Projection
    perspective: f64,
    output_scale: f64,

    // Lifecycle
    steps_done: u64,
    max_steps: u64,
}

#[derive(Clone, Copy, PartialEq)]
enum SurfaceKind {
    Torus,
    Sphere,
    Cylinder,
    Cone,
    Mobius,
    Spring,
}

const SURFACE_KINDS: &[SurfaceKind] = &[
    SurfaceKind::Torus,
    SurfaceKind::Sphere,
    SurfaceKind::Cylinder,
    SurfaceKind::Cone,
    SurfaceKind::Mobius,
    SurfaceKind::Spring,
];

impl SurfaceKind {
    fn from_index(i: usize) -> Self {
        SURFACE_KINDS[i % SURFACE_KINDS.len()]
    }

    fn index(self) -> usize {
        SURFACE_KINDS.iter().position(|&k| k == self).unwrap_or(0)
    }
}

impl Surface {
    pub fn new() -> Self {
        let mut s = Self {
            kind: SurfaceKind::Torus,
            big_r: 0.6,
            small_r: 0.3,
            wraps: 80.0,
            height: 1.5,
            t: 0.0,
            dt: 0.008,
            angle_x: 0.3,
            angle_y: 0.0,
            angle_z: 0.0,
            rot_speed_x: 0.0015,
            rot_speed_y: 0.0020,
            rot_speed_z: 0.0005,
            perspective: 3.5,
            output_scale: 1.8,
            steps_done: 0,
            max_steps: 80000,
        };
        s.randomize();
        s
    }

    /// Evaluate the surface point at parameter t, returning (x, y, z).
    fn surface_point(&self, t: f64) -> [f64; 3] {
        match self.kind {
            SurfaceKind::Torus => {
                // Spiral around the torus tube: u goes around the ring,
                // v = wraps*u goes around the tube cross-section.
                let u = t;
                let v = self.wraps * t;
                let x = (self.big_r + self.small_r * v.cos()) * u.cos();
                let y = (self.big_r + self.small_r * v.cos()) * u.sin();
                let z = self.small_r * v.sin();
                [x, y, z]
            }
            SurfaceKind::Sphere => {
                // Spiral from north pole to south pole and back.
                // theta oscillates 0→π→0 while phi spins fast.
                let max_t = self.max_steps as f64 * self.dt;
                let frac = t / max_t;
                let theta = PI * (1.0 - (frac * TAU).cos()) / 2.0;
                let phi = self.wraps * t;
                let r = self.big_r;
                let x = r * theta.sin() * phi.cos();
                let y = r * theta.sin() * phi.sin();
                let z = r * theta.cos();
                [x, y, z]
            }
            SurfaceKind::Cylinder => {
                // Helical path up and down a cylinder.
                let max_t = self.max_steps as f64 * self.dt;
                let frac = t / max_t;
                let z_pos = self.height * (frac * TAU).sin();
                let angle = self.wraps * t;
                let r = self.big_r;
                [r * angle.cos(), r * angle.sin(), z_pos]
            }
            SurfaceKind::Cone => {
                // Spiral from tip to base and back.
                let max_t = self.max_steps as f64 * self.dt;
                let frac = t / max_t;
                let z_pos = self.height * (1.0 - (frac * TAU).cos()) / 2.0;
                let r = self.big_r * z_pos / self.height;
                let angle = self.wraps * t;
                [r * angle.cos(), r * angle.sin(), z_pos - self.height / 2.0]
            }
            SurfaceKind::Mobius => {
                // Möbius strip: a ribbon with a half-twist.
                // s oscillates across the width, t goes around the loop.
                let max_t = self.max_steps as f64 * self.dt;
                let frac = t / max_t;
                let s = self.small_r * (self.wraps * frac * TAU).sin();
                let u = t;
                let half = u / 2.0;
                let x = (self.big_r + s * half.cos()) * u.cos();
                let y = (self.big_r + s * half.cos()) * u.sin();
                let z = s * half.sin();
                [x, y, z]
            }
            SurfaceKind::Spring => {
                // Helical coil — a tube bent into a helix.
                // Main helix goes around the central axis, secondary
                // wrapping goes around the tube cross-section.
                let coils = self.wraps / 10.0;
                let z_pos = self.height * (t / (TAU * coils)) - self.height / 2.0;
                let major_angle = t;
                let minor_angle = self.wraps * t;
                let cx = self.big_r * major_angle.cos();
                let cy = self.big_r * major_angle.sin();
                // Offset along the radial and z directions for tube thickness
                let radial_dir_x = major_angle.cos();
                let radial_dir_y = major_angle.sin();
                let x = cx + self.small_r * minor_angle.cos() * radial_dir_x;
                let y = cy + self.small_r * minor_angle.cos() * radial_dir_y;
                let z = z_pos + self.small_r * minor_angle.sin();
                [x, y, z]
            }
        }
    }

    fn project(&self, p: [f64; 3]) -> (f64, f64) {
        let s = self.perspective / (self.perspective + p[2]);
        (p[0] * s * self.output_scale, p[1] * s * self.output_scale)
    }

    fn randomize_for_kind(&mut self, rng: &mut rand::rngs::ThreadRng) {
        match self.kind {
            SurfaceKind::Torus => {
                self.big_r = rng.gen_range(0.45..0.7);
                self.small_r = rng.gen_range(0.15..0.35);
                self.wraps = rng.gen_range(40.0f64..120.0).round();
                self.output_scale = rng.gen_range(1.5..2.2);
                self.dt = 0.008;
            }
            SurfaceKind::Sphere => {
                self.big_r = rng.gen_range(0.6..0.9);
                self.wraps = rng.gen_range(30.0f64..80.0).round();
                self.output_scale = rng.gen_range(1.3..1.8);
                self.dt = 0.006;
            }
            SurfaceKind::Cylinder => {
                self.big_r = rng.gen_range(0.4..0.7);
                self.height = rng.gen_range(1.0..2.0);
                self.wraps = rng.gen_range(30.0f64..80.0).round();
                self.output_scale = rng.gen_range(1.3..1.8);
                self.dt = 0.008;
            }
            SurfaceKind::Cone => {
                self.big_r = rng.gen_range(0.5..0.8);
                self.height = rng.gen_range(1.2..2.0);
                self.wraps = rng.gen_range(30.0f64..80.0).round();
                self.output_scale = rng.gen_range(1.3..1.8);
                self.dt = 0.008;
            }
            SurfaceKind::Mobius => {
                self.big_r = rng.gen_range(0.5..0.8);
                self.small_r = rng.gen_range(0.15..0.35);
                self.wraps = rng.gen_range(20.0f64..60.0).round();
                self.output_scale = rng.gen_range(1.3..1.8);
                self.dt = 0.008;
            }
            SurfaceKind::Spring => {
                self.big_r = rng.gen_range(0.35..0.6);
                self.small_r = rng.gen_range(0.08..0.2);
                self.height = rng.gen_range(1.0..2.0);
                self.wraps = rng.gen_range(15.0f64..40.0).round();
                self.output_scale = rng.gen_range(1.3..1.8);
                self.dt = 0.005;
            }
        }
    }
}

impl Shape for Surface {
    fn name(&self) -> &'static str {
        "surface"
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.steps_done >= self.max_steps {
            return None;
        }

        let p = self.surface_point(self.t);
        let rotated = super::rotate_xyz(p, self.angle_x, self.angle_y, self.angle_z);
        let (px, py) = self.project(rotated);

        self.t += self.dt;
        self.steps_done += 1;
        self.angle_x += self.rot_speed_x;
        self.angle_y += self.rot_speed_y;
        self.angle_z += self.rot_speed_z;

        Some((px, py))
    }

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        self.kind = SurfaceKind::from_index(rng.gen_range(0..SURFACE_KINDS.len()));
        self.randomize_for_kind(&mut rng);

        self.angle_x = rng.gen_range(0.0..TAU);
        self.angle_y = rng.gen_range(0.0..TAU);
        self.angle_z = rng.gen_range(0.0..TAU);

        self.rot_speed_x = rng.gen_range(0.0008..0.003) * super::random_sign(&mut rng);
        self.rot_speed_y = rng.gen_range(0.0008..0.003) * super::random_sign(&mut rng);
        self.rot_speed_z = rng.gen_range(0.0003..0.0015) * super::random_sign(&mut rng);

        self.perspective = rng.gen_range(2.5..5.0);

        self.t = 0.0;
        self.steps_done = 0;
    }

    fn reset(&mut self) {
        let mut rng = rand::thread_rng();
        self.angle_x = rng.gen_range(0.0..TAU);
        self.angle_y = rng.gen_range(0.0..TAU);
        self.t = 0.0;
        self.steps_done = 0;
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "surf.kind" => Some(self.kind.index() as f64),
            "surf.big_r" => Some(self.big_r),
            "surf.small_r" => Some(self.small_r),
            "surf.wraps" => Some(self.wraps),
            "surf.height" => Some(self.height),
            "surf.dt" => Some(self.dt),
            "surf.rot_x" => Some(self.rot_speed_x),
            "surf.rot_y" => Some(self.rot_speed_y),
            "surf.rot_z" => Some(self.rot_speed_z),
            "surf.perspective" => Some(self.perspective),
            "surf.scale" => Some(self.output_scale),
            "surf.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "surf.kind" => {
                self.kind = SurfaceKind::from_index(value as usize);
            }
            "surf.big_r" => self.big_r = value,
            "surf.small_r" => self.small_r = value,
            "surf.wraps" => self.wraps = value,
            "surf.height" => self.height = value,
            "surf.dt" => self.dt = value,
            "surf.rot_x" => self.rot_speed_x = value,
            "surf.rot_y" => self.rot_speed_y = value,
            "surf.rot_z" => self.rot_speed_z = value,
            "surf.perspective" => self.perspective = value,
            "surf.scale" => self.output_scale = value,
            "surf.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("surf.kind", self.kind.index() as f64),
            ("surf.big_r", self.big_r),
            ("surf.small_r", self.small_r),
            ("surf.wraps", self.wraps),
            ("surf.height", self.height),
            ("surf.dt", self.dt),
            ("surf.rot_x", self.rot_speed_x),
            ("surf.rot_y", self.rot_speed_y),
            ("surf.rot_z", self.rot_speed_z),
            ("surf.perspective", self.perspective),
            ("surf.scale", self.output_scale),
            ("surf.max_steps", self.max_steps as f64),
        ]
    }
}