//! Entity selection system for the editor

use crate::physics::world::PhysicsWorld;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, Window};
use rapier3d::prelude::*;
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
    mut physics_world: ResMut<PhysicsWorld>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    selectable_query: Query<Entity, With<Selectable>>,
    selected_query: Query<Entity, With<Selected>>,
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

    // Implement mouse picking with raycasting
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Some(picked_entity) = perform_mouse_picking(
            &windows,
            &camera_query,
            &mut physics_world,
            &selectable_query,
        ) {
            // Handle selection modifiers
            if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
                // Add to selection
                selection.add(picked_entity);
            } else if keyboard.pressed(KeyCode::ControlLeft)
                || keyboard.pressed(KeyCode::ControlRight)
            {
                // Toggle selection
                selection.toggle(picked_entity);
            } else {
                // Replace selection
                selection.select(picked_entity);
            }
        } else if !keyboard.pressed(KeyCode::ShiftLeft) && !keyboard.pressed(KeyCode::ControlLeft) {
            // Clear selection if clicking on empty space (and not holding modifiers)
            selection.clear();
        }
    }
}

/// Perform raycasting from mouse position to find selectable entity
fn perform_mouse_picking(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_query: &Query<(&Camera, &GlobalTransform)>,
    physics_world: &mut PhysicsWorld,
    selectable_query: &Query<Entity, With<Selectable>>,
) -> Option<Entity> {
    // Get the primary window
    let window = windows.get_single().ok()?;

    // Get mouse position
    let cursor_position = window.cursor_position()?;

    // Find the primary camera
    let (camera, camera_transform) = camera_query.iter().next()?;

    // Convert screen coordinates to world ray
    let ray = screen_to_world_ray(
        cursor_position,
        camera,
        camera_transform,
        window.resolution.width(),
        window.resolution.height(),
    )?;

    // Perform raycast
    if let Some((handle, _toi)) = physics_world.query_pipeline.cast_ray(
        &physics_world.rigid_body_set,
        &physics_world.collider_set,
        &ray,
        1000.0, // Max distance
        true,
        QueryFilter::default().exclude_sensors(),
    ) {
        // Get the collider that was hit
        if let Some(collider) = physics_world.collider_set.get(handle) {
            // Get the rigid body associated with this collider
            if let Some(parent_handle) = collider.parent() {
                // Get the entity from the rigid body
                if let Some(entity) = physics_world.get_entity_from_handle(parent_handle) {
                    // Check if entity is selectable
                    if selectable_query.contains(entity) {
                        return Some(entity);
                    }
                }
            }
        }
    }

    None
}

/// Convert screen coordinates to a world space ray
fn screen_to_world_ray(
    cursor_position: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window_width: f32,
    window_height: f32,
) -> Option<Ray> {
    // Get the camera's projection matrix inverse
    let projection_matrix = camera.clip_from_view();
    let inverse_projection = projection_matrix.inverse();

    // Convert screen coordinates to normalized device coordinates (-1 to 1)
    let ndc_x = (2.0 * cursor_position.x / window_width) - 1.0;
    let ndc_y = 1.0 - (2.0 * cursor_position.y / window_height); // Flip Y

    // Convert NDC to view space
    let ndc_near = Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let ndc_far = Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

    let view_near = inverse_projection * ndc_near;
    let view_far = inverse_projection * ndc_far;

    // Perspective divide
    let view_near = view_near.truncate() / view_near.w;
    let view_far = view_far.truncate() / view_far.w;

    // Convert view space to world space
    let world_near = camera_transform.transform_point(view_near);
    let world_far = camera_transform.transform_point(view_far);

    // Create ray from camera position to far point
    let ray_origin = point![world_near.x, world_near.y, world_near.z];
    let ray_direction = (world_far - world_near).normalize();

    Some(Ray::new(
        ray_origin,
        vector![ray_direction.x, ray_direction.y, ray_direction.z],
    ))
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
