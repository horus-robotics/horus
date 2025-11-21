//! CLI validation tool for scene files and URDFs
//!
//! Provides command-line tools to validate scene files, URDF files,
//! and check mesh references before running the simulator.

use crate::error::{EnhancedError, Result};
use crate::scene::{loader::SceneDefinition, validation::SceneValidator};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Validation type enum (for horus_manager CLI)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationType {
    Scene,
    Urdf,
    Auto,
}

/// Output format enum (for horus_manager CLI)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Html,
}

/// Validation result for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub file: PathBuf,
    pub file_type: String,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub mesh_references: Vec<MeshReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshReference {
    pub path: String,
    pub exists: bool,
    pub resolved_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidationReport {
    pub total_files: usize,
    pub valid_files: usize,
    pub invalid_files: usize,
    pub results: Vec<ValidationResult>,
}

/// Validate a single file
pub fn validate_file(
    path: &Path,
    validation_type: Option<ValidationType>,
    check_meshes: bool,
) -> Result<ValidationResult> {
    // Auto-detect file type if not specified
    let file_type = match validation_type {
        Some(ValidationType::Scene) => "scene",
        Some(ValidationType::Urdf) => "urdf",
        Some(ValidationType::Auto) | None => detect_file_type(path)?,
    };

    match file_type {
        "scene" => validate_scene_file(path, check_meshes),
        "urdf" => validate_urdf_file(path, check_meshes),
        _ => Err(
            EnhancedError::new(format!("Unknown file type: {}", file_type))
                .with_file(path)
                .with_hint("Supported types: scene (.yaml, .json), urdf (.urdf)"),
        ),
    }
}

/// Detect file type from extension
fn detect_file_type(path: &Path) -> Result<&'static str> {
    let extension = path.extension().and_then(|e| e.to_str()).ok_or_else(|| {
        EnhancedError::new("Could not determine file type")
            .with_file(path)
            .with_hint("File must have a recognizable extension (.yaml, .yml, .json, .urdf)")
    })?;

    match extension {
        "yaml" | "yml" | "json" => Ok("scene"),
        "urdf" => Ok("urdf"),
        _ => Err(
            EnhancedError::new(format!("Unsupported file extension: {}", extension))
                .with_file(path)
                .with_hint(
                    "Supported extensions: .yaml, .yml, .json (for scenes), .urdf (for robots)",
                ),
        ),
    }
}

/// Validate a scene file
fn validate_scene_file(path: &Path, check_meshes: bool) -> Result<ValidationResult> {
    let mut result = ValidationResult {
        file: path.to_path_buf(),
        file_type: "scene".to_string(),
        valid: true,
        errors: Vec::new(),
        warnings: Vec::new(),
        mesh_references: Vec::new(),
    };

    // Read file content
    let content = std::fs::read_to_string(path).map_err(|e| {
        result.valid = false;
        result.errors.push(format!("Failed to read file: {}", e));
        EnhancedError::file_not_found(path)
    })?;

    // Validate with schema
    let validator = SceneValidator::new().map_err(|e| {
        result.valid = false;
        result
            .errors
            .push(format!("Failed to create validator: {}", e));
        EnhancedError::new(format!("Validator error: {}", e))
    })?;

    let report = validator.validate_yaml(&content).map_err(|e| {
        result.valid = false;
        result.errors.push(format!("Validation error: {}", e));
        EnhancedError::new(format!("Validation failed: {}", e))
    })?;

    if !report.valid {
        result.valid = false;
        result
            .errors
            .extend(report.errors.iter().map(|e| e.to_string()));
    }

    // Try to parse the scene to check for additional issues
    match SceneDefinition::from_yaml_str(&content) {
        Ok(scene) => {
            // Check mesh references if requested
            if check_meshes {
                check_scene_mesh_references(&scene, path, &mut result);
            }

            // Additional warnings
            if scene.robots.is_empty() && scene.objects.is_empty() {
                result
                    .warnings
                    .push("Scene contains no robots or objects".to_string());
            }
        }
        Err(e) => {
            result.valid = false;
            result.errors.push(format!("Parse error: {}", e));
        }
    }

    Ok(result)
}

/// Validate a URDF file
fn validate_urdf_file(path: &Path, check_meshes: bool) -> Result<ValidationResult> {
    let mut result = ValidationResult {
        file: path.to_path_buf(),
        file_type: "urdf".to_string(),
        valid: true,
        errors: Vec::new(),
        warnings: Vec::new(),
        mesh_references: Vec::new(),
    };

    // Read URDF content
    let content = std::fs::read_to_string(path).map_err(|e| {
        result.valid = false;
        result.errors.push(format!("Failed to read file: {}", e));
        EnhancedError::file_not_found(path)
    })?;

    // Parse URDF
    match urdf_rs::read_from_string(&content) {
        Ok(robot) => {
            // Check for common issues
            if robot.links.is_empty() {
                result.warnings.push("URDF contains no links".to_string());
            }

            if robot.joints.is_empty() {
                result
                    .warnings
                    .push("URDF contains no joints (single rigid body)".to_string());
            }

            // Check mesh references if requested
            if check_meshes {
                check_urdf_mesh_references(&robot, path, &mut result);
            }

            // Check for required base link
            if robot.name.is_empty() {
                result.warnings.push("Robot name is empty".to_string());
            }
        }
        Err(e) => {
            result.valid = false;
            result.errors.push(format!("URDF parse error: {}", e));
        }
    }

    Ok(result)
}

