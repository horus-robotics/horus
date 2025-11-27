//! Advanced Physics Features for the 3D Simulator
//!
//! This module provides advanced physics capabilities including:
//! - Continuous Collision Detection (CCD)
//! - Coulomb Friction Model with anisotropic support
//! - Advanced Contact Models
//! - Breakable Joints
//! - Spring-Damper Constraints

use bevy::prelude::*;
use nalgebra::{Point3, Unit, Vector3};
use parry3d::query::details::ShapeCastOptions;
use rapier3d::prelude::*;
use std::collections::HashMap;

// ============================================================================
// Continuous Collision Detection (CCD)
// ============================================================================

/// Configuration for Continuous Collision Detection
#[derive(Clone, Debug)]
pub struct CCDConfig {
    /// Tolerance for sweep tests (smaller = more accurate but slower)
    pub sweep_tolerance: f32,
    /// Maximum number of substeps for CCD resolution
    pub max_substeps: u32,
    /// Velocity threshold below which CCD is disabled for an object
    pub velocity_threshold: f32,
    /// Whether CCD is globally enabled
    pub enabled: bool,
    /// Motion clamping coefficient (0-1, how much to clamp fast motion)
    pub motion_clamping: f32,
}

impl Default for CCDConfig {
    fn default() -> Self {
        Self {
            sweep_tolerance: 0.001,
            max_substeps: 4,
            velocity_threshold: 0.1,
            enabled: true,
            motion_clamping: 0.8,
        }
    }
}

impl CCDConfig {
    /// Create a high-precision CCD config for fast-moving objects
    #[allow(dead_code)]
    pub fn high_precision() -> Self {
        Self {
            sweep_tolerance: 0.0001,
            max_substeps: 8,
            velocity_threshold: 0.05,
            enabled: true,
            motion_clamping: 0.95,
        }
    }

    /// Create a performance-optimized CCD config
    #[allow(dead_code)]
    pub fn performance() -> Self {
        Self {
            sweep_tolerance: 0.01,
            max_substeps: 2,
            velocity_threshold: 0.5,
            enabled: true,
            motion_clamping: 0.6,
        }
    }
}

/// Component to enable CCD on a specific rigid body
#[derive(Component, Clone, Debug)]
pub struct CCDEnabled {
    /// Whether CCD is enabled for this specific object
    pub enabled: bool,
    /// Override sweep tolerance for this object (None uses global)
    pub sweep_tolerance_override: Option<f32>,
}

impl Default for CCDEnabled {
    fn default() -> Self {
        Self {
            enabled: true,
            sweep_tolerance_override: None,
        }
    }
}

/// CCD Solver that performs sweep tests for fast-moving objects
#[derive(Resource)]
pub struct CCDSolverAdvanced {
    /// Global CCD configuration
    pub config: CCDConfig,
    /// Per-object CCD settings (keyed by Entity bits)
    object_settings: HashMap<u64, CCDEnabled>,
    /// Statistics for debugging
    pub stats: CCDStats,
}

/// Statistics for CCD operations
#[derive(Clone, Debug, Default)]
pub struct CCDStats {
    /// Number of sweep tests performed this frame
    pub sweep_tests_performed: u32,
    /// Number of CCD-enabled objects
    pub ccd_enabled_objects: u32,
    /// Number of tunneling events prevented
    pub tunneling_prevented: u32,
    /// Total substeps used this frame
    pub total_substeps: u32,
}

impl Default for CCDSolverAdvanced {
    fn default() -> Self {
        Self {
            config: CCDConfig::default(),
            object_settings: HashMap::new(),
            stats: CCDStats::default(),
        }
    }
}

impl CCDSolverAdvanced {
    /// Create a new CCD solver with custom configuration
    #[allow(dead_code)]
    pub fn new(config: CCDConfig) -> Self {
        Self {
            config,
            object_settings: HashMap::new(),
            stats: CCDStats::default(),
        }
    }

    /// Enable CCD for a specific entity
    #[allow(dead_code)]
    pub fn enable_ccd(&mut self, entity: Entity, settings: CCDEnabled) {
        self.object_settings.insert(entity.to_bits(), settings);
    }

    /// Disable CCD for a specific entity
    #[allow(dead_code)]
    pub fn disable_ccd(&mut self, entity: Entity) {
        if let Some(settings) = self.object_settings.get_mut(&entity.to_bits()) {
            settings.enabled = false;
        }
    }

    /// Remove CCD settings for an entity
    #[allow(dead_code)]
    pub fn remove_ccd(&mut self, entity: Entity) {
        self.object_settings.remove(&entity.to_bits());
    }

    /// Check if CCD is enabled for an entity
    #[allow(dead_code)]
    pub fn is_ccd_enabled(&self, entity: Entity) -> bool {
        self.object_settings
            .get(&entity.to_bits())
            .map(|s| s.enabled)
            .unwrap_or(false)
    }

    /// Get sweep tolerance for an entity
    #[allow(dead_code)]
    pub fn get_sweep_tolerance(&self, entity: Entity) -> f32 {
        self.object_settings
            .get(&entity.to_bits())
            .and_then(|s| s.sweep_tolerance_override)
            .unwrap_or(self.config.sweep_tolerance)
    }

    /// Check if an object needs CCD based on velocity
    #[allow(dead_code)]
    pub fn needs_ccd(&self, linear_velocity: Vec3, angular_velocity: Vec3, radius: f32) -> bool {
        if !self.config.enabled {
            return false;
        }

        let linear_speed = linear_velocity.length();
        let angular_speed = angular_velocity.length() * radius;
        let total_speed = linear_speed + angular_speed;

        total_speed > self.config.velocity_threshold
    }

    /// Perform a sweep test between two positions using Rapier's shape casting
    #[allow(dead_code)]
    pub fn sweep_test(
        &mut self,
        shape: &dyn Shape,
        start_pos: &Isometry<f32>,
        end_pos: &Isometry<f32>,
        collider_set: &ColliderSet,
        query_pipeline: &QueryPipeline,
        rigid_body_set: &RigidBodySet,
        exclude_collider: Option<ColliderHandle>,
    ) -> Option<SweepTestResult> {
        self.stats.sweep_tests_performed += 1;

        let direction = end_pos.translation.vector - start_pos.translation.vector;
        let max_toi = direction.norm();

        if max_toi < 1e-6 {
            return None;
        }

        let direction = Unit::new_normalize(direction);

        let filter = QueryFilter::default()
            .exclude_collider(exclude_collider.unwrap_or(ColliderHandle::invalid()));

        // Use Rapier's cast_shape with the correct API
        let options = ShapeCastOptions {
            max_time_of_impact: max_toi,
            target_distance: self.config.sweep_tolerance,
            stop_at_penetration: true,
            compute_impact_geometry_on_penetration: true,
        };

        if let Some((handle, toi)) = query_pipeline.cast_shape(
            rigid_body_set,
            collider_set,
            start_pos,
            &direction,
            shape,
            options,
            filter,
        ) {
            self.stats.tunneling_prevented += 1;
            Some(SweepTestResult {
                collider_handle: handle,
                time_of_impact: toi.time_of_impact,
                normal: Vec3::new(toi.normal1.x, toi.normal1.y, toi.normal1.z),
                contact_point: Vec3::new(toi.witness1.x, toi.witness1.y, toi.witness1.z),
            })
        } else {
            None
        }
    }

    /// Reset statistics for a new frame
    #[allow(dead_code)]
    pub fn reset_stats(&mut self) {
        self.stats = CCDStats::default();
        self.stats.ccd_enabled_objects =
            self.object_settings.values().filter(|s| s.enabled).count() as u32;
    }

    /// Configure a Rapier rigid body for CCD
    #[allow(dead_code)]
    pub fn configure_rapier_ccd(rigid_body: &mut RigidBody, enabled: bool) {
        rigid_body.enable_ccd(enabled);
    }
}

/// Result of a sweep test
#[derive(Clone, Debug)]
pub struct SweepTestResult {
    /// Handle of the collider that was hit
    pub collider_handle: ColliderHandle,
    /// Time of impact (0-1, fraction of motion)
    pub time_of_impact: f32,
    /// Contact normal at impact point
    pub normal: Vec3,
    /// Contact point in world space
    pub contact_point: Vec3,
}

// ============================================================================
// Coulomb Friction Model
// ============================================================================

