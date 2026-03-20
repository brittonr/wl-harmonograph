use rand::Rng;
use super::Shape;

/// Rotating 3D wireframe Platonic solids projected to 2D.
///
/// Picks a random Platonic solid (tetrahedron, cube, octahedron, icosahedron,
/// or dodecahedron), then continuously traces its edges while slowly rotating
/// the shape in 3D. The trail accumulation creates ghostly overlapping
/// wireframes as the viewing angle drifts.
///
/// Edge traversal uses greedy DFS to build a continuous path through all
/// edges. Gaps between disconnected subpaths produce diagonal connecting
/// lines that add internal structure.
pub struct Wireframe {
    vertices: Vec<[f64; 3]>,
    path: Vec<usize>,

    // Walking state
    t: f64,
    steps_per_edge: f64,

    // Rotation
    angle_x: f64,
    angle_y: f64,
    angle_z: f64,
    rot_speed_x: f64,
    rot_speed_y: f64,
    rot_speed_z: f64,

    // Projection
    perspective: f64,

    // Output
    output_scale: f64,
    damping: f64,

    // Lifecycle
    total_steps: u64,
    max_steps: u64,
}

impl Wireframe {
    pub fn new() -> Self {
        let mut w = Self {
            vertices: vec![],
            path: vec![],
            t: 0.0,
            steps_per_edge: 80.0,
            angle_x: 0.0,
            angle_y: 0.0,
            angle_z: 0.0,
            rot_speed_x: 0.001,
            rot_speed_y: 0.0013,
            rot_speed_z: 0.0007,
            perspective: 3.0,
            output_scale: 2.0,
            damping: 0.00003,
            total_steps: 0,
            max_steps: 50000,
        };
        w.randomize();
        w
    }

    fn set_polyhedron(&mut self, name: &str) {
        let (verts, edges) = match name {
            "tetrahedron" => tetrahedron(),
            "octahedron" => octahedron(),
            "icosahedron" => icosahedron(),
            "dodecahedron" => dodecahedron(),
            _ => cube(),
        };
        self.path = build_edge_path(verts.len(), &edges);
        self.vertices = verts;
    }

    fn rotate(&self, p: [f64; 3]) -> [f64; 3] {
        let (sx, cx) = self.angle_x.sin_cos();
        let (sy, cy) = self.angle_y.sin_cos();
        let (sz, cz) = self.angle_z.sin_cos();
        let [x, y, z] = p;

        // Rx
        let (x1, y1, z1) = (x, cx * y - sx * z, sx * y + cx * z);
        // Ry
        let (x2, y2, z2) = (cy * x1 + sy * z1, y1, -sy * x1 + cy * z1);
        // Rz
        [cz * x2 - sz * y2, sz * x2 + cz * y2, z2]
    }

    fn project(&self, p: [f64; 3]) -> (f64, f64) {
        let s = self.perspective / (self.perspective + p[2]);
        (p[0] * s, p[1] * s)
    }
}

impl Shape for Wireframe {

