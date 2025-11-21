use bevy::prelude::*;
use rand::Rng;
use rapier3d::prelude::*;
use std::f32::consts::PI;

use crate::physics::world::PhysicsWorld;

#[derive(Component)]
pub struct Lidar3D {
    pub horizontal_rays: usize,
    pub vertical_rays: usize,
    pub horizontal_fov: f32,
    pub vertical_fov: f32,
    pub max_range: f32,
    pub min_range: f32,
    pub rate_hz: f32,
    pub noise_std: f32,
    pub last_update: f32,
}

impl Default for Lidar3D {
    fn default() -> Self {
        Self {
            horizontal_rays: 720,
            vertical_rays: 16,
            horizontal_fov: PI * 2.0,
            vertical_fov: PI / 6.0,
            max_range: 20.0,
            min_range: 0.1,
            rate_hz: 10.0,
            noise_std: 0.01,
            last_update: 0.0,
        }
    }
}

impl Lidar3D {
    pub fn new(horizontal_rays: usize, vertical_rays: usize) -> Self {
        Self {
            horizontal_rays,
            vertical_rays,
            ..default()
        }
    }

    pub fn with_range(mut self, min_range: f32, max_range: f32) -> Self {
        self.min_range = min_range;
        self.max_range = max_range;
        self
    }

    pub fn with_fov(mut self, horizontal_fov: f32, vertical_fov: f32) -> Self {
        self.horizontal_fov = horizontal_fov;
        self.vertical_fov = vertical_fov;
        self
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_noise(mut self, noise_std: f32) -> Self {
        self.noise_std = noise_std;
        self
    }

    pub fn update_time(&self) -> f32 {
        1.0 / self.rate_hz
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= self.update_time()
    }
}

#[derive(Component, Clone)]
pub struct PointCloud {
    pub points: Vec<Vec3>,
    pub intensities: Vec<f32>,
    pub timestamp: f32,
}

impl PointCloud {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            intensities: Vec::new(),
            timestamp: 0.0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            points: Vec::with_capacity(capacity),
            intensities: Vec::with_capacity(capacity),
            timestamp: 0.0,
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.intensities.clear();
    }

