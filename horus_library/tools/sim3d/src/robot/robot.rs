use bevy::prelude::*;

#[derive(Component)]
pub struct Robot {
    pub name: String,
}

impl Robot {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
