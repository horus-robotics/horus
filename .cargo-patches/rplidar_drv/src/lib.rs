//! # Rplidar Driver
//!
//! `rplidar_drv` is driver for Slamtec Rplidar series

extern crate byteorder;
extern crate crc;
extern crate rpos_drv;

mod internals;
mod answers;
mod capsuled_parser;
mod ultra_capsuled_parser;
mod checksum;
mod cmds;
mod errors;
mod prelude;
mod protocol;
pub mod utils;

pub use self::prelude::*;
pub use self::errors::*;

pub use self::answers::RplidarResponseDeviceInfo;

use self::answers::*;
use self::internals::*;
use self::capsuled_parser::parse_capsuled;
use self::ultra_capsuled_parser::parse_ultra_capsuled;
use self::checksum::Checksum;
use self::cmds::*;
pub use self::protocol::RplidarHostProtocol;
use byteorder::{ByteOrder, LittleEndian};
use rpos_drv::{Channel, Message, Result};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::mem::transmute_copy;
use std::time::{ Instant, Duration };
use crc::{ crc32 };

const RPLIDAR_GET_LIDAR_CONF_START_VERSION:u16 = ((1 << 8) | (24)) as u16;

/// Rplidar device driver
#[derive(Debug)]
pub struct RplidarDevice<T: ?Sized> {
    channel: Channel<RplidarHostProtocol, T>,
    cached_measurement_nodes: VecDeque<ScanPoint>,
    cached_prev_capsule: CachedPrevCapsule,
}

macro_rules! parse_resp_data {
    ($x:expr, $t:ty) => {{
        const SIZE: usize = std::mem::size_of::<$t>();
        if $x.len() != SIZE {
            Err(Error::OperationFail("answer type mismatch".to_owned()))
        } else {
            let mut slice = [0u8; SIZE];
            slice.clone_from_slice(&$x[..]);
            Ok(unsafe { transmute_copy::<[u8; SIZE], $t>(&slice) })
        }
    }};
}

macro_rules! parse_resp {
    ($x:expr, $t:ty) => {
        parse_resp_data!($x.data, $t)
    };
}

macro_rules! handle_resp {
    ($ans:expr, $x:expr, $t:ty) => {
        if $x.cmd != $ans {
            Err(Error::OperationFail("answer type mismatch".to_owned()))
        } else {
            parse_resp!($x, $t)
        }
    };
}

impl From<RplidarResponseMeasurementNodeHq> for ScanPoint {
    fn from(p: RplidarResponseMeasurementNodeHq) -> ScanPoint {
        ScanPoint {
            angle_z_q14: p.angle_z_q14,
            dist_mm_q2: p.dist_mm_q2,
            quality: p.quality,
            flag: p.flag,
        }
    }
}