    fn name(&self) -> &'static str {
        "wireframe"
    }

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();

        let polys = [
            "tetrahedron",
            "cube",
            "octahedron",
            "icosahedron",
            "dodecahedron",
        ];
        self.set_polyhedron(polys[rng.gen_range(0..polys.len())]);

        self.angle_x = rng.gen_range(0.0..std::f64::consts::TAU);
        self.angle_y = rng.gen_range(0.0..std::f64::consts::TAU);
        self.angle_z = rng.gen_range(0.0..std::f64::consts::TAU);

        let sign = |rng: &mut rand::rngs::ThreadRng| {
            if rng.gen_bool(0.5) {
                1.0
            } else {
                -1.0
            }
        };
        self.rot_speed_x = rng.gen_range(0.0005..0.003) * sign(&mut rng);
        self.rot_speed_y = rng.gen_range(0.0005..0.003) * sign(&mut rng);
        self.rot_speed_z = rng.gen_range(0.0003..0.002) * sign(&mut rng);

        self.perspective = rng.gen_range(2.5..4.0);
        self.output_scale = rng.gen_range(1.8..2.5);
        self.damping = rng.gen_range(0.00002..0.00005);
        self.steps_per_edge = rng.gen_range(60.0..120.0);

        self.t = 0.0;
        self.total_steps = 0;
    }

    fn reset(&mut self) {
        self.t = 0.0;
        self.total_steps = 0;
    }

    fn step(&mut self) -> Option<(f64, f64)> {
        if self.total_steps >= self.max_steps || self.path.len() < 2 {
            return None;
        }

        let path_len = (self.path.len() - 1) as f64;
        let wrapped = self.t % path_len;
        let seg = (wrapped.floor() as usize).min(self.path.len() - 2);
        let frac = wrapped - seg as f64;

        let v0 = self.vertices[self.path[seg]];
        let v1 = self.vertices[self.path[seg + 1]];

        // Interpolate in 3D
        let p = [
            v0[0] + (v1[0] - v0[0]) * frac,
            v0[1] + (v1[1] - v0[1]) * frac,
            v0[2] + (v1[2] - v0[2]) * frac,
        ];

        let rotated = self.rotate(p);
        let (px, py) = self.project(rotated);
        let decay = (-self.damping * self.total_steps as f64).exp();

        self.t += 1.0 / self.steps_per_edge;
        self.total_steps += 1;
        self.angle_x += self.rot_speed_x;
        self.angle_y += self.rot_speed_y;
        self.angle_z += self.rot_speed_z;

        Some((
            px * decay * self.output_scale,
            py * decay * self.output_scale,
        ))
    }

    fn get_param(&self, name: &str) -> Option<f64> {
        match name {
            "wire.rot_x" => Some(self.rot_speed_x),
            "wire.rot_y" => Some(self.rot_speed_y),
            "wire.rot_z" => Some(self.rot_speed_z),
            "wire.perspective" => Some(self.perspective),
            "wire.scale" => Some(self.output_scale),
            "wire.damping" => Some(self.damping),
            "wire.steps_per_edge" => Some(self.steps_per_edge),
            "wire.max_steps" => Some(self.max_steps as f64),
            _ => None,
        }
    }

    fn set_param(&mut self, name: &str, value: f64) -> bool {
        match name {
            "wire.rot_x" => self.rot_speed_x = value,
            "wire.rot_y" => self.rot_speed_y = value,
            "wire.rot_z" => self.rot_speed_z = value,
            "wire.perspective" => self.perspective = value,
            "wire.scale" => self.output_scale = value,
            "wire.damping" => self.damping = value,
            "wire.steps_per_edge" => self.steps_per_edge = value,
            "wire.max_steps" => self.max_steps = value as u64,
            _ => return false,
        }
        true
    }

    fn all_params(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("wire.rot_x", self.rot_speed_x),
            ("wire.rot_y", self.rot_speed_y),
            ("wire.rot_z", self.rot_speed_z),
            ("wire.perspective", self.perspective),
            ("wire.scale", self.output_scale),
            ("wire.damping", self.damping),
            ("wire.steps_per_edge", self.steps_per_edge),
            ("wire.max_steps", self.max_steps as f64),
        ]
    }
}

// ---------------------------------------------------------------------------
// Platonic solids
// ---------------------------------------------------------------------------

fn normalize(vertices: &mut [[f64; 3]]) {
    for v in vertices.iter_mut() {
        let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if len > 0.0 {
            v[0] /= len;
            v[1] /= len;
            v[2] /= len;
        }
    }
}

/// Find all edges where vertex distance matches `target ± tol`.
fn edges_by_distance(verts: &[[f64; 3]], target: f64, tol: f64) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for i in 0..verts.len() {
        for j in (i + 1)..verts.len() {
            let d = ((verts[i][0] - verts[j][0]).powi(2)
                + (verts[i][1] - verts[j][1]).powi(2)
                + (verts[i][2] - verts[j][2]).powi(2))
            .sqrt();
            if (d - target).abs() < tol {
                edges.push((i, j));
            }
        }
    }
    edges
}

