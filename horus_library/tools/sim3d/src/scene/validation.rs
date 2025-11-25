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
    ///
    /// Matches the SceneDefinition struct with tagged enum shapes
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
                    "type": "number",
                    "description": "Gravity magnitude in m/s² (negative for downward)",
                    "default": -9.81
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
                            "pattern": "^[a-zA-Z_][a-zA-Z0-9_]*$",
                            "description": "Unique robot identifier (alphanumeric and underscores, must start with letter or underscore)"
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
                    "required": ["name", "shape", "position"],
                    "properties": {
                        "name": {
                            "type": "string",
                            "minLength": 1
                        },
                        "shape": {
                            "$ref": "#/definitions/Shape",
                            "description": "Object shape with type and dimensions"
                        },
                        "position": {
                            "$ref": "#/definitions/Vec3"
                        },
                        "rotation": {
                            "$ref": "#/definitions/Quaternion"
                        },
                        "rotation_euler": {
                            "$ref": "#/definitions/Vec3",
                            "description": "Euler angles in degrees [x, y, z]"
                        },
                        "is_static": {
                            "type": "boolean",
                            "default": false
                        },
                        "mass": {
                            "type": "number",
                            "minimum": 0.0,
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
                        "color": {
                            "$ref": "#/definitions/Color",
                            "description": "RGB color [r, g, b]"
                        },
                        "damping": {
                            "type": "array",
                            "description": "[linear, angular] damping",
                            "items": { "type": "number" },
                            "minItems": 2,
                            "maxItems": 2
                        }
                    }
                },
                "Shape": {
                    "type": "object",
                    "required": ["type"],
                    "description": "Shape definition with type tag",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["box", "sphere", "cylinder", "capsule", "ground"]
                        },
                        "size": {
                            "$ref": "#/definitions/Vec3",
                            "description": "Size for box [x, y, z]"
                        },
                        "size_x": {
                            "type": "number",
                            "description": "X size for ground"
                        },
                        "size_z": {
                            "type": "number",
                            "description": "Z size for ground"
                        },
                        "radius": {
                            "type": "number",
                            "description": "Radius for sphere, cylinder, capsule"
                        },
                        "height": {
                            "type": "number",
                            "description": "Height for cylinder, capsule"
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
                    "items": { "type": "number" },
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
gravity: -9.81
robots: []
objects: []
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(report.valid, "Scene should be valid: {:?}", report.errors);
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
    fn test_invalid_gravity_wrong_type() {
        let validator = SceneValidator::new().unwrap();
        let yaml = r#"
name: test_scene
gravity: "not a number"
"#;

        let report = validator.validate_yaml(yaml).unwrap();
        assert!(
            !report.valid,
            "Scene should be invalid with wrong gravity type"
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
