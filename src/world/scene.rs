use nalgebra::{Point3, UnitQuaternion};

use crate::render::framebuffer::Color;
use crate::world::mesh::{Aabb, Mesh};
use crate::world::primitives;

pub struct Scene {
    pub meshes: Vec<Mesh>,
    /// Precomputed AABBs for collidable meshes (index matches colliders vec).
    pub colliders: Vec<Aabb>,
    pub spawn_position: Point3<f32>,
    pub spawn_orientation: UnitQuaternion<f32>,
}

/// Freestyle FPV playground with structures to rip, gap, dive, and power loop.
pub fn test_course() -> Scene {
    let mut meshes = Vec::new();

    // Ground
    meshes.push(primitives::ground_plane(200.0, 50));

    // Colors
    let orange = Color::new(255, 100, 30);
    let blue = Color::new(30, 150, 255);
    let green = Color::new(30, 220, 80);
    let purple = Color::new(180, 50, 220);
    let red = Color::new(220, 40, 40);
    let yellow = Color::new(255, 220, 30);
    let cyan = Color::new(30, 220, 220);
    let white = Color::new(200, 200, 200);
    let gray = Color::new(120, 120, 120);
    let dark_gray = Color::new(70, 70, 70);

    // === POWER LOOP PILLARS (right in front of spawn) ===
    // Three pillars in a row; power loop around them
    meshes.push(primitives::pillar(Point3::new(-4.0, 0.0, -12.0), 0.5, 8.0, red));
    meshes.push(primitives::pillar(Point3::new( 0.0, 0.0, -12.0), 0.5, 10.0, orange));
    meshes.push(primitives::pillar(Point3::new( 4.0, 0.0, -12.0), 0.5, 8.0, red));

    // === DIVE GAP: wall on pillars with a gap at the bottom ===
    meshes.push(primitives::wall(Point3::new(0.0, 3.0, -24.0), 0.0, 12.0, 6.0, 0.3, blue));
    // Support pillars for the wall
    meshes.push(primitives::pillar(Point3::new(-5.5, 0.0, -24.0), 0.3, 3.0, blue));
    meshes.push(primitives::pillar(Point3::new( 5.5, 0.0, -24.0), 0.3, 3.0, blue));
    // Gate at the bottom to fly through
    meshes.extend(primitives::gate(Point3::new(0.0, 0.0, -24.0), 0.0, 3.0, 2.5, cyan));

    // === FREESTYLE BUILDING: stack of cubes to orbit and thread ===
    // L-shaped building
    meshes.push(primitives::cube(Point3::new(-15.0, 2.5, -20.0), 5.0, gray));
    meshes.push(primitives::cube(Point3::new(-15.0, 7.5, -20.0), 5.0, dark_gray));
    meshes.push(primitives::cube(Point3::new(-10.0, 2.5, -20.0), 5.0, gray));
    // Gap between building sections, fly through!
    meshes.push(primitives::cube(Point3::new(-15.0, 2.5, -25.0), 5.0, dark_gray));
    meshes.extend(primitives::gate(Point3::new(-12.5, 0.0, -22.5), 90.0, 3.0, 4.0, green));

    // === RAMP SECTION: matty flips and proximity ===
    meshes.push(primitives::ramp(Point3::new(15.0, 0.0, -15.0), 0.0, 6.0, 8.0, 4.0, yellow));
    meshes.push(primitives::ramp(Point3::new(15.0, 0.0, -30.0), 180.0, 6.0, 8.0, 4.0, yellow));
    // Gate between the ramps
    meshes.extend(primitives::gate(Point3::new(15.0, 0.0, -22.5), 0.0, 3.0, 3.0, orange));

    // === SPLIT-S TOWERS: twin towers with a gap ===
    meshes.push(primitives::pillar(Point3::new(-8.0, 0.0, -40.0), 0.8, 12.0, purple));
    meshes.push(primitives::pillar(Point3::new( 8.0, 0.0, -40.0), 0.8, 12.0, purple));
    // Bridge between towers, fly under or over
    meshes.push(primitives::wall(Point3::new(0.0, 8.0, -40.0), 0.0, 16.0, 1.5, 0.5, purple));
    // Gate under the bridge
    meshes.extend(primitives::gate(Point3::new(0.0, 0.0, -40.0), 0.0, 4.0, 3.0, green));

    // === SLALOM PILLARS: weave through at speed ===
    for i in 0..6 {
        let z = -50.0 - i as f32 * 6.0;
        let x = if i % 2 == 0 { -3.0 } else { 3.0 };
        let h = 5.0 + (i as f32 * 0.5);
        let color = if i % 2 == 0 { orange } else { blue };
        meshes.push(primitives::pillar(Point3::new(x, 0.0, z), 0.4, h, color));
    }
    // Gate at the end of the slalom
    meshes.extend(primitives::gate(Point3::new(0.0, 0.0, -86.0), 0.0, 3.0, 2.5, yellow));

    // === BANDO (abandoned building): orbit and explore ===
    let bx = 25.0;
    let bz = -45.0;
    // Walls with window gaps
    meshes.push(primitives::wall(Point3::new(bx - 5.0, 0.0, bz), 90.0, 12.0, 7.0, 0.3, dark_gray));
    meshes.push(primitives::wall(Point3::new(bx + 5.0, 0.0, bz), 90.0, 12.0, 7.0, 0.3, dark_gray));
    meshes.push(primitives::wall(Point3::new(bx, 0.0, bz - 6.0), 0.0, 10.0, 7.0, 0.3, gray));
    // Windows (gates in the walls)
    meshes.extend(primitives::gate(Point3::new(bx, 0.0, bz + 6.0), 0.0, 3.0, 3.5, red));
    meshes.extend(primitives::gate(Point3::new(bx - 5.0, 0.0, bz), 90.0, 3.0, 3.5, red));
    // Roof beams
    meshes.push(primitives::wall(Point3::new(bx, 6.5, bz), 0.0, 10.5, 0.5, 12.5, dark_gray));

    // === SCATTERED FREESTYLE OBJECTS ===
    // Random cubes and pillars around the field for proximity ripping
    meshes.push(primitives::cube(Point3::new(10.0, 1.5, 8.0), 3.0, Color::new(180, 80, 30)));
    meshes.push(primitives::cube(Point3::new(-10.0, 1.0, 5.0), 2.0, Color::new(30, 100, 180)));
    meshes.push(primitives::pillar(Point3::new(20.0, 0.0, 5.0), 0.6, 6.0, green));
    meshes.push(primitives::pillar(Point3::new(-20.0, 0.0, -10.0), 0.7, 9.0, cyan));
    meshes.push(primitives::cube(Point3::new(-25.0, 3.0, -35.0), 6.0, Color::new(90, 90, 90)));

    // High-altitude gate on pillars that sends you up
    meshes.push(primitives::pillar(Point3::new(-2.0, 0.0, -6.0), 0.3, 8.0, white));
    meshes.push(primitives::pillar(Point3::new( 2.0, 0.0, -6.0), 0.3, 8.0, white));
    meshes.extend(primitives::gate(Point3::new(0.0, 8.0, -6.0), 0.0, 3.0, 2.0, white));

    // Low-altitude speed gate right off the start
    meshes.extend(primitives::gate(Point3::new(0.0, 0.0, -4.0), 0.0, 4.0, 1.5, yellow));

    // Precompute collision AABBs
    let colliders: Vec<Aabb> = meshes
        .iter()
        .filter(|m| m.collidable)
        .map(|m| m.world_aabb())
        .collect();

    Scene {
        meshes,
        colliders,
        spawn_position: Point3::new(0.0, 0.05, 0.0),
        spawn_orientation: UnitQuaternion::identity(),
    }
}
