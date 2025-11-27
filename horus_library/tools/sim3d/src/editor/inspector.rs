//! Entity inspector panel for viewing and editing component properties

use super::{
    selection::Selection,
    undo::{TransformOperation, UndoStack},
    EditorState,
};
use crate::physics::rigid_body::Velocity;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// System to display entity inspector panel
pub fn inspector_panel_system(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    selection: Res<Selection>,
    mut undo_stack: ResMut<UndoStack>,
    mut transforms: Query<&mut Transform>,
    names: Query<&Name>,
    velocities: Query<&Velocity>,
    mass_properties: Query<&ReadMassProperties>,
    mut visibilities: Query<&mut Visibility>,
    material_query: Query<(&Handle<StandardMaterial>,)>,
    materials: Res<Assets<StandardMaterial>>,
    mesh_query: Query<&Handle<Mesh>>,
    point_lights: Query<&PointLight>,
    directional_lights: Query<&DirectionalLight>,
    spot_lights: Query<&SpotLight>,
    cameras: Query<&Camera>,
) {
    if !state.show_inspector {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::right("inspector_panel")
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.heading("Inspector");
            ui.separator();

            if selection.is_empty() {
                ui.label("No entity selected");
                return;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Show primary selection
                if let Some(entity) = selection.primary {
                    show_entity_inspector(
                        ui,
                        entity,
                        &mut transforms,
                        &names,
                        &velocities,
                        &mut undo_stack,
                        &mass_properties,
                        &mut visibilities,
                        &material_query,
                        &materials,
                        &mesh_query,
                        &point_lights,
                        &directional_lights,
                        &spot_lights,
                        &cameras,
                    );
                }

                // Show multi-selection info
                if selection.count() > 1 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.label(format!("{} entities selected", selection.count()));
                }
            });
        });
}

