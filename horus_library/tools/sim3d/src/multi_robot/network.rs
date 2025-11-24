//! Network simulation with latency, packet loss, and bandwidth limits

use super::communication::RobotMessage;
use bevy::prelude::*;
use rand::Rng;
use std::collections::VecDeque;

/// Network packet with delivery time
#[derive(Clone, Debug)]
struct NetworkPacket {
    message: RobotMessage,
    delivery_time: f64,
}

/// Network simulation configuration
#[derive(Clone)]
pub struct NetworkConfig {
    /// Base latency in seconds
    pub base_latency: f64,
    /// Latency variance (std dev)
    pub latency_variance: f64,
    /// Packet loss probability (0.0 to 1.0)
    pub packet_loss_rate: f64,
    /// Bandwidth limit in bytes per second (None = unlimited)
    pub bandwidth_limit: Option<usize>,
    /// Maximum jitter in seconds
    pub jitter: f64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            base_latency: 0.01,      // 10ms
            latency_variance: 0.002, // 2ms std dev
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            jitter: 0.005, // 5ms max jitter
        }
    }
}

impl NetworkConfig {
    /// Create WiFi-like network configuration
    pub fn wifi() -> Self {
        Self {
            base_latency: 0.005,
            latency_variance: 0.003,
            packet_loss_rate: 0.01,                // 1% loss
            bandwidth_limit: Some(54_000_000 / 8), // 54 Mbps
            jitter: 0.010,
        }
    }

    /// Create 4G/LTE-like network configuration
    pub fn lte() -> Self {
        Self {
            base_latency: 0.050,
            latency_variance: 0.020,
            packet_loss_rate: 0.02,                 // 2% loss
            bandwidth_limit: Some(100_000_000 / 8), // 100 Mbps
            jitter: 0.030,
        }
    }

    /// Create local wired network configuration
    pub fn wired() -> Self {
        Self {
            base_latency: 0.001,
            latency_variance: 0.0001,
            packet_loss_rate: 0.0001,                 // 0.01% loss
            bandwidth_limit: Some(1_000_000_000 / 8), // 1 Gbps
            jitter: 0.001,
        }
    }

    /// Create unreliable network configuration for testing
    pub fn unreliable() -> Self {
        Self {
            base_latency: 0.100,
            latency_variance: 0.050,
            packet_loss_rate: 0.10,               // 10% loss
            bandwidth_limit: Some(1_000_000 / 8), // 1 Mbps
            jitter: 0.100,
        }
    }
}

/// Network simulator resource
#[derive(Resource)]
pub struct NetworkSimulator {
    /// Pending packets to be delivered
    packets: VecDeque<NetworkPacket>,
    /// Network configuration
    config: NetworkConfig,
    /// Current simulation time
    current_time: f64,
    /// Bytes transmitted this second
    bytes_this_second: usize,
    /// Last reset time for bandwidth tracking
    last_bandwidth_reset: f64,
    /// Statistics
    packets_sent: u64,
    packets_dropped: u64,
    packets_delivered: u64,
}

impl Default for NetworkSimulator {
    fn default() -> Self {
        Self::new(NetworkConfig::default())
    }
}

