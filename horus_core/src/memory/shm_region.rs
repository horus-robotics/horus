// HORUS Shared Memory Region - Cross-platform optimized shared memory
//
// Each platform uses its optimal shared memory mechanism:
// - Linux: /dev/shm files (tmpfs - RAM-backed, already optimal)
// - macOS: shm_open() + mmap (POSIX shared memory, RAM-backed via Mach)
// - Windows: CreateFileMappingW with INVALID_HANDLE_VALUE (pagefile-backed, optimized for IPC)

use crate::error::HorusResult;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use crate::memory::platform::{shm_global_dir, shm_session_topics_dir, shm_topics_dir, write_session_pid};
#[cfg(target_os = "linux")]
use memmap2::{MmapMut, MmapOptions};
#[cfg(target_os = "linux")]
use std::fs::{File, OpenOptions};

/// Cross-platform shared memory region for high-performance IPC
///
/// Uses the optimal shared memory mechanism for each platform:
/// - Linux: tmpfs-backed files in /dev/shm (RAM)
/// - macOS: POSIX shm_open() (Mach shared memory, RAM)
/// - Windows: CreateFileMapping with page file backing (optimized IPC)
#[derive(Debug)]
pub struct ShmRegion {
    #[cfg(target_os = "linux")]
    mmap: MmapMut,
    #[cfg(target_os = "linux")]
    _file: File,
    #[cfg(target_os = "linux")]
    path: PathBuf,

    #[cfg(target_os = "macos")]
    ptr: *mut u8,
    #[cfg(target_os = "macos")]
    fd: i32,
    #[cfg(target_os = "macos")]
    shm_name: String,

    #[cfg(target_os = "windows")]
    ptr: *mut u8,
    #[cfg(target_os = "windows")]
    handle: isize,  // HANDLE

    size: usize,
    #[allow(dead_code)]
    name: String,
    owner: bool,
}

// ============================================================================
// Linux Implementation - File-based mmap on /dev/shm (tmpfs)
// Already optimal - tmpfs is RAM-backed with no disk I/O
// ============================================================================

#[cfg(target_os = "linux")]
impl ShmRegion {
    /// Create or open a shared memory region
    pub fn new(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, false)
    }

    /// Create or open a global shared memory region (accessible across all sessions)
    pub fn new_global(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, true)
    }

    fn new_internal(name: &str, size: usize, global: bool) -> HorusResult<Self> {
        let horus_shm_dir = if global {
            shm_global_dir()
        } else if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            let _ = write_session_pid(&session_id);
            shm_session_topics_dir(&session_id)
        } else {
            shm_topics_dir()
        };
        std::fs::create_dir_all(&horus_shm_dir)?;

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let path = horus_shm_dir.join(format!("horus_{}", name));

        let (file, is_owner) = if path.exists() {
            let file = OpenOptions::new().read(true).write(true).open(&path)?;
            let metadata = file.metadata()?;
            if metadata.len() < size as u64 {
                file.set_len(size as u64)?;
            }
            (file, false)
        } else {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?;
            file.set_len(size as u64)?;
            (file, true)
        };

        let mut mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };

        if is_owner {
            mmap.fill(0);
        }

        Ok(Self {
            mmap,
            size,
            path,
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
        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let path = horus_shm_dir.join(format!("horus_{}", name));

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
            path,
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

#[cfg(target_os = "linux")]
impl Drop for ShmRegion {
    fn drop(&mut self) {
        if self.owner && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

// ============================================================================
// macOS Implementation - POSIX shm_open() (Mach shared memory, RAM-backed)
// Much faster than /tmp file-based approach
// ============================================================================

#[cfg(target_os = "macos")]
impl ShmRegion {
    /// Create or open a shared memory region using shm_open (RAM-backed)
    pub fn new(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, false)
    }

    /// Create or open a global shared memory region
    pub fn new_global(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, true)
    }

    fn new_internal(name: &str, size: usize, global: bool) -> HorusResult<Self> {
        use std::ffi::CString;

        // Create shm name: /horus_<session>_<name> or /horus_global_<name>
        let session_prefix = if global {
            "global".to_string()
        } else if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            session_id
        } else {
            "default".to_string()
        };

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let shm_name = format!("/horus_{}_{}", session_prefix, name);
        let c_name = CString::new(shm_name.clone())
            .map_err(|e| format!("Invalid shm name: {}", e))?;

        // Try to open existing first
        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_RDWR,
                0o666,
            )
        };

        let (fd, is_owner) = if fd >= 0 {
            // Opened existing
            (fd, false)
        } else {
            // Create new
            let fd = unsafe {
                libc::shm_open(
                    c_name.as_ptr(),
                    libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                    0o666,
                )
            };
            if fd < 0 {
                // Race condition: someone else created it, try opening again
                let fd = unsafe {
                    libc::shm_open(
                        c_name.as_ptr(),
                        libc::O_RDWR,
                        0o666,
                    )
                };
                if fd < 0 {
                    return Err(format!(
                        "Failed to open/create shm '{}': {}",
                        shm_name,
                        std::io::Error::last_os_error()
                    ).into());
                }
                (fd, false)
            } else {
                // Set size for new region
                if unsafe { libc::ftruncate(fd, size as libc::off_t) } != 0 {
                    unsafe { libc::close(fd) };
                    unsafe { libc::shm_unlink(c_name.as_ptr()) };
                    return Err(format!(
                        "Failed to set shm size: {}",
                        std::io::Error::last_os_error()
                    ).into());
                }
                (fd, true)
            }
        };

        // Memory map the shared memory
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            if is_owner {
                unsafe { libc::shm_unlink(c_name.as_ptr()) };
            }
            return Err(format!(
                "Failed to mmap shm: {}",
                std::io::Error::last_os_error()
            ).into());
        }

        // Initialize to zero if owner
        if is_owner {
            unsafe {
                std::ptr::write_bytes(ptr as *mut u8, 0, size);
            }
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            fd,
            shm_name,
            size,
            name: name.to_string(),
            owner: is_owner,
        })
    }

    /// Open existing shared memory region
    pub fn open(name: &str) -> HorusResult<Self> {
        use std::ffi::CString;

        let session_prefix = if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            session_id
        } else {
            "default".to_string()
        };

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let shm_name = format!("/horus_{}_{}", session_prefix, name);
        let c_name = CString::new(shm_name.clone())
            .map_err(|e| format!("Invalid shm name: {}", e))?;

        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_RDWR,
                0o666,
            )
        };

        if fd < 0 {
            return Err(format!("Shared memory '{}' does not exist", name).into());
        }

        // Get size
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut stat) } != 0 {
            unsafe { libc::close(fd) };
            return Err(format!(
                "Failed to stat shm: {}",
                std::io::Error::last_os_error()
            ).into());
        }
        let size = stat.st_size as usize;

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(format!(
                "Failed to mmap shm: {}",
                std::io::Error::last_os_error()
            ).into());
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            fd,
            shm_name,
            size,
            name: name.to_string(),
            owner: false,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_owner(&self) -> bool {
        self.owner
    }
}

