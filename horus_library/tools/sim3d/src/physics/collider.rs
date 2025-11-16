use bevy::prelude::*;
use rapier3d::prelude::*;

#[derive(Component)]
pub struct PhysicsCollider {
    pub handle: ColliderHandle,
}

pub enum ColliderShape {
    Box { half_extents: Vec3 },
    Sphere { radius: f32 },
    Capsule { half_height: f32, radius: f32 },
    Cylinder { half_height: f32, radius: f32 },
    Mesh { vertices: Vec<Vec3>, indices: Vec<[u32; 3]> },
}

pub struct ColliderBuilder {
    shape: ColliderShape,
    friction: f32,
    restitution: f32,
    density: Option<f32>,
    is_sensor: bool,
}

impl ColliderBuilder {
    pub fn new(shape: ColliderShape) -> Self {
        Self {
            shape,
            friction: 0.5,
            restitution: 0.0,
            density: None,
            is_sensor: false,
        }
    }

    pub fn friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub fn restitution(mut self, restitution: f32) -> Self {
        self.restitution = restitution;
        self
    }

    pub fn density(mut self, density: f32) -> Self {
        self.density = Some(density);
        self
    }

    pub fn sensor(mut self, is_sensor: bool) -> Self {
        self.is_sensor = is_sensor;
        self
    }

    pub fn build(self) -> Collider {
        let shape = match self.shape {
            ColliderShape::Box { half_extents } => SharedShape::cuboid(
                half_extents.x,
                half_extents.y,
                half_extents.z,
            ),
            ColliderShape::Sphere { radius } => SharedShape::ball(radius),
            ColliderShape::Capsule { half_height, radius } => {
                SharedShape::capsule_y(half_height, radius)
            }
            ColliderShape::Cylinder { half_height, radius } => {
                SharedShape::cylinder(half_height, radius)
            }
            ColliderShape::Mesh { vertices, indices } => {
                let vertices: Vec<_> = vertices
                    .iter()
                    .map(|v| nalgebra::Point3::new(v.x, v.y, v.z))
                    .collect();
                let indices: Vec<_> = indices
                    .iter()
                    .map(|i| [i[0], i[1], i[2]])
                    .collect();
                SharedShape::trimesh(vertices, indices)
            }
        };

        let mut collider = rapier3d::prelude::ColliderBuilder::new(shape)
            .friction(self.friction)
            .restitution(self.restitution)
            .sensor(self.is_sensor);

        if let Some(density) = self.density {
            collider = collider.density(density);
        }

        collider.build()
    }
}

pub fn create_box_collider(half_extents: Vec3) -> Collider {
    ColliderBuilder::new(ColliderShape::Box { half_extents }).build()
}

pub fn create_sphere_collider(radius: f32) -> Collider {
    ColliderBuilder::new(ColliderShape::Sphere { radius }).build()
}

pub fn create_capsule_collider(half_height: f32, radius: f32) -> Collider {
    ColliderBuilder::new(ColliderShape::Capsule { half_height, radius }).build()
}

pub fn create_cylinder_collider(half_height: f32, radius: f32) -> Collider {
    ColliderBuilder::new(ColliderShape::Cylinder { half_height, radius }).build()
}

pub fn create_mesh_collider(vertices: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Collider {
    ColliderBuilder::new(ColliderShape::Mesh { vertices, indices }).build()
}

pub fn create_ground_collider(size_x: f32, size_z: f32) -> Collider {
    create_box_collider(Vec3::new(size_x / 2.0, 0.1, size_z / 2.0))
}
