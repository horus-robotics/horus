//! Asset validation tool for URDF files, meshes, and complete robot/object packages

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Asset validation report
#[derive(Debug, Clone)]
pub struct AssetValidationReport {
    pub asset_path: PathBuf,
    pub asset_type: AssetType,
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum AssetType {
    URDF,
    Mesh,
    RobotPackage,
    ObjectPackage,
    Unknown,
}

impl AssetValidationReport {
    pub fn new(path: PathBuf, asset_type: AssetType) -> Self {
        Self {
            asset_path: path,
            asset_type,
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn add_info(&mut self, info: String) {
        self.info.push(info);
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    pub fn print_report(&self) {
        println!("\n========== Asset Validation Report ==========");
        println!("Asset: {}", self.asset_path.display());
        println!("Type: {:?}", self.asset_type);
        println!(
            "Status: {}",
            if self.valid {
                "✓ VALID"
            } else {
                "✗ INVALID"
            }
        );

        if !self.errors.is_empty() {
            println!("\nErrors ({}):", self.errors.len());
            for error in &self.errors {
                println!("  ✗ {}", error);
            }
        }

        if !self.warnings.is_empty() {
            println!("\nWarnings ({}):", self.warnings.len());
            for warning in &self.warnings {
                println!("  ⚠ {}", warning);
            }
        }

        if !self.info.is_empty() {
            println!("\nInfo:");
            for info in &self.info {
                println!("  ℹ {}", info);
            }
        }

        println!("============================================\n");
    }
}

/// Validate a URDF file
pub fn validate_urdf(urdf_path: &Path) -> Result<AssetValidationReport> {
    let mut report = AssetValidationReport::new(urdf_path.to_path_buf(), AssetType::URDF);

    // Check if file exists
    if !urdf_path.exists() {
        report.add_error(format!("URDF file not found: {}", urdf_path.display()));
        return Ok(report);
    }

    // Check file extension
    if urdf_path.extension().and_then(|s| s.to_str()) != Some("urdf") {
        report.add_warning("File does not have .urdf extension".to_string());
    }

    // Read and parse URDF
    let contents = std::fs::read_to_string(urdf_path).context("Failed to read URDF file")?;

    let doc = roxmltree::Document::parse(&contents).context("Failed to parse URDF XML")?;

    report.add_info("URDF XML is well-formed".to_string());

    // Validate URDF structure
    validate_urdf_structure(&doc, &mut report, urdf_path.parent());

    Ok(report)
}

/// Validate URDF structure
fn validate_urdf_structure(
    doc: &roxmltree::Document,
    report: &mut AssetValidationReport,
    base_path: Option<&Path>,
) {
    // Check for robot root element
    let robot = match doc.root_element().tag_name().name() {
        "robot" => {
            report.add_info("Found <robot> root element".to_string());
            doc.root_element()
        }
        other => {
            report.add_error(format!("Expected <robot> root element, found <{}>", other));
            return;
        }
    };

    // Check robot name
    if let Some(name) = robot.attribute("name") {
        report.add_info(format!("Robot name: {}", name));
    } else {
        report.add_error("Robot missing 'name' attribute".to_string());
    }

    // Count and validate links
    let links: Vec<_> = robot
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "link")
        .collect();

    if links.is_empty() {
        report.add_error("URDF contains no links".to_string());
    } else {
        report.add_info(format!("Found {} links", links.len()));
    }

    // Count and validate joints
    let joints: Vec<_> = robot
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "joint")
        .collect();

    report.add_info(format!("Found {} joints", joints.len()));

    // Validate link names are unique
    let mut link_names = HashSet::new();
    for link in &links {
        if let Some(name) = link.attribute("name") {
            if !link_names.insert(name) {
                report.add_error(format!("Duplicate link name: {}", name));
            }
        } else {
            report.add_error("Link missing 'name' attribute".to_string());
        }
    }

    // Validate joints
    let mut joint_names = HashSet::new();
    for joint in &joints {
        if let Some(name) = joint.attribute("name") {
            if !joint_names.insert(name) {
                report.add_error(format!("Duplicate joint name: {}", name));
            }
        } else {
            report.add_error("Joint missing 'name' attribute".to_string());
        }

        // Check joint type
        if let Some(joint_type) = joint.attribute("type") {
            let valid_types = [
                "revolute",
                "continuous",
                "prismatic",
                "fixed",
                "floating",
                "planar",
            ];
            if !valid_types.contains(&joint_type) {
                report.add_error(format!("Invalid joint type: {}", joint_type));
            }
        } else {
            report.add_error(format!(
                "Joint '{}' missing 'type' attribute",
                joint.attribute("name").unwrap_or("unnamed")
            ));
        }

        // Validate parent and child links exist
        let parent = joint
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "parent");
        let child = joint
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "child");

