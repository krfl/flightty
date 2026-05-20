use nalgebra::{Point3, UnitQuaternion, Vector3};
use rand::Rng;

use crate::render::framebuffer::Color;
use crate::world::mesh::{Aabb, Mesh};
use crate::world::primitives;
use crate::world::scene::Scene;

const MAP_HALF: f32 = 85.0;
const SPAWN_CLEAR: f32 = 12.0;
const MIN_OBJECT_SPACING: f32 = 8.0;

// ─── Color palette system ───

struct Palette {
    primary: Color,
    secondary: Color,
    accent: Color,
    structure: Color,
    dark: Color,
}

const PALETTES: [[Color; 3]; 5] = [
    [Color::new(255, 100, 30), Color::new(30, 150, 255), Color::new(255, 220, 30)],   // orange/blue/yellow
    [Color::new(30, 220, 80), Color::new(180, 50, 220), Color::new(255, 80, 180)],    // green/purple/pink
    [Color::new(30, 220, 220), Color::new(220, 40, 40), Color::new(255, 220, 30)],    // cyan/red/yellow
    [Color::new(255, 100, 30), Color::new(30, 220, 80), Color::new(30, 150, 255)],    // orange/green/blue
    [Color::new(180, 50, 220), Color::new(30, 220, 220), Color::new(255, 100, 30)],   // purple/cyan/orange
];

fn random_palette(rng: &mut impl Rng) -> Palette {
    let colors = PALETTES[rng.random_range(0..PALETTES.len())];
    Palette {
        primary: colors[0],
        secondary: colors[1],
        accent: colors[2],
        structure: Color::new(90, 90, 90),
        dark: Color::new(60, 60, 60),
    }
}

impl Palette {
    fn pick(&self, rng: &mut impl Rng) -> Color {
        match rng.random_range(0..3) {
            0 => self.primary,
            1 => self.secondary,
            _ => self.accent,
        }
    }
}

fn in_bounds(x: f32, z: f32) -> bool {
    x.abs() < MAP_HALF && z.abs() < MAP_HALF
}

fn clear_of_spawn(x: f32, z: f32) -> bool {
    (x * x + z * z).sqrt() > SPAWN_CLEAR
}

fn far_enough(x: f32, z: f32, placed: &[(f32, f32)], min_dist: f32) -> bool {
    placed.iter().all(|&(px, pz)| {
        let dx = x - px;
        let dz = z - pz;
        (dx * dx + dz * dz).sqrt() >= min_dist
    })
}

/// Generate a random freestyle course.
pub fn random_course() -> Scene {
    let mut rng = rand::rng();
    let mut meshes = Vec::new();
    let mut placed: Vec<(f32, f32)> = Vec::new();
    let pal = random_palette(&mut rng);

    // Ground plane
    meshes.push(primitives::ground_plane(200.0, 50));

    // Central landmark, always present to help with orientation
    generate_landmark(&mut rng, &pal, &mut meshes, &mut placed);

    // Gate circuit with sequences and height variation
    let first_gate = generate_gate_circuit(&mut rng, &pal, &mut meshes, &mut placed);

    // Freestyle clusters: 4-6 spread around
    let num_clusters = rng.random_range(4..=6);
    for _ in 0..num_clusters {
        for _ in 0..15 {
            let angle = rng.random_range(0.0..std::f32::consts::TAU);
            let dist = rng.random_range(20.0..75.0f32);
            let cx = angle.cos() * dist;
            let cz = angle.sin() * dist;
            if in_bounds(cx, cz) && clear_of_spawn(cx, cz) && far_enough(cx, cz, &placed, MIN_OBJECT_SPACING) {
                generate_cluster(&mut rng, &pal, cx, cz, &mut meshes);
                placed.push((cx, cz));
                break;
            }
        }
    }

    // Scattered obstacles
    let num_scattered = rng.random_range(12..=20);
    for _ in 0..num_scattered {
        for _ in 0..10 {
            let x = rng.random_range(-75.0..75.0f32);
            let z = rng.random_range(-75.0..75.0f32);
            if in_bounds(x, z) && clear_of_spawn(x, z) && far_enough(x, z, &placed, 6.0) {
                generate_scattered(&mut rng, &pal, x, z, &mut meshes);
                placed.push((x, z));
                break;
            }
        }
    }

    // Power loop pillars near spawn
    for i in 0..3 {
        let x = -4.0 + i as f32 * 4.0;
        let h = rng.random_range(6.0..10.0);
        meshes.push(primitives::pillar(Point3::new(x, 0.0, -15.0), 0.4, h, pal.pick(&mut rng)));
    }

    let colliders: Vec<Aabb> = meshes
        .iter()
        .filter(|m| m.collidable)
        .map(|m| m.world_aabb())
        .collect();

    // Spawn facing the first gate
    let spawn_rot = if let Some((gx, gz)) = first_gate {
        let dir = Vector3::new(gx, 0.0, gz).normalize();
        UnitQuaternion::face_towards(&dir, &Vector3::y())
    } else {
        UnitQuaternion::identity()
    };

    Scene {
        meshes,
        colliders,
        spawn_position: Point3::new(0.0, 0.05, 0.0),
        spawn_orientation: spawn_rot,
    }
}

