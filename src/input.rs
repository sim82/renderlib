//! Input handling module for keyboard and mouse input.
//!
//! Provides input state tracking for frame-rate independent movement and controls.

use std::collections::HashSet;
use winit::dpi::PhysicalPosition;
use winit::event::WindowEvent;
use winit::keyboard::Key;

/// Mouse input mode for camera control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseMode {
    /// Normal mode: mouse look is only enabled while Shift is held down.
    Normal,
    /// Grabbed mode: mouse look is constantly enabled.
    Grabbed,
}

impl Default for MouseMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Mouse movement delta for a frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct MouseDelta {
    /// Horizontal mouse movement (pixels).
    pub x: f32,
    /// Vertical mouse movement (pixels).
    pub y: f32,
}

impl MouseDelta {
    /// Creates a new MouseDelta with zero movement.
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Creates a new MouseDelta with the given values.
    pub fn new_with(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

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
#[derive(Debug)]
pub struct InputController {
    /// Set of currently pressed keys (stored as lowercase strings)
    pressed_keys: HashSet<String>,
    /// Current mouse movement delta for the frame.
    mouse_delta: MouseDelta,
    /// Previous cursor position for calculating delta.
    prev_cursor_pos: Option<(f64, f64)>,
    /// Current mouse input mode.
    mouse_mode: MouseMode,
    /// Whether shift key is currently pressed (for normal mode mouse look).
    shift_pressed: bool,
}

impl Default for InputController {
    fn default() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            mouse_delta: MouseDelta::new(),
            prev_cursor_pos: None,
            mouse_mode: MouseMode::Normal,
            shift_pressed: false,
        }
    }
}

