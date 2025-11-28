use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Twist {
    pub linear: Vector3Message,
    pub angular: Vector3Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3Message {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Vector3Message {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl From<Vec3> for Vector3Message {
    fn from(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<Vector3Message> for Vec3 {
    fn from(val: Vector3Message) -> Self {
        Vec3::new(val.x, val.y, val.z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuaternionMessage {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl From<Quat> for QuaternionMessage {
    fn from(q: Quat) -> Self {
        Self {
            x: q.x,
            y: q.y,
            z: q.z,
            w: q.w,
        }
    }
}

impl From<QuaternionMessage> for Quat {
    fn from(val: QuaternionMessage) -> Self {
        Quat::from_xyzw(val.x, val.y, val.z, val.w)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformStamped {
    pub header: Header,
    pub child_frame_id: String,
    pub transform: TransformMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformMessage {
    pub translation: Vector3Message,
    pub rotation: QuaternionMessage,
}

impl TransformMessage {
    pub fn from_bevy_transform(transform: &Transform) -> Self {
        Self {
            translation: transform.translation.into(),
            rotation: transform.rotation.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub stamp: f64,
    pub frame_id: String,
}

impl Header {
    pub fn new(frame_id: impl Into<String>, time: f64) -> Self {
        Self {
            stamp: time,
            frame_id: frame_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointCloud2 {
    pub header: Header,
    pub height: u32,
    pub width: u32,
    pub fields: Vec<PointField>,
    pub is_bigendian: bool,
    pub point_step: u32,
    pub row_step: u32,
    pub data: Vec<u8>,
    pub is_dense: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointField {
    pub name: String,
    pub offset: u32,
    pub datatype: u8,
    pub count: u32,
}

impl PointField {
    pub const FLOAT32: u8 = 7;

    pub fn new(name: impl Into<String>, offset: u32, datatype: u8, count: u32) -> Self {
        Self {
            name: name.into(),
            offset,
            datatype,
            count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaserScanMessage {
    pub header: Header,
    pub angle_min: f32,
    pub angle_max: f32,
    pub angle_increment: f32,
    pub time_increment: f32,
    pub scan_time: f32,
    pub range_min: f32,
    pub range_max: f32,
    pub ranges: Vec<f32>,
    pub intensities: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMessage {
    pub header: Header,
    pub height: u32,
    pub width: u32,
    pub encoding: String,
    pub is_bigendian: bool,
    pub step: u32,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Imu {
    pub header: Header,
    pub orientation: QuaternionMessage,
    pub orientation_covariance: Vec<f64>,
    pub angular_velocity: Vector3Message,
    pub angular_velocity_covariance: Vec<f64>,
    pub linear_acceleration: Vector3Message,
    pub linear_acceleration_covariance: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JointState {
    pub header: Header,
    pub name: Vec<String>,
    pub position: Vec<f32>,
    pub velocity: Vec<f32>,
    pub effort: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Odometry {
    pub header: Header,
    pub child_frame_id: String,
    pub pose: PoseWithCovariance,
    pub twist: TwistWithCovariance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseWithCovariance {
    pub pose: Pose,
    pub covariance: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pose {
    pub position: Vector3Message,
    pub orientation: QuaternionMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwistWithCovariance {
    pub twist: Twist,
    pub covariance: Vec<f64>,
}
