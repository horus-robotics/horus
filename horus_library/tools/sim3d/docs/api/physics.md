# Physics API Reference

This document provides a comprehensive reference for Sim3D's physics system built on Rapier3D.

## PhysicsWorld

The central resource managing all physics simulation state.

### Structure

```rust
#[derive(Resource)]
pub struct PhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhaseMultiSap,
    pub narrow_phase: NarrowPhase,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
    pub gravity: Vector<f32>,
}
```

### Methods

#### Creating and Configuring

```rust
impl PhysicsWorld {
    /// Create a new physics world with default settings
    pub fn new() -> Self {
        Self::with_gravity(vector![0.0, -9.81, 0.0])
    }

    /// Create with custom gravity
    pub fn with_gravity(gravity: Vector<f32>) -> Self;

    /// Set gravity vector
    pub fn set_gravity(&mut self, gravity: Vector<f32>) {
        self.gravity = gravity;
    }

    /// Configure integration parameters
    pub fn set_timestep(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;
    }

    /// Set solver iterations
    pub fn set_solver_iterations(&mut self, velocity_iters: usize, position_iters: usize) {
        self.integration_parameters.num_solver_iterations = velocity_iters;
        self.integration_parameters.num_additional_friction_iterations = position_iters;
    }
}
```

#### Spawning Bodies

```rust
impl PhysicsWorld {
    /// Spawn a rigid body and associate with entity
    pub fn spawn_rigid_body(&mut self, body: RigidBody, entity: Entity) -> RigidBodyHandle {
        // Store entity in user_data for later retrieval
        let mut body = body;
        body.user_data = entity.to_bits() as u128;
        self.rigid_body_set.insert(body)
    }

    /// Spawn a collider attached to a rigid body
    pub fn spawn_collider(
        &mut self,
        collider: Collider,
        parent: RigidBodyHandle,
    ) -> ColliderHandle {
        self.collider_set.insert_with_parent(
            collider,
            parent,
            &mut self.rigid_body_set,
        )
    }

    /// Remove a rigid body and all attached colliders
    pub fn remove_rigid_body(&mut self, handle: RigidBodyHandle) {
        self.rigid_body_set.remove(
            handle,
            &mut self.island_manager,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            true,
        );
    }
}
```

#### Simulation

```rust
impl PhysicsWorld {
    /// Step the physics simulation
    pub fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );

        // Update query pipeline after step
        self.query_pipeline.update(
            &self.rigid_body_set,
            &self.collider_set,
        );
    }

    /// Step with custom timestep
    pub fn step_with_dt(&mut self, dt: f32) {
        let prev_dt = self.integration_parameters.dt;
        self.integration_parameters.dt = dt;
        self.step();
        self.integration_parameters.dt = prev_dt;
    }
}
```

#### Queries

```rust
impl PhysicsWorld {
    /// Cast a ray and find first intersection
    pub fn cast_ray(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<(ColliderHandle, f32)> {
        let ray = Ray::new(
            nalgebra::Point3::new(origin.x, origin.y, origin.z),
            nalgebra::Vector3::new(direction.x, direction.y, direction.z),
        );

        self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_distance,
            true,
            QueryFilter::default(),
        )
    }

    /// Cast a ray and get all intersections
    pub fn cast_ray_all(
        &self,
        origin: Vec3,
        direction: Vec3,
        max_distance: f32,
    ) -> Vec<(ColliderHandle, f32)> {
        let ray = Ray::new(
            nalgebra::Point3::new(origin.x, origin.y, origin.z),
            nalgebra::Vector3::new(direction.x, direction.y, direction.z),
        );

        let mut hits = Vec::new();
        self.query_pipeline.intersections_with_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_distance,
            true,
            QueryFilter::default(),
            |handle, intersection| {
                hits.push((handle, intersection.time_of_impact));
                true // Continue searching
            },
        );

        hits
    }

    /// Query point intersection
    pub fn point_intersections(&self, point: Vec3) -> Vec<ColliderHandle> {
        let point = nalgebra::Point3::new(point.x, point.y, point.z);
        let mut handles = Vec::new();

        self.query_pipeline.intersections_with_point(
            &self.rigid_body_set,
            &self.collider_set,
            &point,
            QueryFilter::default(),
            |handle| {
                handles.push(handle);
                true
            },
        );

        handles
    }

    /// Shape cast (sweep test)
    pub fn cast_shape(
        &self,
        shape: &dyn Shape,
        origin: &Isometry<f32>,
        direction: Vec3,
        max_distance: f32,
    ) -> Option<(ColliderHandle, f32)> {
        let dir = nalgebra::Vector3::new(direction.x, direction.y, direction.z);

        self.query_pipeline.cast_shape(
            &self.rigid_body_set,
            &self.collider_set,
            origin,
            &dir,
            shape,
            ShapeCastOptions::with_max_time_of_impact(max_distance),
            QueryFilter::default(),
        ).map(|(handle, hit)| (handle, hit.time_of_impact))
    }
}
```

