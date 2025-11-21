use bevy::prelude::*;
use std::fs::File;
use std::io::Read as IoRead;
use std::path::Path;

/// SDF (Simulation Description Format) importer
/// Supports importing Gazebo SDF world files into the simulator
pub struct SDFImporter;

/// Parsed SDF world data
#[derive(Clone, Debug)]
pub struct SDFWorld {
    pub name: String,
    pub gravity: Vec3,
    pub models: Vec<SDFModel>,
    pub lights: Vec<SDFLight>,
    pub physics: SDFPhysics,
}

impl Default for SDFWorld {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            gravity: Vec3::new(0.0, 0.0, -9.81), // SDF uses Z-up convention
            models: Vec::new(),
            lights: Vec::new(),
            physics: SDFPhysics::default(),
        }
    }
}

/// SDF model (robot, obstacle, etc.)
#[derive(Clone, Debug)]
pub struct SDFModel {
    pub name: String,
    pub pose: SDFPose,
    pub links: Vec<SDFLink>,
    pub joints: Vec<SDFJoint>,
    pub is_static: bool,
}

/// SDF link (rigid body part)
#[derive(Clone, Debug)]
pub struct SDFLink {
    pub name: String,
    pub pose: SDFPose,
    pub inertial: SDFInertial,
    pub collisions: Vec<SDFCollision>,
    pub visuals: Vec<SDFVisual>,
}

/// SDF joint (connection between links)
#[derive(Clone, Debug)]
pub struct SDFJoint {
    pub name: String,
    pub joint_type: String, // revolute, prismatic, fixed, etc.
    pub parent: String,
    pub child: String,
    pub pose: SDFPose,
    pub axis: SDFAxis,
}

/// SDF axis configuration
#[derive(Clone, Debug)]
pub struct SDFAxis {
    pub xyz: Vec3,
    pub limit_lower: f32,
    pub limit_upper: f32,
    pub limit_effort: f32,
    pub limit_velocity: f32,
}

impl Default for SDFAxis {
    fn default() -> Self {
        Self {
            xyz: Vec3::Z,
            limit_lower: -f32::INFINITY,
            limit_upper: f32::INFINITY,
            limit_effort: f32::INFINITY,
            limit_velocity: f32::INFINITY,
        }
    }
}

/// SDF pose (position + orientation)
#[derive(Clone, Debug, Default)]
pub struct SDFPose {
    pub position: Vec3,
    pub rotation: Vec3, // Roll, pitch, yaw (Euler angles)
}

impl SDFPose {
    pub fn to_transform(&self) -> Transform {
        // Convert from SDF (Z-up) to Bevy (Y-up)
        let position = Vec3::new(self.position.x, self.position.z, self.position.y);

        // Convert Euler angles to quaternion
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            self.rotation.x,
            self.rotation.z, // Swap Y and Z
            self.rotation.y,
        );

        Transform {
            translation: position,
            rotation,
            scale: Vec3::ONE,
        }
    }
}

/// SDF inertial properties
#[derive(Clone, Debug)]
pub struct SDFInertial {
    pub mass: f32,
    pub inertia: SDFInertia,
}

impl Default for SDFInertial {
    fn default() -> Self {
        Self {
            mass: 1.0,
            inertia: SDFInertia::default(),
        }
    }
}

/// SDF inertia tensor
#[derive(Clone, Debug, Default)]
pub struct SDFInertia {
    pub ixx: f32,
    pub iyy: f32,
    pub izz: f32,
    pub ixy: f32,
    pub ixz: f32,
    pub iyz: f32,
}

/// SDF collision shape
#[derive(Clone, Debug)]
pub struct SDFCollision {
    pub name: String,
    pub pose: SDFPose,
    pub geometry: SDFGeometry,
}

/// SDF visual element
#[derive(Clone, Debug)]
pub struct SDFVisual {
    pub name: String,
    pub pose: SDFPose,
    pub geometry: SDFGeometry,
    pub material: SDFMaterial,
}

