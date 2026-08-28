//! Application state module.
//!
//! This module provides the [`AppState`] struct which contains all mutable application
//! data that changes during runtime, such as the mesh cache, camera, input state,
//! and timing information.
//!
//! # Main Types
//!
//! - [`AppState`] - Main application state container
//! - [`TimeState`] - Timing information
//! - [`InputState`] - Input state

use crate::camera::Camera;
use crate::mesh::{MeshCache, MeshHandle};

/// Input state tracking keyboard, mouse, and other input devices.
#[derive(Debug, Default)]
pub struct InputState {
    /// Currently pressed keys
    pub pressed_keys: Vec<winit::keyboard::Key>,
    /// Mouse position (x, y) in window coordinates
    pub mouse_position: Option<(f64, f64)>,
    /// Mouse buttons currently pressed
    pub mouse_buttons: Vec<u16>,
    /// Scroll wheel delta (x, y)
    pub scroll_delta: (f64, f64),
}

impl InputState {
    /// Creates a new input state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the mouse position.
    pub fn set_mouse_position(&mut self, x: f64, y: f64) {
        self.mouse_position = Some((x, y));
    }

    /// Clears the scroll delta.
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

/// Mutable application state.
///
/// Contains all mutable data for the application, including the mesh cache,
/// camera, input state, and timing information.
///
/// # Example
///
/// ```no_run
/// use renderlib::state::AppState;
///
/// # fn example(device: &wgpu::Device) {
/// // Create application state
/// let mut state = AppState::new(device);
///
/// // Load a mesh
/// use renderlib::mesh::MeshSource;
/// let mesh_handle = state.mesh_cache.load_mut(&MeshSource::Path("mesh.gltf".to_string())).unwrap();
/// # }
/// ```
#[derive(Debug)]
pub struct AppState {
    /// Central cache for managing mesh assets and GPU resources
    pub mesh_cache: MeshCache,
    /// Main camera for the scene
    pub camera: Camera,
    /// Input state for tracking user input
    pub input: InputState,
    /// Timing information
    pub time: TimeState,
    /// Currently active mesh handle
    pub active_mesh: Option<MeshHandle>,
}

impl AppState {
    /// Creates a new application state with the given wgpu device.
    ///
    /// # Arguments
    ///
    /// * `device` - The wgpu device to use for creating GPU resources
    ///
    /// # Returns
    ///
    /// Returns a new [`AppState`] instance ready for use.
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

    /// Clears the active mesh handle.
    pub fn clear_active_mesh(&mut self) {
        self.active_mesh = None;
    }

    /// Returns the active mesh handle, if any.
    pub fn get_active_mesh(&self) -> Option<MeshHandle> {
        self.active_mesh
    }

    /// Loads a mesh and sets it as active.
    ///
    /// Convenience method that loads a mesh and automatically sets it as the active mesh.
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