## Rigid Bodies

### RigidBody Types

```rust
// Dynamic body - affected by forces and gravity
let dynamic_body = RigidBodyBuilder::dynamic()
    .translation(vector![0.0, 5.0, 0.0])
    .build();

// Static body - never moves
let static_body = RigidBodyBuilder::fixed()
    .translation(vector![0.0, 0.0, 0.0])
    .build();

// Kinematic position-based - moved by setting position
let kinematic_pos = RigidBodyBuilder::kinematic_position_based()
    .translation(vector![0.0, 1.0, 0.0])
    .build();

// Kinematic velocity-based - moved by setting velocity
let kinematic_vel = RigidBodyBuilder::kinematic_velocity_based()
    .translation(vector![0.0, 1.0, 0.0])
    .build();
```

### RigidBody Configuration

```rust
let body = RigidBodyBuilder::dynamic()
    // Initial position
    .translation(vector![x, y, z])
    // Initial rotation (quaternion)
    .rotation(vector![roll, pitch, yaw])
    // Initial linear velocity
    .linvel(vector![vx, vy, vz])
    // Initial angular velocity
    .angvel(vector![wx, wy, wz])
    // Linear damping (air resistance)
    .linear_damping(0.5)
    // Angular damping (rotational resistance)
    .angular_damping(1.0)
    // Additional mass (adds to collider-computed mass)
    .additional_mass(10.0)
    // Can this body sleep when inactive?
    .can_sleep(true)
    // Is this body affected by gravity?
    .gravity_scale(1.0)
    // Continuous collision detection
    .ccd_enabled(true)
    // Restrict motion to specific axes
    .locked_axes(LockedAxes::ROTATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z)
    .build();
```

### Accessing RigidBody State

```rust
// Get position and rotation
let position = rb.position();
let translation = position.translation;  // Vector3
let rotation = position.rotation;        // UnitQuaternion

// Get velocities
let linear_velocity = rb.linvel();
let angular_velocity = rb.angvel();

// Get mass properties
let mass = rb.mass();
let center_of_mass = rb.center_of_mass();
let inertia = rb.mass_properties().principal_inertia();

// Check state
let is_sleeping = rb.is_sleeping();
let is_dynamic = rb.is_dynamic();
let is_kinematic = rb.is_kinematic();
```

### Modifying RigidBody State

```rust
// Set position directly (teleport)
rb.set_position(Isometry::new(
    vector![x, y, z],
    vector![roll, pitch, yaw],
), true);

// Set velocity
rb.set_linvel(vector![vx, vy, vz], true);
rb.set_angvel(vector![wx, wy, wz], true);

// Apply forces (accumulated until next step)
rb.add_force(vector![fx, fy, fz], true);
rb.add_torque(vector![tx, ty, tz], true);

// Apply force at specific point
rb.add_force_at_point(
    vector![fx, fy, fz],
    nalgebra::Point3::new(px, py, pz),
    true,
);

// Apply impulse (instant velocity change)
rb.apply_impulse(vector![ix, iy, iz], true);
rb.apply_torque_impulse(vector![tx, ty, tz], true);

// Wake up sleeping body
rb.wake_up(true);

// Put body to sleep
rb.sleep();
```

## Collision Shapes

### ColliderShape Enum

```rust
pub enum ColliderShape {
    /// Box/cuboid shape
    Box { half_extents: Vec3 },

    /// Sphere shape
    Sphere { radius: f32 },

    /// Capsule (cylinder with hemispherical caps)
    Capsule { half_height: f32, radius: f32 },

    /// Cylinder
    Cylinder { half_height: f32, radius: f32 },

    /// Triangle mesh (for complex geometry)
    Mesh {
        vertices: Vec<Vec3>,
        indices: Vec<[u32; 3]>,
    },
}
```

### Creating Colliders

