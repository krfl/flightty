use nalgebra::{Matrix4, Vector3, Vector4};

use crate::render::framebuffer::{Color, Framebuffer};
use crate::world::mesh::Mesh;

/// Sun direction for flat shading (normalized, pointing toward light).
const SUN_DIR: Vector3<f32> = Vector3::new(0.3, 0.8, 0.5);
const NEAR_W: f32 = 0.01;

/// Fog parameters. Blends to horizon color based on linear eye-space distance.
pub const FOG_COLOR: Color = Color::new(80, 100, 140);
const FOG_START_DIST: f32 = 60.0;  // meters; no fog closer than this
const FOG_END_DIST: f32 = 150.0;   // meters; fully fogged beyond this

/// Convert NDC z to linear eye-space distance (inverse of perspective projection).
/// near=0.05, far=200.0 from camera.rs
#[inline(always)]
fn ndc_to_linear_depth(ndc_z: f32) -> f32 {
    let near = 0.05f32;
    let far = 200.0f32;
    (2.0 * near * far) / (far + near - ndc_z * (far - near))
}

#[inline(always)]
fn apply_fog(color: Color, ndc_z: f32) -> Color {
    let dist = ndc_to_linear_depth(ndc_z);
    if dist <= FOG_START_DIST {
        return color;
    }
    if dist >= FOG_END_DIST {
        return FOG_COLOR;
    }
    let t = (dist - FOG_START_DIST) / (FOG_END_DIST - FOG_START_DIST);
    color.lerp(FOG_COLOR, t)
}

/// Test if a mesh's bounding sphere is visible in the view frustum.
fn is_mesh_visible(mesh: &Mesh, view_proj: &Matrix4<f32>) -> bool {
    let center = mesh.world_bounds_center();
    let radius = mesh.bounds_radius();

    let row0 = view_proj.row(0);
    let row1 = view_proj.row(1);
    let row2 = view_proj.row(2);
    let row3 = view_proj.row(3);

    let planes = [
        row3 + row0,
        row3 - row0,
        row3 + row1,
        row3 - row1,
        row3 + row2,
        row3 - row2,
    ];

    for plane in &planes {
        let dist = plane.x * center.x + plane.y * center.y + plane.z * center.z + plane.w;
        let normal_len = (plane.x * plane.x + plane.y * plane.y + plane.z * plane.z).sqrt();
        if normal_len > 1e-8 && dist / normal_len < -radius {
            return false;
        }
    }

    true
}

/// A clip-space vertex (before perspective divide).
#[derive(Clone, Copy)]
struct ClipVertex {
    clip: Vector4<f32>,
    color: Color,
}

impl ClipVertex {
    /// Linearly interpolate between two clip vertices at the near plane.
    fn lerp(a: &ClipVertex, b: &ClipVertex, t: f32) -> ClipVertex {
        ClipVertex {
            clip: a.clip * (1.0 - t) + b.clip * t,
            color: a.color, // flat shading, color is uniform per triangle
        }
    }
}

/// Clip a triangle against the near plane (w = NEAR_W).
/// Returns 0, 1, or 2 triangles via the callback.
fn clip_near_plane(
    verts: [ClipVertex; 3],
    mut emit: impl FnMut([ClipVertex; 3]),
) {
    // Classify vertices: inside (w >= NEAR_W) or outside
    let inside = [
        verts[0].clip.w >= NEAR_W,
        verts[1].clip.w >= NEAR_W,
        verts[2].clip.w >= NEAR_W,
    ];
    let num_inside = inside.iter().filter(|&&b| b).count();

    match num_inside {
        3 => emit(verts), // all inside
        0 => {}           // all outside
        _ => {
            // Sutherland-Hodgman: collect vertices after clipping
            let mut out: Vec<ClipVertex> = Vec::with_capacity(4);
            for i in 0..3 {
                let j = (i + 1) % 3;
                let vi = &verts[i];
                let vj = &verts[j];

                if inside[i] {
                    out.push(*vi);
                }

                // Edge crosses near plane?
                if inside[i] != inside[j] {
                    let t = (NEAR_W - vi.clip.w) / (vj.clip.w - vi.clip.w);
                    out.push(ClipVertex::lerp(vi, vj, t));
                }
            }

            // Triangulate the clipped polygon (fan from first vertex)
            for k in 1..out.len() - 1 {
                emit([out[0], out[k], out[k + 1]]);
            }
        }
    }
}

/// Project a clip-space vertex to screen space.
#[inline(always)]
fn to_screen(v: &ClipVertex, sw: f32, sh: f32) -> (f32, f32, f32) {
    let inv_w = 1.0 / v.clip.w;
    let sx = (v.clip.x * inv_w * 0.5 + 0.5) * sw;
    let sy = (1.0 - (v.clip.y * inv_w * 0.5 + 0.5)) * sh;
    let sz = v.clip.z * inv_w;
    (sx, sy, sz)
}