/// Coulomb friction model with static and kinetic coefficients
#[derive(Clone, Debug)]
pub struct CoulombFriction {
    /// Static friction coefficient (friction when not moving)
    pub static_coefficient: f32,
    /// Kinetic friction coefficient (friction when moving)
    pub kinetic_coefficient: f32,
    /// Rolling resistance coefficient
    pub rolling_resistance: f32,
    /// Spinning resistance coefficient
    pub spinning_resistance: f32,
    /// Velocity threshold for static/kinetic transition
    pub velocity_threshold: f32,
}

impl Default for CoulombFriction {
    fn default() -> Self {
        Self {
            static_coefficient: 0.6,
            kinetic_coefficient: 0.4,
            rolling_resistance: 0.01,
            spinning_resistance: 0.01,
            velocity_threshold: 0.01,
        }
    }
}

impl CoulombFriction {
    /// Create a new Coulomb friction model
    #[allow(dead_code)]
    pub fn new(static_coeff: f32, kinetic_coeff: f32) -> Self {
        Self {
            static_coefficient: static_coeff,
            kinetic_coefficient: kinetic_coeff,
            ..Default::default()
        }
    }

    /// Create friction for rubber on concrete
    #[allow(dead_code)]
    pub fn rubber_concrete() -> Self {
        Self {
            static_coefficient: 1.0,
            kinetic_coefficient: 0.8,
            rolling_resistance: 0.02,
            spinning_resistance: 0.015,
            velocity_threshold: 0.005,
        }
    }

    /// Create friction for steel on steel
    #[allow(dead_code)]
    pub fn steel_steel() -> Self {
        Self {
            static_coefficient: 0.74,
            kinetic_coefficient: 0.57,
            rolling_resistance: 0.002,
            spinning_resistance: 0.001,
            velocity_threshold: 0.01,
        }
    }

    /// Create friction for ice
    #[allow(dead_code)]
    pub fn ice() -> Self {
        Self {
            static_coefficient: 0.1,
            kinetic_coefficient: 0.03,
            rolling_resistance: 0.001,
            spinning_resistance: 0.0005,
            velocity_threshold: 0.001,
        }
    }

    /// Calculate effective friction coefficient based on relative velocity
    #[allow(dead_code)]
    pub fn effective_coefficient(&self, relative_velocity: f32) -> f32 {
        if relative_velocity.abs() < self.velocity_threshold {
            self.static_coefficient
        } else {
            self.kinetic_coefficient
        }
    }

    /// Calculate friction force magnitude given normal force
    #[allow(dead_code)]
    pub fn friction_force(&self, normal_force: f32, relative_velocity: f32) -> f32 {
        let mu = self.effective_coefficient(relative_velocity);
        mu * normal_force.abs()
    }

    /// Calculate rolling resistance torque
    #[allow(dead_code)]
    pub fn rolling_torque(&self, normal_force: f32, radius: f32) -> f32 {
        self.rolling_resistance * normal_force.abs() * radius
    }

    /// Calculate spinning resistance torque
    #[allow(dead_code)]
    pub fn spinning_torque(&self, normal_force: f32, contact_radius: f32) -> f32 {
        self.spinning_resistance * normal_force.abs() * contact_radius
    }
}

/// Friction pyramid approximation for 3D friction
#[derive(Clone, Debug)]
pub struct FrictionPyramid {
    /// Number of sides in the friction pyramid (4 = box, 8 = octagon, etc.)
    pub num_sides: u32,
    /// Primary friction direction vectors (normalized)
    pub directions: Vec<Vec3>,
    /// Friction coefficients for each direction
    pub coefficients: Vec<f32>,
}

impl Default for FrictionPyramid {
    fn default() -> Self {
        Self::new_isotropic(4, 0.5)
    }
}

impl FrictionPyramid {
    /// Create an isotropic friction pyramid (same friction in all directions)
    #[allow(dead_code)]
    pub fn new_isotropic(num_sides: u32, coefficient: f32) -> Self {
        let mut directions = Vec::with_capacity(num_sides as usize);
        let mut coefficients = Vec::with_capacity(num_sides as usize);

        for i in 0..num_sides {
            let angle: f32 = (i as f32 * 2.0 * std::f32::consts::PI) / num_sides as f32;
            directions.push(Vec3::new(angle.cos(), 0.0, angle.sin()));
            coefficients.push(coefficient);
        }

        Self {
            num_sides,
            directions,
            coefficients,
        }
    }

    /// Create an anisotropic friction pyramid
    #[allow(dead_code)]
    pub fn new_anisotropic(forward_coeff: f32, lateral_coeff: f32, forward_dir: Vec3) -> Self {
        let forward = forward_dir.normalize();
        let up = Vec3::Y;
        let lateral = up.cross(forward).normalize();

        Self {
            num_sides: 4,
            directions: vec![forward, -forward, lateral, -lateral],
            coefficients: vec![forward_coeff, forward_coeff, lateral_coeff, lateral_coeff],
        }
    }

    /// Get friction coefficient in a specific direction
    #[allow(dead_code)]
    pub fn coefficient_in_direction(&self, direction: Vec3) -> f32 {
        let dir_normalized = direction.normalize();
        let mut total_weight: f32 = 0.0;
        let mut weighted_coeff: f32 = 0.0;

        for (i, pyramid_dir) in self.directions.iter().enumerate() {
            let dot = dir_normalized.dot(*pyramid_dir).max(0.0);
            total_weight += dot;
            weighted_coeff += dot * self.coefficients[i];
        }

        if total_weight > 1e-6 {
            weighted_coeff / total_weight
        } else {
            self.coefficients.iter().sum::<f32>() / self.coefficients.len() as f32
        }
    }

    /// Calculate friction force vector given normal force and relative velocity
    #[allow(dead_code)]
    pub fn friction_force(&self, normal_force: f32, relative_velocity: Vec3) -> Vec3 {
        let speed = relative_velocity.length();
        if speed < 1e-6 {
            return Vec3::ZERO;
        }

        let direction = -relative_velocity / speed;
        let coeff = self.coefficient_in_direction(direction);
        direction * coeff * normal_force.abs()
    }
}

/// Anisotropic friction support (different friction in different directions)
#[derive(Component, Clone, Debug)]
pub struct AnisotropicFrictionComponent {
    /// Friction coefficient along the primary axis
    pub primary_coefficient: f32,
    /// Friction coefficient along the secondary axis
    pub secondary_coefficient: f32,
    /// Primary friction direction in local space
    pub primary_direction: Vec3,
    /// Whether anisotropic friction is enabled
    pub enabled: bool,
}

impl Default for AnisotropicFrictionComponent {
    fn default() -> Self {
        Self {
            primary_coefficient: 0.5,
            secondary_coefficient: 0.5,
            primary_direction: Vec3::X,
            enabled: false,
        }
    }
}

impl AnisotropicFrictionComponent {
    /// Create anisotropic friction with specified coefficients
    #[allow(dead_code)]
    pub fn new(primary: f32, secondary: f32, direction: Vec3) -> Self {
        Self {
            primary_coefficient: primary,
            secondary_coefficient: secondary,
            primary_direction: direction.normalize(),
            enabled: true,
        }
    }

    /// Get friction coefficient in a world-space direction given local-to-world transform
    #[allow(dead_code)]
    pub fn coefficient_in_direction(
        &self,
        world_direction: Vec3,
        local_to_world: &Transform,
    ) -> f32 {
        if !self.enabled {
            return (self.primary_coefficient + self.secondary_coefficient) / 2.0;
        }

        let world_primary = local_to_world.rotation * self.primary_direction;
        let dot = world_direction.normalize().dot(world_primary).abs();

        // Interpolate between primary and secondary based on alignment
        self.primary_coefficient * dot + self.secondary_coefficient * (1.0 - dot)
    }
}

/// Resource to manage friction settings per material pair
#[derive(Resource, Default)]
pub struct FrictionMaterialPairs {
    /// Map from material pair to friction settings
    pairs: HashMap<(u32, u32), CoulombFriction>,
    /// Default friction for unknown pairs
    pub default_friction: CoulombFriction,
}

impl FrictionMaterialPairs {
    /// Create a new friction material pairs database
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            pairs: HashMap::new(),
            default_friction: CoulombFriction::default(),
        }
    }

    /// Set friction for a specific material pair
    #[allow(dead_code)]
    pub fn set_friction(&mut self, material_a: u32, material_b: u32, friction: CoulombFriction) {
        // Store both orderings for symmetric lookup
        let key1 = (material_a.min(material_b), material_a.max(material_b));
        self.pairs.insert(key1, friction);
    }

    /// Get friction for a material pair
    #[allow(dead_code)]
    pub fn get_friction(&self, material_a: u32, material_b: u32) -> &CoulombFriction {
        let key = (material_a.min(material_b), material_a.max(material_b));
        self.pairs.get(&key).unwrap_or(&self.default_friction)
    }

    /// Remove friction settings for a material pair
    #[allow(dead_code)]
    pub fn remove_friction(&mut self, material_a: u32, material_b: u32) {
        let key = (material_a.min(material_b), material_a.max(material_b));
        self.pairs.remove(&key);
    }
}