```rust
// Box collider
let box_collider = ColliderBuilder::new(ColliderShape::Box {
    half_extents: Vec3::new(0.5, 0.5, 0.5),
})
.friction(0.5)
.restitution(0.3)
.density(1.0)
.build();

// Sphere collider
let sphere_collider = ColliderBuilder::new(ColliderShape::Sphere {
    radius: 0.5,
})
.friction(0.3)
.restitution(0.8)
.build();

// Capsule collider
let capsule_collider = ColliderBuilder::new(ColliderShape::Capsule {
    half_height: 0.5,
    radius: 0.2,
})
.build();

// Cylinder collider
let cylinder_collider = ColliderBuilder::new(ColliderShape::Cylinder {
    half_height: 1.0,
    radius: 0.3,
})
.build();

// Mesh collider (for complex shapes)
let mesh_collider = ColliderBuilder::new(ColliderShape::Mesh {
    vertices: vec![
        Vec3::new(-1.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, -1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 0.0),
    ],
    indices: vec![
        [0, 1, 2],  // Base
        [0, 1, 3],  // Side 1
        [1, 2, 3],  // Side 2
        [2, 0, 3],  // Side 3
    ],
})
.build();
```

### ColliderBuilder Methods

```rust
impl ColliderBuilder {
    /// Create new builder with shape
    pub fn new(shape: ColliderShape) -> Self;

    /// Set friction coefficient (0.0 - 1.0+)
    pub fn friction(mut self, friction: f32) -> Self;

    /// Set restitution/bounciness (0.0 - 1.0)
    pub fn restitution(mut self, restitution: f32) -> Self;

    /// Set density (kg/m^3, used for mass calculation)
    pub fn density(mut self, density: f32) -> Self;

    /// Make this collider a sensor (no physical response)
    pub fn sensor(mut self, is_sensor: bool) -> Self;

    /// Set collision groups
    pub fn collision_groups(mut self, groups: InteractionGroups) -> Self;

    /// Set solver groups
    pub fn solver_groups(mut self, groups: InteractionGroups) -> Self;

    /// Build the collider
    pub fn build(self) -> Collider;
}
```

### Helper Functions

```rust
/// Create a box collider
pub fn create_box_collider(half_extents: Vec3) -> Collider;

/// Create a sphere collider
pub fn create_sphere_collider(radius: f32) -> Collider;

/// Create a capsule collider (Y-axis aligned)
pub fn create_capsule_collider(half_height: f32, radius: f32) -> Collider;

/// Create a cylinder collider
pub fn create_cylinder_collider(half_height: f32, radius: f32) -> Collider;

/// Create a mesh collider from vertices and indices
pub fn create_mesh_collider(vertices: Vec<Vec3>, indices: Vec<[u32; 3]>) -> Collider;

/// Create a ground plane collider
pub fn create_ground_collider(size_x: f32, size_z: f32) -> Collider;
```

## Joint Types

### Joint Types Overview

| Type | DOF Locked | Use Case |
|------|------------|----------|
| Fixed | All 6 | Rigid attachment |
| Revolute | 5 | Hinges, wheels |
| Prismatic | 5 | Linear slides |
| Spherical | 3 | Ball joints |

### Creating Joints

```rust
use sim3d::physics::joints::*;

// Fixed joint (no relative motion)
let fixed_joint = create_fixed_joint(
    Vec3::new(0.0, 0.5, 0.0),  // Anchor on parent
    Vec3::new(0.0, -0.5, 0.0), // Anchor on child
);

// Revolute joint (rotation around axis)
let revolute_joint = create_revolute_joint(
    Vec3::new(0.0, 0.0, 0.0),  // Anchor on parent
    Vec3::new(0.0, 0.0, 0.0),  // Anchor on child
    Vec3::new(0.0, 1.0, 0.0),  // Rotation axis
);

// Revolute joint with limits
let limited_revolute = create_revolute_joint_with_limits(
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),
    -std::f32::consts::FRAC_PI_2,  // Min angle
    std::f32::consts::FRAC_PI_2,   // Max angle
);

// Prismatic joint (translation along axis)
let prismatic_joint = create_prismatic_joint(
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),  // Translation axis
);

// Prismatic joint with limits
let limited_prismatic = create_prismatic_joint_with_limits(
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 1.0, 0.0),
    0.0,   // Min distance
    1.0,   // Max distance
);

// Spherical joint (ball joint)
let spherical_joint = create_spherical_joint(
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(0.0, 0.0, 0.0),
);
```

### Adding Joints to Physics World

