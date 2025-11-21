//! Editor camera controls (orbit, pan, zoom)

use super::{selection::Selection, EditorCameraMode, EditorState};
use bevy::prelude::*;

/// Marker component for editor camera
#[derive(Component)]
pub struct EditorCamera {
    /// Focus point for orbit
    pub focus: Vec3,
    /// Distance from focus
    pub distance: f32,
    /// Orbit angles (yaw, pitch)
    pub yaw: f32,
    pub pitch: f32,
    /// Movement speed
    pub move_speed: f32,
    /// Rotation speed
    pub rotation_speed: f32,
    /// Zoom speed
    pub zoom_speed: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            distance: 10.0,
            yaw: 0.0,
            pitch: 30f32.to_radians(),
            move_speed: 5.0,
            rotation_speed: 0.005,
            zoom_speed: 1.0,
        }
    }
}

impl EditorCamera {
    pub fn new(focus: Vec3, distance: f32) -> Self {
        Self {
            focus,
            distance,
            ..default()
        }
    }

    /// Calculate camera position based on orbit parameters
    pub fn calculate_position(&self) -> Vec3 {
        let offset = Vec3::new(
            self.distance * self.pitch.cos() * self.yaw.sin(),
            self.distance * self.pitch.sin(),
            self.distance * self.pitch.cos() * self.yaw.cos(),
        );
        self.focus + offset
    }

    /// Frame the selected object
    pub fn frame_target(&mut self, target_position: Vec3, target_size: f32) {
        self.focus = target_position;
        self.distance = (target_size * 2.0).max(5.0);
    }

    /// Set to top view
    pub fn set_top_view(&mut self) {
        self.pitch = 89f32.to_radians();
        self.yaw = 0.0;
    }

    /// Set to front view
    pub fn set_front_view(&mut self) {
        self.pitch = 0.0;
        self.yaw = 0.0;
    }

    /// Set to side view
    pub fn set_side_view(&mut self) {
        self.pitch = 0.0;
        self.yaw = 90f32.to_radians();
    }
}

/// System to handle editor camera controls
pub fn editor_camera_system(
    state: Res<EditorState>,
    selection: Res<Selection>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: EventReader<bevy::input::mouse::MouseMotion>,
    mut scroll: EventReader<bevy::input::mouse::MouseWheel>,
    mut cameras: Query<(&mut EditorCamera, &mut Transform), With<Camera>>,
    transforms: Query<&GlobalTransform>,
    time: Res<Time>,
) {
    let Ok((mut editor_camera, mut camera_transform)) = cameras.get_single_mut() else {
        return;
    };

    let delta_time = time.delta_secs();

    // Handle keyboard camera mode changes
    if keyboard.just_pressed(KeyCode::Numpad7) {
        editor_camera.set_top_view();
    }
    if keyboard.just_pressed(KeyCode::Numpad1) {
        editor_camera.set_front_view();
    }
    if keyboard.just_pressed(KeyCode::Numpad3) {
        editor_camera.set_side_view();
    }

    // Frame selected object with F key
    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Some(entity) = selection.primary {
            if let Ok(transform) = transforms.get(entity) {
                editor_camera.frame_target(transform.translation(), 2.0);
            }
        }
    }

    // Mouse controls
    let middle_mouse = mouse_button.pressed(MouseButton::Middle);
    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);

    // Accumulate mouse motion
    let mut mouse_delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        mouse_delta += event.delta;
    }

    match state.camera_mode {
        EditorCameraMode::Orbit => {
            if middle_mouse && !shift_pressed && mouse_delta != Vec2::ZERO {
                // Orbit camera
                editor_camera.yaw -= mouse_delta.x * editor_camera.rotation_speed;
                editor_camera.pitch -= mouse_delta.y * editor_camera.rotation_speed;

                // Clamp pitch to avoid gimbal lock
                editor_camera.pitch = editor_camera
                    .pitch
                    .clamp(-89f32.to_radians(), 89f32.to_radians());
            }
        }
        EditorCameraMode::Pan => {
            if middle_mouse && shift_pressed && mouse_delta != Vec2::ZERO {
                // Pan camera
                let right = camera_transform.right();
                let up = camera_transform.up();

                let pan_speed = editor_camera.distance * 0.001;
                editor_camera.focus -= right * mouse_delta.x * pan_speed;
                editor_camera.focus += up * mouse_delta.y * pan_speed;
            }
        }
        EditorCameraMode::Fly => {
            // WASD fly controls
            let mut movement = Vec3::ZERO;

            if keyboard.pressed(KeyCode::KeyW) {
                movement += *camera_transform.forward();
            }
            if keyboard.pressed(KeyCode::KeyS) {
                movement -= *camera_transform.forward();
            }
            if keyboard.pressed(KeyCode::KeyA) {
                movement -= *camera_transform.right();
            }
            if keyboard.pressed(KeyCode::KeyD) {
                movement += *camera_transform.right();
            }
            if keyboard.pressed(KeyCode::KeyQ) {
                movement += Vec3::Y;
            }
            if keyboard.pressed(KeyCode::KeyE) {
                movement -= Vec3::Y;
            }

            if movement != Vec3::ZERO {
                movement = movement.normalize();
                let move_speed = editor_camera.move_speed;
                editor_camera.focus += movement * move_speed * delta_time;
            }
        }
        _ => {}
    }

    // Zoom with scroll wheel
    for event in scroll.read() {
        editor_camera.distance -= event.y * editor_camera.zoom_speed;
        editor_camera.distance = editor_camera.distance.clamp(0.1, 1000.0);
    }

    // Update camera transform
    let position = editor_camera.calculate_position();
    camera_transform.translation = position;
    camera_transform.look_at(editor_camera.focus, Vec3::Y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_camera_default() {
        let camera = EditorCamera::default();
        assert_eq!(camera.focus, Vec3::ZERO);
        assert_eq!(camera.distance, 10.0);
    }

    #[test]
    fn test_editor_camera_position() {
        let camera = EditorCamera::default();
        let position = camera.calculate_position();
        assert!(position.length() > 0.0);
    }

    #[test]
    fn test_frame_target() {
        let mut camera = EditorCamera::default();
        let target = Vec3::new(5.0, 0.0, 5.0);

        camera.frame_target(target, 1.0);
        assert_eq!(camera.focus, target);
        assert!(camera.distance >= 5.0);
    }

    #[test]
    fn test_camera_views() {
        let mut camera = EditorCamera::default();

        camera.set_top_view();
        assert!(camera.pitch > 1.0); // Near 90 degrees

        camera.set_front_view();
        assert_eq!(camera.pitch, 0.0);
        assert_eq!(camera.yaw, 0.0);

        camera.set_side_view();
        assert_eq!(camera.pitch, 0.0);
        assert!(camera.yaw.abs() > 1.0); // Near 90 degrees
    }
}