/// Display inspector UI for a single entity
fn show_entity_inspector(
    ui: &mut egui::Ui,
    entity: Entity,
    transforms: &mut Query<&mut Transform>,
    names: &Query<&Name>,
    velocities: &Query<&Velocity>,
    undo_stack: &mut UndoStack,
    mass_properties: &Query<&ReadMassProperties>,
    visibilities: &mut Query<&mut Visibility>,
    material_query: &Query<(&Handle<StandardMaterial>,)>,
    materials: &Res<Assets<StandardMaterial>>,
    mesh_query: &Query<&Handle<Mesh>>,
    point_lights: &Query<&PointLight>,
    directional_lights: &Query<&DirectionalLight>,
    spot_lights: &Query<&SpotLight>,
    cameras: &Query<&Camera>,
) {
    // Entity header
    ui.heading(format!("Entity {:?}", entity.index()));

    // Name component
    if let Ok(name) = names.get(entity) {
        ui.label(format!("Name: {}", name.as_str()));
    }

    ui.add_space(10.0);

    // Transform component
    if let Ok(mut transform) = transforms.get_mut(entity) {
        ui.collapsing("Transform", |ui| {
            ui.label("Translation:");
            let mut translation = transform.translation;
            let old_translation = translation;

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut translation.x).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut translation.y).speed(0.1));
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut translation.z).speed(0.1));
            });

            if translation != old_translation {
                let old_transform = *transform;
                transform.translation = translation;
                // Push transform change to undo stack
                let operation = TransformOperation::new(entity, old_transform, *transform);
                undo_stack.push(Box::new(operation));
            }

            ui.add_space(5.0);

            // Rotation (Euler angles)
            ui.label("Rotation (deg):");
            let (mut x, mut y, mut z) = transform.rotation.to_euler(EulerRot::XYZ);
            x = x.to_degrees();
            y = y.to_degrees();
            z = z.to_degrees();
            let old_rotation = (x, y, z);

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut x).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut y).speed(1.0));
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut z).speed(1.0));
            });

            if (x, y, z) != old_rotation {
                transform.rotation = Quat::from_euler(
                    EulerRot::XYZ,
                    x.to_radians(),
                    y.to_radians(),
                    z.to_radians(),
                );
            }

            ui.add_space(5.0);

            // Scale
            ui.label("Scale:");
            let mut scale = transform.scale;
            let old_scale = scale;

            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(
                    egui::DragValue::new(&mut scale.x)
                        .speed(0.01)
                        .clamp_range(0.01..=100.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Y:");
                ui.add(
                    egui::DragValue::new(&mut scale.y)
                        .speed(0.01)
                        .clamp_range(0.01..=100.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Z:");
                ui.add(
                    egui::DragValue::new(&mut scale.z)
                        .speed(0.01)
                        .clamp_range(0.01..=100.0),
                );
            });

            if scale != old_scale {
                transform.scale = scale;
            }

            // Reset button
            if ui.button("Reset Transform").clicked() {
                *transform = Transform::IDENTITY;
            }
        });
    }

    ui.add_space(10.0);

    // Velocity component (read-only)
    if let Ok(velocity) = velocities.get(entity) {
        ui.collapsing("Velocity (Read-Only)", |ui| {
            ui.label(format!(
                "Linear: [{:.2}, {:.2}, {:.2}]",
                velocity.linear.x, velocity.linear.y, velocity.linear.z
            ));
            ui.label(format!(
                "Angular: [{:.2}, {:.2}, {:.2}]",
                velocity.angular.x, velocity.angular.y, velocity.angular.z
            ));
        });
    }

    // Additional component inspectors
    ui.add_space(10.0);

    // Mass properties component
    if let Ok(mass_props) = mass_properties.get(entity) {
        ui.collapsing("Mass Properties", |ui| {
            ui.label(format!("Mass: {:.2} kg", mass_props.mass()));

            if let Some(center) = mass_props.local_center_of_mass() {
                ui.label(format!(
                    "Center of Mass: [{:.2}, {:.2}, {:.2}]",
                    center.x, center.y, center.z
                ));
            }

            ui.label(format!(
                "Principal Inertia: [{:.3}, {:.3}, {:.3}]",
                mass_props.principal_inertia().x,
                mass_props.principal_inertia().y,
                mass_props.principal_inertia().z
            ));
        });
    }

    // Visibility component
    if let Ok(mut visibility) = visibilities.get_mut(entity) {
        ui.collapsing("Visibility", |ui| {
            let mut is_visible = visibility.is_visible();
            if ui.checkbox(&mut is_visible, "Visible").changed() {
                *visibility = if is_visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        });
    }

    // Material component (if using StandardMaterial)
    if let Ok((material_handle, materials)) = material_query.get(entity) {
        if let Some(material) = materials.get(material_handle) {
            ui.collapsing("Material", |ui| {
                ui.label("Base Color:");
                let color = material.base_color.to_srgba();
                ui.label(format!(
                    "RGBA: [{:.2}, {:.2}, {:.2}, {:.2}]",
                    color.red, color.green, color.blue, color.alpha
                ));

                ui.label(format!("Metallic: {:.2}", material.metallic));
                ui.label(format!(
                    "Perceptual Roughness: {:.2}",
                    material.perceptual_roughness
                ));
                ui.label(format!("Reflectance: {:.2}", material.reflectance));

                if material.emissive != LinearRgba::BLACK {
                    let emissive = material.emissive;
                    ui.label(format!(
                        "Emissive: [{:.2}, {:.2}, {:.2}, {:.2}]",
                        emissive.red, emissive.green, emissive.blue, emissive.alpha
                    ));
                    ui.label(format!(
                        "Emissive Exposure: {:.2}",
                        material.emissive_exposure_weight
                    ));
                }

                ui.label(format!("Double Sided: {}", material.double_sided));
                ui.label(format!("Unlit: {}", material.unlit));
                ui.label(format!("Alpha Mode: {:?}", material.alpha_mode));
            });
        }
    }

    // Mesh component info
    if let Ok(mesh_handle) = mesh_query.get(entity) {
        ui.collapsing("Mesh", |ui| {
            ui.label(format!("Mesh Handle: {:?}", mesh_handle));
            // Additional mesh info would require access to mesh assets
        });
    }

    // Light components
    if let Ok(point_light) = point_lights.get(entity) {
        ui.collapsing("Point Light", |ui| {
            ui.label(format!("Intensity: {:.1} lumens", point_light.intensity));
            ui.label(format!("Range: {:.1} m", point_light.range));
            ui.label(format!("Radius: {:.2} m", point_light.radius));

            let color = point_light.color.to_srgba();
            ui.label(format!(
                "Color: [{:.2}, {:.2}, {:.2}]",
                color.red, color.green, color.blue
            ));

            ui.label(format!("Shadows: {}", point_light.shadows_enabled));
        });
    }

    if let Ok(directional_light) = directional_lights.get(entity) {
        ui.collapsing("Directional Light", |ui| {
            ui.label(format!(
                "Illuminance: {:.1} lux",
                directional_light.illuminance
            ));

            let color = directional_light.color.to_srgba();
            ui.label(format!(
                "Color: [{:.2}, {:.2}, {:.2}]",
                color.red, color.green, color.blue
            ));

            ui.label(format!("Shadows: {}", directional_light.shadows_enabled));
        });
    }

    if let Ok(spot_light) = spot_lights.get(entity) {
        ui.collapsing("Spot Light", |ui| {
            ui.label(format!("Intensity: {:.1} lumens", spot_light.intensity));
            ui.label(format!("Range: {:.1} m", spot_light.range));
            ui.label(format!("Radius: {:.2} m", spot_light.radius));

            ui.label(format!(
                "Inner Angle: {:.1}°",
                spot_light.inner_angle.to_degrees()
            ));
            ui.label(format!(
                "Outer Angle: {:.1}°",
                spot_light.outer_angle.to_degrees()
            ));

            let color = spot_light.color.to_srgba();
            ui.label(format!(
                "Color: [{:.2}, {:.2}, {:.2}]",
                color.red, color.green, color.blue
            ));

            ui.label(format!("Shadows: {}", spot_light.shadows_enabled));
        });
    }

    // Camera component
    if let Ok(camera) = cameras.get(entity) {
        ui.collapsing("Camera", |ui| {
            ui.label(format!("Is Active: {}", camera.is_active));
            ui.label(format!("Order: {}", camera.order));

            if let Some(viewport) = &camera.viewport {
                ui.label(format!(
                    "Viewport: [{}, {}] - [{}, {}]",
                    viewport.physical_position.x,
                    viewport.physical_position.y,
                    viewport.physical_size.x,
                    viewport.physical_size.y
                ));
            }

            ui.label(format!("HDR: {}", camera.hdr));

            // Clear color
            match &camera.clear_color {
                ClearColorConfig::Default => ui.label("Clear: Default"),
                ClearColorConfig::Custom(color) => {
                    let srgba = color.to_srgba();
                    ui.label(format!(
                        "Clear Color: [{:.2}, {:.2}, {:.2}, {:.2}]",
                        srgba.red, srgba.green, srgba.blue, srgba.alpha
                    ));
                }
                ClearColorConfig::None => ui.label("Clear: None"),
            };
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_editing() {
        let mut world = World::new();
        let entity = world.spawn(Transform::IDENTITY).id();

        let mut transforms = world.query::<&mut Transform>();
        if let Ok(mut transform) = transforms.get_mut(&mut world, entity) {
            transform.translation.x = 5.0;
            assert_eq!(transform.translation.x, 5.0);
        }
    }
}
