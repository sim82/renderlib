//! Input handling module for keyboard and mouse input.
//!
//! Provides input state tracking for frame-rate independent movement and controls.

use std::collections::HashSet;
use winit::event::WindowEvent;
use winit::keyboard::Key;

/// Tracks the state of keyboard keys for frame-rate independent input handling.
///
/// This controller maintains a set of currently pressed keys and allows
/// querying their state during the render loop. This enables smooth,
/// frame-rate independent movement that doesn't depend on key repeat rates.
///
/// # Example
///
/// ```ignore
/// use renderlib::input::InputController;
///
/// let mut input = InputController::new();
///
/// // In your input handler:
/// input.handle_window_event(&event);
///
/// // In your render loop:
/// if input.is_key_pressed("w") {
///     camera.move_forward(speed * delta_time);
/// }
/// ```
#[derive(Debug, Default)]
pub struct InputController {
    /// Set of currently pressed keys (stored as lowercase strings)
    pressed_keys: HashSet<String>,
}

impl InputController {
    /// Creates a new, empty InputController.
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
        }
    }

    /// Processes a window event and updates the internal key state.
    ///
    /// Call this from your application's input handler to track which keys
    /// are currently pressed.
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            if let Key::Character(c) = &key_event.logical_key {
                let key_str = c.to_ascii_lowercase();

                if key_event.state.is_pressed() {
                    self.pressed_keys.insert(key_str);
                } else {
                    self.pressed_keys.remove(&key_str);
                }
            }
        }
    }

    /// Checks if a specific key is currently pressed.
    ///
    /// The key should be provided as a lowercase string (e.g., "w", "a", "s", "d").
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check, as a lowercase string
    ///
    /// # Returns
    ///
    /// `true` if the key is currently pressed, `false` otherwise.
    pub fn is_key_pressed(&self, key: &str) -> bool {
        self.pressed_keys.contains(key)
    }

    /// Returns a reference to the set of currently pressed keys.
    ///
    /// This is useful if you need to iterate over all pressed keys or
    /// perform more complex queries.
    pub fn pressed_keys(&self) -> &HashSet<String> {
        &self.pressed_keys
    }

    /// Clears all pressed key states.
    ///
    /// This can be useful when resetting the controller or handling
    /// window focus changes.
    pub fn clear(&mut self) {
        self.pressed_keys.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_controller() {
        let input = InputController::new();
        assert!(input.pressed_keys().is_empty());
    }

    #[test]
    fn test_is_key_pressed_empty() {
        let input = InputController::new();
        assert!(!input.is_key_pressed("w"));
        assert!(!input.is_key_pressed("a"));
        assert!(!input.is_key_pressed("s"));
        assert!(!input.is_key_pressed("d"));
    }

    #[test]
    fn test_clear() {
        let mut input = InputController::new();
        // Clear should work even on empty set
        input.clear();
        assert!(input.pressed_keys().is_empty());
    }

    #[test]
    fn test_pressed_keys_accessor() {
        let input = InputController::new();
        let keys = input.pressed_keys();
        // Should be able to access the keys set
        assert_eq!(keys.len(), 0);
    }
}
