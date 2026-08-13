//! Camera module for 3D scene viewing.
//!
//! Provides camera abstractions for creating view and projection matrices,
//! with support for both static cameras and simple orbit controls.

use cgmath::{Deg, InnerSpace, Matrix4, Point3, Rad, SquareMatrix, Transform as _, Vector3};

/// Default camera configuration values.
pub mod defaults {
    use cgmath::{Point3, Vector3};

    /// Default field of view in degrees.
    pub const FOV: f32 = 45.0;
    /// Default near clipping plane distance.
    pub const NEAR: f32 = 0.1;
    /// Default far clipping plane distance.
    pub const FAR: f32 = 100.0;
    /// Default camera position (looking at origin from z=5).
    pub fn position() -> Point3<f32> {
        Point3::new(0.0, 0.0, 5.0)
    }
    /// Default camera target (origin).
    pub fn target() -> Point3<f32> {
        Point3::new(0.0, 0.0, 0.0)
    }
    /// Default up vector (Y-axis).
    pub fn up() -> Vector3<f32> {
        Vector3::new(0.0, 1.0, 0.0)
    }
}

/// A 3D camera that defines a view into the scene.
///
/// The camera uses a standard 3D coordinate system:
/// - Position: where the camera is located
/// - Target: what the camera is looking at
/// - Up: which direction is "up"
///
/// # Example
///
/// ```ignore
/// use renderlib::camera::{Camera, CameraUniform};
///
/// let mut camera = Camera::new();
/// // Or with custom parameters:
/// let camera = Camera::look_at(
///     Point3::new(0.0, 2.0, 5.0),
///     Point3::new(0.0, 0.0, 0.0),
///     Vector3::new(0.0, 1.0, 0.0),
/// );
///
/// let view = camera.get_view_matrix();
/// let proj = camera.get_projection_matrix(aspect_ratio);
/// let mvp = camera.get_view_projection_matrix(aspect_ratio);
/// ```
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space.
    pub position: Point3<f32>,
    /// Point the camera is looking at.
    pub target: Point3<f32>,
    /// Up vector defining the camera's orientation.
    pub up: Vector3<f32>,
    /// Vertical field of view in degrees.
    pub fov: f32,
    /// Near clipping plane distance.
    pub near: f32,
    /// Far clipping plane distance.
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: defaults::position(),
            target: defaults::target(),
            up: defaults::up(),
            fov: defaults::FOV,
            near: defaults::NEAR,
            far: defaults::FAR,
        }
    }
}

impl Camera {
    /// Create a new camera with default parameters.
    ///
    /// The camera is positioned at (0, 0, 5) looking at the origin (0, 0, 0).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a camera looking at a specific target from a position.
    ///
    /// # Arguments
    ///
    /// * `position` - The camera's position in world space
    /// * `target` - The point the camera is looking at
    /// * `up` - The up vector
    pub fn look_at(position: Point3<f32>, target: Point3<f32>, up: Vector3<f32>) -> Self {
        Self {
            position,
            target,
            up,
            fov: defaults::FOV,
            near: defaults::NEAR,
            far: defaults::FAR,
        }
    }

    /// Create a camera with full customization.
    ///
    /// # Arguments
    ///
    /// * `position` - The camera's position in world space
    /// * `target` - The point the camera is looking at
    /// * `up` - The up vector
    /// * `fov` - Vertical field of view in degrees
    /// * `near` - Near clipping plane distance
    /// * `far` - Far clipping plane distance
    pub fn with_params(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: f32,
        near: f32,
        far: f32,
    ) -> Self {
        Self {
            position,
            target,
            up,
            fov,
            near,
            far,
        }
    }

