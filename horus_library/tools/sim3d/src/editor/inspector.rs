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
    _undo_stack: &mut UndoStack,
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
                transform.translation = translation;
                // TODO: Push to undo stack
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

    // TODO: Add more component inspectors as needed
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
