//! Comprehensive keybindings system for sim3d robotics simulator
//!
//! This module provides:
//! - Configurable keybindings with modifiers (Ctrl, Alt, Shift, Super)
//! - Preset keybinding schemes (Default/Blender, Maya, Unity)
//! - Conflict detection
//! - Serialization/deserialization for config files
//! - Event-driven input handling with Bevy integration

use bevy::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

// ============================================================================
// KeyCode Serialization Helper
// ============================================================================

/// Wrapper for KeyCode that supports serialization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SerializableKeyCode(pub KeyCode);

impl From<KeyCode> for SerializableKeyCode {
    fn from(code: KeyCode) -> Self {
        SerializableKeyCode(code)
    }
}

impl From<SerializableKeyCode> for KeyCode {
    fn from(wrapper: SerializableKeyCode) -> Self {
        wrapper.0
    }
}

impl Serialize for SerializableKeyCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as string representation
        let key_str = keycode_to_string(self.0);
        serializer.serialize_str(&key_str)
    }
}

impl<'de> Deserialize<'de> for SerializableKeyCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        string_to_keycode(&s)
            .map(SerializableKeyCode)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown key code: {}", s)))
    }
}

/// Convert KeyCode to string for serialization
fn keycode_to_string(code: KeyCode) -> String {
    match code {
        // Letters
        KeyCode::KeyA => "KeyA".to_string(),
        KeyCode::KeyB => "KeyB".to_string(),
        KeyCode::KeyC => "KeyC".to_string(),
        KeyCode::KeyD => "KeyD".to_string(),
        KeyCode::KeyE => "KeyE".to_string(),
        KeyCode::KeyF => "KeyF".to_string(),
        KeyCode::KeyG => "KeyG".to_string(),
        KeyCode::KeyH => "KeyH".to_string(),
        KeyCode::KeyI => "KeyI".to_string(),
        KeyCode::KeyJ => "KeyJ".to_string(),
        KeyCode::KeyK => "KeyK".to_string(),
        KeyCode::KeyL => "KeyL".to_string(),
        KeyCode::KeyM => "KeyM".to_string(),
        KeyCode::KeyN => "KeyN".to_string(),
        KeyCode::KeyO => "KeyO".to_string(),
        KeyCode::KeyP => "KeyP".to_string(),
        KeyCode::KeyQ => "KeyQ".to_string(),
        KeyCode::KeyR => "KeyR".to_string(),
        KeyCode::KeyS => "KeyS".to_string(),
        KeyCode::KeyT => "KeyT".to_string(),
        KeyCode::KeyU => "KeyU".to_string(),
        KeyCode::KeyV => "KeyV".to_string(),
        KeyCode::KeyW => "KeyW".to_string(),
        KeyCode::KeyX => "KeyX".to_string(),
        KeyCode::KeyY => "KeyY".to_string(),
        KeyCode::KeyZ => "KeyZ".to_string(),
        // Numbers
        KeyCode::Digit0 => "Digit0".to_string(),
        KeyCode::Digit1 => "Digit1".to_string(),
        KeyCode::Digit2 => "Digit2".to_string(),
        KeyCode::Digit3 => "Digit3".to_string(),
        KeyCode::Digit4 => "Digit4".to_string(),
        KeyCode::Digit5 => "Digit5".to_string(),
        KeyCode::Digit6 => "Digit6".to_string(),
        KeyCode::Digit7 => "Digit7".to_string(),
        KeyCode::Digit8 => "Digit8".to_string(),
        KeyCode::Digit9 => "Digit9".to_string(),
        // Function keys
        KeyCode::F1 => "F1".to_string(),
        KeyCode::F2 => "F2".to_string(),
        KeyCode::F3 => "F3".to_string(),
        KeyCode::F4 => "F4".to_string(),
        KeyCode::F5 => "F5".to_string(),
        KeyCode::F6 => "F6".to_string(),
        KeyCode::F7 => "F7".to_string(),
        KeyCode::F8 => "F8".to_string(),
        KeyCode::F9 => "F9".to_string(),
        KeyCode::F10 => "F10".to_string(),
        KeyCode::F11 => "F11".to_string(),
        KeyCode::F12 => "F12".to_string(),
        // Numpad
        KeyCode::Numpad0 => "Numpad0".to_string(),
        KeyCode::Numpad1 => "Numpad1".to_string(),
        KeyCode::Numpad2 => "Numpad2".to_string(),
        KeyCode::Numpad3 => "Numpad3".to_string(),
        KeyCode::Numpad4 => "Numpad4".to_string(),
        KeyCode::Numpad5 => "Numpad5".to_string(),
        KeyCode::Numpad6 => "Numpad6".to_string(),
        KeyCode::Numpad7 => "Numpad7".to_string(),
        KeyCode::Numpad8 => "Numpad8".to_string(),
        KeyCode::Numpad9 => "Numpad9".to_string(),
        KeyCode::NumpadAdd => "NumpadAdd".to_string(),
        KeyCode::NumpadSubtract => "NumpadSubtract".to_string(),
        KeyCode::NumpadMultiply => "NumpadMultiply".to_string(),
        KeyCode::NumpadDivide => "NumpadDivide".to_string(),
        KeyCode::NumpadDecimal => "NumpadDecimal".to_string(),
        KeyCode::NumpadEnter => "NumpadEnter".to_string(),
        // Special keys
        KeyCode::Escape => "Escape".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Space => "Space".to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        // Arrow keys
        KeyCode::ArrowUp => "ArrowUp".to_string(),
        KeyCode::ArrowDown => "ArrowDown".to_string(),
        KeyCode::ArrowLeft => "ArrowLeft".to_string(),
        KeyCode::ArrowRight => "ArrowRight".to_string(),
        // Modifiers (for completeness, though not used as primary keys)
        KeyCode::ShiftLeft => "ShiftLeft".to_string(),
        KeyCode::ShiftRight => "ShiftRight".to_string(),
        KeyCode::ControlLeft => "ControlLeft".to_string(),
        KeyCode::ControlRight => "ControlRight".to_string(),
        KeyCode::AltLeft => "AltLeft".to_string(),
        KeyCode::AltRight => "AltRight".to_string(),
        KeyCode::SuperLeft => "SuperLeft".to_string(),
        KeyCode::SuperRight => "SuperRight".to_string(),
        // Punctuation and symbols
        KeyCode::Minus => "Minus".to_string(),
        KeyCode::Equal => "Equal".to_string(),
        KeyCode::BracketLeft => "BracketLeft".to_string(),
        KeyCode::BracketRight => "BracketRight".to_string(),
        KeyCode::Backslash => "Backslash".to_string(),
        KeyCode::Semicolon => "Semicolon".to_string(),
        KeyCode::Quote => "Quote".to_string(),
        KeyCode::Backquote => "Backquote".to_string(),
        KeyCode::Comma => "Comma".to_string(),
        KeyCode::Period => "Period".to_string(),
        KeyCode::Slash => "Slash".to_string(),
        // Default for unknown
        _ => format!("{:?}", code),
    }
}

/// Convert string to KeyCode for deserialization
fn string_to_keycode(s: &str) -> Option<KeyCode> {
    match s {
        // Letters
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        // Numbers
        "Digit0" => Some(KeyCode::Digit0),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "Digit6" => Some(KeyCode::Digit6),
        "Digit7" => Some(KeyCode::Digit7),
        "Digit8" => Some(KeyCode::Digit8),
        "Digit9" => Some(KeyCode::Digit9),
        // Function keys
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        // Numpad
        "Numpad0" => Some(KeyCode::Numpad0),
        "Numpad1" => Some(KeyCode::Numpad1),
        "Numpad2" => Some(KeyCode::Numpad2),
        "Numpad3" => Some(KeyCode::Numpad3),
        "Numpad4" => Some(KeyCode::Numpad4),
        "Numpad5" => Some(KeyCode::Numpad5),
        "Numpad6" => Some(KeyCode::Numpad6),
        "Numpad7" => Some(KeyCode::Numpad7),
        "Numpad8" => Some(KeyCode::Numpad8),
        "Numpad9" => Some(KeyCode::Numpad9),
        "NumpadAdd" => Some(KeyCode::NumpadAdd),
        "NumpadSubtract" => Some(KeyCode::NumpadSubtract),
        "NumpadMultiply" => Some(KeyCode::NumpadMultiply),
        "NumpadDivide" => Some(KeyCode::NumpadDivide),
        "NumpadDecimal" => Some(KeyCode::NumpadDecimal),
        "NumpadEnter" => Some(KeyCode::NumpadEnter),
        // Special keys
        "Escape" => Some(KeyCode::Escape),
        "Tab" => Some(KeyCode::Tab),
        "Space" => Some(KeyCode::Space),
        "Enter" => Some(KeyCode::Enter),
        "Backspace" => Some(KeyCode::Backspace),
        "Delete" => Some(KeyCode::Delete),
        "Insert" => Some(KeyCode::Insert),
        "Home" => Some(KeyCode::Home),
        "End" => Some(KeyCode::End),
        "PageUp" => Some(KeyCode::PageUp),
        "PageDown" => Some(KeyCode::PageDown),
        // Arrow keys
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        // Modifiers
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),
        "SuperLeft" => Some(KeyCode::SuperLeft),
        "SuperRight" => Some(KeyCode::SuperRight),
        // Punctuation and symbols
        "Minus" => Some(KeyCode::Minus),
        "Equal" => Some(KeyCode::Equal),
        "BracketLeft" => Some(KeyCode::BracketLeft),
        "BracketRight" => Some(KeyCode::BracketRight),
        "Backslash" => Some(KeyCode::Backslash),
        "Semicolon" => Some(KeyCode::Semicolon),
        "Quote" => Some(KeyCode::Quote),
        "Backquote" => Some(KeyCode::Backquote),
        "Comma" => Some(KeyCode::Comma),
        "Period" => Some(KeyCode::Period),
        "Slash" => Some(KeyCode::Slash),
        _ => None,
    }
}

