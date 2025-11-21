//! Scene hierarchy tree view panel

use super::{
    selection::{Selectable, Selection},
    EditorState,
};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

/// System to display scene hierarchy panel
pub fn hierarchy_panel_system(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    mut selection: ResMut<Selection>,
    entities: Query<(
        Entity,
        Option<&Name>,
        Option<&Selectable>,
        Option<&Children>,
    )>,
    parents: Query<&Parent>,
) {
    if !state.show_hierarchy {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::SidePanel::left("hierarchy_panel")
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading("Scene Hierarchy");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Show root entities (those without parents)
                for (entity, name_opt, selectable_opt, children_opt) in entities.iter() {
                    if parents.get(entity).is_ok() {
                        continue; // Skip entities with parents
                    }

                    show_entity_tree(
                        ui,
                        entity,
                        name_opt,
                        selectable_opt,
                        children_opt,
                        &mut selection,
                        &entities,
                        0,
                    );
                }
            });
        });
}

/// Recursively show entity and its children in tree view
fn show_entity_tree(
    ui: &mut egui::Ui,
    entity: Entity,
    name_opt: Option<&Name>,
    selectable_opt: Option<&Selectable>,
    children_opt: Option<&Children>,
    selection: &mut Selection,
    entities: &Query<(
        Entity,
        Option<&Name>,
        Option<&Selectable>,
        Option<&Children>,
    )>,
    depth: usize,
) {
    let indent = depth as f32 * 16.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        // Show expand/collapse arrow if has children
        let has_children = children_opt.map_or(false, |c| !c.is_empty());
        if has_children {
            if ui.button(">").clicked() {
                // TODO: Store collapse state
            }
        } else {
            ui.add_space(20.0); // Space for alignment
        }

        // Get display name
        let display_name = if let Some(selectable) = selectable_opt {
            &selectable.name
        } else if let Some(name) = name_opt {
            name.as_str()
        } else {
            "Entity"
        };

        // Show selectable button
        let is_selected = selection.is_selected(entity);
        if ui
            .selectable_label(
                is_selected,
                format!("{} [{:?}]", display_name, entity.index()),
            )
            .clicked()
        {
            if ui.input(|i| i.modifiers.shift) {
                selection.add(entity);
            } else if ui.input(|i| i.modifiers.ctrl) {
                selection.toggle(entity);
            } else {
                selection.select(entity);
            }
        }

        // Context menu
        ui.menu_button("⋮", |ui| {
            if ui.button("Duplicate").clicked() {
                // TODO: Implement duplication
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                // TODO: Implement deletion
                ui.close_menu();
            }
        });
    });

    // Show children recursively
    if let Some(children) = children_opt {
        for &child in children.iter() {
            if let Ok((child_entity, child_name, child_selectable, child_children)) =
                entities.get(child)
            {
                show_entity_tree(
                    ui,
                    child_entity,
                    child_name,
                    child_selectable,
                    child_children,
                    selection,
                    entities,
                    depth + 1,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hierarchy_entities() {
        let mut world = World::new();

        // Create parent-child hierarchy
        let parent = world
            .spawn((Name::new("Parent"), Selectable::new("Parent")))
            .id();

        let child = world
            .spawn((Name::new("Child"), Selectable::new("Child")))
            .id();

        world.entity_mut(parent).add_child(child);

        // Verify hierarchy
        let parent_entity = world.entity(parent);
        assert!(parent_entity.contains::<Children>());
    }
}
