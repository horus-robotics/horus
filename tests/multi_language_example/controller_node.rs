#!/usr/bin/env rust
//! Rust Controller Node - Multi-Language Example
//!
//! Subscribes to robot pose data from Python node and generates velocity commands.
//! Demonstrates Rust <- Python communication using GenericMessage with MessagePack serialization.

use horus::prelude::*;
use std::collections::HashMap;
extern crate rmp_serde;
use serde::{Serialize, Deserialize};

// Generic message type for cross-language communication (compatible with PyHub)
#[derive(Clone, Debug, Serialize, Deserialize)]
struct GenericMessage {
    data: Vec<u8>,
    metadata: Option<String>,
}

// Simple pose structure to store robot position
#[derive(Clone, Debug)]
struct Pose {
    x: f64,
    y: f64,
    theta: f64,
}

impl LogSummary for GenericMessage {
    fn log_summary(&self) -> String {
        format!("<data: {} bytes>", self.data.len())
    }
}

fn main() -> Result<()> {
    println!("==============================================================");
    println!("Rust Controller Node - Multi-Language Example");
    println!("==============================================================");
    println!("Subscribing to 'robot_pose' from Python node");
    println!("Publishing velocity commands at 20Hz on 'cmd_vel'");
    println!();

    // Create generic hubs for cross-language communication
    let pose_hub = Hub::<GenericMessage>::new("robot_pose")?;
    let cmd_hub = Hub::<GenericMessage>::new("cmd_vel")?;

    let mut last_pose: Option<Pose> = None;
    let mut tick_count = 0;

    // Control loop at 20Hz (faster than Python's 10Hz)
    loop {
        tick_count += 1;

        // Try to receive pose from Python node
        if let Some(msg) = pose_hub.recv(None) {
            // Deserialize MessagePack data from Python
            match rmp_serde::from_slice::<HashMap<String, f64>>(&msg.data) {
                Ok(pose_map) => {
                    let pose = Pose {
                        x: *pose_map.get("x").unwrap_or(&0.0),
                        y: *pose_map.get("y").unwrap_or(&0.0),
                        theta: *pose_map.get("theta").unwrap_or(&0.0),
                    };

                    // Calculate distance moved since last pose
                    let distance = if let Some(ref last) = last_pose {
                        ((pose.x - last.x).powi(2) + (pose.y - last.y).powi(2)).sqrt()
                    } else {
                        0.0
                    };

                    // Simple proportional controller based on distance from origin
                    let distance_from_origin = (pose.x * pose.x + pose.y * pose.y).sqrt();
                    let linear_vel = if distance_from_origin > 1.5 {
                        1.5 // Move forward if far from origin
                    } else {
                        0.5 // Slow down if close
                    };

                    // Angular velocity to maintain circular motion
                    let angular_vel = 0.5;

                    // Create velocity command and serialize to MessagePack
                    let mut cmd_map = HashMap::new();
                    cmd_map.insert("linear", linear_vel);
                    cmd_map.insert("angular", angular_vel);

                    if let Ok(data) = rmp_serde::to_vec(&cmd_map) {
                        let cmd_msg = GenericMessage {
                            data,
                            metadata: None,
                        };

                        if let Err(msg) = cmd_hub.send(cmd_msg, None) {
                            eprintln!("[Rust]   Warning: Failed to send command: {:?}", msg);
                        }
                    }

                    println!(
                        "[Rust]   Received pose: x={:.2}, y={:.2}, theta={:.2} | \
                         distance_moved={:.3}m | Sent cmd: lin={:.2}, ang={:.2}",
                        pose.x, pose.y, pose.theta, distance, linear_vel, angular_vel
                    );

                    last_pose = Some(pose);
                }
                Err(e) => {
                    eprintln!("[Rust]   Error deserializing pose: {:?}", e);
                }
            }
        } else if tick_count % 40 == 0 {
            // Print status every 2 seconds (40 ticks at 20Hz)
            println!("[Rust]   Waiting for pose data from Python node...");
        }

        // Sleep to maintain 20Hz rate (50ms)
        std::thread::sleep(Duration::from_millis(50));
    }
}
