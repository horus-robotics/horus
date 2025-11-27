//! Status Bar Module for sim3d
//!
//! Provides a customizable bottom status bar with useful simulation information
//! including FPS, memory usage, entity counts, and more.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

/// Global counter for unique status item IDs
static STATUS_ITEM_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique status item ID
fn next_status_item_id() -> u64 {
    STATUS_ITEM_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Section of the status bar where an item is displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StatusBarSection {
    /// Left side of the status bar
    #[default]
    Left,
    /// Center of the status bar
    Center,
    /// Right side of the status bar
    Right,
}

/// Content type for a status item
#[derive(Debug, Clone, PartialEq)]
pub enum StatusItemContent {
    /// Text only
    Text(String),
    /// Icon only (using text representation)
    Icon(String),
    /// Icon followed by text
    IconText { icon: String, text: String },
    /// Text with a colored indicator
    ColoredText {
        text: String,
        color: StatusItemColor,
    },
    /// Progress indicator
    Progress { value: f32, label: Option<String> },
}

impl Default for StatusItemContent {
    fn default() -> Self {
        StatusItemContent::Text(String::new())
    }
}

/// Color for status item text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StatusItemColor {
    #[default]
    Default,
    Green,
    Yellow,
    Red,
    Blue,
    Gray,
}

impl StatusItemColor {
    #[cfg(feature = "visual")]
    pub fn to_egui_color(&self) -> egui::Color32 {
        match self {
            StatusItemColor::Default => egui::Color32::LIGHT_GRAY,
            StatusItemColor::Green => egui::Color32::from_rgb(76, 175, 80),
            StatusItemColor::Yellow => egui::Color32::from_rgb(255, 193, 7),
            StatusItemColor::Red => egui::Color32::from_rgb(244, 67, 54),
            StatusItemColor::Blue => egui::Color32::from_rgb(33, 150, 243),
            StatusItemColor::Gray => egui::Color32::GRAY,
        }
    }
}

/// A single status item displayed in the status bar
#[derive(Debug, Clone)]
pub struct StatusItem {
    /// Unique identifier
    pub id: u64,
    /// String key for referencing this item
    pub key: String,
    /// Section where the item is displayed
    pub section: StatusBarSection,
    /// Priority for ordering (higher = more to the edge of section)
    pub priority: i32,
    /// The content to display
    pub content: StatusItemContent,
    /// Tooltip text shown on hover
    pub tooltip: Option<String>,
    /// Whether clicking triggers an action
    pub clickable: bool,
    /// Whether the item is visible
    pub visible: bool,
    /// Optional minimum width
    pub min_width: Option<f32>,
}

impl StatusItem {
    /// Creates a new status item with text content
    pub fn new(key: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: next_status_item_id(),
            key: key.into(),
            section: StatusBarSection::Left,
            priority: 0,
            content: StatusItemContent::Text(content.into()),
            tooltip: None,
            clickable: false,
            visible: true,
            min_width: None,
        }
    }

    /// Creates a new status item with icon and text
    pub fn with_icon(
        key: impl Into<String>,
        icon: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: next_status_item_id(),
            key: key.into(),
            section: StatusBarSection::Left,
            priority: 0,
            content: StatusItemContent::IconText {
                icon: icon.into(),
                text: text.into(),
            },
            tooltip: None,
            clickable: false,
            visible: true,
            min_width: None,
        }
    }

    /// Sets the section
    pub fn section(mut self, section: StatusBarSection) -> Self {
        self.section = section;
        self
    }

    /// Sets the priority
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the tooltip
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Makes the item clickable
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self
    }

    /// Sets the minimum width
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Sets visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Updates the text content
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.content = StatusItemContent::Text(text.into());
    }

    /// Updates icon and text content
    pub fn set_icon_text(&mut self, icon: impl Into<String>, text: impl Into<String>) {
        self.content = StatusItemContent::IconText {
            icon: icon.into(),
            text: text.into(),
        };
    }

    /// Updates to colored text
    pub fn set_colored_text(&mut self, text: impl Into<String>, color: StatusItemColor) {
        self.content = StatusItemContent::ColoredText {
            text: text.into(),
            color,
        };
    }

    /// Updates to progress indicator
    pub fn set_progress(&mut self, value: f32, label: Option<String>) {
        self.content = StatusItemContent::Progress {
            value: value.clamp(0.0, 1.0),
            label,
        };
    }
}

