use crate::process::ProcessRegistry;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub struct ProcessExecutor {
    registry: Arc<ProcessRegistry>,
}

impl ProcessExecutor {
    pub fn new(registry: Arc<ProcessRegistry>) -> Self {
        Self { registry }
    }

    /// Start background monitoring task
    pub fn start_monitoring(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(2));

            loop {
                interval.tick().await;
                self.registry.update_stats();
            }
        });
    }

    /// Start cleanup task for old deployments
    pub fn start_cleanup(registry: Arc<ProcessRegistry>) {
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(3600)); // Every hour

            loop {
                interval.tick().await;
                registry.cleanup_old(24); // Remove deployments older than 24 hours
            }
        });
    }
}