impl NetworkSimulator {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            packets: VecDeque::new(),
            config,
            current_time: 0.0,
            bytes_this_second: 0,
            last_bandwidth_reset: 0.0,
            packets_sent: 0,
            packets_dropped: 0,
            packets_delivered: 0,
        }
    }

    /// Send a message through the network
    pub fn send(&mut self, message: RobotMessage) -> Result<(), String> {
        // Check packet loss
        let mut rng = rand::thread_rng();
        if rng.gen::<f64>() < self.config.packet_loss_rate {
            self.packets_dropped += 1;
            return Ok(()); // Packet lost
        }

        // Check bandwidth limit
        if let Some(limit) = self.config.bandwidth_limit {
            // Reset bandwidth counter every second
            if self.current_time - self.last_bandwidth_reset >= 1.0 {
                self.bytes_this_second = 0;
                self.last_bandwidth_reset = self.current_time;
            }

            // Check if we're over the limit
            if self.bytes_this_second + message.size_bytes() > limit {
                self.packets_dropped += 1;
                return Err("Bandwidth limit exceeded".to_string());
            }

            self.bytes_this_second += message.size_bytes();
        }

        // Calculate delivery time with latency and jitter
        let latency = self.calculate_latency();
        let delivery_time = self.current_time + latency;

        self.packets.push_back(NetworkPacket {
            message,
            delivery_time,
        });

        self.packets_sent += 1;
        Ok(())
    }

    /// Get all messages ready for delivery
    pub fn receive(&mut self) -> Vec<RobotMessage> {
        let mut delivered = Vec::new();

        while let Some(packet) = self.packets.front() {
            if packet.delivery_time <= self.current_time {
                let packet = self.packets.pop_front().unwrap();
                delivered.push(packet.message);
                self.packets_delivered += 1;
            } else {
                break;
            }
        }

        delivered
    }

    /// Update simulation time
    pub fn update_time(&mut self, delta: f64) {
        self.current_time += delta;
    }

    /// Calculate latency with variance and jitter
    fn calculate_latency(&self) -> f64 {
        let mut rng = rand::thread_rng();

        // Base latency with Gaussian variance
        let latency = self.config.base_latency
            + rng.gen::<f64>() * self.config.latency_variance * 2.0
            - self.config.latency_variance;

        // Add random jitter
        let jitter = (rng.gen::<f64>() - 0.5) * self.config.jitter * 2.0;

        (latency + jitter).max(0.0)
    }

    /// Get number of pending packets
    pub fn pending_count(&self) -> usize {
        self.packets.len()
    }

    /// Get network statistics
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            packets_sent: self.packets_sent,
            packets_dropped: self.packets_dropped,
            packets_delivered: self.packets_delivered,
            pending_packets: self.packets.len(),
        }
    }

    /// Update configuration
    pub fn set_config(&mut self, config: NetworkConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }
}

/// Network statistics
#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub packets_delivered: u64,
    pub pending_packets: usize,
}

impl NetworkStats {
    pub fn packet_loss_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            0.0
        } else {
            self.packets_dropped as f64 / self.packets_sent as f64
        }
    }

    pub fn delivery_rate(&self) -> f64 {
        if self.packets_sent == 0 {
            0.0
        } else {
            self.packets_delivered as f64 / self.packets_sent as f64
        }
    }
}

