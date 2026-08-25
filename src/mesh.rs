//! Mesh loading and management module.
//!
//! Provides functionality for loading 3D meshes from various formats,
//! particularly GLTF/GLB files, and managing their vertex and index buffers.
//!
//! # Architecture
//!
//! This module uses a decoupled design to separate CPU-side mesh data
//! (`MeshAsset`) from GPU-side resources (`MeshResource`). This allows:
//! - CPU-side operations (e.g., physics, culling) without GPU overhead.
//! - Thread-safe loading (CPU data can be loaded in background threads).
//! - Memory efficiency (avoid redundant CPU data for GPU-only use cases).
//!
//! # Key Types
//!
//! - [`MeshAsset`]: CPU-side mesh data (vertices, indices, metadata).
//! - [`MeshResource`]: GPU-side buffers for rendering.
//! - [`MeshHandle`]: Opaque identifier for cached meshes.
//! - [`MeshCache`]: Central cache for managing mesh assets and resources.

use std::sync::atomic::{AtomicU64, Ordering};

use cgmath::{Vector3, Zero};
use gltf::mesh::util::ReadIndices;
use wgpu::VertexBufferLayout;

use crate::device_helpers::create_buffer_from_slice;
use crate::geometry::PosColorNormalVertex;

/// Bounding box for a mesh, used for calculating scale and center.
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min: Vector3<f32>,
    pub max: Vector3<f32>,
}

impl BoundingBox {
    /// Create a new bounding box from min and max points.
    pub fn new(min: Vector3<f32>, max: Vector3<f32>) -> Self {
        Self { min, max }
    }

    /// Calculate the scale factor to fit the mesh in a unit size.
    pub fn scale_factor(&self) -> f32 {
        let width = self.max.x - self.min.x;
        let height = self.max.y - self.min.y;
        let depth = self.max.z - self.min.z;
        let max_dim = width.max(height).max(depth);

        if max_dim > 0.0 && max_dim.is_finite() {
            2.0 / max_dim
        } else {
            1.0
        }
    }

    /// Calculate the center point of the bounding box.
    pub fn center(&self) -> Vector3<f32> {
        Vector3::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
            (self.min.z + self.max.z) / 2.0,
        )
    }
}

/// Opaque handle to a mesh in the cache.
///
/// This is a lightweight identifier that can be cloned and passed around
/// to reference a mesh without holding onto the actual data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u64);

impl MeshHandle {
    /// Generate a new unique mesh handle.
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// CPU-side representation of a 3D mesh.
///
/// Contains the raw vertex and index data along with metadata like
/// bounding box, scale, and center. This data is independent of any
/// GPU resources and can be used for CPU-side operations like
/// physics, culling, or mesh processing.
///
/// To create GPU resources for rendering, use [`MeshAsset::create_resource`].
#[derive(Debug, Clone)]
pub struct MeshAsset {
    /// The vertices of the mesh.
    pub vertices: Vec<PosColorNormalVertex>,
    /// The indices of the mesh (triangles).
    pub indices: Vec<u16>,
    /// The bounding box of the mesh.
    pub bounding_box: BoundingBox,
    /// Scale factor to normalize the mesh to approximately unit size.
    pub scale: f32,
    /// Center point of the mesh for translation to origin.
    pub center: Vector3<f32>,
    /// Name of the mesh (for debugging and identification).
    pub name: String,
}

/// GPU-side representation of a mesh.
///
/// Contains the vertex and index buffers needed for rendering.
/// This is decoupled from the CPU-side [`MeshAsset`] to allow:
/// - Independent lifecycle management (GPU resources can be recreated without reloading CPU data).
/// - Memory efficiency (multiple instances can share the same GPU buffers).
/// - Thread safety (GPU resources are only accessed from the render thread).
#[derive(Debug)]
pub struct MeshResource {
    /// The vertex buffer for this mesh.
    pub vertex_buffer: wgpu::Buffer,
    /// The index buffer for this mesh.
    pub index_buffer: wgpu::Buffer,
    /// Number of indices in the mesh.
    pub num_indices: u32,
    /// Vertex buffer layout for pipeline compatibility.
    pub vertex_layout: VertexBufferLayout<'static>,
}

impl MeshAsset {
    /// Create a new mesh asset from vertices and indices with a default bounding box.
    pub fn new(vertices: Vec<PosColorNormalVertex>, indices: Vec<u16>) -> Self {
        Self::with_name(vertices, indices, "unnamed".to_string())
    }