impl InputController {
    /// Creates a new, empty InputController.
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            mouse_delta: MouseDelta::new(),
            prev_cursor_pos: None,
            mouse_mode: MouseMode::Normal,
            shift_pressed: false,
        }
    }

    /// Processes a window event and updates the internal key and mouse state.
    ///
    /// Call this from your application's input handler to track which keys
    /// are currently pressed and mouse movement.
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                // Handle tilde key (`) for toggling mouse mode
                if let Key::Character(c) = &key_event.logical_key {
                    let key_str = c.to_ascii_lowercase();

                    if key_str == "`" && key_event.state.is_pressed() {
                        // Toggle mouse mode
                        self.mouse_mode = match self.mouse_mode {
                            MouseMode::Normal => MouseMode::Grabbed,
                            MouseMode::Grabbed => MouseMode::Normal,
                        };
                    }

                    if key_event.state.is_pressed() {
                        self.pressed_keys.insert(key_str);
                    } else {
                        self.pressed_keys.remove(&key_str);
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                // Handle modifier changes for shift key tracking
                self.shift_pressed = modifiers.state().shift_key();
            }
            WindowEvent::MouseInput { .. } => {
                // We don't track mouse button state for now, just movement
            }
            WindowEvent::MouseWheel { .. } => {
                // We don't track mouse wheel for now
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                // Calculate delta from previous position
                let PhysicalPosition { x, y } = *position;
                if let Some((prev_x, prev_y)) = self.prev_cursor_pos {
                    let dx = x - prev_x;
                    let dy = y - prev_y;
                    self.mouse_delta.x += dx as f32;
                    self.mouse_delta.y += dy as f32;
                }
                // Update previous position
                self.prev_cursor_pos = Some((x, y));
            }
            _ => {}
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

    /// Returns the current mouse mode.
    pub fn get_mouse_mode(&self) -> MouseMode {
        self.mouse_mode
    }

    /// Sets the mouse mode.
    pub fn set_mouse_mode(&mut self, mode: MouseMode) {
        self.mouse_mode = mode;
    }

    /// Returns whether shift key is currently pressed.
    pub fn is_shift_pressed(&self) -> bool {
        self.shift_pressed
    }

    /// Returns whether mouse look should be active based on current mode.
    /// In Normal mode: active only when Shift is pressed.
    /// In Grabbed mode: always active.
    pub fn is_mouse_look_active(&self) -> bool {
        match self.mouse_mode {
            MouseMode::Normal => self.shift_pressed,
            MouseMode::Grabbed => true,
        }
    }

    /// Clears all pressed key states.
    ///
    /// This can be useful when resetting the controller or handling
    /// window focus changes.
    pub fn clear(&mut self) {
        self.pressed_keys.clear();
    }

    /// Gets the current mouse delta and resets it to zero.
    ///
    /// This should be called once per frame to get the mouse movement
    /// for that frame. The delta is reset after reading to ensure
    /// each frame gets fresh mouse movement data.
    pub fn take_mouse_delta(&mut self) -> MouseDelta {
        let delta = self.mouse_delta;
        self.mouse_delta = MouseDelta::new();
        delta
    }

    /// Gets the current mouse delta without resetting it.
    pub fn get_mouse_delta(&self) -> MouseDelta {
        self.mouse_delta
    }

    /// Resets the mouse delta to zero.
    pub fn reset_mouse_delta(&mut self) {
        self.mouse_delta = MouseDelta::new();
    }

    /// Creates a PlayerInput with mouse delta filtered based on mouse mode.
    ///
    /// In Normal mode: mouse delta is zeroed out unless Shift is pressed.
    /// In Grabbed mode: mouse delta is passed through as-is.
    /// This allows the player logic to remain unaware of mouse grab state.
    /// Creates a PlayerInput with mouse delta filtered based on mouse mode.
    ///
    /// In Normal mode: mouse delta is zeroed out unless Shift is pressed.
    /// In Grabbed mode: mouse delta is passed through as-is.
    /// This allows the player logic to remain unaware of mouse grab state.
    pub fn get_player_input(&mut self) -> crate::player::PlayerInput {
        let mouse_delta = self.take_mouse_delta();
        let mouse_look_active = self.is_mouse_look_active();

        // Filter mouse delta based on whether mouse look is active
        let filtered_mouse_delta = if mouse_look_active {
            mouse_delta
        } else {
            MouseDelta::new()
        };

        crate::player::PlayerInput {
            move_forward: self.is_key_pressed("w"),
            move_backward: self.is_key_pressed("s"),
            move_left: self.is_key_pressed("a"),
            move_right: self.is_key_pressed("d"),
            mouse_delta: filtered_mouse_delta,
        }
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

    #[test]
    fn test_mouse_mode_default() {
        let input = InputController::new();
        assert_eq!(input.get_mouse_mode(), MouseMode::Normal);
    }

    #[test]
    fn test_mouse_mode_toggle() {
        let mut input = InputController::new();
        assert_eq!(input.get_mouse_mode(), MouseMode::Normal);

        // Simulate tilde key press to toggle to Grabbed
        input.set_mouse_mode(MouseMode::Grabbed);
        assert_eq!(input.get_mouse_mode(), MouseMode::Grabbed);

        // Toggle back to Normal
        input.set_mouse_mode(MouseMode::Normal);
        assert_eq!(input.get_mouse_mode(), MouseMode::Normal);
    }

    #[test]
    fn test_shift_pressed_default() {
        let input = InputController::new();
        assert!(!input.is_shift_pressed());
    }

    #[test]
    fn test_is_mouse_look_active() {
        let mut input = InputController::new();

        // In Normal mode without shift, mouse look should be inactive
        assert!(!input.is_mouse_look_active());

        // In Normal mode with shift, mouse look should be active
        input.shift_pressed = true;
        assert!(input.is_mouse_look_active());

        // In Grabbed mode without shift, mouse look should still be active
        input.shift_pressed = false;
        input.set_mouse_mode(MouseMode::Grabbed);
        assert!(input.is_mouse_look_active());

        // In Grabbed mode with shift, mouse look should be active
        input.shift_pressed = true;
        assert!(input.is_mouse_look_active());
    }

    #[test]
    fn test_get_player_input_filters_mouse_delta() {
        let mut input = InputController::new();

        // Simulate mouse movement
        input.mouse_delta = MouseDelta::new_with(100.0, 50.0);

        // In Normal mode without shift, mouse delta should be filtered out
        let player_input = input.get_player_input();
        assert_eq!(player_input.mouse_delta.x, 0.0);
        assert_eq!(player_input.mouse_delta.y, 0.0);

        // Simulate mouse movement again
        input.mouse_delta = MouseDelta::new_with(100.0, 50.0);
        input.shift_pressed = true; // Now shift is pressed

        // In Normal mode with shift, mouse delta should pass through
        let player_input = input.get_player_input();
        assert_eq!(player_input.mouse_delta.x, 100.0);
        assert_eq!(player_input.mouse_delta.y, 50.0);

        // Simulate mouse movement again
        input.mouse_delta = MouseDelta::new_with(100.0, 50.0);
        input.shift_pressed = false; // Shift released
        input.set_mouse_mode(MouseMode::Grabbed); // Switch to grabbed mode

        // In Grabbed mode, mouse delta should pass through regardless of shift
        let player_input = input.get_player_input();
        assert_eq!(player_input.mouse_delta.x, 100.0);
        assert_eq!(player_input.mouse_delta.y, 50.0);
    }

    #[test]
    fn test_get_player_input_includes_key_states() {
        let mut input = InputController::new();

        // Simulate pressing W and D keys
        input.pressed_keys.insert("w".to_string());
        input.pressed_keys.insert("d".to_string());

        let player_input = input.get_player_input();
        assert!(player_input.move_forward);
        assert!(!player_input.move_backward);
        assert!(!player_input.move_left);
        assert!(player_input.move_right);
    }
}