/// SDF geometry types
#[derive(Clone, Debug)]
pub enum SDFGeometry {
    Box { size: Vec3 },
    Cylinder { radius: f32, length: f32 },
    Sphere { radius: f32 },
    Plane { normal: Vec3, size: Vec2 },
    Mesh { uri: String, scale: Vec3 },
}

/// SDF material
#[derive(Clone, Debug, Default)]
pub struct SDFMaterial {
    pub ambient: [f32; 4],
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub emissive: [f32; 4],
}

/// SDF light
#[derive(Clone, Debug)]
pub struct SDFLight {
    pub name: String,
    pub light_type: String, // point, directional, spot
    pub pose: SDFPose,
    pub diffuse: [f32; 4],
    pub specular: [f32; 4],
    pub attenuation: SDFAttenuation,
}

#[derive(Clone, Debug, Default)]
pub struct SDFAttenuation {
    pub range: f32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
}

/// SDF physics configuration
#[derive(Clone, Debug)]
pub struct SDFPhysics {
    pub max_step_size: f32,
    pub real_time_factor: f32,
    pub real_time_update_rate: f32,
}

impl Default for SDFPhysics {
    fn default() -> Self {
        Self {
            max_step_size: 0.001,
            real_time_factor: 1.0,
            real_time_update_rate: 1000.0,
        }
    }
}

impl SDFImporter {
    /// Load SDF world from file
    pub fn load_file(path: impl AsRef<Path>) -> Result<SDFWorld, String> {
        let mut file =
            File::open(path.as_ref()).map_err(|e| format!("Failed to open SDF file: {}", e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read SDF file: {}", e))?;

        Self::parse_string(&contents)
    }

    /// Parse SDF from string
    pub fn parse_string(xml: &str) -> Result<SDFWorld, String> {
        let doc =
            roxmltree::Document::parse(xml).map_err(|e| format!("Failed to parse XML: {}", e))?;

        let root = doc.root_element();

        // Check SDF version if present (warn if unsupported, but continue parsing)
        if root.has_tag_name("sdf") {
            if let Some(version) = root.attribute("version") {
                if let Err(msg) = Self::validate_sdf_version(version) {
                    eprintln!("Warning: {}", msg);
                }
            }
        }

        // Find <world> element
        let world_elem = if root.has_tag_name("world") {
            root
        } else if let Some(sdf) = root.children().find(|n| n.has_tag_name("sdf")) {
            sdf.children()
                .find(|n| n.has_tag_name("world"))
                .ok_or("No <world> element found in SDF")?
        } else {
            root.children()
                .find(|n| n.has_tag_name("world"))
                .ok_or("No <world> element found")?
        };

        Self::parse_world(world_elem)
    }

    /// Validate SDF version (supports 1.4, 1.5, 1.6, 1.7, 1.8)
    fn validate_sdf_version(version: &str) -> Result<(), String> {
        const SUPPORTED_VERSIONS: &[&str] = &["1.4", "1.5", "1.6", "1.7", "1.8"];

        if SUPPORTED_VERSIONS.contains(&version) {
            Ok(())
        } else {
            Err(format!(
                "SDF version {} not officially supported. Supported versions: {:?}. Will attempt to parse anyway.",
                version, SUPPORTED_VERSIONS
            ))
        }
    }

    fn parse_world(elem: roxmltree::Node) -> Result<SDFWorld, String> {
        let mut world = SDFWorld::default();

        if let Some(name) = elem.attribute("name") {
            world.name = name.to_string();
        }

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "gravity" => {
                    if let Some(text) = child.text() {
                        let parts: Vec<f32> = text
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if parts.len() >= 3 {
                            world.gravity = Vec3::new(parts[0], parts[1], parts[2]);
                        }
                    }
                }
                "model" => {
                    if let Ok(model) = Self::parse_model(child) {
                        world.models.push(model);
                    }
                }
                "light" => {
                    if let Ok(light) = Self::parse_light(child) {
                        world.lights.push(light);
                    }
                }
                "physics" => {
                    if let Ok(physics) = Self::parse_physics(child) {
                        world.physics = physics;
                    }
                }
                _ => {}
            }
        }

        Ok(world)
    }

