//! Inter-robot communication simulation

use super::RobotId;
use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

/// Message between robots
#[derive(Clone, Debug)]
pub struct RobotMessage {
    /// Sender robot ID
    pub from: RobotId,
    /// Receiver robot ID (None for broadcast)
    pub to: Option<RobotId>,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message timestamp
    pub timestamp: f64,
    /// Message ID
    pub id: u64,
}

impl RobotMessage {
    pub fn new(from: RobotId, to: Option<RobotId>, payload: Vec<u8>, timestamp: f64) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Self {
            from,
            to,
            payload,
            timestamp,
            id,
        }
    }

    pub fn is_broadcast(&self) -> bool {
        self.to.is_none()
    }

    pub fn size_bytes(&self) -> usize {
        self.payload.len()
    }
}

/// Communication channel configuration
#[derive(Clone)]
pub struct ChannelConfig {
    /// Maximum messages in queue per robot
    pub max_queue_size: usize,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Bandwidth limit (bytes per second)
    pub bandwidth_limit: Option<usize>,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_message_size: 1024 * 1024, // 1MB
            bandwidth_limit: None,
        }
    }
}

/// Communication manager resource
#[derive(Resource)]
pub struct CommunicationManager {
    /// Message queues per robot
    queues: HashMap<RobotId, VecDeque<RobotMessage>>,
    /// Channel configuration
    config: ChannelConfig,
    /// Sent message count
    sent_count: u64,
    /// Received message count
    received_count: u64,
    /// Total bytes transmitted
    bytes_transmitted: usize,
}

impl Default for CommunicationManager {
    fn default() -> Self {
        Self::new(ChannelConfig::default())
    }
}

impl CommunicationManager {
    pub fn new(config: ChannelConfig) -> Self {
        Self {
            queues: HashMap::new(),
            config,
            sent_count: 0,
            received_count: 0,
            bytes_transmitted: 0,
        }
    }

    /// Send a message from one robot to another
    pub fn send_message(
        &mut self,
        from: RobotId,
        to: Option<RobotId>,
        payload: Vec<u8>,
        timestamp: f64,
    ) -> Result<(), String> {
        // Check message size
        if payload.len() > self.config.max_message_size {
            return Err(format!(
                "Message size {} exceeds maximum {}",
                payload.len(),
                self.config.max_message_size
            ));
        }

        let message = RobotMessage::new(from, to.clone(), payload, timestamp);
        let message_size = message.size_bytes();

        if let Some(recipient) = to {
            // Unicast message
            self.deliver_to_robot(&recipient, message)?;
        } else {
            // Broadcast message - deliver to all robots except sender
            for robot_id in self.queues.keys().cloned().collect::<Vec<_>>() {
                if robot_id != message.from {
                    self.deliver_to_robot(&robot_id, message.clone())?;
                }
            }
        }

        self.sent_count += 1;
        self.bytes_transmitted += message_size;

        Ok(())
    }

    /// Deliver message to specific robot
    fn deliver_to_robot(
        &mut self,
        robot_id: &RobotId,
        message: RobotMessage,
    ) -> Result<(), String> {
        let queue = self
            .queues
            .entry(robot_id.clone())
            .or_insert_with(VecDeque::new);

        if queue.len() >= self.config.max_queue_size {
            return Err(format!("Message queue full for robot {:?}", robot_id));
        }

        queue.push_back(message);
        self.received_count += 1;

        Ok(())
    }

    /// Receive next message for a robot
    pub fn receive_message(&mut self, robot_id: &RobotId) -> Option<RobotMessage> {
        self.queues
            .get_mut(robot_id)
            .and_then(|queue| queue.pop_front())
    }

    /// Peek at next message without removing it
    pub fn peek_message(&self, robot_id: &RobotId) -> Option<&RobotMessage> {
        self.queues.get(robot_id).and_then(|queue| queue.front())
    }

    /// Get number of pending messages for a robot
    pub fn pending_count(&self, robot_id: &RobotId) -> usize {
        self.queues.get(robot_id).map_or(0, |q| q.len())
    }

    /// Clear all messages for a robot
    pub fn clear_queue(&mut self, robot_id: &RobotId) {
        if let Some(queue) = self.queues.get_mut(robot_id) {
            queue.clear();
        }
    }

    /// Register a robot (creates empty queue)
    pub fn register_robot(&mut self, robot_id: RobotId) {
        self.queues.entry(robot_id).or_insert_with(VecDeque::new);
    }