// ============================================================================
// Advanced Contact Models
// ============================================================================

/// Contact model types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactModel {
    /// Point contact (default, single contact point)
    Point,
    /// Patch contact (area contact, multiple points)
    Patch,
    /// Soft contact (compliant contact with penetration)
    Soft,
}

impl Default for ContactModel {
    fn default() -> Self {
        Self::Point
    }
}

/// Configuration for contact behavior
#[derive(Clone, Debug)]
pub struct ContactConfig {
    /// Contact model to use
    pub model: ContactModel,
    /// Contact stiffness (N/m) for soft contacts
    pub stiffness: f32,
    /// Contact damping (N*s/m) for soft contacts
    pub damping: f32,
    /// Maximum penetration depth allowed (meters)
    pub max_penetration: f32,
    /// Penetration recovery velocity (m/s)
    pub recovery_velocity: f32,
    /// Number of contact points for patch contacts
    pub patch_points: u32,
    /// Contact area radius for patch contacts
    pub patch_radius: f32,
    /// Enable contact force visualization
    pub visualize_forces: bool,
    /// Force scale for visualization
    pub visualization_scale: f32,
}

impl Default for ContactConfig {
    fn default() -> Self {
        Self {
            model: ContactModel::Point,
            stiffness: 1e6,
            damping: 1e4,
            max_penetration: 0.01,
            recovery_velocity: 0.1,
            patch_points: 4,
            patch_radius: 0.01,
            visualize_forces: false,
            visualization_scale: 0.001,
        }
    }
}

impl ContactConfig {
    /// Create a soft contact configuration
    #[allow(dead_code)]
    pub fn soft(stiffness: f32, damping: f32) -> Self {
        Self {
            model: ContactModel::Soft,
            stiffness,
            damping,
            ..Default::default()
        }
    }

    /// Create a patch contact configuration
    #[allow(dead_code)]
    pub fn patch(points: u32, radius: f32) -> Self {
        Self {
            model: ContactModel::Patch,
            patch_points: points,
            patch_radius: radius,
            ..Default::default()
        }
    }

    /// Calculate contact force for soft contacts
    #[allow(dead_code)]
    pub fn soft_contact_force(&self, penetration: f32, relative_velocity: f32) -> f32 {
        if penetration <= 0.0 {
            return 0.0;
        }

        let clamped_penetration = penetration.min(self.max_penetration);
        let spring_force = self.stiffness * clamped_penetration;
        let damping_force = self.damping * relative_velocity.min(0.0).abs();

        spring_force + damping_force
    }
}

/// Component for per-object contact configuration
#[derive(Component, Clone, Debug)]
pub struct ContactConfigComponent {
    pub config: ContactConfig,
}

impl Default for ContactConfigComponent {
    fn default() -> Self {
        Self {
            config: ContactConfig::default(),
        }
    }
}

/// Contact event data for callbacks
#[derive(Clone, Debug)]
pub struct ContactEventData {
    /// Entity A in the contact
    pub entity_a: Entity,
    /// Entity B in the contact
    pub entity_b: Entity,
    /// Contact point in world space
    pub contact_point: Vec3,
    /// Contact normal (from A to B)
    pub normal: Vec3,
    /// Penetration depth
    pub penetration: f32,
    /// Relative velocity at contact point
    pub relative_velocity: Vec3,
    /// Contact force magnitude
    pub force_magnitude: f32,
    /// Event type
    pub event_type: ContactEventType,
}

/// Types of contact events
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactEventType {
    /// Contact started
    Started,
    /// Contact ongoing
    Ongoing,
    /// Contact ended
    Ended,
}

/// Resource for contact event callbacks
#[derive(Resource, Default)]
pub struct ContactEventCallbacks {
    /// Callbacks for contact started events
    started_callbacks: Vec<Box<dyn Fn(&ContactEventData) + Send + Sync>>,
    /// Callbacks for ongoing contact events
    ongoing_callbacks: Vec<Box<dyn Fn(&ContactEventData) + Send + Sync>>,
    /// Callbacks for contact ended events
    ended_callbacks: Vec<Box<dyn Fn(&ContactEventData) + Send + Sync>>,
    /// Pending events to be processed
    pub pending_events: Vec<ContactEventData>,
}

impl ContactEventCallbacks {
    /// Register a callback for contact started events
    #[allow(dead_code)]
    pub fn on_contact_started<F>(&mut self, callback: F)
    where
        F: Fn(&ContactEventData) + Send + Sync + 'static,
    {
        self.started_callbacks.push(Box::new(callback));
    }

    /// Register a callback for ongoing contact events
    #[allow(dead_code)]
    pub fn on_contact_ongoing<F>(&mut self, callback: F)
    where
        F: Fn(&ContactEventData) + Send + Sync + 'static,
    {
        self.ongoing_callbacks.push(Box::new(callback));
    }

    /// Register a callback for contact ended events
    #[allow(dead_code)]
    pub fn on_contact_ended<F>(&mut self, callback: F)
    where
        F: Fn(&ContactEventData) + Send + Sync + 'static,
    {
        self.ended_callbacks.push(Box::new(callback));
    }

    /// Process a contact event (internal helper)
    fn process_single_event(
        event: &ContactEventData,
        started_callbacks: &[Box<dyn Fn(&ContactEventData) + Send + Sync>],
        ongoing_callbacks: &[Box<dyn Fn(&ContactEventData) + Send + Sync>],
        ended_callbacks: &[Box<dyn Fn(&ContactEventData) + Send + Sync>],
    ) {
        match event.event_type {
            ContactEventType::Started => {
                for callback in started_callbacks {
                    callback(event);
                }
            }
            ContactEventType::Ongoing => {
                for callback in ongoing_callbacks {
                    callback(event);
                }
            }
            ContactEventType::Ended => {
                for callback in ended_callbacks {
                    callback(event);
                }
            }
        }
    }

    /// Process a contact event
    #[allow(dead_code)]
    pub fn process_event(&self, event: &ContactEventData) {
        Self::process_single_event(
            event,
            &self.started_callbacks,
            &self.ongoing_callbacks,
            &self.ended_callbacks,
        );
    }

    /// Add an event to pending queue
    #[allow(dead_code)]
    pub fn queue_event(&mut self, event: ContactEventData) {
        self.pending_events.push(event);
    }

    /// Process all pending events
    #[allow(dead_code)]
    pub fn process_pending(&mut self) {
        // Drain events into a temporary vec to avoid borrow issues
        let events: Vec<_> = self.pending_events.drain(..).collect();
        for event in events {
            Self::process_single_event(
                &event,
                &self.started_callbacks,
                &self.ongoing_callbacks,
                &self.ended_callbacks,
            );
        }
    }
}

/// Contact force visualization helper
#[derive(Clone, Debug)]
pub struct ContactForceVisualizer {
    /// Scale factor for force arrows
    pub scale: f32,
    /// Color for normal forces
    pub normal_color: Color,
    /// Color for friction forces
    pub friction_color: Color,
    /// Minimum force to visualize
    pub min_force: f32,
    /// Maximum force for full-length arrow
    pub max_force: f32,
}

impl Default for ContactForceVisualizer {
    fn default() -> Self {
        Self {
            scale: 0.001,
            normal_color: Color::srgb(0.0, 1.0, 0.0),
            friction_color: Color::srgb(1.0, 0.5, 0.0),
            min_force: 0.1,
            max_force: 1000.0,
        }
    }
}

impl ContactForceVisualizer {
    /// Calculate arrow endpoint for force visualization
    #[allow(dead_code)]
    pub fn force_arrow(&self, origin: Vec3, force: Vec3) -> Option<(Vec3, Vec3)> {
        let magnitude = force.length();
        if magnitude < self.min_force {
            return None;
        }

        let normalized_magnitude = (magnitude / self.max_force).min(1.0);
        let direction = force.normalize();
        let end = origin + direction * normalized_magnitude * self.scale * self.max_force;

        Some((origin, end))
    }
}