impl<T: ?Sized> RplidarDevice<T>
where
    T: Read + Write,
{
    /// Construct a new RplidarDevice with channel
    ///
    /// # Example
    /// ```ignore
    /// let mut serial_port = serialport::open(serial_port_name)?;
    /// let channel = Channel::new(RplidarHostProtocol::new(), serial_port);
    /// let rplidar_device = RplidarDevice::new(channel);
    /// ```
    pub fn new(channel: Channel<RplidarHostProtocol, T>) -> RplidarDevice<T> {
        RplidarDevice {
            channel: channel,
            cached_measurement_nodes: VecDeque::with_capacity(RPLIDAR_DEFAULT_CACHE_DEPTH),
            cached_prev_capsule: CachedPrevCapsule::None,
        }
    }

    /// Construct a new RplidarDevice with stream
    ///
    /// # Example
    /// ```ignore
    /// let mut serial_port = serialport::open(serial_port_name)?;
    /// let rplidar_device = RplidarDevice::with_stream(serial_port);
    /// ```
    pub fn with_stream(stream: Box<T>) -> RplidarDevice<T> {
        RplidarDevice::<T>::new(rpos_drv::Channel::new(RplidarHostProtocol::new(), stream))
    }

    /// get device info of the RPLIDAR
    pub fn get_device_info(&mut self) -> Result<RplidarResponseDeviceInfo> {
        self.get_device_info_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// get device info of the RPLIDAR with timeout
    pub fn get_device_info_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<RplidarResponseDeviceInfo> {
        if let Some(msg) = self
            .channel
            .invoke(&Message::new(RPLIDAR_CMD_GET_DEVICE_INFO), timeout)?
        {
            return handle_resp!(RPLIDAR_ANS_TYPE_DEVINFO, msg, RplidarResponseDeviceInfo);
        }

        return Err(Error::OperationTimeout);
    }

    /// Stop lidar
    pub fn stop(&mut self) -> Result<()> {
        self.channel.write(&Message::new(RPLIDAR_CMD_STOP))?;
        return Ok(());
    }

    /// Reset RPLIDAR core
    pub fn core_reset(&mut self) -> Result<()> {
        self.channel.write(&Message::new(RPLIDAR_CMD_RESET))?;
        return Ok(());
    }

    /// Set motor PWM (via accessory board)
    pub fn set_motor_pwm(&mut self, pwm: u16) -> Result<()> {
        let mut payload = [0; 2];
        LittleEndian::write_u16(&mut payload, pwm);

        self.channel
            .write(&Message::with_data(RPLIDAR_CMD_SET_MOTOR_PWM, &payload))?;

        return Ok(());
    }

    /// Stop motor
    pub fn stop_motor(&mut self) -> Result<()> {
        self.set_motor_pwm(0)
    }

    /// Start motor
    pub fn start_motor(&mut self) -> Result<()> {
        self.set_motor_pwm(RPLIDAR_DEFAULT_MOTOR_PWM)
    }

    /*
    /// Get LIDAR config
    fn get_lidar_conf(&mut self, config_type: u32) -> Result<Vec<u8>> {
        self.get_lidar_conf_with_timeout(config_type, RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// get lidar config with parameter
    fn get_lidar_conf_with_param(&mut self, config_type: u32, param: &[u8]) -> Result<Vec<u8>> {
        self.get_lidar_conf_with_param_and_timeout(config_type, param, RPLIDAR_DEFAULT_TIMEOUT)
    }
    */

    /// get lidar config with timeout
    fn get_lidar_conf_with_timeout(
        &mut self,
        config_type: u32,
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        self.get_lidar_conf_with_param_and_timeout(config_type, &[], timeout)
    }

    /// get lidar config with parameter and timeout
    fn get_lidar_conf_with_param_and_timeout(
        &mut self,
        config_type: u32,
        param: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>> {
        let mut msg = Message::with_data(RPLIDAR_CMD_GET_LIDAR_CONF, &[0; 4]);

        LittleEndian::write_u32(&mut msg.data, config_type);
        msg.data.extend_from_slice(param);

        let response = self.channel.invoke(&msg, timeout)?;

        if let Some(mut response_msg) = response {
            if response_msg.cmd != RPLIDAR_ANS_TYPE_GET_LIDAR_CONF {
                return Err(Error::OperationFail("answer type mismatch".to_owned()));
            } else if response_msg.data.len() < 4
                || LittleEndian::read_u32(&response_msg.data[0..4]) != config_type
            {
                return Err(Error::OperationFail("answer config type mismatch".to_owned()));
            } else {
                return Ok(response_msg.data.split_off(4));
            }
        } else {
            return Err(Error::OperationTimeout);
        }
    }

    /// get typical scan mode of target LIDAR
    pub fn get_typical_scan_mode(&mut self) -> Result<u16> {
        self.get_typical_scan_mode_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// get typical scan mode of target LIDAR with timeout
    pub fn get_typical_scan_mode_with_timeout(&mut self, timeout: Duration) -> Result<u16> {
        let device_info = self.get_device_info_with_timeout(timeout)?;

        if device_info.firmware_version < RPLIDAR_GET_LIDAR_CONF_START_VERSION {
            return Ok(if device_info.model >= 0x20u8 {
                1u16
            } else {
                0u16
            });
        }

        let scan_mode_data =
            self.get_lidar_conf_with_timeout(RPLIDAR_CONF_SCAN_MODE_TYPICAL, timeout)?;

        return parse_resp_data!(scan_mode_data, u16);
    }

    /// get lidar sample duration
    fn get_scan_mode_us_per_sample_with_timeout(
        &mut self,
        scan_mode: u16,
        timeout: Duration,
    ) -> Result<f32> {
        let mut param = [0; 2];
        LittleEndian::write_u16(&mut param, scan_mode);
        let us_per_sample_data = self.get_lidar_conf_with_param_and_timeout(
            RPLIDAR_CONF_SCAN_MODE_US_PER_SAMPLE,
            &param,
            timeout,
        )?;
        let us_per_sample = (parse_resp_data!(us_per_sample_data, u32)? as f32) / 256f32;
        return Ok(us_per_sample);
    }

    /// get lidar scan mode max distance
    fn get_scan_mode_max_distance_with_timeout(
        &mut self,
        scan_mode: u16,
        timeout: Duration,
    ) -> Result<f32> {
        let mut param = [0; 2];
        LittleEndian::write_u16(&mut param, scan_mode);
        let max_distance_data = self.get_lidar_conf_with_param_and_timeout(
            RPLIDAR_CONF_SCAN_MODE_MAX_DISTANCE,
            &param,
            timeout,
        )?;
        let max_distance = (parse_resp_data!(max_distance_data, u32)? as f32) / 256f32;
        return Ok(max_distance);
    }

    /// get scan mode answer type
    fn get_scan_mode_ans_type_with_timeout(
        &mut self,
        scan_mode: u16,
        timeout: Duration,
    ) -> Result<u8> {
        let mut param = [0; 2];
        LittleEndian::write_u16(&mut param, scan_mode);
        let ans_type_data = self.get_lidar_conf_with_param_and_timeout(
            RPLIDAR_CONF_SCAN_MODE_ANS_TYPE,
            &param,
            timeout,
        )?;
        return parse_resp_data!(ans_type_data, u8);
    }

    /// get scan mode name
    fn get_scan_mode_name_with_timeout(
        &mut self,
        scan_mode: u16,
        timeout: Duration,
    ) -> Result<String> {
        let mut param = [0; 2];
        LittleEndian::write_u16(&mut param, scan_mode);
        let ans_type_data = self.get_lidar_conf_with_param_and_timeout(
            RPLIDAR_CONF_SCAN_MODE_NAME,
            &param,
            timeout,
        )?;

        if let Ok(name) = std::str::from_utf8(&ans_type_data) {
            return Ok(name.to_owned().trim_matches('\0').to_owned());
        } else {
            return Err(Error::ProtocolError("invalid scan mode name".to_owned()));
        }
    }

    /// get scan mode count
    fn get_scan_mode_count_with_timeout(&mut self, timeout: Duration) -> Result<u16> {
        let scan_mode_count_data =
            self.get_lidar_conf_with_timeout(RPLIDAR_CONF_SCAN_MODE_COUNT, timeout)?;
        return parse_resp_data!(scan_mode_count_data, u16);
    }

    /// get scan mode of specific scan mode id
    fn get_scan_mode_with_timeout(
        &mut self,
        scan_mode: u16,
        timeout: Duration,
    ) -> Result<ScanMode> {
        Ok(ScanMode {
            id: scan_mode,
            us_per_sample: self.get_scan_mode_us_per_sample_with_timeout(scan_mode, timeout)?
                as f32,
            max_distance: self.get_scan_mode_max_distance_with_timeout(scan_mode, timeout)?,
            ans_type: self.get_scan_mode_ans_type_with_timeout(scan_mode, timeout)?,
            name: self.get_scan_mode_name_with_timeout(scan_mode, timeout)?,
        })
    }

    /// get all supported scan modes supported by the LIDAR
    pub fn get_all_supported_scan_modes(&mut self) -> Result<Vec<ScanMode>> {
        self.get_all_supported_scan_modes_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// get all supported scan modes supported by the LIDAR with timeout
    pub fn get_all_supported_scan_modes_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Vec<ScanMode>> {
        let device_info = self.get_device_info_with_timeout(timeout)?;

        if device_info.firmware_version < RPLIDAR_GET_LIDAR_CONF_START_VERSION {
            let mut output: Vec<ScanMode> = Vec::with_capacity(2);

            output.push(ScanMode {
                id: 0u16,
                us_per_sample: 1000000f32 / 2000f32,
                max_distance: 8000f32,
                ans_type: RPLIDAR_ANS_TYPE_MEASUREMENT,
                name: "Standard".to_owned()
            });

            if device_info.model >= 0x20u8 {
                output.push(ScanMode {
                    id: 1u16,
                    us_per_sample: 1000000f32 / 4000f32,
                    max_distance: 16000f32,
                    ans_type: RPLIDAR_ANS_TYPE_MEASUREMENT_CAPSULED,
                    name: "Express".to_owned()
                });
            }

            return Ok(output);
        } else {
            let scan_mode_count = self.get_scan_mode_count_with_timeout(timeout)?;
            let mut output: Vec<ScanMode> = Vec::with_capacity(scan_mode_count as usize);

            for i in 0..scan_mode_count {
                output.push(self.get_scan_mode_with_timeout(i as u16, timeout)?);
            }

            return Ok(output);
        }
    }

    /// start scan
    pub fn start_scan(&mut self) -> Result<ScanMode> {
        self.start_scan_with_options(&ScanOptions::default())
    }

    /// start scan with timeout
    pub fn start_scan_with_timeout(&mut self, timeout: Duration) -> Result<ScanMode> {
        self.start_scan_with_options_and_timeout(&ScanOptions::default(), timeout)
    }

    /// start scan with options
    pub fn start_scan_with_options(&mut self, options: &ScanOptions) -> Result<ScanMode> {
        self.start_scan_with_options_and_timeout(options, RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// start scan with options and non-default timeout
    pub fn start_scan_with_options_and_timeout(
        &mut self,
        options: &ScanOptions,
        timeout: Duration,
    ) -> Result<ScanMode> {
        self.cached_prev_capsule = CachedPrevCapsule::None;

        let scan_mode = match options.scan_mode {
            Some(mode) => mode,
            None => self.get_typical_scan_mode_with_timeout(timeout)?,
        };

        let scan_mode_info = self.get_scan_mode_with_timeout(scan_mode, timeout)?;

        match scan_mode {
            0 => self.legacy_start_scan(options.force_scan)?,
            _ => {
                let payload = RplidarPayloadExpressScan {
                    work_mode: scan_mode as u8,
                    work_flags: options.options as u16,
                    param: 0,
                };
                self.start_express_scan(&payload)?;
            }
        }

        return Ok(scan_mode_info);
    }

    /// use legacy command to start scan
    fn legacy_start_scan(&mut self, force_scan: bool) -> Result<()> {
        self.channel.write(&Message::new(if force_scan {
            RPLIDAR_CMD_FORCE_SCAN
        } else {
            RPLIDAR_CMD_SCAN
        }))?;
        return Ok(());
    }

    /// start express scan with options
    fn start_express_scan(&mut self, options: &RplidarPayloadExpressScan) -> Result<()> {
        let data = unsafe {
            transmute_copy::<
                RplidarPayloadExpressScan,
                [u8; std::mem::size_of::<RplidarPayloadExpressScan>()],
            >(options)
        };
        self.channel
            .write(&Message::with_data(RPLIDAR_CMD_EXPRESS_SCAN, &data))?;
        return Ok(());
    }

    /// when hq measurement node received
    fn on_measurement_node_hq(&mut self, node: RplidarResponseMeasurementNodeHq) {
        self.cached_measurement_nodes
            .push_back(ScanPoint::from(node));
    }

    /// when measurement node received
    fn on_measurement_node(&mut self, node: RplidarResponseMeasurementNode) {
        self.on_measurement_node_hq(RplidarResponseMeasurementNodeHq {
            angle_z_q14: ((((node.angle_q6_checkbit as u32)
                >> RPLIDAR_RESP_MEASUREMENT_ANGLE_SHIFT)
                << 8)
                / 90) as u16,
            dist_mm_q2: node.distance_q2 as u32,
            flag: node.sync_quality & RPLIDAR_RESP_MEASUREMENT_SYNCBIT,
            quality: (node.sync_quality >> RPLIDAR_RESP_MEASUREMENT_QUALITY_SHIFT)
                << RPLIDAR_RESP_MEASUREMENT_QUALITY_SHIFT,
        });
    }

    /// when capsuled measurement msg received
    fn on_measurement_capsuled_msg(&mut self, msg: &Message) -> Result<()> {
        check_sync_and_checksum(msg)?;
        self.on_measurement_capsuled(parse_resp!(msg, RplidarResponseCapsuleMeasurementNodes)?);
        return Ok(());
    }

    /// when capsuled measurement response received
    fn on_measurement_capsuled(&mut self, nodes: RplidarResponseCapsuleMeasurementNodes) {
        let (parsed_nodes, new_cached_capsuled) = parse_capsuled(&self.cached_prev_capsule, nodes);
        self.cached_prev_capsule = new_cached_capsuled;

        for node in parsed_nodes {
            self.on_measurement_node_hq(node);
        }
    }

    /// when ultra capsuled measurement msg received
    fn on_measurement_ultra_capsuled_msg(&mut self, msg: &Message) -> Result<()> {
        check_sync_and_checksum(msg)?;
        self.on_measurement_ultra_capsuled(parse_resp!(
            msg,
            RplidarResponseUltraCapsuleMeasurementNodes
        )?);
        return Ok(());
    }

    /// when ultra capsuled measurement response received
    fn on_measurement_ultra_capsuled(
        &mut self,
        nodes: RplidarResponseUltraCapsuleMeasurementNodes,
    ) {
        let (parsed_nodes, new_cached_capsuled) = parse_ultra_capsuled(&self.cached_prev_capsule, nodes);
        self.cached_prev_capsule = new_cached_capsuled;

        for node in parsed_nodes {
            self.on_measurement_node_hq(node);
        }
    }

    /// when hq capsuled measurement msg received
    fn on_measurement_hq_capsuled_msg(&mut self, msg: &Message) -> Result<()> {
        check_sync_and_checksum_hq(msg)?;
        self.on_measurement_hq_capsuled(parse_resp!(
            msg,
            RplidarResponseHqCapsuledMeasurementNodes
        )?);
        return Ok(());
    }

    /// when hq capsuled measurement response received
    fn on_measurement_hq_capsuled(
        &mut self,
        nodes: RplidarResponseHqCapsuledMeasurementNodes,
    ) {
        for node in nodes.nodes.iter() {
            self.on_measurement_node_hq(node.clone());
        }
    }

    /// wait for next section of scan data
    fn wait_scan_data_with_timeout(&mut self, timeout: Duration) -> Result<()> {
        let opt_msg = self.channel.read_until(timeout)?;

        if let Some(msg) = opt_msg {
            match msg.cmd {
                RPLIDAR_ANS_TYPE_MEASUREMENT => {
                    self.on_measurement_node(parse_resp!(msg, RplidarResponseMeasurementNode)?)
                }
                RPLIDAR_ANS_TYPE_MEASUREMENT_CAPSULED => self.on_measurement_capsuled_msg(&msg)?,
                RPLIDAR_ANS_TYPE_MEASUREMENT_CAPSULED_ULTRA => self.on_measurement_ultra_capsuled_msg(&msg)?,
                RPLIDAR_ANS_TYPE_MEASUREMENT_HQ => self.on_measurement_hq_capsuled_msg(&msg)?,
                _ => {
                    return Err(Error::ProtocolError("unexpected response".to_owned()));
                }
            }
            return Ok(());
        } else {
            return Ok(());
        }
    }

    /// read scan point
    pub fn grab_scan_point(&mut self) -> Result<ScanPoint> {
        self.grab_scan_point_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// read scan point with timeout
    pub fn grab_scan_point_with_timeout(&mut self, timeout: Duration) -> Result<ScanPoint> {
        if self.cached_measurement_nodes.is_empty() {
            self.wait_scan_data_with_timeout(timeout)?;

            if self.cached_measurement_nodes.is_empty() {
                return Err(Error::OperationTimeout);
            }
        }

        return Ok(self.cached_measurement_nodes.pop_front().unwrap());
    }

    /// read scan frame
    pub fn grab_scan(&mut self) -> Result<Vec<ScanPoint>> {
        self.grab_scan_with_timeout(RPLIDAR_DEFAULT_TIMEOUT * 5)
    }

    /// read scan frame
    pub fn grab_scan_with_timeout(&mut self, timeout: Duration) -> Result<Vec<ScanPoint>> {
        let deadline = Instant::now() + timeout;
        let mut end = 1;

        'outter_loop: loop {
            if Instant::now() > deadline {
                return Err(Error::OperationTimeout);
            }

            if self.cached_measurement_nodes.len() <= end {
                self.wait_scan_data_with_timeout(std::cmp::min(deadline - Instant::now(), RPLIDAR_DEFAULT_TIMEOUT))?;
            }

            for i in end..self.cached_measurement_nodes.len() {
                if self.cached_measurement_nodes[i].is_sync() {
                    end = i;
                    break 'outter_loop;
                }
            }

            end = self.cached_measurement_nodes.len();
        }

        let mut out = Vec::<ScanPoint>::with_capacity(end);
        for _ in 0..end {
            if let Some(point) = self.cached_measurement_nodes.pop_front() {
                out.push(point);
            }
        }

        return Ok(out);
    }

    /// Get LIDAR health information
    pub fn get_device_health(&mut self) -> Result<Health> {
        self.get_device_health_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// Get LIDAR health information
    pub fn get_device_health_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Health> {
        if let Some(msg) = self
            .channel
            .invoke(&Message::new(RPLIDAR_CMD_GET_DEVICE_HEALTH), timeout)?
        {
            let resp = handle_resp!(RPLIDAR_ANS_TYPE_DEVHEALTH, msg, RplidarResponseDeviceHealth)?;

            return Ok(match resp.status {
                RPLIDAR_HEALTH_STATUS_OK => Health::Healthy,
                RPLIDAR_HEALTH_STATUS_WARNING => Health::Warning(resp.error_code),
                RPLIDAR_HEALTH_STATUS_ERROR => Health::Error(resp.error_code),
                _ => Health::Healthy
            });
        }

        return Err(Error::OperationTimeout);
    }

    /// Check if the connected LIDAR supports motor control
    pub fn check_motor_ctrl_support(&mut self) -> Result<bool> {
        self.check_motor_ctrl_support_with_timeout(RPLIDAR_DEFAULT_TIMEOUT)
    }

    /// Check if the connected LIDAR supports motor control with timeout
    pub fn check_motor_ctrl_support_with_timeout(&mut self, timeout: Duration) -> Result<bool> {
        let mut data = [0u8; 4];
        LittleEndian::write_u32(&mut data, 0u32);

        let resp_msg = self.channel.invoke(&Message::with_data(RPLIDAR_CMD_GET_ACC_BOARD_FLAG, &data), timeout)?;

        if let Some(msg) = resp_msg {
            let support_flag = handle_resp!(RPLIDAR_ANS_TYPE_ACC_BOARD_FLAG, msg, u32)?;
            
            return Ok((support_flag & RPLIDAR_RESP_ACC_BOARD_FLAG_MOTOR_CTRL_SUPPORT_MASK) == RPLIDAR_RESP_ACC_BOARD_FLAG_MOTOR_CTRL_SUPPORT_MASK);
        } else {
            return Err(Error::OperationTimeout);
        }
    }
}

fn check_sync_and_checksum(msg: &Message) -> Result<()> {
    if msg.data.len() < 2 {
        return Err(Error::ProtocolError("data too short".to_owned()));
    }

    if (msg.data[0] >> 4) != RPLIDAR_RESP_MEASUREMENT_EXP_SYNC_1 {
        return Err(Error::ProtocolError("miss sync 1".to_owned()));
    }

    if (msg.data[1] >> 4) != RPLIDAR_RESP_MEASUREMENT_EXP_SYNC_2 {
        return Err(Error::ProtocolError("miss sync 2".to_owned()));
    }

    let recv_checksum = (msg.data[0] & 0xf) | (msg.data[1] << 4);
    let mut checksum = Checksum::new();
    checksum.push_slice(&msg.data[2..]);

    if checksum.checksum() != recv_checksum {
        return Err(Error::ProtocolError("checksum mismatch".to_owned()));
    } else {
        return Ok(());
    }
}

fn check_sync_and_checksum_hq(msg: &Message) -> Result<()> {
    if msg.data.len() != std::mem::size_of::<RplidarResponseHqCapsuledMeasurementNodes>() {
        return Err(Error::ProtocolError("data length mismatch".to_owned()));
    }

    if msg.data[0] != RPLIDAR_RESP_MEASUREMENT_HQ_SYNC {
        return Err(Error::ProtocolError("sync mismatch".to_owned()));
    }

    let checksum = crc32::checksum_ieee(&msg.data[0..msg.data.len()-4]);
    let recv_checksum = LittleEndian::read_u32(&msg.data[msg.data.len()-4..msg.data.len()]);

    if checksum != recv_checksum {
        return Err(Error::ProtocolError("checksum mismatch".to_owned()));
    } else {
        return Ok(());
    }
}
