//! Player module for first-person camera control.
//!
//! Provides player state management for frame-rate independent movement and
//! camera control in 3D applications.

use cgmath::{InnerSpace, Matrix3, Point3, Rad, Vector3};

use crate::camera::Camera;
use crate::input::{InputController, MouseDelta};

/// Default movement speed in units per second.
const DEFAULT_MOVE_SPEED: f32 = 2.5;

/// Default deceleration rate (higher = faster deceleration).
const DEFAULT_DECELERATION: f32 = 10.0;

/// Default acceleration rate (higher = faster acceleration).
const DEFAULT_ACCELERATION: f32 = 20.0;

/// Default mouse look sensitivity in radians per pixel.
const DEFAULT_MOUSE_SENSITIVITY: f32 = 0.002;

/// Player input for a single frame.
///
/// Captures movement direction flags and mouse delta for camera control.
///
/// # Example
///
/// ```no_run
/// use renderlib::player::{PlayerInput, MovementSettings};
/// use renderlib::input::MouseDelta;
///
/// let input = PlayerInput::new()
///     .with_move_forward(true)
///     .with_move_right(true)
///     .with_mouse_delta(MouseDelta::new_with(10.0, 5.0));
/// ```
#[derive(Debug, Clone, Default)]
pub struct PlayerInput {
    /// Whether to move forward
    pub move_forward: bool,
    /// Whether to move backward
    pub move_backward: bool,
    /// Whether to move left
    pub move_left: bool,
    /// Whether to move right
    pub move_right: bool,
    /// Mouse movement delta for this frame
    pub mouse_delta: MouseDelta,
}

impl PlayerInput {
    /// Creates a new PlayerInput with all movement flags set to false and zero mouse delta.
    pub fn new() -> Self {
        Self {
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            mouse_delta: MouseDelta::new(),
        }
    }

    /// Creates a PlayerInput from an InputController.
    ///
    /// This is a convenience method to create a PlayerInput from the current
    /// state of an InputController. It also takes the mouse delta from the controller.
    pub fn from_input_controller(input: &mut InputController) -> Self {
        Self {
            move_forward: input.is_key_pressed("w"),
            move_backward: input.is_key_pressed("s"),
            move_left: input.is_key_pressed("a"),
            move_right: input.is_key_pressed("d"),
            mouse_delta: input.take_mouse_delta(),
        }
    }

    /// Sets whether the player should move forward.
    pub fn with_move_forward(mut self, value: bool) -> Self {
        self.move_forward = value;
        self
    }

    /// Sets whether the player should move backward.
    pub fn with_move_backward(mut self, value: bool) -> Self {
        self.move_backward = value;
        self
    }

    /// Sets whether the player should move left.
    pub fn with_move_left(mut self, value: bool) -> Self {
        self.move_left = value;
        self
    }

    /// Sets whether the player should move right.
    pub fn with_move_right(mut self, value: bool) -> Self {
        self.move_right = value;
        self
    }

    /// Sets the mouse delta for this input.
    pub fn with_mouse_delta(mut self, delta: MouseDelta) -> Self {
        self.mouse_delta = delta;
        self
    }
}