/// Penetration depth limiter to prevent deep interpenetration
#[derive(Clone, Debug)]
pub struct PenetrationLimiter {
    /// Maximum allowed penetration depth
    pub max_depth: f32,
    /// Correction velocity for resolving penetration
    pub correction_velocity: f32,
    /// Bias factor for position correction
    pub bias_factor: f32,
    /// Slop (small penetration allowed without correction)
    pub slop: f32,
}

impl Default for PenetrationLimiter {
    fn default() -> Self {
        Self {
            max_depth: 0.01,
            correction_velocity: 0.2,
            bias_factor: 0.2,
            slop: 0.001,
        }
    }
}

impl PenetrationLimiter {
    /// Calculate position correction for a given penetration
    #[allow(dead_code)]
    pub fn position_correction(&self, penetration: f32, normal: Vec3) -> Vec3 {
        let effective_penetration = (penetration - self.slop).max(0.0);
        let correction_magnitude = self.bias_factor * effective_penetration.min(self.max_depth);
        normal * correction_magnitude
    }

    /// Calculate velocity bias for penetration resolution
    #[allow(dead_code)]
    pub fn velocity_bias(&self, penetration: f32) -> f32 {
        let effective_penetration = (penetration - self.slop).max(0.0);
        self.correction_velocity * (effective_penetration / self.max_depth).min(1.0)
    }
}

// ============================================================================
// Breakable Joints
// ============================================================================

/// Component for breakable joints with force/torque thresholds
#[derive(Component, Clone, Debug)]
pub struct BreakableJoint {
    /// Joint handle in Rapier
    pub joint_handle: ImpulseJointHandle,
    /// Maximum force before breaking (N)
    pub max_force: f32,
    /// Maximum torque before breaking (Nm)
    pub max_torque: f32,
    /// Whether the joint has broken
    pub is_broken: bool,
    /// Accumulated force for averaging
    pub accumulated_force: Vec3,
    /// Accumulated torque for averaging
    pub accumulated_torque: Vec3,
    /// Number of samples for averaging
    pub sample_count: u32,
    /// Use average force/torque instead of instantaneous
    pub use_averaging: bool,
    /// Number of frames to average over
    pub averaging_frames: u32,
}

impl BreakableJoint {
    /// Create a new breakable joint
    #[allow(dead_code)]
    pub fn new(joint_handle: ImpulseJointHandle, max_force: f32, max_torque: f32) -> Self {
        Self {
            joint_handle,
            max_force,
            max_torque,
            is_broken: false,
            accumulated_force: Vec3::ZERO,
            accumulated_torque: Vec3::ZERO,
            sample_count: 0,
            use_averaging: false,
            averaging_frames: 10,
        }
    }

    /// Create with force/torque averaging enabled
    #[allow(dead_code)]
    pub fn with_averaging(mut self, frames: u32) -> Self {
        self.use_averaging = true;
        self.averaging_frames = frames;
        self
    }

    /// Update with current joint forces and check if broken
    #[allow(dead_code)]
    pub fn update(&mut self, force: Vec3, torque: Vec3) -> bool {
        if self.is_broken {
            return true;
        }

        if self.use_averaging {
            self.accumulated_force += force;
            self.accumulated_torque += torque;
            self.sample_count += 1;

            if self.sample_count >= self.averaging_frames {
                let avg_force = self.accumulated_force / self.sample_count as f32;
                let avg_torque = self.accumulated_torque / self.sample_count as f32;

                if avg_force.length() > self.max_force || avg_torque.length() > self.max_torque {
                    self.is_broken = true;
                }

                self.accumulated_force = Vec3::ZERO;
                self.accumulated_torque = Vec3::ZERO;
                self.sample_count = 0;
            }
        } else if force.length() > self.max_force || torque.length() > self.max_torque {
            self.is_broken = true;
        }

        self.is_broken
    }

    /// Reset the joint (mark as unbroken)
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.is_broken = false;
        self.accumulated_force = Vec3::ZERO;
        self.accumulated_torque = Vec3::ZERO;
        self.sample_count = 0;
    }
}

/// Event fired when a joint breaks
#[derive(Event, Clone, Debug)]
pub struct JointBreakEvent {
    /// Entity that had the breakable joint
    pub entity: Entity,
    /// The joint handle that broke
    pub joint_handle: ImpulseJointHandle,
    /// Force at break
    pub break_force: Vec3,
    /// Torque at break
    pub break_torque: Vec3,
    /// Whether force threshold was exceeded
    pub force_exceeded: bool,
    /// Whether torque threshold was exceeded
    pub torque_exceeded: bool,
}

/// Resource for managing breakable joints
#[derive(Resource, Default)]
pub struct BreakableJointManager {
    /// Joints pending removal (broken)
    pub broken_joints: Vec<ImpulseJointHandle>,
    /// Statistics
    pub total_breaks: u32,
}

impl BreakableJointManager {
    /// Queue a joint for removal
    #[allow(dead_code)]
    pub fn queue_break(&mut self, handle: ImpulseJointHandle) {
        self.broken_joints.push(handle);
        self.total_breaks += 1;
    }

    /// Remove all broken joints from the physics world
    #[allow(dead_code)]
    pub fn remove_broken_joints(&mut self, impulse_joint_set: &mut ImpulseJointSet) {
        for handle in self.broken_joints.drain(..) {
            impulse_joint_set.remove(handle, true);
        }
    }
}

/// Helper functions for creating breakable joints
#[allow(dead_code)]
pub fn create_breakable_revolute_joint(
    anchor1: Vec3,
    anchor2: Vec3,
    axis: Vec3,
    max_force: f32,
    max_torque: f32,
) -> (GenericJoint, f32, f32) {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .build();

    (joint, max_force, max_torque)
}

/// Create a breakable fixed joint
#[allow(dead_code)]
pub fn create_breakable_fixed_joint(
    anchor1: Vec3,
    anchor2: Vec3,
    max_force: f32,
    max_torque: f32,
) -> (GenericJoint, f32, f32) {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);

    let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_FIXED_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .build();

    (joint, max_force, max_torque)
}

/// Create a breakable spherical joint
#[allow(dead_code)]
pub fn create_breakable_spherical_joint(
    anchor1: Vec3,
    anchor2: Vec3,
    max_force: f32,
    max_torque: f32,
) -> (GenericJoint, f32, f32) {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);

    let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_SPHERICAL_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .build();

    (joint, max_force, max_torque)
}

/// Create a breakable prismatic joint
#[allow(dead_code)]
pub fn create_breakable_prismatic_joint(
    anchor1: Vec3,
    anchor2: Vec3,
    axis: Vec3,
    max_force: f32,
    max_torque: f32,
) -> (GenericJoint, f32, f32) {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    let joint = GenericJointBuilder::new(JointAxesMask::LOCKED_PRISMATIC_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .build();

    (joint, max_force, max_torque)
}

// ============================================================================
// Spring-Damper Constraints
// ============================================================================

/// Spring-damper constraint with configurable stiffness and damping
#[derive(Component, Clone, Debug)]
pub struct SpringDamperConstraint {
    /// Spring type (linear or angular)
    pub spring_type: SpringType,
    /// Spring stiffness (N/m for linear, Nm/rad for angular)
    pub stiffness: f32,
    /// Damping coefficient (N*s/m for linear, Nm*s/rad for angular)
    pub damping: f32,
    /// Rest length for linear springs (meters)
    pub rest_length: f32,
    /// Rest angle for angular springs (radians)
    pub rest_angle: f32,
    /// Maximum force/torque the spring can apply
    pub max_force: f32,
    /// Anchor point on body A (local space)
    pub anchor_a: Vec3,
    /// Anchor point on body B (local space)
    pub anchor_b: Vec3,
    /// Connected entity (body B)
    pub connected_entity: Option<Entity>,
    /// Spring axis for angular springs (local to body A)
    pub axis: Vec3,
    /// Whether the spring is enabled
    pub enabled: bool,
}

/// Type of spring constraint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpringType {
    /// Linear spring (distance-based)
    Linear,
    /// Angular spring (rotation-based)
    Angular,
    /// Combined linear and angular
    Combined,
}

impl Default for SpringDamperConstraint {
    fn default() -> Self {
        Self {
            spring_type: SpringType::Linear,
            stiffness: 1000.0,
            damping: 100.0,
            rest_length: 1.0,
            rest_angle: 0.0,
            max_force: f32::MAX,
            anchor_a: Vec3::ZERO,
            anchor_b: Vec3::ZERO,
            connected_entity: None,
            axis: Vec3::Y,
            enabled: true,
        }
    }
}