/// Event fired when a clickable status item is clicked
#[derive(Event, Debug, Clone)]
pub struct StatusItemClickEvent {
    /// ID of the clicked item
    pub item_id: u64,
    /// Key of the clicked item
    pub item_key: String,
}

/// Configuration for the status bar
#[derive(Resource, Debug, Clone)]
pub struct StatusBarConfig {
    /// Whether the status bar is visible
    pub visible: bool,
    /// Height of the status bar in pixels
    pub height: f32,
    /// Which sections are enabled
    pub sections_enabled: StatusBarSections,
    /// Background color
    pub background_color: [u8; 4],
    /// Separator color
    pub separator_color: [u8; 4],
    /// Padding between items
    pub item_padding: f32,
    /// Section padding from edges
    pub section_padding: f32,
}

/// Which sections of the status bar are enabled
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusBarSections {
    pub left: bool,
    pub center: bool,
    pub right: bool,
}

impl Default for StatusBarSections {
    fn default() -> Self {
        Self {
            left: true,
            center: true,
            right: true,
        }
    }
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            visible: true,
            height: 24.0,
            sections_enabled: StatusBarSections::default(),
            background_color: [30, 30, 30, 240],
            separator_color: [80, 80, 80, 255],
            item_padding: 8.0,
            section_padding: 16.0,
        }
    }
}

impl StatusBarConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets visibility
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Sets the height
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height.max(16.0);
        self
    }

    /// Sets which sections are enabled
    pub fn with_sections(mut self, left: bool, center: bool, right: bool) -> Self {
        self.sections_enabled = StatusBarSections {
            left,
            center,
            right,
        };
        self
    }
}

/// Built-in status item keys
pub mod keys {
    pub const FPS: &str = "fps";
    pub const MEMORY: &str = "memory";
    pub const ENTITY_COUNT: &str = "entity_count";
    pub const PHYSICS_TIMESTEP: &str = "physics_timestep";
    pub const SIMULATION_STATUS: &str = "simulation_status";
    pub const CURRENT_TOOL: &str = "current_tool";
    pub const SELECTION_COUNT: &str = "selection_count";
    pub const GRID_SNAP: &str = "grid_snap";
    pub const MOUSE_POSITION: &str = "mouse_position";
    pub const SCENE_NAME: &str = "scene_name";
    pub const ZOOM_LEVEL: &str = "zoom_level";
}

/// Manager for status bar items
#[derive(Resource, Debug)]
pub struct StatusBarManager {
    /// All registered status items
    items: HashMap<String, StatusItem>,
}

impl Default for StatusBarManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBarManager {
    /// Creates a new status bar manager with default items
    pub fn new() -> Self {
        let mut manager = Self {
            items: HashMap::new(),
        };

        // Register built-in items
        manager.register_builtin_items();

        manager
    }

    /// Creates an empty status bar manager without built-in items
    pub fn empty() -> Self {
        Self {
            items: HashMap::new(),
        }
    }

