use std::f32::consts::PI;
use super::answers::RPLIDAR_RESP_HQ_FLAG_SYNCBIT;
use std::cmp::Ordering;

/// Scan point in a particular laser scan
#[derive(Debug, Clone, Eq)]
pub struct ScanPoint {
    pub angle_z_q14: u16,
    pub dist_mm_q2: u32,
    pub quality: u8,
    pub flag: u8,
}

impl ScanPoint {
    pub fn angle(&self) -> f32 {
        return (self.angle_z_q14 as f32) / 16384f32 / 2f32 * PI;
    }

    pub fn set_angle(&mut self, angle:f32) {
        self.angle_z_q14 = (angle * 16384f32 * 2f32 / PI) as u16;
    }

    pub fn distance(&self) -> f32 {
        return (self.dist_mm_q2 as f32) / 4000f32;
    }

    pub fn set_distance(&mut self, dist: f32) {
        self.dist_mm_q2 = (dist * 4000f32) as u32;
    }

    pub fn is_sync(&self) -> bool {
        return (self.flag & RPLIDAR_RESP_HQ_FLAG_SYNCBIT) == RPLIDAR_RESP_HQ_FLAG_SYNCBIT;
    }

    pub fn is_valid(&self) -> bool {
        return self.quality != 0 && self.dist_mm_q2 != 0;
    }
}

impl Ord for ScanPoint {
    fn cmp(&self, other: &ScanPoint) -> Ordering {
        self.angle_z_q14.cmp(&other.angle_z_q14)
    }
}

impl PartialOrd for ScanPoint {
    fn partial_cmp(&self, other: &ScanPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for ScanPoint {
    fn eq(&self, other: &ScanPoint) -> bool {
        self.angle_z_q14 == other.angle_z_q14
        && self.dist_mm_q2 == other.dist_mm_q2
        && self.quality == other.quality
        && self.flag == other.flag
    }
}

/// Description of a specific scan mode
#[derive(Debug, Clone, PartialEq)]
pub struct ScanMode {
    /// The scan mode id
    pub id: u16,

    /// Microseconds per measurement sample
    pub us_per_sample: f32,

    /// Max distance of this measurement mode
    pub max_distance: f32,

    /// The answer command value of this scan mode (mainly used to decode messages)
    pub ans_type: u8,

    /// The name of the scan mode
    pub name: String,
}

/// Scan options
#[derive(Debug, Clone, PartialEq)]
pub struct ScanOptions {
    /// Specify this field to force use specific scan mode
    pub scan_mode: Option<u16>,

    /// Make LIDAR scan regardless of it's spinning or not
    pub force_scan: bool,

    /// Parameters sent to LIDAR. Please use 0 for now
    pub options: u32,
}

impl ScanOptions {
    /// default options
    pub fn default() -> ScanOptions {
        ScanOptions {
            scan_mode: None,
            force_scan: false,
            options: 0,
        }
    }

    /// with specific mode
    pub fn with_mode(scan_mode: u16) -> ScanOptions {
        ScanOptions {
            scan_mode: Some(scan_mode),
            force_scan: false,
            options: 0,
        }
    }

    /// force scan
    pub fn force_scan() -> ScanOptions {
        ScanOptions {
            scan_mode: None,
            force_scan: true,
            options: 0,
        }
    }

    /// force scan with mode
    pub fn force_scan_with_mode(scan_mode: u16) -> ScanOptions {
        ScanOptions {
            scan_mode: Some(scan_mode),
            force_scan: true,
            options: 0,
        }
    }
}

/// Health status of device
#[derive(Debug, Clone, PartialEq)]
pub enum Health {
    Healthy,
    Warning(u16),
    Error(u16)
}
