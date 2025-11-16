use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

#[derive(Component)]
pub struct RGBCamera {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub rate_hz: f32,
    pub last_update: f32,
}

impl Default for RGBCamera {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            fov: 60.0,
            near: 0.1,
            far: 100.0,
            rate_hz: 30.0,
            last_update: 0.0,
        }
    }
}

impl RGBCamera {
    pub fn new(width: u32, height: u32, fov: f32) -> Self {
        Self {
            width,
            height,
            fov,
            ..default()
        }
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_range(mut self, near: f32, far: f32) -> Self {
        self.near = near;
        self.far = far;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }
}

#[derive(Component)]
pub struct DepthCamera {
    pub width: u32,
    pub height: u32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub rate_hz: f32,
    pub last_update: f32,
}

impl Default for DepthCamera {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            fov: 60.0,
            near: 0.1,
            far: 10.0,
            rate_hz: 30.0,
            last_update: 0.0,
        }
    }
}

impl DepthCamera {
    pub fn new(width: u32, height: u32, fov: f32) -> Self {
        Self {
            width,
            height,
            fov,
            ..default()
        }
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rate_hz = rate_hz;
        self
    }

    pub fn with_range(mut self, near: f32, far: f32) -> Self {
        self.near = near;
        self.far = far;
        self
    }

    pub fn should_update(&self, current_time: f32) -> bool {
        current_time - self.last_update >= 1.0 / self.rate_hz
    }
}

#[derive(Component)]
pub struct RGBDCamera {
    pub rgb: RGBCamera,
    pub depth: DepthCamera,
}

impl Default for RGBDCamera {
    fn default() -> Self {
        Self {
            rgb: RGBCamera::default(),
            depth: DepthCamera::default(),
        }
    }
}

impl RGBDCamera {
    pub fn new(width: u32, height: u32, fov: f32) -> Self {
        Self {
            rgb: RGBCamera::new(width, height, fov),
            depth: DepthCamera::new(width, height, fov),
        }
    }

    pub fn with_rate(mut self, rate_hz: f32) -> Self {
        self.rgb = self.rgb.with_rate(rate_hz);
        self.depth = self.depth.with_rate(rate_hz);
        self
    }

    pub fn with_range(mut self, near: f32, far: f32) -> Self {
        self.rgb = self.rgb.with_range(near, far);
        self.depth = self.depth.with_range(near, far);
        self
    }
}

#[derive(Component)]
pub struct CameraImage {
    pub image_handle: Handle<Image>,
    pub width: u32,
    pub height: u32,
    pub timestamp: f32,
}

impl CameraImage {
    pub fn new(image_handle: Handle<Image>, width: u32, height: u32) -> Self {
        Self {
            image_handle,
            width,
            height,
            timestamp: 0.0,
        }
    }
}

#[derive(Component)]
pub struct DepthImage {
    pub image_handle: Handle<Image>,
    pub width: u32,
    pub height: u32,
    pub timestamp: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}

impl DepthImage {
    pub fn new(image_handle: Handle<Image>, width: u32, height: u32, min_depth: f32, max_depth: f32) -> Self {
        Self {
            image_handle,
            width,
            height,
            timestamp: 0.0,
            min_depth,
            max_depth,
        }
    }
}

pub fn setup_rgb_camera_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &RGBCamera), Added<RGBCamera>>,
) {
    for (entity, camera) in query.iter() {
        // Create render target image
        let size = Extent3d {
            width: camera.width,
            height: camera.height,
            depth_or_array_layers: 1,
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };

        image.resize(size);

        let image_handle = images.add(image);

        // Add camera components
        commands.entity(entity).insert((
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: camera.fov.to_radians(),
                near: camera.near,
                far: camera.far,
                aspect_ratio: camera.width as f32 / camera.height as f32,
            }),
            CameraImage::new(image_handle, camera.width, camera.height),
        ));
    }
}

pub fn setup_depth_camera_system(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &DepthCamera), Added<DepthCamera>>,
) {
    for (entity, camera) in query.iter() {
        // Create depth render target
        let size = Extent3d {
            width: camera.width,
            height: camera.height,
            depth_or_array_layers: 1,
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };

        image.resize(size);

        let image_handle = images.add(image);

        // Add camera components
        commands.entity(entity).insert((
            Camera3d::default(),
            Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: camera.fov.to_radians(),
                near: camera.near,
                far: camera.far,
                aspect_ratio: camera.width as f32 / camera.height as f32,
            }),
            DepthImage::new(image_handle, camera.width, camera.height, camera.near, camera.far),
        ));
    }
}

