use bevy::prelude::*;
use std::f32::consts::PI;

/// Angle utilities
pub struct AngleUtils;

impl AngleUtils {
    /// Normalize angle to [-π, π]
    pub fn normalize_angle(angle: f32) -> f32 {
        let mut normalized = angle % (2.0 * PI);
        // Don't wrap the exact boundary values to preserve both PI and -PI
        if normalized > PI {
            normalized -= 2.0 * PI;
        } else if normalized < -PI {
            normalized += 2.0 * PI;
        }
        normalized
    }

    /// Normalize angle to [0, 2π]
    pub fn normalize_angle_positive(angle: f32) -> f32 {
        let mut normalized = angle % (2.0 * PI);
        if normalized < 0.0 {
            normalized += 2.0 * PI;
        }
        normalized
    }

    /// Convert degrees to radians
    pub fn deg_to_rad(degrees: f32) -> f32 {
        degrees * PI / 180.0
    }

    /// Convert radians to degrees
    pub fn rad_to_deg(radians: f32) -> f32 {
        radians * 180.0 / PI
    }

    /// Calculate shortest angular distance between two angles
    pub fn angle_diff(from: f32, to: f32) -> f32 {
        Self::normalize_angle(to - from)
    }

    /// Linear interpolation between two angles (handles wrap-around)
    pub fn angle_lerp(from: f32, to: f32, t: f32) -> f32 {
        from + Self::angle_diff(from, to) * t
    }
}

/// Vector utilities
pub struct VectorUtils;

impl VectorUtils {
    /// Project vector a onto vector b
    pub fn project(a: Vec3, b: Vec3) -> Vec3 {
        b * (a.dot(b) / b.dot(b))
    }

    /// Reject vector a from vector b (perpendicular component)
    pub fn reject(a: Vec3, b: Vec3) -> Vec3 {
        a - Self::project(a, b)
    }

    /// Reflect vector across normal
    pub fn reflect(incident: Vec3, normal: Vec3) -> Vec3 {
        incident - 2.0 * incident.dot(normal) * normal
    }

    /// Calculate angle between two vectors
    pub fn angle_between(a: Vec3, b: Vec3) -> f32 {
        (a.dot(b) / (a.length() * b.length())).acos()
    }

    /// Clamp vector magnitude
    pub fn clamp_magnitude(v: Vec3, max_magnitude: f32) -> Vec3 {
        let mag = v.length();
        if mag > max_magnitude {
            v * (max_magnitude / mag)
        } else {
            v
        }
    }

    /// Check if vector is approximately zero
    pub fn is_zero(v: Vec3, epsilon: f32) -> bool {
        v.length_squared() < epsilon * epsilon
    }

    /// Get orthogonal vector (perpendicular in 3D)
    pub fn orthogonal(v: Vec3) -> Vec3 {
        let x = v.x.abs();
        let y = v.y.abs();
        let z = v.z.abs();

        let other = if x < y {
            if x < z {
                Vec3::X
            } else {
                Vec3::Z
            }
        } else if y < z {
            Vec3::Y
        } else {
            Vec3::Z
        };

        v.cross(other).normalize()
    }
}

/// Quaternion utilities
pub struct QuaternionUtils;

impl QuaternionUtils {
    /// Create quaternion from axis-angle representation
    pub fn from_axis_angle(axis: Vec3, angle: f32) -> Quat {
        Quat::from_axis_angle(axis.normalize(), angle)
    }

    /// Convert quaternion to axis-angle
    pub fn to_axis_angle(q: Quat) -> (Vec3, f32) {
        let angle = 2.0 * q.w.acos();
        let s = (1.0 - q.w * q.w).sqrt();

        let axis = if s < 0.001 {
            Vec3::X // Arbitrary axis when angle is ~0
        } else {
            Vec3::new(q.x / s, q.y / s, q.z / s)
        };

        (axis, angle)
    }

    /// Get rotation angle from quaternion
    pub fn get_angle(q: Quat) -> f32 {
        2.0 * q.w.acos()
    }

    /// Get rotation axis from quaternion
    pub fn get_axis(q: Quat) -> Vec3 {
        let (axis, _) = Self::to_axis_angle(q);
        axis
    }

