// Commands without payload and response

/// Stop measurement of LIDAR
pub const RPLIDAR_CMD_STOP : u8 = 0x25;

/// Start scan in default mode (usually legacy mode)
pub const RPLIDAR_CMD_SCAN : u8 = 0x20;

/// Start force scan (measure distance regardless of LIDAR is spinning or not) in default mode (usually legacy mode)
pub const RPLIDAR_CMD_FORCE_SCAN : u8 = 0x21;

/// Reset the LIDAR core
pub const RPLIDAR_CMD_RESET : u8 = 0x40;


// Commands without payload but have response

/// Get device information
pub const RPLIDAR_CMD_GET_DEVICE_INFO : u8 = 0x50;

/// Get device health info
pub const RPLIDAR_CMD_GET_DEVICE_HEALTH : u8 = 0x52;

// pub const RPLIDAR_CMD_GET_SAMPLERATE : u8 = 0x59; //added in fw 1.17

// pub const RPLIDAR_CMD_HQ_MOTOR_SPEED_CTRL : u8 = 0xA8;

// Commands with payload and have response

/// Start express scan (both legacy and extended mode)
pub const RPLIDAR_CMD_EXPRESS_SCAN : u8 = 0x82; //added in fw 1.17;

/// Options to start scan
#[repr(packed)]
#[repr(C)]
pub struct RplidarPayloadExpressScan {
    /// The work mode requested
    /// 0 for legacy express scan (usually in Standard mode)
    /// otherwise, the scan mode id requested
    pub work_mode: u8,

    /// Workflags (reserved, please keep zero)
    pub work_flags: u16,

    /// Param (reserved, please keep zero)
    pub param: u16
}

// pub const RPLIDAR_CMD_HQ_SCAN : u8 = 0x83; //added in fw 1.24;

/// Get LIDAR configuration
pub const RPLIDAR_CMD_GET_LIDAR_CONF : u8 = 0x84; //added in fw 1.24;

// pub const RPLIDAR_CMD_SET_LIDAR_CONF : u8 = 0x85; //added in fw 1.24;

/// Set motor PWM for the accessory board with RPLIDAR A2 and A3 Kit Models
/// (add for A2 to set RPLIDAR motor pwm when using accessory board)
pub const RPLIDAR_CMD_SET_MOTOR_PWM : u8 = 0xF0;

/// Get capability of accessory board
pub const RPLIDAR_CMD_GET_ACC_BOARD_FLAG : u8 = 0xFF;

// LIDAR configurations

/// LIDAR config entry for scan mode count
pub const RPLIDAR_CONF_SCAN_MODE_COUNT: u32 = 0x00000070;

/// LIDAR config entry for sample duration
pub const RPLIDAR_CONF_SCAN_MODE_US_PER_SAMPLE: u32 = 0x00000071;

/// LIDAR config entry for max distance supported by specific scan mode
pub const RPLIDAR_CONF_SCAN_MODE_MAX_DISTANCE: u32 = 0x00000074;

/// LIDAR config entry for actual answer type for specific scan mode
pub const RPLIDAR_CONF_SCAN_MODE_ANS_TYPE: u32 = 0x00000075;

/// LIDAR config entry for typical scan mode id
pub const RPLIDAR_CONF_SCAN_MODE_TYPICAL: u32 = 0x0000007C;

/// LIDAR config entry for the name of specific scan mode
pub const RPLIDAR_CONF_SCAN_MODE_NAME: u32 = 0x0000007F;