// ─── Central landmark ───

fn generate_landmark(rng: &mut impl Rng, pal: &Palette, meshes: &mut Vec<Mesh>, placed: &mut Vec<(f32, f32)>) {
    let cx = rng.random_range(-10.0..10.0f32);
    let cz = rng.random_range(-25.0..-15.0f32);

    match rng.random_range(0..3) {
        0 => {
            // Tall tower with archway
            let h = rng.random_range(12.0..20.0);
            meshes.push(primitives::pillar(Point3::new(cx, 0.0, cz), 1.2, h, pal.primary));
            // Archway at base
            meshes.push(primitives::pillar(Point3::new(cx - 3.0, 0.0, cz), 0.5, 6.0, pal.secondary));
            meshes.push(primitives::pillar(Point3::new(cx + 3.0, 0.0, cz), 0.5, 6.0, pal.secondary));
            meshes.push(primitives::wall(Point3::new(cx, 5.5, cz), 0.0, 6.5, 1.0, 0.4, pal.secondary));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 0.0, 4.0, 4.5, pal.accent));
        }
        1 => {
            // Stacked cubes monument
            meshes.push(primitives::cube(Point3::new(cx, 3.0, cz), 6.0, pal.structure));
            meshes.push(primitives::cube(Point3::new(cx, 9.0, cz), 5.0, pal.dark));
            meshes.push(primitives::cube(Point3::new(cx, 14.0, cz), 3.0, pal.primary));
            // Gates on two sides
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz + 3.0), 0.0, 3.5, 4.5, pal.accent));
            meshes.extend(primitives::gate(Point3::new(cx + 3.0, 0.0, cz), 90.0, 3.5, 4.5, pal.secondary));
        }
        _ => {
            // Four pillars with high bridge; fly under or over
            let h = rng.random_range(10.0..15.0);
            for &(dx, dz) in &[(-4.0, -4.0), (4.0, -4.0), (-4.0, 4.0), (4.0, 4.0)] {
                meshes.push(primitives::pillar(Point3::new(cx + dx, 0.0, cz + dz), 0.6, h, pal.primary));
            }
            meshes.push(primitives::wall(Point3::new(cx, h - 1.5, cz), 0.0, 9.0, 1.5, 9.0, pal.secondary));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 0.0, 5.0, 4.0, pal.accent));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 90.0, 5.0, 4.0, pal.accent));
        }
    }
    placed.push((cx, cz));
}

// ─── Gate circuit with sequences and height variation ───