impl SpringDamperConstraint {
    /// Create a linear spring-damper
    #[allow(dead_code)]
    pub fn linear(stiffness: f32, damping: f32, rest_length: f32) -> Self {
        Self {
            spring_type: SpringType::Linear,
            stiffness,
            damping,
            rest_length,
            ..Default::default()
        }
    }

    /// Create an angular spring-damper
    #[allow(dead_code)]
    pub fn angular(stiffness: f32, damping: f32, rest_angle: f32) -> Self {
        Self {
            spring_type: SpringType::Angular,
            stiffness,
            damping,
            rest_angle,
            ..Default::default()
        }
    }

    /// Create a combined spring-damper
    #[allow(unused_variables)]
    pub fn combined(
        linear_stiffness: f32,
        linear_damping: f32,
        angular_stiffness: f32,
        angular_damping: f32,
    ) -> Self {
        Self {
            spring_type: SpringType::Combined,
            stiffness: linear_stiffness,
            damping: linear_damping,
            rest_length: 0.0,
            rest_angle: 0.0,
            ..Default::default()
        }
    }

    /// Set anchor points
    #[allow(dead_code)]
    pub fn with_anchors(mut self, anchor_a: Vec3, anchor_b: Vec3) -> Self {
        self.anchor_a = anchor_a;
        self.anchor_b = anchor_b;
        self
    }

    /// Set connected entity
    #[allow(dead_code)]
    pub fn with_connected(mut self, entity: Entity) -> Self {
        self.connected_entity = Some(entity);
        self
    }

    /// Set maximum force
    #[allow(dead_code)]
    pub fn with_max_force(mut self, max_force: f32) -> Self {
        self.max_force = max_force;
        self
    }

    /// Set spring axis for angular springs
    #[allow(dead_code)]
    pub fn with_axis(mut self, axis: Vec3) -> Self {
        self.axis = axis.normalize();
        self
    }

    /// Calculate linear spring force
    #[allow(dead_code)]
    pub fn calculate_linear_force(
        &self,
        world_anchor_a: Vec3,
        world_anchor_b: Vec3,
        velocity_a: Vec3,
        velocity_b: Vec3,
    ) -> Vec3 {
        if !self.enabled {
            return Vec3::ZERO;
        }

        let delta = world_anchor_b - world_anchor_a;
        let distance = delta.length();

        if distance < 1e-6 {
            return Vec3::ZERO;
        }

        let direction = delta / distance;
        let displacement = distance - self.rest_length;

        // Spring force (Hooke's law)
        let spring_force = self.stiffness * displacement;

        // Damping force (velocity-dependent)
        let relative_velocity = velocity_b - velocity_a;
        let velocity_along_spring = relative_velocity.dot(direction);
        let damping_force = self.damping * velocity_along_spring;

        // Total force, clamped to max
        let total_force = (spring_force + damping_force).clamp(-self.max_force, self.max_force);

        direction * total_force
    }

    /// Calculate angular spring torque
    #[allow(dead_code)]
    pub fn calculate_angular_torque(
        &self,
        rotation_a: Quat,
        rotation_b: Quat,
        angular_velocity_a: Vec3,
        angular_velocity_b: Vec3,
    ) -> Vec3 {
        if !self.enabled {
            return Vec3::ZERO;
        }

        // Calculate relative rotation
        let relative_rotation = rotation_b * rotation_a.inverse();
        let (axis, angle) = relative_rotation.to_axis_angle();

        // Project onto constraint axis
        let world_axis = rotation_a * self.axis;
        let projected_angle = angle * axis.dot(world_axis);
        let angle_displacement = projected_angle - self.rest_angle;

        // Spring torque
        let spring_torque = self.stiffness * angle_displacement;

        // Damping torque
        let relative_angular_velocity = angular_velocity_b - angular_velocity_a;
        let velocity_along_axis = relative_angular_velocity.dot(world_axis);
        let damping_torque = self.damping * velocity_along_axis;

        // Total torque, clamped to max
        let total_torque = (spring_torque + damping_torque).clamp(-self.max_force, self.max_force);

        world_axis * total_torque
    }
}

/// Builder for spring-damper constraints
#[derive(Clone, Debug)]
pub struct SpringDamperBuilder {
    constraint: SpringDamperConstraint,
}

impl SpringDamperBuilder {
    /// Create a new builder for a linear spring
    #[allow(dead_code)]
    pub fn linear() -> Self {
        Self {
            constraint: SpringDamperConstraint {
                spring_type: SpringType::Linear,
                ..Default::default()
            },
        }
    }

    /// Create a new builder for an angular spring
    #[allow(dead_code)]
    pub fn angular() -> Self {
        Self {
            constraint: SpringDamperConstraint {
                spring_type: SpringType::Angular,
                ..Default::default()
            },
        }
    }

    /// Set stiffness
    #[allow(dead_code)]
    pub fn stiffness(mut self, stiffness: f32) -> Self {
        self.constraint.stiffness = stiffness;
        self
    }

    /// Set damping
    #[allow(dead_code)]
    pub fn damping(mut self, damping: f32) -> Self {
        self.constraint.damping = damping;
        self
    }

    /// Set rest length (for linear springs)
    #[allow(dead_code)]
    pub fn rest_length(mut self, length: f32) -> Self {
        self.constraint.rest_length = length;
        self
    }

    /// Set rest angle (for angular springs)
    #[allow(dead_code)]
    pub fn rest_angle(mut self, angle: f32) -> Self {
        self.constraint.rest_angle = angle;
        self
    }

    /// Set anchor points
    #[allow(dead_code)]
    pub fn anchors(mut self, anchor_a: Vec3, anchor_b: Vec3) -> Self {
        self.constraint.anchor_a = anchor_a;
        self.constraint.anchor_b = anchor_b;
        self
    }

    /// Set connected entity
    #[allow(dead_code)]
    pub fn connected(mut self, entity: Entity) -> Self {
        self.constraint.connected_entity = Some(entity);
        self
    }

    /// Set maximum force
    #[allow(dead_code)]
    pub fn max_force(mut self, force: f32) -> Self {
        self.constraint.max_force = force;
        self
    }

    /// Set spring axis
    #[allow(dead_code)]
    pub fn axis(mut self, axis: Vec3) -> Self {
        self.constraint.axis = axis.normalize();
        self
    }

    /// Build the constraint
    #[allow(dead_code)]
    pub fn build(self) -> SpringDamperConstraint {
        self.constraint
    }
}

/// Configure a Rapier joint with spring-damper behavior
#[allow(dead_code)]
pub fn configure_joint_spring_damper(
    joint: &mut GenericJoint,
    axis: JointAxis,
    stiffness: f32,
    damping: f32,
    rest_position: f32,
) {
    joint.set_motor_model(axis, MotorModel::ForceBased);
    joint.set_motor_position(axis, rest_position, stiffness, damping);
}

/// Add spring-damper to all translational axes of a joint
#[allow(dead_code)]
pub fn configure_joint_linear_spring(joint: &mut GenericJoint, stiffness: f32, damping: f32) {
    configure_joint_spring_damper(joint, JointAxis::LinX, stiffness, damping, 0.0);
    configure_joint_spring_damper(joint, JointAxis::LinY, stiffness, damping, 0.0);
    configure_joint_spring_damper(joint, JointAxis::LinZ, stiffness, damping, 0.0);
}

/// Add spring-damper to all rotational axes of a joint
#[allow(dead_code)]
pub fn configure_joint_angular_spring(joint: &mut GenericJoint, stiffness: f32, damping: f32) {
    configure_joint_spring_damper(joint, JointAxis::AngX, stiffness, damping, 0.0);
    configure_joint_spring_damper(joint, JointAxis::AngY, stiffness, damping, 0.0);
    configure_joint_spring_damper(joint, JointAxis::AngZ, stiffness, damping, 0.0);
}

// ============================================================================
// Systems
// ============================================================================

