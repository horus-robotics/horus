//! Enhanced error types with context and helpful suggestions
//!
//! Provides detailed error messages with file locations, hints, and suggestions
//! to help users quickly diagnose and fix issues.

use std::fmt;
use std::path::PathBuf;

/// Enhanced error with context and suggestions
#[derive(Debug, Clone)]
pub struct EnhancedError {
    /// Error message
    pub message: String,
    /// File where error occurred
    pub file: Option<PathBuf>,
    /// Line number
    pub line: Option<usize>,
    /// Column number
    pub column: Option<usize>,
    /// Helpful hint about the error
    pub hint: Option<String>,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Error category
    pub category: ErrorCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    FileNotFound,
    ParseError,
    ValidationError,
    MeshLoadError,
    URDFError,
    PhysicsError,
    ConfigError,
    Unknown,
}

impl EnhancedError {
    /// Create a new enhanced error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            file: None,
            line: None,
            column: None,
            hint: None,
            suggestion: None,
            category: ErrorCategory::Unknown,
        }
    }

    /// Set the file path
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the line number
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Add a helpful hint
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Add a suggested fix
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Set the error category
    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }

    /// Create a file not found error
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self::new(format!("File not found: {}", path.display()))
            .with_file(path.clone())
            .with_category(ErrorCategory::FileNotFound)
            .with_hint("Check that the file exists and the path is correct")
            .with_suggestion(format!(
                "Verify the file exists:\n  ls -la {}",
                path.parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| ".".to_string())
            ))
    }

    /// Create a mesh load error
    pub fn mesh_load_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        let path = path.into();
        let reason = reason.into();

        let hint = match path.extension().and_then(|e| e.to_str()) {
            Some("obj") => "OBJ files must be valid Wavefront format with proper vertex/face data",
            Some("stl") => "STL files must be valid binary or ASCII format",
            Some("dae") => "COLLADA files must be valid XML with proper geometry definitions",
            Some("gltf") | Some("glb") => "glTF files must follow the glTF 2.0 specification",
            _ => "Check that the mesh file is in a supported format (OBJ, STL, COLLADA, glTF)",
        };

        Self::new(format!("Failed to load mesh: {}", reason))
            .with_file(path)
            .with_category(ErrorCategory::MeshLoadError)
            .with_hint(hint)
            .with_suggestion(
                "Try opening the mesh file in a 3D viewer (Blender, MeshLab) to verify it's valid",
            )
    }

    /// Create a URDF parse error
    pub fn urdf_parse_failed(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        let path = path.into();
        Self::new(format!("Failed to parse URDF: {}", reason.into()))
            .with_file(path)
            .with_category(ErrorCategory::URDFError)
            .with_hint("URDF files must be valid XML conforming to the URDF specification")
            .with_suggestion(
                "Validate your URDF:\n  \
                 - Check XML syntax: xmllint --noout your_robot.urdf\n  \
                 - Verify URDF structure with: check_urdf your_robot.urdf",
            )
    }

    /// Create a validation error
    pub fn validation_failed(field: impl Into<String>, reason: impl Into<String>) -> Self {
        let field = field.into();
        let reason = reason.into();

        Self::new(format!("Validation failed for '{}': {}", field, reason))
            .with_category(ErrorCategory::ValidationError)
            .with_hint("Check the field value against the schema requirements")
    }

    /// Create a mesh reference error
    pub fn mesh_reference_not_found(
        urdf_path: impl Into<PathBuf>,
        mesh_ref: impl Into<String>,
        searched_paths: &[PathBuf],
    ) -> Self {
        let urdf_path = urdf_path.into();
        let mesh_ref = mesh_ref.into();

        let search_list = searched_paths
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        let suggestion = if mesh_ref.starts_with("package://") {
            format!(
                "For ROS package URIs:\n  \
                 1. Set ROS_PACKAGE_PATH environment variable\n  \
                 2. Or place assets in assets/ros_packages/\n  \
                 3. Or use a relative path instead\n\n\
                 Searched in:\n{}",
                search_list
            )
        } else {
            format!(
                "Place the mesh file in one of these locations:\n{}",
                search_list
            )
        };

        Self::new(format!(
            "Mesh file referenced in URDF not found: {}",
            mesh_ref
        ))
        .with_file(urdf_path)
        .with_category(ErrorCategory::MeshLoadError)
        .with_hint("URDF references a mesh file that doesn't exist in the search paths")
        .with_suggestion(suggestion)
    }

    /// Create a physics configuration error
    pub fn invalid_physics_value(
        field: impl Into<String>,
        value: f32,
        valid_range: (f32, f32),
    ) -> Self {
        let field = field.into();
        Self::new(format!(
            "Invalid value for '{}': {} (must be between {} and {})",
            field, value, valid_range.0, valid_range.1
        ))
        .with_category(ErrorCategory::PhysicsError)
        .with_hint(format!(
            "Physics parameter '{}' must be in range [{}, {}]",
            field, valid_range.0, valid_range.1
        ))
        .with_suggestion(format!(
            "Set '{}' to a value within the valid range, e.g., {}",
            field,
            (valid_range.0 + valid_range.1) / 2.0
        ))
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Error header with icon based on category
        let icon = match self.category {
            ErrorCategory::FileNotFound => "📁",
            ErrorCategory::ParseError => "📝",
            ErrorCategory::ValidationError => "✓",
            ErrorCategory::MeshLoadError => "🔷",
            ErrorCategory::URDFError => "🤖",
            ErrorCategory::PhysicsError => "⚙️",
            ErrorCategory::ConfigError => "⚙️",
            ErrorCategory::Unknown => "❌",
        };

        write!(f, "{} Error", icon)?;

        // File location
        if let Some(file) = &self.file {
            write!(f, " in '{}'", file.display())?;

            if let (Some(line), Some(col)) = (self.line, self.column) {
                write!(f, " at line {}, column {}", line, col)?;
            } else if let Some(line) = self.line {
                write!(f, " at line {}", line)?;
            }
        }

        writeln!(f, ":")?;

        // Main error message (indented)
        writeln!(f, "  {}", self.message)?;

        // Hint (if available)
        if let Some(hint) = &self.hint {
            writeln!(f)?;
            writeln!(f, "💡 Hint: {}", hint)?;
        }

        // Suggestion (if available)
        if let Some(suggestion) = &self.suggestion {
            writeln!(f)?;
            writeln!(f, "✏️  Suggestion:")?;
            for line in suggestion.lines() {
                writeln!(f, "   {}", line)?;
            }
        }

        Ok(())
    }
}

