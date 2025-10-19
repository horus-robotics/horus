use chrono::{DateTime, Utc};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid as NixPid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sysinfo::{Pid, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Stopped,
    Failed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub deployment_id: String,
    pub pid: u32,
    pub status: ProcessStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub cpu_usage: f32,
    pub memory_mb: u64,
    pub exit_code: Option<i32>,
    pub language: String,
    pub entrypoint: String,
}

pub struct ProcessRegistry {
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    system: Arc<Mutex<System>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }

    pub fn register(&self, deployment_id: String, pid: u32, language: String, entrypoint: String) {
        let info = ProcessInfo {
            deployment_id: deployment_id.clone(),
            pid,
            status: ProcessStatus::Running,
            start_time: Utc::now(),
            end_time: None,
            cpu_usage: 0.0,
            memory_mb: 0,
            exit_code: None,
            language,
            entrypoint,
        };

        self.processes.lock().unwrap().insert(deployment_id, info);
    }

    pub fn get(&self, deployment_id: &str) -> Option<ProcessInfo> {
        self.processes.lock().unwrap().get(deployment_id).cloned()
    }

    pub fn list(&self) -> Vec<ProcessInfo> {
        self.processes.lock().unwrap().values().cloned().collect()
    }

    pub fn update_stats(&self) {
        let mut system = self.system.lock().unwrap();
        system.refresh_all();

        let mut processes = self.processes.lock().unwrap();
        for (_, info) in processes.iter_mut() {
            if let ProcessStatus::Running = info.status {
                if let Some(proc) = system.process(Pid::from_u32(info.pid)) {
                    info.cpu_usage = proc.cpu_usage();
                    info.memory_mb = proc.memory() / 1024 / 1024;
                } else {
                    // Process no longer exists
                    info.status = ProcessStatus::Completed;
                    info.end_time = Some(Utc::now());
                }
            }
        }
    }

    pub fn stop(&self, deployment_id: &str) -> Result<(), String> {
        let mut processes = self.processes.lock().unwrap();

        if let Some(info) = processes.get_mut(deployment_id) {
            if let ProcessStatus::Running = info.status {
                let pid = NixPid::from_raw(info.pid as i32);

                // Send SIGTERM first
                if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
                    return Err(format!("Failed to send SIGTERM: {}", e));
                }

                // Wait a bit, then check if process is still alive
                std::thread::sleep(std::time::Duration::from_millis(100));

                // If still alive, send SIGKILL
                if signal::kill(pid, Signal::SIGTERM).is_ok() {
                    let _ = signal::kill(pid, Signal::SIGKILL);
                }

                info.status = ProcessStatus::Stopped;
                info.end_time = Some(Utc::now());
                Ok(())
            } else {
                Err("Process is not running".to_string())
            }
        } else {
            Err("Deployment not found".to_string())
        }
    }

    pub fn mark_failed(&self, deployment_id: &str, exit_code: i32) {
        let mut processes = self.processes.lock().unwrap();
        if let Some(info) = processes.get_mut(deployment_id) {
            info.status = ProcessStatus::Failed;
            info.exit_code = Some(exit_code);
            info.end_time = Some(Utc::now());
        }
    }

    pub fn cleanup_old(&self, max_age_hours: i64) {
        let mut processes = self.processes.lock().unwrap();
        let cutoff = Utc::now() - chrono::Duration::hours(max_age_hours);

        processes.retain(|_, info| match info.status {
            ProcessStatus::Running => true,
            _ => {
                if let Some(end_time) = info.end_time {
                    end_time > cutoff
                } else {
                    true
                }
            }
        });
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}
