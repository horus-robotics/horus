//! Entity selection system for the editor

use bevy::prelude::*;
use std::collections::HashSet;

/// Marker component for selectable entities
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct Selectable {
    /// Display name in the editor
    pub name: String,
    /// Whether this entity can be selected
    pub enabled: bool,
}

impl Selectable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Marker component for currently selected entities
#[derive(Component)]
pub struct Selected;

/// Resource tracking current selection
#[derive(Resource, Default)]
pub struct Selection {
    /// Set of selected entity IDs
    pub entities: HashSet<Entity>,
    /// Primary selected entity (last selected)
    pub primary: Option<Entity>,
}

impl Selection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Select an entity (replace current selection)
    pub fn select(&mut self, entity: Entity) {
        self.clear();
        self.add(entity);
    }

    /// Add entity to selection (multi-select)
    pub fn add(&mut self, entity: Entity) {
        self.entities.insert(entity);
        self.primary = Some(entity);
    }

    /// Remove entity from selection
    pub fn remove(&mut self, entity: Entity) {
        self.entities.remove(&entity);
        if self.primary == Some(entity) {
            self.primary = self.entities.iter().next().copied();
        }
    }

    /// Toggle entity selection
    pub fn toggle(&mut self, entity: Entity) {
        if self.entities.contains(&entity) {
            self.remove(entity);
        } else {
            self.add(entity);
        }
    }

    /// Clear all selection
    pub fn clear(&mut self) {
        self.entities.clear();
        self.primary = None;
    }

    /// Check if entity is selected
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    /// Get number of selected entities
    pub fn count(&self) -> usize {
        self.entities.len()
    }

    /// Check if selection is empty
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Get all selected entities
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter()
    }
}

/// Selection event
#[derive(Event)]
pub enum SelectionEvent {
    Selected(Entity),
    Deselected(Entity),
    Cleared,
}

/// System to handle selection via mouse clicks
pub fn selection_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selection: ResMut<Selection>,
    mut commands: Commands,
    selectable_query: Query<Entity, With<Selectable>>,
    selected_query: Query<Entity, With<Selected>>,
    // TODO: Add raycasting for mouse picking
) {
    // Update Selected components to match Selection resource
    for entity in selected_query.iter() {
        if !selection.is_selected(entity) {
            commands.entity(entity).remove::<Selected>();
        }
    }

    for &entity in selection.iter() {
        if !selected_query.contains(entity) {
            commands.entity(entity).insert(Selected);
        }
    }

    // Handle keyboard shortcuts
    if keyboard.just_pressed(KeyCode::Escape) {
        selection.clear();
    }

    // Select all with Ctrl+A
    if keyboard.pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::KeyA) {
        for entity in selectable_query.iter() {
            selection.add(entity);
        }
    }

    // TODO: Implement mouse picking with raycasting
    // This would require:
    // 1. Camera and viewport info
    // 2. Raycast into scene
    // 3. Find hit entity
    // 4. Handle Shift/Ctrl for multi-select
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_single() {
        let mut selection = Selection::new();
        let entity = Entity::from_raw(1);

        selection.select(entity);
        assert!(selection.is_selected(entity));
        assert_eq!(selection.count(), 1);
        assert_eq!(selection.primary, Some(entity));
    }

    #[test]
    fn test_selection_multi() {
        let mut selection = Selection::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        selection.add(entity1);
        selection.add(entity2);

        assert!(selection.is_selected(entity1));
        assert!(selection.is_selected(entity2));
        assert_eq!(selection.count(), 2);
        assert_eq!(selection.primary, Some(entity2));
    }

    #[test]
    fn test_selection_toggle() {
        let mut selection = Selection::new();
        let entity = Entity::from_raw(1);

        selection.toggle(entity);
        assert!(selection.is_selected(entity));

        selection.toggle(entity);
        assert!(!selection.is_selected(entity));
    }

    #[test]
    fn test_selection_remove() {
        let mut selection = Selection::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        selection.add(entity1);
        selection.add(entity2);
        selection.remove(entity1);

        assert!(!selection.is_selected(entity1));
        assert!(selection.is_selected(entity2));
        assert_eq!(selection.primary, Some(entity2));
    }

    #[test]
    fn test_selection_clear() {
        let mut selection = Selection::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        selection.add(entity1);
        selection.add(entity2);
        selection.clear();

        assert!(selection.is_empty());
        assert_eq!(selection.primary, None);
    }

    #[test]
    fn test_selection_replace() {
        let mut selection = Selection::new();
        let entity1 = Entity::from_raw(1);
        let entity2 = Entity::from_raw(2);

        selection.add(entity1);
        selection.select(entity2); // Should replace

        assert!(!selection.is_selected(entity1));
        assert!(selection.is_selected(entity2));
        assert_eq!(selection.count(), 1);
    }

    #[test]
    fn test_selectable_component() {
        let selectable = Selectable::new("TestEntity");
        assert_eq!(selectable.name, "TestEntity");
        assert!(selectable.enabled);

        let disabled = Selectable::new("Disabled").with_enabled(false);
        assert!(!disabled.enabled);
    }
}
