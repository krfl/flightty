use nalgebra::{Isometry3, Point3, Translation3, UnitQuaternion, Vector3};

use crate::render::framebuffer::Color;
use crate::world::mesh::{Mesh, Vertex};

/// Generate a checkerboard ground plane centered at origin.
pub fn ground_plane(size: f32, divisions: u32) -> Mesh {
    let color_a = Color::new(60, 120, 60);  // dark green
    let color_b = Color::new(40, 80, 40);   // darker green
    let step = size / divisions as f32;
    let half = size / 2.0;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for iz in 0..divisions {
        for ix in 0..divisions {
            let x = -half + ix as f32 * step;
            let z = -half + iz as f32 * step;
            let color = if (ix + iz) % 2 == 0 { color_a } else { color_b };

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: Point3::new(x, 0.0, z), color });
            vertices.push(Vertex { position: Point3::new(x + step, 0.0, z), color });
            vertices.push(Vertex { position: Point3::new(x + step, 0.0, z + step), color });
            vertices.push(Vertex { position: Point3::new(x, 0.0, z + step), color });

            // Two triangles per quad (CCW winding when viewed from above, but we'll
            // handle face culling direction in the rasterizer)
            indices.push([base, base + 2, base + 1]);
            indices.push([base, base + 3, base + 2]);
        }
    }

    Mesh::new(vertices, indices).no_collision()
}

/// Generate a colored cube.
pub fn cube(center: Point3<f32>, size: f32, color: Color) -> Mesh {
    let h = size / 2.0;

    // 8 vertices of the cube
    let positions = [
        Point3::new(-h, -h, -h), // 0: left-bottom-back
        Point3::new( h, -h, -h), // 1: right-bottom-back
        Point3::new( h,  h, -h), // 2: right-top-back
        Point3::new(-h,  h, -h), // 3: left-top-back
        Point3::new(-h, -h,  h), // 4: left-bottom-front
        Point3::new( h, -h,  h), // 5: right-bottom-front
        Point3::new( h,  h,  h), // 6: right-top-front
        Point3::new(-h,  h,  h), // 7: left-top-front
    ];

    let vertices: Vec<Vertex> = positions
        .iter()
        .map(|&position| Vertex { position, color })
        .collect();

    // 12 triangles (2 per face), winding order for outward-facing normals
    let indices = vec![
        // Front (+Z)
        [4, 5, 6], [4, 6, 7],
        // Back (-Z)
        [1, 0, 3], [1, 3, 2],
        // Right (+X)
        [5, 1, 2], [5, 2, 6],
        // Left (-X)
        [0, 4, 7], [0, 7, 3],
        // Top (+Y)
        [7, 6, 2], [7, 2, 3],
        // Bottom (-Y)
        [0, 1, 5], [0, 5, 4],
    ];

    let transform = Isometry3::from_parts(
        Translation3::from(center.coords),
        UnitQuaternion::identity(),
    );

    Mesh::new(vertices, indices).with_transform(transform)
}

/// Generate a racing gate as 3 separate meshes (left post, right post, top beam).
/// Each part has its own AABB collider so the opening is flyable.
pub fn gate(position: Point3<f32>, rotation_y_deg: f32, width: f32, height: f32, color: Color) -> Vec<Mesh> {
    let thickness = 0.15;
    let post_width = 0.15;
    let hw = width / 2.0;

    let rotation = UnitQuaternion::from_axis_angle(
        &Vector3::y_axis(),
        rotation_y_deg.to_radians(),
    );
    let transform = Isometry3::from_parts(
        Translation3::from(position.coords),
        rotation,
    );

    vec![
        // Left post
        make_box(-hw, height / 2.0, 0.0, post_width, height, thickness, color)
            .with_transform(transform),
        // Right post
        make_box(hw, height / 2.0, 0.0, post_width, height, thickness, color)
            .with_transform(transform),
        // Top beam
        make_box(0.0, height, 0.0, width + post_width, post_width, thickness, color)
            .with_transform(transform),
    ]
}

