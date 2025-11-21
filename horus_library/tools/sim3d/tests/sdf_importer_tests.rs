use sim3d::scene::sdf_importer::{SDFGeometry, SDFImporter};
use std::path::PathBuf;

#[test]
fn test_load_empty_world() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/empty.world");

    let result = SDFImporter::load_file(&test_world);
    assert!(
        result.is_ok(),
        "Failed to load empty.world: {:?}",
        result.err()
    );

    let world = result.unwrap();
    assert_eq!(world.name, "empty_world");
    assert_eq!(world.gravity.z, -9.81);
    assert_eq!(world.models.len(), 1); // ground_plane
    assert_eq!(world.lights.len(), 1); // sun
}

#[test]
fn test_load_simple_robot_world() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let result = SDFImporter::load_file(&test_world);
    assert!(
        result.is_ok(),
        "Failed to load simple_robot.world: {:?}",
        result.err()
    );

    let world = result.unwrap();
    assert_eq!(world.name, "simple_robot_world");
    assert_eq!(world.models.len(), 2); // simple_robot + ground_plane
    assert_eq!(world.lights.len(), 1); // point_light
}

#[test]
fn test_parse_robot_model() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();

    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .expect("Robot model not found");

    assert!(!robot.is_static);
    assert_eq!(robot.links.len(), 4); // base_link, left_wheel, right_wheel, caster
    assert_eq!(robot.joints.len(), 3); // left_wheel_joint, right_wheel_joint, caster_joint
}

#[test]
fn test_parse_links() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let base_link = robot
        .links
        .iter()
        .find(|l| l.name == "base_link")
        .expect("base_link not found");

    assert_eq!(base_link.inertial.mass, 5.0);
    assert_eq!(base_link.collisions.len(), 1);
    assert_eq!(base_link.visuals.len(), 1);
}

#[test]
fn test_parse_joints() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let left_wheel_joint = robot
        .joints
        .iter()
        .find(|j| j.name == "left_wheel_joint")
        .expect("left_wheel_joint not found");

    assert_eq!(left_wheel_joint.joint_type, "revolute");
    assert_eq!(left_wheel_joint.parent, "base_link");
    assert_eq!(left_wheel_joint.child, "left_wheel");
    assert_eq!(left_wheel_joint.axis.limit_effort, 10.0);
    assert_eq!(left_wheel_joint.axis.limit_velocity, 100.0);
}

#[test]
fn test_parse_geometry_box() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let base_link = robot.links.iter().find(|l| l.name == "base_link").unwrap();

    match &base_link.collisions[0].geometry {
        SDFGeometry::Box { size } => {
            assert_eq!(size.x, 0.4);
            assert_eq!(size.y, 0.3);
            assert_eq!(size.z, 0.1);
        }
        _ => panic!("Expected Box geometry"),
    }
}

#[test]
fn test_parse_geometry_cylinder() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let left_wheel = robot.links.iter().find(|l| l.name == "left_wheel").unwrap();

    match &left_wheel.collisions[0].geometry {
        SDFGeometry::Cylinder { radius, length } => {
            assert_eq!(*radius, 0.05);
            assert_eq!(*length, 0.05);
        }
        _ => panic!("Expected Cylinder geometry"),
    }
}

#[test]
fn test_parse_geometry_sphere() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let caster = robot.links.iter().find(|l| l.name == "caster").unwrap();

    match &caster.collisions[0].geometry {
        SDFGeometry::Sphere { radius } => {
            assert_eq!(*radius, 0.025);
        }
        _ => panic!("Expected Sphere geometry"),
    }
}

#[test]
fn test_parse_materials() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();
    let robot = world
        .models
        .iter()
        .find(|m| m.name == "simple_robot")
        .unwrap();

    let base_link = robot.links.iter().find(|l| l.name == "base_link").unwrap();

    let material = &base_link.visuals[0].material;
    // Blue color (0, 0, 1, 1)
    assert_eq!(material.ambient[2], 1.0); // Blue channel
    assert_eq!(material.diffuse[2], 1.0); // Blue channel
}

#[test]
fn test_parse_lights() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/simple_robot.world");

    let world = SDFImporter::load_file(&test_world).unwrap();

    let point_light = world
        .lights
        .iter()
        .find(|l| l.name == "point_light")
        .expect("point_light not found");

    assert_eq!(point_light.light_type, "point");
    assert_eq!(point_light.pose.position.x, 5.0);
    assert_eq!(point_light.pose.position.y, 5.0);
    assert_eq!(point_light.pose.position.z, 5.0);
    assert_eq!(point_light.attenuation.range, 20.0);
}

#[test]
fn test_parse_physics() {
    let test_world =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_worlds/empty.world");

    let world = SDFImporter::load_file(&test_world).unwrap();

    assert_eq!(world.physics.max_step_size, 0.001);
    assert_eq!(world.physics.real_time_factor, 1.0);
    assert_eq!(world.physics.real_time_update_rate, 1000.0);
}

#[test]
fn test_sdf_version_1_6() {
    let xml = r#"
    <sdf version="1.6">
        <world name="test">
            <gravity>0 0 -9.81</gravity>
        </world>
    </sdf>
    "#;

    let result = SDFImporter::parse_string(xml);
    assert!(result.is_ok());
}

#[test]
fn test_sdf_version_1_4() {
    let xml = r#"
    <sdf version="1.4">
        <world name="test">
            <gravity>0 0 -9.81</gravity>
        </world>
    </sdf>
    "#;

    let result = SDFImporter::parse_string(xml);
    assert!(result.is_ok());
}

#[test]
fn test_malformed_xml() {
    let xml = r#"
    <sdf version="1.6">
        <world name="test">
            <gravity>0 0 -9.81</gravity>
        <!-- Missing closing tag
    </sdf>
    "#;

    let result = SDFImporter::parse_string(xml);
    assert!(result.is_err());
}

#[test]
fn test_missing_world_element() {
    let xml = r#"
    <sdf version="1.6">
        <model name="test">
        </model>
    </sdf>
    "#;

    let result = SDFImporter::parse_string(xml);
    assert!(result.is_err());
}

#[test]
fn test_coordinate_conversion() {
    let xml = r#"
    <sdf version="1.6">
        <world name="test">
            <model name="test_model">
                <pose>1 2 3 0 0 1.57</pose>
            </model>
        </world>
    </sdf>
    "#;

    let world = SDFImporter::parse_string(xml).unwrap();
    let model = &world.models[0];

    // SDF pose: position (1, 2, 3), rotation (0, 0, 1.57)
    let transform = model.pose.to_transform();

    // Check that Z-up is converted to Y-up
    // SDF (1, 2, 3) -> Bevy (1, 3, 2)
    assert_eq!(transform.translation.x, 1.0);
    assert_eq!(transform.translation.y, 3.0); // Z becomes Y
    assert_eq!(transform.translation.z, 2.0); // Y becomes Z
}

#[test]
fn test_static_model() {
    let xml = r#"
    <sdf version="1.6">
        <world name="test">
            <model name="static_model">
                <static>true</static>
                <link name="link1" />
            </model>
            <model name="dynamic_model">
                <static>false</static>
                <link name="link2" />
            </model>
        </world>
    </sdf>
    "#;

    let world = SDFImporter::parse_string(xml).unwrap();

    assert!(world.models[0].is_static);
    assert!(!world.models[1].is_static);
}
