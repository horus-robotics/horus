//! Tactile and touch sensor simulation for robotic manipulation

use crate::physics::world::PhysicsWorld;
use bevy::prelude::*;
use rapier3d::prelude::*;

/// Tactile sensor array component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct TactileSensor {
    /// Sensor type
    pub sensor_type: TactileSensorType,
    /// Array dimensions (rows, cols)
    pub array_size: (usize, usize),
    /// Physical size of sensor surface (meters)
    pub physical_size: (f32, f32),
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
    /// Force sensitivity (N)
    pub force_resolution: f32,
    /// Maximum measurable force (N)
    pub max_force: f32,
}

impl Default for TactileSensor {
    fn default() -> Self {
        Self {
            sensor_type: TactileSensorType::Pressure,
            array_size: (16, 16),
            physical_size: (0.04, 0.04), // 40mm x 40mm
            rate_hz: 100.0,
            last_update: 0.0,
            force_resolution: 0.01, // 10mN
            max_force: 50.0,
        }
    }
}

impl TactileSensor {
    pub fn new(rows: usize, cols: usize, size_x: f32, size_y: f32) -> Self {
        Self {
            array_size: (rows, cols),
            physical_size: (size_x, size_y),
            ..default()
        }
    }

    pub fn with_type(mut self, sensor_type: TactileSensorType) -> Self {
        self.sensor_type = sensor_type;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_sensitivity(mut self, resolution: f32, max_force: f32) -> Self {
        self.force_resolution = resolution;
        self.max_force = max_force;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn taxel_count(&self) -> usize {
        self.array_size.0 * self.array_size.1
    }

    /// Get physical size of each taxel (tactile pixel)
    pub fn taxel_size(&self) -> (f32, f32) {
        (
            self.physical_size.0 / self.array_size.1 as f32,
            self.physical_size.1 / self.array_size.0 as f32,
        )
    }

    /// Fingertip sensor (small, high resolution)
    pub fn fingertip() -> Self {
        Self {
            sensor_type: TactileSensorType::Pressure,
            array_size: (32, 32),
            physical_size: (0.015, 0.015), // 15mm x 15mm
            force_resolution: 0.005,       // 5mN
            max_force: 20.0,
            rate_hz: 200.0,
            ..default()
        }
    }

    /// Palm sensor (larger area, lower resolution)
    pub fn palm() -> Self {
        Self {
            sensor_type: TactileSensorType::Pressure,
            array_size: (16, 16),
            physical_size: (0.08, 0.08), // 80mm x 80mm
            force_resolution: 0.02,
            max_force: 100.0,
            rate_hz: 100.0,
            ..default()
        }
    }

    /// GelSight-style optical tactile sensor
    pub fn optical_gel() -> Self {
        Self {
            sensor_type: TactileSensorType::OpticalGel,
            array_size: (64, 64),
            physical_size: (0.025, 0.025), // 25mm x 25mm
            force_resolution: 0.001,       // Very sensitive
            max_force: 10.0,
            rate_hz: 30.0,
            ..default()
        }
    }
}

/// Types of tactile sensors
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TactileSensorType {
    /// Pressure/force sensor (resistive, capacitive)
    Pressure,
    /// Strain gauge based
    StrainGauge,
    /// Optical/vision-based (e.g., GelSight)
    OpticalGel,
    /// Piezoelectric sensor
    Piezoelectric,
    /// BioTac-style fluid-filled sensor
    BioTac,
}

/// Single taxel (tactile element) reading
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TaxelReading {
    /// Normal force (N)
    pub force_normal: f32,
    /// Shear force X (N)
    pub force_shear_x: f32,
    /// Shear force Y (N)
    pub force_shear_y: f32,
    /// Contact area ratio (0.0-1.0)
    pub contact_area: f32,
    /// Vibration amplitude (for dynamic sensing)
    pub vibration: f32,
}

impl TaxelReading {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_force(normal: f32, shear_x: f32, shear_y: f32) -> Self {
        Self {
            force_normal: normal,
            force_shear_x: shear_x,
            force_shear_y: shear_y,
            contact_area: if normal > 0.0 { 1.0 } else { 0.0 },
            vibration: 0.0,
        }
    }

    pub fn total_force(&self) -> f32 {
        (self.force_normal.powi(2) + self.force_shear_x.powi(2) + self.force_shear_y.powi(2)).sqrt()
    }

    pub fn is_in_contact(&self) -> bool {
        self.force_normal > 0.0 || self.contact_area > 0.0
    }
}

/// Tactile array data
#[derive(Component, Clone)]
pub struct TactileData {
    /// Array of taxel readings (row-major order)
    pub readings: Vec<TaxelReading>,
    /// Array dimensions (rows, cols)
    pub dimensions: (usize, usize),
    /// Timestamp
    pub timestamp: f32,
    /// Total contact force
    pub total_force: Vec3,
    /// Center of pressure (normalized coordinates 0.0-1.0)
    pub center_of_pressure: Option<(f32, f32)>,
}

impl TactileData {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            readings: vec![TaxelReading::default(); rows * cols],
            dimensions: (rows, cols),
            timestamp: 0.0,
            total_force: Vec3::ZERO,
            center_of_pressure: None,
        }
    }

