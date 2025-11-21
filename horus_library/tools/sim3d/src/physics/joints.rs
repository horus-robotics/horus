use bevy::prelude::*;
use nalgebra::{Point3, Unit, Vector3};
use rapier3d::prelude::*;

#[derive(Component)]
pub struct PhysicsJoint {
    pub handle: ImpulseJointHandle,
    pub joint_type: JointType,
}

#[derive(Debug, Clone, Copy)]
pub enum JointType {
    Revolute,
    Prismatic,
    Fixed,
    Spherical,
}

pub fn create_revolute_joint(anchor1: Vec3, anchor2: Vec3, axis: Vec3) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .build()
}

pub fn create_revolute_joint_with_limits(
    anchor1: Vec3,
    anchor2: Vec3,
    axis: Vec3,
    min_angle: f32,
    max_angle: f32,
) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    GenericJointBuilder::new(JointAxesMask::LOCKED_REVOLUTE_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .limits(JointAxis::AngX, [min_angle, max_angle])
        .build()
}

pub fn create_prismatic_joint(anchor1: Vec3, anchor2: Vec3, axis: Vec3) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    GenericJointBuilder::new(JointAxesMask::LOCKED_PRISMATIC_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .build()
}

pub fn create_prismatic_joint_with_limits(
    anchor1: Vec3,
    anchor2: Vec3,
    axis: Vec3,
    min_distance: f32,
    max_distance: f32,
) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);
    let axis = Unit::new_normalize(Vector3::new(axis.x, axis.y, axis.z));

    GenericJointBuilder::new(JointAxesMask::LOCKED_PRISMATIC_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .local_axis1(axis)
        .local_axis2(axis)
        .limits(JointAxis::LinX, [min_distance, max_distance])
        .build()
}

pub fn create_fixed_joint(anchor1: Vec3, anchor2: Vec3) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);

    GenericJointBuilder::new(JointAxesMask::LOCKED_FIXED_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .build()
}

pub fn create_spherical_joint(anchor1: Vec3, anchor2: Vec3) -> GenericJoint {
    let anchor1 = Point3::new(anchor1.x, anchor1.y, anchor1.z);
    let anchor2 = Point3::new(anchor2.x, anchor2.y, anchor2.z);

    GenericJointBuilder::new(JointAxesMask::LOCKED_SPHERICAL_AXES)
        .local_anchor1(anchor1)
        .local_anchor2(anchor2)
        .build()
}

pub fn add_joint_motor(joint: &mut GenericJoint, axis: JointAxis, target_vel: f32, max_force: f32) {
    joint.set_motor_velocity(axis, target_vel, max_force);
}

pub fn add_joint_spring(joint: &mut GenericJoint, axis: JointAxis, stiffness: f32, damping: f32) {
    joint.set_motor_model(axis, MotorModel::ForceBased);
    joint.set_motor(axis, 0.0, 0.0, stiffness, damping);
}