    /// Create a new mesh asset with a custom name.
    pub fn with_name(vertices: Vec<PosColorNormalVertex>, indices: Vec<u16>, name: String) -> Self {
        // Calculate bounding box from vertices
        let (bounding_box, scale, center) = if !vertices.is_empty() {
            let mut min_x = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = f32::NEG_INFINITY;
            let mut min_z = f32::INFINITY;
            let mut max_z = f32::NEG_INFINITY;

            for v in &vertices {
                min_x = min_x.min(v.position[0]);
                max_x = max_x.max(v.position[0]);
                min_y = min_y.min(v.position[1]);
                max_y = max_y.max(v.position[1]);
                min_z = min_z.min(v.position[2]);
                max_z = max_z.max(v.position[2]);
            }

            let min = Vector3::new(min_x, min_y, min_z);
            let max = Vector3::new(max_x, max_y, max_z);
            let bounding_box = BoundingBox::new(min, max);
            let scale = bounding_box.scale_factor();
            let center = bounding_box.center();

            (bounding_box, scale, center)
        } else {
            (
                BoundingBox::new(Vector3::zero(), Vector3::zero()),
                1.0,
                Vector3::zero(),
            )
        };

        Self {
            vertices,
            indices,
            bounding_box,
            scale,
            center,
            name,
        }
    }

    /// Get the vertex buffer layout for this mesh asset.
    ///
    /// This is used to configure the render pipeline's vertex input.
    pub fn vertex_layout() -> VertexBufferLayout<'static> {
        PosColorNormalVertex::desc()
    }

    /// Create GPU resources for this mesh asset on the given device.
    ///
    /// # Arguments
    ///
    /// * `device` - The wgpu device to create buffers on
    /// * `label_prefix` - Optional prefix for buffer labels
    ///
    /// # Returns
    ///
    /// A [`MeshResource`] containing the GPU buffers.
    pub fn create_resource(
        &self,
        device: &wgpu::Device,
        label_prefix: Option<&str>,
    ) -> MeshResource {
        let label = label_prefix.unwrap_or(&self.name);

        let vertex_buffer = create_buffer_from_slice(
            device,
            Some(&format!("{} Vertex Buffer", label)),
            &self.vertices,
            wgpu::BufferUsages::VERTEX,
        );

        let index_buffer = create_buffer_from_slice(
            device,
            Some(&format!("{} Index Buffer", label)),
            &self.indices,
            wgpu::BufferUsages::INDEX,
        );

        MeshResource {
            vertex_buffer,
            index_buffer,
            num_indices: self.indices.len() as u32,
            vertex_layout: Self::vertex_layout(),
        }
    }
}

/// Error type for mesh loading operations.
#[derive(Debug)]
pub enum MeshLoadError {
    /// Failed to read the file.
    IoError(std::io::Error),
    /// Failed to import the GLTF/GLB file.
    ImportError(String),
    /// No meshes found in the file.
    NoMeshesFound,
    /// No vertices loaded from the mesh.
    NoVerticesLoaded,
    /// A mesh primitive has no POSITION attribute.
    NoPositionAttribute,
}

impl std::fmt::Display for MeshLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshLoadError::IoError(e) => write!(f, "IO error: {}", e),
            MeshLoadError::ImportError(s) => write!(f, "Import error: {}", s),
            MeshLoadError::NoMeshesFound => write!(f, "No meshes found in GLTF file"),
            MeshLoadError::NoVerticesLoaded => write!(f, "No vertices loaded from GLTF mesh"),
            MeshLoadError::NoPositionAttribute => {
                write!(f, "Mesh primitive has no POSITION attribute")
            }
        }
    }
}

impl std::error::Error for MeshLoadError {}