        if let Some(parent_elem) = parent {
            if let Some(parent_link) = parent_elem.attribute("link") {
                if !link_names.contains(parent_link) {
                    report.add_error(format!(
                        "Joint '{}' references non-existent parent link: {}",
                        joint.attribute("name").unwrap_or("unnamed"),
                        parent_link
                    ));
                }
            }
        } else {
            report.add_error(format!(
                "Joint '{}' missing <parent> element",
                joint.attribute("name").unwrap_or("unnamed")
            ));
        }

        if let Some(child_elem) = child {
            if let Some(child_link) = child_elem.attribute("link") {
                if !link_names.contains(child_link) {
                    report.add_error(format!(
                        "Joint '{}' references non-existent child link: {}",
                        joint.attribute("name").unwrap_or("unnamed"),
                        child_link
                    ));
                }
            }
        } else {
            report.add_error(format!(
                "Joint '{}' missing <child> element",
                joint.attribute("name").unwrap_or("unnamed")
            ));
        }
    }

    // Validate mesh file references
    if let Some(base) = base_path {
        validate_mesh_references(&links, report, base);
    }

    // Check for inertial properties
    let links_with_inertia = links
        .iter()
        .filter(|link| {
            link.children()
                .any(|n| n.is_element() && n.tag_name().name() == "inertial")
        })
        .count();

    if links_with_inertia == 0 {
        report.add_warning("No links have inertial properties defined".to_string());
    } else if links_with_inertia < links.len() {
        report.add_warning(format!(
            "Only {}/{} links have inertial properties",
            links_with_inertia,
            links.len()
        ));
    } else {
        report.add_info("All links have inertial properties".to_string());
    }

    // Check for collision geometry
    let links_with_collision = links
        .iter()
        .filter(|link| {
            link.children()
                .any(|n| n.is_element() && n.tag_name().name() == "collision")
        })
        .count();

    if links_with_collision == 0 {
        report.add_warning("No links have collision geometry".to_string());
    } else {
        report.add_info(format!(
            "{}/{} links have collision geometry",
            links_with_collision,
            links.len()
        ));
    }
}

/// Validate mesh file references in URDF
fn validate_mesh_references(
    links: &[roxmltree::Node],
    report: &mut AssetValidationReport,
    base_path: &Path,
) {
    let mut mesh_files = HashSet::new();

    for link in links {
        // Check visual meshes
        for visual in link
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "visual")
        {
            if let Some(geometry) = visual
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "geometry")
            {
                if let Some(mesh_elem) = geometry
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "mesh")
                {
                    if let Some(filename) = mesh_elem.attribute("filename") {
                        mesh_files.insert(filename);
                    }
                }
            }
        }

        // Check collision meshes
        for collision in link
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "collision")
        {
            if let Some(geometry) = collision
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "geometry")
            {
                if let Some(mesh_elem) = geometry
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == "mesh")
                {
                    if let Some(filename) = mesh_elem.attribute("filename") {
                        mesh_files.insert(filename);
                    }
                }
            }
        }
    }

    if !mesh_files.is_empty() {
        report.add_info(format!("Found {} mesh references", mesh_files.len()));

        // Check if mesh files exist
        for mesh_file in mesh_files {
            // Handle package:// URIs
            let mesh_path = if mesh_file.starts_with("package://") {
                report.add_warning(format!(
                    "Mesh uses package:// URI which cannot be validated: {}",
                    mesh_file
                ));
                continue;
            } else if let Some(stripped) = mesh_file.strip_prefix("file://") {
                PathBuf::from(stripped)
            } else {
                base_path.join(mesh_file)
            };

            if !mesh_path.exists() {
                report.add_error(format!(
                    "Referenced mesh file not found: {}",
                    mesh_path.display()
                ));
            } else {
                // Check mesh file extension
                let ext = mesh_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let valid_extensions = ["obj", "stl", "dae", "gltf", "glb"];
                if !valid_extensions.contains(&ext) {
                    report.add_warning(format!(
                        "Mesh file has unsupported extension: {} ({})",
                        ext,
                        mesh_path.display()
                    ));
                }
            }
        }
    } else {
        report
            .add_warning("No mesh files referenced (URDF uses only primitive shapes)".to_string());
    }
}