// ============================================================================
// KeyBinding Categories
// ============================================================================

/// Categories for organizing keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum KeyBindingCategory {
    #[default]
    Camera,
    Selection,
    Tools,
    File,
    Edit,
    View,
    Simulation,
}

impl KeyBindingCategory {
    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            KeyBindingCategory::Camera => "Camera",
            KeyBindingCategory::Selection => "Selection",
            KeyBindingCategory::Tools => "Tools",
            KeyBindingCategory::File => "File",
            KeyBindingCategory::Edit => "Edit",
            KeyBindingCategory::View => "View",
            KeyBindingCategory::Simulation => "Simulation",
        }
    }

    /// Get all categories
    pub fn all() -> &'static [KeyBindingCategory] {
        &[
            KeyBindingCategory::Camera,
            KeyBindingCategory::Selection,
            KeyBindingCategory::Tools,
            KeyBindingCategory::File,
            KeyBindingCategory::Edit,
            KeyBindingCategory::View,
            KeyBindingCategory::Simulation,
        ]
    }
}

// ============================================================================
// KeyBinding Actions
// ============================================================================

/// All possible keybinding actions in the simulator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyBindingAction {
    // Camera actions
    CameraOrbit,
    CameraPan,
    CameraZoomIn,
    CameraZoomOut,
    CameraReset,
    CameraFocusSelection,
    CameraTopView,
    CameraFrontView,
    CameraSideView,
    CameraPerspectiveView,

    // Selection actions
    Select,
    MultiSelect,
    SelectAll,
    DeselectAll,
    InvertSelection,
    SelectParent,
    SelectChildren,

    // Tools actions
    ToolTranslate,
    ToolRotate,
    ToolScale,
    SwitchGizmoMode,
    ToggleLocalGlobal,
    ToggleSnapping,
    IncreaseSnapSize,
    DecreaseSnapSize,

    // File actions
    NewScene,
    OpenScene,
    SaveScene,
    SaveSceneAs,
    Export,
    Import,
    Quit,

    // Edit actions
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Duplicate,
    Rename,
    Group,
    Ungroup,

    // View actions
    ToggleGrid,
    ToggleWireframe,
    ToggleStats,
    ToggleInspector,
    ToggleHierarchy,
    ToggleConsole,
    ToggleFullscreen,
    ToggleDebugPanel,

    // Simulation actions
    SimulationPlay,
    SimulationPause,
    SimulationStep,
    SimulationReset,
    TogglePhysics,
    ToggleCollisionVisualization,
    SpawnRobot,
    RemoveRobot,
}

impl KeyBindingAction {
    /// Get the category for this action
    pub fn category(&self) -> KeyBindingCategory {
        match self {
            // Camera
            KeyBindingAction::CameraOrbit
            | KeyBindingAction::CameraPan
            | KeyBindingAction::CameraZoomIn
            | KeyBindingAction::CameraZoomOut
            | KeyBindingAction::CameraReset
            | KeyBindingAction::CameraFocusSelection
            | KeyBindingAction::CameraTopView
            | KeyBindingAction::CameraFrontView
            | KeyBindingAction::CameraSideView
            | KeyBindingAction::CameraPerspectiveView => KeyBindingCategory::Camera,

            // Selection
            KeyBindingAction::Select
            | KeyBindingAction::MultiSelect
            | KeyBindingAction::SelectAll
            | KeyBindingAction::DeselectAll
            | KeyBindingAction::InvertSelection
            | KeyBindingAction::SelectParent
            | KeyBindingAction::SelectChildren => KeyBindingCategory::Selection,

            // Tools
            KeyBindingAction::ToolTranslate
            | KeyBindingAction::ToolRotate
            | KeyBindingAction::ToolScale
            | KeyBindingAction::SwitchGizmoMode
            | KeyBindingAction::ToggleLocalGlobal
            | KeyBindingAction::ToggleSnapping
            | KeyBindingAction::IncreaseSnapSize
            | KeyBindingAction::DecreaseSnapSize => KeyBindingCategory::Tools,

            // File
            KeyBindingAction::NewScene
            | KeyBindingAction::OpenScene
            | KeyBindingAction::SaveScene
            | KeyBindingAction::SaveSceneAs
            | KeyBindingAction::Export
            | KeyBindingAction::Import
            | KeyBindingAction::Quit => KeyBindingCategory::File,

            // Edit
            KeyBindingAction::Undo
            | KeyBindingAction::Redo
            | KeyBindingAction::Cut
            | KeyBindingAction::Copy
            | KeyBindingAction::Paste
            | KeyBindingAction::Delete
            | KeyBindingAction::Duplicate
            | KeyBindingAction::Rename
            | KeyBindingAction::Group
            | KeyBindingAction::Ungroup => KeyBindingCategory::Edit,

            // View
            KeyBindingAction::ToggleGrid
            | KeyBindingAction::ToggleWireframe
            | KeyBindingAction::ToggleStats
            | KeyBindingAction::ToggleInspector
            | KeyBindingAction::ToggleHierarchy
            | KeyBindingAction::ToggleConsole
            | KeyBindingAction::ToggleFullscreen
            | KeyBindingAction::ToggleDebugPanel => KeyBindingCategory::View,

            // Simulation
            KeyBindingAction::SimulationPlay
            | KeyBindingAction::SimulationPause
            | KeyBindingAction::SimulationStep
            | KeyBindingAction::SimulationReset
            | KeyBindingAction::TogglePhysics
            | KeyBindingAction::ToggleCollisionVisualization
            | KeyBindingAction::SpawnRobot
            | KeyBindingAction::RemoveRobot => KeyBindingCategory::Simulation,
        }
    }