    pub fn add_point(&mut self, point: Vec3, intensity: f32) {
        self.points.push(point);
        self.intensities.push(intensity);
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

impl Default for PointCloud {
    fn default() -> Self {
        Self::new()
    }
}

pub fn lidar3d_update_system(
    time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut query: Query<(&mut Lidar3D, &mut PointCloud, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();

    for (mut lidar, mut point_cloud, transform) in query.iter_mut() {
        if !lidar.should_update(current_time) {
            continue;
        }

        lidar.last_update = current_time;

        // Clear previous point cloud
        point_cloud.clear();
        point_cloud.timestamp = current_time;

        // Get sensor pose
        let sensor_pos = transform.translation();
        let sensor_rot = transform.to_scale_rotation_translation().1;

        // Perform ray casting
        cast_lidar_rays(
            &lidar,
            sensor_pos,
            sensor_rot,
            &mut point_cloud,
            &mut physics_world,
        );
    }
}

fn cast_lidar_rays(
    lidar: &Lidar3D,
    position: Vec3,
    rotation: Quat,
    point_cloud: &mut PointCloud,
    physics_world: &mut PhysicsWorld,
) {
    let mut rng = rand::thread_rng();

    // Calculate angle increments
    let horizontal_step = if lidar.horizontal_rays > 1 {
        lidar.horizontal_fov / (lidar.horizontal_rays - 1) as f32
    } else {
        0.0
    };

    let vertical_step = if lidar.vertical_rays > 1 {
        lidar.vertical_fov / (lidar.vertical_rays - 1) as f32
    } else {
        0.0
    };

    let horizontal_start = -lidar.horizontal_fov / 2.0;
    let vertical_start = -lidar.vertical_fov / 2.0;

    // Convert position and rotation to nalgebra
    let ray_origin = point![position.x, position.y, position.z];
    let base_rotation = nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
        rotation.w, rotation.x, rotation.y, rotation.z,
    ));

    // Cast rays in a spherical pattern
    for v_idx in 0..lidar.vertical_rays {
        let vertical_angle = vertical_start + v_idx as f32 * vertical_step;

        for h_idx in 0..lidar.horizontal_rays {
            let horizontal_angle = horizontal_start + h_idx as f32 * horizontal_step;

            // Compute ray direction in sensor frame
            let dir_x = horizontal_angle.cos() * vertical_angle.cos();
            let dir_y = vertical_angle.sin();
            let dir_z = horizontal_angle.sin() * vertical_angle.cos();

            let local_dir = nalgebra::Vector3::new(dir_x, dir_y, dir_z);

            // Transform to world frame
            let world_dir = base_rotation * local_dir;
            let ray_dir = nalgebra::Unit::new_normalize(world_dir);

            // Create ray
            let ray = Ray::new(ray_origin, ray_dir.into_inner());

            // Cast ray
            if let Some((_handle, toi)) = physics_world.query_pipeline.cast_ray(
                &physics_world.rigid_body_set,
                &physics_world.collider_set,
                &ray,
                lidar.max_range,
                true,
                QueryFilter::default(),
            ) {
                if toi >= lidar.min_range {
                    // Add noise if configured
                    let noisy_toi = if lidar.noise_std > 0.0 {
                        let noise: f32 = rng.gen_range(-lidar.noise_std..lidar.noise_std);
                        (toi + noise).clamp(lidar.min_range, lidar.max_range)
                    } else {
                        toi
                    };

                    // Compute 3D point in world coordinates
                    let hit_point = ray.point_at(noisy_toi);
                    let point = Vec3::new(hit_point.x, hit_point.y, hit_point.z);

                    // Compute intensity based on distance (inverse square law)
                    let intensity = 1.0 / (1.0 + (noisy_toi / lidar.max_range).powi(2));

                    point_cloud.add_point(point, intensity);
                }
            }
        }
    }
}

pub fn visualize_lidar_system(
    mut gizmos: Gizmos,
    query: Query<(&Lidar3D, &PointCloud, &GlobalTransform)>,
) {
    for (_lidar, point_cloud, transform) in query.iter() {
        let sensor_pos = transform.translation();

        // Draw sensor origin
        gizmos.sphere(sensor_pos, 0.1, Color::srgb(1.0, 0.0, 0.0));

        // Draw point cloud
        for point in &point_cloud.points {
            gizmos.sphere(*point, 0.02, Color::srgb(0.0, 1.0, 0.0));
        }
    }
}

#[derive(Component)]
pub struct Lidar2D {
    pub num_rays: usize,
    pub fov: f32,
    pub max_range: f32,
    pub min_range: f32,
    pub rate_hz: f32,
    pub noise_std: f32,
    pub last_update: f32,
}

impl Default for Lidar2D {
    fn default() -> Self {
        Self {
            num_rays: 360,
            fov: PI * 2.0,
            max_range: 10.0,
            min_range: 0.1,
            rate_hz: 10.0,
            noise_std: 0.01,
            last_update: 0.0,
        }
    }
}

impl Lidar2D {
    pub fn new(num_rays: usize, fov: f32, max_range: f32) -> Self {
        Self {
            num_rays,
            fov,
            max_range,
            ..default()
        }
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }
}

#[derive(Component, Clone)]
pub struct LaserScan {
    pub ranges: Vec<f32>,
    pub intensities: Vec<f32>,
    pub angle_min: f32,
    pub angle_max: f32,
    pub angle_increment: f32,
    pub range_min: f32,
    pub range_max: f32,
    pub timestamp: f32,
}

impl LaserScan {
    pub fn new(num_rays: usize, fov: f32, min_range: f32, max_range: f32) -> Self {
        let angle_min = -fov / 2.0;
        let angle_max = fov / 2.0;
        let angle_increment = fov / (num_rays - 1) as f32;

        Self {
            ranges: vec![max_range; num_rays],
            intensities: vec![0.0; num_rays],
            angle_min,
            angle_max,
            angle_increment,
            range_min: min_range,
            range_max: max_range,
            timestamp: 0.0,
        }
    }

