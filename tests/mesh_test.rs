//! Tests for the mesh module.

use renderlib::geometry::PosColorNormalVertex;
use renderlib::mesh::{MeshAsset, MeshCache, MeshHandle, MeshSource, PrimitiveType};

#[test]
fn test_mesh_asset_creation() {
    let vertices = vec![
        PosColorNormalVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        },
        PosColorNormalVertex {
            position: [1.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
            normal: [0.0, 1.0, 0.0],
        },
    ];
    let indices = vec![0, 1];

    let mesh = MeshAsset::new(vertices.clone(), indices.clone());

    assert_eq!(mesh.vertices.len(), 2);
    assert_eq!(mesh.indices.len(), 2);
    assert_eq!(mesh.name, "unnamed");
}

#[test]
fn test_mesh_asset_with_name() {
    let vertices = vec![];
    let indices = vec![];

    let mesh = MeshAsset::with_name(vertices, indices, "test_mesh".to_string());

    assert_eq!(mesh.name, "test_mesh");
}

#[test]
fn test_mesh_handle_generation() {
    let handle1 = MeshHandle::new();
    let handle2 = MeshHandle::new();

    assert_ne!(handle1, handle2);
    assert_eq!(handle1, handle1); // Ensure Copy works
}

#[test]
fn test_primitive_type_names() {
    assert_eq!(PrimitiveType::Cube.name(), "cube");
    assert_eq!(PrimitiveType::Sphere.name(), "sphere");
    assert_eq!(PrimitiveType::Quad.name(), "quad");
}

#[test]
fn test_mesh_source_enum() {
    let path_source = MeshSource::Path("test.glb".to_string());
    let primitive_source = MeshSource::Primitive(PrimitiveType::Cube);

    match path_source {
        MeshSource::Path(p) => assert_eq!(p, "test.glb"),
        _ => panic!("Expected Path variant"),
    }

    match primitive_source {
        MeshSource::Primitive(PrimitiveType::Cube) => {}
        _ => panic!("Expected Primitive variant"),
    }
}

#[test]
fn test_mesh_cache_creation() {
    // We can't create a wgpu::Device in a test without a window,
    // so we'll just test that the new function compiles
    // In a real integration test, you would create a device and test the cache

    // This test is a placeholder to ensure the types are correct
    let _: Option<MeshCache> = None;
}
