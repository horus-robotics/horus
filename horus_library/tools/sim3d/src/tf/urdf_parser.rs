use bevy::prelude::*;
use roxmltree;
use std::fs;
use std::path::Path;

/// URDF Robot description
#[derive(Debug, Clone)]
pub struct URDFRobot {
    pub name: String,
    pub links: Vec<URDFLink>,
    pub joints: Vec<URDFJoint>,
    pub materials: Vec<URDFMaterial>,
}

/// URDF Link (robot body part)
#[derive(Debug, Clone)]
pub struct URDFLink {
    pub name: String,
    pub visual: Vec<URDFVisual>,
    pub collision: Vec<URDFCollision>,
    pub inertial: Option<URDFInertial>,
}

/// URDF Joint (connection between links)
#[derive(Debug, Clone)]
pub struct URDFJoint {
    pub name: String,
    pub joint_type: URDFJointType,
    pub parent: String,
    pub child: String,
    pub origin: URDFPose,
    pub axis: Vec3,
    pub limit: Option<URDFLimit>,
    pub dynamics: Option<URDFDynamics>,
}

/// Joint types in URDF
#[derive(Debug, Clone, PartialEq)]
pub enum URDFJointType {
    Fixed,
    Revolute,
    Continuous,
    Prismatic,
    Floating,
    Planar,
}

/// Visual element
#[derive(Debug, Clone)]
pub struct URDFVisual {
    pub name: Option<String>,
    pub origin: URDFPose,
    pub geometry: URDFGeometry,
    pub material: Option<String>,
}

/// Collision element
#[derive(Debug, Clone)]
pub struct URDFCollision {
    pub name: Option<String>,
    pub origin: URDFPose,
    pub geometry: URDFGeometry,
}

/// Inertial properties
#[derive(Debug, Clone)]
pub struct URDFInertial {
    pub origin: URDFPose,
    pub mass: f32,
    pub inertia: [f32; 6], // ixx, ixy, ixz, iyy, iyz, izz
}

/// Geometry types
#[derive(Debug, Clone)]
pub enum URDFGeometry {
    Box { size: Vec3 },
    Cylinder { radius: f32, length: f32 },
    Sphere { radius: f32 },
    Mesh { filename: String, scale: Vec3 },
}

/// Material definition
#[derive(Debug, Clone)]
pub struct URDFMaterial {
    pub name: String,
    pub color: Option<Color>,
    pub texture: Option<String>,
}

/// Joint limits
#[derive(Debug, Clone)]
pub struct URDFLimit {
    pub lower: f32,
    pub upper: f32,
    pub effort: f32,
    pub velocity: f32,
}

/// Joint dynamics
#[derive(Debug, Clone)]
pub struct URDFDynamics {
    pub damping: f32,
    pub friction: f32,
}

/// Pose (position + orientation)
#[derive(Debug, Clone)]
pub struct URDFPose {
    pub position: Vec3,
    pub rotation: Vec3, // RPY (roll, pitch, yaw)
}

impl Default for URDFPose {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
        }
    }
}

impl URDFPose {
    /// Convert URDF pose to Bevy Transform
    pub fn to_transform(&self) -> Transform {
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
        );
        Transform {
            translation: self.position,
            rotation,
            scale: Vec3::ONE,
        }
    }
}

/// URDF Parser
pub struct URDFParser;

impl URDFParser {
    /// Load and parse URDF from file
    pub fn load_file(path: impl AsRef<Path>) -> Result<URDFRobot, String> {
        let xml = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read URDF file: {}", e))?;
        Self::parse_string(&xml)
    }

