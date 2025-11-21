//! Undo/redo system for editor operations

use bevy::prelude::*;
use std::collections::VecDeque;

/// Maximum number of undo operations to store
const MAX_UNDO_STACK_SIZE: usize = 100;

/// Trait for undoable operations
pub trait UndoableOperation: Send + Sync {
    /// Execute the operation
    fn execute(&mut self, world: &mut World);

    /// Undo the operation
    fn undo(&mut self, world: &mut World);

    /// Get description of this operation
    fn description(&self) -> &str;
}

/// Undo/redo stack resource
#[derive(Resource)]
pub struct UndoStack {
    /// Stack of undoable operations
    undo_stack: VecDeque<Box<dyn UndoableOperation>>,
    /// Stack of redoable operations
    redo_stack: VecDeque<Box<dyn UndoableOperation>>,
    /// Maximum stack size
    max_size: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(MAX_UNDO_STACK_SIZE)
    }
}

impl UndoStack {
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_size,
        }
    }

    /// Push a new operation onto the undo stack
    pub fn push(&mut self, operation: Box<dyn UndoableOperation>) {
        self.undo_stack.push_back(operation);

        // Clear redo stack when new operation is added
        self.redo_stack.clear();

        // Enforce max size
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.pop_front();
        }
    }

    /// Undo the last operation
    pub fn undo(&mut self, world: &mut World) -> bool {
        if let Some(mut operation) = self.undo_stack.pop_back() {
            operation.undo(world);
            self.redo_stack.push_back(operation);
            true
        } else {
            false
        }
    }

    /// Redo the last undone operation
    pub fn redo(&mut self, world: &mut World) -> bool {
        if let Some(mut operation) = self.redo_stack.pop_back() {
            operation.execute(world);
            self.undo_stack.push_back(operation);
            true
        } else {
            false
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get description of next undo operation
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|op| op.description())
    }

    /// Get description of next redo operation
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.back().map(|op| op.description())
    }

    /// Clear all stacks
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Get undo stack size
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack size
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

/// Transform change operation
pub struct TransformOperation {
    entity: Entity,
    old_transform: Transform,
    new_transform: Transform,
    description: String,
}

impl TransformOperation {
    pub fn new(entity: Entity, old_transform: Transform, new_transform: Transform) -> Self {
        Self {
            entity,
            old_transform,
            new_transform,
            description: format!("Transform entity {:?}", entity),
        }
    }
}

impl UndoableOperation for TransformOperation {
    fn execute(&mut self, world: &mut World) {
        if let Ok(mut entity) = world.get_entity_mut(self.entity) {
            if let Some(mut transform) = entity.get_mut::<Transform>() {
                *transform = self.new_transform;
            }
        }
    }

    fn undo(&mut self, world: &mut World) {
        if let Ok(mut entity) = world.get_entity_mut(self.entity) {
            if let Some(mut transform) = entity.get_mut::<Transform>() {
                *transform = self.old_transform;
            }
        }
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Delete entity operation
pub struct DeleteOperation {
    entity: Entity,
    // TODO: Store entity bundle data for restoration
    description: String,
}

impl DeleteOperation {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            description: format!("Delete entity {:?}", entity),
        }
    }
}

impl UndoableOperation for DeleteOperation {
    fn execute(&mut self, world: &mut World) {
        world.despawn(self.entity);
    }

    fn undo(&mut self, _world: &mut World) {
        // TODO: Restore entity from stored data
        // This requires serialization of entity components
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// Batch operation (multiple operations as one)
pub struct BatchOperation {
    operations: Vec<Box<dyn UndoableOperation>>,
    description: String,
}

impl BatchOperation {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            operations: Vec::new(),
            description: description.into(),
        }
    }

    pub fn add(&mut self, operation: Box<dyn UndoableOperation>) {
        self.operations.push(operation);
    }
}

impl UndoableOperation for BatchOperation {
    fn execute(&mut self, world: &mut World) {
        for operation in &mut self.operations {
            operation.execute(world);
        }
    }

    fn undo(&mut self, world: &mut World) {
        // Undo in reverse order
        for operation in self.operations.iter_mut().rev() {
            operation.undo(world);
        }
    }

    fn description(&self) -> &str {
        &self.description
    }
}

/// System to handle undo/redo keyboard shortcuts
pub fn undo_system(keyboard: Res<ButtonInput<KeyCode>>, _undo_stack: ResMut<UndoStack>) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    // TODO: Implement undo/redo with proper command pattern
    // This requires storing commands differently or using exclusive system

    // Undo with Ctrl+Z
    if ctrl && keyboard.just_pressed(KeyCode::KeyZ) && !keyboard.pressed(KeyCode::ShiftLeft) {
        // undo_stack.undo(world);
    }

    // Redo with Ctrl+Shift+Z or Ctrl+Y
    if (ctrl && keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyZ))
        || (ctrl && keyboard.just_pressed(KeyCode::KeyY))
    {
        // undo_stack.redo(world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestOperation {
        value: i32,
        executed: bool,
        description: String,
    }

    impl TestOperation {
        fn new(value: i32) -> Self {
            Self {
                value,
                executed: false,
                description: format!("Test operation {}", value),
            }
        }
    }

    impl UndoableOperation for TestOperation {
        fn execute(&mut self, _world: &mut World) {
            self.executed = true;
        }

        fn undo(&mut self, _world: &mut World) {
            self.executed = false;
        }

        fn description(&self) -> &str {
            &self.description
        }
    }

    #[test]
    fn test_undo_stack_push() {
        let mut stack = UndoStack::new(10);
        let op = Box::new(TestOperation::new(1));

        stack.push(op);
        assert_eq!(stack.undo_count(), 1);
        assert!(stack.can_undo());
    }

    #[test]
    fn test_undo_redo() {
        let mut world = World::new();
        let mut stack = UndoStack::new(10);

        let mut op = Box::new(TestOperation::new(1));
        op.execute(&mut world);
        stack.push(op);

        assert!(stack.can_undo());
        stack.undo(&mut world);
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        stack.redo(&mut world);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_undo_stack_clear_redo() {
        let mut stack = UndoStack::new(10);

        stack.push(Box::new(TestOperation::new(1)));
        let mut world = World::new();
        stack.undo(&mut world);

        assert!(stack.can_redo());

        // Pushing new operation should clear redo stack
        stack.push(Box::new(TestOperation::new(2)));
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_max_stack_size() {
        let mut stack = UndoStack::new(3);

        for i in 0..5 {
            stack.push(Box::new(TestOperation::new(i)));
        }

        assert_eq!(stack.undo_count(), 3);
    }

    #[test]
    fn test_descriptions() {
        let mut stack = UndoStack::new(10);

        stack.push(Box::new(TestOperation::new(1)));
        assert_eq!(stack.undo_description(), Some("Test operation 1"));

        let mut world = World::new();
        stack.undo(&mut world);
        assert_eq!(stack.redo_description(), Some("Test operation 1"));
    }

    #[test]
    fn test_batch_operation() {
        let mut batch = BatchOperation::new("Batch test");
        batch.add(Box::new(TestOperation::new(1)));
        batch.add(Box::new(TestOperation::new(2)));

        assert_eq!(batch.description(), "Batch test");
    }
}
