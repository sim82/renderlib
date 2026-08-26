//! Application state module - manages mutable application data.
//!
//! This module provides the mutable application state that changes during runtime.
//! Unlike the GPU infrastructure (GraphicsDevice), this represents the "scene"
//! and "resources" layer that can be modified as the application runs.

use crate::camera::Camera;
use crate::mesh::{MeshCache, MeshHandle};

/// Input state for tracking keyboard, mouse, and other input devices.
#[derive(Debug, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub pressed_keys: Vec<winit::keyboard::Key>,
    /// Mouse position
    pub mouse_position: Option<(f64, f64)>,
    /// Mouse buttons pressed
    pub mouse_buttons: Vec<u16>,
    /// Scroll delta
    pub scroll_delta: (f64, f64),
}

impl InputState {
    /// Create a new input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the mouse position.
    pub fn set_mouse_position(&mut self, x: f64, y: f64) {
        self.mouse_position = Some((x, y));
    }

    /// Clear the scroll delta (should be called after processing).
    pub fn clear_scroll(&mut self) {
        self.scroll_delta = (0.0, 0.0);
    }
}

/// Timing information for the application.
#[derive(Debug)]
pub struct TimeState {
    /// Total time since application start (in seconds)
    pub total_time: f64,
    /// Time since last frame (in seconds)
    pub delta_time: f64,
    /// Frame count
    pub frame_count: u64,
    /// Time when the application started
    pub start_time: std::time::Instant,
}

impl Default for TimeState {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeState {
    /// Create a new time state.
    pub fn new() -> Self {
        Self {
            total_time: 0.0,
            delta_time: 0.0,
            frame_count: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Update the time state for a new frame.
    pub fn update(&mut self) {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.start_time).as_secs_f64();

        if self.frame_count > 0 {
            self.delta_time = elapsed - self.total_time;
        } else {
            self.delta_time = 0.0;
        }

        self.total_time = elapsed;
        self.frame_count += 1;
    }
}

/// Mutable application state that changes during runtime.
///
/// This struct contains all the mutable data for the application, including
/// the mesh cache, camera, scene information, and input state.
///
/// # Example
///
/// ```ignore
/// use renderlib::state::AppState;
/// use renderlib::device::GraphicsDevice;
///
/// // Create application state
/// let device = GraphicsDevice::new(...).await;
/// let mut state = AppState::new(device.wgpu_device());
///
/// // Load a mesh
/// let mesh_source = MeshSource::Path("mesh.gltf".to_string());
/// let mesh_handle = state.mesh_cache.load(&mesh_source).unwrap();
/// ```
#[derive(Debug)]
pub struct AppState {
    /// Central cache for managing mesh assets and GPU resources.
    pub mesh_cache: MeshCache,
    /// Main camera for the scene.
    pub camera: Camera,
    /// Input state for tracking user input.
    pub input: InputState,
    /// Timing information.
    pub time: TimeState,
    /// Currently active mesh handle (for debugging/demonstration).
    pub active_mesh: Option<MeshHandle>,
}

impl AppState {
    /// Create a new application state with the given wgpu device.
    ///
    /// # Arguments
    ///
    /// * `device` - The wgpu device to use for creating GPU resources
    ///
    /// # Returns
    ///
    /// A new `AppState` instance ready for use.
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            mesh_cache: MeshCache::new(device),
            camera: Camera::default(),
            input: InputState::new(),
            time: TimeState::new(),
            active_mesh: None,
        }
    }

    /// Update the time state for a new frame.
    pub fn update_time(&mut self) {
        self.time.update();
    }

    /// Set the active mesh handle.
    pub fn set_active_mesh(&mut self, handle: MeshHandle) {
        self.active_mesh = Some(handle);
    }

    /// Clear the active mesh handle.
    pub fn clear_active_mesh(&mut self) {
        self.active_mesh = None;
    }

    /// Get the active mesh handle.
    pub fn get_active_mesh(&self) -> Option<MeshHandle> {
        self.active_mesh
    }

    /// Load a mesh and set it as active.
    pub fn load_and_set_active(
        &mut self,
        source: &crate::mesh::MeshSource,
    ) -> Result<MeshHandle, crate::mesh::MeshLoadError> {
        let handle = self.mesh_cache.load_mut(source)?;
        self.set_active_mesh(handle);
        Ok(handle)
    }
}

impl Default for AppState {
    fn default() -> Self {
        // This is a temporary implementation for cases where we don't have a device yet
        // In practice, you should use AppState::new(device) for proper initialization
        panic!("AppState::default() requires a wgpu device. Use AppState::new(device) instead.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        // This test would require a proper wgpu device
        // For now, just verify the types compile
        let _: Option<AppState> = None;
    }

    #[test]
    fn test_input_state() {
        let mut input = InputState::new();
        input.set_mouse_position(100.0, 200.0);
        assert_eq!(input.mouse_position, Some((100.0, 200.0)));
    }

    #[test]
    fn test_time_state() {
        let mut time = TimeState::new();
        time.update();
        assert_eq!(time.frame_count, 1);
        assert!(time.total_time >= 0.0);
    }
}