/// Check mesh references in a scene
fn check_scene_mesh_references(
    scene: &SceneDefinition,
    scene_path: &Path,
    result: &mut ValidationResult,
) {
    let scene_dir = scene_path.parent().unwrap_or(Path::new("."));

    for robot in &scene.robots {
        let urdf_path = if Path::new(&robot.urdf_path).is_absolute() {
            PathBuf::from(&robot.urdf_path)
        } else {
            scene_dir.join(&robot.urdf_path)
        };

        let mesh_ref = MeshReference {
            path: robot.urdf_path.clone(),
            exists: urdf_path.exists(),
            resolved_path: Some(urdf_path.clone()),
        };

        if !mesh_ref.exists {
            result.warnings.push(format!(
                "URDF file not found: {} (resolved to: {})",
                robot.urdf_path,
                urdf_path.display()
            ));
        }

        result.mesh_references.push(mesh_ref);
    }
}

/// Check mesh references in a URDF
fn check_urdf_mesh_references(
    robot: &urdf_rs::Robot,
    urdf_path: &Path,
    result: &mut ValidationResult,
) {
    let urdf_dir = urdf_path.parent().unwrap_or(Path::new("."));

    for link in &robot.links {
        // Check visual meshes
        for visual in &link.visual {
            if let urdf_rs::Geometry::Mesh { filename, .. } = &visual.geometry {
                check_mesh_reference(filename, urdf_dir, result);
            }
        }

        // Check collision meshes
        for collision in &link.collision {
            if let urdf_rs::Geometry::Mesh { filename, .. } = &collision.geometry {
                check_mesh_reference(filename, urdf_dir, result);
            }
        }
    }
}

/// Check a single mesh reference
fn check_mesh_reference(filename: &str, base_dir: &Path, result: &mut ValidationResult) {
    // Try to resolve the mesh path
    let resolved = resolve_mesh_path(filename, base_dir);

    let mesh_ref = MeshReference {
        path: filename.to_string(),
        exists: resolved.as_ref().map(|p| p.exists()).unwrap_or(false),
        resolved_path: resolved.clone(),
    };

    if !mesh_ref.exists {
        let msg = if let Some(path) = &resolved {
            format!(
                "Mesh file not found: {} (resolved to: {})",
                filename,
                path.display()
            )
        } else {
            format!("Mesh file could not be resolved: {}", filename)
        };
        result.warnings.push(msg);
    }

    result.mesh_references.push(mesh_ref);
}

