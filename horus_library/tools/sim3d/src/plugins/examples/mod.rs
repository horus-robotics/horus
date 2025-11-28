//! Example plugins demonstrating the plugin system

pub mod example_actuator;
pub mod example_sensor;
pub mod example_world;

// Re-export example plugin types for public API
pub use example_actuator::ExampleActuatorPlugin;
#[allow(unused_imports)]
pub use example_actuator::ThrusterActuator;
pub use example_sensor::ExampleSensorPlugin;
#[allow(unused_imports)]
pub use example_sensor::ProximitySensor;
pub use example_world::ExampleWorldPlugin;
#[allow(unused_imports)]
pub use example_world::{WindAffected, WindSystem};