    fn parse_model(elem: roxmltree::Node) -> Result<SDFModel, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_model")
            .to_string();

        let mut model = SDFModel {
            name,
            pose: SDFPose::default(),
            links: Vec::new(),
            joints: Vec::new(),
            is_static: false,
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "pose" => {
                    model.pose = Self::parse_pose(child)?;
                }
                "static" => {
                    if let Some(text) = child.text() {
                        model.is_static = text.trim() == "true" || text.trim() == "1";
                    }
                }
                "link" => {
                    if let Ok(link) = Self::parse_link(child) {
                        model.links.push(link);
                    }
                }
                "joint" => {
                    if let Ok(joint) = Self::parse_joint(child) {
                        model.joints.push(joint);
                    }
                }
                _ => {}
            }
        }

        Ok(model)
    }

    fn parse_link(elem: roxmltree::Node) -> Result<SDFLink, String> {
        let name = elem.attribute("name").unwrap_or("unnamed_link").to_string();

        let mut link = SDFLink {
            name,
            pose: SDFPose::default(),
            inertial: SDFInertial::default(),
            collisions: Vec::new(),
            visuals: Vec::new(),
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "pose" => link.pose = Self::parse_pose(child)?,
                "inertial" => link.inertial = Self::parse_inertial(child)?,
                "collision" => {
                    if let Ok(collision) = Self::parse_collision(child) {
                        link.collisions.push(collision);
                    }
                }
                "visual" => {
                    if let Ok(visual) = Self::parse_visual(child) {
                        link.visuals.push(visual);
                    }
                }
                _ => {}
            }
        }

        Ok(link)
    }

    fn parse_joint(elem: roxmltree::Node) -> Result<SDFJoint, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_joint")
            .to_string();
        let joint_type = elem.attribute("type").unwrap_or("fixed").to_string();

        let mut joint = SDFJoint {
            name,
            joint_type,
            parent: String::new(),
            child: String::new(),
            pose: SDFPose::default(),
            axis: SDFAxis::default(),
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "parent" => {
                    if let Some(text) = child.text() {
                        joint.parent = text.trim().to_string();
                    }
                }
                "child" => {
                    if let Some(text) = child.text() {
                        joint.child = text.trim().to_string();
                    }
                }
                "pose" => joint.pose = Self::parse_pose(child)?,
                "axis" => joint.axis = Self::parse_axis(child)?,
                _ => {}
            }
        }

        Ok(joint)
    }

    fn parse_axis(elem: roxmltree::Node) -> Result<SDFAxis, String> {
        let mut axis = SDFAxis::default();

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "xyz" => {
                    if let Some(text) = child.text() {
                        let parts: Vec<f32> = text
                            .split_whitespace()
                            .filter_map(|s| s.parse().ok())
                            .collect();
                        if parts.len() >= 3 {
                            axis.xyz = Vec3::new(parts[0], parts[1], parts[2]);
                        }
                    }
                }
                "limit" => {
                    for limit_child in child.children().filter(|n| n.is_element()) {
                        if let Some(text) = limit_child.text() {
                            match limit_child.tag_name().name() {
                                "lower" => {
                                    axis.limit_lower = text.trim().parse().unwrap_or(-f32::INFINITY)
                                }
                                "upper" => {
                                    axis.limit_upper = text.trim().parse().unwrap_or(f32::INFINITY)
                                }
                                "effort" => {
                                    axis.limit_effort = text.trim().parse().unwrap_or(f32::INFINITY)
                                }
                                "velocity" => {
                                    axis.limit_velocity =
                                        text.trim().parse().unwrap_or(f32::INFINITY)
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(axis)
    }

    fn parse_pose(elem: roxmltree::Node) -> Result<SDFPose, String> {
        let text = elem.text().ok_or("Empty pose element")?;
        let parts: Vec<f32> = text
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 6 {
            Ok(SDFPose {
                position: Vec3::new(parts[0], parts[1], parts[2]),
                rotation: Vec3::new(parts[3], parts[4], parts[5]),
            })
        } else {
            Ok(SDFPose::default())
        }
    }

    fn parse_inertial(elem: roxmltree::Node) -> Result<SDFInertial, String> {
        let mut inertial = SDFInertial::default();

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "mass" => {
                    if let Some(text) = child.text() {
                        inertial.mass = text.trim().parse().unwrap_or(1.0);
                    }
                }
                _ => {}
            }
        }

        Ok(inertial)
    }

    fn parse_collision(elem: roxmltree::Node) -> Result<SDFCollision, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_collision")
            .to_string();

        let mut collision = SDFCollision {
            name,
            pose: SDFPose::default(),
            geometry: SDFGeometry::Box { size: Vec3::ONE },
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "pose" => collision.pose = Self::parse_pose(child)?,
                "geometry" => {
                    if let Ok(geom) = Self::parse_geometry(child) {
                        collision.geometry = geom;
                    }
                }
                _ => {}
            }
        }

        Ok(collision)
    }

    fn parse_visual(elem: roxmltree::Node) -> Result<SDFVisual, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_visual")
            .to_string();

        let mut visual = SDFVisual {
            name,
            pose: SDFPose::default(),
            geometry: SDFGeometry::Box { size: Vec3::ONE },
            material: SDFMaterial::default(),
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "pose" => visual.pose = Self::parse_pose(child)?,
                "geometry" => {
                    if let Ok(geom) = Self::parse_geometry(child) {
                        visual.geometry = geom;
                    }
                }
                "material" => {
                    if let Ok(mat) = Self::parse_material(child) {
                        visual.material = mat;
                    }
                }
                _ => {}
            }
        }

        Ok(visual)
    }

    fn parse_geometry(elem: roxmltree::Node) -> Result<SDFGeometry, String> {
        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "box" => {
                    if let Some(size_elem) = child.children().find(|n| n.has_tag_name("size")) {
                        if let Some(text) = size_elem.text() {
                            let parts: Vec<f32> = text
                                .split_whitespace()
                                .filter_map(|s| s.parse().ok())
                                .collect();
                            if parts.len() >= 3 {
                                return Ok(SDFGeometry::Box {
                                    size: Vec3::new(parts[0], parts[1], parts[2]),
                                });
                            }
                        }
                    }
                    return Ok(SDFGeometry::Box { size: Vec3::ONE });
                }
                "cylinder" => {
                    let mut radius = 0.5;
                    let mut length = 1.0;
                    for prop in child.children().filter(|n| n.is_element()) {
                        if let Some(text) = prop.text() {
                            match prop.tag_name().name() {
                                "radius" => radius = text.trim().parse().unwrap_or(0.5),
                                "length" => length = text.trim().parse().unwrap_or(1.0),
                                _ => {}
                            }
                        }
                    }
                    return Ok(SDFGeometry::Cylinder { radius, length });
                }
                "sphere" => {
                    let mut radius = 0.5;
                    if let Some(radius_elem) = child.children().find(|n| n.has_tag_name("radius")) {
                        if let Some(text) = radius_elem.text() {
                            radius = text.trim().parse().unwrap_or(0.5);
                        }
                    }
                    return Ok(SDFGeometry::Sphere { radius });
                }
                "mesh" => {
                    let mut uri = String::new();
                    let mut scale = Vec3::ONE;
                    for prop in child.children().filter(|n| n.is_element()) {
                        match prop.tag_name().name() {
                            "uri" => {
                                if let Some(text) = prop.text() {
                                    uri = text.trim().to_string();
                                }
                            }
                            "scale" => {
                                if let Some(text) = prop.text() {
                                    let parts: Vec<f32> = text
                                        .split_whitespace()
                                        .filter_map(|s| s.parse().ok())
                                        .collect();
                                    if parts.len() >= 3 {
                                        scale = Vec3::new(parts[0], parts[1], parts[2]);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    return Ok(SDFGeometry::Mesh { uri, scale });
                }
                _ => {}
            }
        }

        Err("No valid geometry found".to_string())
    }

    fn parse_material(elem: roxmltree::Node) -> Result<SDFMaterial, String> {
        let mut material = SDFMaterial::default();

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "ambient" => {
                    material.ambient = Self::parse_color(child);
                }
                "diffuse" => {
                    material.diffuse = Self::parse_color(child);
                }
                "specular" => {
                    material.specular = Self::parse_color(child);
                }
                "emissive" => {
                    material.emissive = Self::parse_color(child);
                }
                _ => {}
            }
        }

        Ok(material)
    }

    fn parse_color(elem: roxmltree::Node) -> [f32; 4] {
        if let Some(text) = elem.text() {
            let parts: Vec<f32> = text
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 4 {
                return [parts[0], parts[1], parts[2], parts[3]];
            } else if parts.len() == 3 {
                return [parts[0], parts[1], parts[2], 1.0];
            }
        }
        [1.0, 1.0, 1.0, 1.0]
    }

    fn parse_light(elem: roxmltree::Node) -> Result<SDFLight, String> {
        let name = elem
            .attribute("name")
            .unwrap_or("unnamed_light")
            .to_string();
        let light_type = elem.attribute("type").unwrap_or("point").to_string();

        let mut light = SDFLight {
            name,
            light_type,
            pose: SDFPose::default(),
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [1.0, 1.0, 1.0, 1.0],
            attenuation: SDFAttenuation::default(),
        };

        for child in elem.children().filter(|n| n.is_element()) {
            match child.tag_name().name() {
                "pose" => {
                    if let Ok(pose) = Self::parse_pose(child) {
                        light.pose = pose;
                    }
                }
                "diffuse" => {
                    light.diffuse = Self::parse_color(child);
                }
                "specular" => {
                    light.specular = Self::parse_color(child);
                }
                "attenuation" => {
                    if let Ok(atten) = Self::parse_attenuation(child) {
                        light.attenuation = atten;
                    }
                }
                _ => {}
            }
        }

        Ok(light)
    }

    fn parse_attenuation(elem: roxmltree::Node) -> Result<SDFAttenuation, String> {
        let mut attenuation = SDFAttenuation::default();

        for child in elem.children().filter(|n| n.is_element()) {
            if let Some(text) = child.text() {
                match child.tag_name().name() {
                    "range" => attenuation.range = text.trim().parse().unwrap_or(10.0),
                    "constant" => attenuation.constant = text.trim().parse().unwrap_or(1.0),
                    "linear" => attenuation.linear = text.trim().parse().unwrap_or(0.0),
                    "quadratic" => attenuation.quadratic = text.trim().parse().unwrap_or(0.0),
                    _ => {}
                }
            }
        }

        Ok(attenuation)
    }

    fn parse_physics(elem: roxmltree::Node) -> Result<SDFPhysics, String> {
        let mut physics = SDFPhysics::default();

        for child in elem.children().filter(|n| n.is_element()) {
            if let Some(text) = child.text() {
                match child.tag_name().name() {
                    "max_step_size" => physics.max_step_size = text.trim().parse().unwrap_or(0.001),
                    "real_time_factor" => {
                        physics.real_time_factor = text.trim().parse().unwrap_or(1.0)
                    }
                    "real_time_update_rate" => {
                        physics.real_time_update_rate = text.trim().parse().unwrap_or(1000.0)
                    }
                    _ => {}
                }
            }
        }

        Ok(physics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pose() {
        let xml = r#"<pose>1.0 2.0 3.0 0.0 0.0 1.57</pose>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let pose = SDFImporter::parse_pose(doc.root_element()).unwrap();

        assert_eq!(pose.position, Vec3::new(1.0, 2.0, 3.0));
        assert!((pose.rotation.z - 1.57).abs() < 0.01);
    }

    #[test]
    fn test_parse_simple_world() {
        let xml = r#"
        <sdf version="1.6">
            <world name="test_world">
                <gravity>0 0 -9.81</gravity>
            </world>
        </sdf>
        "#;

        let world = SDFImporter::parse_string(xml).unwrap();
        assert_eq!(world.name, "test_world");
        assert_eq!(world.gravity, Vec3::new(0.0, 0.0, -9.81));
    }
}