/// Rasterize all triangles of a mesh into the framebuffer.
pub fn rasterize_mesh(fb: &mut Framebuffer, mesh: &Mesh, view_proj: &Matrix4<f32>) {
    if !is_mesh_visible(mesh, view_proj) {
        return;
    }

    let sun = SUN_DIR.normalize();
    let model = mesh.transform.to_homogeneous();
    let mvp = view_proj * model;

    let sw = fb.width as f32;
    let sh = fb.height as f32;

    for tri in &mesh.indices {
        let v0 = &mesh.vertices[tri[0] as usize];
        let v1 = &mesh.vertices[tri[1] as usize];
        let v2 = &mesh.vertices[tri[2] as usize];

        // Compute face normal in world space for shading
        let wp0 = mesh.transform * v0.position;
        let wp1 = mesh.transform * v1.position;
        let wp2 = mesh.transform * v2.position;
        let world_normal = (wp1 - wp0).cross(&(wp2 - wp0));
        let normal = if world_normal.norm() > 1e-8 {
            world_normal.normalize()
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };

        // Flat shading: ambient + diffuse (double-sided lighting)
        let diffuse = normal.dot(&sun).abs();
        let shade = 0.25 + 0.75 * diffuse;
        let color = v0.color.scale(shade);

        // Transform to clip space
        let c0 = mvp * v0.position.to_homogeneous();
        let c1 = mvp * v1.position.to_homogeneous();
        let c2 = mvp * v2.position.to_homogeneous();

        let clip_verts = [
            ClipVertex { clip: c0, color },
            ClipVertex { clip: c1, color },
            ClipVertex { clip: c2, color },
        ];

        // Clip against near plane and rasterize resulting triangles
        clip_near_plane(clip_verts, |clipped| {
            let s0 = to_screen(&clipped[0], sw, sh);
            let s1 = to_screen(&clipped[1], sw, sh);
            let s2 = to_screen(&clipped[2], sw, sh);
            rasterize_triangle(fb, s0, s1, s2, color);
        });
    }
}

/// Rasterize a single triangle using incremental edge functions.
#[inline(always)]
fn rasterize_triangle(
    fb: &mut Framebuffer,
    v0: (f32, f32, f32),
    v1: (f32, f32, f32),
    v2: (f32, f32, f32),
    color: Color,
) {
    let min_x = v0.0.min(v1.0).min(v2.0).max(0.0) as u32;
    let max_x = v0.0.max(v1.0).max(v2.0).min(fb.width as f32 - 1.0) as u32;
    let min_y = v0.1.min(v1.1).min(v2.1).max(0.0) as u32;
    let max_y = v0.1.max(v1.1).max(v2.1).min(fb.height as f32 - 1.0) as u32;

    if min_x > max_x || min_y > max_y {
        return;
    }

    let denom = (v1.1 - v2.1) * (v0.0 - v2.0) + (v2.0 - v1.0) * (v0.1 - v2.1);
    if denom.abs() < 1e-8 {
        return;
    }
    let inv_denom = 1.0 / denom;

    let a01 = v1.1 - v2.1;
    let b01 = v2.0 - v1.0;
    let a12 = v2.1 - v0.1;
    let b12 = v0.0 - v2.0;

    let start_px = min_x as f32 + 0.5;
    let start_py = min_y as f32 + 0.5;

    let mut w0_row = (a01 * (start_px - v2.0) + b01 * (start_py - v2.1)) * inv_denom;
    let mut w1_row = (a12 * (start_px - v2.0) + b12 * (start_py - v2.1)) * inv_denom;

    let w0_dx = a01 * inv_denom;
    let w1_dx = a12 * inv_denom;
    let w0_dy = b01 * inv_denom;
    let w1_dy = b12 * inv_denom;

    let width = fb.width;

    for y in min_y..=max_y {
        let mut w0 = w0_row;
        let mut w1 = w1_row;
        let row_offset = y * width;

        for x in min_x..=max_x {
            let w2 = 1.0 - w0 - w1;

            if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                let z = w0 * v0.2 + w1 * v1.2 + w2 * v2.2;
                let idx = (row_offset + x) as usize;
                if z < fb.depth[idx] {
                    fb.depth[idx] = z;
                    fb.color[idx] = apply_fog(color, z);
                }
            }

            w0 += w0_dx;
            w1 += w1_dx;
        }

        w0_row += w0_dy;
        w1_row += w1_dy;
    }
}