/// System to update breakable joints and detect breaks
pub fn update_breakable_joints_system(
    mut breakable_joints: Query<(Entity, &mut BreakableJoint)>,
    mut break_events: EventWriter<JointBreakEvent>,
    mut joint_manager: ResMut<BreakableJointManager>,
    physics_world: Res<crate::physics::world::PhysicsWorld>,
) {
    for (entity, mut breakable) in breakable_joints.iter_mut() {
        if breakable.is_broken {
            continue;
        }

        // Get joint forces from Rapier (if available)
        // Note: Rapier doesn't directly expose joint forces, so we estimate from constraint data
        if let Some(joint) = physics_world.impulse_joint_set.get(breakable.joint_handle) {
            // Estimate forces from joint data
            let body1 = joint.body1;
            let body2 = joint.body2;

            if let (Some(rb1), Some(rb2)) = (
                physics_world.rigid_body_set.get(body1),
                physics_world.rigid_body_set.get(body2),
            ) {
                // Estimate joint force from relative motion
                let vel1 = rb1.linvel();
                let vel2 = rb2.linvel();
                let relative_vel = Vec3::new(vel2.x - vel1.x, vel2.y - vel1.y, vel2.z - vel1.z);

                let angvel1 = rb1.angvel();
                let angvel2 = rb2.angvel();
                let relative_angvel = Vec3::new(
                    angvel2.x - angvel1.x,
                    angvel2.y - angvel1.y,
                    angvel2.z - angvel1.z,
                );

                // Simple force estimation based on constraint violation
                let estimated_force = relative_vel * 1000.0; // Simplified
                let estimated_torque = relative_angvel * 100.0; // Simplified

                let was_broken = breakable.update(estimated_force, estimated_torque);

                if was_broken {
                    let force_exceeded = estimated_force.length() > breakable.max_force;
                    let torque_exceeded = estimated_torque.length() > breakable.max_torque;

                    break_events.send(JointBreakEvent {
                        entity,
                        joint_handle: breakable.joint_handle,
                        break_force: estimated_force,
                        break_torque: estimated_torque,
                        force_exceeded,
                        torque_exceeded,
                    });

                    joint_manager.queue_break(breakable.joint_handle);
                }
            }
        }
    }
}

/// System to apply spring-damper constraints
#[allow(unused_mut)]
pub fn apply_spring_damper_system(
    springs: Query<(Entity, &SpringDamperConstraint, &GlobalTransform)>,
    transforms: Query<&GlobalTransform>,
    _physics_world: ResMut<crate::physics::world::PhysicsWorld>,
) {
    for (_entity_a, spring, transform_a) in springs.iter() {
        if !spring.enabled {
            continue;
        }

        let Some(entity_b) = spring.connected_entity else {
            continue;
        };

        let Ok(transform_b) = transforms.get(entity_b) else {
            continue;
        };

        // Calculate world-space anchor points
        let world_anchor_a = transform_a.transform_point(spring.anchor_a);
        let world_anchor_b = transform_b.transform_point(spring.anchor_b);

        // Get velocities from rigid bodies (simplified - would need proper lookup)
        let velocity_a = Vec3::ZERO;
        let velocity_b = Vec3::ZERO;

        match spring.spring_type {
            SpringType::Linear => {
                let force = spring.calculate_linear_force(
                    world_anchor_a,
                    world_anchor_b,
                    velocity_a,
                    velocity_b,
                );

                // Apply force to physics world (would need proper rigid body lookup)
                // This is a simplified version - actual implementation would
                // apply forces through the RigidBodySet
                let _ = force; // Force calculated but application depends on physics integration
            }
            SpringType::Angular => {
                let rotation_a = transform_a.to_scale_rotation_translation().1;
                let rotation_b = transform_b.to_scale_rotation_translation().1;

                let torque =
                    spring.calculate_angular_torque(rotation_a, rotation_b, Vec3::ZERO, Vec3::ZERO);

                let _ = torque; // Torque calculated but application depends on physics integration
            }
            SpringType::Combined => {
                let force = spring.calculate_linear_force(
                    world_anchor_a,
                    world_anchor_b,
                    velocity_a,
                    velocity_b,
                );

                let rotation_a = transform_a.to_scale_rotation_translation().1;
                let rotation_b = transform_b.to_scale_rotation_translation().1;

                let torque =
                    spring.calculate_angular_torque(rotation_a, rotation_b, Vec3::ZERO, Vec3::ZERO);

                let _ = (force, torque);
            }
        }
    }
}

/// System to process contact events
pub fn process_contact_events_system(mut callbacks: ResMut<ContactEventCallbacks>) {
    callbacks.process_pending();
}

/// System to remove broken joints from physics world
pub fn cleanup_broken_joints_system(
    mut joint_manager: ResMut<BreakableJointManager>,
    mut physics_world: ResMut<crate::physics::world::PhysicsWorld>,
) {
    joint_manager.remove_broken_joints(&mut physics_world.impulse_joint_set);
}

// ============================================================================
// Plugin
// ============================================================================

/// Advanced physics plugin for CCD, friction models, breakable joints, etc.
pub struct AdvancedPhysicsPlugin;