fn generate_gate_circuit(
    rng: &mut impl Rng, pal: &Palette, meshes: &mut Vec<Mesh>, placed: &mut Vec<(f32, f32)>,
) -> Option<(f32, f32)> {
    let num_gates = rng.random_range(10..=14);
    let radius_x = rng.random_range(35.0..65.0f32);
    let radius_z = rng.random_range(35.0..65.0f32);
    let offset_x = rng.random_range(-8.0..8.0f32);
    let offset_z = rng.random_range(-10.0..0.0f32);

    let mut first_gate: Option<(f32, f32)> = None;
    let gate_colors = [pal.primary, pal.secondary, pal.accent];

    for i in 0..num_gates {
        let angle = (i as f32 / num_gates as f32) * std::f32::consts::TAU
            + rng.random_range(-0.12..0.12);

        let base_x = offset_x + angle.cos() * radius_x;
        let base_z = offset_z + angle.sin() * radius_z;
        let x = (base_x + rng.random_range(-6.0..6.0f32)).clamp(-MAP_HALF + 5.0, MAP_HALF - 5.0);
        let z = (base_z + rng.random_range(-6.0..6.0f32)).clamp(-MAP_HALF + 5.0, MAP_HALF - 5.0);

        if !clear_of_spawn(x, z) { continue; }

        let tangent_angle = angle + std::f32::consts::FRAC_PI_2;
        let rot = tangent_angle.to_degrees() + rng.random_range(-15.0..15.0);
        let color = gate_colors[i % gate_colors.len()];

        // Height variation: some ground, some mid, some high
        let y = match rng.random_range(0..10) {
            0..=5 => 0.0,                         // 60% ground level
            6..=7 => rng.random_range(2.0..4.0),  // 20% mid height
            8 => rng.random_range(5.0..8.0),       // 10% high (dive target)
            _ => 0.0,
        };

        let width = rng.random_range(2.5..4.0);
        let height = rng.random_range(2.0..3.5);

        meshes.extend(primitives::gate(Point3::new(x, y, z), rot, width, height, color));
        placed.push((x, z));

        if first_gate.is_none() {
            first_gate = Some((x, z));
        }

        // Support pillars for elevated gates
        if y > 0.3 {
            let hw = width / 2.0 + 0.1;
            let sin_r = rot.to_radians().sin();
            let cos_r = rot.to_radians().cos();
            meshes.push(primitives::pillar(
                Point3::new(x - cos_r * hw, 0.0, z + sin_r * hw), 0.15, y, pal.structure,
            ));
            meshes.push(primitives::pillar(
                Point3::new(x + cos_r * hw, 0.0, z - sin_r * hw), 0.15, y, pal.structure,
            ));
        }

        // Gate sequences: occasionally add 1-2 follow-up gates in the same direction
        if rng.random_bool(0.25) && i < num_gates - 1 {
            let seq_count = rng.random_range(1..=2);
            let dx = tangent_angle.cos() * 8.0;
            let dz = tangent_angle.sin() * 8.0;
            let seq_sin = rot.to_radians().sin();
            let seq_cos = rot.to_radians().cos();
            for s in 1..=seq_count {
                let sx = (x + dx * s as f32).clamp(-MAP_HALF + 5.0, MAP_HALF - 5.0);
                let sz = (z + dz * s as f32).clamp(-MAP_HALF + 5.0, MAP_HALF - 5.0);
                if in_bounds(sx, sz) && clear_of_spawn(sx, sz) {
                    let seq_y = if y > 0.3 {
                        (y - s as f32 * 2.0).max(0.0)
                    } else {
                        0.0
                    };
                    meshes.extend(primitives::gate(
                        Point3::new(sx, seq_y, sz), rot, width, height, color,
                    ));
                    placed.push((sx, sz));
                    if seq_y > 0.3 {
                        let hw2 = width / 2.0 + 0.1;
                        meshes.push(primitives::pillar(
                            Point3::new(sx - seq_cos * hw2, 0.0, sz + seq_sin * hw2), 0.15, seq_y, pal.structure,
                        ));
                        meshes.push(primitives::pillar(
                            Point3::new(sx + seq_cos * hw2, 0.0, sz - seq_sin * hw2), 0.15, seq_y, pal.structure,
                        ));
                    }
                }
            }
        }
    }

    first_gate
}