    /// Get display name for this action
    pub fn display_name(&self) -> &'static str {
        match self {
            // Camera
            KeyBindingAction::CameraOrbit => "Orbit Camera",
            KeyBindingAction::CameraPan => "Pan Camera",
            KeyBindingAction::CameraZoomIn => "Zoom In",
            KeyBindingAction::CameraZoomOut => "Zoom Out",
            KeyBindingAction::CameraReset => "Reset Camera",
            KeyBindingAction::CameraFocusSelection => "Focus Selection",
            KeyBindingAction::CameraTopView => "Top View",
            KeyBindingAction::CameraFrontView => "Front View",
            KeyBindingAction::CameraSideView => "Side View",
            KeyBindingAction::CameraPerspectiveView => "Perspective View",

            // Selection
            KeyBindingAction::Select => "Select",
            KeyBindingAction::MultiSelect => "Multi-Select",
            KeyBindingAction::SelectAll => "Select All",
            KeyBindingAction::DeselectAll => "Deselect All",
            KeyBindingAction::InvertSelection => "Invert Selection",
            KeyBindingAction::SelectParent => "Select Parent",
            KeyBindingAction::SelectChildren => "Select Children",

            // Tools
            KeyBindingAction::ToolTranslate => "Translate Tool",
            KeyBindingAction::ToolRotate => "Rotate Tool",
            KeyBindingAction::ToolScale => "Scale Tool",
            KeyBindingAction::SwitchGizmoMode => "Switch Gizmo Mode",
            KeyBindingAction::ToggleLocalGlobal => "Toggle Local/Global",
            KeyBindingAction::ToggleSnapping => "Toggle Snapping",
            KeyBindingAction::IncreaseSnapSize => "Increase Snap Size",
            KeyBindingAction::DecreaseSnapSize => "Decrease Snap Size",

            // File
            KeyBindingAction::NewScene => "New Scene",
            KeyBindingAction::OpenScene => "Open Scene",
            KeyBindingAction::SaveScene => "Save Scene",
            KeyBindingAction::SaveSceneAs => "Save Scene As",
            KeyBindingAction::Export => "Export",
            KeyBindingAction::Import => "Import",
            KeyBindingAction::Quit => "Quit",

            // Edit
            KeyBindingAction::Undo => "Undo",
            KeyBindingAction::Redo => "Redo",
            KeyBindingAction::Cut => "Cut",
            KeyBindingAction::Copy => "Copy",
            KeyBindingAction::Paste => "Paste",
            KeyBindingAction::Delete => "Delete",
            KeyBindingAction::Duplicate => "Duplicate",
            KeyBindingAction::Rename => "Rename",
            KeyBindingAction::Group => "Group",
            KeyBindingAction::Ungroup => "Ungroup",

            // View
            KeyBindingAction::ToggleGrid => "Toggle Grid",
            KeyBindingAction::ToggleWireframe => "Toggle Wireframe",
            KeyBindingAction::ToggleStats => "Toggle Stats",
            KeyBindingAction::ToggleInspector => "Toggle Inspector",
            KeyBindingAction::ToggleHierarchy => "Toggle Hierarchy",
            KeyBindingAction::ToggleConsole => "Toggle Console",
            KeyBindingAction::ToggleFullscreen => "Toggle Fullscreen",
            KeyBindingAction::ToggleDebugPanel => "Toggle Debug Panel",

            // Simulation
            KeyBindingAction::SimulationPlay => "Play Simulation",
            KeyBindingAction::SimulationPause => "Pause Simulation",
            KeyBindingAction::SimulationStep => "Step Simulation",
            KeyBindingAction::SimulationReset => "Reset Simulation",
            KeyBindingAction::TogglePhysics => "Toggle Physics",
            KeyBindingAction::ToggleCollisionVisualization => "Toggle Collision Visualization",
            KeyBindingAction::SpawnRobot => "Spawn Robot",
            KeyBindingAction::RemoveRobot => "Remove Robot",
        }
    }

    /// Get description for this action
    pub fn description(&self) -> &'static str {
        match self {
            // Camera
            KeyBindingAction::CameraOrbit => "Orbit the camera around the focus point",
            KeyBindingAction::CameraPan => "Pan the camera in the view plane",
            KeyBindingAction::CameraZoomIn => "Zoom the camera in",
            KeyBindingAction::CameraZoomOut => "Zoom the camera out",
            KeyBindingAction::CameraReset => "Reset camera to default position",
            KeyBindingAction::CameraFocusSelection => "Focus camera on selected object",
            KeyBindingAction::CameraTopView => "Switch to top-down view",
            KeyBindingAction::CameraFrontView => "Switch to front view",
            KeyBindingAction::CameraSideView => "Switch to side view",
            KeyBindingAction::CameraPerspectiveView => "Switch to perspective view",

            // Selection
            KeyBindingAction::Select => "Select object under cursor",
            KeyBindingAction::MultiSelect => "Add object to selection",
            KeyBindingAction::SelectAll => "Select all objects in scene",
            KeyBindingAction::DeselectAll => "Clear current selection",
            KeyBindingAction::InvertSelection => "Invert current selection",
            KeyBindingAction::SelectParent => "Select parent of current selection",
            KeyBindingAction::SelectChildren => "Select children of current selection",

            // Tools
            KeyBindingAction::ToolTranslate => "Activate translate/move tool",
            KeyBindingAction::ToolRotate => "Activate rotate tool",
            KeyBindingAction::ToolScale => "Activate scale tool",
            KeyBindingAction::SwitchGizmoMode => "Cycle through gizmo modes",
            KeyBindingAction::ToggleLocalGlobal => "Toggle between local and global coordinates",
            KeyBindingAction::ToggleSnapping => "Toggle grid snapping",
            KeyBindingAction::IncreaseSnapSize => "Increase snap grid size",
            KeyBindingAction::DecreaseSnapSize => "Decrease snap grid size",

            // File
            KeyBindingAction::NewScene => "Create a new empty scene",
            KeyBindingAction::OpenScene => "Open an existing scene file",
            KeyBindingAction::SaveScene => "Save the current scene",
            KeyBindingAction::SaveSceneAs => "Save the current scene with a new name",
            KeyBindingAction::Export => "Export scene to external format",
            KeyBindingAction::Import => "Import external assets or scenes",
            KeyBindingAction::Quit => "Exit the application",

            // Edit
            KeyBindingAction::Undo => "Undo the last action",
            KeyBindingAction::Redo => "Redo the last undone action",
            KeyBindingAction::Cut => "Cut selected objects to clipboard",
            KeyBindingAction::Copy => "Copy selected objects to clipboard",
            KeyBindingAction::Paste => "Paste objects from clipboard",
            KeyBindingAction::Delete => "Delete selected objects",
            KeyBindingAction::Duplicate => "Duplicate selected objects",
            KeyBindingAction::Rename => "Rename selected object",
            KeyBindingAction::Group => "Group selected objects",
            KeyBindingAction::Ungroup => "Ungroup selected objects",

            // View
            KeyBindingAction::ToggleGrid => "Show/hide the grid",
            KeyBindingAction::ToggleWireframe => "Toggle wireframe rendering mode",
            KeyBindingAction::ToggleStats => "Show/hide performance statistics",
            KeyBindingAction::ToggleInspector => "Show/hide the inspector panel",
            KeyBindingAction::ToggleHierarchy => "Show/hide the hierarchy panel",
            KeyBindingAction::ToggleConsole => "Show/hide the console",
            KeyBindingAction::ToggleFullscreen => "Toggle fullscreen mode",
            KeyBindingAction::ToggleDebugPanel => "Show/hide the debug panel",

            // Simulation
            KeyBindingAction::SimulationPlay => "Start the simulation",
            KeyBindingAction::SimulationPause => "Pause the simulation",
            KeyBindingAction::SimulationStep => "Advance simulation by one frame",
            KeyBindingAction::SimulationReset => "Reset simulation to initial state",
            KeyBindingAction::TogglePhysics => "Enable/disable physics simulation",
            KeyBindingAction::ToggleCollisionVisualization => "Show/hide collision shapes",
            KeyBindingAction::SpawnRobot => "Spawn a new robot in the scene",
            KeyBindingAction::RemoveRobot => "Remove selected robot from scene",
        }
    }

    /// Get all actions
    pub fn all() -> Vec<KeyBindingAction> {
        vec![
            // Camera
            KeyBindingAction::CameraOrbit,
            KeyBindingAction::CameraPan,
            KeyBindingAction::CameraZoomIn,
            KeyBindingAction::CameraZoomOut,
            KeyBindingAction::CameraReset,
            KeyBindingAction::CameraFocusSelection,
            KeyBindingAction::CameraTopView,
            KeyBindingAction::CameraFrontView,
            KeyBindingAction::CameraSideView,
            KeyBindingAction::CameraPerspectiveView,
            // Selection
            KeyBindingAction::Select,
            KeyBindingAction::MultiSelect,
            KeyBindingAction::SelectAll,
            KeyBindingAction::DeselectAll,
            KeyBindingAction::InvertSelection,
            KeyBindingAction::SelectParent,
            KeyBindingAction::SelectChildren,
            // Tools
            KeyBindingAction::ToolTranslate,
            KeyBindingAction::ToolRotate,
            KeyBindingAction::ToolScale,
            KeyBindingAction::SwitchGizmoMode,
            KeyBindingAction::ToggleLocalGlobal,
            KeyBindingAction::ToggleSnapping,
            KeyBindingAction::IncreaseSnapSize,
            KeyBindingAction::DecreaseSnapSize,
            // File
            KeyBindingAction::NewScene,
            KeyBindingAction::OpenScene,
            KeyBindingAction::SaveScene,
            KeyBindingAction::SaveSceneAs,
            KeyBindingAction::Export,
            KeyBindingAction::Import,
            KeyBindingAction::Quit,
            // Edit
            KeyBindingAction::Undo,
            KeyBindingAction::Redo,
            KeyBindingAction::Cut,
            KeyBindingAction::Copy,
            KeyBindingAction::Paste,
            KeyBindingAction::Delete,
            KeyBindingAction::Duplicate,
            KeyBindingAction::Rename,
            KeyBindingAction::Group,
            KeyBindingAction::Ungroup,
            // View
            KeyBindingAction::ToggleGrid,
            KeyBindingAction::ToggleWireframe,
            KeyBindingAction::ToggleStats,
            KeyBindingAction::ToggleInspector,
            KeyBindingAction::ToggleHierarchy,
            KeyBindingAction::ToggleConsole,
            KeyBindingAction::ToggleFullscreen,
            KeyBindingAction::ToggleDebugPanel,
            // Simulation
            KeyBindingAction::SimulationPlay,
            KeyBindingAction::SimulationPause,
            KeyBindingAction::SimulationStep,
            KeyBindingAction::SimulationReset,
            KeyBindingAction::TogglePhysics,
            KeyBindingAction::ToggleCollisionVisualization,
            KeyBindingAction::SpawnRobot,
            KeyBindingAction::RemoveRobot,
        ]
    }
}

// ============================================================================
// Key Modifiers
// ============================================================================

/// Key modifiers for keybindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl KeyModifiers {
    /// Create modifiers with no modifiers pressed
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        super_key: false,
    };

    /// Create modifiers with only Ctrl pressed
    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
        super_key: false,
    };

    /// Create modifiers with only Alt pressed
    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
        super_key: false,
    };

    /// Create modifiers with only Shift pressed
    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        super_key: false,
    };

    /// Create modifiers with Ctrl+Shift pressed
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        alt: false,
        shift: true,
        super_key: false,
    };

    /// Create modifiers with Ctrl+Alt pressed
    pub const CTRL_ALT: Self = Self {
        ctrl: true,
        alt: true,
        shift: false,
        super_key: false,
    };

    /// Create new modifiers with specified keys
    pub const fn new(ctrl: bool, alt: bool, shift: bool, super_key: bool) -> Self {
        Self {
            ctrl,
            alt,
            shift,
            super_key,
        }
    }

    /// Check if any modifier is pressed
    pub fn any(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.super_key
    }

    /// Check if no modifiers are pressed
    pub fn none(&self) -> bool {
        !self.any()
    }

    /// Get display string for modifiers
    pub fn display_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.super_key {
            parts.push("Super");
        }
        parts.join("+")
    }

    /// Check current keyboard state for modifiers
    pub fn from_keyboard(keyboard: &ButtonInput<KeyCode>) -> Self {
        Self {
            ctrl: keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight),
            alt: keyboard.pressed(KeyCode::AltLeft) || keyboard.pressed(KeyCode::AltRight),
            shift: keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight),
            super_key: keyboard.pressed(KeyCode::SuperLeft)
                || keyboard.pressed(KeyCode::SuperRight),
        }
    }
}

// ============================================================================
// Key Combination
// ============================================================================

/// A key combination (key code + modifiers)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombination {
    #[serde(with = "serializable_keycode_serde")]
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

/// Custom serde module for KeyCode
mod serializable_keycode_serde {
    use super::*;

    pub fn serialize<S>(key: &KeyCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&keycode_to_string(*key))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<KeyCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        string_to_keycode(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown key code: {}", s)))
    }
}

impl KeyCombination {
    /// Create a new key combination
    pub const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a key combination with no modifiers
    pub const fn key_only(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// Create a key combination with Ctrl modifier
    pub const fn ctrl(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::CTRL,
        }
    }