pub fn update_camera_timestamps_system(
    time: Res<Time>,
    mut rgb_query: Query<(&RGBCamera, &mut CameraImage)>,
    mut depth_query: Query<(&DepthCamera, &mut DepthImage)>,
) {
    let current_time = time.elapsed_secs();

    for (camera, mut image) in rgb_query.iter_mut() {
        if camera.should_update(current_time) {
            image.timestamp = current_time;
        }
    }

    for (camera, mut image) in depth_query.iter_mut() {
        if camera.should_update(current_time) {
            image.timestamp = current_time;
        }
    }
}

pub fn extract_camera_images_system(
    images: Res<Assets<Image>>,
    query: Query<(&CameraImage, &Name)>,
) {
    for (camera_image, name) in query.iter() {
        if let Some(_image) = images.get(&camera_image.image_handle) {
            // Image data is now available for processing
            // Can be exported to HORUS bridge or saved to disk
            // Access via: image.data (Vec<u8>)
        }
    }
}

#[derive(Component, Clone)]
pub struct CameraVisualization {
    pub show_frustum: bool,
    pub frustum_color: Color,
}

impl Default for CameraVisualization {
    fn default() -> Self {
        Self {
            show_frustum: true,
            frustum_color: Color::srgb(1.0, 1.0, 0.0),
        }
    }
}

pub fn visualize_camera_system(
    mut gizmos: Gizmos,
    query: Query<(&GlobalTransform, &RGBCamera, Option<&CameraVisualization>)>,
) {
    for (transform, camera, viz_opt) in query.iter() {
        let viz = viz_opt.cloned().unwrap_or_default();

        if !viz.show_frustum {
            continue;
        }

        let pos = transform.translation();
        let (_, rotation, _) = transform.to_scale_rotation_translation();

        // Calculate frustum corners
        let aspect = camera.width as f32 / camera.height as f32;
        let fov_rad = camera.fov.to_radians();
        let tan_half_fov = (fov_rad / 2.0).tan();

        let near_height = camera.near * tan_half_fov;
        let near_width = near_height * aspect;

        let far_height = camera.far * tan_half_fov * 0.2; // Scale down far plane for visualization
        let far_width = far_height * aspect;

        // Near plane corners
        let forward = rotation * Vec3::Z;
        let right = rotation * Vec3::X;
        let up = rotation * Vec3::Y;

        let near_center = pos + forward * camera.near;
        let near_tl = near_center + up * near_height - right * near_width;
        let near_tr = near_center + up * near_height + right * near_width;
        let near_bl = near_center - up * near_height - right * near_width;
        let near_br = near_center - up * near_height + right * near_width;

        // Far plane corners (scaled down for viz)
        let far_dist = camera.far * 0.2;
        let far_center = pos + forward * far_dist;
        let far_tl = far_center + up * far_height - right * far_width;
        let far_tr = far_center + up * far_height + right * far_width;
        let far_bl = far_center - up * far_height - right * far_width;
        let far_br = far_center - up * far_height + right * far_width;

        // Draw frustum
        // Near plane
        gizmos.line(near_tl, near_tr, viz.frustum_color);
        gizmos.line(near_tr, near_br, viz.frustum_color);
        gizmos.line(near_br, near_bl, viz.frustum_color);
        gizmos.line(near_bl, near_tl, viz.frustum_color);

        // Far plane
        gizmos.line(far_tl, far_tr, viz.frustum_color);
        gizmos.line(far_tr, far_br, viz.frustum_color);
        gizmos.line(far_br, far_bl, viz.frustum_color);
        gizmos.line(far_bl, far_tl, viz.frustum_color);

        // Connecting lines
        gizmos.line(near_tl, far_tl, viz.frustum_color);
        gizmos.line(near_tr, far_tr, viz.frustum_color);
        gizmos.line(near_bl, far_bl, viz.frustum_color);
        gizmos.line(near_br, far_br, viz.frustum_color);
    }
}
