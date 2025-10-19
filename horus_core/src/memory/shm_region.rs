// HORUS Shared Memory Region - Using /dev/shm/horus for performance
use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

/// Shared memory region using /dev/shm/horus for performance
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
    pub fn new(name: &str, size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        // Create HORUS directory in /dev/shm if it doesn't exist
        let horus_shm_dir = PathBuf::from("/dev/shm/horus/topics");
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
    pub fn open(name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let horus_shm_dir = PathBuf::from("/dev/shm/horus/topics");
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
        // If we're the owner and file exists, optionally remove it
        // For now, we keep files persistent for debugging
        // If you want to clean up:
        // if self.owner && self.path.exists() {
        //     let _ = std::fs::remove_file(&self.path);
        // }
    }
}

// Thread safety
unsafe impl Send for ShmRegion {}
unsafe impl Sync for ShmRegion {}