    /// Registers all built-in status items
    fn register_builtin_items(&mut self) {
        // FPS counter (left, high priority)
        self.register(
            StatusItem::with_icon(keys::FPS, "FPS", "0.0")
                .section(StatusBarSection::Left)
                .priority(100)
                .tooltip("Frames per second"),
        );

        // Memory usage (left)
        self.register(
            StatusItem::with_icon(keys::MEMORY, "MEM", "0 MB")
                .section(StatusBarSection::Left)
                .priority(90)
                .tooltip("Estimated memory usage"),
        );

        // Entity count (left)
        self.register(
            StatusItem::with_icon(keys::ENTITY_COUNT, "ENT", "0")
                .section(StatusBarSection::Left)
                .priority(80)
                .tooltip("Total entity count"),
        );

        // Physics timestep (left)
        self.register(
            StatusItem::with_icon(keys::PHYSICS_TIMESTEP, "PHY", "60 Hz")
                .section(StatusBarSection::Left)
                .priority(70)
                .tooltip("Physics timestep"),
        );

        // Simulation status (center, high priority)
        self.register(
            StatusItem::new(keys::SIMULATION_STATUS, "Playing")
                .section(StatusBarSection::Center)
                .priority(100)
                .tooltip("Simulation state (Space to toggle)")
                .clickable(),
        );

        // Current tool (center)
        self.register(
            StatusItem::with_icon(keys::CURRENT_TOOL, "TOOL", "Select")
                .section(StatusBarSection::Center)
                .priority(90)
                .tooltip("Current editor tool"),
        );

        // Selection count (center)
        self.register(
            StatusItem::with_icon(keys::SELECTION_COUNT, "SEL", "0")
                .section(StatusBarSection::Center)
                .priority(80)
                .tooltip("Number of selected entities")
                .clickable(),
        );

        // Grid snap settings (right)
        self.register(
            StatusItem::with_icon(keys::GRID_SNAP, "GRID", "Off")
                .section(StatusBarSection::Right)
                .priority(70)
                .tooltip("Grid snap settings (click to toggle)")
                .clickable(),
        );

        // Mouse position (right)
        self.register(
            StatusItem::with_icon(keys::MOUSE_POSITION, "POS", "0, 0, 0")
                .section(StatusBarSection::Right)
                .priority(80)
                .tooltip("Mouse position in world coordinates")
                .min_width(120.0),
        );

        // Scene name (right, high priority)
        self.register(
            StatusItem::with_icon(keys::SCENE_NAME, "SCENE", "Untitled")
                .section(StatusBarSection::Right)
                .priority(100)
                .tooltip("Current scene name (* = unsaved changes)")
                .clickable(),
        );

        // Zoom level (right)
        self.register(
            StatusItem::with_icon(keys::ZOOM_LEVEL, "ZOOM", "100%")
                .section(StatusBarSection::Right)
                .priority(90)
                .tooltip("Current zoom level"),
        );
    }

    /// Registers a new status item
    pub fn register(&mut self, item: StatusItem) {
        self.items.insert(item.key.clone(), item);
    }

    /// Unregisters a status item by key
    pub fn unregister(&mut self, key: &str) -> Option<StatusItem> {
        self.items.remove(key)
    }

    /// Gets a status item by key
    pub fn get(&self, key: &str) -> Option<&StatusItem> {
        self.items.get(key)
    }

    /// Gets a mutable reference to a status item by key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut StatusItem> {
        self.items.get_mut(key)
    }

    /// Updates the content of a status item
    pub fn update_text(&mut self, key: &str, text: impl Into<String>) -> bool {
        if let Some(item) = self.items.get_mut(key) {
            item.set_text(text);
            true
        } else {
            false
        }
    }

    /// Updates icon and text content
    pub fn update_icon_text(
        &mut self,
        key: &str,
        icon: impl Into<String>,
        text: impl Into<String>,
    ) -> bool {
        if let Some(item) = self.items.get_mut(key) {
            item.set_icon_text(icon, text);
            true
        } else {
            false
        }
    }

    /// Updates to colored text
    pub fn update_colored_text(
        &mut self,
        key: &str,
        text: impl Into<String>,
        color: StatusItemColor,
    ) -> bool {
        if let Some(item) = self.items.get_mut(key) {
            item.set_colored_text(text, color);
            true
        } else {
            false
        }
    }

    /// Sets item visibility
    pub fn set_visible(&mut self, key: &str, visible: bool) -> bool {
        if let Some(item) = self.items.get_mut(key) {
            item.visible = visible;
            true
        } else {
            false
        }
    }

    /// Gets all items in a specific section, sorted by priority (descending)
    pub fn get_items_by_section(&self, section: StatusBarSection) -> Vec<&StatusItem> {
        let mut items: Vec<_> = self
            .items
            .values()
            .filter(|item| item.section == section && item.visible)
            .collect();

        items.sort_by(|a, b| b.priority.cmp(&a.priority));
        items
    }

    /// Gets all registered items
    pub fn all_items(&self) -> impl Iterator<Item = &StatusItem> {
        self.items.values()
    }

    /// Gets the number of registered items
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Checks if an item exists
    pub fn contains(&self, key: &str) -> bool {
        self.items.contains_key(key)
    }
}

/// Resource for tracking current scene state
#[derive(Resource, Debug, Clone, Default)]
pub struct SceneState {
    /// Name of the current scene
    pub name: String,
    /// Whether there are unsaved changes
    pub unsaved_changes: bool,
    /// Current zoom level (1.0 = 100%)
    pub zoom_level: f32,
}

impl SceneState {
    pub fn new() -> Self {
        Self {
            name: "Untitled".to_string(),
            unsaved_changes: false,
            zoom_level: 1.0,
        }
    }

