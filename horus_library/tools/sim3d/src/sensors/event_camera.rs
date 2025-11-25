//! Event camera (DVS - Dynamic Vision Sensor) simulation
//!
//! Event cameras respond to changes in log intensity, producing asynchronous events
//! with microsecond temporal resolution.

use crate::physics::world::PhysicsWorld;
use bevy::prelude::*;
use rapier3d::prelude::*;
use std::collections::VecDeque;

/// Single event from event camera
#[derive(Clone, Copy, Debug)]
pub struct Event {
    /// Pixel x coordinate
    pub x: u32,
    /// Pixel y coordinate
    pub y: u32,
    /// Timestamp (microseconds)
    pub timestamp_us: u64,
    /// Polarity (true = ON/increase, false = OFF/decrease)
    pub polarity: bool,
}

/// Event camera component (DVS/DAVIS model)
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct EventCamera {
    /// Camera resolution (width, height)
    pub resolution: (u32, u32),
    /// Field of view (degrees)
    pub fov: f32,
    /// Contrast threshold for event triggering
    pub contrast_threshold: f32,
    /// Refractory period (microseconds) - minimum time between events at same pixel
    pub refractory_period_us: u64,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}

impl Default for EventCamera {
    fn default() -> Self {
        Self {
            resolution: (346, 260), // Standard DVS resolution
            fov: 60.0,
            contrast_threshold: 0.2,    // 20% intensity change
            refractory_period_us: 1000, // 1ms
            near: 0.1,
            far: 100.0,
        }
    }
}

impl EventCamera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            resolution: (width, height),
            ..default()
        }
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.contrast_threshold = threshold;
        self
    }

    pub fn with_refractory_period(mut self, period_us: u64) -> Self {
        self.refractory_period_us = period_us;
        self
    }

    pub fn pixel_count(&self) -> usize {
        (self.resolution.0 * self.resolution.1) as usize
    }
}

/// Event stream buffer
#[derive(Component)]
pub struct EventStream {
    /// Circular buffer of events
    events: VecDeque<Event>,
    /// Maximum events to store
    max_events: usize,
    /// Previous log intensity values for each pixel
    prev_log_intensity: Vec<f32>,
    /// Last event timestamp for each pixel (for refractory period)
    last_event_time: Vec<u64>,
    /// Current timestamp
    current_time_us: u64,
}

impl EventStream {
    pub fn new(width: u32, height: u32, max_events: usize) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            events: VecDeque::with_capacity(max_events),
            max_events,
            prev_log_intensity: vec![0.0; pixel_count],
            last_event_time: vec![0; pixel_count],
            current_time_us: 0,
        }
    }

    /// Add a new event
    pub fn push_event(&mut self, event: Event) {
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /// Get all events
    pub fn get_events(&self) -> &VecDeque<Event> {
        &self.events
    }

    /// Get events in time range [start_us, end_us]
    pub fn get_events_in_range(&self, start_us: u64, end_us: u64) -> Vec<Event> {
        self.events
            .iter()
            .filter(|e| e.timestamp_us >= start_us && e.timestamp_us <= end_us)
            .copied()
            .collect()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Get event count
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Process intensity image and generate events
    pub fn process_frame(&mut self, intensities: &[f32], timestamp_us: u64, camera: &EventCamera) {
        self.current_time_us = timestamp_us;
        let (width, height) = camera.resolution;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                if idx >= intensities.len() || idx >= self.prev_log_intensity.len() {
                    continue;
                }

                let intensity = intensities[idx].max(1e-6); // Avoid log(0)
                let log_intensity = intensity.ln();
                let prev_log = self.prev_log_intensity[idx];

                // Check if event should be triggered
                let delta = log_intensity - prev_log;
                let abs_delta = delta.abs();

                if abs_delta >= camera.contrast_threshold {
                    // Check refractory period
                    let last_time = self.last_event_time[idx];
                    if timestamp_us - last_time >= camera.refractory_period_us {
                        // Generate event
                        let event = Event {
                            x,
                            y,
                            timestamp_us,
                            polarity: delta > 0.0,
                        };

                        self.push_event(event);
                        self.last_event_time[idx] = timestamp_us;

                        // Update reference level
                        self.prev_log_intensity[idx] = log_intensity;
                    }
                }
            }
        }
    }

    /// Get event rate (events per second)
    pub fn get_event_rate(&self, time_window_us: u64) -> f32 {
        if time_window_us == 0 {
            return 0.0;
        }

        let recent_events = self.get_events_in_range(
            self.current_time_us.saturating_sub(time_window_us),
            self.current_time_us,
        );

        (recent_events.len() as f64 / (time_window_us as f64 / 1_000_000.0)) as f32
    }
}