/// System to simulate network and deliver messages
pub fn network_simulation_system(
    mut network: ResMut<NetworkSimulator>,
    mut comm_manager: ResMut<super::communication::CommunicationManager>,
    time: Res<Time>,
) {
    // Update network time
    network.update_time(time.delta_secs_f64());

    // Deliver ready packets
    let messages = network.receive();
    for message in messages {
        // Deliver to communication manager
        if let Some(recipient) = &message.to {
            let _ = comm_manager.send_message(
                message.from.clone(),
                Some(recipient.clone()),
                message.payload,
                message.timestamp,
            );
        } else {
            // Broadcast
            let _ = comm_manager.send_message(
                message.from.clone(),
                None,
                message.payload,
                message.timestamp,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_robot::RobotId;

    #[test]
    fn test_network_config_presets() {
        let wifi = NetworkConfig::wifi();
        assert!(wifi.base_latency < 0.01);
        assert!(wifi.packet_loss_rate > 0.0);

        let lte = NetworkConfig::lte();
        assert!(lte.base_latency > wifi.base_latency);

        let wired = NetworkConfig::wired();
        assert!(wired.base_latency < wifi.base_latency);
        assert!(wired.packet_loss_rate < 0.001);
    }

    #[test]
    fn test_network_simulator() {
        let mut network = NetworkSimulator::new(NetworkConfig::default());

        let msg = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![1, 2, 3],
            0.0,
        );

        network.send(msg).unwrap();
        assert_eq!(network.pending_count(), 1);
    }

    #[test]
    fn test_message_delivery() {
        let mut network = NetworkSimulator::new(NetworkConfig {
            base_latency: 0.1,
            latency_variance: 0.0,
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            jitter: 0.0,
        });

        let msg = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![1, 2, 3],
            0.0,
        );

        network.send(msg).unwrap();

        // Should not be delivered yet
        let delivered = network.receive();
        assert_eq!(delivered.len(), 0);

        // Advance time
        network.update_time(0.15);

        // Should be delivered now
        let delivered = network.receive();
        assert_eq!(delivered.len(), 1);
    }

    #[test]
    fn test_packet_loss() {
        let mut network = NetworkSimulator::new(NetworkConfig {
            base_latency: 0.0,
            latency_variance: 0.0,
            packet_loss_rate: 1.0, // 100% loss
            bandwidth_limit: None,
            jitter: 0.0,
        });

        for _ in 0..100 {
            let msg = RobotMessage::new(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1],
                0.0,
            );
            let _ = network.send(msg);
        }

        let stats = network.get_stats();
        // With 100% loss, no packets should be sent
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_dropped, 100);
    }

    #[test]
    fn test_bandwidth_limit() {
        let mut network = NetworkSimulator::new(NetworkConfig {
            base_latency: 0.0,
            latency_variance: 0.0,
            packet_loss_rate: 0.0,
            bandwidth_limit: Some(100), // 100 bytes per second
            jitter: 0.0,
        });

        // Send 50 bytes - should succeed
        let msg1 = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![0; 50],
            0.0,
        );
        assert!(network.send(msg1).is_ok());

        // Send another 60 bytes - should fail (total 110 > 100)
        let msg2 = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![0; 60],
            0.0,
        );
        assert!(network.send(msg2).is_err());

        // After 1 second, should work again
        network.update_time(1.0);
        let msg3 = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![0; 60],
            0.0,
        );
        assert!(network.send(msg3).is_ok());
    }

    #[test]
    fn test_statistics() {
        let mut network = NetworkSimulator::new(NetworkConfig {
            base_latency: 0.0,
            latency_variance: 0.0,
            packet_loss_rate: 0.5, // 50% loss
            bandwidth_limit: None,
            jitter: 0.0,
        });

        for _ in 0..100 {
            let msg = RobotMessage::new(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1],
                0.0,
            );
            let _ = network.send(msg);
        }

        network.update_time(1.0);
        let _ = network.receive();

        let stats = network.get_stats();

        // Total attempts = packets_sent + packets_dropped
        let total_attempts = stats.packets_sent + stats.packets_dropped;
        assert_eq!(total_attempts, 100);

        // With 50% loss rate, approximately 50 should be dropped
        // (allowing for randomness)
        assert!(stats.packets_dropped > 30 && stats.packets_dropped < 70);
        assert!(stats.packets_delivered > 30 && stats.packets_delivered < 70);
    }

    #[test]
    fn test_latency_variance() {
        let mut network = NetworkSimulator::new(NetworkConfig {
            base_latency: 0.1,
            latency_variance: 0.05,
            packet_loss_rate: 0.0,
            bandwidth_limit: None,
            jitter: 0.0,
        });

        let mut delivery_times = Vec::new();

        for _ in 0..10 {
            let msg = RobotMessage::new(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1],
                0.0,
            );
            network.send(msg).unwrap();
        }

        // Check that delivery times vary
        for packet in &network.packets {
            delivery_times.push(packet.delivery_time);
        }

        // Check that at least some delivery times are different
        let mut has_different_values = false;
        if delivery_times.len() > 1 {
            let first = delivery_times[0];
            for &time in &delivery_times[1..] {
                if (time - first).abs() > 0.0001 {
                    has_different_values = true;
                    break;
                }
            }
        }
        assert!(has_different_values);
    }
}