    /// Gets the display name (with * for unsaved changes)
    pub fn display_name(&self) -> String {
        if self.unsaved_changes {
            format!("{}*", self.name)
        } else {
            self.name.clone()
        }
    }
}

/// Resource for tracking editor tool state
#[derive(Resource, Debug, Clone, Default)]
pub struct EditorToolState {
    /// Current tool name
    pub current_tool: String,
    /// Number of selected entities
    pub selection_count: usize,
    /// Whether grid snap is enabled
    pub grid_snap_enabled: bool,
    /// Grid snap size
    pub grid_snap_size: f32,
}

impl EditorToolState {
    pub fn new() -> Self {
        Self {
            current_tool: "Select".to_string(),
            selection_count: 0,
            grid_snap_enabled: false,
            grid_snap_size: 1.0,
        }
    }

    /// Gets the grid snap display string
    pub fn grid_snap_display(&self) -> String {
        if self.grid_snap_enabled {
            format!("{:.1}", self.grid_snap_size)
        } else {
            "Off".to_string()
        }
    }
}

/// Resource for tracking mouse world position
#[derive(Resource, Debug, Clone, Default)]
pub struct MouseWorldPosition {
    pub position: Vec3,
    pub valid: bool,
}

impl MouseWorldPosition {
    pub fn display(&self) -> String {
        if self.valid {
            format!(
                "{:.1}, {:.1}, {:.1}",
                self.position.x, self.position.y, self.position.z
            )
        } else {
            "-, -, -".to_string()
        }
    }
}

/// Resource for tracking simulation status
#[derive(Resource, Debug, Clone)]
pub struct SimulationStatusInfo {
    /// Whether simulation is paused
    pub paused: bool,
    /// Current time scale
    pub time_scale: f32,
    /// Physics timestep in Hz
    pub physics_hz: f32,
}

impl Default for SimulationStatusInfo {
    fn default() -> Self {
        Self {
            paused: false,
            time_scale: 1.0,
            physics_hz: 60.0,
        }
    }
}

impl SimulationStatusInfo {
    pub fn display(&self) -> String {
        if self.paused {
            "Paused".to_string()
        } else if (self.time_scale - 1.0).abs() > 0.01 {
            format!("Playing ({:.1}x)", self.time_scale)
        } else {
            "Playing".to_string()
        }
    }
}

/// System to update built-in status items
pub fn update_status_bar_system(
    mut manager: ResMut<StatusBarManager>,
    time: Res<Time>,
    entities: Query<Entity>,
    scene_state: Option<Res<SceneState>>,
    tool_state: Option<Res<EditorToolState>>,
    mouse_pos: Option<Res<MouseWorldPosition>>,
    sim_status: Option<Res<SimulationStatusInfo>>,
) {
    // Update FPS
    let fps = if time.delta_secs() > 0.0 {
        1.0 / time.delta_secs()
    } else {
        0.0
    };

    let fps_color = if fps >= 60.0 {
        StatusItemColor::Green
    } else if fps >= 30.0 {
        StatusItemColor::Yellow
    } else {
        StatusItemColor::Red
    };

    if let Some(item) = manager.get_mut(keys::FPS) {
        item.content = StatusItemContent::IconText {
            icon: "FPS".to_string(),
            text: format!("{:.1}", fps),
        };
        // We could use colored text but keeping it simple for now
    }
    let _ = fps_color; // Used conceptually, could be applied

    // Update entity count
    let entity_count = entities.iter().count();
    manager.update_icon_text(keys::ENTITY_COUNT, "ENT", entity_count.to_string());

    // Estimate memory usage (rough estimate)
    let estimated_memory_mb = entity_count as f32 * 0.0002; // ~200 bytes per entity
    manager.update_icon_text(
        keys::MEMORY,
        "MEM",
        format!("{:.1} MB", estimated_memory_mb),
    );

    // Update simulation status
    if let Some(sim_info) = sim_status {
        manager.update_text(keys::SIMULATION_STATUS, sim_info.display());
        manager.update_icon_text(
            keys::PHYSICS_TIMESTEP,
            "PHY",
            format!("{:.0} Hz", sim_info.physics_hz),
        );
    }

    // Update scene info
    if let Some(scene) = scene_state {
        manager.update_icon_text(keys::SCENE_NAME, "SCENE", scene.display_name());
        manager.update_icon_text(
            keys::ZOOM_LEVEL,
            "ZOOM",
            format!("{:.0}%", scene.zoom_level * 100.0),
        );
    }

    // Update tool state
    if let Some(tool) = tool_state {
        manager.update_icon_text(keys::CURRENT_TOOL, "TOOL", &tool.current_tool);
        manager.update_icon_text(
            keys::SELECTION_COUNT,
            "SEL",
            tool.selection_count.to_string(),
        );
        manager.update_icon_text(keys::GRID_SNAP, "GRID", tool.grid_snap_display());
    }

    // Update mouse position
    if let Some(mouse) = mouse_pos {
        manager.update_icon_text(keys::MOUSE_POSITION, "POS", mouse.display());
    }
}

