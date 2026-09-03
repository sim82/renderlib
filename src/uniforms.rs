//! Uniform buffer types and management.
//!
//! This module provides common uniform buffer structs and utilities for
//! instanced rendering and other GPU operations.

use cgmath::Matrix4;

/// Camera uniform data for instanced rendering.
///
/// Contains the view-projection matrix used by vertex shaders to transform
/// world-space positions to clip space.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View-projection matrix as a 4x4 column-major matrix
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Creates a new camera uniform with the given view-projection matrix.
    ///
    /// # Arguments
    ///
    /// * `view_proj` - The view-projection matrix to use for transformations
    pub fn new(view_proj: Matrix4<f32>) -> Self {
        Self {
            view_proj: view_proj.into(),
        }
    }
}

/// Instance uniform data for instanced rendering.
///
/// Contains the model matrix for transforming instance-local positions to world space.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceUniform {
    /// Model matrix as a 4x4 column-major matrix
    pub model: [[f32; 4]; 4],
}

impl InstanceUniform {
    /// Creates a new instance uniform with the given model matrix.
    ///
    /// # Arguments
    ///
    /// * `model` - The model matrix for this instance
    pub fn new(model: Matrix4<f32>) -> Self {
        Self {
            model: model.into(),
        }
    }
}
