use std::path::{Path, PathBuf};

/// String sanitization utilities
pub struct StringUtils;

impl StringUtils {
    /// Sanitize string to be a valid identifier (letters, numbers, underscores)
    pub fn sanitize_identifier(s: &str) -> String {
        s.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Convert string to snake_case
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_is_lowercase = false;

        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 && prev_is_lowercase {
                    result.push('_');
                }
                result.push(c.to_lowercase().next().unwrap());
                prev_is_lowercase = false;
            } else {
                result.push(c);
                prev_is_lowercase = c.is_lowercase();
            }
        }

        result
    }

    /// Convert string to PascalCase
    pub fn to_pascal_case(s: &str) -> String {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                    }
                    None => String::new(),
                }
            })
            .collect()
    }

    /// Convert string to camelCase
    pub fn to_camel_case(s: &str) -> String {
        let pascal = Self::to_pascal_case(s);
        if pascal.is_empty() {
            return pascal;
        }

        let mut chars = pascal.chars();
        match chars.next() {
            Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    }

    /// Remove whitespace from string
    pub fn remove_whitespace(s: &str) -> String {
        s.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// Truncate string to max length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else if max_len <= 3 {
            s[..max_len].to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    /// Pad string to specified width
    pub fn pad_left(s: &str, width: usize, pad_char: char) -> String {
        if s.len() >= width {
            s.to_string()
        } else {
            format!("{}{}", pad_char.to_string().repeat(width - s.len()), s)
        }
    }

    /// Pad string to specified width (right)
    pub fn pad_right(s: &str, width: usize, pad_char: char) -> String {
        if s.len() >= width {
            s.to_string()
        } else {
            format!("{}{}", s, pad_char.to_string().repeat(width - s.len()))
        }
    }

    /// Check if string is a valid frame name (ROS-style)
    pub fn is_valid_frame_name(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Must start with letter or underscore
        if !s.chars().next().unwrap().is_alphabetic() && s.chars().next().unwrap() != '_' {
            return false;
        }

        // Must contain only alphanumeric, underscore, or slash
        s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '/')
    }

    /// Sanitize frame name
    pub fn sanitize_frame_name(s: &str) -> String {
        let mut result = String::new();

        for (i, c) in s.chars().enumerate() {
            if i == 0 {
                if c.is_alphabetic() || c == '_' {
                    result.push(c);
                } else {
                    result.push('_');
                }
            } else if c.is_alphanumeric() || c == '_' || c == '/' {
                result.push(c);
            } else {
                result.push('_');
            }
        }

        result
    }
}

/// Path manipulation utilities
pub struct PathUtils;