    pub fn get_taxel(&self, row: usize, col: usize) -> Option<&TaxelReading> {
        if row >= self.dimensions.0 || col >= self.dimensions.1 {
            return None;
        }
        let index = row * self.dimensions.1 + col;
        self.readings.get(index)
    }

    pub fn set_taxel(&mut self, row: usize, col: usize, reading: TaxelReading) {
        if row >= self.dimensions.0 || col >= self.dimensions.1 {
            return;
        }
        let index = row * self.dimensions.1 + col;
        if let Some(taxel) = self.readings.get_mut(index) {
            *taxel = reading;
        }
    }

    pub fn clear(&mut self) {
        for reading in &mut self.readings {
            *reading = TaxelReading::default();
        }
        self.total_force = Vec3::ZERO;
        self.center_of_pressure = None;
    }

    /// Calculate center of pressure from force distribution
    pub fn calculate_center_of_pressure(&mut self) {
        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;
        let mut total_force = 0.0;

        for row in 0..self.dimensions.0 {
            for col in 0..self.dimensions.1 {
                if let Some(reading) = self.get_taxel(row, col) {
                    let force = reading.force_normal;
                    if force > 0.0 {
                        weighted_x += col as f32 * force;
                        weighted_y += row as f32 * force;
                        total_force += force;
                    }
                }
            }
        }

        if total_force > 0.0 {
            let cop_x = weighted_x / total_force / self.dimensions.1 as f32;
            let cop_y = weighted_y / total_force / self.dimensions.0 as f32;
            self.center_of_pressure = Some((cop_x, cop_y));
        } else {
            self.center_of_pressure = None;
        }
    }

    /// Get contact area (fraction of taxels in contact)
    pub fn get_contact_fraction(&self) -> f32 {
        let in_contact = self.readings.iter().filter(|r| r.is_in_contact()).count();
        in_contact as f32 / self.readings.len() as f32
    }

    /// Get maximum force across all taxels
    pub fn get_max_force(&self) -> f32 {
        self.readings
            .iter()
            .map(|r| r.force_normal)
            .fold(0.0, f32::max)
    }

    /// Convert to pressure heatmap (grayscale image)
    pub fn to_heatmap(&self) -> Vec<u8> {
        let max_force = self.get_max_force().max(0.01);
        let mut heatmap = Vec::with_capacity(self.readings.len());

        for reading in &self.readings {
            let normalized = (reading.force_normal / max_force).clamp(0.0, 1.0);
            heatmap.push((normalized * 255.0) as u8);
        }

        heatmap
    }
}

/// Contact point information for physics integration
#[derive(Clone, Debug)]
pub struct ContactPoint {
    /// Contact position in world space
    pub position: Vec3,
    /// Contact normal
    pub normal: Vec3,
    /// Contact force
    pub force: Vec3,
    /// Contact depth/penetration
    pub depth: f32,
}