    pub fn clear(&mut self) {
        for range in &mut self.ranges {
            *range = self.range_max;
        }
        for intensity in &mut self.intensities {
            *intensity = 0.0;
        }
    }
}

pub fn lidar2d_update_system(
    time: Res<Time>,
    mut physics_world: ResMut<PhysicsWorld>,
    mut query: Query<(&mut Lidar2D, &mut LaserScan, &GlobalTransform)>,
) {
    let current_time = time.elapsed_secs();

    for (mut lidar, mut scan, transform) in query.iter_mut() {
        if !lidar.should_update(current_time) {
            continue;
        }

        lidar.last_update = current_time;
        scan.clear();
        scan.timestamp = current_time;

        // Get sensor pose (assume 2D lidar scans in XZ plane, Y-up)
        let sensor_pos = transform.translation();
        let sensor_rot = transform.to_scale_rotation_translation().1;

        cast_lidar2d_rays(
            &lidar,
            sensor_pos,
            sensor_rot,
            &mut scan,
            &mut physics_world,
        );
    }
}

fn cast_lidar2d_rays(
    lidar: &Lidar2D,
    position: Vec3,
    rotation: Quat,
    scan: &mut LaserScan,
    physics_world: &mut PhysicsWorld,
) {
    let mut rng = rand::thread_rng();

    let ray_origin = point![position.x, position.y, position.z];
    let base_rotation = nalgebra::UnitQuaternion::new_normalize(nalgebra::Quaternion::new(
        rotation.w, rotation.x, rotation.y, rotation.z,
    ));

    for i in 0..lidar.num_rays {
        let angle = scan.angle_min + i as f32 * scan.angle_increment;

        // 2D ray in XZ plane
        let local_dir = nalgebra::Vector3::new(angle.cos(), 0.0, angle.sin());
        let world_dir = base_rotation * local_dir;
        let ray_dir = nalgebra::Unit::new_normalize(world_dir);

        let ray = Ray::new(ray_origin, ray_dir.into_inner());

        if let Some((_handle, toi)) = physics_world.query_pipeline.cast_ray(
            &physics_world.rigid_body_set,
            &physics_world.collider_set,
            &ray,
            lidar.max_range,
            true,
            QueryFilter::default(),
        ) {
            if toi >= lidar.min_range {
                let noisy_toi = if lidar.noise_std > 0.0 {
                    let noise: f32 = rng.gen_range(-lidar.noise_std..lidar.noise_std);
                    (toi + noise).clamp(lidar.min_range, lidar.max_range)
                } else {
                    toi
                };

                scan.ranges[i] = noisy_toi;
                scan.intensities[i] = 1.0 / (1.0 + (noisy_toi / lidar.max_range).powi(2));
            }
        }
    }
}

pub fn visualize_lidar2d_system(
    mut gizmos: Gizmos,
    query: Query<(&Lidar2D, &LaserScan, &GlobalTransform)>,
) {
    for (_lidar, scan, transform) in query.iter() {
        let sensor_pos = transform.translation();
        let sensor_rot = transform.to_scale_rotation_translation().1;

        // Draw sensor origin
        gizmos.sphere(sensor_pos, 0.05, Color::srgb(1.0, 0.0, 0.0));

        // Draw laser rays
        for (i, &range) in scan.ranges.iter().enumerate() {
            if range < scan.range_max {
                let angle = scan.angle_min + i as f32 * scan.angle_increment;
                let local_dir = Vec3::new(angle.cos(), 0.0, angle.sin()) * range;
                let world_dir = sensor_rot * local_dir;
                let end_point = sensor_pos + world_dir;

                gizmos.line(sensor_pos, end_point, Color::srgb(0.0, 1.0, 0.0));
            }
        }
    }
}