// ─── Clusters ───

fn generate_cluster(rng: &mut impl Rng, pal: &Palette, cx: f32, cz: f32, meshes: &mut Vec<Mesh>) {
    match rng.random_range(0..13) {
        0 => {
            // Pillar forest: weaving territory
            let n = rng.random_range(4..=7);
            for _ in 0..n {
                let x = cx + rng.random_range(-6.0..6.0f32);
                let z = cz + rng.random_range(-6.0..6.0f32);
                if !in_bounds(x, z) { continue; }
                let h = rng.random_range(5.0..14.0);
                meshes.push(primitives::pillar(Point3::new(x, 0.0, z), rng.random_range(0.3..0.7), h, pal.pick(rng)));
            }
        }
        1 => {
            // Multi-level bando with gates at different heights on different sides
            let rot = rng.random_range(0.0..180.0f32);
            meshes.push(primitives::cube(Point3::new(cx, 3.0, cz), 6.0, pal.structure));
            meshes.push(primitives::cube(Point3::new(cx, 9.0, cz), 6.0, pal.dark));
            // Low gate
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz + 3.0), rot, 3.5, 4.0, pal.primary));
            // High gate on opposite side
            meshes.extend(primitives::gate(Point3::new(cx, 5.0, cz - 3.0), rot + 180.0, 3.5, 3.0, pal.accent));
            // Side gate at mid height
            meshes.extend(primitives::gate(Point3::new(cx + 3.0, 2.5, cz), rot + 90.0, 3.0, 3.0, pal.secondary));
        }
        2 => {
            // Twin towers with bridge: split-S territory
            let color = pal.pick(rng);
            let h = rng.random_range(8.0..16.0);
            let gap = rng.random_range(4.0..7.0);
            meshes.push(primitives::pillar(Point3::new(cx - gap, 0.0, cz), 0.7, h, color));
            meshes.push(primitives::pillar(Point3::new(cx + gap, 0.0, cz), 0.7, h, color));
            meshes.push(primitives::wall(Point3::new(cx, h - 2.0, cz), 0.0, gap * 2.0 + 1.0, 1.5, 0.5, color));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 0.0, gap * 1.5, 3.0, pal.accent));
        }
        4 => {
            // Ramp pair: matty flip playground
            let color = pal.pick(rng);
            let rot = rng.random_range(0.0..360.0f32);
            meshes.push(primitives::ramp(Point3::new(cx - 4.0, 0.0, cz), rot, 5.0, 7.0, 4.0, color));
            meshes.push(primitives::ramp(Point3::new(cx + 4.0, 0.0, cz), rot + 180.0, 5.0, 7.0, 4.0, color));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), rot + 90.0, 3.0, 3.0, pal.accent));
        }
        5 => {
            // Archway row: power loop targets
            let rot = rng.random_range(0.0..180.0f32);
            let count = rng.random_range(2..=4);
            let sin_r = rot.to_radians().sin();
            let cos_r = rot.to_radians().cos();
            for i in 0..count {
                let offset = (i as f32 - (count - 1) as f32 / 2.0) * 7.0;
                let ax = cx + cos_r * offset;
                let az = cz + sin_r * offset;
                if !in_bounds(ax, az) { continue; }
                let h = rng.random_range(6.0..10.0);
                let color = pal.pick(rng);
                // Pillars offset perpendicular to row direction
                meshes.push(primitives::pillar(Point3::new(ax - sin_r * 2.5, 0.0, az + cos_r * 2.5), 0.4, h, color));
                meshes.push(primitives::pillar(Point3::new(ax + sin_r * 2.5, 0.0, az - cos_r * 2.5), 0.4, h, color));
                // Beam spans between pillars: wall X must align with pillar offset direction
                meshes.push(primitives::wall(Point3::new(ax, h - 0.5, az), 90.0 - rot, 5.5, 1.0, 0.4, color));
            }
        }
        6 => {
            // Stacked gates: two heights at same position
            let rot = rng.random_range(0.0..360.0f32);
            // Low gate
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), rot, 3.5, 2.5, pal.primary));
            // High gate
            let high_y = rng.random_range(5.0..8.0);
            meshes.extend(primitives::gate(Point3::new(cx, high_y, cz), rot, 3.5, 2.5, pal.secondary));
            // Support pillars for high gate
            let hw = 2.0;
            let sin_r = rot.to_radians().sin();
            let cos_r = rot.to_radians().cos();
            meshes.push(primitives::pillar(Point3::new(cx - cos_r * hw, 0.0, cz + sin_r * hw), 0.2, high_y, pal.structure));
            meshes.push(primitives::pillar(Point3::new(cx + cos_r * hw, 0.0, cz - sin_r * hw), 0.2, high_y, pal.structure));
        }
        7 => {
            // Cube canyon: two tall cubes with a flyable gap between them
            let h = rng.random_range(6.0..12.0);
            let gap = rng.random_range(3.0..5.0);
            let size = rng.random_range(4.0..6.0);
            meshes.push(primitives::cube(Point3::new(cx - gap / 2.0 - size / 2.0, h / 2.0, cz), size, pal.pick(rng)));
            meshes.push(primitives::cube(Point3::new(cx + gap / 2.0 + size / 2.0, h / 2.0, cz), size, pal.pick(rng)));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 90.0, gap, h - 1.0, pal.accent));
        }
        8 => {
            // Cube bridge: two tall cubes with a cube spanning the top
            let h = rng.random_range(6.0..10.0);
            let gap = rng.random_range(4.0..6.0);
            let pillar_size = rng.random_range(3.0..4.0);
            let color = pal.pick(rng);
            // Two pillars
            meshes.push(primitives::cube(Point3::new(cx - gap / 2.0 - pillar_size / 2.0, h / 2.0, cz), pillar_size, color));
            meshes.push(primitives::cube(Point3::new(cx + gap / 2.0 + pillar_size / 2.0, h / 2.0, cz), pillar_size, color));
            // Bridge beam on top (spans between the two cubes)
            meshes.push(primitives::cube(Point3::new(cx, h + 0.75, cz), gap + pillar_size, pal.secondary));
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), 90.0, gap, h - 0.5, pal.accent));
        }
        9 => {
            // Pillar ring: circle of pillars to weave through
            let count = rng.random_range(6..=8);
            let radius = rng.random_range(5.0..8.0);
            let h = rng.random_range(5.0..10.0);
            for i in 0..count {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let px = cx + angle.cos() * radius;
                let pz = cz + angle.sin() * radius;
                if !in_bounds(px, pz) { continue; }
                meshes.push(primitives::pillar(Point3::new(px, 0.0, pz), 0.4, h, pal.pick(rng)));
            }
            // Gate in the center
            meshes.extend(primitives::gate(Point3::new(cx, 0.0, cz), rng.random_range(0.0..360.0f32), 3.5, 3.0, pal.accent));
        }
        10 => {
            // Pyramid: stack of 3 decreasing cubes
            meshes.push(primitives::cube(Point3::new(cx, 2.5, cz), 5.0, pal.structure));
            meshes.push(primitives::cube(Point3::new(cx, 6.5, cz), 3.5, pal.dark));
            meshes.push(primitives::cube(Point3::new(cx, 9.5, cz), 2.0, pal.primary));
        }
        11 => {
            // Elevated platform: cube on 4 pillar legs, fly underneath
            let h = rng.random_range(4.0..7.0);
            let platform_size = rng.random_range(5.0..8.0);
            let half = platform_size / 2.0 - 0.5;
            let color = pal.pick(rng);
            // Four legs
            meshes.push(primitives::pillar(Point3::new(cx - half, 0.0, cz - half), 0.3, h, color));
            meshes.push(primitives::pillar(Point3::new(cx + half, 0.0, cz - half), 0.3, h, color));
            meshes.push(primitives::pillar(Point3::new(cx - half, 0.0, cz + half), 0.3, h, color));
            meshes.push(primitives::pillar(Point3::new(cx + half, 0.0, cz + half), 0.3, h, color));
            // Platform on top
            meshes.push(primitives::cube(Point3::new(cx, h + 1.0, cz), platform_size, pal.secondary));
        }
        _ => {
            // Pillar slalom: tight alternating left-right pillars
            let count = rng.random_range(4..=6);
            let spacing = rng.random_range(4.0..6.0);
            let offset = rng.random_range(2.0..3.5);
            let h = rng.random_range(5.0..10.0);
            for i in 0..count {
                let pz = cz + (i as f32 - count as f32 / 2.0) * spacing;
                let px = cx + if i % 2 == 0 { -offset } else { offset };
                if !in_bounds(px, pz) { continue; }
                meshes.push(primitives::pillar(Point3::new(px, 0.0, pz), 0.5, h, pal.pick(rng)));
            }
        }
    }
}

