// HORUS Shared Memory Region - Cross-platform shared memory support
//
// Linux: /dev/shm/horus (tmpfs - RAM-backed, fastest)
// macOS: /tmp/horus (regular filesystem)
// Windows: %TEMP%\horus (system temp directory)

use crate::error::HorusResult;
use crate::memory::platform::{shm_global_dir, shm_session_topics_dir, shm_topics_dir};
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Cross-platform shared memory region for high-performance IPC
#[derive(Debug)]
pub struct ShmRegion {
    #[allow(dead_code)]
    mmap: MmapMut,
    size: usize,
    #[allow(dead_code)]
    path: PathBuf,
    _file: File,
    #[allow(dead_code)]
    name: String,
    owner: bool,
}

impl ShmRegion {
    /// Create or open a shared memory region in /dev/shm/horus
    pub fn new(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, false)
    }

    /// Create or open a global shared memory region (accessible across all sessions)
    pub fn new_global(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, true)
    }

    /// Internal function to create shared memory region with optional global flag
    fn new_internal(name: &str, size: usize, global: bool) -> HorusResult<Self> {
        // Create HORUS directory if it doesn't exist (platform-specific path)
        // Use session-isolated path if HORUS_SESSION_ID is set and not global
        let horus_shm_dir = if global {
            shm_global_dir()
        } else if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            shm_session_topics_dir(&session_id)
        } else {
            shm_topics_dir()
        };
        std::fs::create_dir_all(&horus_shm_dir)?;

        // Convert topic name to safe filename
        let safe_name = name.replace(['/', ':'], "_");
        let path = horus_shm_dir.join(format!("horus_{}", safe_name));

        // Check if file already exists
        let (file, is_owner) = if path.exists() {
            // Open existing file
            let file = OpenOptions::new().read(true).write(true).open(&path)?;

            // Check existing size
            let metadata = file.metadata()?;
            if metadata.len() < size as u64 {
                // Resize if needed
                file.set_len(size as u64)?;
            }

            (file, false)
        } else {
            // Create new file
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?;

            // Set initial size
            file.set_len(size as u64)?;
            (file, true)
        };

        // Memory map the file
        let mut mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };

        // Initialize to zero if we're the owner
        if is_owner {
            mmap.fill(0);
        }

        Ok(Self {
            mmap,
            size,
            path: path.clone(),
            _file: file,
            name: name.to_string(),
            owner: is_owner,
        })
    }

    /// Open existing shared memory region (no creation)
    pub fn open(name: &str) -> HorusResult<Self> {
        let horus_shm_dir = if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            shm_session_topics_dir(&session_id)
        } else {
            shm_topics_dir()
        };
        let safe_name = name.replace(['/', ':'], "_");
        let path = horus_shm_dir.join(format!("horus_{}", safe_name));

        if !path.exists() {
            return Err(format!("Shared memory '{}' does not exist", name).into());
        }

        let file = OpenOptions::new().read(true).write(true).open(&path)?;

        let metadata = file.metadata()?;
        let size = metadata.len() as usize;

        let mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };

        Ok(Self {
            mmap,
            size,
            path: path.clone(),
            _file: file,
            name: name.to_string(),
            owner: false,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_owner(&self) -> bool {
        self.owner
    }
}

impl Drop for ShmRegion {
    fn drop(&mut self) {
        // Clean up shared memory file if we're the owner
        if self.owner && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// Thread safety
unsafe impl Send for ShmRegion {}
unsafe impl Sync for ShmRegion {}