    /// Get the view matrix for this camera.
    ///
    /// The view matrix transforms world space coordinates to view (camera) space.
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get the perspective projection matrix for this camera.
    ///
    /// # Arguments
    ///
    /// * `aspect_ratio` - The width divided by height of the viewport
    pub fn get_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        cgmath::perspective::<f32, Deg<f32>>(Deg(self.fov), aspect_ratio, self.near, self.far)
    }

    /// Get the combined view-projection matrix.
    ///
    /// This is the most commonly used matrix for rendering.
    /// MVP = projection * view
    ///
    /// # Arguments
    ///
    /// * `aspect_ratio` - The width divided by height of the viewport
    pub fn get_view_projection_matrix(&self, aspect_ratio: f32) -> Matrix4<f32> {
        self.get_projection_matrix(aspect_ratio) * self.get_view_matrix()
    }

    /// Get the camera's position as a cgmath Point3.
    pub fn get_position(&self) -> Point3<f32> {
        self.position
    }

    /// Get the camera's target as a cgmath Point3.
    pub fn get_target(&self) -> Point3<f32> {
        self.target
    }

    /// Get the camera's forward direction (normalized).
    pub fn get_forward(&self) -> Vector3<f32> {
        (self.target - self.position).normalize()
    }

    /// Get the camera's right direction (normalized).
    pub fn get_right(&self) -> Vector3<f32> {
        self.get_forward().cross(self.up).normalize()
    }

    /// Set the camera's position.
    pub fn set_position(&mut self, position: Point3<f32>) -> &mut Self {
        self.position = position;
        self
    }

    /// Set the camera's target.
    pub fn set_target(&mut self, target: Point3<f32>) -> &mut Self {
        self.target = target;
        self
    }

    /// Set the camera's up vector.
    pub fn set_up(&mut self, up: Vector3<f32>) -> &mut Self {
        self.up = up;
        self
    }

    /// Set the field of view in degrees.
    pub fn set_fov(&mut self, fov: f32) -> &mut Self {
        self.fov = fov;
        self
    }

    /// Set the near clipping plane.
    pub fn set_near(&mut self, near: f32) -> &mut Self {
        self.near = near;
        self
    }

    /// Set the far clipping plane.
    pub fn set_far(&mut self, far: f32) -> &mut Self {
        self.far = far;
        self
    }

    /// Move the camera position by the given delta.
    pub fn translate(&mut self, delta: Vector3<f32>) -> &mut Self {
        self.position += delta;
        self.target += delta;
        self
    }

    /// Orbit the camera around the target.
    ///
    /// # Arguments
    ///
    /// * `yaw` - Rotation around the Y axis in radians
    /// * `pitch` - Rotation around the X axis in radians
    pub fn orbit(&mut self, yaw: f32, pitch: f32) -> &mut Self {
        let forward = self.get_forward();
        let right = self.get_right();

        // Rotate around Y axis (yaw)
        let rotation_y = Matrix4::from_angle_y(Rad(yaw));
        let forward_rotated = rotation_y.transform_vector(forward);

        // Rotate around right axis (pitch) - using right as the axis
        let rotation_pitch = Matrix4::from_axis_angle(right.normalize(), Rad(pitch));
        let forward_final = rotation_pitch.transform_vector(forward_rotated);

        // Calculate new position based on distance from target
        let distance = (self.position - self.target).magnitude();
        self.position = self.target + forward_final * distance;

        self
    }
}

/// Uniform data structure for passing camera matrices to shaders.
///
/// This matches the std140 layout requirements for uniform buffers.
/// The matrices are stored as 4x4 column-major arrays.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// View matrix (world to view space).
    pub view: [[f32; 4]; 4],
    /// Projection matrix.
    pub projection: [[f32; 4]; 4],
    /// Combined view-projection matrix.
    pub view_projection: [[f32; 4]; 4],
    /// View position (camera position in world space).
    pub view_position: [f32; 4],
}

impl CameraUniform {
    /// Create camera uniform data from a camera and aspect ratio.
    pub fn from_camera(camera: &Camera, aspect_ratio: f32) -> Self {
        let view = camera.get_view_matrix();
        let proj = camera.get_projection_matrix(aspect_ratio);
        let view_proj = proj * view;

        Self {
            view: view.into(),
            projection: proj.into(),
            view_projection: view_proj.into(),
            view_position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
        }
    }