/// System to update tactile sensors from physics contacts
pub fn tactile_sensor_update_system(
    time: Res<Time>,
    physics_world: Res<PhysicsWorld>,
    mut sensors: Query<(
        Entity,
        &mut TactileSensor,
        &mut TactileData,
        &GlobalTransform,
    )>,
) {
    let current_time = time.elapsed_secs();

    for (entity, mut sensor, mut data, transform) in sensors.iter_mut() {
        if !sensor.should_update(current_time) {
            continue;
        }

        sensor.last_update = current_time;
        data.timestamp = current_time;
        data.clear();

        // Query physics contacts from Rapier
        process_tactile_contacts(entity, &sensor, &mut data, transform, &physics_world);

        // Calculate aggregate values
        let total_normal: f32 = data.readings.iter().map(|r| r.force_normal).sum();
        let total_shear_x: f32 = data.readings.iter().map(|r| r.force_shear_x).sum();
        let total_shear_y: f32 = data.readings.iter().map(|r| r.force_shear_y).sum();

        data.total_force = Vec3::new(total_shear_x, total_normal, total_shear_y);
        data.calculate_center_of_pressure();
    }
}

/// Process physics contacts and map them to tactile sensor taxels
fn process_tactile_contacts(
    sensor_entity: Entity,
    sensor: &TactileSensor,
    data: &mut TactileData,
    transform: &GlobalTransform,
    physics_world: &PhysicsWorld,
) {
    let sensor_pos = transform.translation();
    let sensor_rot = transform.to_scale_rotation_translation().1;

    // Get the sensor's local coordinate system
    let sensor_normal = sensor_rot * Vec3::Y; // Assuming Y is up/normal
    let sensor_tangent_x = sensor_rot * Vec3::X;
    let sensor_tangent_z = sensor_rot * Vec3::Z;

    // Physical dimensions of each taxel
    let (taxel_width, taxel_height) = sensor.taxel_size();

    // Find all colliders associated with this entity
    for (collider_handle, collider) in physics_world.collider_set.iter() {
        // Check if this collider belongs to our sensor entity
        if let Some(parent_handle) = collider.parent() {
            if let Some(entity) = physics_world.get_entity_from_handle(parent_handle) {
                if entity != sensor_entity {
                    continue;
                }

                // Get all contact pairs involving this collider
                for contact_pair in physics_world.narrow_phase.contact_pairs() {
                    // Check if our collider is involved in this contact
                    let involves_sensor = contact_pair.collider1 == collider_handle
                        || contact_pair.collider2 == collider_handle;

                    if !involves_sensor {
                        continue;
                    }

                    // Process each contact manifold
                    for manifold in &contact_pair.manifolds {
                        // Process each contact point in the manifold
                        for contact in manifold.points.iter() {
                            // Get contact point in world space
                            let contact_point = if contact_pair.collider1 == collider_handle {
                                point![
                                    manifold.local_n1.x + contact.local_p1.x,
                                    manifold.local_n1.y + contact.local_p1.y,
                                    manifold.local_n1.z + contact.local_p1.z
                                ]
                            } else {
                                point![
                                    manifold.local_n2.x + contact.local_p2.x,
                                    manifold.local_n2.y + contact.local_p2.y,
                                    manifold.local_n2.z + contact.local_p2.z
                                ]
                            };

                            // Transform contact point to sensor local space
                            let local_point = Vec3::new(
                                contact_point.x - sensor_pos.x,
                                contact_point.y - sensor_pos.y,
                                contact_point.z - sensor_pos.z,
                            );

                            // Project onto sensor surface
                            let x_proj = local_point.dot(sensor_tangent_x);
                            let z_proj = local_point.dot(sensor_tangent_z);

                            // Convert to taxel coordinates
                            let taxel_x =
                                ((x_proj + sensor.physical_size.0 / 2.0) / taxel_width) as usize;
                            let taxel_y =
                                ((z_proj + sensor.physical_size.1 / 2.0) / taxel_height) as usize;

                            // Check if within sensor bounds
                            if taxel_x < sensor.array_size.0 && taxel_y < sensor.array_size.1 {
                                // Calculate contact force components from contact data
                                // Normal impulse is stored in the contact data
                                let impulse = contact.data.impulse;
                                let force_magnitude =
                                    impulse.abs() / physics_world.integration_parameters.dt;

                                // Decompose force into normal and shear components
                                let normal_dir = Vec3::new(
                                    manifold.local_n1.x,
                                    manifold.local_n1.y,
                                    manifold.local_n1.z,
                                );
                                let force_normal =
                                    force_magnitude * normal_dir.dot(sensor_normal).abs();

                                // Calculate shear forces (tangential to surface)
                                let force_vector = normal_dir * force_magnitude;
                                let force_shear_x = force_vector.dot(sensor_tangent_x);
                                let force_shear_y = force_vector.dot(sensor_tangent_z);

                                // Apply force to taxel (with sensor characteristics)
                                let quantized_normal = (force_normal / sensor.force_resolution)
                                    .round()
                                    * sensor.force_resolution;
                                let quantized_normal = quantized_normal.min(sensor.max_force);

                                let quantized_shear_x = (force_shear_x / sensor.force_resolution)
                                    .round()
                                    * sensor.force_resolution;
                                let quantized_shear_y = (force_shear_y / sensor.force_resolution)
                                    .round()
                                    * sensor.force_resolution;

                                // Update taxel reading
                                let reading = TaxelReading {
                                    force_normal: quantized_normal,
                                    force_shear_x: quantized_shear_x,
                                    force_shear_y: quantized_shear_y,
                                    contact_area: 1.0, // Full taxel contact assumed
                                    vibration: 0.0,    // Static contact, no vibration
                                };

                                data.set_taxel(taxel_x, taxel_y, reading);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Gripper force sensor (simplified 1D force sensor for parallel grippers)
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct GripperForceSensor {
    /// Current grip force (N)
    pub force: f32,
    /// Maximum force (N)
    pub max_force: f32,
    /// Force resolution (N)
    pub resolution: f32,
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
}

impl Default for GripperForceSensor {
    fn default() -> Self {
        Self {
            force: 0.0,
            max_force: 100.0,
            resolution: 0.1,
            rate_hz: 100.0,
            last_update: 0.0,
        }
    }
}

impl GripperForceSensor {
    pub fn new(max_force: f32) -> Self {
        Self {
            max_force,
            ..default()
        }
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn is_grasping(&self, threshold: f32) -> bool {
        self.force >= threshold
    }
}

/// Plugin for tactile sensor support
pub struct TactileSensorPlugin;

impl Plugin for TactileSensorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, tactile_sensor_update_system);

        tracing::info!("Tactile sensor plugin loaded with high-resolution pressure sensing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tactile_sensor_creation() {
        let sensor = TactileSensor::new(16, 16, 0.04, 0.04);
        assert_eq!(sensor.array_size, (16, 16));
        assert_eq!(sensor.taxel_count(), 256);
    }

    #[test]
    fn test_taxel_size() {
        let sensor = TactileSensor::new(10, 10, 0.05, 0.05);
        let (size_x, size_y) = sensor.taxel_size();
        assert!((size_x - 0.005).abs() < 0.0001);
        assert!((size_y - 0.005).abs() < 0.0001);
    }

    #[test]
    fn test_fingertip_sensor() {
        let sensor = TactileSensor::fingertip();
        assert_eq!(sensor.array_size, (32, 32));
        assert!(sensor.physical_size.0 < 0.02); // Small size
        assert!(sensor.force_resolution < 0.01); // High sensitivity
    }

    #[test]
    fn test_taxel_reading() {
        let reading = TaxelReading::with_force(5.0, 1.0, 0.5);
        assert_eq!(reading.force_normal, 5.0);
        assert!(reading.is_in_contact());

        let total = reading.total_force();
        assert!(total > 5.0); // Should include shear forces
    }

    #[test]
    fn test_tactile_data() {
        let mut data = TactileData::new(4, 4);

        data.set_taxel(2, 2, TaxelReading::with_force(10.0, 0.0, 0.0));
        let reading = data.get_taxel(2, 2).unwrap();
        assert_eq!(reading.force_normal, 10.0);

        assert_eq!(data.get_taxel(10, 10), None);
    }

    #[test]
    fn test_center_of_pressure() {
        let mut data = TactileData::new(5, 5);

        // Apply force at center
        data.set_taxel(2, 2, TaxelReading::with_force(10.0, 0.0, 0.0));
        data.calculate_center_of_pressure();

        let cop = data.center_of_pressure.unwrap();
        assert!((cop.0 - 0.5).abs() < 0.1);
        assert!((cop.1 - 0.5).abs() < 0.1);
    }

    #[test]
    fn test_center_of_pressure_offset() {
        let mut data = TactileData::new(5, 5);

        // Apply force at corner
        data.set_taxel(0, 0, TaxelReading::with_force(10.0, 0.0, 0.0));
        data.calculate_center_of_pressure();

        let cop = data.center_of_pressure.unwrap();
        assert!(cop.0 < 0.2);
        assert!(cop.1 < 0.2);
    }

    #[test]
    fn test_contact_fraction() {
        let mut data = TactileData::new(4, 4);

        // Set 4 out of 16 taxels in contact
        for i in 0..2 {
            for j in 0..2 {
                data.set_taxel(i, j, TaxelReading::with_force(1.0, 0.0, 0.0));
            }
        }

        let fraction = data.get_contact_fraction();
        assert!((fraction - 0.25).abs() < 0.01); // 4/16 = 0.25
    }

    #[test]
    fn test_max_force() {
        let mut data = TactileData::new(4, 4);

        data.set_taxel(0, 0, TaxelReading::with_force(5.0, 0.0, 0.0));
        data.set_taxel(1, 1, TaxelReading::with_force(15.0, 0.0, 0.0));
        data.set_taxel(2, 2, TaxelReading::with_force(8.0, 0.0, 0.0));

        assert_eq!(data.get_max_force(), 15.0);
    }

    #[test]
    fn test_heatmap_generation() {
        let mut data = TactileData::new(4, 4);

        data.set_taxel(0, 0, TaxelReading::with_force(10.0, 0.0, 0.0));
        data.set_taxel(1, 1, TaxelReading::with_force(5.0, 0.0, 0.0));

        let heatmap = data.to_heatmap();
        assert_eq!(heatmap.len(), 16);

        // Max force taxel should be brightest
        assert_eq!(heatmap[0], 255);
        assert!(heatmap[5] < 255); // Second taxel should be dimmer
    }

    #[test]
    fn test_gripper_force_sensor() {
        let mut sensor = GripperForceSensor::new(50.0);
        assert_eq!(sensor.force, 0.0);
        assert!(!sensor.is_grasping(1.0));

        sensor.force = 5.0;
        assert!(sensor.is_grasping(1.0));
        assert!(!sensor.is_grasping(10.0));
    }

    #[test]
    fn test_optical_gel_sensor() {
        let sensor = TactileSensor::optical_gel();
        assert_eq!(sensor.sensor_type, TactileSensorType::OpticalGel);
        assert_eq!(sensor.array_size, (64, 64)); // High resolution
    }

    #[test]
    fn test_clear_tactile_data() {
        let mut data = TactileData::new(4, 4);

        data.set_taxel(0, 0, TaxelReading::with_force(10.0, 1.0, 1.0));
        data.total_force = Vec3::new(1.0, 10.0, 1.0);

        data.clear();

        assert_eq!(data.get_taxel(0, 0).unwrap().force_normal, 0.0);
        assert_eq!(data.total_force, Vec3::ZERO);
        assert_eq!(data.center_of_pressure, None);
    }
}