impl std::error::Error for EnhancedError {}

impl From<std::io::Error> for EnhancedError {
    fn from(err: std::io::Error) -> Self {
        use std::io::ErrorKind;

        match err.kind() {
            ErrorKind::NotFound => EnhancedError::new(format!("File not found: {}", err))
                .with_category(ErrorCategory::FileNotFound)
                .with_hint("Check that the file path is correct and the file exists"),
            ErrorKind::PermissionDenied => {
                EnhancedError::new(format!("Permission denied: {}", err))
                    .with_hint("Check file permissions and ensure you have read access")
                    .with_suggestion("Run: chmod +r <file>")
            }
            _ => EnhancedError::new(format!("I/O error: {}", err))
                .with_hint("An unexpected file system error occurred"),
        }
    }
}

impl From<serde_yaml::Error> for EnhancedError {
    fn from(err: serde_yaml::Error) -> Self {
        let location = err.location();

        let mut error = EnhancedError::new(format!("YAML parse error: {}", err))
            .with_category(ErrorCategory::ParseError)
            .with_hint("Check YAML syntax: proper indentation, no tabs, correct data types");

        if let Some(loc) = location {
            error = error.with_line(loc.line()).with_column(loc.column());
        }

        error.with_suggestion(
            "Common YAML issues:\n  \
             - Use spaces for indentation, not tabs\n  \
             - Ensure proper nesting with consistent indentation\n  \
             - Quote strings containing special characters\n  \
             - Use proper list syntax: `- item` or `[item1, item2]`",
        )
    }
}

impl From<serde_json::Error> for EnhancedError {
    fn from(err: serde_json::Error) -> Self {
        EnhancedError::new(format!("JSON parse error: {}", err))
            .with_category(ErrorCategory::ParseError)
            .with_line(err.line())
            .with_column(err.column())
            .with_hint("Check JSON syntax: matching braces, proper commas, quoted strings")
            .with_suggestion(
                "Validate JSON:\n  \
                 - Use a JSON validator: jq . your_file.json\n  \
                 - Check for trailing commas (not allowed in JSON)\n  \
                 - Ensure all strings are double-quoted",
            )
    }
}

impl From<&str> for EnhancedError {
    fn from(msg: &str) -> Self {
        EnhancedError::new(msg)
    }
}

impl From<String> for EnhancedError {
    fn from(msg: String) -> Self {
        EnhancedError::new(msg)
    }
}

impl From<anyhow::Error> for EnhancedError {
    fn from(err: anyhow::Error) -> Self {
        EnhancedError::new(err.to_string()).with_hint("An unexpected error occurred")
    }
}

/// Result type using EnhancedError
pub type Result<T> = std::result::Result<T, EnhancedError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_error_display() {
        let error = EnhancedError::new("Test error")
            .with_file("test.yaml")
            .with_line(42)
            .with_column(10)
            .with_hint("This is a test hint")
            .with_suggestion("Try this fix");

        let display = format!("{}", error);
        assert!(display.contains("test.yaml"));
        assert!(display.contains("line 42"));
        assert!(display.contains("Test error"));
        assert!(display.contains("Hint"));
        assert!(display.contains("Suggestion"));
    }

    #[test]
    fn test_file_not_found() {
        let error = EnhancedError::file_not_found("/path/to/missing.obj");
        let display = format!("{}", error);
        assert!(display.contains("File not found"));
        assert!(display.contains("missing.obj"));
    }

    #[test]
    fn test_mesh_load_failed() {
        let error = EnhancedError::mesh_load_failed("model.obj", "Invalid vertex data");
        let display = format!("{}", error);
        assert!(display.contains("Failed to load mesh"));
        assert!(display.contains("model.obj"));
        assert!(display.contains("Wavefront"));
    }

    #[test]
    fn test_validation_failed() {
        let error = EnhancedError::validation_failed("mass", "must be positive");
        let display = format!("{}", error);
        assert!(display.contains("Validation failed"));
        assert!(display.contains("mass"));
    }
}