    /// Create camera uniform with identity matrices.
    pub fn identity() -> Self {
        Self {
            view: Matrix4::<f32>::identity().into(),
            projection: Matrix4::<f32>::identity().into(),
            view_projection: Matrix4::<f32>::identity().into(),
            view_position: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Helper struct for building model matrices with common transformations.
///
/// This is useful for objects that need to be positioned, rotated, and scaled
/// in the scene.
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Translation component.
    pub translation: Vector3<f32>,
    /// Rotation as Euler angles in radians (x, y, z).
    pub rotation: Vector3<f32>,
    /// Scale component.
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    /// Create a new transform with default values (identity).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a transform with translation only.
    pub fn with_translation(translation: Vector3<f32>) -> Self {
        Self {
            translation,
            ..Self::default()
        }
    }

    /// Create a transform with rotation only.
    pub fn with_rotation(rotation: Vector3<f32>) -> Self {
        Self {
            rotation,
            ..Self::default()
        }
    }

    /// Create a transform with scale only.
    pub fn with_scale(scale: Vector3<f32>) -> Self {
        Self {
            scale,
            ..Self::default()
        }
    }

    /// Create a transform with all components.
    pub fn with_all(
        translation: Vector3<f32>,
        rotation: Vector3<f32>,
        scale: Vector3<f32>,
    ) -> Self {
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Get the model matrix for this transform.
    ///
    /// The matrix is calculated as: model = translation * rotation * scale
    /// This means transformations are applied in the order: scale, then rotate, then translate.
    pub fn get_model_matrix(&self) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(self.translation);
        let rotation_matrix = Matrix4::from_angle_z(Rad(self.rotation.z))
            * Matrix4::from_angle_y(Rad(self.rotation.y))
            * Matrix4::from_angle_x(Rad(self.rotation.x));
        let scale_matrix = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);

        // Note: In column-major matrices (cgmath), transformations are applied right-to-left
        // So we multiply as T * R * S to get the order: first scale, then rotate, then translate
        translation_matrix * rotation_matrix * scale_matrix
    }

    /// Apply rotation based on elapsed time.
    ///
    /// This is a convenience method for creating animated rotations.
    ///
    /// # Arguments
    ///
    /// * `elapsed` - Time elapsed in seconds
    /// * `speeds` - Rotation speeds in radians per second for (x, y, z)
    pub fn with_time_based_rotation(&self, elapsed: f32, speeds: Vector3<f32>) -> Self {
        Self {
            rotation: Vector3::new(
                self.rotation.x + speeds.x * elapsed,
                self.rotation.y + speeds.y * elapsed,
                self.rotation.z + speeds.z * elapsed,
            ),
            ..*self
        }
    }

    /// Set translation.
    pub fn set_translation(&mut self, translation: Vector3<f32>) -> &mut Self {
        self.translation = translation;
        self
    }

    /// Set rotation.
    pub fn set_rotation(&mut self, rotation: Vector3<f32>) -> &mut Self {
        self.rotation = rotation;
        self
    }

    /// Set scale.
    pub fn set_scale(&mut self, scale: Vector3<f32>) -> &mut Self {
        self.scale = scale;
        self
    }
}

/// Combined camera and model uniform data for shaders that need both.
///
/// This is commonly used in demos where you have a single uniform buffer
/// containing both camera and model transformation data.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraModelUniform {
    /// Model-view-projection matrix.
    pub mvp: [[f32; 4]; 4],
    /// Model matrix.
    pub model: [[f32; 4]; 4],
    /// Light position (for lighting calculations).
    pub light_pos: [f32; 3],
    pub _padding: f32,
}

impl CameraModelUniform {
    /// Create from camera, model matrix, and light position.
    pub fn new(
        camera: &Camera,
        model_matrix: Matrix4<f32>,
        light_pos: [f32; 3],
        aspect_ratio: f32,
    ) -> Self {
        let view_proj = camera.get_view_projection_matrix(aspect_ratio);
        let mvp = view_proj * model_matrix;

        Self {
            mvp: mvp.into(),
            model: model_matrix.into(),
            light_pos,
            _padding: 0.0,
        }
    }

    /// Create with identity matrices.
    pub fn identity() -> Self {
        Self {
            mvp: Matrix4::<f32>::identity().into(),
            model: Matrix4::<f32>::identity().into(),
            light_pos: [0.0, 0.0, 0.0],
            _padding: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_default() {
        let camera = Camera::new();
        assert_eq!(camera.position, defaults::position());
        assert_eq!(camera.target, defaults::target());
        assert_eq!(camera.up, defaults::up());
    }

    #[test]
    fn test_camera_view_matrix() {
        let camera = Camera::look_at(
            Point3::new(0.0, 0.0, 5.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let view = camera.get_view_matrix();
        // The view matrix should not be identity
        assert_ne!(view, Matrix4::<f32>::identity());
    }

    #[test]
    fn test_camera_projection_matrix() {
        let camera = Camera::new();
        let proj = camera.get_projection_matrix(16.0 / 9.0);
        // The projection matrix should not be identity
        assert_ne!(proj, Matrix4::<f32>::identity());
    }

    #[test]
    fn test_transform_model_matrix() {
        let transform = Transform::new();
        let model = transform.get_model_matrix();
        assert_eq!(model, Matrix4::<f32>::identity());

        let transform = Transform::with_translation(Vector3::new(1.0, 2.0, 3.0));
        let model = transform.get_model_matrix();
        assert_ne!(model, Matrix4::<f32>::identity());
    }

    #[test]
    fn test_camera_uniform() {
        let camera = Camera::new();
        let uniform = CameraUniform::from_camera(&camera, 16.0 / 9.0);
        // View position should match camera position
        assert_eq!(uniform.view_position[0], camera.position.x);
        assert_eq!(uniform.view_position[1], camera.position.y);
        assert_eq!(uniform.view_position[2], camera.position.z);
    }
}