```rust
// Insert joint between two bodies
let joint_handle = physics_world.impulse_joint_set.insert(
    parent_body_handle,
    child_body_handle,
    joint_data,
    true, // Wake bodies
);

// Associate joint with entity
commands.entity(child_entity).insert(PhysicsJoint {
    handle: joint_handle,
    joint_type: JointType::Revolute,
});
```

### Joint Motors

```rust
// Get joint from physics world
if let Some(joint) = physics_world.impulse_joint_set.get_mut(joint_handle) {
    // Position motor (PD control)
    joint.data.set_motor_position(
        JointAxis::AngX,    // Axis
        target_angle,       // Target position
        100.0,              // Stiffness
        10.0,               // Damping
    );

    // Velocity motor
    joint.data.set_motor_velocity(
        JointAxis::AngX,
        target_velocity,    // Target velocity
        max_force,          // Maximum force/torque
    );

    // Set motor model
    joint.data.set_motor_model(JointAxis::AngX, MotorModel::ForceBased);
}
```

### Joint Springs

```rust
// Add spring behavior to joint
add_joint_spring(
    &mut joint,
    JointAxis::AngX,
    stiffness,  // Spring constant
    damping,    // Damping coefficient
);
```

## Advanced Physics

### Continuous Collision Detection (CCD)

For fast-moving objects that might tunnel through thin objects:

```rust
// Enable CCD on rigid body
let body = RigidBodyBuilder::dynamic()
    .ccd_enabled(true)
    .build();

// Configure CCD solver
physics_world.ccd_solver = CCDSolver::new();
```

### Collision Groups

Control which objects can collide:

```rust
// Define collision groups (up to 32 groups)
const GROUP_ROBOT: u32 = 0b0001;
const GROUP_OBSTACLE: u32 = 0b0010;
const GROUP_SENSOR: u32 = 0b0100;

// Robot collides with obstacles, not sensors
let robot_groups = InteractionGroups::new(
    GROUP_ROBOT.into(),                    // Member of
    (GROUP_OBSTACLE | GROUP_ROBOT).into(), // Collides with
);

// Sensor detects robot, doesn't collide physically
let sensor_groups = InteractionGroups::new(
    GROUP_SENSOR.into(),
    GROUP_ROBOT.into(),
);

let collider = ColliderBuilder::new(shape)
    .collision_groups(robot_groups)
    .build();
```

### Contact Events

Listen for collision events:

```rust
// Check for contacts on a collider
for contact_pair in physics_world.narrow_phase.contact_pairs() {
    let (collider1, collider2) = (contact_pair.collider1, contact_pair.collider2);

    if contact_pair.has_any_active_contact {
        for manifold in contact_pair.manifolds {
            for point in &manifold.points {
                let contact_point = point.local_p1;
                let normal = manifold.local_n1;
                let penetration = point.dist;

                println!("Contact at {:?}, normal: {:?}", contact_point, normal);
            }
        }
    }
}
```

### Integration Parameters

Fine-tune simulation accuracy vs performance:

```rust
let params = IntegrationParameters {
    // Timestep (smaller = more accurate, slower)
    dt: 1.0 / 60.0,

    // Minimum timestep for CCD
    min_ccd_dt: 1.0 / 60.0 / 100.0,

    // Velocity solver iterations (more = better stacking)
    num_solver_iterations: 4,

    // Additional friction iterations
    num_additional_friction_iterations: 4,

    // Position correction iterations
    num_internal_pgs_iterations: 1,

    // Error reduction parameter
    erp: 0.8,

    // Contact parameters
    allowed_linear_error: 0.001,
    prediction_distance: 0.002,

    ..Default::default()
};

physics_world.integration_parameters = params;
```

### Performance Tips

1. **Use simple collision shapes** when possible (spheres, boxes)
2. **Enable sleeping** for objects that come to rest
3. **Use collision groups** to reduce collision checks
4. **Batch physics queries** instead of many individual calls
5. **Adjust solver iterations** based on accuracy needs
6. **Consider CCD only for fast-moving objects**

```rust
// Example: Optimized physics setup
let mut physics_world = PhysicsWorld::new();

// Reduce solver iterations for better performance
physics_world.set_solver_iterations(2, 1);

// Use fixed timestep
physics_world.set_timestep(1.0 / 60.0);

// Enable sleeping
let body = RigidBodyBuilder::dynamic()
    .can_sleep(true)
    .sleeping(true)  // Start asleep
    .build();
```
