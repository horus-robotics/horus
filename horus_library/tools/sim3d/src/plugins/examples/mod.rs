//! Example plugins demonstrating the plugin system

pub mod example_actuator;
pub mod example_sensor;
pub mod example_world;

pub use example_actuator::{ExampleActuatorPlugin, ThrusterActuator};
pub use example_sensor::{ExampleSensorPlugin, ProximitySensor};
pub use example_world::{ExampleWorldPlugin, WindAffected, WindSystem};