/// Validate a robot package (directory with URDF and meshes)
pub fn validate_robot_package(package_path: &Path) -> Result<AssetValidationReport> {
    let mut report =
        AssetValidationReport::new(package_path.to_path_buf(), AssetType::RobotPackage);

    // Check if directory exists
    if !package_path.exists() {
        report.add_error(format!(
            "Package directory not found: {}",
            package_path.display()
        ));
        return Ok(report);
    }

    if !package_path.is_dir() {
        report.add_error(format!(
            "Path is not a directory: {}",
            package_path.display()
        ));
        return Ok(report);
    }

    // Find URDF files
    let urdf_files: Vec<_> = std::fs::read_dir(package_path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("urdf"))
        .collect();

    if urdf_files.is_empty() {
        report.add_error("No URDF files found in package".to_string());
    } else {
        report.add_info(format!("Found {} URDF file(s)", urdf_files.len()));

        // Validate each URDF
        for urdf_entry in urdf_files {
            let urdf_path = urdf_entry.path();
            report.add_info(format!("Validating URDF: {}", urdf_path.display()));

            match validate_urdf(&urdf_path) {
                Ok(urdf_report) => {
                    if !urdf_report.is_valid() {
                        report.add_error(format!(
                            "URDF validation failed for {}: {} errors",
                            urdf_path.display(),
                            urdf_report.errors.len()
                        ));
                        // Merge errors
                        for error in urdf_report.errors {
                            report.add_error(format!("  {}", error));
                        }
                    } else {
                        report.add_info(format!("URDF {} is valid", urdf_path.display()));
                    }

                    // Merge warnings
                    for warning in urdf_report.warnings {
                        report.add_warning(format!("  {}", warning));
                    }
                }
                Err(e) => {
                    report.add_error(format!(
                        "Failed to validate URDF {}: {}",
                        urdf_path.display(),
                        e
                    ));
                }
            }
        }
    }

    // Check for meshes directory
    let meshes_dir = package_path.join("meshes");
    if meshes_dir.exists() {
        report.add_info("Found meshes directory".to_string());

        // Count mesh files
        let mesh_count =
            count_files_with_extensions(&meshes_dir, &["obj", "stl", "dae", "gltf", "glb"])?;

        if mesh_count > 0 {
            report.add_info(format!("Found {} mesh files", mesh_count));
        } else {
            report.add_warning("Meshes directory is empty".to_string());
        }
    } else {
        report.add_warning("No meshes directory found".to_string());
    }

    Ok(report)
}

/// Count files with specific extensions in a directory (recursive)
fn count_files_with_extensions(dir: &Path, extensions: &[&str]) -> Result<usize> {
    let mut count = 0;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            count += count_files_with_extensions(&path, extensions)?;
        } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if extensions.contains(&ext) {
                count += 1;
            }
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validation_report_creation() {
        let report = AssetValidationReport::new(PathBuf::from("/test/path"), AssetType::URDF);

        assert!(report.is_valid());
        assert_eq!(report.errors.len(), 0);
        assert_eq!(report.warnings.len(), 0);
    }

    #[test]
    fn test_validation_report_errors() {
        let mut report = AssetValidationReport::new(PathBuf::from("/test/path"), AssetType::URDF);

        report.add_error("Test error".to_string());
        assert!(!report.is_valid());
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn test_validate_nonexistent_urdf() {
        let result = validate_urdf(Path::new("/nonexistent/file.urdf"));
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(!report.is_valid());
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn test_validate_minimal_urdf() {
        let temp_dir = TempDir::new().unwrap();
        let urdf_path = temp_dir.path().join("test.urdf");

        let minimal_urdf = r#"<?xml version="1.0"?>
<robot name="test_robot">
    <link name="base_link">
        <inertial>
            <mass value="1.0"/>
            <inertia ixx="1.0" ixy="0" ixz="0" iyy="1.0" iyz="0" izz="1.0"/>
        </inertial>
    </link>
</robot>"#;

        fs::write(&urdf_path, minimal_urdf).unwrap();

        let result = validate_urdf(&urdf_path);
        assert!(result.is_ok());

        let report = result.unwrap();
        assert!(report.is_valid());
    }
}