#[cfg(target_os = "macos")]
impl Drop for ShmRegion {
    fn drop(&mut self) {
        // Unmap memory
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
            libc::close(self.fd);
        }

        // Unlink if owner
        if self.owner {
            if let Ok(c_name) = std::ffi::CString::new(self.shm_name.clone()) {
                unsafe { libc::shm_unlink(c_name.as_ptr()) };
            }
        }
    }
}

// ============================================================================
// Windows Implementation - CreateFileMappingW with pagefile backing
// Uses INVALID_HANDLE_VALUE for pure shared memory (no temp files)
// ============================================================================

#[cfg(target_os = "windows")]
impl ShmRegion {
    /// Create or open a shared memory region using Windows API (pagefile-backed)
    pub fn new(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, false)
    }

    /// Create or open a global shared memory region
    pub fn new_global(name: &str, size: usize) -> HorusResult<Self> {
        Self::new_internal(name, size, true)
    }

    fn new_internal(name: &str, size: usize, global: bool) -> HorusResult<Self> {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, INVALID_HANDLE_VALUE, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Memory::{
            CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
        };

        // Create mapping name: Global\horus_<session>_<name> or Local\horus_<name>
        let session_prefix = if global {
            "Global\\horus_global".to_string()
        } else if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            format!("Local\\horus_{}", session_id)
        } else {
            "Local\\horus_default".to_string()
        };

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let mapping_name = format!("{}_{}", session_prefix, name);

        // Convert to wide string
        let wide_name: Vec<u16> = mapping_name.encode_utf16().chain(std::iter::once(0)).collect();

        // Create or open file mapping (INVALID_HANDLE_VALUE = pagefile-backed)
        let handle = unsafe {
            CreateFileMappingW(
                INVALID_HANDLE_VALUE as isize,
                std::ptr::null(),
                PAGE_READWRITE,
                (size >> 32) as u32,  // High DWORD
                size as u32,          // Low DWORD
                wide_name.as_ptr(),
            )
        };

        if handle == 0 {
            return Err(format!(
                "CreateFileMappingW failed: error {}",
                unsafe { GetLastError() }
            ).into());
        }

        let is_owner = unsafe { GetLastError() } != ERROR_ALREADY_EXISTS;

        // Map view of file
        let ptr = unsafe {
            MapViewOfFile(
                handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                size,
            )
        };

        if ptr.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(format!(
                "MapViewOfFile failed: error {}",
                unsafe { GetLastError() }
            ).into());
        }

        // Initialize to zero if owner
        if is_owner {
            unsafe {
                std::ptr::write_bytes(ptr as *mut u8, 0, size);
            }
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            handle,
            size,
            name: name.to_string(),
            owner: is_owner,
        })
    }

    /// Open existing shared memory region
    pub fn open(name: &str) -> HorusResult<Self> {
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
        use windows_sys::Win32::System::Memory::{
            OpenFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS,
        };

        let session_prefix = if let Ok(session_id) = std::env::var("HORUS_SESSION_ID") {
            format!("Local\\horus_{}", session_id)
        } else {
            "Local\\horus_default".to_string()
        };

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let mapping_name = format!("{}_{}", session_prefix, name);
        let wide_name: Vec<u16> = mapping_name.encode_utf16().chain(std::iter::once(0)).collect();

        let handle = unsafe {
            OpenFileMappingW(
                FILE_MAP_ALL_ACCESS,
                0,  // bInheritHandle = FALSE
                wide_name.as_ptr(),
            )
        };

        if handle == 0 {
            return Err(format!("Shared memory '{}' does not exist", name).into());
        }

        // Map view - we don't know size, map entire region
        let ptr = unsafe {
            MapViewOfFile(
                handle,
                FILE_MAP_ALL_ACCESS,
                0,
                0,
                0,  // Map entire file
            )
        };

        if ptr.is_null() {
            unsafe { CloseHandle(handle) };
            return Err(format!(
                "MapViewOfFile failed: error {}",
                unsafe { GetLastError() }
            ).into());
        }

        // Note: We can't easily get the size of an existing mapping on Windows
        // without additional tracking. For now, use a reasonable default.
        // In practice, the caller should know the expected size.
        let size = 0; // Unknown - caller should track

        Ok(Self {
            ptr: ptr as *mut u8,
            handle,
            size,
            name: name.to_string(),
            owner: false,
        })
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_owner(&self) -> bool {
        self.owner
    }
}

