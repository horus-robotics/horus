//! Semantic segmentation camera for object detection and scene understanding

use crate::physics::world::PhysicsWorld;
use bevy::prelude::*;
use rapier3d::prelude::*;
use std::collections::HashMap;

/// Semantic class ID for entity labeling
pub type ClassId = u32;

/// Semantic segmentation camera component
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct SegmentationCamera {
    /// Camera resolution (width, height)
    pub resolution: (u32, u32),
    /// Field of view (degrees)
    pub fov: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Update rate (Hz)
    pub rate_hz: f32,
    /// Last update time
    pub last_update: f32,
}

impl Default for SegmentationCamera {
    fn default() -> Self {
        Self {
            resolution: (640, 480),
            fov: 60.0,
            near: 0.1,
            far: 100.0,
            rate_hz: 30.0,
            last_update: 0.0,
        }
    }
}

impl SegmentationCamera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            resolution: (width, height),
            ..default()
        }
    }

    pub fn with_fov(mut self, fov: f32) -> Self {
        self.fov = fov;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }

    pub fn pixel_count(&self) -> usize {
        (self.resolution.0 * self.resolution.1) as usize
    }
}

/// Semantic class label component
#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub struct SemanticClass {
    pub class_id: ClassId,
}

impl SemanticClass {
    pub fn new(class_id: ClassId) -> Self {
        Self { class_id }
    }
}

/// Semantic class registry resource
#[derive(Resource, Default)]
pub struct SemanticClassRegistry {
    /// Class ID to name mapping
    class_names: HashMap<ClassId, String>,
    /// Class ID to color mapping (for visualization)
    class_colors: HashMap<ClassId, Color>,
    /// Next available class ID
    next_id: ClassId,
}

impl SemanticClassRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            class_names: HashMap::new(),
            class_colors: HashMap::new(),
            next_id: 1, // Reserve 0 for unlabeled/background
        };

        // Register default classes
        registry.register_class(0, "unlabeled", Color::BLACK);
        registry
    }

    /// Register a new semantic class
    pub fn register_class(&mut self, id: ClassId, name: &str, color: Color) {
        self.class_names.insert(id, name.to_string());
        self.class_colors.insert(id, color);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
    }

    /// Auto-register a class with the next available ID
    pub fn auto_register(&mut self, name: &str, color: Color) -> ClassId {
        let id = self.next_id;
        self.register_class(id, name, color);
        id
    }

    /// Get class name by ID
    pub fn get_name(&self, id: ClassId) -> Option<&str> {
        self.class_names.get(&id).map(|s| s.as_str())
    }

    /// Get class color by ID
    pub fn get_color(&self, id: ClassId) -> Option<Color> {
        self.class_colors.get(&id).copied()
    }

    /// Get class ID by name
    pub fn get_id(&self, name: &str) -> Option<ClassId> {
        self.class_names
            .iter()
            .find(|(_, n)| n.as_str() == name)
            .map(|(id, _)| *id)
    }

    /// List all registered classes
    pub fn list_classes(&self) -> Vec<(ClassId, String, Color)> {
        self.class_names
            .iter()
            .map(|(id, name)| {
                let color = self.class_colors.get(id).copied().unwrap_or(Color::BLACK);
                (*id, name.clone(), color)
            })
            .collect()
    }
}

/// Segmentation image data
#[derive(Component, Clone)]
pub struct SegmentationImage {
    /// Class IDs for each pixel (row-major order)
    pub class_ids: Vec<ClassId>,
    /// Image dimensions (width, height)
    pub dimensions: (u32, u32),
    /// Timestamp
    pub timestamp: f32,
}

impl SegmentationImage {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            class_ids: vec![0; (width * height) as usize],
            dimensions: (width, height),
            timestamp: 0.0,
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> Option<ClassId> {
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return None;
        }
        let index = (y * self.dimensions.0 + x) as usize;
        self.class_ids.get(index).copied()
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, class_id: ClassId) {
        if x >= self.dimensions.0 || y >= self.dimensions.1 {
            return;
        }
        let index = (y * self.dimensions.0 + x) as usize;
        if let Some(pixel) = self.class_ids.get_mut(index) {
            *pixel = class_id;
        }
    }

    /// Get class histogram (class_id → pixel count)
    pub fn get_histogram(&self) -> HashMap<ClassId, usize> {
        let mut histogram = HashMap::new();
        for &class_id in &self.class_ids {
            *histogram.entry(class_id).or_insert(0) += 1;
        }
        histogram
    }

    /// Convert to RGB color image for visualization
    pub fn to_color_image(&self, registry: &SemanticClassRegistry) -> Vec<u8> {
        let mut rgb = Vec::with_capacity(self.class_ids.len() * 3);

        for &class_id in &self.class_ids {
            let color = registry.get_color(class_id).unwrap_or(Color::BLACK);
            let srgba = color.to_srgba();
            rgb.push((srgba.red * 255.0) as u8);
            rgb.push((srgba.green * 255.0) as u8);
            rgb.push((srgba.blue * 255.0) as u8);
        }

        rgb
    }
}

