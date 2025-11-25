use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::messages::*;
use crate::physics::diff_drive::CmdVel;

#[derive(Resource)]
pub struct HorusSubscriber {
    cmd_vel_subscriber: Arc<Mutex<HashMap<String, Twist>>>,
    enabled: bool,
}

impl Default for HorusSubscriber {
    fn default() -> Self {
        Self::new()
    }
}

impl HorusSubscriber {
    pub fn new() -> Self {
        Self {
            cmd_vel_subscriber: Arc::new(Mutex::new(HashMap::new())),
            enabled: true,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn receive_cmd_vel(&self, topic: impl Into<String>, twist: Twist) {
        if !self.enabled {
            return;
        }

        if let Ok(mut subscriber) = self.cmd_vel_subscriber.lock() {
            subscriber.insert(topic.into(), twist);
        }
    }

    pub fn get_cmd_vel(&self, topic: &str) -> Option<Twist> {
        if let Ok(subscriber) = self.cmd_vel_subscriber.lock() {
            subscriber.get(topic).cloned()
        } else {
            None
        }
    }

    pub fn get_all_cmd_vel(&self) -> HashMap<String, Twist> {
        if let Ok(subscriber) = self.cmd_vel_subscriber.lock() {
            subscriber.clone()
        } else {
            HashMap::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut subscriber) = self.cmd_vel_subscriber.lock() {
            subscriber.clear();
        }
    }
}

pub fn apply_cmd_vel_system(
    subscriber: Res<HorusSubscriber>,
    mut query: Query<(&Name, &mut CmdVel)>,
) {
    if !subscriber.is_enabled() {
        return;
    }

    for (name, mut cmd_vel) in query.iter_mut() {
        let topic = format!("{}.cmd_vel", name.as_str());
        if let Some(twist) = subscriber.get_cmd_vel(&topic) {
            cmd_vel.linear = twist.linear.x;
            cmd_vel.angular = twist.angular.z;
        }
    }
}

#[derive(Component)]
pub struct RobotCommandHandler {
    pub cmd_vel_topic: String,
}

impl RobotCommandHandler {
    pub fn new(robot_name: impl Into<String>) -> Self {
        Self {
            cmd_vel_topic: format!("{}.cmd_vel", robot_name.into()),
        }
    }
}

pub fn handle_robot_commands_system(
    subscriber: Res<HorusSubscriber>,
    mut query: Query<(&RobotCommandHandler, &mut CmdVel)>,
) {
    if !subscriber.is_enabled() {
        return;
    }

    for (handler, mut cmd_vel) in query.iter_mut() {
        if let Some(twist) = subscriber.get_cmd_vel(&handler.cmd_vel_topic) {
            cmd_vel.linear = twist.linear.x;
            cmd_vel.angular = twist.angular.z;
        }
    }
}
