//! Dynamic library plugin loader using libloading

use super::traits::Sim3dPlugin;
use libloading::{Library, Symbol};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Plugin loader for dynamic libraries
pub struct PluginLoader {
    /// Loaded libraries (path → library handle)
    libraries: HashMap<PathBuf, Library>,
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginLoader {
    /// Create a new plugin loader
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
        }
    }

    /// Load a plugin from a dynamic library
    ///
    /// # Safety
    ///
    /// Loading dynamic libraries is inherently unsafe. The library must:
    /// - Export a function named `create_plugin` with signature: `extern "C" fn() -> *mut dyn Sim3dPlugin`
    /// - Be compiled with the same Rust version and ABI
    /// - Not violate memory safety
    pub unsafe fn load_plugin(
        &mut self,
        library_path: &Path,
    ) -> Result<Box<dyn Sim3dPlugin>, String> {
        // Check if library exists
        if !library_path.exists() {
            return Err(format!("Library not found: {}", library_path.display()));
        }

        // Load the library
        let library =
            Library::new(library_path).map_err(|e| format!("Failed to load library: {}", e))?;

        // Get the create_plugin symbol
        let create_plugin: Symbol<extern "C" fn() -> *mut dyn Sim3dPlugin> = library
            .get(b"create_plugin\0")
            .map_err(|e| format!("Failed to find create_plugin symbol: {}", e))?;

        // Call create_plugin to instantiate the plugin
        let plugin_ptr = create_plugin();
        if plugin_ptr.is_null() {
            return Err("create_plugin returned null".to_string());
        }

        let plugin = Box::from_raw(plugin_ptr);

        // Store library handle to prevent unloading
        self.libraries.insert(library_path.to_path_buf(), library);

        tracing::info!("Loaded plugin from: {}", library_path.display());

        Ok(plugin)
    }

    /// Load multiple plugins from a directory
    pub unsafe fn load_plugins_from_directory(
        &mut self,
        dir: &Path,
    ) -> Result<Vec<Box<dyn Sim3dPlugin>>, String> {
        let mut plugins = Vec::new();

        // Get library extension for current platform
        let extension = Self::get_library_extension();

        // Iterate through directory
        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            // Check if file has correct extension
            if let Some(ext) = path.extension() {
                if ext == extension {
                    match self.load_plugin(&path) {
                        Ok(plugin) => plugins.push(plugin),
                        Err(e) => tracing::warn!("Failed to load plugin {}: {}", path.display(), e),
                    }
                }
            }
        }

        Ok(plugins)
    }

    /// Get the dynamic library extension for the current platform
    fn get_library_extension() -> &'static str {
        #[cfg(target_os = "linux")]
        return "so";

        #[cfg(target_os = "macos")]
        return "dylib";

        #[cfg(target_os = "windows")]
        return "dll";

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        compile_error!("Unsupported platform for plugin loading");
    }

    /// Unload all libraries
    pub fn unload_all(&mut self) {
        self.libraries.clear();
        tracing::info!("Unloaded all plugin libraries");
    }

    /// Get number of loaded libraries
    pub fn library_count(&self) -> usize {
        self.libraries.len()
    }
}

impl Drop for PluginLoader {
    fn drop(&mut self) {
        self.unload_all();
    }
}

/// Helper macro for defining a plugin export function
///
/// # Example
///
/// ```ignore
/// use sim3d::plugins::export_plugin;
///
/// pub struct MyPlugin {
///     // ...
/// }
///
/// impl Plugin for MyPlugin {
///     // ...
/// }
///
/// export_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut dyn $crate::plugins::Sim3dPlugin {
            let plugin: Box<dyn $crate::plugins::Sim3dPlugin> = Box::new(<$plugin_type>::default());
            Box::into_raw(plugin)
        }

        #[no_mangle]
        pub extern "C" fn destroy_plugin(ptr: *mut dyn $crate::plugins::Sim3dPlugin) {
            if !ptr.is_null() {
                unsafe {
                    let _ = Box::from_raw(ptr);
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_loader_creation() {
        let loader = PluginLoader::new();
        assert_eq!(loader.library_count(), 0);
    }

    #[test]
    fn test_get_library_extension() {
        let ext = PluginLoader::get_library_extension();

        #[cfg(target_os = "linux")]
        assert_eq!(ext, "so");

        #[cfg(target_os = "macos")]
        assert_eq!(ext, "dylib");

        #[cfg(target_os = "windows")]
        assert_eq!(ext, "dll");
    }

    #[test]
    fn test_load_nonexistent_library() {
        let mut loader = PluginLoader::new();
        let path = Path::new("/nonexistent/plugin.so");

        unsafe {
            let result = loader.load_plugin(path);
            assert!(result.is_err());
        }
    }
}