    /// Create a key combination with Alt modifier
    pub const fn alt(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::ALT,
        }
    }

    /// Create a key combination with Shift modifier
    pub const fn shift(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Create a key combination with Ctrl+Shift modifiers
    pub const fn ctrl_shift(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::CTRL_SHIFT,
        }
    }

    /// Get display string for key combination
    pub fn display_string(&self) -> String {
        let mod_str = self.modifiers.display_string();
        let key_str = keycode_to_string(self.key);

        if mod_str.is_empty() {
            key_str
        } else {
            format!("{}+{}", mod_str, key_str)
        }
    }

    /// Check if this key combination matches current input state
    pub fn is_pressed(&self, keyboard: &ButtonInput<KeyCode>) -> bool {
        let current_modifiers = KeyModifiers::from_keyboard(keyboard);
        keyboard.pressed(self.key) && current_modifiers == self.modifiers
    }

    /// Check if this key combination was just pressed
    pub fn just_pressed(&self, keyboard: &ButtonInput<KeyCode>) -> bool {
        let current_modifiers = KeyModifiers::from_keyboard(keyboard);
        keyboard.just_pressed(self.key) && current_modifiers == self.modifiers
    }
}

// ============================================================================
// KeyBinding
// ============================================================================

/// A complete keybinding definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    /// The action this binding triggers
    pub action: KeyBindingAction,
    /// The key combination for this binding
    pub combination: KeyCombination,
    /// Whether this binding is enabled
    pub enabled: bool,
}

impl KeyBinding {
    /// Create a new keybinding
    pub fn new(action: KeyBindingAction, combination: KeyCombination) -> Self {
        Self {
            action,
            combination,
            enabled: true,
        }
    }

    /// Get the category for this binding
    pub fn category(&self) -> KeyBindingCategory {
        self.action.category()
    }

    /// Get display name for this binding
    pub fn display_name(&self) -> &'static str {
        self.action.display_name()
    }

    /// Get description for this binding
    pub fn description(&self) -> &'static str {
        self.action.description()
    }

    /// Check if this keybinding matches current input state
    pub fn is_triggered(&self, keyboard: &ButtonInput<KeyCode>) -> bool {
        self.enabled && self.combination.just_pressed(keyboard)
    }
}

// ============================================================================
// Keybinding Presets
// ============================================================================

/// Available keybinding preset schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum KeyBindingPreset {
    #[default]
    Default, // Similar to Blender
    Maya,
    Unity,
    Custom,
}

impl KeyBindingPreset {
    /// Get display name for this preset
    pub fn display_name(&self) -> &'static str {
        match self {
            KeyBindingPreset::Default => "Default (Blender-style)",
            KeyBindingPreset::Maya => "Maya-style",
            KeyBindingPreset::Unity => "Unity-style",
            KeyBindingPreset::Custom => "Custom",
        }
    }

    /// Get all presets
    pub fn all() -> &'static [KeyBindingPreset] {
        &[
            KeyBindingPreset::Default,
            KeyBindingPreset::Maya,
            KeyBindingPreset::Unity,
            KeyBindingPreset::Custom,
        ]
    }
}

// ============================================================================
// Keybinding Conflict
// ============================================================================

/// Represents a keybinding conflict
#[derive(Debug, Clone)]
pub struct KeyBindingConflict {
    pub combination: KeyCombination,
    pub actions: Vec<KeyBindingAction>,
}

impl KeyBindingConflict {
    /// Get description of the conflict
    pub fn description(&self) -> String {
        let action_names: Vec<&str> = self.actions.iter().map(|a| a.display_name()).collect();
        format!(
            "Key '{}' is bound to multiple actions: {}",
            self.combination.display_string(),
            action_names.join(", ")
        )
    }
}

// ============================================================================
// KeyBindingMap Resource
// ============================================================================

/// Resource containing all keybindings
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindingMap {
    /// Map of action to keybinding
    bindings: HashMap<KeyBindingAction, KeyBinding>,
    /// Current preset
    current_preset: KeyBindingPreset,
    /// Whether bindings have been modified from preset
    modified: bool,
}

impl Default for KeyBindingMap {
    fn default() -> Self {
        Self::new(KeyBindingPreset::Default)
    }
}

impl KeyBindingMap {
    /// Create a new keybinding map with specified preset
    pub fn new(preset: KeyBindingPreset) -> Self {
        let mut map = Self {
            bindings: HashMap::new(),
            current_preset: preset,
            modified: false,
        };
        map.load_preset(preset);
        map
    }

    /// Get the current preset
    pub fn current_preset(&self) -> KeyBindingPreset {
        self.current_preset
    }

    /// Check if bindings have been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get binding for an action
    pub fn get(&self, action: &KeyBindingAction) -> Option<&KeyBinding> {
        self.bindings.get(action)
    }

    /// Get mutable binding for an action
    pub fn get_mut(&mut self, action: &KeyBindingAction) -> Option<&mut KeyBinding> {
        self.bindings.get_mut(action)
    }

    /// Set binding for an action
    pub fn set(&mut self, action: KeyBindingAction, combination: KeyCombination) {
        let binding = KeyBinding::new(action, combination);
        self.bindings.insert(action, binding);
        self.modified = true;
        self.current_preset = KeyBindingPreset::Custom;
    }

    /// Remove binding for an action
    pub fn remove(&mut self, action: &KeyBindingAction) -> Option<KeyBinding> {
        self.modified = true;
        self.current_preset = KeyBindingPreset::Custom;
        self.bindings.remove(action)
    }

    /// Enable/disable a binding
    pub fn set_enabled(&mut self, action: &KeyBindingAction, enabled: bool) {
        if let Some(binding) = self.bindings.get_mut(action) {
            binding.enabled = enabled;
            self.modified = true;
        }
    }

    /// Get all bindings
    pub fn all_bindings(&self) -> impl Iterator<Item = &KeyBinding> {
        self.bindings.values()
    }

    /// Get bindings by category
    pub fn bindings_by_category(&self, category: KeyBindingCategory) -> Vec<&KeyBinding> {
        self.bindings
            .values()
            .filter(|b| b.category() == category)
            .collect()
    }

    /// Find action by key combination
    pub fn find_action(&self, combination: &KeyCombination) -> Option<KeyBindingAction> {
        self.bindings
            .iter()
            .find(|(_, binding)| binding.enabled && binding.combination == *combination)
            .map(|(action, _)| *action)
    }

    /// Detect conflicts in keybindings
    pub fn detect_conflicts(&self) -> Vec<KeyBindingConflict> {
        let mut combination_map: HashMap<KeyCombination, Vec<KeyBindingAction>> = HashMap::new();

        for (action, binding) in &self.bindings {
            if binding.enabled {
                combination_map
                    .entry(binding.combination)
                    .or_default()
                    .push(*action);
            }
        }

        combination_map
            .into_iter()
            .filter(|(_, actions)| actions.len() > 1)
            .map(|(combination, actions)| KeyBindingConflict {
                combination,
                actions,
            })
            .collect()
    }

    /// Check if there are any conflicts
    pub fn has_conflicts(&self) -> bool {
        !self.detect_conflicts().is_empty()
    }

    /// Load a preset
    pub fn load_preset(&mut self, preset: KeyBindingPreset) {
        self.bindings.clear();
        self.current_preset = preset;
        self.modified = false;

        let bindings = match preset {
            KeyBindingPreset::Default => Self::default_bindings(),
            KeyBindingPreset::Maya => Self::maya_bindings(),
            KeyBindingPreset::Unity => Self::unity_bindings(),
            KeyBindingPreset::Custom => Self::default_bindings(), // Start from default
        };

        for binding in bindings {
            self.bindings.insert(binding.action, binding);
        }
    }

    /// Reset to defaults
    pub fn reset_to_defaults(&mut self) {
        self.load_preset(KeyBindingPreset::Default);
    }