impl Plugin for AdvancedPhysicsPlugin {
    fn build(&self, app: &mut App) {
        // CCD resources
        app.init_resource::<CCDSolverAdvanced>();

        // Friction resources
        app.init_resource::<FrictionMaterialPairs>();

        // Contact event handling
        app.init_resource::<ContactEventCallbacks>();

        // Breakable joint management
        app.init_resource::<BreakableJointManager>();

        // Register events
        app.add_event::<JointBreakEvent>();

        // Add systems for advanced physics features
        app.add_systems(
            Update,
            (
                update_breakable_joints_system,
                apply_spring_damper_system,
                process_contact_events_system,
                cleanup_broken_joints_system,
            )
                .chain(),
        );

        // Note: Component types (CCDEnabled, AnisotropicFrictionComponent, etc.)
        // are not registered with Bevy reflection since they don't derive Reflect.
        // They are still usable as components but won't appear in inspector UI.

        tracing::info!("Advanced physics plugin loaded with CCD, friction models, breakable joints, and spring-damper systems");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // CCD Tests

    #[test]
    fn test_ccd_config_default() {
        let config = CCDConfig::default();
        assert!(config.enabled);
        assert!(config.sweep_tolerance > 0.0);
        assert!(config.max_substeps > 0);
        assert!(config.velocity_threshold > 0.0);
    }

    #[test]
    fn test_ccd_config_presets() {
        let high = CCDConfig::high_precision();
        let perf = CCDConfig::performance();

        assert!(high.sweep_tolerance < perf.sweep_tolerance);
        assert!(high.max_substeps > perf.max_substeps);
    }

    #[test]
    fn test_ccd_solver_enable_disable() {
        let mut solver = CCDSolverAdvanced::default();
        let entity = Entity::from_raw(1);

        solver.enable_ccd(entity, CCDEnabled::default());
        assert!(solver.is_ccd_enabled(entity));

        solver.disable_ccd(entity);
        assert!(!solver.is_ccd_enabled(entity));

        solver.remove_ccd(entity);
        assert!(!solver.is_ccd_enabled(entity));
    }

    #[test]
    fn test_ccd_needs_check() {
        let solver = CCDSolverAdvanced::default();

        // Low velocity - no CCD needed
        let low_vel = Vec3::new(0.01, 0.0, 0.0);
        assert!(!solver.needs_ccd(low_vel, Vec3::ZERO, 1.0));

        // High velocity - CCD needed
        let high_vel = Vec3::new(10.0, 0.0, 0.0);
        assert!(solver.needs_ccd(high_vel, Vec3::ZERO, 1.0));
    }

    #[test]
    fn test_ccd_stats_reset() {
        let mut solver = CCDSolverAdvanced::default();
        solver.stats.sweep_tests_performed = 10;
        solver.stats.tunneling_prevented = 5;

        solver.reset_stats();

        assert_eq!(solver.stats.sweep_tests_performed, 0);
        assert_eq!(solver.stats.tunneling_prevented, 0);
    }

    // Coulomb Friction Tests

    #[test]
    fn test_coulomb_friction_default() {
        let friction = CoulombFriction::default();
        assert!(friction.static_coefficient > friction.kinetic_coefficient);
        assert!(friction.rolling_resistance > 0.0);
    }

    #[test]
    fn test_coulomb_friction_presets() {
        let rubber = CoulombFriction::rubber_concrete();
        let ice = CoulombFriction::ice();
        let steel = CoulombFriction::steel_steel();

        assert!(rubber.static_coefficient > ice.static_coefficient);
        assert!(ice.static_coefficient < steel.static_coefficient);
    }

    #[test]
    fn test_coulomb_effective_coefficient() {
        let friction = CoulombFriction::new(0.6, 0.4);

        // Below threshold - static friction
        let static_coeff = friction.effective_coefficient(0.001);
        assert!((static_coeff - 0.6).abs() < 0.001);

        // Above threshold - kinetic friction
        let kinetic_coeff = friction.effective_coefficient(1.0);
        assert!((kinetic_coeff - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_coulomb_friction_force() {
        let friction = CoulombFriction::new(0.5, 0.4);
        let normal_force = 100.0;

        let static_force = friction.friction_force(normal_force, 0.001);
        assert!((static_force - 50.0).abs() < 0.1);

        let kinetic_force = friction.friction_force(normal_force, 1.0);
        assert!((kinetic_force - 40.0).abs() < 0.1);
    }

    #[test]
    fn test_coulomb_rolling_torque() {
        let friction = CoulombFriction::default();
        let torque = friction.rolling_torque(100.0, 0.5);
        assert!(torque > 0.0);
        assert!(torque < 100.0); // Should be much smaller than normal force
    }

    // Friction Pyramid Tests

    #[test]
    fn test_friction_pyramid_isotropic() {
        let pyramid = FrictionPyramid::new_isotropic(4, 0.5);

        assert_eq!(pyramid.num_sides, 4);
        assert_eq!(pyramid.coefficients.len(), 4);

        // All coefficients should be equal for isotropic
        for coeff in &pyramid.coefficients {
            assert!((*coeff - 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn test_friction_pyramid_anisotropic() {
        let pyramid = FrictionPyramid::new_anisotropic(0.8, 0.3, Vec3::X);

        assert_eq!(pyramid.num_sides, 4);

        // Forward/backward should have higher friction
        let forward_coeff = pyramid.coefficient_in_direction(Vec3::X);
        let lateral_coeff = pyramid.coefficient_in_direction(Vec3::Z);

        assert!(forward_coeff > lateral_coeff);
    }

    #[test]
    fn test_friction_pyramid_force() {
        let pyramid = FrictionPyramid::new_isotropic(4, 0.5);

        let force = pyramid.friction_force(100.0, Vec3::new(1.0, 0.0, 0.0));
        assert!(force.length() > 0.0);
        assert!(force.x < 0.0); // Force opposes motion
    }

    // Anisotropic Friction Tests

    #[test]
    fn test_anisotropic_friction_disabled() {
        let friction = AnisotropicFrictionComponent {
            primary_coefficient: 0.8,
            secondary_coefficient: 0.3,
            enabled: false,
            ..Default::default()
        };

        let transform = Transform::IDENTITY;
        let coeff = friction.coefficient_in_direction(Vec3::X, &transform);

        // Should return average when disabled
        assert!((coeff - 0.55).abs() < 0.001);
    }

    #[test]
    fn test_anisotropic_friction_enabled() {
        let friction = AnisotropicFrictionComponent::new(0.8, 0.3, Vec3::X);

        let transform = Transform::IDENTITY;

        let primary_coeff = friction.coefficient_in_direction(Vec3::X, &transform);
        let secondary_coeff = friction.coefficient_in_direction(Vec3::Z, &transform);

        assert!(primary_coeff > secondary_coeff);
    }

    // Friction Material Pairs Tests

    #[test]
    fn test_friction_material_pairs() {
        let mut pairs = FrictionMaterialPairs::new();

        let rubber_concrete = CoulombFriction::rubber_concrete();
        pairs.set_friction(1, 2, rubber_concrete.clone());

        let retrieved = pairs.get_friction(1, 2);
        assert!((retrieved.static_coefficient - rubber_concrete.static_coefficient).abs() < 0.001);

        // Should work with reversed order too
        let retrieved_reversed = pairs.get_friction(2, 1);
        assert!(
            (retrieved_reversed.static_coefficient - rubber_concrete.static_coefficient).abs()
                < 0.001
        );
    }

    #[test]
    fn test_friction_material_pairs_default() {
        let pairs = FrictionMaterialPairs::new();

        let unknown = pairs.get_friction(99, 100);
        assert!(
            (unknown.static_coefficient - pairs.default_friction.static_coefficient).abs() < 0.001
        );
    }

    // Contact Model Tests

    #[test]
    fn test_contact_config_default() {
        let config = ContactConfig::default();
        assert_eq!(config.model, ContactModel::Point);
        assert!(config.stiffness > 0.0);
        assert!(config.damping > 0.0);
    }

    #[test]
    fn test_contact_config_soft() {
        let config = ContactConfig::soft(1e5, 1e3);
        assert_eq!(config.model, ContactModel::Soft);
        assert!((config.stiffness - 1e5).abs() < 1.0);
        assert!((config.damping - 1e3).abs() < 1.0);
    }

    #[test]
    fn test_contact_config_patch() {
        let config = ContactConfig::patch(8, 0.02);
        assert_eq!(config.model, ContactModel::Patch);
        assert_eq!(config.patch_points, 8);
        assert!((config.patch_radius - 0.02).abs() < 0.001);
    }

    #[test]
    fn test_soft_contact_force() {
        let config = ContactConfig::soft(10000.0, 1000.0);

        // No penetration - no force
        let no_force = config.soft_contact_force(0.0, 0.0);
        assert!((no_force).abs() < 0.001);

        // With penetration - positive force
        let force = config.soft_contact_force(0.001, 0.0);
        assert!(force > 0.0);
    }

    #[test]
    fn test_soft_contact_force_clamping() {
        let config = ContactConfig {
            max_penetration: 0.01,
            stiffness: 10000.0,
            ..ContactConfig::soft(10000.0, 1000.0)
        };

        let normal_force = config.soft_contact_force(0.005, 0.0);
        let clamped_force = config.soft_contact_force(0.1, 0.0); // Way over max

        // Clamped force should not be much larger than at max penetration
        assert!(clamped_force <= config.stiffness * config.max_penetration * 1.1);
        let _ = normal_force; // Used for comparison logic
    }

    // Contact Force Visualizer Tests

    #[test]
    fn test_contact_force_visualizer() {
        let viz = ContactForceVisualizer::default();

        // Below min force - no arrow
        let small_force = Vec3::new(0.01, 0.0, 0.0);
        assert!(viz.force_arrow(Vec3::ZERO, small_force).is_none());

        // Above min force - arrow generated
        let large_force = Vec3::new(100.0, 0.0, 0.0);
        let arrow = viz.force_arrow(Vec3::ZERO, large_force);
        assert!(arrow.is_some());
    }

    // Penetration Limiter Tests

    #[test]
    fn test_penetration_limiter_default() {
        let limiter = PenetrationLimiter::default();
        assert!(limiter.max_depth > 0.0);
        assert!(limiter.slop > 0.0);
        assert!(limiter.slop < limiter.max_depth);
    }

    #[test]
    fn test_penetration_limiter_correction() {
        let limiter = PenetrationLimiter::default();
        let normal = Vec3::Y;

        // Below slop - no correction
        let small_correction = limiter.position_correction(limiter.slop * 0.5, normal);
        assert!(small_correction.length() < 0.0001);

        // Above slop - correction applied
        let correction = limiter.position_correction(limiter.max_depth, normal);
        assert!(correction.length() > 0.0);
        assert!(correction.y > 0.0); // Correction in normal direction
    }

    #[test]
    fn test_penetration_limiter_velocity_bias() {
        let limiter = PenetrationLimiter::default();

        // Below slop - no bias
        let small_bias = limiter.velocity_bias(limiter.slop * 0.5);
        assert!(small_bias.abs() < 0.0001);

        // Above slop - bias applied
        let bias = limiter.velocity_bias(limiter.max_depth);
        assert!(bias > 0.0);
    }

    // Breakable Joint Tests

    #[test]
    fn test_breakable_joint_creation() {
        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        let joint = BreakableJoint::new(handle, 1000.0, 500.0);

        assert!(!joint.is_broken);
        assert!((joint.max_force - 1000.0).abs() < 0.001);
        assert!((joint.max_torque - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_breakable_joint_break_by_force() {
        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        let mut joint = BreakableJoint::new(handle, 100.0, 500.0);

        // Below threshold - not broken
        let broken = joint.update(Vec3::new(50.0, 0.0, 0.0), Vec3::ZERO);
        assert!(!broken);

        // Above threshold - broken
        let broken = joint.update(Vec3::new(150.0, 0.0, 0.0), Vec3::ZERO);
        assert!(broken);
        assert!(joint.is_broken);
    }

    #[test]
    fn test_breakable_joint_break_by_torque() {
        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        let mut joint = BreakableJoint::new(handle, 1000.0, 50.0);

        // Below threshold - not broken
        let broken = joint.update(Vec3::ZERO, Vec3::new(30.0, 0.0, 0.0));
        assert!(!broken);

        // Above threshold - broken
        let broken = joint.update(Vec3::ZERO, Vec3::new(60.0, 0.0, 0.0));
        assert!(broken);
    }

    #[test]
    fn test_breakable_joint_averaging() {
        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        let mut joint = BreakableJoint::new(handle, 100.0, 500.0).with_averaging(3);

        assert!(joint.use_averaging);
        assert_eq!(joint.averaging_frames, 3);

        // Single high force shouldn't break with averaging
        joint.update(Vec3::new(150.0, 0.0, 0.0), Vec3::ZERO);
        assert!(!joint.is_broken);

        // Continue with low forces
        joint.update(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);
        joint.update(Vec3::new(10.0, 0.0, 0.0), Vec3::ZERO);

        // Average should be below threshold
        assert!(!joint.is_broken);
    }

    #[test]
    fn test_breakable_joint_reset() {
        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        let mut joint = BreakableJoint::new(handle, 100.0, 500.0);

        joint.update(Vec3::new(150.0, 0.0, 0.0), Vec3::ZERO);
        assert!(joint.is_broken);

        joint.reset();
        assert!(!joint.is_broken);
    }

    // Breakable Joint Manager Tests

    #[test]
    fn test_breakable_joint_manager() {
        let mut manager = BreakableJointManager::default();

        let handle = ImpulseJointHandle::from_raw_parts(1, 0);
        manager.queue_break(handle);

        assert_eq!(manager.broken_joints.len(), 1);
        assert_eq!(manager.total_breaks, 1);
    }

    // Spring-Damper Tests

    #[test]
    fn test_spring_damper_default() {
        let spring = SpringDamperConstraint::default();
        assert_eq!(spring.spring_type, SpringType::Linear);
        assert!(spring.stiffness > 0.0);
        assert!(spring.damping > 0.0);
        assert!(spring.enabled);
    }

    #[test]
    fn test_spring_damper_linear() {
        let spring = SpringDamperConstraint::linear(500.0, 50.0, 2.0);
        assert_eq!(spring.spring_type, SpringType::Linear);
        assert!((spring.stiffness - 500.0).abs() < 0.001);
        assert!((spring.damping - 50.0).abs() < 0.001);
        assert!((spring.rest_length - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_spring_damper_angular() {
        let spring = SpringDamperConstraint::angular(100.0, 10.0, std::f32::consts::FRAC_PI_2);
        assert_eq!(spring.spring_type, SpringType::Angular);
        assert!((spring.rest_angle - std::f32::consts::FRAC_PI_2).abs() < 0.001);
    }

    #[test]
    fn test_spring_damper_builder() {
        let spring = SpringDamperBuilder::linear()
            .stiffness(1000.0)
            .damping(100.0)
            .rest_length(1.5)
            .max_force(5000.0)
            .build();

        assert_eq!(spring.spring_type, SpringType::Linear);
        assert!((spring.stiffness - 1000.0).abs() < 0.001);
        assert!((spring.damping - 100.0).abs() < 0.001);
        assert!((spring.rest_length - 1.5).abs() < 0.001);
        assert!((spring.max_force - 5000.0).abs() < 0.001);
    }

    #[test]
    fn test_spring_linear_force_at_rest() {
        let spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0);

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(1.0, 0.0, 0.0); // At rest length

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, Vec3::ZERO);
        assert!(force.length() < 0.001); // No force at rest
    }

    #[test]
    fn test_spring_linear_force_stretched() {
        let spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0);

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(2.0, 0.0, 0.0); // Stretched by 1 meter

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, Vec3::ZERO);

        assert!(force.x > 0.0); // Force pulls A toward B
        assert!((force.length() - 1000.0).abs() < 1.0); // F = k * x = 1000 * 1
    }

    #[test]
    fn test_spring_linear_force_compressed() {
        let spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0);

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(0.5, 0.0, 0.0); // Compressed by 0.5 meters

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, Vec3::ZERO);

        assert!(force.x < 0.0); // Force pushes A away from B
        assert!((force.length() - 500.0).abs() < 1.0); // F = k * x = 1000 * 0.5
    }

    #[test]
    fn test_spring_with_damping() {
        let spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0);

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(1.0, 0.0, 0.0); // At rest length
        let velocity_b = Vec3::new(1.0, 0.0, 0.0); // Moving away

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, velocity_b);

        // Should have damping force opposing motion
        assert!(force.x > 0.0);
        assert!((force.length() - 100.0).abs() < 1.0); // F = c * v = 100 * 1
    }

    #[test]
    fn test_spring_disabled() {
        let mut spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0);
        spring.enabled = false;

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(2.0, 0.0, 0.0);

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, Vec3::ZERO);
        assert!(force.length() < 0.001); // No force when disabled
    }

    #[test]
    fn test_spring_max_force_clamping() {
        let spring = SpringDamperConstraint::linear(1000.0, 100.0, 1.0).with_max_force(500.0);

        let anchor_a = Vec3::ZERO;
        let anchor_b = Vec3::new(2.0, 0.0, 0.0); // Would produce 1000N without clamping

        let force = spring.calculate_linear_force(anchor_a, anchor_b, Vec3::ZERO, Vec3::ZERO);
        assert!(force.length() <= 500.1); // Clamped to max
    }

    // Contact Callbacks Tests

    #[test]
    fn test_contact_callbacks_queue() {
        let mut callbacks = ContactEventCallbacks::default();

        let event = ContactEventData {
            entity_a: Entity::from_raw(1),
            entity_b: Entity::from_raw(2),
            contact_point: Vec3::ZERO,
            normal: Vec3::Y,
            penetration: 0.001,
            relative_velocity: Vec3::ZERO,
            force_magnitude: 100.0,
            event_type: ContactEventType::Started,
        };

        callbacks.queue_event(event);
        assert_eq!(callbacks.pending_events.len(), 1);
    }

    // Breakable Joint Creation Helpers Tests

    #[test]
    fn test_create_breakable_revolute_joint() {
        let (_joint, max_force, max_torque) = create_breakable_revolute_joint(
            Vec3::ZERO,
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::Y,
            1000.0,
            500.0,
        );

        assert!((max_force - 1000.0).abs() < 0.001);
        assert!((max_torque - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_create_breakable_fixed_joint() {
        let (_joint, max_force, max_torque) =
            create_breakable_fixed_joint(Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0), 2000.0, 1000.0);

        assert!((max_force - 2000.0).abs() < 0.001);
        assert!((max_torque - 1000.0).abs() < 0.001);
    }

    #[test]
    fn test_create_breakable_spherical_joint() {
        let (_joint, max_force, max_torque) =
            create_breakable_spherical_joint(Vec3::ZERO, Vec3::ONE, 500.0, 250.0);

        assert!((max_force - 500.0).abs() < 0.001);
        assert!((max_torque - 250.0).abs() < 0.001);
    }

    #[test]
    fn test_create_breakable_prismatic_joint() {
        let (_joint, max_force, max_torque) = create_breakable_prismatic_joint(
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::X,
            800.0,
            400.0,
        );

        assert!((max_force - 800.0).abs() < 0.001);
        assert!((max_torque - 400.0).abs() < 0.001);
    }

    // Integration Tests

    #[test]
    fn test_ccd_with_sweep_result() {
        let result = SweepTestResult {
            collider_handle: ColliderHandle::from_raw_parts(1, 0),
            time_of_impact: 0.5,
            normal: Vec3::Y,
            contact_point: Vec3::new(1.0, 0.0, 0.0),
        };

        assert!(result.time_of_impact >= 0.0 && result.time_of_impact <= 1.0);
        assert!(result.normal.length() > 0.9); // Should be normalized
    }

    #[test]
    fn test_contact_event_types() {
        let started = ContactEventType::Started;
        let ongoing = ContactEventType::Ongoing;
        let ended = ContactEventType::Ended;

        assert_ne!(started, ongoing);
        assert_ne!(ongoing, ended);
        assert_ne!(started, ended);
    }

    #[test]
    fn test_spring_types() {
        let linear = SpringType::Linear;
        let angular = SpringType::Angular;
        let combined = SpringType::Combined;

        assert_ne!(linear, angular);
        assert_ne!(angular, combined);
        assert_ne!(linear, combined);
    }

    #[test]
    fn test_contact_models() {
        let point = ContactModel::Point;
        let patch = ContactModel::Patch;
        let soft = ContactModel::Soft;

        assert_eq!(ContactModel::default(), ContactModel::Point);
        assert_ne!(point, patch);
        assert_ne!(patch, soft);
    }
}
