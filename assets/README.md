# GLTF/GLB Assets

Place your GLTF 2.0 (.gltf or .glb) files in this directory.

The `gltf.rs` demo will look for a file named `cube.glb` by default (a simple cube is included).

To use a different file, modify the `GLTF_PATH` constant in `src/bin/gltf.rs`.

The mesh should have POSITION and optionally NORMAL attributes. If NORMAL is missing, default upward normals will be used.

## Public Domain Test Models

You can download standard test models from:
- https://github.com/KhronosGroup/glTF-Sample-Models
- https://sketchfab.com/ (many free models available)

### Duck.glb
A common test model is the Duck from the glTF-Sample-Models repository:
- Direct download: https://github.com/KhronosGroup/glTF-Sample-Models/raw/master/2.0/Duck/glTF-Embedded/Duck.glb

### Cube.glb
You can also create a simple cube.glb or use any other GLTF/GLB file.
