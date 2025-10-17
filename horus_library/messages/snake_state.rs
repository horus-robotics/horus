use serde::{Deserialize, Serialize};

pub type Direction = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnakeState {
    pub direction: Direction,
}
