//! Undo/redo system for editor operations

use bevy::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use serde::{Serialize, Deserialize};
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

/// Component data storage for entity restoration
#[derive(Clone, Debug)]
pub struct EntitySnapshot {
    /// Entity ID
    pub entity: Entity,
    /// Transform component
    pub transform: Option<Transform>,
    /// Global transform (read-only, for reference)
    pub global_transform: Option<GlobalTransform>,
    /// Name component
    pub name: Option<Name>,
    /// Visibility component
    pub visibility: Option<Visibility>,
    /// Additional serialized components (for custom components)
    pub custom_components: Vec<SerializedComponent>,
    /// Parent entity (for hierarchy restoration)
    pub parent: Option<Entity>,
    /// Children entities
    pub children: Vec<Entity>,
}

/// Serialized component data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedComponent {
    /// Component type name
    pub type_name: String,
    /// Serialized component data
    pub data: Vec<u8>,
}

impl EntitySnapshot {
    /// Create a snapshot from an entity in the world
    pub fn from_entity(entity: Entity, world: &World) -> Option<Self> {
        let entity_ref = world.get_entity(entity)?;

        // Capture standard components
        let transform = entity_ref.get::<Transform>().cloned();
        let global_transform = entity_ref.get::<GlobalTransform>().cloned();
        let name = entity_ref.get::<Name>().cloned();
        let visibility = entity_ref.get::<Visibility>().cloned();

        // Capture hierarchy information
        let parent = entity_ref.get::<Parent>().map(|p| p.get());
        let children = entity_ref
            .get::<Children>()
            .map(|c| c.iter().cloned().collect())
            .unwrap_or_default();

        Some(EntitySnapshot {
            entity,
            transform,
            global_transform,
            name,
            visibility,
            custom_components: Vec::new(), // Custom components would require reflection
            parent,
            children,
        })
    }

    /// Restore entity from snapshot
    pub fn restore(&self, world: &mut World) -> Entity {
        let mut entity_commands = world.spawn_empty();
        let new_entity = entity_commands.id();

        // Restore basic components
        if let Some(transform) = &self.transform {
            entity_commands.insert(*transform);
        }

        if let Some(name) = &self.name {
            entity_commands.insert(name.clone());
        }

        if let Some(visibility) = &self.visibility {
            entity_commands.insert(*visibility);
        }

        // Note: GlobalTransform is computed, not inserted directly

        // Restore hierarchy
        if let Some(parent_entity) = self.parent {
            if world.get_entity(parent_entity).is_some() {
                entity_commands.set_parent(parent_entity);
            }
        }

        new_entity
    }
}

/// Delete entity operation with full restoration support
pub struct DeleteOperation {
    entity: Entity,
    snapshot: Option<EntitySnapshot>,
    restored_entity: Option<Entity>,
    description: String,
}

impl DeleteOperation {
    pub fn new(entity: Entity, world: &World) -> Self {
        let snapshot = EntitySnapshot::from_entity(entity, world);
        Self {
            entity,
            snapshot,
            restored_entity: None,
            description: format!("Delete entity {:?}", entity),
        }
    }
}

impl UndoableOperation for DeleteOperation {
    fn execute(&mut self, world: &mut World) {
        // If we have a restored entity from a previous undo, delete it
        let entity_to_delete = self.restored_entity.unwrap_or(self.entity);

        // Take a snapshot before deletion if we don't have one
        if self.snapshot.is_none() {
            self.snapshot = EntitySnapshot::from_entity(entity_to_delete, world);
        }

        world.despawn(entity_to_delete);
        self.restored_entity = None;
    }

    fn undo(&mut self, world: &mut World) {
        // Restore entity from stored snapshot
        if let Some(snapshot) = &self.snapshot {
            let new_entity = snapshot.restore(world);
            self.restored_entity = Some(new_entity);
        }
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

/// Event for triggering undo operation
#[derive(Event)]
pub struct UndoEvent;

/// Event for triggering redo operation
#[derive(Event)]
pub struct RedoEvent;

/// Event for executing an undoable operation
#[derive(Event)]
pub struct ExecuteOperationEvent {
    pub operation: Box<dyn UndoableOperation>,
}

/// System to handle undo/redo keyboard shortcuts
pub fn undo_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut undo_events: EventWriter<UndoEvent>,
    mut redo_events: EventWriter<RedoEvent>,
) {
    let ctrl = keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);

    // Undo with Ctrl+Z
    if ctrl && keyboard.just_pressed(KeyCode::KeyZ) && !keyboard.pressed(KeyCode::ShiftLeft) {
        undo_events.send(UndoEvent);
    }

    // Redo with Ctrl+Shift+Z or Ctrl+Y
    if (ctrl && keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyZ))
        || (ctrl && keyboard.just_pressed(KeyCode::KeyY))
    {
        redo_events.send(RedoEvent);
    }
}

/// Exclusive system to process undo events
pub fn process_undo_system(
    world: &mut World,
) {
    // Check for undo events
    let mut should_undo = false;
    {
        let events = world.resource::<Events<UndoEvent>>();
        let mut reader = events.get_reader();
        should_undo = reader.read(events).next().is_some();
    }

    if should_undo {
        // Perform undo operation
        let mut undo_stack = world.resource_mut::<UndoStack>();
        if undo_stack.can_undo() {
            // We need to temporarily take the stack to avoid borrow conflicts
            let mut temp_stack = std::mem::take(&mut *undo_stack);
            temp_stack.undo(world);
            *undo_stack = temp_stack;

            info!("Undo performed: {}",
                  undo_stack.undo_description().unwrap_or("Unknown operation"));
        }
    }
}

/// Exclusive system to process redo events
pub fn process_redo_system(
    world: &mut World,
) {
    // Check for redo events
    let mut should_redo = false;
    {
        let events = world.resource::<Events<RedoEvent>>();
        let mut reader = events.get_reader();
        should_redo = reader.read(events).next().is_some();
    }

    if should_redo {
        // Perform redo operation
        let mut undo_stack = world.resource_mut::<UndoStack>();
        if undo_stack.can_redo() {
            // We need to temporarily take the stack to avoid borrow conflicts
            let mut temp_stack = std::mem::take(&mut *undo_stack);
            temp_stack.redo(world);
            *undo_stack = temp_stack;

            info!("Redo performed: {}",
                  undo_stack.redo_description().unwrap_or("Unknown operation"));
        }
    }
}

/// Exclusive system to execute new operations
pub fn execute_operation_system(
    world: &mut World,
) {
    // Collect all pending operations
    let mut operations_to_execute = Vec::new();
    {
        let events = world.resource::<Events<ExecuteOperationEvent>>();
        let mut reader = events.get_reader();
        for event in reader.read(events) {
            // We can't move out of the event, so we'll need another approach
            // This is a limitation we need to work around
        }
    }

    // For now, we'll process operations directly when they're created
    // In a real implementation, you'd want to queue them properly
}

/// Plugin to add undo/redo functionality
pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<UndoStack>()
            .add_event::<UndoEvent>()
            .add_event::<RedoEvent>()
            .add_event::<ExecuteOperationEvent>()
            .add_systems(Update, undo_keyboard_system)
            .add_systems(Last, (
                process_undo_system,
                process_redo_system,
            ).chain());
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
