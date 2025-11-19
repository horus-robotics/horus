//! AABB (Axis-Aligned Bounding Box) Collision Detection
//!
//! Fast collision detection using axis-aligned bounding boxes.
//!
//! # Features
//!
//! - AABB vs AABB collision
//! - Point in AABB test
//! - Ray vs AABB intersection
//! - Fast broad-phase collision detection
//!
//! # Example
//!
//! ```rust
//! use horus_library::algorithms::aabb::AABB;
//!
//! let box1 = AABB::new(0.0, 0.0, 2.0, 2.0);
//! let box2 = AABB::new(1.0, 1.0, 3.0, 3.0);
//!
//! if box1.intersects(&box2) {
//!     println!("Collision detected!");
//! }
//! ```

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl AABB {
    /// Create new AABB from min/max coordinates
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    /// Create AABB from center and size
    pub fn from_center(center_x: f64, center_y: f64, width: f64, height: f64) -> Self {
        let half_w = width / 2.0;
        let half_h = height / 2.0;
        Self {
            min_x: center_x - half_w,
            min_y: center_y - half_h,
            max_x: center_x + half_w,
            max_y: center_y + half_h,
        }
    }

    /// Check if this AABB intersects with another
    pub fn intersects(&self, other: &AABB) -> bool {
        !(self.max_x < other.min_x
            || self.min_x > other.max_x
            || self.max_y < other.min_y
            || self.min_y > other.max_y)
    }

    /// Check if point is inside AABB
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Check if this AABB fully contains another
    pub fn contains(&self, other: &AABB) -> bool {
        other.min_x >= self.min_x
            && other.max_x <= self.max_x
            && other.min_y >= self.min_y
            && other.max_y <= self.max_y
    }

    /// Get width
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    /// Get height
    pub fn height(&self) -> f64 {
        self.max_y - self.min_y
    }

    /// Get center point
    pub fn center(&self) -> (f64, f64) {
        ((self.min_x + self.max_x) / 2.0, (self.min_y + self.max_y) / 2.0)
    }

    /// Get area
    pub fn area(&self) -> f64 {
        self.width() * self.height()
    }

    /// Expand AABB by margin
    pub fn expand(&self, margin: f64) -> AABB {
        AABB {
            min_x: self.min_x - margin,
            min_y: self.min_y - margin,
            max_x: self.max_x + margin,
            max_y: self.max_y + margin,
        }
    }

    /// Merge with another AABB
    pub fn merge(&self, other: &AABB) -> AABB {
        AABB {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Ray intersection test
    ///
    /// Returns Some(t) where t is the distance along the ray to intersection
    pub fn ray_intersect(&self, origin: (f64, f64), direction: (f64, f64)) -> Option<f64> {
        let (ox, oy) = origin;
        let (dx, dy) = direction;

        let t1 = (self.min_x - ox) / dx;
        let t2 = (self.max_x - ox) / dx;
        let t3 = (self.min_y - oy) / dy;
        let t4 = (self.max_y - oy) / dy;

        let tmin = t1.min(t2).max(t3.min(t4));
        let tmax = t1.max(t2).min(t3.max(t4));

        if tmax < 0.0 || tmin > tmax {
            None
        } else {
            Some(if tmin < 0.0 { tmax } else { tmin })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intersection() {
        let box1 = AABB::new(0.0, 0.0, 2.0, 2.0);
        let box2 = AABB::new(1.0, 1.0, 3.0, 3.0);
        let box3 = AABB::new(5.0, 5.0, 7.0, 7.0);

        assert!(box1.intersects(&box2));
        assert!(!box1.intersects(&box3));
    }

    #[test]
    fn test_contains_point() {
        let bbox = AABB::new(0.0, 0.0, 10.0, 10.0);

        assert!(bbox.contains_point(5.0, 5.0));
        assert!(bbox.contains_point(0.0, 0.0));
        assert!(bbox.contains_point(10.0, 10.0));
        assert!(!bbox.contains_point(11.0, 5.0));
    }

    #[test]
    fn test_contains_aabb() {
        let outer = AABB::new(0.0, 0.0, 10.0, 10.0);
        let inner = AABB::new(2.0, 2.0, 8.0, 8.0);
        let partial = AABB::new(5.0, 5.0, 12.0, 12.0);

        assert!(outer.contains(&inner));
        assert!(!outer.contains(&partial));
    }

    #[test]
    fn test_from_center() {
        let bbox = AABB::from_center(5.0, 5.0, 4.0, 6.0);

        assert_eq!(bbox.min_x, 3.0);
        assert_eq!(bbox.max_x, 7.0);
        assert_eq!(bbox.min_y, 2.0);
        assert_eq!(bbox.max_y, 8.0);
    }

    #[test]
    fn test_dimensions() {
        let bbox = AABB::new(0.0, 0.0, 10.0, 20.0);

        assert_eq!(bbox.width(), 10.0);
        assert_eq!(bbox.height(), 20.0);
        assert_eq!(bbox.area(), 200.0);
    }

    #[test]
    fn test_center() {
        let bbox = AABB::new(0.0, 0.0, 10.0, 20.0);
        let (cx, cy) = bbox.center();

        assert_eq!(cx, 5.0);
        assert_eq!(cy, 10.0);
    }

    #[test]
    fn test_expand() {
        let bbox = AABB::new(0.0, 0.0, 10.0, 10.0);
        let expanded = bbox.expand(2.0);

        assert_eq!(expanded.min_x, -2.0);
        assert_eq!(expanded.max_x, 12.0);
        assert_eq!(expanded.min_y, -2.0);
        assert_eq!(expanded.max_y, 12.0);
    }

    #[test]
    fn test_merge() {
        let box1 = AABB::new(0.0, 0.0, 5.0, 5.0);
        let box2 = AABB::new(3.0, 3.0, 10.0, 10.0);
        let merged = box1.merge(&box2);

        assert_eq!(merged.min_x, 0.0);
        assert_eq!(merged.max_x, 10.0);
        assert_eq!(merged.min_y, 0.0);
        assert_eq!(merged.max_y, 10.0);
    }

    #[test]
    fn test_ray_intersection() {
        let bbox = AABB::new(5.0, 5.0, 10.0, 10.0);

        // Ray from origin toward box
        let t = bbox.ray_intersect((0.0, 7.5), (1.0, 0.0));
        assert!(t.is_some());
        assert!((t.unwrap() - 5.0).abs() < 0.01);

        // Ray pointing away
        let t = bbox.ray_intersect((0.0, 7.5), (-1.0, 0.0));
        assert!(t.is_none());
    }

    #[test]
    fn test_edge_cases() {
        let box1 = AABB::new(0.0, 0.0, 5.0, 5.0);
        let box2 = AABB::new(5.0, 5.0, 10.0, 10.0);

        // Touching edges should intersect
        assert!(box1.intersects(&box2));
    }
}
