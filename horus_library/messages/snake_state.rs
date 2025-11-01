use horus_core::core::LogSummary;
use serde::{Deserialize, Serialize};

pub type Direction = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeState {
    pub direction: Direction,
}


impl LogSummary for SnakeState {
    fn log_summary(&self) -> String {
        format!("{:?}", self)
    }
}