fn tetrahedron() -> (Vec<[f64; 3]>, Vec<(usize, usize)>) {
    let mut v = vec![
        [1.0, 1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
    ];
    normalize(&mut v);
    let edges = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    (v, edges)
}

fn cube() -> (Vec<[f64; 3]>, Vec<(usize, usize)>) {
    let mut v = vec![
        [-1.0, -1.0, -1.0],
        [1.0, -1.0, -1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [1.0, -1.0, 1.0],
        [1.0, 1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    normalize(&mut v);
    let edges = vec![
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    (v, edges)
}

fn octahedron() -> (Vec<[f64; 3]>, Vec<(usize, usize)>) {
    // Already on the unit sphere
    let v = vec![
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    let edges = vec![
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
        (1, 2),
        (1, 3),
        (1, 4),
        (1, 5),
        (2, 4),
        (2, 5),
        (3, 4),
        (3, 5),
    ];
    (v, edges)
}

fn icosahedron() -> (Vec<[f64; 3]>, Vec<(usize, usize)>) {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let raw = vec![
        [0.0, 1.0, phi],
        [0.0, -1.0, phi],
        [0.0, 1.0, -phi],
        [0.0, -1.0, -phi],
        [1.0, phi, 0.0],
        [-1.0, phi, 0.0],
        [1.0, -phi, 0.0],
        [-1.0, -phi, 0.0],
        [phi, 0.0, 1.0],
        [phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
    ];
    // Adjacent icosahedron vertices are exactly distance 2.0 apart
    let edges = edges_by_distance(&raw, 2.0, 0.01);
    let mut v = raw;
    normalize(&mut v);
    (v, edges)
}

fn dodecahedron() -> (Vec<[f64; 3]>, Vec<(usize, usize)>) {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
    let inv = 1.0 / phi;
    let raw = vec![
        // Cube vertices
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
        [1.0, -1.0, 1.0],
        [1.0, -1.0, -1.0],
        [-1.0, 1.0, 1.0],
        [-1.0, 1.0, -1.0],
        [-1.0, -1.0, 1.0],
        [-1.0, -1.0, -1.0],
        // (0, ±1/φ, ±φ)
        [0.0, inv, phi],
        [0.0, inv, -phi],
        [0.0, -inv, phi],
        [0.0, -inv, -phi],
        // (±1/φ, ±φ, 0)
        [inv, phi, 0.0],
        [inv, -phi, 0.0],
        [-inv, phi, 0.0],
        [-inv, -phi, 0.0],
        // (±φ, 0, ±1/φ)
        [phi, 0.0, inv],
        [phi, 0.0, -inv],
        [-phi, 0.0, inv],
        [-phi, 0.0, -inv],
    ];
    // Adjacent dodecahedron vertices are distance 2/φ apart
    let edges = edges_by_distance(&raw, 2.0 / phi, 0.01);
    let mut v = raw;
    normalize(&mut v);
    (v, edges)
}

// ---------------------------------------------------------------------------
// Edge path construction
// ---------------------------------------------------------------------------

/// Build a continuous path through all edges using greedy DFS.
///
/// Returns vertex indices forming the path. When the greedy walk gets
/// stuck (all edges from the current vertex visited), it teleports to
/// the nearest vertex with remaining edges — the connecting line adds
/// internal structure to the wireframe.
fn build_edge_path(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    if edges.is_empty() || n == 0 {
        return vec![];
    }

    let mut adj: Vec<Vec<(usize, usize)>> = vec![vec![]; n];
    for (i, &(a, b)) in edges.iter().enumerate() {
        adj[a].push((b, i));
        adj[b].push((a, i));
    }

    let mut visited = vec![false; edges.len()];
    let mut path = vec![0usize];
    let mut current = 0;
    let mut count = 0;

    while count < edges.len() {
        let next = adj[current]
            .iter()
            .find(|&&(_, ei)| !visited[ei])
            .copied();

        if let Some((neighbor, ei)) = next {
            visited[ei] = true;
            count += 1;
            path.push(neighbor);
            current = neighbor;
        } else {
            // Teleport to a vertex that still has unvisited edges
            match (0..n).find(|&v| adj[v].iter().any(|&(_, ei)| !visited[ei])) {
                Some(v) => {
                    path.push(v);
                    current = v;
                }
                None => break,
            }
        }
    }

    // Close loop back to start for smooth wrapping
    if path.len() > 1 && *path.last().unwrap() != path[0] {
        path.push(path[0]);
    }

    path
}
