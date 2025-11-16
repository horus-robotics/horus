use bevy::prelude::*;
use nalgebra::Vector3;
use rapier3d::prelude::*;
use crate::physics::rigid_body::{RigidBodyComponent, ContactForce};

#[derive(Resource)]
pub struct PhysicsWorld {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub gravity: Vector3<f32>,
    pub integration_parameters: IntegrationParameters,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: DefaultBroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        let mut integration_parameters = IntegrationParameters::default();
        integration_parameters.dt = 1.0 / 240.0; // 240 Hz physics

        Self {
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            gravity: vector![0.0, -9.81, 0.0], // Standard gravity
            integration_parameters,
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn step(&mut self) {
        let physics_hooks = ();
        let event_handler = ();

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
            Some(&mut self.query_pipeline),
            &physics_hooks,
            &event_handler,
        );
    }

    pub fn fast_reset(&mut self) {
        for (_, rb) in self.rigid_body_set.iter_mut() {
            rb.set_linvel(vector![0.0, 0.0, 0.0], true);
            rb.set_angvel(vector![0.0, 0.0, 0.0], true);
        }
    }

    pub fn spawn_rigid_body(&mut self, rigid_body: RigidBody, entity: Entity) -> RigidBodyHandle {
        let mut rb = rigid_body;
        rb.user_data = entity.to_bits() as u128;
        self.rigid_body_set.insert(rb)
    }

    pub fn spawn_collider(
        &mut self,
        collider: Collider,
        rb_handle: RigidBodyHandle,
    ) -> ColliderHandle {
        self.collider_set
            .insert_with_parent(collider, rb_handle, &mut self.rigid_body_set)
    }

    pub fn get_entity_from_handle(&self, handle: RigidBodyHandle) -> Option<Entity> {
        self.rigid_body_set
            .get(handle)
            .map(|rb| Entity::from_bits(rb.user_data as u64))
    }
}

/// System to extract contact forces from Rapier3D and populate ContactForce components
/// This must run after physics step but before sensor updates
pub fn extract_contact_forces_system(
    physics_world: Res<PhysicsWorld>,
    mut query: Query<(&RigidBodyComponent, &mut ContactForce, &GlobalTransform)>,
) {
    // First, reset all contact forces
    for (_, mut contact_force, _) in query.iter_mut() {
        contact_force.reset();
    }

    // Iterate through all contact pairs in the narrow phase
    for contact_pair in physics_world.narrow_phase.contact_pairs() {
        let collider1 = contact_pair.collider1;
        let collider2 = contact_pair.collider2;

        // Get rigid body handles from colliders
        let Some(coll1_data) = physics_world.collider_set.get(collider1) else {
            continue;
        };
        let Some(coll2_data) = physics_world.collider_set.get(collider2) else {
            continue;
        };

        let Some(rb1_handle) = coll1_data.parent() else {
            continue;
        };
        let Some(rb2_handle) = coll2_data.parent() else {
            continue;
        };

        // Get rigid body data
        let Some(rb1) = physics_world.rigid_body_set.get(rb1_handle) else {
            continue;
        };
        let Some(rb2) = physics_world.rigid_body_set.get(rb2_handle) else {
            continue;
        };

        // Get entities from user data
        let entity1 = Entity::from_bits(rb1.user_data as u64);
        let entity2 = Entity::from_bits(rb2.user_data as u64);

        // Get contact manifolds
        for manifold in contact_pair.manifolds.iter() {
            // Get contact normal (points from body1 to body2)
            let normal = manifold.data.normal;

            // Iterate through contact points in the manifold
            for contact_point in manifold.data.solver_contacts.iter() {
                // Calculate contact force magnitude from penetration and material properties
                // Rapier doesn't directly expose contact forces, so we estimate from impulses
                // accumulated during the solve phase

                // Get contact impulse (this is the force * dt)
                let impulse_magnitude = contact_point.warmstart_impulse + contact_point.dist;

                // Convert to force (divide by dt)
                let dt = physics_world.integration_parameters.dt;
                let force_magnitude = if dt > 1e-6 {
                    impulse_magnitude / dt
                } else {
                    0.0
                };

                // Contact force in world coordinates (normal force)
                // Note: Rapier's normal points from body1 to body2
                let contact_force_vec = Vec3::new(normal.x, normal.y, normal.z) * force_magnitude;

                // Get contact point in world coordinates
                let point1 = Vec3::new(
                    contact_point.point.x,
                    contact_point.point.y,
                    contact_point.point.z,
                );

                let contact_normal_vec = Vec3::new(normal.x, normal.y, normal.z);

                // Apply force to body1 (negative because force on body1 opposes normal)
                if let Ok((rb_comp, mut contact_force, transform)) = query.get_mut(entity1) {
                    if rb_comp.handle == rb1_handle {
                        let com = transform.translation();
                        contact_force.add_contact(
                            -contact_force_vec,
                            point1,
                            -contact_normal_vec,
                            com,
                        );
                    }
                }

                // Apply force to body2 (positive because force on body2 follows normal)
                if let Ok((rb_comp, mut contact_force, transform)) = query.get_mut(entity2) {
                    if rb_comp.handle == rb2_handle {
                        let com = transform.translation();
                        contact_force.add_contact(
                            contact_force_vec,
                            point1,
                            contact_normal_vec,
                            com,
                        );
                    }
                }
            }
        }
    }
}

/// Helper system to add ContactForce component to all rigid bodies that don't have it
pub fn ensure_contact_force_components(
    mut commands: Commands,
    query: Query<Entity, (With<RigidBodyComponent>, Without<ContactForce>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(ContactForce::default());
    }
}
