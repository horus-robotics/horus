//! Interactive world editor for sim2d
//!
//! Provides GUI-based world creation without YAML editing.
//! Supports adding/removing obstacles, modifying properties, and saving worlds.

use crate::{Obstacle, ObstacleShape};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Editor tool selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    /// Select and move objects
    #[default]
    Select,
    /// Create rectangular obstacles
    Rectangle,
    /// Create circular obstacles
    Circle,
    /// Delete objects
    Delete,
}

/// State for obstacle being created
#[derive(Debug, Clone)]
pub struct ObstacleInProgress {
    /// Starting position
    pub start_pos: Vec2,
    /// Current position (for size calculation)
    pub current_pos: Vec2,
    /// Shape type
    pub shape: ObstacleShape,
}

/// World editor state
#[derive(Resource, Default)]
pub struct WorldEditor {
    /// Currently active tool
    pub active_tool: EditorTool,

    /// Selected obstacle entities
    pub selected_obstacles: Vec<Entity>,

    /// Enable grid snapping
    pub grid_snap: bool,

    /// Grid size in meters
    pub grid_size: f32,

    /// Obstacle being created
    pub obstacle_in_progress: Option<ObstacleInProgress>,

    /// Mouse position in world coordinates
    pub mouse_world_pos: Vec2,

    /// Is mouse button pressed
    pub mouse_pressed: bool,

    /// Editor enabled
    pub enabled: bool,

    /// Undo stack
    pub undo_stack: Vec<EditorAction>,

    /// Redo stack
    pub redo_stack: Vec<EditorAction>,
}

/// Editor action for undo/redo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorAction {
    /// Obstacle was added
    AddObstacle {
        obstacle: Obstacle,
        entity: Option<Entity>,
    },
    /// Obstacle was removed
    RemoveObstacle { obstacle: Obstacle, entity: Entity },
    /// Obstacle was moved
    MoveObstacle {
        entity: Entity,
        old_pos: [f32; 2],
        new_pos: [f32; 2],
    },
    /// Obstacle was modified
    ModifyObstacle {
        entity: Entity,
        old_obstacle: Obstacle,
        new_obstacle: Obstacle,
    },
}

