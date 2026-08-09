//! Primitive mesh generators.
//!
//! This module provides pre-defined mesh data for common geometric primitives
//! like triangles and cubes, ready to use with the framework's vertex types.

use super::{PosColorNormalVertex, PosColorVertex};

/// Returns vertices for a simple triangle with red, green, blue corners.
/// 
/// The triangle is centered at the origin with one vertex at the top
/// and two at the bottom, forming an upright triangle.
pub fn triangle_vertices() -> &'static [PosColorVertex] {
    &[
        PosColorVertex {
            position: [0.0, 0.5, 0.0],
            color: [1.0, 0.0, 0.0],
        },
        PosColorVertex {
            position: [-0.5, -0.5, 0.0],
            color: [0.0, 1.0, 0.0],
        },
        PosColorVertex {
            position: [0.5, -0.5, 0.0],
            color: [0.0, 0.0, 1.0],
        },
    ]
}

/// Returns vertices and indices for a cube centered at the origin with side length 2.
/// 
/// Each face has a different color:
/// - Front: Red
/// - Back: Green
/// - Right: Blue
/// - Left: Yellow
/// - Top: Magenta
/// - Bottom: Cyan
/// 
/// The cube is oriented with faces aligned to the coordinate axes.
/// Normals point outward from each face.
pub fn cube_vertices() -> (Vec<PosColorNormalVertex>, Vec<u16>) {
    // Define the 8 corner positions
    let positions = [
        // Front face (z = 1)
        [-1.0, -1.0, 1.0], // 0
        [1.0, -1.0, 1.0],  // 1
        [1.0, 1.0, 1.0],   // 2
        [-1.0, 1.0, 1.0],  // 3
        // Back face (z = -1)
        [-1.0, -1.0, -1.0], // 4
        [1.0, -1.0, -1.0],  // 5
        [1.0, 1.0, -1.0],   // 6
        [-1.0, 1.0, -1.0],  // 7
    ];

    // Define face normals (pointing outward)
    let face_normals = [
        [0.0, 0.0, 1.0],  // Front face - Z+
        [0.0, 0.0, -1.0], // Back face - Z-
        [1.0, 0.0, 0.0],  // Right face - X+
        [-1.0, 0.0, 0.0], // Left face - X-
        [0.0, 1.0, 0.0],  // Top face - Y+
        [0.0, -1.0, 0.0], // Bottom face - Y-
    ];

    // Define colors for each face
    let face_colors = [
        [1.0, 0.0, 0.0], // Front - Red
        [0.0, 1.0, 0.0], // Back - Green
        [0.0, 0.0, 1.0], // Right - Blue
        [1.0, 1.0, 0.0], // Left - Yellow
        [1.0, 0.0, 1.0], // Top - Magenta
        [0.0, 1.0, 1.0], // Bottom - Cyan
    ];

    // Build vertices with colors and normals assigned per face
    let mut vertices = Vec::new();

    // Front face (z = 1)
    for &pos_idx in &[0, 1, 2, 3] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[0],
            normal: face_normals[0],
        });
    }
    // Back face (z = -1) - note: vertex order reversed for correct normal orientation
    for &pos_idx in &[5, 4, 7, 6] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[1],
            normal: face_normals[1],
        });
    }
    // Right face (x = 1)
    for &pos_idx in &[1, 5, 6, 2] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[2],
            normal: face_normals[2],
        });
    }
    // Left face (x = -1) - note: vertex order reversed
    for &pos_idx in &[4, 0, 3, 7] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[3],
            normal: face_normals[3],
        });
    }
    // Top face (y = 1)
    for &pos_idx in &[3, 2, 6, 7] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[4],
            normal: face_normals[4],
        });
    }
    // Bottom face (y = -1) - note: vertex order reversed
    for &pos_idx in &[0, 4, 5, 1] {
        vertices.push(PosColorNormalVertex {
            position: positions[pos_idx],
            color: face_colors[5],
            normal: face_normals[5],
        });
    }

    // Indices for each face (2 triangles per face, 6 faces = 36 indices)
    let mut indices = Vec::new();
    for face_offset in 0..6 {
        let base = face_offset * 4;
        // First triangle
        indices.push(base as u16);
        indices.push((base + 1) as u16);
        indices.push((base + 2) as u16);
        // Second triangle
        indices.push(base as u16);
        indices.push((base + 2) as u16);
        indices.push((base + 3) as u16);
    }

    (vertices, indices)
}