    /// Shortest rotation from one quaternion to another
    pub fn shortest_rotation(from: Quat, to: Quat) -> Quat {
        to * from.inverse()
    }

    /// Angular velocity from two quaternions and time delta
    pub fn angular_velocity(from: Quat, to: Quat, dt: f32) -> Vec3 {
        let delta = Self::shortest_rotation(from, to);
        let (axis, angle) = Self::to_axis_angle(delta);
        axis * (angle / dt)
    }
}

/// Matrix utilities
pub struct MatrixUtils;

impl MatrixUtils {
    /// Create transformation matrix from position and rotation
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Mat4 {
        Mat4::from_rotation_translation(rotation, position)
    }

    /// Extract position from transformation matrix
    pub fn extract_position(mat: Mat4) -> Vec3 {
        mat.w_axis.truncate()
    }

    /// Extract rotation from transformation matrix
    pub fn extract_rotation(mat: Mat4) -> Quat {
        Quat::from_mat4(&mat)
    }

    /// Extract scale from transformation matrix
    pub fn extract_scale(mat: Mat4) -> Vec3 {
        Vec3::new(
            mat.x_axis.truncate().length(),
            mat.y_axis.truncate().length(),
            mat.z_axis.truncate().length(),
        )
    }

    /// Decompose matrix into translation, rotation, and scale
    pub fn decompose(mat: Mat4) -> (Vec3, Quat, Vec3) {
        (
            Self::extract_position(mat),
            Self::extract_rotation(mat),
            Self::extract_scale(mat),
        )
    }
}

/// Interpolation utilities
pub struct Interpolation;

impl Interpolation {
    /// Linear interpolation
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Smooth step (cubic hermite interpolation)
    pub fn smooth_step(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Smoother step (quintic interpolation)
    pub fn smoother_step(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    /// Ease in (cubic)
    pub fn ease_in(t: f32) -> f32 {
        t * t * t
    }

    /// Ease out (cubic)
    pub fn ease_out(t: f32) -> f32 {
        let t = 1.0 - t;
        1.0 - t * t * t
    }

    /// Ease in-out (cubic)
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t = 2.0 * t - 2.0;
            1.0 + t * t * t / 2.0
        }
    }

    /// Catmull-Rom spline
    pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        let t2 = t * t;
        let t3 = t2 * t;

        0.5 * ((2.0 * p1)
            + (-p0 + p2) * t
            + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
            + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
    }
}

/// Coordinate system conversions
pub struct CoordinateUtils;

impl CoordinateUtils {
    /// Cartesian to spherical coordinates (r, θ, φ)
    /// θ is polar angle from z-axis, φ is azimuthal angle from x-axis
    pub fn cartesian_to_spherical(pos: Vec3) -> Vec3 {
        let r = pos.length();
        let theta = (pos.x * pos.x + pos.y * pos.y).sqrt().atan2(pos.z);
        let phi = pos.y.atan2(pos.x);
        Vec3::new(r, theta, phi)
    }

    /// Spherical to Cartesian coordinates
    pub fn spherical_to_cartesian(spherical: Vec3) -> Vec3 {
        let r = spherical.x;
        let theta = spherical.y;
        let phi = spherical.z;

        Vec3::new(
            r * theta.sin() * phi.cos(),
            r * theta.sin() * phi.sin(),
            r * theta.cos(),
        )
    }

    /// Cartesian to cylindrical coordinates (r, θ, z)
    pub fn cartesian_to_cylindrical(pos: Vec3) -> Vec3 {
        let r = (pos.x * pos.x + pos.y * pos.y).sqrt();
        let theta = pos.y.atan2(pos.x);
        Vec3::new(r, theta, pos.z)
    }

    /// Cylindrical to Cartesian coordinates
    pub fn cylindrical_to_cartesian(cylindrical: Vec3) -> Vec3 {
        let r = cylindrical.x;
        let theta = cylindrical.y;
        let z = cylindrical.z;

        Vec3::new(r * theta.cos(), r * theta.sin(), z)
    }
}

/// Numeric utilities
pub struct NumericUtils;