impl WorldEditor {
    /// Create a new world editor
    pub fn new() -> Self {
        Self {
            active_tool: EditorTool::Select,
            selected_obstacles: Vec::new(),
            grid_snap: true,
            grid_size: 0.5,
            obstacle_in_progress: None,
            mouse_world_pos: Vec2::ZERO,
            mouse_pressed: false,
            enabled: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Snap position to grid if enabled
    pub fn snap_to_grid(&self, pos: Vec2) -> Vec2 {
        if !self.grid_snap {
            return pos;
        }

        Vec2::new(
            (pos.x / self.grid_size).round() * self.grid_size,
            (pos.y / self.grid_size).round() * self.grid_size,
        )
    }

    /// Handle mouse press at position
    pub fn handle_mouse_press(&mut self, world_pos: Vec2) {
        self.mouse_pressed = true;
        let snapped_pos = self.snap_to_grid(world_pos);

        match self.active_tool {
            EditorTool::Rectangle => {
                self.obstacle_in_progress = Some(ObstacleInProgress {
                    start_pos: snapped_pos,
                    current_pos: snapped_pos,
                    shape: ObstacleShape::Rectangle,
                });
            }
            EditorTool::Circle => {
                self.obstacle_in_progress = Some(ObstacleInProgress {
                    start_pos: snapped_pos,
                    current_pos: snapped_pos,
                    shape: ObstacleShape::Circle,
                });
            }
            EditorTool::Select => {
                // Select mode: store click position for hit testing
                self.mouse_world_pos = snapped_pos;
            }
            EditorTool::Delete => {
                // Delete mode: store click position for hit testing
                self.mouse_world_pos = snapped_pos;
            }
        }
    }

    /// Handle mouse drag
    pub fn handle_mouse_drag(&mut self, world_pos: Vec2) {
        if !self.mouse_pressed {
            return;
        }

        let snapped_pos = self.snap_to_grid(world_pos);

        if let Some(ref mut in_progress) = self.obstacle_in_progress {
            in_progress.current_pos = snapped_pos;
        }
    }

    /// Handle mouse release
    pub fn handle_mouse_release(&mut self) -> Option<Obstacle> {
        self.mouse_pressed = false;

        if let Some(in_progress) = self.obstacle_in_progress.take() {
            // Calculate final obstacle properties
            let center = (in_progress.start_pos + in_progress.current_pos) / 2.0;
            let size = (in_progress.current_pos - in_progress.start_pos).abs();

            // Minimum size check
            if size.x < 0.1 || size.y < 0.1 {
                return None;
            }

            let obstacle = Obstacle {
                pos: [center.x, center.y],
                size: [size.x, size.y],
                shape: in_progress.shape,
                color: Some([0.6, 0.6, 0.6]), // Default gray
            };

            return Some(obstacle);
        }

        None
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selected_obstacles.clear();
    }

    /// Select obstacle
    pub fn select_obstacle(&mut self, entity: Entity) {
        if !self.selected_obstacles.contains(&entity) {
            self.selected_obstacles.push(entity);
        }
    }

    /// Deselect obstacle
    pub fn deselect_obstacle(&mut self, entity: Entity) {
        self.selected_obstacles.retain(|&e| e != entity);
    }

    /// Check if obstacle is selected
    pub fn is_selected(&self, entity: Entity) -> bool {
        self.selected_obstacles.contains(&entity)
    }

    /// Get preview obstacle (for rendering)
    pub fn get_preview_obstacle(&self) -> Option<Obstacle> {
        if let Some(ref in_progress) = self.obstacle_in_progress {
            let center = (in_progress.start_pos + in_progress.current_pos) / 2.0;
            let size = (in_progress.current_pos - in_progress.start_pos).abs();

            Some(Obstacle {
                pos: [center.x, center.y],
                size: [size.x.max(0.1), size.y.max(0.1)],
                shape: in_progress.shape.clone(),
                color: Some([0.5, 0.8, 0.5]), // Green preview
            })
        } else {
            None
        }
    }

    /// Check if a point is inside an obstacle
    pub fn point_in_obstacle(pos: Vec2, obstacle: &Obstacle) -> bool {
        match obstacle.shape {
            ObstacleShape::Rectangle => {
                let half_w = obstacle.size[0] / 2.0;
                let half_h = obstacle.size[1] / 2.0;
                pos.x >= obstacle.pos[0] - half_w
                    && pos.x <= obstacle.pos[0] + half_w
                    && pos.y >= obstacle.pos[1] - half_h
                    && pos.y <= obstacle.pos[1] + half_h
            }
            ObstacleShape::Circle => {
                let dx = pos.x - obstacle.pos[0];
                let dy = pos.y - obstacle.pos[1];
                let radius = obstacle.size[0] / 2.0;
                (dx * dx + dy * dy) <= radius * radius
            }
        }
    }

    /// Find obstacle index at a position
    pub fn find_obstacle_at(&self, pos: Vec2, obstacles: &[Obstacle]) -> Option<usize> {
        obstacles
            .iter()
            .enumerate()
            .find(|(_, obs)| Self::point_in_obstacle(pos, obs))
            .map(|(idx, _)| idx)
    }

    /// Handle select action at current mouse position
    pub fn try_select_at(&mut self, pos: Vec2, obstacles: &[Obstacle], entities: &[Entity]) -> Option<Entity> {
        if let Some(idx) = self.find_obstacle_at(pos, obstacles) {
            if idx < entities.len() {
                let entity = entities[idx];
                self.clear_selection();
                self.select_obstacle(entity);
                return Some(entity);
            }
        }
        self.clear_selection();
        None
    }

    /// Handle delete action at current mouse position, returns index of deleted obstacle
    pub fn try_delete_at(&mut self, pos: Vec2, obstacles: &[Obstacle]) -> Option<usize> {
        self.find_obstacle_at(pos, obstacles)
    }

    /// Push action to undo stack
    pub fn push_undo(&mut self, action: EditorAction) {
        self.undo_stack.push(action);
        // Clear redo stack when new action is performed
        self.redo_stack.clear();
    }

    /// Undo last action
    pub fn undo(&mut self) -> Option<EditorAction> {
        if let Some(action) = self.undo_stack.pop() {
            self.redo_stack.push(action.clone());
            Some(action)
        } else {
            None
        }
    }

    /// Redo last undone action
    pub fn redo(&mut self) -> Option<EditorAction> {
        if let Some(action) = self.redo_stack.pop() {
            self.undo_stack.push(action.clone());
            Some(action)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_creation() {
        let editor = WorldEditor::new();
        assert_eq!(editor.active_tool, EditorTool::Select);
        assert!(editor.grid_snap);
        assert_eq!(editor.grid_size, 0.5);
    }

    #[test]
    fn test_grid_snapping() {
        let editor = WorldEditor::new();

        let pos = Vec2::new(1.23, 4.67);
        let snapped = editor.snap_to_grid(pos);

        assert_eq!(snapped, Vec2::new(1.0, 4.5));
    }

    #[test]
    fn test_grid_snapping_disabled() {
        let mut editor = WorldEditor::new();
        editor.grid_snap = false;

        let pos = Vec2::new(1.23, 4.67);
        let snapped = editor.snap_to_grid(pos);

        assert_eq!(snapped, pos);
    }

    #[test]
    fn test_obstacle_creation() {
        let mut editor = WorldEditor::new();
        editor.active_tool = EditorTool::Rectangle;

        // Start creating
        editor.handle_mouse_press(Vec2::new(0.0, 0.0));
        assert!(editor.obstacle_in_progress.is_some());

        // Drag
        editor.handle_mouse_drag(Vec2::new(2.0, 2.0));

        // Release
        let obstacle = editor.handle_mouse_release();
        assert!(obstacle.is_some());

        let obs = obstacle.unwrap();
        assert_eq!(obs.pos, [1.0, 1.0]); // Center
        assert_eq!(obs.size, [2.0, 2.0]);
        assert_eq!(obs.shape, ObstacleShape::Rectangle);
    }

    #[test]
    fn test_selection() {
        let mut editor = WorldEditor::new();
        let entity = Entity::from_raw(1);

        assert!(!editor.is_selected(entity));

        editor.select_obstacle(entity);
        assert!(editor.is_selected(entity));
        assert_eq!(editor.selected_obstacles.len(), 1);

        editor.deselect_obstacle(entity);
        assert!(!editor.is_selected(entity));
        assert_eq!(editor.selected_obstacles.len(), 0);
    }

    #[test]
    fn test_undo_redo() {
        let mut editor = WorldEditor::new();

        let action = EditorAction::AddObstacle {
            obstacle: Obstacle {
                pos: [0.0, 0.0],
                size: [1.0, 1.0],
                shape: ObstacleShape::Rectangle,
                color: None,
            },
            entity: None,
        };

        editor.push_undo(action.clone());
        assert_eq!(editor.undo_stack.len(), 1);
        assert_eq!(editor.redo_stack.len(), 0);

        let undone = editor.undo();
        assert!(undone.is_some());
        assert_eq!(editor.undo_stack.len(), 0);
        assert_eq!(editor.redo_stack.len(), 1);

        let redone = editor.redo();
        assert!(redone.is_some());
        assert_eq!(editor.undo_stack.len(), 1);
        assert_eq!(editor.redo_stack.len(), 0);
    }
}
