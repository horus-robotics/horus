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
            radius: 15.0, // Good distance for overview
            yaw: 0.5,     // Slight angle to see more of the scene
            pitch: 0.6,   // Positive pitch = camera ABOVE ground looking down (spherical coords)
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
        // Calculate camera position on a sphere around the focus point
        // pitch = 0 means camera at same height as focus, positive = above, negative = below
        // yaw = 0 means camera on +Z axis from focus
        let cos_pitch = orbit.pitch.cos();
        let sin_pitch = orbit.pitch.sin();
        let cos_yaw = orbit.yaw.cos();
        let sin_yaw = orbit.yaw.sin();

        // Spherical to Cartesian: camera position relative to focus
        let offset = Vec3::new(
            orbit.radius * cos_pitch * sin_yaw, // X
            orbit.radius * sin_pitch,           // Y (height above focus)
            orbit.radius * cos_pitch * cos_yaw, // Z
        );

        transform.translation = orbit.focus + offset;
        transform.look_at(orbit.focus, Vec3::Y);
    }

    mouse_motion.clear();
}