impl NumericUtils {
    /// Check if two floats are approximately equal
    pub fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }

    /// Clamp value between min and max
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.clamp(min, max)
    }

    /// Map value from one range to another
    pub fn map_range(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
        let t = (value - from_min) / (from_max - from_min);
        to_min + t * (to_max - to_min)
    }

    /// Sign function (-1, 0, or 1)
    pub fn sign(value: f32) -> f32 {
        if value > 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            0.0
        }
    }

    /// Wrap value to range [min, max]
    pub fn wrap(value: f32, min: f32, max: f32) -> f32 {
        let range = max - min;
        min + ((value - min) % range + range) % range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_normalization() {
        // 3*PI % (2*PI) gives a value very close to PI (but slightly less due to float precision)
        // In the range [-π, π], this stays as PI
        assert!(AngleUtils::approx_eq(
            AngleUtils::normalize_angle(3.0 * PI),
            PI,
            0.001
        ));
        // -3*PI % (2*PI) gives a value very close to -PI
        // In the range [-π, π], this stays as -PI
        assert!(AngleUtils::approx_eq(
            AngleUtils::normalize_angle(-3.0 * PI),
            -PI,
            0.001
        ));
    }

    #[test]
    fn test_deg_rad_conversion() {
        assert!(AngleUtils::approx_eq(
            AngleUtils::deg_to_rad(180.0),
            PI,
            0.001
        ));
        assert!(AngleUtils::approx_eq(
            AngleUtils::rad_to_deg(PI),
            180.0,
            0.001
        ));
    }

    #[test]
    fn test_angle_diff() {
        let diff = AngleUtils::angle_diff(0.0, PI);
        assert!(AngleUtils::approx_eq(diff, PI, 0.001));

        let diff = AngleUtils::angle_diff(0.0, -PI);
        assert!(AngleUtils::approx_eq(diff, -PI, 0.001));
    }

    #[test]
    fn test_vector_project() {
        let a = Vec3::new(1.0, 1.0, 0.0);
        let b = Vec3::new(1.0, 0.0, 0.0);
        let proj = VectorUtils::project(a, b);
        assert_eq!(proj, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn test_vector_clamp_magnitude() {
        let v = Vec3::new(10.0, 0.0, 0.0);
        let clamped = VectorUtils::clamp_magnitude(v, 5.0);
        assert_eq!(clamped.length(), 5.0);
    }

    #[test]
    fn test_quaternion_axis_angle() {
        let axis = Vec3::Y;
        let angle = PI / 2.0;
        let q = QuaternionUtils::from_axis_angle(axis, angle);
        let (result_axis, result_angle) = QuaternionUtils::to_axis_angle(q);

        assert!((result_axis - axis).length() < 0.001);
        assert!((result_angle - angle).abs() < 0.001);
    }

    #[test]
    fn test_interpolation_lerp() {
        assert_eq!(Interpolation::lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(Interpolation::lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(Interpolation::lerp(0.0, 10.0, 1.0), 10.0);
    }

    #[test]
    fn test_smooth_step() {
        assert_eq!(Interpolation::smooth_step(0.0), 0.0);
        assert_eq!(Interpolation::smooth_step(1.0), 1.0);
        assert!(Interpolation::smooth_step(0.5) > 0.4 && Interpolation::smooth_step(0.5) < 0.6);
    }

    #[test]
    fn test_coordinate_conversions() {
        let cart = Vec3::new(1.0, 0.0, 0.0);
        let sph = CoordinateUtils::cartesian_to_spherical(cart);
        let back = CoordinateUtils::spherical_to_cartesian(sph);

        assert!((cart - back).length() < 0.001);
    }

    #[test]
    fn test_numeric_map_range() {
        let result = NumericUtils::map_range(5.0, 0.0, 10.0, 0.0, 100.0);
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_numeric_wrap() {
        assert_eq!(NumericUtils::wrap(15.0, 0.0, 10.0), 5.0);
        assert_eq!(NumericUtils::wrap(-5.0, 0.0, 10.0), 5.0);
    }
}

// Extension trait for AngleUtils
trait AngleUtilsExt {
    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool;
}

impl AngleUtilsExt for AngleUtils {
    fn approx_eq(a: f32, b: f32, epsilon: f32) -> bool {
        (a - b).abs() < epsilon
    }
}