    /// Unregister a robot
    pub fn unregister_robot(&mut self, robot_id: &RobotId) {
        self.queues.remove(robot_id);
    }

    /// Get statistics
    pub fn get_stats(&self) -> CommunicationStats {
        CommunicationStats {
            sent_count: self.sent_count,
            received_count: self.received_count,
            bytes_transmitted: self.bytes_transmitted,
            active_robots: self.queues.len(),
        }
    }
}

/// Communication statistics
#[derive(Debug, Clone, Copy)]
pub struct CommunicationStats {
    pub sent_count: u64,
    pub received_count: u64,
    pub bytes_transmitted: usize,
    pub active_robots: usize,
}

/// Component for robots that can communicate
#[derive(Component)]
pub struct Communicator {
    /// Callback for received messages (optional)
    pub on_message: Option<fn(&RobotMessage)>,
}

impl Default for Communicator {
    fn default() -> Self {
        Self { on_message: None }
    }
}

/// System to process communication
pub fn communication_system(
    mut manager: ResMut<CommunicationManager>,
    communicators: Query<(&super::Robot, &Communicator)>,
) {
    // Process received messages
    for (robot, communicator) in communicators.iter() {
        while let Some(message) = manager.receive_message(&robot.id) {
            if let Some(callback) = communicator.on_message {
                callback(&message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = RobotMessage::new(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![1, 2, 3],
            1.0,
        );

        assert_eq!(msg.from.as_str(), "robot1");
        assert_eq!(msg.to.as_ref().unwrap().as_str(), "robot2");
        assert_eq!(msg.payload, vec![1, 2, 3]);
        assert!(!msg.is_broadcast());
        assert_eq!(msg.size_bytes(), 3);
    }

    #[test]
    fn test_broadcast_message() {
        let msg = RobotMessage::new(RobotId::new("robot1"), None, vec![1, 2, 3], 1.0);

        assert!(msg.is_broadcast());
    }

    #[test]
    fn test_communication_manager() {
        let mut manager = CommunicationManager::default();

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        manager
            .send_message(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1, 2, 3],
                1.0,
            )
            .unwrap();

        assert_eq!(manager.pending_count(&RobotId::new("robot2")), 1);
        assert_eq!(manager.pending_count(&RobotId::new("robot1")), 0);

        let msg = manager.receive_message(&RobotId::new("robot2")).unwrap();
        assert_eq!(msg.payload, vec![1, 2, 3]);
        assert_eq!(manager.pending_count(&RobotId::new("robot2")), 0);
    }

    #[test]
    fn test_broadcast() {
        let mut manager = CommunicationManager::default();

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));
        manager.register_robot(RobotId::new("robot3"));

        manager
            .send_message(RobotId::new("robot1"), None, vec![1, 2, 3], 1.0)
            .unwrap();

        // robot1 shouldn't receive its own broadcast
        assert_eq!(manager.pending_count(&RobotId::new("robot1")), 0);
        // Others should receive it
        assert_eq!(manager.pending_count(&RobotId::new("robot2")), 1);
        assert_eq!(manager.pending_count(&RobotId::new("robot3")), 1);
    }

    #[test]
    fn test_message_size_limit() {
        let mut manager = CommunicationManager::new(ChannelConfig {
            max_message_size: 10,
            ..Default::default()
        });

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        let result = manager.send_message(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![0; 100],
            1.0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_queue_size_limit() {
        let mut manager = CommunicationManager::new(ChannelConfig {
            max_queue_size: 2,
            ..Default::default()
        });

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        manager
            .send_message(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1],
                1.0,
            )
            .unwrap();

        manager
            .send_message(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![2],
                2.0,
            )
            .unwrap();

        let result = manager.send_message(
            RobotId::new("robot1"),
            Some(RobotId::new("robot2")),
            vec![3],
            3.0,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_statistics() {
        let mut manager = CommunicationManager::default();

        manager.register_robot(RobotId::new("robot1"));
        manager.register_robot(RobotId::new("robot2"));

        manager
            .send_message(
                RobotId::new("robot1"),
                Some(RobotId::new("robot2")),
                vec![1, 2, 3, 4, 5],
                1.0,
            )
            .unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.sent_count, 1);
        assert_eq!(stats.received_count, 1);
        assert_eq!(stats.bytes_transmitted, 5);
        assert_eq!(stats.active_robots, 2);
    }
}