impl PathUtils {
    /// Get file extension (without dot)
    pub fn get_extension(path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_string())
    }

    /// Get filename without extension
    pub fn get_stem(path: &Path) -> Option<String> {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map(|s| s.to_string())
    }

    /// Replace extension
    pub fn with_extension(path: &Path, extension: &str) -> PathBuf {
        let mut new_path = path.to_path_buf();
        new_path.set_extension(extension);
        new_path
    }

    /// Check if path has specific extension
    pub fn has_extension(path: &Path, extension: &str) -> bool {
        match Self::get_extension(path) {
            Some(ext) => ext.eq_ignore_ascii_case(extension),
            None => false,
        }
    }

    /// Sanitize filename (remove invalid characters)
    pub fn sanitize_filename(filename: &str) -> String {
        filename
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Join paths safely
    pub fn join_safe(base: &Path, relative: &str) -> PathBuf {
        let mut result = base.to_path_buf();
        for component in relative.split('/') {
            if !component.is_empty() && component != "." && component != ".." {
                result.push(component);
            }
        }
        result
    }

    /// Convert package:// URI to absolute path
    pub fn resolve_package_uri(uri: &str, package_path: &Path) -> Option<PathBuf> {
        if let Some(rest) = uri.strip_prefix("package://") {
            let parts: Vec<&str> = rest.splitn(2, '/').collect();
            if parts.len() == 2 {
                let package_name = parts[0];
                let relative_path = parts[1];

                // Construct path: package_path/package_name/relative_path
                let mut result = package_path.to_path_buf();
                result.push(package_name);
                result.push(relative_path);
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Convert file:// URI to path
    pub fn resolve_file_uri(uri: &str) -> Option<PathBuf> {
        uri.strip_prefix("file://").map(|path| PathBuf::from(path))
    }

    /// Get relative path from base to target
    pub fn get_relative_path(base: &Path, target: &Path) -> Option<PathBuf> {
        pathdiff::diff_paths(target, base)
    }
}

/// Formatting utilities
pub struct FormatUtils;

impl FormatUtils {
    /// Format bytes as human-readable size
    pub fn format_bytes(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// Format duration in seconds as human-readable
    pub fn format_duration(seconds: f32) -> String {
        if seconds < 1.0 {
            format!("{:.0} ms", seconds * 1000.0)
        } else if seconds < 60.0 {
            format!("{:.2} s", seconds)
        } else if seconds < 3600.0 {
            let minutes = (seconds / 60.0).floor();
            let secs = seconds % 60.0;
            format!("{:.0}m {:.0}s", minutes, secs)
        } else {
            let hours = (seconds / 3600.0).floor();
            let minutes = ((seconds % 3600.0) / 60.0).floor();
            format!("{:.0}h {:.0}m", hours, minutes)
        }
    }

    /// Format number with thousands separator
    pub fn format_number(num: usize) -> String {
        let num_str = num.to_string();
        let mut result = String::new();

        for (i, c) in num_str.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.insert(0, ',');
            }
            result.insert(0, c);
        }

        result
    }

    /// Parse bool from string (flexible parsing)
    pub fn parse_bool(s: &str) -> Option<bool> {
        match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" | "enabled" => Some(true),
            "false" | "0" | "no" | "off" | "disabled" => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_identifier() {
        assert_eq!(
            StringUtils::sanitize_identifier("hello-world!"),
            "hello_world_"
        );
        assert_eq!(StringUtils::sanitize_identifier("test_123"), "test_123");
    }

    #[test]
    fn test_snake_case() {
        assert_eq!(StringUtils::to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(StringUtils::to_snake_case("testCase"), "test_case");
        assert_eq!(StringUtils::to_snake_case("already_snake"), "already_snake");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(StringUtils::to_pascal_case("hello_world"), "HelloWorld");
        assert_eq!(StringUtils::to_pascal_case("test-case"), "TestCase");
    }

    #[test]
    fn test_camel_case() {
        assert_eq!(StringUtils::to_camel_case("hello_world"), "helloWorld");
        assert_eq!(StringUtils::to_camel_case("test-case"), "testCase");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(StringUtils::truncate("hello", 10), "hello");
        assert_eq!(StringUtils::truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_pad() {
        assert_eq!(StringUtils::pad_left("test", 8, '0'), "0000test");
        assert_eq!(StringUtils::pad_right("test", 8, '0'), "test0000");
    }

    #[test]
    fn test_frame_name_validation() {
        assert!(StringUtils::is_valid_frame_name("base_link"));
        assert!(StringUtils::is_valid_frame_name("robot/camera"));
        assert!(!StringUtils::is_valid_frame_name("123invalid"));
        assert!(!StringUtils::is_valid_frame_name(""));
    }

    #[test]
    fn test_path_extension() {
        let path = Path::new("test.urdf");
        assert_eq!(PathUtils::get_extension(path), Some("urdf".to_string()));
        assert_eq!(PathUtils::get_stem(path), Some("test".to_string()));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(FormatUtils::format_bytes(512), "512.00 B");
        assert_eq!(FormatUtils::format_bytes(1024), "1.00 KB");
        assert_eq!(FormatUtils::format_bytes(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(FormatUtils::format_duration(0.5), "500 ms");
        assert_eq!(FormatUtils::format_duration(30.0), "30.00 s");
        assert_eq!(FormatUtils::format_duration(90.0), "1m 30s");
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(FormatUtils::parse_bool("true"), Some(true));
        assert_eq!(FormatUtils::parse_bool("1"), Some(true));
        assert_eq!(FormatUtils::parse_bool("false"), Some(false));
        assert_eq!(FormatUtils::parse_bool("invalid"), None);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            PathUtils::sanitize_filename("test file.txt"),
            "test_file.txt"
        );
        assert_eq!(
            PathUtils::sanitize_filename("my-robot.urdf"),
            "my-robot.urdf"
        );
    }
}