/// Player state that encapsulates position and handles movement based on input.
///
/// This struct maintains the player's position, orientation, and velocity, and provides
/// methods to update the position based on input state. The player's position
/// can then be applied to a Camera once per frame.
///
/// # Example
///
/// ```ignore
/// use renderlib::player::{PlayerState, PlayerInput};
///
/// let mut player = PlayerState::new();
/// let mut input = InputController::new();
///
/// // In your input handler:
/// input.handle_window_event(&event);
///
/// // In your render loop:
/// let player_input = PlayerInput::from_input_controller(&input);
/// player.update(&player_input, delta_time);
/// player.apply_to_camera(&mut camera);
/// ```
#[derive(Debug, Clone)]
pub struct PlayerState {
    /// Player position in world space.
    pub position: Point3<f32>,
    /// Direction the player is facing (forward vector).
    pub forward: Vector3<f32>,
    /// Up vector.
    pub up: Vector3<f32>,
    /// Current velocity vector in world space (units per second).
    pub velocity: Vector3<f32>,
    /// Maximum movement speed in units per second.
    pub max_move_speed: f32,
    /// Acceleration rate in units per second per second.
    pub acceleration: f32,
    /// Deceleration rate in units per second per second.
    pub deceleration: f32,
    /// Mouse look sensitivity in radians per pixel.
    pub mouse_sensitivity: f32,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerState {
    /// Creates a new PlayerState with default values.
    ///
    /// The player starts at position (0, 0, 5) looking towards the origin,
    /// with Y as the up vector, zero velocity, and default acceleration/deceleration.
    pub fn new() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 5.0),
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            max_move_speed: DEFAULT_MOVE_SPEED,
            acceleration: DEFAULT_ACCELERATION,
            deceleration: DEFAULT_DECELERATION,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    /// Creates a new PlayerState with a custom position.
    pub fn with_position(position: Point3<f32>) -> Self {
        Self {
            position,
            forward: Vector3::new(0.0, 0.0, -1.0),
            up: Vector3::new(0.0, 1.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            max_move_speed: DEFAULT_MOVE_SPEED,
            acceleration: DEFAULT_ACCELERATION,
            deceleration: DEFAULT_DECELERATION,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    /// Creates a new PlayerState with custom position and forward direction.
    pub fn with_position_and_forward(position: Point3<f32>, forward: Vector3<f32>) -> Self {
        Self {
            position,
            forward: forward.normalize(),
            up: Vector3::new(0.0, 1.0, 0.0),
            velocity: Vector3::new(0.0, 0.0, 0.0),
            max_move_speed: DEFAULT_MOVE_SPEED,
            acceleration: DEFAULT_ACCELERATION,
            deceleration: DEFAULT_DECELERATION,
            mouse_sensitivity: DEFAULT_MOUSE_SENSITIVITY,
        }
    }

    /// Updates the player's position and orientation based on input state and delta time.
    ///
    /// This method should be called once per frame with the current input state
    /// and the time elapsed since the last frame. It updates the orientation based
    /// on mouse movement, then updates the velocity with gradual acceleration and
    /// deceleration, and finally applies the velocity to the position.
    ///
    /// # Arguments
    ///
    /// * `input` - The PlayerInput containing current movement state and mouse delta
    /// * `delta_time` - Time elapsed since the last frame, in seconds
    pub fn update(&mut self, input: &PlayerInput, delta_time: f32) {
        // Handle mouse look first (rotate forward and up vectors)
        // Mouse look is applied whenever there's mouse delta (filtering happens upstream)
        if input.mouse_delta.x != 0.0 || input.mouse_delta.y != 0.0 {
            self.apply_mouse_look(&input.mouse_delta);
        }

        // Calculate right vector from forward and up (needs to be recalculated after mouse look)
        let right = self.forward.cross(self.up).normalize();

        // Build target movement direction from input
        let mut target_direction = Vector3::new(0.0, 0.0, 0.0);

        if input.move_forward {
            target_direction += self.forward;
        }
        if input.move_backward {
            target_direction -= self.forward;
        }
        if input.move_left {
            target_direction -= right;
        }
        if input.move_right {
            target_direction += right;
        }

        // Normalize target direction if there's any movement
        let target_velocity = if target_direction.magnitude2() > 0.0 {
            target_direction.normalize() * self.max_move_speed
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };

        // Calculate the direction we want to accelerate towards
        let velocity_diff = target_velocity - self.velocity;

        // Calculate acceleration amount based on whether we're speeding up or slowing down
        let acceleration_amount = if velocity_diff.magnitude2() > 0.0 {
            // Use acceleration when speeding up, deceleration when slowing down
            if velocity_diff.dot(self.velocity) > 0.0 {
                // Accelerating in the same general direction
                self.acceleration
            } else {
                // Decelerating (moving towards stopping or reversing)
                self.deceleration
            }
        } else {
            0.0
        };

        // Apply acceleration/deceleration, clamped to the target velocity
        let acceleration_vector = if acceleration_amount > 0.0 {
            velocity_diff.normalize() * acceleration_amount * delta_time
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };

        // Update velocity, but don't overshoot the target
        let new_velocity = self.velocity + acceleration_vector;

        // Check if we've overshot the target velocity
        if (new_velocity - target_velocity).magnitude2()
            < (self.velocity - target_velocity).magnitude2()
        {
            // We're getting closer, use the new velocity
            self.velocity = new_velocity;
        } else {
            // We've overshot, just use the target velocity
            self.velocity = target_velocity;
        }

        // Apply velocity to position
        self.position += self.velocity * delta_time;
    }

    /// Applies mouse look rotation to the player's orientation.
    ///
    /// This rotates the forward and up vectors based on mouse movement.
    /// - Horizontal mouse movement (x) rotates around the up vector (yaw)
    /// - Vertical mouse movement (y) rotates around the right vector (pitch)
    ///
    /// # Arguments
    ///
    /// * `mouse_delta` - The mouse movement delta in pixels
    fn apply_mouse_look(&mut self, mouse_delta: &MouseDelta) {
        // Calculate rotation amounts based on mouse delta and sensitivity
        let yaw_angle = Rad(-mouse_delta.x * self.mouse_sensitivity);
        let pitch_angle = Rad(-mouse_delta.y * self.mouse_sensitivity);

        // Rotate forward vector around up vector (yaw)
        let yaw_rotation = Matrix3::from_axis_angle(self.up, yaw_angle);
        self.forward = yaw_rotation * self.forward;

        // Calculate right vector after yaw rotation
        let right = self.up.cross(self.forward).normalize();

        // Rotate forward vector around right vector (pitch)
        let pitch_rotation = Matrix3::from_axis_angle(right, pitch_angle);
        self.forward = pitch_rotation * self.forward;

        // Ensure forward vector stays normalized
        self.forward = self.forward.normalize();
    }

    /// Applies a PlayerInput to update the player's position.
    ///
    /// This is an alternative to `update` that takes a PlayerInput directly.
    /// This enables features like recording and replaying input sequences.
    ///
    /// # Arguments
    ///
    /// * `input` - The PlayerInput to apply
    /// * `delta_time` - Time elapsed since the last frame, in seconds
    pub fn apply_input(&mut self, input: &PlayerInput, delta_time: f32) {
        self.update(input, delta_time);
    }

    /// Applies the player's position to a camera.
    ///
    /// This updates the camera's position while keeping its target relative
    /// to the player's forward direction. The camera will look in the direction
    /// the player is facing.
    ///
    /// # Arguments
    ///
    /// * `camera` - The camera to update
    pub fn apply_to_camera(&self, camera: &mut Camera) {
        // Calculate target based on player position and forward direction
        let target = self.position + self.forward;

        camera.position = self.position;
        camera.target = target;
        camera.up = self.up;
    }

    /// Sets the maximum movement speed.
    pub fn set_max_move_speed(&mut self, speed: f32) {
        self.max_move_speed = speed;
    }

    /// Gets the current maximum movement speed.
    pub fn get_max_move_speed(&self) -> f32 {
        self.max_move_speed
    }

    /// Sets the acceleration rate.
    pub fn set_acceleration(&mut self, acceleration: f32) {
        self.acceleration = acceleration;
    }

    /// Gets the current acceleration rate.
    pub fn get_acceleration(&self) -> f32 {
        self.acceleration
    }

    /// Sets the deceleration rate.
    pub fn set_deceleration(&mut self, deceleration: f32) {
        self.deceleration = deceleration;
    }

    /// Gets the current deceleration rate.
    pub fn get_deceleration(&self) -> f32 {
        self.deceleration
    }

    /// Sets the mouse look sensitivity.
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        self.mouse_sensitivity = sensitivity;
    }

    /// Gets the current mouse look sensitivity.
    pub fn get_mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }

    /// Sets the current velocity.
    pub fn set_velocity(&mut self, velocity: Vector3<f32>) {
        self.velocity = velocity;
    }

    /// Gets the current velocity.
    pub fn get_velocity(&self) -> Vector3<f32> {
        self.velocity
    }

    /// Sets the player's position.
    pub fn set_position(&mut self, position: Point3<f32>) {
        self.position = position;
    }

    /// Gets the player's position.
    pub fn get_position(&self) -> Point3<f32> {
        self.position
    }

    /// Sets the player's forward direction.
    pub fn set_forward(&mut self, forward: Vector3<f32>) {
        self.forward = forward.normalize();
    }

    /// Gets the player's forward direction.
    pub fn get_forward(&self) -> Vector3<f32> {
        self.forward
    }

    /// Sets the player's up vector.
    pub fn set_up(&mut self, up: Vector3<f32>) {
        self.up = up.normalize();
    }

    /// Gets the player's up vector.
    pub fn get_up(&self) -> Vector3<f32> {
        self.up
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_player() {
        let player = PlayerState::new();
        assert_eq!(player.position, Point3::new(0.0, 0.0, 5.0));
        assert_eq!(player.forward, Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(player.up, Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(player.velocity, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(player.max_move_speed, DEFAULT_MOVE_SPEED);
        assert_eq!(player.acceleration, DEFAULT_ACCELERATION);
        assert_eq!(player.deceleration, DEFAULT_DECELERATION);
    }

    #[test]
    fn test_with_position() {
        let position = Point3::new(1.0, 2.0, 3.0);
        let player = PlayerState::with_position(position);
        assert_eq!(player.position, position);
    }

    #[test]
    fn test_setters_and_getters() {
        let mut player = PlayerState::new();

        let new_position = Point3::new(10.0, 20.0, 30.0);
        player.set_position(new_position);
        assert_eq!(player.get_position(), new_position);

        let new_forward = Vector3::new(1.0, 0.0, 0.0);
        player.set_forward(new_forward);
        assert_eq!(player.get_forward(), Vector3::new(1.0, 0.0, 0.0));

        let new_up = Vector3::new(0.0, 0.0, 1.0);
        player.set_up(new_up);
        assert_eq!(player.get_up(), Vector3::new(0.0, 0.0, 1.0));

        let new_velocity = Vector3::new(1.0, 2.0, 3.0);
        player.set_velocity(new_velocity);
        assert_eq!(player.get_velocity(), new_velocity);

        player.set_max_move_speed(5.0);
        assert_eq!(player.get_max_move_speed(), 5.0);
    }

    #[test]
    fn test_apply_to_camera() {
        let mut camera = Camera::default();
        let player = PlayerState::new();

        player.apply_to_camera(&mut camera);

        assert_eq!(camera.position, Point3::new(0.0, 0.0, 5.0));
        // Target should be player position + forward direction
        assert_eq!(camera.target, Point3::new(0.0, 0.0, 4.0));
        assert_eq!(camera.up, Vector3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn test_update_no_input() {
        let mut player = PlayerState::new();
        let input = PlayerInput::new();
        let original_position = player.position;

        player.update(&input, 1.0);

        // Position should not change with no input
        assert_eq!(player.position, original_position);
    }

    #[test]
    fn test_apply_input() {
        let mut player = PlayerState::new();
        let original_position = player.position;

        // Create input with forward movement
        let input = PlayerInput::new().with_move_forward(true);

        // Apply input with 1 second delta
        player.apply_input(&input, 1.0);

        // Position should have moved forward
        assert_ne!(player.position, original_position);

        // Velocity should be set to max speed in forward direction
        assert_eq!(player.velocity, player.forward * player.max_move_speed);
    }

    #[test]
    fn test_velocity_based_movement() {
        let mut player = PlayerState::new();
        let original_position = player.position;

        // Create input with forward movement
        let input = PlayerInput::new().with_move_forward(true);

        // Apply input with 0.5 second delta
        player.update(&input, 0.5);

        // Position should have moved forward by max_speed * 0.5
        let expected_displacement = player.forward * player.max_move_speed * 0.5;
        assert_eq!(player.position, original_position + expected_displacement);

        // Velocity should be at max speed
        assert_eq!(player.velocity, player.forward * player.max_move_speed);
    }

    #[test]
    fn test_velocity_decelerates_when_no_input() {
        let mut player = PlayerState::new();

        // First, accelerate forward to max speed
        let input = PlayerInput::new().with_move_forward(true);
        // Apply input for long enough to reach max speed
        player.update(&input, 2.0);
        assert!(player.velocity.magnitude() > 0.0);

        // Now with no input, velocity should start decelerating
        let no_input = PlayerInput::new();
        let initial_velocity = player.velocity;
        player.update(&no_input, 0.1);

        // Velocity should be less than before (decelerating)
        assert!(player.velocity.magnitude() < initial_velocity.magnitude());

        // But not zero yet (gradual deceleration)
        assert!(player.velocity.magnitude() > 0.0);
    }

    #[test]
    fn test_velocity_eventually_stops() {
        let mut player = PlayerState::new();

        // Accelerate to max speed
        let input = PlayerInput::new().with_move_forward(true);
        player.update(&input, 2.0);

        // Apply deceleration for enough time to stop
        // With default deceleration of 10.0 and max speed of 2.5,
        // it should take about 0.25 seconds to stop
        let no_input = PlayerInput::new();
        player.update(&no_input, 0.5);

        // Velocity should be zero or very close to it
        assert!(player.velocity.magnitude() < 0.001);
    }

    #[test]
    fn test_diagonal_movement_normalized() {
        let mut player = PlayerState::new();

        // Create input with forward + right movement (diagonal)
        let input = PlayerInput::new()
            .with_move_forward(true)
            .with_move_right(true);

        player.update(&input, 1.0);

        // Velocity should be normalized (magnitude should be max_move_speed, not sqrt(2) * max_move_speed)
        let velocity_magnitude = player.velocity.magnitude();
        assert!((velocity_magnitude - player.max_move_speed).abs() < 0.0001);
    }

    #[test]
    fn test_mouse_look() {
        use super::super::input::MouseDelta;

        let mut player = PlayerState::new();
        let original_forward = player.forward;

        // Create input with mouse movement - mouse look should be applied
        let input = PlayerInput::new().with_mouse_delta(MouseDelta::new_with(100.0, 50.0));

        player.update(&input, 0.016); // ~60fps frame

        // Forward vector should have changed
        assert_ne!(player.forward, original_forward);

        // Forward vector should still be normalized
        let forward_magnitude = player.forward.magnitude();
        assert!((forward_magnitude - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_mouse_look_with_zero_delta() {
        use super::super::input::MouseDelta;

        let mut player = PlayerState::new();
        let original_forward = player.forward;

        // Create input with zero mouse movement - mouse look should not be applied
        let input = PlayerInput::new().with_mouse_delta(MouseDelta::new());

        player.update(&input, 0.016); // ~60fps frame

        // Forward vector should NOT have changed (no mouse movement)
        assert_eq!(player.forward, original_forward);
    }

    #[test]
    fn test_mouse_sensitivity_getter_setter() {
        let mut player = PlayerState::new();

        player.set_mouse_sensitivity(0.01);
        assert_eq!(player.get_mouse_sensitivity(), 0.01);
    }

    #[test]
    fn test_from_input_controller() {
        use super::super::input::InputController;

        let mut input_controller = InputController::new();
        let player_input = PlayerInput::from_input_controller(&mut input_controller);

        // With no keys pressed, all movement flags should be false
        assert!(!player_input.move_forward);
        assert!(!player_input.move_backward);
        assert!(!player_input.move_left);
        assert!(!player_input.move_right);
    }

    #[test]
    fn test_player_input_builder() {
        let input = PlayerInput::new()
            .with_move_forward(true)
            .with_move_right(true);

        assert!(input.move_forward);
        assert!(input.move_right);
        assert!(!input.move_backward);
        assert!(!input.move_left);
    }
}
