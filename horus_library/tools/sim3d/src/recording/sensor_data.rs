use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Sensor data types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SensorData {
    Image {
        width: u32,
        height: u32,
        format: ImageFormat,
        data: Vec<u8>,
    },
    PointCloud {
        points: Vec<[f32; 3]>,
        intensities: Option<Vec<f32>>,
    },
    LaserScan {
        ranges: Vec<f32>,
        intensities: Option<Vec<f32>>,
        angle_min: f32,
        angle_max: f32,
        angle_increment: f32,
        range_min: f32,
        range_max: f32,
    },
    IMU {
        orientation: [f32; 4],
        angular_velocity: [f32; 3],
        linear_acceleration: [f32; 3],
    },
    Odometry {
        position: [f32; 3],
        orientation: [f32; 4],
        linear_velocity: [f32; 3],
        angular_velocity: [f32; 3],
    },
    GPS {
        latitude: f64,
        longitude: f64,
        altitude: f64,
    },
    Custom {
        data_type: String,
        data: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageFormat {
    RGB8,
    RGBA8,
    Grayscale8,
    Depth32F,
}

/// Timestamped sensor message
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorMessage {
    pub timestamp: f64,
    pub topic: String,
    pub data: SensorData,
}

/// Sensor bag file (rosbag-like)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorBag {
    pub messages: Vec<SensorMessage>,
    pub metadata: BagMetadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BagMetadata {
    pub name: String,
    pub start_time: f64,
    pub end_time: f64,
    pub message_count: usize,
    pub topics: HashMap<String, TopicInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopicInfo {
    pub message_count: usize,
    pub message_type: String,
    pub frequency: f64,
}

impl SensorBag {
    pub fn new(name: String) -> Self {
        Self {
            messages: Vec::new(),
            metadata: BagMetadata {
                name,
                start_time: 0.0,
                end_time: 0.0,
                message_count: 0,
                topics: HashMap::new(),
            },
        }
    }

    pub fn add_message(&mut self, message: SensorMessage) {
        // Update metadata
        if self.messages.is_empty() {
            self.metadata.start_time = message.timestamp;
        }
        self.metadata.end_time = message.timestamp;
        self.metadata.message_count += 1;

        // Update topic info
        let data_type = match &message.data {
            SensorData::Image { .. } => "Image",
            SensorData::PointCloud { .. } => "PointCloud",
            SensorData::LaserScan { .. } => "LaserScan",
            SensorData::IMU { .. } => "IMU",
            SensorData::Odometry { .. } => "Odometry",
            SensorData::GPS { .. } => "GPS",
            SensorData::Custom { data_type, .. } => data_type.as_str(),
        };

        self.metadata
            .topics
            .entry(message.topic.clone())
            .and_modify(|info| info.message_count += 1)
            .or_insert(TopicInfo {
                message_count: 1,
                message_type: data_type.to_string(),
                frequency: 0.0,
            });

        self.messages.push(message);
    }

    /// Calculate topic frequencies
    pub fn calculate_frequencies(&mut self) {
        let duration = self.metadata.end_time - self.metadata.start_time;
        if duration > 0.0 {
            for info in self.metadata.topics.values_mut() {
                info.frequency = info.message_count as f64 / duration;
            }
        }
    }

    /// Get messages for a specific topic
    pub fn get_messages(&self, topic: &str) -> Vec<&SensorMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.topic == topic)
            .collect()
    }

    /// Get messages in time range
    pub fn get_messages_in_range(&self, start: f64, end: f64) -> Vec<&SensorMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.timestamp >= start && msg.timestamp <= end)
            .collect()
    }

    /// Save to file (MessagePack format for efficiency)
    pub fn save_to_file(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        self.calculate_frequencies();
        let data = rmp_serde::to_vec(&self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load from file
    pub fn load_from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        let bag = rmp_serde::from_slice(&data)?;
        Ok(bag)
    }

    /// Export to JSON (for debugging/analysis)
    pub fn export_to_json(&self, path: &PathBuf) -> anyhow::Result<()> {
        let data = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

/// Sensor bag recorder resource
#[derive(Resource)]
pub struct SensorBagRecorder {
    pub active: bool,
    pub bag: SensorBag,
    pub compression_enabled: bool,
}

impl SensorBagRecorder {
    pub fn new(name: String) -> Self {
        Self {
            active: false,
            bag: SensorBag::new(name),
            compression_enabled: true,
        }
    }

    pub fn start_recording(&mut self) {
        self.active = true;
    }

    pub fn stop_recording(&mut self) {
        self.active = false;
    }

    pub fn record_message(&mut self, message: SensorMessage) {
        if self.active {
            self.bag.add_message(message);
        }
    }
}

/// Marker component for sensors to record
#[derive(Component, Clone, Debug)]
pub struct RecordSensor {
    pub topic: String,
    pub recording_rate: f64,
    pub last_record_time: f64,
}

impl RecordSensor {
    pub fn new(topic: String, rate: f64) -> Self {
        Self {
            topic,
            recording_rate: rate,
            last_record_time: -1.0,
        }
    }

    pub fn should_record(&mut self, current_time: f64) -> bool {
        let interval = 1.0 / self.recording_rate;
        if current_time - self.last_record_time >= interval {
            self.last_record_time = current_time;
            true
        } else {
            false
        }
    }
}

/// Bag playback resource
#[derive(Resource)]
pub struct SensorBagPlayback {
    pub bag: SensorBag,
    pub current_time: f64,
    pub playing: bool,
    pub loop_playback: bool,
    pub playback_speed: f64,
    pub current_message_idx: usize,
}

impl SensorBagPlayback {
    pub fn new(bag: SensorBag) -> Self {
        Self {
            bag,
            current_time: 0.0,
            playing: false,
            loop_playback: false,
            playback_speed: 1.0,
            current_message_idx: 0,
        }
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn reset(&mut self) {
        self.current_time = 0.0;
        self.current_message_idx = 0;
    }

    /// Get messages to publish at current time
    pub fn get_messages_at_time(&mut self, dt: f64) -> Vec<SensorMessage> {
        if !self.playing {
            return Vec::new();
        }

        self.current_time += dt * self.playback_speed;

        let mut messages = Vec::new();
        let playback_time = self.bag.metadata.start_time + self.current_time;

        while self.current_message_idx < self.bag.messages.len() {
            let msg = &self.bag.messages[self.current_message_idx];
            if msg.timestamp <= playback_time {
                messages.push(msg.clone());
                self.current_message_idx += 1;
            } else {
                break;
            }
        }

        // Check if finished
        if self.current_message_idx >= self.bag.messages.len() {
            if self.loop_playback {
                self.reset();
            } else {
                self.pause();
            }
        }

        messages
    }
}

/// Statistics about sensor bag
#[derive(Clone, Debug)]
pub struct BagStatistics {
    pub duration: f64,
    pub total_messages: usize,
    pub topics: HashMap<String, TopicStatistics>,
    pub estimated_size_mb: f64,
}

#[derive(Clone, Debug)]
pub struct TopicStatistics {
    pub message_count: usize,
    pub frequency: f64,
    pub message_type: String,
    pub first_timestamp: f64,
    pub last_timestamp: f64,
}

impl BagStatistics {
    pub fn from_bag(bag: &SensorBag) -> Self {
        let mut topics = HashMap::new();

        for (topic_name, topic_info) in &bag.metadata.topics {
            let messages = bag.get_messages(topic_name);
            let first = messages.first().map(|m| m.timestamp).unwrap_or(0.0);
            let last = messages.last().map(|m| m.timestamp).unwrap_or(0.0);

            topics.insert(
                topic_name.clone(),
                TopicStatistics {
                    message_count: topic_info.message_count,
                    frequency: topic_info.frequency,
                    message_type: topic_info.message_type.clone(),
                    first_timestamp: first,
                    last_timestamp: last,
                },
            );
        }

        // Estimate size (rough)
        let estimated_size = std::mem::size_of_val(&bag.messages[..]) as f64 / (1024.0 * 1024.0);

        Self {
            duration: bag.metadata.end_time - bag.metadata.start_time,
            total_messages: bag.metadata.message_count,
            topics,
            estimated_size_mb: estimated_size,
        }
    }

    pub fn print_summary(&self) {
        println!("Bag Statistics:");
        println!("  Duration: {:.2}s", self.duration);
        println!("  Total Messages: {}", self.total_messages);
        println!("  Estimated Size: {:.2} MB", self.estimated_size_mb);
        println!("  Topics:");
        for (name, stats) in &self.topics {
            println!(
                "    {}: {} messages at {:.1} Hz ({})",
                name, stats.message_count, stats.frequency, stats.message_type
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_bag_creation() {
        let bag = SensorBag::new("test".to_string());
        assert_eq!(bag.metadata.name, "test");
        assert_eq!(bag.messages.len(), 0);
    }

    #[test]
    fn test_add_message() {
        let mut bag = SensorBag::new("test".to_string());
        bag.add_message(SensorMessage {
            timestamp: 1.0,
            topic: "camera".to_string(),
            data: SensorData::Image {
                width: 640,
                height: 480,
                format: ImageFormat::RGB8,
                data: vec![0; 640 * 480 * 3],
            },
        });

        assert_eq!(bag.messages.len(), 1);
        assert_eq!(bag.metadata.message_count, 1);
        assert!(bag.metadata.topics.contains_key("camera"));
    }

    #[test]
    fn test_get_messages_for_topic() {
        let mut bag = SensorBag::new("test".to_string());
        bag.add_message(SensorMessage {
            timestamp: 1.0,
            topic: "camera".to_string(),
            data: SensorData::Image {
                width: 640,
                height: 480,
                format: ImageFormat::RGB8,
                data: vec![],
            },
        });
        bag.add_message(SensorMessage {
            timestamp: 2.0,
            topic: "imu".to_string(),
            data: SensorData::IMU {
                orientation: [0.0, 0.0, 0.0, 1.0],
                angular_velocity: [0.0, 0.0, 0.0],
                linear_acceleration: [0.0, 0.0, 9.81],
            },
        });

        let camera_msgs = bag.get_messages("camera");
        assert_eq!(camera_msgs.len(), 1);

        let imu_msgs = bag.get_messages("imu");
        assert_eq!(imu_msgs.len(), 1);
    }

    #[test]
    fn test_get_messages_in_range() {
        let mut bag = SensorBag::new("test".to_string());
        for i in 0..10 {
            bag.add_message(SensorMessage {
                timestamp: i as f64,
                topic: "test".to_string(),
                data: SensorData::GPS {
                    latitude: 0.0,
                    longitude: 0.0,
                    altitude: 0.0,
                },
            });
        }

        let msgs = bag.get_messages_in_range(2.0, 5.0);
        assert_eq!(msgs.len(), 4); // 2.0, 3.0, 4.0, 5.0
    }

    #[test]
    fn test_frequency_calculation() {
        let mut bag = SensorBag::new("test".to_string());
        for i in 0..10 {
            bag.add_message(SensorMessage {
                timestamp: i as f64 * 0.1, // 0.0, 0.1, 0.2, ...
                topic: "test".to_string(),
                data: SensorData::GPS {
                    latitude: 0.0,
                    longitude: 0.0,
                    altitude: 0.0,
                },
            });
        }

        bag.calculate_frequencies();
        let topic_info = bag.metadata.topics.get("test").unwrap();
        // Frequency should be messages / duration = 10 / 0.9 = 11.11 Hz
        assert!(topic_info.frequency > 8.0 && topic_info.frequency < 13.0);
    }

    #[test]
    fn test_record_sensor() {
        let mut sensor = RecordSensor::new("camera".to_string(), 30.0);
        assert!(sensor.should_record(0.0));
        assert!(!sensor.should_record(0.01)); // Too soon for 30 Hz
        assert!(sensor.should_record(0.04)); // >1/30 seconds passed
    }

    #[test]
    fn test_bag_playback() {
        let mut bag = SensorBag::new("test".to_string());
        for i in 0..5 {
            bag.add_message(SensorMessage {
                timestamp: i as f64 * 0.1,
                topic: "test".to_string(),
                data: SensorData::GPS {
                    latitude: i as f64,
                    longitude: 0.0,
                    altitude: 0.0,
                },
            });
        }

        let mut playback = SensorBagPlayback::new(bag);
        playback.play();

        let messages = playback.get_messages_at_time(0.15);
        assert!(!messages.is_empty());
    }

    #[test]
    fn test_bag_statistics() {
        let mut bag = SensorBag::new("test".to_string());
        for i in 0..100 {
            bag.add_message(SensorMessage {
                timestamp: i as f64 * 0.01,
                topic: "test".to_string(),
                data: SensorData::GPS {
                    latitude: 0.0,
                    longitude: 0.0,
                    altitude: 0.0,
                },
            });
        }

        bag.calculate_frequencies();
        let stats = BagStatistics::from_bag(&bag);

        assert_eq!(stats.total_messages, 100);
        assert!(stats.duration > 0.0);
        assert_eq!(stats.topics.len(), 1);
    }
}