// ─── Scattered objects ───

fn generate_scattered(rng: &mut impl Rng, pal: &Palette, x: f32, z: f32, meshes: &mut Vec<Mesh>) {
    match rng.random_range(0..7) {
        0 => {
            // Tall pillar: power loop target
            let h = rng.random_range(5.0..14.0);
            meshes.push(primitives::pillar(Point3::new(x, 0.0, z), rng.random_range(0.3..0.8), h, pal.pick(rng)));
        }
        1 => {
            // Cube obstacle
            let size = rng.random_range(2.0..5.0);
            meshes.push(primitives::cube(Point3::new(x, size / 2.0, z), size, pal.pick(rng)));
        }
        2 => {
            // Standalone gate
            let rot = rng.random_range(0.0..360.0f32);
            meshes.extend(primitives::gate(Point3::new(x, 0.0, z), rot, rng.random_range(2.5..4.0), rng.random_range(2.0..3.5), pal.pick(rng)));
        }
        3 => {
            // Elevated gate: dive target
            let h = rng.random_range(4.0..8.0);
            let rot = rng.random_range(0.0..360.0f32);
            meshes.extend(primitives::gate(Point3::new(x, h, z), rot, 3.0, 2.5, pal.accent));
            let hw = 1.8;
            let sin_r = rot.to_radians().sin();
            let cos_r = rot.to_radians().cos();
            meshes.push(primitives::pillar(Point3::new(x - cos_r * hw, 0.0, z + sin_r * hw), 0.15, h, pal.structure));
            meshes.push(primitives::pillar(Point3::new(x + cos_r * hw, 0.0, z - sin_r * hw), 0.15, h, pal.structure));
        }
        5 => {
            // Split cube: two cubes with a vertical gap + gate
            let h = rng.random_range(4.0..8.0);
            let gap = rng.random_range(3.0..4.5);
            let size = rng.random_range(3.0..5.0);
            meshes.push(primitives::cube(Point3::new(x - gap / 2.0 - size / 2.0, h / 2.0, z), size, pal.pick(rng)));
            meshes.push(primitives::cube(Point3::new(x + gap / 2.0 + size / 2.0, h / 2.0, z), size, pal.pick(rng)));
            meshes.extend(primitives::gate(Point3::new(x, 0.0, z), 90.0, gap, h - 0.5, pal.accent));
        }
        _ => {
            // Archway: single power loop target
            let h = rng.random_range(6.0..10.0);
            let rot = rng.random_range(0.0..360.0f32);
            let sin_r = rot.to_radians().sin();
            let cos_r = rot.to_radians().cos();
            let color = pal.pick(rng);
            meshes.push(primitives::pillar(Point3::new(x - sin_r * 2.5, 0.0, z + cos_r * 2.5), 0.4, h, color));
            meshes.push(primitives::pillar(Point3::new(x + sin_r * 2.5, 0.0, z - cos_r * 2.5), 0.4, h, color));
            // Beam spans between pillars
            meshes.push(primitives::wall(Point3::new(x, h - 0.5, z), 90.0 - rot, 5.5, 1.0, 0.4, color));
        }
    }
}