/// Helper: create a box mesh at local position with given dimensions.
fn make_box(cx: f32, cy: f32, cz: f32, sx: f32, sy: f32, sz: f32, color: Color) -> Mesh {
    let hx = sx / 2.0;
    let hy = sy / 2.0;
    let hz = sz / 2.0;
    let positions = [
        Point3::new(cx - hx, cy - hy, cz - hz),
        Point3::new(cx + hx, cy - hy, cz - hz),
        Point3::new(cx + hx, cy + hy, cz - hz),
        Point3::new(cx - hx, cy + hy, cz - hz),
        Point3::new(cx - hx, cy - hy, cz + hz),
        Point3::new(cx + hx, cy - hy, cz + hz),
        Point3::new(cx + hx, cy + hy, cz + hz),
        Point3::new(cx - hx, cy + hy, cz + hz),
    ];
    let vertices: Vec<Vertex> = positions.iter().map(|&p| Vertex { position: p, color }).collect();
    let indices = vec![
        [4,5,6],[4,6,7], [1,0,3],[1,3,2], [5,1,2],[5,2,6],
        [0,4,7],[0,7,3], [7,6,2],[7,2,3], [0,1,5],[0,5,4],
    ];
    Mesh::new(vertices, indices)
}

/// Generate a tall pillar (for power loops and dives).
pub fn pillar(position: Point3<f32>, radius: f32, height: f32, color: Color) -> Mesh {
    let segments = 8;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;
        vertices.push(Vertex { position: Point3::new(x, 0.0, z), color });
        vertices.push(Vertex { position: Point3::new(x, height, z), color });
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        let b0 = (i * 2) as u32;
        let t0 = b0 + 1;
        let b1 = (next * 2) as u32;
        let t1 = b1 + 1;
        indices.push([b0, b1, t1]);
        indices.push([b0, t1, t0]);
    }

    let top_center = vertices.len() as u32;
    vertices.push(Vertex { position: Point3::new(0.0, height, 0.0), color });
    for i in 0..segments {
        let next = (i + 1) % segments;
        indices.push([top_center, (i * 2 + 1) as u32, (next * 2 + 1) as u32]);
    }

    let transform = Isometry3::from_parts(
        Translation3::from(position.coords),
        UnitQuaternion::identity(),
    );
    Mesh::new(vertices, indices).with_transform(transform)
}

/// Generate a wall for gaps and splits.
pub fn wall(position: Point3<f32>, rotation_y_deg: f32, width: f32, height: f32, thickness: f32, color: Color) -> Mesh {
    let hx = width / 2.0;
    let hz = thickness / 2.0;
    let positions = [
        Point3::new(-hx, 0.0, -hz), Point3::new( hx, 0.0, -hz),
        Point3::new( hx, height, -hz), Point3::new(-hx, height, -hz),
        Point3::new(-hx, 0.0,  hz), Point3::new( hx, 0.0,  hz),
        Point3::new( hx, height,  hz), Point3::new(-hx, height,  hz),
    ];
    let vertices: Vec<Vertex> = positions.iter().map(|&position| Vertex { position, color }).collect();
    let indices = vec![
        [4,5,6],[4,6,7], [1,0,3],[1,3,2], [5,1,2],[5,2,6],
        [0,4,7],[0,7,3], [7,6,2],[7,2,3], [0,1,5],[0,5,4],
    ];
    let transform = Isometry3::from_parts(
        Translation3::from(position.coords),
        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), rotation_y_deg.to_radians()),
    );
    Mesh::new(vertices, indices).with_transform(transform)
}

/// Generate a ramp for proximity flying (no collision, since AABB can't represent slopes).
pub fn ramp(position: Point3<f32>, rotation_y_deg: f32, width: f32, length: f32, height: f32, color: Color) -> Mesh {
    let hw = width / 2.0;
    let hl = length / 2.0;
    let vertices = vec![
        Vertex { position: Point3::new(-hw, 0.0, -hl), color },
        Vertex { position: Point3::new( hw, 0.0, -hl), color },
        Vertex { position: Point3::new( hw, 0.0,  hl), color },
        Vertex { position: Point3::new(-hw, 0.0,  hl), color },
        Vertex { position: Point3::new(-hw, height, -hl), color },
        Vertex { position: Point3::new( hw, height, -hl), color },
    ];
    let indices = vec![
        [3,2,5],[3,5,4], [0,1,5],[0,5,4],
        [0,3,2],[0,2,1], [0,4,3], [1,2,5],
    ];
    let transform = Isometry3::from_parts(
        Translation3::from(position.coords),
        UnitQuaternion::from_axis_angle(&Vector3::y_axis(), rotation_y_deg.to_radians()),
    );
    Mesh::new(vertices, indices).with_transform(transform).no_collision()
}
