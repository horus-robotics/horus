use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Resource)]
#[command(name = "sim3d")]
#[command(about = "HORUS 3D Robotics Simulator", long_about = None)]
pub struct Cli {
    #[arg(short, long, value_enum, default_value_t = Mode::Visual)]
    pub mode: Mode,

    #[arg(short, long)]
    pub robot: Option<PathBuf>,

    #[arg(short, long)]
    pub world: Option<PathBuf>,

    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub no_gui: bool,

    #[arg(long, default_value_t = false)]
    pub tf_viz: bool,

    #[arg(long, default_value_t = 1.0)]
    pub speed: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    Visual,
    Headless,
}

impl Cli {
    pub fn parse() -> Self {
        Parser::parse()
    }
}

use bevy::prelude::Resource;
