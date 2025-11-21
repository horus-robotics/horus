//! Scene validation using JSON Schema
//!
//! Validates scene YAML/JSON files before parsing to provide clear error messages.

use anyhow::{Context, Result};
use jsonschema::JSONSchema;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::OnceLock;

static SCHEMA: OnceLock<JSONSchema> = OnceLock::new();

/// Scene validation report
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: Vec::new(),
        }
    }
}

/// Scene validator using JSON Schema
pub struct SceneValidator;

impl SceneValidator {
    /// Create a new validator with the built-in scene schema
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// Validate a YAML string
    pub fn validate_yaml(&self, yaml_content: &str) -> Result<ValidationReport> {
        // Parse YAML to JSON value
        let value: Value = serde_yaml::from_str(yaml_content).context("Failed to parse YAML")?;

        self.validate_json(&value)
    }

    /// Validate a JSON value
    pub fn validate_json(&self, value: &Value) -> Result<ValidationReport> {
        // Get or initialize the static schema
        let schema = SCHEMA.get_or_init(|| {
            let schema_value = Box::leak(Box::new(Self::get_schema()));
            JSONSchema::options()
                .compile(schema_value)
                .expect("Failed to compile built-in schema")
        });

        match schema.validate(value) {
            Ok(_) => Ok(ValidationReport::success()),
            Err(errors) => {
                let error_messages: Vec<String> = errors
                    .map(|error| {
                        let path = error.instance_path.to_string();
                        let message = error.to_string();
                        if path.is_empty() {
                            message
                        } else {
                            format!("{}: {}", path, message)
                        }
                    })
                    .collect();

                Ok(ValidationReport::failure(error_messages))
            }
        }
    }

    /// Validate a file
    pub fn validate_file<P: AsRef<Path>>(&self, path: P) -> Result<ValidationReport> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read file: {}", path.as_ref().display()))?;

        self.validate_yaml(&content)
    }

    /// Get the JSON schema for scene definitions
    fn get_schema() -> Value {
        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "sim3d Scene Definition",
            "description": "Schema for sim3d scene configuration files",
            "type": "object",
            "required": ["name"],
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1,
                    "description": "Unique scene name"
                },
                "description": {
                    "type": "string",
                    "description": "Human-readable scene description"
                },
                "gravity": {
                    "type": "array",
                    "description": "Gravity vector [x, y, z] in m/s²",
                    "items": { "type": "number" },
                    "minItems": 3,
                    "maxItems": 3,
                    "default": [0.0, -9.81, 0.0]
                },
                "robots": {
                    "type": "array",
                    "description": "List of robots to spawn",
                    "items": { "$ref": "#/definitions/Robot" }
                },
                "objects": {
                    "type": "array",
                    "description": "List of objects to spawn",
                    "items": { "$ref": "#/definitions/Object" }
                },
                "lighting": {
                    "$ref": "#/definitions/Lighting"
                }
            },
            "definitions": {
                "Robot": {
                    "type": "object",
                    "required": ["name", "urdf_path"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1,
                            "pattern": "^[a-zA-Z0-9_-]+$",
                            "description": "Unique robot identifier"
                        },
                        "urdf_path": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Path to URDF file"
                        },
                        "position": {
                            "$ref": "#/definitions/Vec3"
                        },
                        "rotation": {
                            "$ref": "#/definitions/Quaternion",
                            "description": "Orientation as quaternion [w, x, y, z]"
                        },
                        "rotation_euler": {
                            "$ref": "#/definitions/Vec3",
                            "description": "Orientation as Euler angles [roll, pitch, yaw] in degrees"
                        }
                    }
                },
                "Object": {
                    "type": "object",
                    "required": ["name", "shape"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1
                        },
                        "shape": {
                            "type": "string",
                            "enum": ["box", "sphere", "cylinder", "capsule"],
                            "description": "Object shape type"
                        },
                        "size": {
                            "$ref": "#/definitions/Vec3",
                            "description": "Size for box shape [width, height, depth]"
                        },
                        "radius": {
                            "type": "number",
                            "exclusiveMinimum": 0.0,
                            "description": "Radius for sphere, cylinder, or capsule"
                        },
                        "height": {
                            "type": "number",
                            "exclusiveMinimum": 0.0,
                            "description": "Height for cylinder or capsule"
                        },
                        "position": {
                            "$ref": "#/definitions/Vec3"
                        },
                        "rotation": {
                            "$ref": "#/definitions/Quaternion"
                        },
                        "is_static": {
                            "type": "boolean",
                            "default": false
                        },
                        "mass": {
                            "type": "number",
                            "exclusiveMinimum": 0.0,
                            "default": 1.0
                        },
                        "friction": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "default": 0.5
                        },
                        "restitution": {
                            "type": "number",
                            "minimum": 0.0,
                            "maximum": 1.0,
                            "default": 0.0
                        },
                        "material": {
                            "type": "string",
                            "description": "Material preset name"
                        },
                        "color": {
                            "$ref": "#/definitions/Color",
                            "description": "RGB color [r, g, b]"
                        }
                    }
                },
                "Vec3": {
                    "type": "array",
                    "items": { "type": "number" },
                    "minItems": 3,
                    "maxItems": 3
                },
                "Quaternion": {
                    "type": "array",
                    "description": "[w, x, y, z]",
                    "items": { "type": "number" },
                    "minItems": 4,
                    "maxItems": 4
                },
                "Color": {
                    "type": "array",
                    "description": "[r, g, b] in range 0.0-1.0",
                    "items": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0
                    },
                    "minItems": 3,
                    "maxItems": 3
                },
                "Lighting": {
                    "type": "object",
                    "properties": {
                        "ambient": {
                            "$ref": "#/definitions/Color"
                        },
                        "directional": {
                            "type": "object",
                            "properties": {
                                "direction": {
                                    "$ref": "#/definitions/Vec3"
                                },
                                "color": {
                                    "$ref": "#/definitions/Color"
                                },
                                "illuminance": {
                                    "type": "number",
                                    "minimum": 0.0,
                                    "description": "Light intensity in lux"
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}

impl Default for SceneValidator {
    fn default() -> Self {
        Self::new().expect("Failed to create default scene validator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_scene() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
name: test_scene
description: A test scene
gravity: [0.0, -9.81, 0.0]
robots: []
objects: []
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(report.valid, "Scene should be valid");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_invalid_scene_missing_name() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
description: A test scene
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(!report.valid, "Scene should be invalid without name");
        assert!(!report.errors.is_empty());
    }

    #[test]
    fn test_invalid_gravity_wrong_size() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
name: test_scene
gravity: [0.0, -9.81]
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(
            !report.valid,
            "Scene should be invalid with wrong gravity size"
        );
    }

    #[test]
    fn test_valid_robot() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
name: test_scene
robots:
  - name: robot1
    urdf_path: "path/to/robot.urdf"
    position: [0.0, 0.0, 0.0]
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(report.valid, "Scene with robot should be valid");
    }

    #[test]
    fn test_invalid_robot_name() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
name: test_scene
robots:
  - name: "robot with spaces"
    urdf_path: "path/to/robot.urdf"
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(!report.valid, "Robot with spaces in name should be invalid");
    }
}
