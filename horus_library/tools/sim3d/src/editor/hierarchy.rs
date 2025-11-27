//! Scene hierarchy tree view panel

use super::{
    selection::{Selectable, Selection},
    undo::{DeleteOperation, EntitySnapshot, UndoStack},
    EditorState,
};
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use std::collections::HashSet;

/// Resource to store the collapsed state of entities in the hierarchy
#[derive(Resource, Default)]
pub struct HierarchyCollapseState {
    /// Set of entities that are collapsed in the tree view
    collapsed_entities: HashSet<Entity>,
}

impl HierarchyCollapseState {
    pub fn is_collapsed(&self, entity: Entity) -> bool {
        self.collapsed_entities.contains(&entity)
    }

    pub fn toggle(&mut self, entity: Entity) {
        if self.collapsed_entities.contains(&entity) {
            self.collapsed_entities.remove(&entity);
        } else {
            self.collapsed_entities.insert(entity);
        }
    }

    pub fn expand(&mut self, entity: Entity) {
        self.collapsed_entities.remove(&entity);
    }

    pub fn collapse(&mut self, entity: Entity) {
        self.collapsed_entities.insert(entity);
    }

    pub fn expand_all(&mut self) {
        self.collapsed_entities.clear();
    }

    pub fn collapse_all(&mut self, entities_with_children: impl IntoIterator<Item = Entity>) {
        self.collapsed_entities.extend(entities_with_children);
    }
}

/// System to display scene hierarchy panel
pub fn hierarchy_panel_system(
    mut contexts: EguiContexts,
    state: Res<EditorState>,
    mut selection: ResMut<Selection>,
    mut collapse_state: ResMut<HierarchyCollapseState>,
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
                        &mut collapse_state,
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
    collapse_state: &mut HierarchyCollapseState,
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
        let is_collapsed = collapse_state.is_collapsed(entity);

        if has_children {
            let arrow = if is_collapsed { "▶" } else { "▼" };
            if ui.button(arrow).clicked() {
                collapse_state.toggle(entity);
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
                // Duplication will be triggered by an event
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                // Deletion will be triggered by an event
                ui.close_menu();
            }
        });
    });

    // Show children recursively (only if not collapsed)
    if !is_collapsed {
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
                        collapse_state,
                        entities,
                        depth + 1,
                    );
                }
            }
        }
    }
}

/// Event for duplicating an entity
#[derive(Event)]
pub struct DuplicateEntityEvent {
    pub entity: Entity,
}

/// Event for deleting an entity
#[derive(Event)]
pub struct DeleteEntityEvent {
    pub entity: Entity,
}

/// System to handle entity duplication
pub fn duplicate_entity_system(
    mut commands: Commands,
    mut events: EventReader<DuplicateEntityEvent>,
    query: Query<(
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&Name>,
        Option<&Selectable>,
        Option<&Visibility>,
        Option<&Children>,
    )>,
) {
    for event in events.read() {
        if let Ok((transform, _global, name, selectable, visibility, children)) =
            query.get(event.entity)
        {
            // Create the new entity with duplicated components
            let mut entity_commands = commands.spawn_empty();

            // Duplicate transform
            if let Some(t) = transform {
                let mut new_transform = *t;
                // Offset the position slightly so it's visible
                new_transform.translation.x += 1.0;
                entity_commands.insert(new_transform);
            }

            // Duplicate name (with suffix)
            if let Some(n) = name {
                entity_commands.insert(Name::new(format!("{} (Copy)", n)));
            }

            // Duplicate selectable
            if let Some(s) = selectable {
                entity_commands.insert(Selectable::new(&format!("{} (Copy)", s.name)));
            }

            // Duplicate visibility
            if let Some(v) = visibility {
                entity_commands.insert(*v);
            }

            let new_entity = entity_commands.id();

            // Recursively duplicate children
            if let Some(children) = children {
                for &child in children.iter() {
                    duplicate_entity_recursive(&mut commands, child, new_entity, &query);
                }
            }

            info!("Duplicated entity {:?} as {:?}", event.entity, new_entity);
        }
    }
}