/// Resolve mesh path from URDF reference
fn resolve_mesh_path(filename: &str, base_dir: &Path) -> Option<PathBuf> {
    // Handle package:// URIs
    if let Some(stripped) = filename.strip_prefix("package://") {
        // Try common ROS package locations
        let search_paths = vec![
            PathBuf::from("/opt/ros/humble/share"),
            PathBuf::from("/opt/ros/iron/share"),
            PathBuf::from("assets/ros_packages"),
        ];

        let parts: Vec<&str> = stripped.splitn(2, '/').collect();
        if parts.len() == 2 {
            let package_name = parts[0];
            let file_path = parts[1];

            for base in search_paths {
                let candidate = base.join(package_name).join(file_path);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        return None;
    }

    // Handle file:// URIs
    if let Some(stripped) = filename.strip_prefix("file://") {
        return Some(PathBuf::from(stripped));
    }

    // Try relative to URDF directory
    let relative_path = base_dir.join(filename);
    if relative_path.exists() {
        return Some(relative_path);
    }

    // Try absolute path
    let abs_path = PathBuf::from(filename);
    if abs_path.exists() {
        return Some(abs_path);
    }

    None
}

/// Format validation result as text
pub fn format_text(result: &ValidationResult, verbose: bool) -> String {
    let mut output = String::new();

    let status_icon = if result.valid { "✓" } else { "✗" };
    output.push_str(&format!(
        "{} {} ({})\n",
        status_icon,
        result.file.display(),
        result.file_type
    ));

    if !result.errors.is_empty() {
        output.push_str("\nErrors:\n");
        for error in &result.errors {
            output.push_str(&format!("  ❌ {}\n", error));
        }
    }

    if !result.warnings.is_empty() {
        output.push_str("\nWarnings:\n");
        for warning in &result.warnings {
            output.push_str(&format!("  ⚠️  {}\n", warning));
        }
    }

    if verbose && !result.mesh_references.is_empty() {
        output.push_str("\nMesh References:\n");
        for mesh_ref in &result.mesh_references {
            let status = if mesh_ref.exists { "✓" } else { "✗" };
            output.push_str(&format!("  {} {}\n", status, mesh_ref.path));
            if let Some(resolved) = &mesh_ref.resolved_path {
                output.push_str(&format!("      → {}\n", resolved.display()));
            }
        }
    }

    output
}

/// Format validation result as JSON
pub fn format_json(result: &ValidationResult) -> Result<String> {
    serde_json::to_string_pretty(result)
        .map_err(|e| EnhancedError::new(format!("Failed to serialize to JSON: {}", e)))
}

/// Format validation result as HTML
pub fn format_html(result: &ValidationResult) -> String {
    let status_class = if result.valid { "valid" } else { "invalid" };
    let status_text = if result.valid { "VALID" } else { "INVALID" };

    let mut html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Validation Report: {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: {}; color: white; padding: 10px; border-radius: 5px; }}
        .section {{ margin: 20px 0; }}
        .error {{ color: #d32f2f; }}
        .warning {{ color: #f57c00; }}
        .success {{ color: #388e3c; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Validation Report</h1>
        <p>File: {}</p>
        <p>Type: {}</p>
        <p>Status: <strong>{}</strong></p>
    </div>
"#,
        result.file.display(),
        if result.valid { "#4caf50" } else { "#f44336" },
        result.file.display(),
        result.file_type,
        status_text
    );

    if !result.errors.is_empty() {
        html.push_str(
            r#"    <div class="section">
        <h2>Errors</h2>
        <ul>
"#,
        );
        for error in &result.errors {
            html.push_str(&format!(
                r#"            <li class="error">{}</li>
"#,
                error
            ));
        }
        html.push_str(
            r#"        </ul>
    </div>
"#,
        );
    }

    if !result.warnings.is_empty() {
        html.push_str(
            r#"    <div class="section">
        <h2>Warnings</h2>
        <ul>
"#,
        );
        for warning in &result.warnings {
            html.push_str(&format!(
                r#"            <li class="warning">{}</li>
"#,
                warning
            ));
        }
        html.push_str(
            r#"        </ul>
    </div>
"#,
        );
    }

    if !result.mesh_references.is_empty() {
        html.push_str(
            r#"    <div class="section">
        <h2>Mesh References</h2>
        <table>
            <tr>
                <th>Status</th>
                <th>Path</th>
                <th>Resolved Path</th>
            </tr>
"#,
        );
        for mesh_ref in &result.mesh_references {
            let status = if mesh_ref.exists { "✓" } else { "✗" };
            let status_class = if mesh_ref.exists { "success" } else { "error" };
            let resolved = mesh_ref
                .resolved_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Not resolved".to_string());

            html.push_str(&format!(
                r#"            <tr>
                <td class="{}">{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>
"#,
                status_class, status, mesh_ref.path, resolved
            ));
        }
        html.push_str(
            r#"        </table>
    </div>
"#,
        );
    }

    html.push_str(
        r#"</body>
</html>
"#,
    );

    html
}

/// Format batch validation report
pub fn format_batch_report(report: &BatchValidationReport, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Text => {
            let mut output = format!(
                "Batch Validation Report\n\
                 =======================\n\
                 Total files: {}\n\
                 Valid: {}\n\
                 Invalid: {}\n\n",
                report.total_files, report.valid_files, report.invalid_files
            );

            for result in &report.results {
                output.push_str(&format_text(result, false));
                output.push('\n');
            }

            Ok(output)
        }
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .map_err(|e| EnhancedError::new(format!("Failed to serialize to JSON: {}", e))),
        OutputFormat::Html => {
            let mut html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>Batch Validation Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .summary {{ background: #2196f3; color: white; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
        .file-result {{ margin: 15px 0; padding: 10px; border: 1px solid #ddd; border-radius: 5px; }}
        .valid {{ border-left: 5px solid #4caf50; }}
        .invalid {{ border-left: 5px solid #f44336; }}
    </style>
</head>
<body>
    <div class="summary">
        <h1>Batch Validation Report</h1>
        <p>Total files: {}</p>
        <p>Valid: {} | Invalid: {}</p>
    </div>
"#,
                report.total_files, report.valid_files, report.invalid_files
            );

            for result in &report.results {
                let class = if result.valid { "valid" } else { "invalid" };
                html.push_str(&format!(
                    r#"    <div class="file-result {}">
        {}
    </div>
"#,
                    class,
                    format_html(result)
                ));
            }

            html.push_str("</body>\n</html>\n");
            Ok(html)
        }
    }
}