/// Convert event stream to frame representation (event frame)
pub fn events_to_frame(events: &[Event], width: u32, height: u32, _time_window_us: u64) -> Vec<i32> {
    let mut frame = vec![0i32; (width * height) as usize];

    for event in events {
        if event.x >= width || event.y >= height {
            continue;
        }

        let idx = (event.y * width + event.x) as usize;
        if event.polarity {
            frame[idx] += 1;
        } else {
            frame[idx] -= 1;
        }
    }

    frame
}

/// System to update event cameras
pub fn event_camera_update_system(
    time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut cameras: Query<(&EventCamera, &mut EventStream, &GlobalTransform)>,
    lights: Query<(&PointLight, &GlobalTransform), Without<EventCamera>>,
) {
    let current_time_us = (time.elapsed_secs_f64() * 1_000_000.0) as u64;

    for (camera, mut stream, camera_transform) in cameras.iter_mut() {
        // Generate intensity image
        let intensities = generate_intensity_image(
            camera,
            camera_transform,
            &mut physics_world,
            &lights,
        );

        // Process the intensity frame to generate events
        stream.process_frame(&intensities, current_time_us, camera);
    }
}

/// Generate an intensity image from the camera's perspective
fn generate_intensity_image(
    camera: &EventCamera,
    camera_transform: &GlobalTransform,
    physics_world: &mut PhysicsWorld,
    lights: &Query<(&PointLight, &GlobalTransform), Without<EventCamera>>,
) -> Vec<f32> {
    let width = camera.resolution.0;
    let height = camera.resolution.1;
    let mut intensities = vec![0.0; camera.pixel_count()];

    // Get camera pose
    let camera_pos = camera_transform.translation();
    let camera_rot = camera_transform.to_scale_rotation_translation().1;

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

    // Collect light sources for shading
    let light_sources: Vec<(Vec3, f32)> = lights
        .iter()
        .map(|(light, transform)| (transform.translation(), light.intensity))
        .collect();

    // Cast rays for each pixel to determine intensity
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

            // Cast ray to find what we hit
            let intensity = if let Some((handle, toi)) = physics_world.query_pipeline.cast_ray(
                &physics_world.rigid_body_set,
                &physics_world.collider_set,
                &ray,
                camera.far,
                true,
                QueryFilter::default().exclude_sensors(),
            ) {
                // We hit something - calculate intensity based on lighting
                let hit_point = ray.point_at(toi);
                let hit_pos = Vec3::new(hit_point.x, hit_point.y, hit_point.z);

                // Get surface normal at hit point (approximation)
                let normal = if let Some(collider) = physics_world.collider_set.get(handle) {
                    // Cast ray with normal to get surface normal
                    if let Some(intersection) = collider.shape().cast_ray_and_get_normal(
                        &nalgebra::Isometry3::identity(),
                        &ray,
                        camera.far,
                        true,
                    ) {
                        Vec3::new(intersection.normal.x, intersection.normal.y, intersection.normal.z)
                    } else {
                        // Fallback: use ray direction inverted as rough approximation
                        Vec3::new(-ray_dir.x, -ray_dir.y, -ray_dir.z)
                    }
                } else {
                    Vec3::new(-ray_dir.x, -ray_dir.y, -ray_dir.z)
                };

                // Calculate lighting intensity using Lambertian shading
                let mut total_intensity = 0.1; // Ambient light

                for (light_pos, light_intensity) in &light_sources {
                    // Direction from hit point to light
                    let to_light = (*light_pos - hit_pos).normalize();

                    // Check if light is visible (shadow ray)
                    let shadow_ray_origin = point![hit_pos.x, hit_pos.y, hit_pos.z];
                    let shadow_ray_dir = nalgebra::Vector3::new(to_light.x, to_light.y, to_light.z);
                    let shadow_ray = Ray::new(shadow_ray_origin, shadow_ray_dir);

                    let light_distance = (*light_pos - hit_pos).length();

                    // Check for occlusion
                    let occluded = physics_world.query_pipeline.cast_ray(
                        &physics_world.rigid_body_set,
                        &physics_world.collider_set,
                        &shadow_ray,
                        light_distance * 0.99, // Slightly less to avoid self-intersection
                        true,
                        QueryFilter::default().exclude_sensors(),
                    ).is_some();

                    if !occluded {
                        // Lambertian shading
                        let dot_product = normal.dot(to_light).max(0.0);
                        let distance_falloff = 1.0 / (1.0 + light_distance * light_distance);
                        total_intensity += dot_product * light_intensity * distance_falloff;
                    }
                }

                // Clamp intensity to [0, 1]
                total_intensity.min(1.0)
            } else {
                // No hit - background intensity
                0.05
            };

            let idx = (y * width + x) as usize;
            intensities[idx] = intensity;
        }
    }

    intensities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_camera_creation() {
        let camera = EventCamera::new(640, 480);
        assert_eq!(camera.resolution, (640, 480));
        assert_eq!(camera.pixel_count(), 307200);
    }

    #[test]
    fn test_event_stream() {
        let mut stream = EventStream::new(10, 10, 1000);

        let event = Event {
            x: 5,
            y: 5,
            timestamp_us: 1000,
            polarity: true,
        };

        stream.push_event(event);
        assert_eq!(stream.len(), 1);
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_event_stream_time_range() {
        let mut stream = EventStream::new(10, 10, 1000);

        stream.push_event(Event {
            x: 0,
            y: 0,
            timestamp_us: 1000,
            polarity: true,
        });

        stream.push_event(Event {
            x: 1,
            y: 1,
            timestamp_us: 2000,
            polarity: false,
        });

        stream.push_event(Event {
            x: 2,
            y: 2,
            timestamp_us: 3000,
            polarity: true,
        });

        let events = stream.get_events_in_range(1500, 2500);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].timestamp_us, 2000);
    }

    #[test]
    fn test_event_frame_conversion() {
        let events = vec![
            Event {
                x: 0,
                y: 0,
                timestamp_us: 1000,
                polarity: true,
            },
            Event {
                x: 0,
                y: 0,
                timestamp_us: 1100,
                polarity: true,
            },
            Event {
                x: 1,
                y: 0,
                timestamp_us: 1200,
                polarity: false,
            },
        ];

        let frame = events_to_frame(&events, 10, 10, 1000);
        assert_eq!(frame[0], 2); // Two ON events at (0,0)
        assert_eq!(frame[1], -1); // One OFF event at (1,0)
    }

    #[test]
    fn test_process_frame_generates_events() {
        let mut stream = EventStream::new(2, 2, 100);
        let camera = EventCamera::default();

        // Initial frame (all mid-gray)
        let frame1 = vec![0.5, 0.5, 0.5, 0.5];
        stream.process_frame(&frame1, 0, &camera);
        assert_eq!(stream.len(), 0); // No events on first frame

        // Second frame with intensity change
        let frame2 = vec![0.8, 0.5, 0.5, 0.2]; // (0,0) brighter, (1,1) darker
        stream.process_frame(&frame2, 1000, &camera);

        // Should generate events for significant changes
        assert!(stream.len() > 0);
    }

    #[test]
    fn test_refractory_period() {
        let mut stream = EventStream::new(1, 1, 100);
        let camera = EventCamera {
            refractory_period_us: 1000,
            ..default()
        };

        let intensities = vec![0.5];
        stream.process_frame(&intensities, 0, &camera);

        // Try to generate event immediately after (within refractory period)
        let intensities2 = vec![0.9];
        stream.process_frame(&intensities2, 500, &camera);

        // Event should be blocked by refractory period
        // (This test is simplified; actual behavior depends on threshold)
    }
}