#[cfg(target_os = "windows")]
impl Drop for ShmRegion {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::Memory::UnmapViewOfFile;

        unsafe {
            UnmapViewOfFile(self.ptr as *const std::ffi::c_void);
            CloseHandle(self.handle);
        }
        // Note: Windows automatically cleans up named file mappings when all handles are closed
    }
}

// Thread safety - shared memory regions can be sent between threads
unsafe impl Send for ShmRegion {}
unsafe impl Sync for ShmRegion {}

// ============================================================================
// Fallback for other platforms (BSD, etc.) - Use file-based approach
// ============================================================================

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::memory::platform::{shm_global_dir, shm_session_topics_dir, shm_topics_dir, write_session_pid};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use memmap2::{MmapMut, MmapOptions};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use std::fs::{File, OpenOptions};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl ShmRegion {
    pub fn new(name: &str, size: usize) -> HorusResult<Self> {
        // Fallback to /tmp file-based approach
        let horus_shm_dir = PathBuf::from("/tmp/horus/topics");
        std::fs::create_dir_all(&horus_shm_dir)?;

        // Topic names use dot notation (e.g., "motors.cmd_vel") - no conversion needed
        let path = horus_shm_dir.join(format!("horus_{}", name));

        let (file, is_owner) = if path.exists() {
            let file = OpenOptions::new().read(true).write(true).open(&path)?;
            (file, false)
        } else {
            let file = OpenOptions::new()
                .read(true).write(true).create(true).truncate(true)
                .open(&path)?;
            file.set_len(size as u64)?;
            (file, true)
        };

        let mut mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };
        if is_owner { mmap.fill(0); }

        Ok(Self { mmap, size, path, _file: file, name: name.to_string(), owner: is_owner })
    }

    pub fn new_global(name: &str, size: usize) -> HorusResult<Self> {
        Self::new(name, size)
    }

    pub fn open(name: &str) -> HorusResult<Self> {
        // Topic names use dot notation - no conversion needed
        let path = PathBuf::from("/tmp/horus/topics").join(format!("horus_{}", name));
        if !path.exists() {
            return Err(format!("Shared memory '{}' does not exist", name).into());
        }
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let size = file.metadata()?.len() as usize;
        let mmap = unsafe { MmapOptions::new().len(size).map_mut(&file)? };
        Ok(Self { mmap, size, path, _file: file, name: name.to_string(), owner: false })
    }

    pub fn as_ptr(&self) -> *const u8 { self.mmap.as_ptr() }
    pub fn as_mut_ptr(&mut self) -> *mut u8 { self.mmap.as_mut_ptr() }
    pub fn size(&self) -> usize { self.size }
    pub fn is_owner(&self) -> bool { self.owner }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
impl Drop for ShmRegion {
    fn drop(&mut self) {
        if self.owner && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
