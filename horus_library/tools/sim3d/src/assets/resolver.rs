//! Path resolution for assets
//!
//! Handles various URI schemes and path resolution strategies:
//! - package:// (ROS package URIs)
//! - file:// (File URIs)
//! - model:// (Gazebo model URIs)
//! - Relative paths
//! - Absolute paths

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Path resolver for asset URIs
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Base paths to search for assets
    base_paths: Vec<PathBuf>,
    /// ROS package paths (from ROS_PACKAGE_PATH environment variable)
    ros_package_paths: Vec<PathBuf>,
}

impl PathResolver {
    /// Create a new path resolver with default search paths
    pub fn new() -> Self {
        let mut base_paths = vec![PathBuf::from("assets/models"), PathBuf::from("assets")];

        // Add current directory
        if let Ok(cwd) = std::env::current_dir() {
            base_paths.push(cwd);
        }

        // Parse ROS_PACKAGE_PATH if available
        let ros_package_paths = Self::parse_ros_package_path();

        Self {
            base_paths,
            ros_package_paths,
        }
    }

    /// Create with custom base paths
    pub fn with_base_paths(base_paths: Vec<PathBuf>) -> Self {
        let ros_package_paths = Self::parse_ros_package_path();
        Self {
            base_paths,
            ros_package_paths,
        }
    }

    /// Add a base path to the search list
    pub fn add_base_path(&mut self, path: PathBuf) {
        if !self.base_paths.contains(&path) {
            self.base_paths.push(path);
        }
    }

    /// Resolve a URI or path to an absolute path
    pub fn resolve(&self, uri: &str) -> Result<PathBuf> {
        // Handle package:// URIs (ROS convention)
        if let Some(stripped) = uri.strip_prefix("package://") {
            return self.resolve_ros_package(stripped);
        }

        // Handle file:// URIs
        if let Some(stripped) = uri.strip_prefix("file://") {
            let path = PathBuf::from(stripped);
            if path.exists() {
                return Ok(path);
            }
            anyhow::bail!("File does not exist: {}", stripped);
        }

        // Handle model:// URIs (Gazebo convention)
        if let Some(stripped) = uri.strip_prefix("model://") {
            return self.resolve_model(stripped);
        }

        // Try as absolute path
        let path = PathBuf::from(uri);
        if path.is_absolute() {
            if path.exists() {
                return Ok(path);
            }
            anyhow::bail!("Absolute path does not exist: {}", uri);
        }

        // Try relative to base paths
        self.search_base_paths(&path)
    }

    /// Resolve a ROS package:// URI
    fn resolve_ros_package(&self, package_path: &str) -> Result<PathBuf> {
        // Format: package://package_name/path/to/file
        let parts: Vec<&str> = package_path.splitn(2, '/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid package URI format: package://{}", package_path);
        }

        let package_name = parts[0];
        let file_path = parts[1];

        // Search in ROS package paths
        for base in &self.ros_package_paths {
            let candidate = base.join(package_name).join(file_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try common ROS installation paths
        let common_paths = [
            PathBuf::from("/opt/ros/humble/share"),
            PathBuf::from("/opt/ros/iron/share"),
            PathBuf::from("/opt/ros/jazzy/share"),
            PathBuf::from(shellexpand::tilde("~/ros2_ws/src").as_ref()),
            PathBuf::from(shellexpand::tilde("~/catkin_ws/src").as_ref()),
            PathBuf::from("assets/ros_packages"),
        ];

        for base in common_paths {
            let candidate = base.join(package_name).join(file_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        anyhow::bail!(
            "ROS package not found: {}\n  \
            File: {}\n  \
            Searched in {} locations including:\n  \
            - ROS_PACKAGE_PATH\n  \
            - /opt/ros/*/share\n  \
            - ~/ros2_ws/src\n  \
            - ~/catkin_ws/src\n  \
            - assets/ros_packages",
            package_name,
            file_path,
            self.ros_package_paths.len() + 5
        )
    }

    /// Resolve a model:// URI (Gazebo convention)
    fn resolve_model(&self, model_path: &str) -> Result<PathBuf> {
        // Try in assets/models first
        let model_dir = PathBuf::from("assets/models").join(model_path);
        if model_dir.exists() {
            return Ok(model_dir);
        }

        // Try in GAZEBO_MODEL_PATH
        if let Ok(gazebo_path) = std::env::var("GAZEBO_MODEL_PATH") {
            for base in gazebo_path.split(':') {
                let candidate = PathBuf::from(base).join(model_path);
                if candidate.exists() {
                    return Ok(candidate);
                }
            }
        }

        // Try common Gazebo model paths
        let common_paths = [
            PathBuf::from(shellexpand::tilde("~/.gazebo/models").as_ref()),
            PathBuf::from("/usr/share/gazebo/models"),
            PathBuf::from("/usr/local/share/gazebo/models"),
        ];

        for base in common_paths {
            let candidate = base.join(model_path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        anyhow::bail!("Gazebo model not found: {}", model_path)
    }

    /// Search for a file in base paths
    fn search_base_paths(&self, path: &Path) -> Result<PathBuf> {
        for base in &self.base_paths {
            let candidate = base.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        anyhow::bail!(
            "File not found: {}\n  \
            Searched in {} base paths:\n  {}",
            path.display(),
            self.base_paths.len(),
            self.base_paths
                .iter()
                .map(|p| format!("- {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    }

    /// Parse ROS_PACKAGE_PATH environment variable
    fn parse_ros_package_path() -> Vec<PathBuf> {
        if let Ok(ros_path) = std::env::var("ROS_PACKAGE_PATH") {
            ros_path
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .filter(|p| p.exists())
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for PathResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_absolute_path() {
        let resolver = PathResolver::new();
        // Use unique filename to avoid test collision when running in parallel
        let unique_name = format!("test_abs_path_{}.txt", std::process::id());
        let temp_file = std::env::temp_dir().join(&unique_name);
        std::fs::write(&temp_file, "test").unwrap();

        let resolved = resolver.resolve(temp_file.to_str().unwrap()).unwrap();
        assert_eq!(resolved, temp_file);

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_file_uri() {
        let resolver = PathResolver::new();
        // Use unique filename to avoid test collision when running in parallel
        let unique_name = format!("test_file_uri_{}.txt", std::process::id());
        let temp_file = std::env::temp_dir().join(&unique_name);
        std::fs::write(&temp_file, "test").unwrap();

        let uri = format!("file://{}", temp_file.display());
        let resolved = resolver.resolve(&uri).unwrap();
        assert_eq!(resolved, temp_file);

        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_nonexistent_file() {
        let resolver = PathResolver::new();
        let result = resolver.resolve("/nonexistent/file.obj");
        assert!(result.is_err());
    }
}