/// System to update segmentation cameras
pub fn segmentation_camera_update_system(
    time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut cameras: Query<(
        &mut SegmentationCamera,
        &mut SegmentationImage,
        &GlobalTransform,
    )>,
    objects: Query<(Entity, &SemanticClass)>,
) {
    let current_time = time.elapsed_secs();

    // Build entity to class mapping
    let mut entity_to_class: HashMap<Entity, ClassId> = HashMap::new();
    for (entity, class) in objects.iter() {
        entity_to_class.insert(entity, class.class_id);
    }

    for (mut camera, mut image, camera_transform) in cameras.iter_mut() {
        if !camera.should_update(current_time) {
            continue;
        }

        camera.last_update = current_time;
        image.timestamp = current_time;

        // Perform segmentation raycasting
        perform_segmentation_raycasting(
            &camera,
            &mut image,
            camera_transform,
            &mut physics_world,
            &entity_to_class,
        );
    }
}

/// Perform raycasting to generate segmentation image
fn perform_segmentation_raycasting(
    camera: &SegmentationCamera,
    image: &mut SegmentationImage,
    transform: &GlobalTransform,
    physics_world: &mut PhysicsWorld,
    entity_to_class: &HashMap<Entity, ClassId>,
) {
    let width = camera.resolution.0;
    let height = camera.resolution.1;

    // Get camera pose
    let camera_pos = transform.translation();
    let camera_rot = transform.to_scale_rotation_translation().1;

    // Convert to nalgebra types
    let ray_origin = point![camera_pos.x, camera_pos.y, camera_pos.z];
    let rotation = nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
        camera_rot.w,
        camera_rot.x,
        camera_rot.y,
        camera_rot.z,
    ));

    // Calculate field of view parameters
    let fov_rad = camera.fov.to_radians();
    let aspect_ratio = width as f32 / height as f32;
    let half_fov_tan = (fov_rad / 2.0).tan();

    // Cast rays for each pixel
    for y in 0..height {
        for x in 0..width {
            // Calculate normalized device coordinates (-1 to 1)
            let ndc_x = (2.0 * x as f32 / width as f32) - 1.0;
            let ndc_y = 1.0 - (2.0 * y as f32 / height as f32); // Flip Y

            // Calculate ray direction in camera space
            let camera_dir = nalgebra::Vector3::new(
                ndc_x * half_fov_tan * aspect_ratio,
                ndc_y * half_fov_tan,
                -1.0, // Looking down negative Z in camera space
            );

            // Transform to world space
            let world_dir = rotation * camera_dir;
            let ray_dir = nalgebra::Unit::new_normalize(world_dir);

            // Create ray
            let ray = Ray::new(ray_origin, ray_dir.into_inner());

            // Cast ray and get hit information
            let mut class_id = 0; // Default to background

            if let Some((handle, _toi)) = physics_world.query_pipeline.cast_ray(
                &physics_world.rigid_body_set,
                &physics_world.collider_set,
                &ray,
                camera.far,
                true,
                QueryFilter::default().exclude_sensors(),
            ) {
                // Get the collider that was hit
                if let Some(collider) = physics_world.collider_set.get(handle) {
                    // Get the rigid body associated with this collider
                    if let Some(parent_handle) = collider.parent() {
                        // Get the entity from the rigid body
                        if let Some(entity) = physics_world.get_entity_from_handle(parent_handle) {
                            // Look up the semantic class for this entity
                            if let Some(&entity_class) = entity_to_class.get(&entity) {
                                class_id = entity_class;
                            }
                        }
                    }
                }
            }

            // Set the pixel value
            image.set_pixel(x, y, class_id);
        }
    }
}

/// Predefined semantic class IDs for common categories
pub mod classes {
    use super::ClassId;

