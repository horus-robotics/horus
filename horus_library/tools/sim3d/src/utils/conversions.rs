use bevy::prelude::*;
use nalgebra::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};

pub fn nalgebra_to_glam(v: Vector3<f32>) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn glam_to_nalgebra(v: Vec3) -> Vector3<f32> {
    Vector3::new(v.x, v.y, v.z)
}

pub fn isometry_to_transform(iso: Isometry3<f32>) -> Transform {
    Transform {
        translation: Vec3::new(iso.translation.x, iso.translation.y, iso.translation.z),
        rotation: Quat::from_xyzw(
            iso.rotation.i,
            iso.rotation.j,
            iso.rotation.k,
            iso.rotation.w,
        ),
        scale: Vec3::ONE,
    }
}

pub fn transform_to_isometry(t: Transform) -> Isometry3<f32> {
    let translation = Translation3::new(t.translation.x, t.translation.y, t.translation.z);
    let rotation = UnitQuaternion::from_quaternion(Quaternion::new(
        t.rotation.w,
        t.rotation.x,
        t.rotation.y,
        t.rotation.z,
    ));
    Isometry3::from_parts(translation, rotation)
}