/// Recursively duplicate an entity and its children
fn duplicate_entity_recursive(
    commands: &mut Commands,
    source: Entity,
    new_parent: Entity,
    query: &Query<(
        Option<&Transform>,
        Option<&GlobalTransform>,
        Option<&Name>,
        Option<&Selectable>,
        Option<&Visibility>,
        Option<&Children>,
    )>,
) {
    if let Ok((transform, _global, name, selectable, visibility, children)) = query.get(source) {
        let mut entity_commands = commands.spawn_empty();

        // Copy components
        if let Some(t) = transform {
            entity_commands.insert(*t);
        }
        if let Some(n) = name {
            entity_commands.insert(n.clone());
        }
        if let Some(s) = selectable {
            entity_commands.insert(s.clone());
        }
        if let Some(v) = visibility {
            entity_commands.insert(*v);
        }

        let new_entity = entity_commands.id();

        // Set parent
        entity_commands.set_parent(new_parent);

        // Recursively duplicate children
        if let Some(children) = children {
            for &child in children.iter() {
                duplicate_entity_recursive(commands, child, new_entity, query);
            }
        }
    }
}

/// System to handle entity deletion with undo support
pub fn delete_entity_system(
    mut commands: Commands,
    mut events: EventReader<DeleteEntityEvent>,
    mut undo_stack: ResMut<UndoStack>,
    world: &World,
    query: Query<&Children>,
) {
    for event in events.read() {
        // Create delete operation with snapshot for undo
        let operation = DeleteOperation::new(event.entity, world);

        // Execute the deletion
        despawn_recursive(&mut commands, event.entity, &query);

        // Add to undo stack
        undo_stack.push(Box::new(operation));

        info!("Deleted entity {:?}", event.entity);
    }
}

/// Recursively despawn an entity and all its children
fn despawn_recursive(commands: &mut Commands, entity: Entity, children_query: &Query<&Children>) {
    if let Ok(children) = children_query.get(entity) {
        for &child in children.iter() {
            despawn_recursive(commands, child, children_query);
        }
    }
    commands.entity(entity).despawn();
}

/// Updated show_entity_tree to send events for duplication and deletion
fn show_entity_tree_with_events(
    ui: &mut egui::Ui,
    entity: Entity,
    name: Option<&Name>,
    selectable_opt: Option<&Selectable>,
    children_opt: Option<&Children>,
    selection: &mut Selection,
    collapse_state: &mut HierarchyCollapseState,
    entities: &Query<(
        Entity,
        Option<&Name>,
        Option<&Selectable>,
        Option<&Children>,
    )>,
    depth: usize,
    duplicate_events: &mut EventWriter<DuplicateEntityEvent>,
    delete_events: &mut EventWriter<DeleteEntityEvent>,
) {
    let indent = "  ".repeat(depth);
    let display_name = name
        .map(|n| n.as_str())
        .or_else(|| selectable_opt.map(|s| s.name.as_str()))
        .unwrap_or("Entity");

    let is_selected = selection.is_selected(entity);
    let is_collapsed = collapse_state.is_collapsed(entity);
    let has_children = children_opt.map_or(false, |c| !c.is_empty());

    ui.horizontal(|ui| {
        ui.label(&indent);

        // Collapse/expand button for entities with children
        if has_children {
            let arrow = if is_collapsed { "▶" } else { "▼" };
            if ui.button(arrow).clicked() {
                collapse_state.toggle(entity);
            }
        } else {
            ui.add_space(20.0);
        }

        // Entity label
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
                duplicate_events.send(DuplicateEntityEvent { entity });
                ui.close_menu();
            }
            if ui.button("Delete").clicked() {
                delete_events.send(DeleteEntityEvent { entity });
                ui.close_menu();
            }
        });
    });

    // Show children recursively (only if not collapsed)
    if !is_collapsed {
        if let Some(children) = children_opt {
            for &child in children.iter() {
                if let Ok((child_entity, child_name, child_selectable, child_children)) =
                    entities.get(child)
                {
                    show_entity_tree_with_events(
                        ui,
                        child_entity,
                        child_name,
                        child_selectable,
                        child_children,
                        selection,
                        collapse_state,
                        entities,
                        depth + 1,
                        duplicate_events,
                        delete_events,
                    );
                }
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
