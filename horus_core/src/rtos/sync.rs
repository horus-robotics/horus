// RTOS synchronization primitives

use crate::error::HorusResult;
use std::ffi::c_void;
use std::time::Duration;

/// RTOS mutex for mutual exclusion
pub trait RTOSMutex: Send + Sync {
    /// Create a new mutex
    fn create() -> HorusResult<MutexHandle>;

    /// Lock the mutex
    fn lock(&self, timeout: Option<Duration>) -> HorusResult<()>;

    /// Try to lock without blocking
    fn try_lock(&self) -> bool;

    /// Unlock the mutex
    fn unlock(&self) -> HorusResult<()>;

    /// Delete the mutex
    fn delete(self) -> HorusResult<()>;

    /// Check if mutex is locked
    fn is_locked(&self) -> bool;

    /// Get owner task (if recursive mutex)
    fn get_owner(&self) -> Option<super::TaskHandle>;
}

/// Mutex handle
#[derive(Debug, Clone, Copy)]
pub struct MutexHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for MutexHandle {}
unsafe impl Sync for MutexHandle {}

/// RTOS semaphore for synchronization
pub trait RTOSSemaphore: Send + Sync {
    /// Create a counting semaphore
    fn create_counting(max_count: u32, initial_count: u32) -> HorusResult<SemaphoreHandle>;

    /// Create a binary semaphore
    fn create_binary() -> HorusResult<SemaphoreHandle>;

    /// Take/wait for semaphore
    fn take(&self, timeout: Option<Duration>) -> HorusResult<()>;

    /// Give/signal semaphore
    fn give(&self) -> HorusResult<()>;

    /// Try to take without blocking
    fn try_take(&self) -> bool;

    /// Get current count
    fn get_count(&self) -> u32;

    /// Delete the semaphore
    fn delete(self) -> HorusResult<()>;
}

/// Semaphore handle
#[derive(Debug, Clone, Copy)]
pub struct SemaphoreHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for SemaphoreHandle {}
unsafe impl Sync for SemaphoreHandle {}

/// RTOS condition variable
pub trait RTOSCondVar: Send + Sync {
    /// Create a condition variable
    fn create() -> HorusResult<CondVarHandle>;

    /// Wait on condition variable
    fn wait(&self, mutex: &MutexHandle, timeout: Option<Duration>) -> HorusResult<()>;

    /// Signal one waiting task
    fn signal(&self) -> HorusResult<()>;

    /// Broadcast to all waiting tasks
    fn broadcast(&self) -> HorusResult<()>;

    /// Delete the condition variable
    fn delete(self) -> HorusResult<()>;
}

/// Condition variable handle
#[derive(Debug, Clone, Copy)]
pub struct CondVarHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for CondVarHandle {}
unsafe impl Sync for CondVarHandle {}

/// Read-Write lock for RTOS
pub trait RTOSRWLock: Send + Sync {
    /// Create a read-write lock
    fn create() -> HorusResult<RWLockHandle>;

    /// Acquire read lock
    fn read_lock(&self, timeout: Option<Duration>) -> HorusResult<()>;

    /// Acquire write lock
    fn write_lock(&self, timeout: Option<Duration>) -> HorusResult<()>;

    /// Try to acquire read lock
    fn try_read_lock(&self) -> bool;

    /// Try to acquire write lock
    fn try_write_lock(&self) -> bool;

    /// Unlock (read or write)
    fn unlock(&self) -> HorusResult<()>;

    /// Delete the RW lock
    fn delete(self) -> HorusResult<()>;

    /// Get number of readers
    fn reader_count(&self) -> u32;

    /// Check if write locked
    fn is_write_locked(&self) -> bool;
}

/// Read-Write lock handle
#[derive(Debug, Clone, Copy)]
pub struct RWLockHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for RWLockHandle {}
unsafe impl Sync for RWLockHandle {}