    pub const UNLABELED: ClassId = 0;
    pub const BUILDING: ClassId = 1;
    pub const FENCE: ClassId = 2;
    pub const PERSON: ClassId = 3;
    pub const POLE: ClassId = 4;
    pub const ROAD: ClassId = 5;
    pub const SIDEWALK: ClassId = 6;
    pub const VEGETATION: ClassId = 7;
    pub const VEHICLE: ClassId = 8;
    pub const WALL: ClassId = 9;
    pub const TRAFFIC_SIGN: ClassId = 10;
    pub const SKY: ClassId = 11;
    pub const GROUND: ClassId = 12;
    pub const BRIDGE: ClassId = 13;
    pub const RAIL_TRACK: ClassId = 14;
    pub const TRAFFIC_LIGHT: ClassId = 15;
    pub const TERRAIN: ClassId = 16;
    pub const RIDER: ClassId = 17;
    pub const CAR: ClassId = 18;
    pub const TRUCK: ClassId = 19;
    pub const BUS: ClassId = 20;
    pub const TRAIN: ClassId = 21;
    pub const MOTORCYCLE: ClassId = 22;
    pub const BICYCLE: ClassId = 23;
}

/// Initialize default semantic classes
pub fn initialize_default_classes(registry: &mut SemanticClassRegistry) {
    registry.register_class(classes::BUILDING, "building", Color::srgb(0.4, 0.4, 0.4));
    registry.register_class(classes::FENCE, "fence", Color::srgb(0.7, 0.5, 0.3));
    registry.register_class(classes::PERSON, "person", Color::srgb(1.0, 0.0, 0.0));
    registry.register_class(classes::POLE, "pole", Color::srgb(0.6, 0.6, 0.6));
    registry.register_class(classes::ROAD, "road", Color::srgb(0.5, 0.2, 0.5));
    registry.register_class(classes::SIDEWALK, "sidewalk", Color::srgb(0.9, 0.6, 1.0));
    registry.register_class(
        classes::VEGETATION,
        "vegetation",
        Color::srgb(0.0, 0.5, 0.0),
    );
    registry.register_class(classes::VEHICLE, "vehicle", Color::srgb(0.0, 0.0, 1.0));
    registry.register_class(classes::WALL, "wall", Color::srgb(0.6, 0.4, 0.2));
    registry.register_class(
        classes::TRAFFIC_SIGN,
        "traffic_sign",
        Color::srgb(1.0, 1.0, 0.0),
    );
    registry.register_class(classes::SKY, "sky", Color::srgb(0.3, 0.5, 1.0));
    registry.register_class(classes::GROUND, "ground", Color::srgb(0.4, 0.3, 0.2));
    registry.register_class(classes::CAR, "car", Color::srgb(0.0, 0.0, 0.8));
    registry.register_class(classes::PERSON, "person", Color::srgb(0.8, 0.0, 0.0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segmentation_camera_creation() {
        let camera = SegmentationCamera::new(320, 240);
        assert_eq!(camera.resolution, (320, 240));
        assert_eq!(camera.pixel_count(), 76800);
    }

    #[test]
    fn test_semantic_class_registry() {
        let mut registry = SemanticClassRegistry::new();

        let id = registry.auto_register("car", Color::srgb(0.0, 0.0, 1.0));
        assert_eq!(registry.get_name(id), Some("car"));
        assert_eq!(registry.get_id("car"), Some(id));
    }

    #[test]
    fn test_segmentation_image() {
        let mut image = SegmentationImage::new(10, 10);

        image.set_pixel(5, 5, 42);
        assert_eq!(image.get_pixel(5, 5), Some(42));
        assert_eq!(image.get_pixel(15, 15), None);
    }

    #[test]
    fn test_segmentation_histogram() {
        let mut image = SegmentationImage::new(10, 10);

        for i in 0..50 {
            image.class_ids[i] = 1;
        }
        for i in 50..100 {
            image.class_ids[i] = 2;
        }

        let histogram = image.get_histogram();
        assert_eq!(histogram.get(&1), Some(&50));
        assert_eq!(histogram.get(&2), Some(&50));
    }

    #[test]
    fn test_default_classes() {
        let mut registry = SemanticClassRegistry::new();
        initialize_default_classes(&mut registry);

        assert_eq!(registry.get_name(classes::PERSON), Some("person"));
        assert_eq!(registry.get_name(classes::VEHICLE), Some("vehicle"));
        assert!(registry.list_classes().len() > 10);
    }
}