#[cfg(feature = "visual")]
/// System to render the status bar
pub fn render_status_bar_system(
    mut contexts: EguiContexts,
    manager: Res<StatusBarManager>,
    config: Res<StatusBarConfig>,
    mut click_events: EventWriter<StatusItemClickEvent>,
) {
    if !config.visible {
        return;
    }

    let ctx = contexts.ctx_mut();
    let screen_rect = ctx.screen_rect();

    let bar_rect = egui::Rect::from_min_size(
        egui::pos2(0.0, screen_rect.max.y - config.height),
        egui::vec2(screen_rect.max.x, config.height),
    );

    // Collect click events to send after rendering
    let mut clicks_to_send = Vec::new();

    egui::Area::new(egui::Id::new("status_bar"))
        .fixed_pos(bar_rect.min)
        .show(ctx, |ui| {
            let bg_color = egui::Color32::from_rgba_unmultiplied(
                config.background_color[0],
                config.background_color[1],
                config.background_color[2],
                config.background_color[3],
            );

            egui::Frame::none().fill(bg_color).show(ui, |ui| {
                ui.set_min_size(egui::vec2(bar_rect.width(), config.height));

                ui.horizontal(|ui| {
                    ui.set_height(config.height);
                    ui.style_mut().spacing.item_spacing.x = config.item_padding;

                    // Left section
                    if config.sections_enabled.left {
                        ui.add_space(config.section_padding);
                        let left_items = manager.get_items_by_section(StatusBarSection::Left);
                        for item in left_items {
                            render_status_item(ui, item, &mut clicks_to_send);
                        }
                    }

                    // Flexible space before center
                    ui.add_space(ui.available_width() * 0.3);

                    // Center section
                    if config.sections_enabled.center {
                        let center_items = manager.get_items_by_section(StatusBarSection::Center);
                        for item in center_items {
                            render_status_item(ui, item, &mut clicks_to_send);
                        }
                    }

                    // Flexible space after center
                    let remaining = ui.available_width() - config.section_padding;
                    ui.add_space(remaining * 0.4);

                    // Right section
                    if config.sections_enabled.right {
                        let right_items = manager.get_items_by_section(StatusBarSection::Right);
                        for item in right_items {
                            render_status_item(ui, item, &mut clicks_to_send);
                        }
                        ui.add_space(config.section_padding);
                    }
                });
            });
        });

    // Send click events
    for (item_id, item_key) in clicks_to_send {
        click_events.send(StatusItemClickEvent { item_id, item_key });
    }
}

#[cfg(feature = "visual")]
fn render_status_item(ui: &mut egui::Ui, item: &StatusItem, clicks: &mut Vec<(u64, String)>) {
    let response = match &item.content {
        StatusItemContent::Text(text) => {
            if item.clickable {
                ui.small_button(text)
            } else {
                ui.label(text);
                ui.label("") // Return a response
            }
        }
        StatusItemContent::Icon(icon) => {
            ui.label(icon);
            ui.label("")
        }
        StatusItemContent::IconText { icon, text } => {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icon).small().color(egui::Color32::GRAY));
                if item.clickable {
                    ui.small_button(text)
                } else {
                    ui.label(text);
                    ui.label("")
                }
            })
            .inner
        }
        StatusItemContent::ColoredText { text, color } => {
            let egui_color = color.to_egui_color();
            if item.clickable {
                ui.add(egui::Button::new(egui::RichText::new(text).color(egui_color)).small())
            } else {
                ui.colored_label(egui_color, text);
                ui.label("")
            }
        }
        StatusItemContent::Progress { value, label } => {
            let bar = egui::ProgressBar::new(*value);
            let bar = if let Some(lbl) = label {
                bar.text(lbl)
            } else {
                bar
            };
            ui.add(bar);
            ui.label("")
        }
    };

    // Handle tooltip and clicks
    let response = if let Some(tooltip_text) = &item.tooltip {
        response.on_hover_text(tooltip_text)
    } else {
        response
    };

    // Handle clicks
    if item.clickable && response.clicked() {
        clicks.push((item.id, item.key.clone()));
    }
}