/// Spinlock for short critical sections
pub trait RTOSSpinlock: Send + Sync {
    /// Create a spinlock
    fn create() -> HorusResult<SpinlockHandle>;

    /// Acquire spinlock (busy wait)
    fn lock(&self);

    /// Try to acquire without spinning
    fn try_lock(&self) -> bool;

    /// Release spinlock
    fn unlock(&self);

    /// Delete the spinlock
    fn delete(self) -> HorusResult<()>;
}

/// Spinlock handle
#[derive(Debug, Clone, Copy)]
pub struct SpinlockHandle {
    pub id: usize,
    pub platform_handle: *mut c_void,
}

unsafe impl Send for SpinlockHandle {}
unsafe impl Sync for SpinlockHandle {}

/// Recursive mutex that can be locked multiple times by same task
pub struct RecursiveMutex {
    handle: MutexHandle,
    owner: Option<super::TaskHandle>,
    count: u32,
}

impl RecursiveMutex {
    /// Create a new recursive mutex
    pub fn new() -> HorusResult<Self> {
        let handle = MutexHandle {
            id: 0,
            platform_handle: std::ptr::null_mut(),
        };

        Ok(Self {
            handle,
            owner: None,
            count: 0,
        })
    }

    /// Lock recursively
    pub fn lock(&mut self, timeout: Option<Duration>) -> HorusResult<()> {
        if let Some(rtos) = super::rtos() {
            let current = rtos.current_task();

            if let Some(owner) = self.owner {
                if owner.id == current.id {
                    // Same task, increment count
                    self.count += 1;
                    return Ok(());
                }
            }

            // Different task or no owner, acquire lock
            // Implementation would call RTOS-specific recursive mutex
            self.owner = Some(current);
            self.count = 1;
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }

    /// Unlock recursively
    pub fn unlock(&mut self) -> HorusResult<()> {
        if let Some(rtos) = super::rtos() {
            let current = rtos.current_task();

            if let Some(owner) = self.owner {
                if owner.id != current.id {
                    return Err(crate::error::HorusError::Internal(
                        "Mutex not owned by current task".to_string(),
                    ));
                }

                self.count -= 1;
                if self.count == 0 {
                    self.owner = None;
                    // Implementation would release RTOS mutex
                }
                Ok(())
            } else {
                Err(crate::error::HorusError::Internal(
                    "Mutex not locked".to_string(),
                ))
            }
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }
}

/// Priority inheritance mutex to prevent priority inversion
pub struct PriorityInheritanceMutex {
    handle: MutexHandle,
    owner: Option<super::TaskHandle>,
    original_priority: Option<super::TaskPriority>,
    waiting_tasks: Vec<(super::TaskHandle, super::TaskPriority)>,
}

impl PriorityInheritanceMutex {
    /// Create priority inheritance mutex
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            handle: MutexHandle {
                id: 0,
                platform_handle: std::ptr::null_mut(),
            },
            owner: None,
            original_priority: None,
            waiting_tasks: Vec::new(),
        })
    }

    /// Lock with priority inheritance
    pub fn lock(&mut self, timeout: Option<Duration>) -> HorusResult<()> {
        if let Some(rtos) = super::rtos_mut() {
            let current = rtos.current_task();
            let current_priority = rtos.get_task_priority(current)?;

            if let Some(owner) = self.owner {
                // Add to waiting list
                self.waiting_tasks.push((current, current_priority));

                // Boost owner's priority if necessary
                let owner_priority = rtos.get_task_priority(owner)?;
                if current_priority < owner_priority {
                    // Current task has higher priority, boost owner
                    rtos.set_task_priority(owner, current_priority)?;
                }

                // Wait for mutex
                // Implementation would block on RTOS mutex
            }

            // Acquired mutex
            self.owner = Some(current);
            self.original_priority = Some(current_priority);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }

    /// Unlock with priority restoration
    pub fn unlock(&mut self) -> HorusResult<()> {
        if let Some(rtos) = super::rtos_mut() {
            let current = rtos.current_task();

            if let Some(owner) = self.owner {
                if owner.id != current.id {
                    return Err(crate::error::HorusError::Internal(
                        "Mutex not owned by current task".to_string(),
                    ));
                }

                // Restore original priority
                if let Some(original) = self.original_priority {
                    rtos.set_task_priority(current, original)?;
                }

                // Find highest priority waiting task
                if !self.waiting_tasks.is_empty() {
                    self.waiting_tasks.sort_by_key(|(_, p)| *p);
                    let (next_owner, _) = self.waiting_tasks.remove(0);
                    self.owner = Some(next_owner);
                    // Wake up next owner
                    rtos.resume_task(next_owner)?;
                } else {
                    self.owner = None;
                }

                self.original_priority = None;
                Ok(())
            } else {
                Err(crate::error::HorusError::Internal(
                    "Mutex not locked".to_string(),
                ))
            }
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }
}

/// Reader-Writer lock optimized for multiple readers
pub struct ReaderWriterLock {
    handle: RWLockHandle,
    readers: Vec<super::TaskHandle>,
    writer: Option<super::TaskHandle>,
}

impl ReaderWriterLock {
    /// Create new RW lock
    pub fn new() -> HorusResult<Self> {
        Ok(Self {
            handle: RWLockHandle {
                id: 0,
                platform_handle: std::ptr::null_mut(),
            },
            readers: Vec::new(),
            writer: None,
        })
    }

    /// Acquire read lock
    pub fn read_lock(&mut self, timeout: Option<Duration>) -> HorusResult<()> {
        if let Some(rtos) = super::rtos() {
            let current = rtos.current_task();

            // Wait if writer is active
            while self.writer.is_some() {
                rtos.task_yield();
                // In real implementation, would block on condition
            }

            // Add to readers
            self.readers.push(current);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }

    /// Acquire write lock
    pub fn write_lock(&mut self, timeout: Option<Duration>) -> HorusResult<()> {
        if let Some(rtos) = super::rtos() {
            let current = rtos.current_task();

            // Wait for all readers and writer
            while !self.readers.is_empty() || self.writer.is_some() {
                rtos.task_yield();
                // In real implementation, would block on condition
            }

            // Acquire write lock
            self.writer = Some(current);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }

    /// Release lock
    pub fn unlock(&mut self) -> HorusResult<()> {
        if let Some(rtos) = super::rtos() {
            let current = rtos.current_task();

            // Check if writer
            if let Some(writer) = self.writer {
                if writer.id == current.id {
                    self.writer = None;
                    return Ok(());
                }
            }

            // Check if reader
            self.readers.retain(|&r| r.id != current.id);
            Ok(())
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }
}

/// Barrier for synchronizing multiple tasks
pub struct Barrier {
    count: u32,
    waiting: Vec<super::TaskHandle>,
    threshold: u32,
}

impl Barrier {
    /// Create barrier for n tasks
    pub fn new(n: u32) -> Self {
        Self {
            count: 0,
            waiting: Vec::new(),
            threshold: n,
        }
    }

    /// Wait at barrier
    pub fn wait(&mut self) -> HorusResult<()> {
        if let Some(rtos) = super::rtos_mut() {
            let current = rtos.current_task();

            self.count += 1;
            self.waiting.push(current);

            if self.count >= self.threshold {
                // All tasks arrived, release them
                for task in &self.waiting {
                    if task.id != current.id {
                        rtos.resume_task(*task)?;
                    }
                }

                // Reset for next use
                self.count = 0;
                self.waiting.clear();
                Ok(())
            } else {
                // Suspend until all arrive
                rtos.suspend_task(current)?;
                Ok(())
            }
        } else {
            Err(crate::error::HorusError::Internal(
                "No RTOS backend".to_string(),
            ))
        }
    }

    /// Reset barrier
    pub fn reset(&mut self) {
        self.count = 0;
        self.waiting.clear();
    }
}