    /// Default/Blender-style keybindings
    fn default_bindings() -> Vec<KeyBinding> {
        vec![
            // Camera
            KeyBinding::new(
                KeyBindingAction::CameraOrbit,
                KeyCombination::key_only(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPan,
                KeyCombination::shift(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomIn,
                KeyCombination::key_only(KeyCode::NumpadAdd),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomOut,
                KeyCombination::key_only(KeyCode::NumpadSubtract),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraReset,
                KeyCombination::key_only(KeyCode::Home),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFocusSelection,
                KeyCombination::key_only(KeyCode::KeyF),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraTopView,
                KeyCombination::key_only(KeyCode::Numpad7),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFrontView,
                KeyCombination::key_only(KeyCode::Numpad1),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraSideView,
                KeyCombination::key_only(KeyCode::Numpad3),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPerspectiveView,
                KeyCombination::key_only(KeyCode::Numpad5),
            ),
            // Selection
            KeyBinding::new(
                KeyBindingAction::Select,
                KeyCombination::key_only(KeyCode::Space),
            ),
            KeyBinding::new(
                KeyBindingAction::MultiSelect,
                KeyCombination::shift(KeyCode::Space),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectAll,
                KeyCombination::key_only(KeyCode::KeyA),
            ),
            KeyBinding::new(
                KeyBindingAction::DeselectAll,
                KeyCombination::alt(KeyCode::KeyA),
            ),
            KeyBinding::new(
                KeyBindingAction::InvertSelection,
                KeyCombination::ctrl(KeyCode::KeyI),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectParent,
                KeyCombination::key_only(KeyCode::BracketLeft),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectChildren,
                KeyCombination::key_only(KeyCode::BracketRight),
            ),
            // Tools
            KeyBinding::new(
                KeyBindingAction::ToolTranslate,
                KeyCombination::key_only(KeyCode::KeyG),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolRotate,
                KeyCombination::key_only(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolScale,
                KeyCombination::key_only(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::SwitchGizmoMode,
                KeyCombination::key_only(KeyCode::Tab),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleLocalGlobal,
                KeyCombination::key_only(KeyCode::KeyL),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleSnapping,
                KeyCombination::shift(KeyCode::Tab),
            ),
            KeyBinding::new(
                KeyBindingAction::IncreaseSnapSize,
                KeyCombination::ctrl(KeyCode::Equal),
            ),
            KeyBinding::new(
                KeyBindingAction::DecreaseSnapSize,
                KeyCombination::ctrl(KeyCode::Minus),
            ),
            // File
            KeyBinding::new(
                KeyBindingAction::NewScene,
                KeyCombination::ctrl(KeyCode::KeyN),
            ),
            KeyBinding::new(
                KeyBindingAction::OpenScene,
                KeyCombination::ctrl(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveScene,
                KeyCombination::ctrl(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveSceneAs,
                KeyCombination::ctrl_shift(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::Export,
                KeyCombination::ctrl(KeyCode::KeyE),
            ),
            KeyBinding::new(
                KeyBindingAction::Import,
                KeyCombination::ctrl_shift(KeyCode::KeyI),
            ),
            KeyBinding::new(KeyBindingAction::Quit, KeyCombination::ctrl(KeyCode::KeyQ)),
            // Edit
            KeyBinding::new(KeyBindingAction::Undo, KeyCombination::ctrl(KeyCode::KeyZ)),
            KeyBinding::new(
                KeyBindingAction::Redo,
                KeyCombination::ctrl_shift(KeyCode::KeyZ),
            ),
            KeyBinding::new(KeyBindingAction::Cut, KeyCombination::ctrl(KeyCode::KeyX)),
            KeyBinding::new(KeyBindingAction::Copy, KeyCombination::ctrl(KeyCode::KeyC)),
            KeyBinding::new(KeyBindingAction::Paste, KeyCombination::ctrl(KeyCode::KeyV)),
            KeyBinding::new(
                KeyBindingAction::Delete,
                KeyCombination::key_only(KeyCode::Delete),
            ),
            KeyBinding::new(
                KeyBindingAction::Duplicate,
                KeyCombination::ctrl(KeyCode::KeyD),
            ),
            KeyBinding::new(
                KeyBindingAction::Rename,
                KeyCombination::key_only(KeyCode::F2),
            ),
            KeyBinding::new(KeyBindingAction::Group, KeyCombination::ctrl(KeyCode::KeyG)),
            KeyBinding::new(
                KeyBindingAction::Ungroup,
                KeyCombination::ctrl_shift(KeyCode::KeyG),
            ),
            // View
            KeyBinding::new(
                KeyBindingAction::ToggleGrid,
                KeyCombination::shift(KeyCode::KeyG),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleWireframe,
                KeyCombination::key_only(KeyCode::KeyZ),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleStats,
                KeyCombination::key_only(KeyCode::F3),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleInspector,
                KeyCombination::key_only(KeyCode::KeyI),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleHierarchy,
                KeyCombination::key_only(KeyCode::KeyH),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleConsole,
                KeyCombination::key_only(KeyCode::Backquote),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleFullscreen,
                KeyCombination::key_only(KeyCode::F11),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleDebugPanel,
                KeyCombination::key_only(KeyCode::F12),
            ),
            // Simulation
            KeyBinding::new(
                KeyBindingAction::SimulationPlay,
                KeyCombination::key_only(KeyCode::F5),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationPause,
                KeyCombination::key_only(KeyCode::F6),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationStep,
                KeyCombination::key_only(KeyCode::F7),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationReset,
                KeyCombination::key_only(KeyCode::F8),
            ),
            KeyBinding::new(
                KeyBindingAction::TogglePhysics,
                KeyCombination::key_only(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleCollisionVisualization,
                KeyCombination::shift(KeyCode::KeyC),
            ),
            KeyBinding::new(
                KeyBindingAction::SpawnRobot,
                KeyCombination::ctrl_shift(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::RemoveRobot,
                KeyCombination::ctrl(KeyCode::Delete),
            ),
        ]
    }

    /// Maya-style keybindings
    fn maya_bindings() -> Vec<KeyBinding> {
        vec![
            // Camera - Maya uses Alt+mouse for camera controls
            KeyBinding::new(
                KeyBindingAction::CameraOrbit,
                KeyCombination::alt(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPan,
                KeyCombination::alt(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomIn,
                KeyCombination::alt(KeyCode::Equal),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomOut,
                KeyCombination::alt(KeyCode::Minus),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraReset,
                KeyCombination::alt(KeyCode::KeyH),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFocusSelection,
                KeyCombination::key_only(KeyCode::KeyF),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraTopView,
                KeyCombination::key_only(KeyCode::KeyY),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFrontView,
                KeyCombination::key_only(KeyCode::KeyZ),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraSideView,
                KeyCombination::key_only(KeyCode::KeyX),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPerspectiveView,
                KeyCombination::key_only(KeyCode::KeyP),
            ),
            // Selection
            KeyBinding::new(
                KeyBindingAction::Select,
                KeyCombination::key_only(KeyCode::KeyQ),
            ),
            KeyBinding::new(
                KeyBindingAction::MultiSelect,
                KeyCombination::shift(KeyCode::KeyQ),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectAll,
                KeyCombination::ctrl(KeyCode::KeyA),
            ),
            KeyBinding::new(
                KeyBindingAction::DeselectAll,
                KeyCombination::key_only(KeyCode::Escape),
            ),
            KeyBinding::new(
                KeyBindingAction::InvertSelection,
                KeyCombination::ctrl_shift(KeyCode::KeyI),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectParent,
                KeyCombination::key_only(KeyCode::ArrowUp),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectChildren,
                KeyCombination::key_only(KeyCode::ArrowDown),
            ),
            // Tools - Maya QWER
            KeyBinding::new(
                KeyBindingAction::ToolTranslate,
                KeyCombination::key_only(KeyCode::KeyW),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolRotate,
                KeyCombination::key_only(KeyCode::KeyE),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolScale,
                KeyCombination::key_only(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::SwitchGizmoMode,
                KeyCombination::key_only(KeyCode::Space),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleLocalGlobal,
                KeyCombination::key_only(KeyCode::Insert),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleSnapping,
                KeyCombination::key_only(KeyCode::KeyX),
            ),
            KeyBinding::new(
                KeyBindingAction::IncreaseSnapSize,
                KeyCombination::key_only(KeyCode::Period),
            ),
            KeyBinding::new(
                KeyBindingAction::DecreaseSnapSize,
                KeyCombination::key_only(KeyCode::Comma),
            ),
            // File
            KeyBinding::new(
                KeyBindingAction::NewScene,
                KeyCombination::ctrl(KeyCode::KeyN),
            ),
            KeyBinding::new(
                KeyBindingAction::OpenScene,
                KeyCombination::ctrl(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveScene,
                KeyCombination::ctrl(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveSceneAs,
                KeyCombination::ctrl_shift(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::Export,
                KeyCombination::ctrl(KeyCode::KeyE),
            ),
            KeyBinding::new(
                KeyBindingAction::Import,
                KeyCombination::ctrl(KeyCode::KeyI),
            ),
            KeyBinding::new(KeyBindingAction::Quit, KeyCombination::ctrl(KeyCode::KeyQ)),
            // Edit
            KeyBinding::new(KeyBindingAction::Undo, KeyCombination::ctrl(KeyCode::KeyZ)),
            KeyBinding::new(KeyBindingAction::Redo, KeyCombination::ctrl(KeyCode::KeyY)),
            KeyBinding::new(KeyBindingAction::Cut, KeyCombination::ctrl(KeyCode::KeyX)),
            KeyBinding::new(KeyBindingAction::Copy, KeyCombination::ctrl(KeyCode::KeyC)),
            KeyBinding::new(KeyBindingAction::Paste, KeyCombination::ctrl(KeyCode::KeyV)),
            KeyBinding::new(
                KeyBindingAction::Delete,
                KeyCombination::key_only(KeyCode::Delete),
            ),
            KeyBinding::new(
                KeyBindingAction::Duplicate,
                KeyCombination::ctrl(KeyCode::KeyD),
            ),
            KeyBinding::new(
                KeyBindingAction::Rename,
                KeyCombination::key_only(KeyCode::F2),
            ),
            KeyBinding::new(KeyBindingAction::Group, KeyCombination::ctrl(KeyCode::KeyG)),
            KeyBinding::new(
                KeyBindingAction::Ungroup,
                KeyCombination::shift(KeyCode::KeyG),
            ),
            // View
            KeyBinding::new(
                KeyBindingAction::ToggleGrid,
                KeyCombination::key_only(KeyCode::KeyG),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleWireframe,
                KeyCombination::key_only(KeyCode::Digit4),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleStats,
                KeyCombination::key_only(KeyCode::Digit7),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleInspector,
                KeyCombination::ctrl(KeyCode::KeyA),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleHierarchy,
                KeyCombination::key_only(KeyCode::KeyH),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleConsole,
                KeyCombination::key_only(KeyCode::Backquote),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleFullscreen,
                KeyCombination::key_only(KeyCode::F11),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleDebugPanel,
                KeyCombination::key_only(KeyCode::F12),
            ),
            // Simulation
            KeyBinding::new(
                KeyBindingAction::SimulationPlay,
                KeyCombination::alt(KeyCode::KeyV),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationPause,
                KeyCombination::key_only(KeyCode::Escape),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationStep,
                KeyCombination::alt(KeyCode::Period),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationReset,
                KeyCombination::alt(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::TogglePhysics,
                KeyCombination::ctrl(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleCollisionVisualization,
                KeyCombination::shift(KeyCode::KeyC),
            ),
            KeyBinding::new(
                KeyBindingAction::SpawnRobot,
                KeyCombination::ctrl_shift(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::RemoveRobot,
                KeyCombination::ctrl(KeyCode::Delete),
            ),
        ]
    }

    /// Unity-style keybindings
    fn unity_bindings() -> Vec<KeyBinding> {
        vec![
            // Camera
            KeyBinding::new(
                KeyBindingAction::CameraOrbit,
                KeyCombination::alt(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPan,
                KeyCombination::alt(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomIn,
                KeyCombination::key_only(KeyCode::Equal),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraZoomOut,
                KeyCombination::key_only(KeyCode::Minus),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraReset,
                KeyCombination::shift(KeyCode::KeyF),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFocusSelection,
                KeyCombination::key_only(KeyCode::KeyF),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraTopView,
                KeyCombination::ctrl(KeyCode::Digit2),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraFrontView,
                KeyCombination::ctrl(KeyCode::Digit1),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraSideView,
                KeyCombination::ctrl(KeyCode::Digit3),
            ),
            KeyBinding::new(
                KeyBindingAction::CameraPerspectiveView,
                KeyCombination::ctrl(KeyCode::Digit0),
            ),
            // Selection
            KeyBinding::new(
                KeyBindingAction::Select,
                KeyCombination::key_only(KeyCode::KeyQ),
            ),
            KeyBinding::new(
                KeyBindingAction::MultiSelect,
                KeyCombination::shift(KeyCode::KeyQ),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectAll,
                KeyCombination::ctrl(KeyCode::KeyA),
            ),
            KeyBinding::new(
                KeyBindingAction::DeselectAll,
                KeyCombination::shift(KeyCode::KeyD),
            ),
            KeyBinding::new(
                KeyBindingAction::InvertSelection,
                KeyCombination::ctrl(KeyCode::KeyI),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectParent,
                KeyCombination::key_only(KeyCode::ArrowUp),
            ),
            KeyBinding::new(
                KeyBindingAction::SelectChildren,
                KeyCombination::key_only(KeyCode::ArrowDown),
            ),
            // Tools - Unity QWER
            KeyBinding::new(
                KeyBindingAction::ToolTranslate,
                KeyCombination::key_only(KeyCode::KeyW),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolRotate,
                KeyCombination::key_only(KeyCode::KeyE),
            ),
            KeyBinding::new(
                KeyBindingAction::ToolScale,
                KeyCombination::key_only(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::SwitchGizmoMode,
                KeyCombination::key_only(KeyCode::KeyT),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleLocalGlobal,
                KeyCombination::key_only(KeyCode::KeyX),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleSnapping,
                KeyCombination::ctrl(KeyCode::Semicolon),
            ),
            KeyBinding::new(
                KeyBindingAction::IncreaseSnapSize,
                KeyCombination::ctrl(KeyCode::BracketRight),
            ),
            KeyBinding::new(
                KeyBindingAction::DecreaseSnapSize,
                KeyCombination::ctrl(KeyCode::BracketLeft),
            ),
            // File
            KeyBinding::new(
                KeyBindingAction::NewScene,
                KeyCombination::ctrl(KeyCode::KeyN),
            ),
            KeyBinding::new(
                KeyBindingAction::OpenScene,
                KeyCombination::ctrl(KeyCode::KeyO),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveScene,
                KeyCombination::ctrl(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::SaveSceneAs,
                KeyCombination::ctrl_shift(KeyCode::KeyS),
            ),
            KeyBinding::new(
                KeyBindingAction::Export,
                KeyCombination::ctrl_shift(KeyCode::KeyE),
            ),
            KeyBinding::new(
                KeyBindingAction::Import,
                KeyCombination::ctrl(KeyCode::KeyI),
            ),
            KeyBinding::new(KeyBindingAction::Quit, KeyCombination::ctrl(KeyCode::KeyQ)),
            // Edit
            KeyBinding::new(KeyBindingAction::Undo, KeyCombination::ctrl(KeyCode::KeyZ)),
            KeyBinding::new(KeyBindingAction::Redo, KeyCombination::ctrl(KeyCode::KeyY)),
            KeyBinding::new(KeyBindingAction::Cut, KeyCombination::ctrl(KeyCode::KeyX)),
            KeyBinding::new(KeyBindingAction::Copy, KeyCombination::ctrl(KeyCode::KeyC)),
            KeyBinding::new(KeyBindingAction::Paste, KeyCombination::ctrl(KeyCode::KeyV)),
            KeyBinding::new(
                KeyBindingAction::Delete,
                KeyCombination::key_only(KeyCode::Delete),
            ),
            KeyBinding::new(
                KeyBindingAction::Duplicate,
                KeyCombination::ctrl(KeyCode::KeyD),
            ),
            KeyBinding::new(
                KeyBindingAction::Rename,
                KeyCombination::key_only(KeyCode::F2),
            ),
            KeyBinding::new(KeyBindingAction::Group, KeyCombination::ctrl(KeyCode::KeyG)),
            KeyBinding::new(
                KeyBindingAction::Ungroup,
                KeyCombination::ctrl_shift(KeyCode::KeyG),
            ),
            // View
            KeyBinding::new(
                KeyBindingAction::ToggleGrid,
                KeyCombination::ctrl(KeyCode::Quote),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleWireframe,
                KeyCombination::key_only(KeyCode::KeyZ),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleStats,
                KeyCombination::key_only(KeyCode::F3),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleInspector,
                KeyCombination::ctrl(KeyCode::Digit3),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleHierarchy,
                KeyCombination::ctrl(KeyCode::Digit4),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleConsole,
                KeyCombination::ctrl_shift(KeyCode::KeyC),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleFullscreen,
                KeyCombination::key_only(KeyCode::F11),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleDebugPanel,
                KeyCombination::key_only(KeyCode::F12),
            ),
            // Simulation
            KeyBinding::new(
                KeyBindingAction::SimulationPlay,
                KeyCombination::ctrl(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationPause,
                KeyCombination::ctrl_shift(KeyCode::KeyP),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationStep,
                KeyCombination::ctrl(KeyCode::Period),
            ),
            KeyBinding::new(
                KeyBindingAction::SimulationReset,
                KeyCombination::ctrl_shift(KeyCode::KeyR),
            ),
            KeyBinding::new(
                KeyBindingAction::TogglePhysics,
                KeyCombination::ctrl(KeyCode::KeyY),
            ),
            KeyBinding::new(
                KeyBindingAction::ToggleCollisionVisualization,
                KeyCombination::shift(KeyCode::KeyC),
            ),
            KeyBinding::new(
                KeyBindingAction::SpawnRobot,
                KeyCombination::ctrl_shift(KeyCode::KeyN),
            ),
            KeyBinding::new(
                KeyBindingAction::RemoveRobot,
                KeyCombination::ctrl(KeyCode::Backspace),
            ),
        ]
    }

    // ========================================================================
    // Configuration file handling
    // ========================================================================

    /// Save keybindings to a JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), KeyBindingError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| KeyBindingError::SerializationError(e.to_string()))?;
        fs::write(path, json).map_err(|e| KeyBindingError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Load keybindings from a JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self, KeyBindingError> {
        let json = fs::read_to_string(path).map_err(|e| KeyBindingError::IoError(e.to_string()))?;
        let map: KeyBindingMap = serde_json::from_str(&json)
            .map_err(|e| KeyBindingError::DeserializationError(e.to_string()))?;
        Ok(map)
    }

    /// Save keybindings to a TOML file
    pub fn save_to_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), KeyBindingError> {
        let toml = toml::to_string_pretty(self)
            .map_err(|e| KeyBindingError::SerializationError(e.to_string()))?;
        fs::write(path, toml).map_err(|e| KeyBindingError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Load keybindings from a TOML file
    pub fn load_from_toml<P: AsRef<Path>>(path: P) -> Result<Self, KeyBindingError> {
        let toml_str =
            fs::read_to_string(path).map_err(|e| KeyBindingError::IoError(e.to_string()))?;
        let map: KeyBindingMap = toml::from_str(&toml_str)
            .map_err(|e| KeyBindingError::DeserializationError(e.to_string()))?;
        Ok(map)
    }

    /// Export keybindings for sharing (returns serialized string)
    pub fn export_scheme(&self) -> Result<String, KeyBindingError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| KeyBindingError::SerializationError(e.to_string()))
    }

    /// Import keybindings from exported string
    pub fn import_scheme(data: &str) -> Result<Self, KeyBindingError> {
        serde_json::from_str(data).map_err(|e| KeyBindingError::DeserializationError(e.to_string()))
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Errors that can occur in keybinding operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum KeyBindingError {
    #[error("Failed to serialize keybindings: {0}")]
    SerializationError(String),

    #[error("Failed to deserialize keybindings: {0}")]
    DeserializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Keybinding conflict: {0}")]
    Conflict(String),

    #[error("Invalid keybinding: {0}")]
    Invalid(String),
}

// ============================================================================
// Events
// ============================================================================

/// Event sent when a keybinding action is triggered
#[derive(Event, Debug, Clone)]
pub struct KeyBindingTriggeredEvent {
    pub action: KeyBindingAction,
    pub modifiers: KeyModifiers,
}

impl KeyBindingTriggeredEvent {
    pub fn new(action: KeyBindingAction, modifiers: KeyModifiers) -> Self {
        Self { action, modifiers }
    }
}

// ============================================================================
// Input Handling System
// ============================================================================

/// Check if a specific keybinding action is triggered
pub fn check_keybinding(
    action: KeyBindingAction,
    keybindings: &KeyBindingMap,
    keyboard: &ButtonInput<KeyCode>,
) -> bool {
    if let Some(binding) = keybindings.get(&action) {
        binding.is_triggered(keyboard)
    } else {
        false
    }
}

/// Check multiple keybinding actions and return which ones are triggered
pub fn check_keybindings(
    actions: &[KeyBindingAction],
    keybindings: &KeyBindingMap,
    keyboard: &ButtonInput<KeyCode>,
) -> Vec<KeyBindingAction> {
    actions
        .iter()
        .filter(|action| check_keybinding(**action, keybindings, keyboard))
        .copied()
        .collect()
}

/// System to handle keybinding input and generate events
pub fn keybinding_input_system(
    keybindings: Res<KeyBindingMap>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: EventWriter<KeyBindingTriggeredEvent>,
) {
    let current_modifiers = KeyModifiers::from_keyboard(&keyboard);

    for binding in keybindings.all_bindings() {
        if binding.is_triggered(&keyboard) {
            events.send(KeyBindingTriggeredEvent::new(
                binding.action,
                current_modifiers,
            ));
        }
    }
}

// ============================================================================
// Bevy Plugin
// ============================================================================

/// Plugin for keybinding system integration with Bevy
pub struct KeyBindingsPlugin {
    /// Initial preset to use
    pub preset: KeyBindingPreset,
    /// Path to config file (optional)
    pub config_path: Option<String>,
}

impl Default for KeyBindingsPlugin {
    fn default() -> Self {
        Self {
            preset: KeyBindingPreset::Default,
            config_path: None,
        }
    }
}

impl KeyBindingsPlugin {
    /// Create a new plugin with default preset
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new plugin with specified preset
    pub fn with_preset(preset: KeyBindingPreset) -> Self {
        Self {
            preset,
            config_path: None,
        }
    }

    /// Create a new plugin that loads from config file
    pub fn with_config<P: Into<String>>(path: P) -> Self {
        Self {
            preset: KeyBindingPreset::Default,
            config_path: Some(path.into()),
        }
    }
}

impl Plugin for KeyBindingsPlugin {
    fn build(&self, app: &mut App) {
        // Try to load from config file, fall back to preset
        let keybindings = if let Some(ref path) = self.config_path {
            KeyBindingMap::load_from_json(path).unwrap_or_else(|_| KeyBindingMap::new(self.preset))
        } else {
            KeyBindingMap::new(self.preset)
        };

        app.insert_resource(keybindings)
            .add_event::<KeyBindingTriggeredEvent>()
            .add_systems(PreUpdate, keybinding_input_system);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_modifiers_creation() {
        let none = KeyModifiers::NONE;
        assert!(!none.ctrl);
        assert!(!none.alt);
        assert!(!none.shift);
        assert!(!none.super_key);
        assert!(none.none());
        assert!(!none.any());

        let ctrl = KeyModifiers::CTRL;
        assert!(ctrl.ctrl);
        assert!(!ctrl.alt);
        assert!(ctrl.any());
        assert!(!ctrl.none());

        let ctrl_shift = KeyModifiers::CTRL_SHIFT;
        assert!(ctrl_shift.ctrl);
        assert!(ctrl_shift.shift);
        assert!(!ctrl_shift.alt);
    }

    #[test]
    fn test_key_modifiers_display() {
        assert_eq!(KeyModifiers::NONE.display_string(), "");
        assert_eq!(KeyModifiers::CTRL.display_string(), "Ctrl");
        assert_eq!(KeyModifiers::ALT.display_string(), "Alt");
        assert_eq!(KeyModifiers::SHIFT.display_string(), "Shift");
        assert_eq!(KeyModifiers::CTRL_SHIFT.display_string(), "Ctrl+Shift");
        assert_eq!(KeyModifiers::CTRL_ALT.display_string(), "Ctrl+Alt");
    }

    #[test]
    fn test_key_combination_creation() {
        let key_only = KeyCombination::key_only(KeyCode::KeyA);
        assert_eq!(key_only.key, KeyCode::KeyA);
        assert!(key_only.modifiers.none());

        let ctrl_a = KeyCombination::ctrl(KeyCode::KeyA);
        assert_eq!(ctrl_a.key, KeyCode::KeyA);
        assert!(ctrl_a.modifiers.ctrl);

        let ctrl_shift_s = KeyCombination::ctrl_shift(KeyCode::KeyS);
        assert_eq!(ctrl_shift_s.key, KeyCode::KeyS);
        assert!(ctrl_shift_s.modifiers.ctrl);
        assert!(ctrl_shift_s.modifiers.shift);
    }

    #[test]
    fn test_key_combination_display() {
        assert_eq!(
            KeyCombination::key_only(KeyCode::KeyA).display_string(),
            "KeyA"
        );
        assert_eq!(
            KeyCombination::ctrl(KeyCode::KeyA).display_string(),
            "Ctrl+KeyA"
        );
        assert_eq!(
            KeyCombination::ctrl_shift(KeyCode::KeyS).display_string(),
            "Ctrl+Shift+KeyS"
        );
    }

    #[test]
    fn test_keybinding_creation() {
        let binding = KeyBinding::new(
            KeyBindingAction::SaveScene,
            KeyCombination::ctrl(KeyCode::KeyS),
        );
        assert_eq!(binding.action, KeyBindingAction::SaveScene);
        assert!(binding.enabled);
        assert_eq!(binding.category(), KeyBindingCategory::File);
        assert_eq!(binding.display_name(), "Save Scene");
    }

    #[test]
    fn test_keybinding_action_categories() {
        assert_eq!(
            KeyBindingAction::CameraOrbit.category(),
            KeyBindingCategory::Camera
        );
        assert_eq!(
            KeyBindingAction::Select.category(),
            KeyBindingCategory::Selection
        );
        assert_eq!(
            KeyBindingAction::ToolTranslate.category(),
            KeyBindingCategory::Tools
        );
        assert_eq!(
            KeyBindingAction::SaveScene.category(),
            KeyBindingCategory::File
        );
        assert_eq!(KeyBindingAction::Undo.category(), KeyBindingCategory::Edit);
        assert_eq!(
            KeyBindingAction::ToggleGrid.category(),
            KeyBindingCategory::View
        );
        assert_eq!(
            KeyBindingAction::SimulationPlay.category(),
            KeyBindingCategory::Simulation
        );
    }

    #[test]
    fn test_keybinding_action_all() {
        let all_actions = KeyBindingAction::all();
        assert!(!all_actions.is_empty());
        // Check we have actions from all categories
        let categories: std::collections::HashSet<_> =
            all_actions.iter().map(|a| a.category()).collect();
        assert!(categories.contains(&KeyBindingCategory::Camera));
        assert!(categories.contains(&KeyBindingCategory::Selection));
        assert!(categories.contains(&KeyBindingCategory::Tools));
        assert!(categories.contains(&KeyBindingCategory::File));
        assert!(categories.contains(&KeyBindingCategory::Edit));
        assert!(categories.contains(&KeyBindingCategory::View));
        assert!(categories.contains(&KeyBindingCategory::Simulation));
    }

    #[test]
    fn test_keybinding_map_creation() {
        let map = KeyBindingMap::default();
        assert_eq!(map.current_preset(), KeyBindingPreset::Default);
        assert!(!map.is_modified());
    }

    #[test]
    fn test_keybinding_map_get_set() {
        let mut map = KeyBindingMap::default();

        // Get existing binding
        let save_binding = map.get(&KeyBindingAction::SaveScene);
        assert!(save_binding.is_some());

        // Set new binding
        map.set(
            KeyBindingAction::CameraOrbit,
            KeyCombination::alt(KeyCode::KeyO),
        );
        let orbit_binding = map.get(&KeyBindingAction::CameraOrbit);
        assert!(orbit_binding.is_some());
        assert!(orbit_binding.unwrap().combination.modifiers.alt);

        // Check modified flag
        assert!(map.is_modified());
        assert_eq!(map.current_preset(), KeyBindingPreset::Custom);
    }

    #[test]
    fn test_keybinding_map_remove() {
        let mut map = KeyBindingMap::default();

        let removed = map.remove(&KeyBindingAction::SaveScene);
        assert!(removed.is_some());
        assert!(map.get(&KeyBindingAction::SaveScene).is_none());
    }

    #[test]
    fn test_keybinding_map_enable_disable() {
        let mut map = KeyBindingMap::default();

        map.set_enabled(&KeyBindingAction::SaveScene, false);
        let binding = map.get(&KeyBindingAction::SaveScene);
        assert!(binding.is_some());
        assert!(!binding.unwrap().enabled);

        map.set_enabled(&KeyBindingAction::SaveScene, true);
        let binding = map.get(&KeyBindingAction::SaveScene);
        assert!(binding.unwrap().enabled);
    }

    #[test]
    fn test_keybinding_map_find_action() {
        let map = KeyBindingMap::default();

        // Find by key combination
        let action = map.find_action(&KeyCombination::ctrl(KeyCode::KeyS));
        assert_eq!(action, Some(KeyBindingAction::SaveScene));
    }

    #[test]
    fn test_keybinding_map_bindings_by_category() {
        let map = KeyBindingMap::default();

        let camera_bindings = map.bindings_by_category(KeyBindingCategory::Camera);
        assert!(!camera_bindings.is_empty());
        for binding in camera_bindings {
            assert_eq!(binding.category(), KeyBindingCategory::Camera);
        }

        let file_bindings = map.bindings_by_category(KeyBindingCategory::File);
        assert!(!file_bindings.is_empty());
        for binding in file_bindings {
            assert_eq!(binding.category(), KeyBindingCategory::File);
        }
    }

    #[test]
    fn test_keybinding_conflict_detection() {
        let mut map = KeyBindingMap::default();

        // Add an explicit conflict
        map.set(
            KeyBindingAction::CameraOrbit,
            KeyCombination::ctrl(KeyCode::KeyS),
        );

        let conflicts = map.detect_conflicts();
        // Should have at least the conflict we just created
        let has_ctrl_s_conflict = conflicts
            .iter()
            .any(|c| c.combination == KeyCombination::ctrl(KeyCode::KeyS) && c.actions.len() > 1);
        assert!(has_ctrl_s_conflict);
    }

    #[test]
    fn test_keybinding_preset_loading() {
        let mut map = KeyBindingMap::new(KeyBindingPreset::Default);

        // Load Maya preset
        map.load_preset(KeyBindingPreset::Maya);
        assert_eq!(map.current_preset(), KeyBindingPreset::Maya);
        assert!(!map.is_modified());

        // Verify Maya-style tools are QWER (actually WER for translate/rotate/scale)
        let translate = map.get(&KeyBindingAction::ToolTranslate);
        assert!(translate.is_some());
        assert_eq!(translate.unwrap().combination.key, KeyCode::KeyW);

        // Load Unity preset
        map.load_preset(KeyBindingPreset::Unity);
        assert_eq!(map.current_preset(), KeyBindingPreset::Unity);

        // Reset to defaults
        map.reset_to_defaults();
        assert_eq!(map.current_preset(), KeyBindingPreset::Default);
    }

    #[test]
    fn test_keybinding_serialization_json() {
        let map = KeyBindingMap::default();

        // Export
        let json = map.export_scheme();
        assert!(json.is_ok());
        let json_str = json.unwrap();
        assert!(!json_str.is_empty());

        // Import
        let imported = KeyBindingMap::import_scheme(&json_str);
        assert!(imported.is_ok());
        let imported_map = imported.unwrap();

        // Verify same preset
        assert_eq!(imported_map.current_preset(), map.current_preset());
    }

    #[test]
    fn test_keybinding_file_operations() {
        use std::env::temp_dir;

        let map = KeyBindingMap::default();
        let temp_path = temp_dir().join("test_keybindings.json");

        // Save to JSON
        let save_result = map.save_to_json(&temp_path);
        assert!(save_result.is_ok());

        // Load from JSON
        let load_result = KeyBindingMap::load_from_json(&temp_path);
        assert!(load_result.is_ok());

        // Cleanup
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_keybinding_toml_operations() {
        use std::env::temp_dir;

        let map = KeyBindingMap::default();
        let temp_path = temp_dir().join("test_keybindings.toml");

        // Save to TOML
        let save_result = map.save_to_toml(&temp_path);
        assert!(save_result.is_ok());

        // Load from TOML
        let load_result = KeyBindingMap::load_from_toml(&temp_path);
        assert!(load_result.is_ok());

        // Cleanup
        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_key_binding_conflict_description() {
        let conflict = KeyBindingConflict {
            combination: KeyCombination::ctrl(KeyCode::KeyS),
            actions: vec![KeyBindingAction::SaveScene, KeyBindingAction::ToolScale],
        };

        let desc = conflict.description();
        assert!(desc.contains("Ctrl+KeyS"));
        assert!(desc.contains("Save Scene"));
        assert!(desc.contains("Scale Tool"));
    }

    #[test]
    fn test_keybinding_triggered_event() {
        let event = KeyBindingTriggeredEvent::new(KeyBindingAction::SaveScene, KeyModifiers::CTRL);
        assert_eq!(event.action, KeyBindingAction::SaveScene);
        assert!(event.modifiers.ctrl);
    }

    #[test]
    fn test_keybinding_category_display() {
        assert_eq!(KeyBindingCategory::Camera.display_name(), "Camera");
        assert_eq!(KeyBindingCategory::Selection.display_name(), "Selection");
        assert_eq!(KeyBindingCategory::Tools.display_name(), "Tools");
        assert_eq!(KeyBindingCategory::File.display_name(), "File");
        assert_eq!(KeyBindingCategory::Edit.display_name(), "Edit");
        assert_eq!(KeyBindingCategory::View.display_name(), "View");
        assert_eq!(KeyBindingCategory::Simulation.display_name(), "Simulation");
    }

    #[test]
    fn test_keybinding_category_all() {
        let categories = KeyBindingCategory::all();
        assert_eq!(categories.len(), 7);
    }

    #[test]
    fn test_keybinding_preset_all() {
        let presets = KeyBindingPreset::all();
        assert_eq!(presets.len(), 4);
        assert!(presets.contains(&KeyBindingPreset::Default));
        assert!(presets.contains(&KeyBindingPreset::Maya));
        assert!(presets.contains(&KeyBindingPreset::Unity));
        assert!(presets.contains(&KeyBindingPreset::Custom));
    }

    #[test]
    fn test_keybinding_preset_display_names() {
        assert!(KeyBindingPreset::Default.display_name().contains("Blender"));
        assert!(KeyBindingPreset::Maya.display_name().contains("Maya"));
        assert!(KeyBindingPreset::Unity.display_name().contains("Unity"));
        assert!(KeyBindingPreset::Custom.display_name().contains("Custom"));
    }

    #[test]
    fn test_all_actions_have_descriptions() {
        for action in KeyBindingAction::all() {
            let name = action.display_name();
            let desc = action.description();
            assert!(
                !name.is_empty(),
                "Action {:?} has empty display name",
                action
            );
            assert!(
                !desc.is_empty(),
                "Action {:?} has empty description",
                action
            );
        }
    }

    #[test]
    fn test_check_keybindings_function() {
        let map = KeyBindingMap::default();
        let actions = vec![
            KeyBindingAction::SaveScene,
            KeyBindingAction::Undo,
            KeyBindingAction::CameraOrbit,
        ];

        // Without actual keyboard input, this will return empty
        // This just tests the function compiles and runs
        let keyboard = ButtonInput::<KeyCode>::default();
        let triggered = check_keybindings(&actions, &map, &keyboard);
        assert!(triggered.is_empty()); // No keys pressed
    }

    #[test]
    fn test_key_modifiers_new() {
        let mods = KeyModifiers::new(true, false, true, false);
        assert!(mods.ctrl);
        assert!(!mods.alt);
        assert!(mods.shift);
        assert!(!mods.super_key);
    }

    #[test]
    fn test_keybinding_map_all_bindings() {
        let map = KeyBindingMap::default();
        let all = map.all_bindings().collect::<Vec<_>>();
        assert!(!all.is_empty());

        // Should have bindings from all categories
        let has_camera = all
            .iter()
            .any(|b| b.category() == KeyBindingCategory::Camera);
        let has_file = all.iter().any(|b| b.category() == KeyBindingCategory::File);
        assert!(has_camera);
        assert!(has_file);
    }

    #[test]
    fn test_keybindings_plugin_creation() {
        let plugin = KeyBindingsPlugin::new();
        assert_eq!(plugin.preset, KeyBindingPreset::Default);
        assert!(plugin.config_path.is_none());

        let plugin_with_preset = KeyBindingsPlugin::with_preset(KeyBindingPreset::Maya);
        assert_eq!(plugin_with_preset.preset, KeyBindingPreset::Maya);

        let plugin_with_config = KeyBindingsPlugin::with_config("/path/to/config.json");
        assert!(plugin_with_config.config_path.is_some());
    }

    #[test]
    fn test_keybinding_has_conflicts() {
        let mut map = KeyBindingMap::default();

        // Add explicit conflict
        map.set(
            KeyBindingAction::CameraOrbit,
            KeyCombination::ctrl(KeyCode::KeyS),
        );

        // Should detect conflict with SaveScene
        assert!(map.has_conflicts());
    }

    #[test]
    fn test_key_combination_equality() {
        let a = KeyCombination::ctrl(KeyCode::KeyS);
        let b = KeyCombination::ctrl(KeyCode::KeyS);
        let c = KeyCombination::ctrl(KeyCode::KeyA);
        let d = KeyCombination::alt(KeyCode::KeyS);

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn test_key_modifiers_equality() {
        let a = KeyModifiers::CTRL;
        let b = KeyModifiers::CTRL;
        let c = KeyModifiers::ALT;
        let d = KeyModifiers::CTRL_SHIFT;

        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn test_keycode_serialization() {
        // Test keycode to string conversion
        assert_eq!(keycode_to_string(KeyCode::KeyA), "KeyA");
        assert_eq!(keycode_to_string(KeyCode::F1), "F1");
        assert_eq!(keycode_to_string(KeyCode::Space), "Space");
        assert_eq!(keycode_to_string(KeyCode::ControlLeft), "ControlLeft");

        // Test string to keycode conversion
        assert_eq!(string_to_keycode("KeyA"), Some(KeyCode::KeyA));
        assert_eq!(string_to_keycode("F1"), Some(KeyCode::F1));
        assert_eq!(string_to_keycode("Space"), Some(KeyCode::Space));
        assert_eq!(string_to_keycode("InvalidKey"), None);
    }

    #[test]
    fn test_serializable_keycode() {
        let skc = SerializableKeyCode(KeyCode::KeyA);
        assert_eq!(skc.0, KeyCode::KeyA);

        // Test From implementations
        let skc2: SerializableKeyCode = KeyCode::KeyB.into();
        assert_eq!(skc2.0, KeyCode::KeyB);

        let kc: KeyCode = skc.into();
        assert_eq!(kc, KeyCode::KeyA);
    }
}
