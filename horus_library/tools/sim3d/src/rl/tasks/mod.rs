pub mod reaching;
pub mod balancing;
pub mod locomotion;
pub mod navigation;
pub mod manipulation;
pub mod push;

pub use reaching::ReachingTask;
pub use balancing::BalancingTask;
pub use locomotion::LocomotionTask;
pub use navigation::NavigationTask;
pub use manipulation::ManipulationTask;
pub use push::PushTask;
