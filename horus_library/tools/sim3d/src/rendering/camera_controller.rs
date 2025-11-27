use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

#[derive(Component)]
pub struct OrbitCamera {
    pub focus: Vec3,
    pub radius: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            focus: Vec3::ZERO,
            radius: 10.0,
            yaw: 0.0,
            pitch: 0.5, // Positive pitch positions camera above, looking down at scene
        }
    }
}

pub fn camera_controller_system(
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<(&mut OrbitCamera, &mut Transform)>,
) {
    for (mut orbit, mut transform) in query.iter_mut() {
        // Rotation with right mouse button
        if mouse_button.pressed(MouseButton::Right) {
            for motion in mouse_motion.read() {
                orbit.yaw -= motion.delta.x * 0.005;
                orbit.pitch -= motion.delta.y * 0.005;
                orbit.pitch = orbit.pitch.clamp(-1.5, 1.5);
            }
        }

        // Zoom with mouse wheel
        for wheel in mouse_wheel.read() {
            orbit.radius -= wheel.y * 0.5;
            orbit.radius = orbit.radius.clamp(1.0, 100.0);
        }

        // Pan with middle mouse button
        if mouse_button.pressed(MouseButton::Middle) {
            for motion in mouse_motion.read() {
                let yaw_rot = Quat::from_rotation_y(orbit.yaw);
                let right = yaw_rot * Vec3::X;
                let up = Vec3::Y;

                let pan_speed = orbit.radius * 0.001;
                orbit.focus -= right * motion.delta.x * pan_speed;
                orbit.focus += up * motion.delta.y * pan_speed;
            }
        }

        // Keyboard controls for panning
        let mut pan_direction = Vec3::ZERO;
        let pan_speed = 5.0 * time.delta_secs();

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            pan_direction.z -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            pan_direction.z += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            pan_direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            pan_direction.x += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyQ) {
            pan_direction.y -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            pan_direction.y += 1.0;
        }

        if pan_direction.length_squared() > 0.0 {
            let yaw_rot = Quat::from_rotation_y(orbit.yaw);
            let rotated_pan = yaw_rot * pan_direction.normalize();
            orbit.focus += rotated_pan * pan_speed;
        }

        // Update camera transform
        let yaw_rot = Quat::from_rotation_y(orbit.yaw);
        let pitch_rot = Quat::from_rotation_x(orbit.pitch);
        let rotation = yaw_rot * pitch_rot;

        transform.translation = orbit.focus + rotation * Vec3::new(0.0, 0.0, orbit.radius);
        transform.look_at(orbit.focus, Vec3::Y);
    }

    mouse_motion.clear();
}