impl From<std::io::Error> for MeshLoadError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// Load a mesh from a GLTF or GLB file.
///
/// # Arguments
///
/// * `path` - Path to the GLTF (.gltf) or GLB (.glb) file
///
/// # Returns
///
/// A `Mesh` containing the loaded vertices and indices.
pub fn load_gltf(path: &str) -> Result<MeshAsset, MeshLoadError> {
    // Load the GLTF/GLB file
    let (document, buffers, _images) = if path.ends_with(".glb") {
        // For .glb files, read as bytes first
        let data = std::fs::read(path)?;
        gltf::import_slice(&data).map_err(|e| MeshLoadError::ImportError(e.to_string()))?
    } else {
        // For .gltf files, import from path (handles external buffers)
        gltf::import(path).map_err(|e| MeshLoadError::ImportError(e.to_string()))?
    };

    // Get the first mesh
    let mesh = document
        .meshes()
        .next()
        .ok_or(MeshLoadError::NoMeshesFound)?;

    // First pass: collect all positions to calculate bounding box
    let mut all_positions: Vec<[f32; 3]> = Vec::new();
    let mut all_indices: Vec<u16> = Vec::new();
    let mut vertex_offset: u32 = 0;

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &*b.0));

        // Read positions (required)
        let positions: Vec<[f32; 3]> = match reader.read_positions() {
            Some(positions) => positions.collect(),
            None => return Err(MeshLoadError::NoPositionAttribute),
        };

        let num_positions = positions.len();
        all_positions.extend(positions);

        // Read indices
        let primitive_indices: Vec<u16> = match reader.read_indices() {
            Some(indices) => match indices {
                ReadIndices::U8(iter) => iter.map(|x| x as u16).collect(),
                ReadIndices::U16(iter) => iter.collect(),
                ReadIndices::U32(iter) => iter.map(|x| x as u16).collect(),
            },
            None => (0..num_positions as u16).collect(),
        };

        // Adjust indices with vertex offset (for multi-primitive meshes)
        let offset_indices: Vec<u16> = primitive_indices
            .iter()
            .map(|&idx| idx.saturating_add(vertex_offset as u16))
            .collect();

        all_indices.extend(offset_indices);
        vertex_offset += num_positions as u32;
    }

    // Second pass: build vertices with normals and color
    let mut all_vertices: Vec<PosColorNormalVertex> = Vec::new();

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|b| &*b.0));

        // Read positions (required)
        let positions: Vec<[f32; 3]> = match reader.read_positions() {
            Some(positions) => positions.collect(),
            None => return Err(MeshLoadError::NoPositionAttribute),
        };

        // Read normals (optional, default to upward if missing)
        let normals: Vec<[f32; 3]> = match reader.read_normals() {
            Some(normals) => normals.collect(),
            None => vec![[0.0, 1.0, 0.0]; positions.len()],
        };

        // Create vertices with default color (light gray)
        let default_color: [f32; 3] = [0.8, 0.8, 0.8];
        let mut mesh_vertices = Vec::new();

        for (i, position) in positions.iter().enumerate() {
            let normal = normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]);
            mesh_vertices.push(PosColorNormalVertex {
                position: *position,
                color: default_color,
                normal,
            });
        }

        all_vertices.extend(mesh_vertices);
    }

    if all_vertices.is_empty() {
        return Err(MeshLoadError::NoVerticesLoaded);
    }

    // Calculate bounding box, scale, and center from all_positions (single calculation)
    let (bounding_box, scale, center) = if !all_positions.is_empty() {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;

        for &[x, y, z] in &all_positions {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }

        let min = Vector3::new(min_x, min_y, min_z);
        let max = Vector3::new(max_x, max_y, max_z);
        let bounding_box = BoundingBox::new(min, max);
        let scale = bounding_box.scale_factor();
        let center = bounding_box.center();

        (bounding_box, scale, center)
    } else {
        (
            BoundingBox::new(Vector3::zero(), Vector3::zero()),
            1.0,
            Vector3::zero(),
        )
    };

    // Extract the filename from the path for the mesh name
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string();

    Ok(MeshAsset {
        vertices: all_vertices,
        indices: all_indices,
        bounding_box,
        scale,
        center,
        name,
    })
}

/// Vertex with just position for full-screen quad (2D coordinates).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
}

impl QuadVertex {
    /// Returns the vertex buffer layout for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

/// Full-screen quad vertices using 2D positions (NDC coordinates).
pub fn quad_vertices_2d() -> &'static [QuadVertex] {
    &[
        // First triangle
        QuadVertex {
            position: [0.0, 0.0],
        },
        QuadVertex {
            position: [1.0, 0.0],
        },
        QuadVertex {
            position: [0.0, 1.0],
        },
        // Second triangle
        QuadVertex {
            position: [1.0, 0.0],
        },
        QuadVertex {
            position: [1.0, 1.0],
        },
        QuadVertex {
            position: [0.0, 1.0],
        },
    ]
}

/// Creates a full-screen quad vertex buffer.
pub fn create_quad_buffer(device: &wgpu::Device, label: Option<&str>) -> wgpu::Buffer {
    create_buffer_from_slice(
        device,
        label,
        quad_vertices_2d(),
        wgpu::BufferUsages::VERTEX,
    )
}
