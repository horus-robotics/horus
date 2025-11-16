use bevy::prelude::*;
use rapier3d::prelude::*;

#[derive(Component)]
pub struct RigidBodyComponent {
    pub handle: RigidBodyHandle,
}

impl RigidBodyComponent {
    pub fn new(handle: RigidBodyHandle) -> Self {
        Self { handle }
    }
}

/// Contact forces accumulated during physics step
#[derive(Component, Default, Clone, Debug)]
pub struct ContactForce {
    /// Total contact force (N) in world frame
    pub force: Vec3,
    /// Total contact torque (Nm) in world frame about center of mass
    pub torque: Vec3,
    /// Number of active contacts
    pub contact_count: usize,
    /// Contact points in world frame
    pub contact_points: Vec<Vec3>,
    /// Contact normals in world frame
    pub contact_normals: Vec<Vec3>,
}

impl ContactForce {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset accumulated forces (call at start of physics step)
    pub fn reset(&mut self) {
        self.force = Vec3::ZERO;
        self.torque = Vec3::ZERO;
        self.contact_count = 0;
        self.contact_points.clear();
        self.contact_normals.clear();
    }

    /// Add contact force at a point
    pub fn add_contact(&mut self, force: Vec3, point: Vec3, normal: Vec3, center_of_mass: Vec3) {
        self.force += force;
        self.torque += (point - center_of_mass).cross(force);
        self.contact_count += 1;
        self.contact_points.push(point);
        self.contact_normals.push(normal);
    }

    /// Check if object is in contact
    pub fn is_in_contact(&self) -> bool {
        self.contact_count > 0
    }

    /// Get average contact point
    pub fn average_contact_point(&self) -> Option<Vec3> {
        if self.contact_points.is_empty() {
            return None;
        }

        let sum: Vec3 = self.contact_points.iter().sum();
        Some(sum / self.contact_points.len() as f32)
    }

    /// Get total force magnitude
    pub fn force_magnitude(&self) -> f32 {
        self.force.length()
    }

    /// Get total torque magnitude
    pub fn torque_magnitude(&self) -> f32 {
        self.torque.length()
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum RigidBodyType {
    Dynamic,
    Fixed,
    KinematicPositionBased,
    KinematicVelocityBased,
}

impl Default for RigidBodyType {
    fn default() -> Self {
        Self::Dynamic
    }
}

#[derive(Component, Default, Clone)]
pub struct Velocity {
    pub linear: Vec3,
    pub angular: Vec3,
}

impl Velocity {
    pub fn new(linear: Vec3, angular: Vec3) -> Self {
        Self { linear, angular }
    }

    pub fn zero() -> Self {
        Self {
            linear: Vec3::ZERO,
            angular: Vec3::ZERO,
        }
    }
}

#[derive(Component, Default, Clone)]
pub struct ExternalForce {
    pub force: Vec3,
    pub torque: Vec3,
}

impl ExternalForce {
    pub fn new(force: Vec3, torque: Vec3) -> Self {
        Self { force, torque }
    }

    pub fn zero() -> Self {
        Self {
            force: Vec3::ZERO,
            torque: Vec3::ZERO,
        }
    }

    pub fn at_point(force: Vec3, point: Vec3, center_of_mass: Vec3) -> Self {
        let torque = (point - center_of_mass).cross(force);
        Self { force, torque }
    }
}

#[derive(Component, Default, Clone)]
pub struct ExternalImpulse {
    pub impulse: Vec3,
    pub torque_impulse: Vec3,
}

impl ExternalImpulse {
    pub fn new(impulse: Vec3, torque_impulse: Vec3) -> Self {
        Self {
            impulse,
            torque_impulse,
        }
    }

    pub fn zero() -> Self {
        Self {
            impulse: Vec3::ZERO,
            torque_impulse: Vec3::ZERO,
        }
    }

    pub fn at_point(impulse: Vec3, point: Vec3, center_of_mass: Vec3) -> Self {
        let torque_impulse = (point - center_of_mass).cross(impulse);
        Self {
            impulse,
            torque_impulse,
        }
    }
}

#[derive(Component, Clone)]
pub struct Mass {
    pub mass: f32,
}

impl Mass {
    pub fn new(mass: f32) -> Self {
        Self { mass }
    }
}

impl Default for Mass {
    fn default() -> Self {
        Self { mass: 1.0 }
    }
}

#[derive(Component, Clone)]
pub struct Damping {
    pub linear_damping: f32,
    pub angular_damping: f32,
}

impl Damping {
    pub fn new(linear: f32, angular: f32) -> Self {
        Self {
            linear_damping: linear,
            angular_damping: angular,
        }
    }
}

impl Default for Damping {
    fn default() -> Self {
        Self {
            linear_damping: 0.0,
            angular_damping: 0.0,
        }
    }
}

#[derive(Component)]
pub struct Sleeping {
    pub sleeping: bool,
}

impl Sleeping {
    pub fn awake() -> Self {
        Self { sleeping: false }
    }

    pub fn asleep() -> Self {
        Self { sleeping: true }
    }
}

impl Default for Sleeping {
    fn default() -> Self {
        Self::awake()
    }
}

#[derive(Component, Clone)]
pub struct GravityScale {
    pub scale: f32,
}

impl GravityScale {
    pub fn new(scale: f32) -> Self {
        Self { scale }
    }
}

impl Default for GravityScale {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

#[derive(Component)]
pub struct LockedAxes {
    pub locked_axes: u8,
}

impl LockedAxes {
    pub const TRANSLATION_LOCKED_X: u8 = 1 << 0;
    pub const TRANSLATION_LOCKED_Y: u8 = 1 << 1;
    pub const TRANSLATION_LOCKED_Z: u8 = 1 << 2;
    pub const ROTATION_LOCKED_X: u8 = 1 << 3;
    pub const ROTATION_LOCKED_Y: u8 = 1 << 4;
    pub const ROTATION_LOCKED_Z: u8 = 1 << 5;

    pub fn new() -> Self {
        Self { locked_axes: 0 }
    }

    pub fn lock_translation_x(mut self) -> Self {
        self.locked_axes |= Self::TRANSLATION_LOCKED_X;
        self
    }

    pub fn lock_translation_y(mut self) -> Self {
        self.locked_axes |= Self::TRANSLATION_LOCKED_Y;
        self
    }

    pub fn lock_translation_z(mut self) -> Self {
        self.locked_axes |= Self::TRANSLATION_LOCKED_Z;
        self
    }

    pub fn lock_rotation_x(mut self) -> Self {
        self.locked_axes |= Self::ROTATION_LOCKED_X;
        self
    }

    pub fn lock_rotation_y(mut self) -> Self {
        self.locked_axes |= Self::ROTATION_LOCKED_Y;
        self
    }

    pub fn lock_rotation_z(mut self) -> Self {
        self.locked_axes |= Self::ROTATION_LOCKED_Z;
        self
    }

    pub fn lock_all_translation(mut self) -> Self {
        self.locked_axes |= Self::TRANSLATION_LOCKED_X
            | Self::TRANSLATION_LOCKED_Y
            | Self::TRANSLATION_LOCKED_Z;
        self
    }

    pub fn lock_all_rotation(mut self) -> Self {
        self.locked_axes |= Self::ROTATION_LOCKED_X
            | Self::ROTATION_LOCKED_Y
            | Self::ROTATION_LOCKED_Z;
        self
    }
}

impl Default for LockedAxes {
    fn default() -> Self {
        Self::new()
    }
}