    /// Parse URDF from XML string
    pub fn parse_string(xml: &str) -> Result<URDFRobot, String> {
        // Strip XML declaration if present (roxmltree doesn't handle it well with leading whitespace)
        let xml_cleaned = xml.trim();
        let xml_cleaned = if xml_cleaned.starts_with("<?xml") {
            if let Some(end_idx) = xml_cleaned.find("?>") {
                xml_cleaned[end_idx + 2..].trim_start()
            } else {
                xml_cleaned
            }
        } else {
            xml_cleaned
        };

        let doc = roxmltree::Document::parse(xml_cleaned)
            .map_err(|e| format!("Failed to parse XML: {}", e))?;

        let robot_elem = doc
            .root_element()
            .children()
            .find(|n| n.has_tag_name("robot"))
            .or_else(|| {
                // Robot might be the root element
                if doc.root_element().has_tag_name("robot") {
                    Some(doc.root_element())
                } else {
                    None
                }
            })
            .ok_or("No <robot> element found")?;

        Self::parse_robot(robot_elem)
    }

    fn parse_robot(elem: roxmltree::Node) -> Result<URDFRobot, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_robot")
            .to_string();

        let mut links = Vec::new();
        let mut joints = Vec::new();
        let mut materials = Vec::new();

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "link" => links.push(Self::parse_link(child)?),
                "joint" => joints.push(Self::parse_joint(child)?),
                "material" => materials.push(Self::parse_material(child)?),
                _ => {} // Ignore unknown elements
            }
        }

        Ok(URDFRobot {
            name,
            links,
            joints,
            materials,
        })
    }

    fn parse_link(elem: roxmltree::Node) -> Result<URDFLink, String> {
        let name = elem
            .attribute("name")
            .ok_or("Link missing name")?
            .to_string();

        let mut visual = Vec::new();
        let mut collision = Vec::new();
        let mut inertial = None;

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "visual" => visual.push(Self::parse_visual(child)?),
                "collision" => collision.push(Self::parse_collision(child)?),
                "inertial" => inertial = Some(Self::parse_inertial(child)?),
                _ => {}
            }
        }

        Ok(URDFLink {
            name,
            visual,
            collision,
            inertial,
        })
    }

    fn parse_joint(elem: roxmltree::Node) -> Result<URDFJoint, String> {
        let name = elem
            .attribute("name")
            .ok_or("Joint missing name")?
            .to_string();

        let joint_type = elem
            .attribute("type")
            .ok_or("Joint missing type")?;

        let joint_type = match joint_type {
            "fixed" => URDFJointType::Fixed,
            "revolute" => URDFJointType::Revolute,
            "continuous" => URDFJointType::Continuous,
            "prismatic" => URDFJointType::Prismatic,
            "floating" => URDFJointType::Floating,
            "planar" => URDFJointType::Planar,
            _ => return Err(format!("Unknown joint type: {}", joint_type)),
        };

        let mut parent = None;
        let mut child = None;
        let mut origin = URDFPose::default();
        let mut axis = Vec3::X; // Default axis
        let mut limit = None;
        let mut dynamics = None;

        for node in elem.children().filter(|n| n.is_element()) {
            match node.tag_name().name() {
                "parent" => parent = node.attribute("link").map(|s| s.to_string()),
                "child" => child = node.attribute("link").map(|s| s.to_string()),
                "origin" => origin = Self::parse_origin(node)?,
                "axis" => axis = Self::parse_axis(node)?,
                "limit" => limit = Some(Self::parse_limit(node)?),
                "dynamics" => dynamics = Some(Self::parse_dynamics(node)?),
                _ => {}
            }
        }

        Ok(URDFJoint {
            name,
            joint_type,
            parent: parent.ok_or("Joint missing parent")?,
            child: child.ok_or("Joint missing child")?,
            origin,
            axis,
            limit,
            dynamics,
        })
    }

    fn parse_visual(elem: roxmltree::Node) -> Result<URDFVisual, String> {
        let name = elem.attribute("name").map(|s| s.to_string());
        let mut origin = URDFPose::default();
        let mut geometry = None;
        let mut material = None;

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "origin" => origin = Self::parse_origin(child)?,
                "geometry" => geometry = Some(Self::parse_geometry(child)?),
                "material" => material = child.attribute("name").map(|s| s.to_string()),
                _ => {}
            }
        }

        Ok(URDFVisual {
            name,
            origin,
            geometry: geometry.ok_or("Visual missing geometry")?,
            material,
        })
    }

    fn parse_collision(elem: roxmltree::Node) -> Result<URDFCollision, String> {
        let name = elem.attribute("name").map(|s| s.to_string());
        let mut origin = URDFPose::default();
        let mut geometry = None;

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "origin" => origin = Self::parse_origin(child)?,
                "geometry" => geometry = Some(Self::parse_geometry(child)?),
                _ => {}
            }
        }

        Ok(URDFCollision {
            name,
            origin,
            geometry: geometry.ok_or("Collision missing geometry")?,
        })
    }

    fn parse_inertial(elem: roxmltree::Node) -> Result<URDFInertial, String> {
        let mut origin = URDFPose::default();
        let mut mass = 1.0;
        let mut inertia = [1.0, 0.0, 0.0, 1.0, 0.0, 1.0];

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "origin" => origin = Self::parse_origin(child)?,
                "mass" => {
                    mass = child
                        .attribute("value")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                }
                "inertia" => {
                    inertia[0] = child.attribute("ixx").and_then(|s| s.parse().ok()).unwrap_or(1.0);
                    inertia[1] = child.attribute("ixy").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    inertia[2] = child.attribute("ixz").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    inertia[3] = child.attribute("iyy").and_then(|s| s.parse().ok()).unwrap_or(1.0);
                    inertia[4] = child.attribute("iyz").and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    inertia[5] = child.attribute("izz").and_then(|s| s.parse().ok()).unwrap_or(1.0);
                }
                _ => {}
            }
        }

        Ok(URDFInertial {
            origin,
            mass,
            inertia,
        })
    }

    fn parse_geometry(elem: roxmltree::Node) -> Result<URDFGeometry, String> {
        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "box" => {
                    let size = Self::parse_vec3(child.attribute("size").unwrap_or("1 1 1"))?;
                    return Ok(URDFGeometry::Box { size });
                }
                "cylinder" => {
                    let radius = child
                        .attribute("radius")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.5);
                    let length = child
                        .attribute("length")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1.0);
                    return Ok(URDFGeometry::Cylinder { radius, length });
                }
                "sphere" => {
                    let radius = child
                        .attribute("radius")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.5);
                    return Ok(URDFGeometry::Sphere { radius });
                }
                "mesh" => {
                    let filename = child
                        .attribute("filename")
                        .ok_or("Mesh missing filename")?
                        .to_string();
                    let scale = child
                        .attribute("scale")
                        .and_then(|s| Self::parse_vec3(s).ok())
                        .unwrap_or(Vec3::ONE);
                    return Ok(URDFGeometry::Mesh { filename, scale });
                }
                _ => {}
            }
        }
        Err("No geometry found".to_string())
    }

    fn parse_material(elem: roxmltree::Node) -> Result<URDFMaterial, String> {
        let name = elem
            .attribute("name")
            .ok_or("Material missing name")?
            .to_string();

        let mut color = None;
        let mut texture = None;

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "color" => {
                    if let Some(rgba) = child.attribute("rgba") {
                        let parts: Vec<f32> = rgba
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if parts.len() >= 3 {
                            color = Some(Color::srgba(parts[0], parts[1], parts[2], parts.get(3).copied().unwrap_or(1.0)));
                        }
                    }
                }
                "texture" => {
                    texture = child.attribute("filename").map(|s| s.to_string());
                }
                _ => {}
            }
        }

        Ok(URDFMaterial {
            name,
            color,
            texture,
        })
    }

    fn parse_origin(elem: roxmltree::Node) -> Result<URDFPose, String> {
        let xyz = elem.attribute("xyz").unwrap_or("0 0 0");
        let rpy = elem.attribute("rpy").unwrap_or("0 0 0");

        Ok(URDFPose {
            position: Self::parse_vec3(xyz)?,
            rotation: Self::parse_vec3(rpy)?,
        })
    }

    fn parse_axis(elem: roxmltree::Node) -> Result<Vec3, String> {
        let xyz = elem.attribute("xyz").unwrap_or("1 0 0");
        Self::parse_vec3(xyz)
    }

    fn parse_limit(elem: roxmltree::Node) -> Result<URDFLimit, String> {
        Ok(URDFLimit {
            lower: elem
                .attribute("lower")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            upper: elem
                .attribute("upper")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            effort: elem
                .attribute("effort")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            velocity: elem
                .attribute("velocity")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
        })
    }

    fn parse_dynamics(elem: roxmltree::Node) -> Result<URDFDynamics, String> {
        Ok(URDFDynamics {
            damping: elem
                .attribute("damping")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            friction: elem
                .attribute("friction")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
        })
    }

    fn parse_vec3(s: &str) -> Result<Vec3, String> {
        let parts: Vec<f32> = s
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 3 {
            Ok(Vec3::new(parts[0], parts[1], parts[2]))
        } else {
            Err(format!("Invalid Vec3: {}", s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_urdf() {
        let urdf = r#"
        <?xml version="1.0"?>
        <robot name="test_robot">
            <link name="base_link">
                <visual>
                    <geometry>
                        <box size="1 1 1"/>
                    </geometry>
                </visual>
            </link>
        </robot>
        "#;

        let robot = URDFParser::parse_string(urdf).unwrap();
        assert_eq!(robot.name, "test_robot");
        assert_eq!(robot.links.len(), 1);
        assert_eq!(robot.links[0].name, "base_link");
    }

    #[test]
    fn test_parse_joint() {
        let urdf = r#"
        <?xml version="1.0"?>
        <robot name="test">
            <link name="base"/>
            <link name="link1"/>
            <joint name="joint1" type="revolute">
                <parent link="base"/>
                <child link="link1"/>
                <axis xyz="0 0 1"/>
                <limit lower="-3.14" upper="3.14" effort="100" velocity="1"/>
            </joint>
        </robot>
        "#;

        let robot = URDFParser::parse_string(urdf).unwrap();
        assert_eq!(robot.joints.len(), 1);
        assert_eq!(robot.joints[0].joint_type, URDFJointType::Revolute);
        assert_eq!(robot.joints[0].parent, "base");
        assert_eq!(robot.joints[0].child, "link1");
    }

    #[test]
    fn test_parse_geometry() {
        let urdf = r#"
        <?xml version="1.0"?>
        <robot name="test">
            <link name="box_link">
                <visual>
                    <geometry>
                        <box size="2 3 4"/>
                    </geometry>
                </visual>
            </link>
            <link name="sphere_link">
                <visual>
                    <geometry>
                        <sphere radius="1.5"/>
                    </geometry>
                </visual>
            </link>
        </robot>
        "#;

        let robot = URDFParser::parse_string(urdf).unwrap();
        assert_eq!(robot.links.len(), 2);

        match &robot.links[0].visual[0].geometry {
            URDFGeometry::Box { size } => {
                assert_eq!(*size, Vec3::new(2.0, 3.0, 4.0));
            }
            _ => panic!("Expected box geometry"),
        }

        match &robot.links[1].visual[0].geometry {
            URDFGeometry::Sphere { radius } => {
                assert_eq!(*radius, 1.5);
            }
            _ => panic!("Expected sphere geometry"),
        }
    }

    #[test]
    fn test_pose_to_transform() {
        let pose = URDFPose {
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Vec3::new(0.0, 0.0, 1.57),
        };

        let transform = pose.to_transform();
        assert_eq!(transform.translation, Vec3::new(1.0, 2.0, 3.0));
    }
}