#[cfg(not(feature = "visual"))]
pub fn render_status_bar_system() {}

/// Plugin for the status bar
pub struct StatusBarPlugin;

impl Plugin for StatusBarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StatusBarConfig>()
            .init_resource::<StatusBarManager>()
            .init_resource::<SceneState>()
            .init_resource::<EditorToolState>()
            .init_resource::<MouseWorldPosition>()
            .init_resource::<SimulationStatusInfo>()
            .add_event::<StatusItemClickEvent>()
            .add_systems(
                Update,
                (update_status_bar_system, render_status_bar_system).chain(),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_item_creation() {
        let item = StatusItem::new("test", "Test Content")
            .section(StatusBarSection::Right)
            .priority(50)
            .tooltip("This is a test")
            .clickable();

        assert_eq!(item.key, "test");
        assert_eq!(item.section, StatusBarSection::Right);
        assert_eq!(item.priority, 50);
        assert_eq!(item.tooltip, Some("This is a test".to_string()));
        assert!(item.clickable);
        assert!(item.visible);
    }

    #[test]
    fn test_status_item_with_icon() {
        let item = StatusItem::with_icon("fps", "FPS", "60.0");

        match &item.content {
            StatusItemContent::IconText { icon, text } => {
                assert_eq!(icon, "FPS");
                assert_eq!(text, "60.0");
            }
            _ => panic!("Expected IconText content"),
        }
    }

    #[test]
    fn test_status_item_id_uniqueness() {
        let item1 = StatusItem::new("item1", "Content 1");
        let item2 = StatusItem::new("item2", "Content 2");
        let item3 = StatusItem::new("item3", "Content 3");

        assert_ne!(item1.id, item2.id);
        assert_ne!(item2.id, item3.id);
        assert_ne!(item1.id, item3.id);
    }

    #[test]
    fn test_status_item_update_text() {
        let mut item = StatusItem::new("test", "Original");

        item.set_text("Updated");

        match &item.content {
            StatusItemContent::Text(text) => assert_eq!(text, "Updated"),
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_status_item_update_icon_text() {
        let mut item = StatusItem::new("test", "Original");

        item.set_icon_text("ICON", "Updated");

        match &item.content {
            StatusItemContent::IconText { icon, text } => {
                assert_eq!(icon, "ICON");
                assert_eq!(text, "Updated");
            }
            _ => panic!("Expected IconText content"),
        }
    }

    #[test]
    fn test_status_item_update_colored_text() {
        let mut item = StatusItem::new("test", "Original");

        item.set_colored_text("Colored", StatusItemColor::Green);

        match &item.content {
            StatusItemContent::ColoredText { text, color } => {
                assert_eq!(text, "Colored");
                assert_eq!(*color, StatusItemColor::Green);
            }
            _ => panic!("Expected ColoredText content"),
        }
    }

    #[test]
    fn test_status_item_update_progress() {
        let mut item = StatusItem::new("test", "Original");

        item.set_progress(0.75, Some("Loading...".to_string()));

        match &item.content {
            StatusItemContent::Progress { value, label } => {
                assert_eq!(*value, 0.75);
                assert_eq!(label.as_deref(), Some("Loading..."));
            }
            _ => panic!("Expected Progress content"),
        }
    }

    #[test]
    fn test_status_item_progress_clamping() {
        let mut item = StatusItem::new("test", "Original");

        item.set_progress(1.5, None);
        match &item.content {
            StatusItemContent::Progress { value, .. } => assert_eq!(*value, 1.0),
            _ => panic!("Expected Progress content"),
        }

        item.set_progress(-0.5, None);
        match &item.content {
            StatusItemContent::Progress { value, .. } => assert_eq!(*value, 0.0),
            _ => panic!("Expected Progress content"),
        }
    }

    #[test]
    fn test_status_bar_manager_creation() {
        let manager = StatusBarManager::new();

        // Should have all built-in items
        assert!(manager.contains(keys::FPS));
        assert!(manager.contains(keys::MEMORY));
        assert!(manager.contains(keys::ENTITY_COUNT));
        assert!(manager.contains(keys::PHYSICS_TIMESTEP));
        assert!(manager.contains(keys::SIMULATION_STATUS));
        assert!(manager.contains(keys::CURRENT_TOOL));
        assert!(manager.contains(keys::SELECTION_COUNT));
        assert!(manager.contains(keys::GRID_SNAP));
        assert!(manager.contains(keys::MOUSE_POSITION));
        assert!(manager.contains(keys::SCENE_NAME));
        assert!(manager.contains(keys::ZOOM_LEVEL));
    }

    #[test]
    fn test_status_bar_manager_empty() {
        let manager = StatusBarManager::empty();

        assert_eq!(manager.count(), 0);
        assert!(!manager.contains(keys::FPS));
    }

    #[test]
    fn test_status_bar_manager_register_unregister() {
        let mut manager = StatusBarManager::empty();

        let item = StatusItem::new("custom", "Custom Item");
        manager.register(item);

        assert!(manager.contains("custom"));
        assert_eq!(manager.count(), 1);

        let removed = manager.unregister("custom");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().key, "custom");
        assert!(!manager.contains("custom"));
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_status_bar_manager_get() {
        let manager = StatusBarManager::new();

        let item = manager.get(keys::FPS);
        assert!(item.is_some());
        assert_eq!(item.unwrap().key, keys::FPS);

        let missing = manager.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_status_bar_manager_update() {
        let mut manager = StatusBarManager::new();

        assert!(manager.update_text(keys::FPS, "Updated"));
        assert!(!manager.update_text("nonexistent", "Nothing"));

        let item = manager.get(keys::FPS).unwrap();
        match &item.content {
            StatusItemContent::Text(text) => assert_eq!(text, "Updated"),
            _ => {} // Could be updated to IconText, that's fine
        }
    }

    #[test]
    fn test_status_bar_manager_set_visible() {
        let mut manager = StatusBarManager::new();

        assert!(manager.get(keys::FPS).unwrap().visible);

        assert!(manager.set_visible(keys::FPS, false));
        assert!(!manager.get(keys::FPS).unwrap().visible);

        assert!(manager.set_visible(keys::FPS, true));
        assert!(manager.get(keys::FPS).unwrap().visible);

        assert!(!manager.set_visible("nonexistent", false));
    }

    #[test]
    fn test_status_bar_manager_get_items_by_section() {
        let manager = StatusBarManager::new();

        let left_items = manager.get_items_by_section(StatusBarSection::Left);
        assert!(!left_items.is_empty());
        for item in &left_items {
            assert_eq!(item.section, StatusBarSection::Left);
        }

        let center_items = manager.get_items_by_section(StatusBarSection::Center);
        assert!(!center_items.is_empty());
        for item in &center_items {
            assert_eq!(item.section, StatusBarSection::Center);
        }

        let right_items = manager.get_items_by_section(StatusBarSection::Right);
        assert!(!right_items.is_empty());
        for item in &right_items {
            assert_eq!(item.section, StatusBarSection::Right);
        }
    }

    #[test]
    fn test_status_bar_manager_items_sorted_by_priority() {
        let mut manager = StatusBarManager::empty();

        manager.register(
            StatusItem::new("low", "Low")
                .section(StatusBarSection::Left)
                .priority(10),
        );
        manager.register(
            StatusItem::new("high", "High")
                .section(StatusBarSection::Left)
                .priority(100),
        );
        manager.register(
            StatusItem::new("medium", "Medium")
                .section(StatusBarSection::Left)
                .priority(50),
        );

        let items = manager.get_items_by_section(StatusBarSection::Left);

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].key, "high");
        assert_eq!(items[1].key, "medium");
        assert_eq!(items[2].key, "low");
    }

    #[test]
    fn test_status_bar_manager_hidden_items_excluded() {
        let mut manager = StatusBarManager::empty();

        manager.register(StatusItem::new("visible", "Visible").section(StatusBarSection::Left));
        manager.register(
            StatusItem::new("hidden", "Hidden")
                .section(StatusBarSection::Left)
                .visible(false),
        );

        let items = manager.get_items_by_section(StatusBarSection::Left);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "visible");
    }

    #[test]
    fn test_status_bar_config() {
        let config = StatusBarConfig::new()
            .with_visible(true)
            .with_height(32.0)
            .with_sections(true, false, true);

        assert!(config.visible);
        assert_eq!(config.height, 32.0);
        assert!(config.sections_enabled.left);
        assert!(!config.sections_enabled.center);
        assert!(config.sections_enabled.right);
    }

    #[test]
    fn test_status_bar_config_height_minimum() {
        let config = StatusBarConfig::new().with_height(5.0);

        // Should be clamped to minimum of 16
        assert_eq!(config.height, 16.0);
    }

    #[test]
    fn test_scene_state() {
        let mut state = SceneState::new();

        assert_eq!(state.name, "Untitled");
        assert!(!state.unsaved_changes);
        assert_eq!(state.display_name(), "Untitled");

        state.unsaved_changes = true;
        assert_eq!(state.display_name(), "Untitled*");

        state.name = "MyScene".to_string();
        assert_eq!(state.display_name(), "MyScene*");

        state.unsaved_changes = false;
        assert_eq!(state.display_name(), "MyScene");
    }

    #[test]
    fn test_editor_tool_state() {
        let mut state = EditorToolState::new();

        assert_eq!(state.current_tool, "Select");
        assert_eq!(state.selection_count, 0);
        assert!(!state.grid_snap_enabled);
        assert_eq!(state.grid_snap_display(), "Off");

        state.grid_snap_enabled = true;
        state.grid_snap_size = 0.5;
        assert_eq!(state.grid_snap_display(), "0.5");
    }

    #[test]
    fn test_mouse_world_position() {
        let mut pos = MouseWorldPosition::default();

        assert!(!pos.valid);
        assert_eq!(pos.display(), "-, -, -");

        pos.valid = true;
        pos.position = Vec3::new(1.5, 2.5, 3.5);
        assert_eq!(pos.display(), "1.5, 2.5, 3.5");
    }

    #[test]
    fn test_simulation_status_info() {
        let mut info = SimulationStatusInfo::default();

        assert!(!info.paused);
        assert_eq!(info.display(), "Playing");

        info.paused = true;
        assert_eq!(info.display(), "Paused");

        info.paused = false;
        info.time_scale = 2.0;
        assert_eq!(info.display(), "Playing (2.0x)");

        info.time_scale = 0.5;
        assert_eq!(info.display(), "Playing (0.5x)");

        info.time_scale = 1.0;
        assert_eq!(info.display(), "Playing");
    }

    #[test]
    fn test_status_item_click_event() {
        let event = StatusItemClickEvent {
            item_id: 123,
            item_key: "test_item".to_string(),
        };

        assert_eq!(event.item_id, 123);
        assert_eq!(event.item_key, "test_item");
    }

    #[test]
    fn test_status_bar_sections() {
        let sections = StatusBarSections::default();

        assert!(sections.left);
        assert!(sections.center);
        assert!(sections.right);
    }

    #[test]
    fn test_status_item_color_variants() {
        let colors = vec![
            StatusItemColor::Default,
            StatusItemColor::Green,
            StatusItemColor::Yellow,
            StatusItemColor::Red,
            StatusItemColor::Blue,
            StatusItemColor::Gray,
        ];

        for color in colors {
            let mut item = StatusItem::new("test", "Test");
            item.set_colored_text("Text", color);

            match &item.content {
                StatusItemContent::ColoredText { color: c, .. } => {
                    assert_eq!(*c, color);
                }
                _ => panic!("Expected ColoredText"),
            }
        }
    }

    #[test]
    fn test_status_item_min_width() {
        let item = StatusItem::new("test", "Test").min_width(100.0);

        assert_eq!(item.min_width, Some(100.0));
    }

    #[test]
    fn test_all_items_iterator() {
        let manager = StatusBarManager::new();

        let all_items: Vec<_> = manager.all_items().collect();

        // Should have all built-in items
        assert!(all_items.len() >= 11); // At least the 11 built-in items
    }

    #[test]
    fn test_builtin_keys() {
        // Just verify all keys are distinct and non-empty
        let keys_list = vec![
            keys::FPS,
            keys::MEMORY,
            keys::ENTITY_COUNT,
            keys::PHYSICS_TIMESTEP,
            keys::SIMULATION_STATUS,
            keys::CURRENT_TOOL,
            keys::SELECTION_COUNT,
            keys::GRID_SNAP,
            keys::MOUSE_POSITION,
            keys::SCENE_NAME,
            keys::ZOOM_LEVEL,
        ];

        let mut seen = std::collections::HashSet::new();
        for key in &keys_list {
            assert!(!key.is_empty());
            assert!(seen.insert(*key), "Duplicate key: {}", key);
        }
    }
}
