//! Tooltip and contextual help system for sim3d.
//!
//! This module provides a comprehensive tooltip system with support for
//! contextual help, keyboard shortcut hints, and help overlays.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

/// Style variants for tooltips
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TooltipStyle {
    /// Standard informational tooltip
    #[default]
    Info,
    /// Warning tooltip (yellow/orange theme)
    Warning,
    /// Error tooltip (red theme)
    Error,
    /// Success tooltip (green theme)
    Success,
    /// Hint tooltip (blue theme, for keyboard shortcuts)
    Hint,
}

impl TooltipStyle {
    /// Get the background color for this style
    #[cfg(feature = "visual")]
    pub fn background_color(&self) -> egui::Color32 {
        match self {
            TooltipStyle::Info => egui::Color32::from_rgba_unmultiplied(40, 40, 40, 240),
            TooltipStyle::Warning => egui::Color32::from_rgba_unmultiplied(80, 60, 20, 240),
            TooltipStyle::Error => egui::Color32::from_rgba_unmultiplied(80, 30, 30, 240),
            TooltipStyle::Success => egui::Color32::from_rgba_unmultiplied(30, 70, 40, 240),
            TooltipStyle::Hint => egui::Color32::from_rgba_unmultiplied(30, 50, 80, 240),
        }
    }

    /// Get the text color for this style
    #[cfg(feature = "visual")]
    pub fn text_color(&self) -> egui::Color32 {
        match self {
            TooltipStyle::Info => egui::Color32::WHITE,
            TooltipStyle::Warning => egui::Color32::from_rgb(255, 220, 100),
            TooltipStyle::Error => egui::Color32::from_rgb(255, 150, 150),
            TooltipStyle::Success => egui::Color32::from_rgb(150, 255, 150),
            TooltipStyle::Hint => egui::Color32::from_rgb(150, 200, 255),
        }
    }

    /// Get the border color for this style
    #[cfg(feature = "visual")]
    pub fn border_color(&self) -> egui::Color32 {
        match self {
            TooltipStyle::Info => egui::Color32::from_rgb(80, 80, 80),
            TooltipStyle::Warning => egui::Color32::from_rgb(200, 150, 50),
            TooltipStyle::Error => egui::Color32::from_rgb(200, 80, 80),
            TooltipStyle::Success => egui::Color32::from_rgb(80, 180, 100),
            TooltipStyle::Hint => egui::Color32::from_rgb(80, 120, 200),
        }
    }

    /// Get the icon for this style
    pub fn icon(&self) -> &'static str {
        match self {
            TooltipStyle::Info => "i",
            TooltipStyle::Warning => "!",
            TooltipStyle::Error => "X",
            TooltipStyle::Success => "*",
            TooltipStyle::Hint => "?",
        }
    }
}

/// Position mode for tooltips
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TooltipPosition {
    /// Follow the cursor
    #[default]
    FollowCursor,
    /// Fixed position on screen
    Fixed { x: i32, y: i32 },
    /// Relative to a UI element (offset from element's position)
    RelativeToElement { offset_x: i32, offset_y: i32 },
    /// Above the cursor/element
    Above,
    /// Below the cursor/element
    Below,
    /// Left of the cursor/element
    Left,
    /// Right of the cursor/element
    Right,
}

/// A single tooltip definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tooltip {
    /// Unique identifier for this tooltip
    pub id: String,
    /// Text content (supports basic markdown: **bold**, *italic*, `code`)
    pub text: String,
    /// Title (optional)
    pub title: Option<String>,
    /// Position mode
    pub position: TooltipPosition,
    /// Delay before showing (milliseconds)
    pub show_delay_ms: u64,
    /// Duration to show (milliseconds, 0 for until mouse leaves)
    pub duration_ms: u64,
    /// Visual style
    pub style: TooltipStyle,
    /// Maximum width in pixels
    pub max_width: f32,
    /// Whether this tooltip is enabled
    pub enabled: bool,
}

impl Default for Tooltip {
    fn default() -> Self {
        Self {
            id: String::new(),
            text: String::new(),
            title: None,
            position: TooltipPosition::FollowCursor,
            show_delay_ms: 500,
            duration_ms: 0,
            style: TooltipStyle::Info,
            max_width: 300.0,
            enabled: true,
        }
    }
}

impl Tooltip {
    /// Create a new tooltip with text
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            ..Default::default()
        }
    }

    /// Builder pattern: set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Builder pattern: set position
    pub fn with_position(mut self, position: TooltipPosition) -> Self {
        self.position = position;
        self
    }

    /// Builder pattern: set show delay
    pub fn with_show_delay(mut self, delay_ms: u64) -> Self {
        self.show_delay_ms = delay_ms;
        self
    }

    /// Builder pattern: set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Builder pattern: set style
    pub fn with_style(mut self, style: TooltipStyle) -> Self {
        self.style = style;
        self
    }

    /// Builder pattern: set max width
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Builder pattern: set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Get the show delay as Duration
    pub fn show_delay(&self) -> Duration {
        Duration::from_millis(self.show_delay_ms)
    }

    /// Get the duration as Duration
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    /// Parse markdown-like formatting in text
    pub fn formatted_text(&self) -> Vec<TextSegment> {
        parse_markdown_text(&self.text)
    }
}

/// Text segment for formatted tooltip text
#[derive(Debug, Clone)]
pub enum TextSegment {
    Normal(String),
    Bold(String),
    Italic(String),
    Code(String),
}

/// Parse basic markdown formatting
fn parse_markdown_text(text: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    // Bold
                    chars.next();
                    if !current.is_empty() {
                        segments.push(TextSegment::Normal(current.clone()));
                        current.clear();
                    }
                    let mut bold_text = String::new();
                    while let Some(bc) = chars.next() {
                        if bc == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(bc);
                    }
                    if !bold_text.is_empty() {
                        segments.push(TextSegment::Bold(bold_text));
                    }
                } else {
                    // Italic
                    if !current.is_empty() {
                        segments.push(TextSegment::Normal(current.clone()));
                        current.clear();
                    }
                    let mut italic_text = String::new();
                    for ic in chars.by_ref() {
                        if ic == '*' {
                            break;
                        }
                        italic_text.push(ic);
                    }
                    if !italic_text.is_empty() {
                        segments.push(TextSegment::Italic(italic_text));
                    }
                }
            }
            '`' => {
                // Code
                if !current.is_empty() {
                    segments.push(TextSegment::Normal(current.clone()));
                    current.clear();
                }
                let mut code_text = String::new();
                for cc in chars.by_ref() {
                    if cc == '`' {
                        break;
                    }
                    code_text.push(cc);
                }
                if !code_text.is_empty() {
                    segments.push(TextSegment::Code(code_text));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        segments.push(TextSegment::Normal(current));
    }

    if segments.is_empty() {
        segments.push(TextSegment::Normal(text.to_string()));
    }

    segments
}

/// Global tooltip configuration
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct TooltipConfig {
    /// Enable/disable all tooltips globally
    pub enabled: bool,
    /// Default show delay (milliseconds)
    pub show_delay_ms: u64,
    /// Hide delay after mouse leaves (milliseconds)
    pub hide_delay_ms: u64,
    /// Default maximum width
    pub max_width: f32,
    /// Tooltip opacity (0.0 - 1.0)
    pub opacity: f32,
    /// Font size multiplier
    pub font_scale: f32,
    /// Enable animations
    pub animate: bool,
    /// Animation duration (milliseconds)
    pub animation_duration_ms: u64,
    /// Offset from cursor
    pub cursor_offset: [f32; 2],
}

impl Default for TooltipConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_delay_ms: 500,
            hide_delay_ms: 200,
            max_width: 300.0,
            opacity: 0.95,
            font_scale: 1.0,
            animate: true,
            animation_duration_ms: 150,
            cursor_offset: [15.0, 15.0],
        }
    }
}

impl TooltipConfig {
    /// Create new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder pattern: set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Builder pattern: set show delay
    pub fn with_show_delay(mut self, delay_ms: u64) -> Self {
        self.show_delay_ms = delay_ms;
        self
    }

    /// Builder pattern: set hide delay
    pub fn with_hide_delay(mut self, delay_ms: u64) -> Self {
        self.hide_delay_ms = delay_ms;
        self
    }

    /// Builder pattern: set max width
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Builder pattern: set opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }
}

/// Registry for tooltips mapped to UI element IDs
#[derive(Resource, Default)]
pub struct TooltipRegistry {
    /// Map of element ID to tooltip
    tooltips: HashMap<String, Tooltip>,
    /// Currently active tooltip ID
    active_tooltip: Option<String>,
    /// Time when hover started (for delay calculation)
    hover_start: Option<f64>,
    /// Current cursor position
    cursor_position: [f32; 2],
}

impl TooltipRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tooltip for a UI element
    pub fn register(&mut self, tooltip: Tooltip) {
        self.tooltips.insert(tooltip.id.clone(), tooltip);
    }

    /// Register a simple tooltip with just text
    pub fn register_simple(&mut self, id: impl Into<String>, text: impl Into<String>) {
        let tooltip = Tooltip::new(id, text);
        self.tooltips.insert(tooltip.id.clone(), tooltip);
    }

    /// Unregister a tooltip
    pub fn unregister(&mut self, id: &str) -> Option<Tooltip> {
        self.tooltips.remove(id)
    }

    /// Get a tooltip by ID
    pub fn get(&self, id: &str) -> Option<&Tooltip> {
        self.tooltips.get(id)
    }

    /// Get mutable reference to a tooltip
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Tooltip> {
        self.tooltips.get_mut(id)
    }

    /// Check if a tooltip exists
    pub fn has(&self, id: &str) -> bool {
        self.tooltips.contains_key(id)
    }

    /// Get all registered tooltip IDs
    pub fn ids(&self) -> Vec<&String> {
        self.tooltips.keys().collect()
    }

    /// Set the active tooltip
    pub fn set_active(&mut self, id: Option<String>, current_time: f64) {
        if self.active_tooltip != id {
            self.active_tooltip = id;
            self.hover_start = Some(current_time);
        }
    }

    /// Get the currently active tooltip
    pub fn active(&self) -> Option<&Tooltip> {
        self.active_tooltip
            .as_ref()
            .and_then(|id| self.tooltips.get(id))
    }

    /// Update cursor position
    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor_position = [x, y];
    }

    /// Check if delay has passed for active tooltip
    pub fn should_show(&self, current_time: f64, config: &TooltipConfig) -> bool {
        if !config.enabled {
            return false;
        }

        if let (Some(tooltip), Some(start)) = (self.active(), self.hover_start) {
            let delay = tooltip.show_delay_ms.max(config.show_delay_ms);
            let elapsed_ms = ((current_time - start) * 1000.0) as u64;
            elapsed_ms >= delay
        } else {
            false
        }
    }

    /// Clear all tooltips
    pub fn clear(&mut self) {
        self.tooltips.clear();
        self.active_tooltip = None;
        self.hover_start = None;
    }
}

/// Contextual help content for detailed help
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextualHelp {
    /// Unique identifier
    pub id: String,
    /// Title of the help topic
    pub title: String,
    /// Detailed description (supports markdown)
    pub description: String,
    /// Keyboard shortcut hint (e.g., "Ctrl+S")
    pub shortcut: Option<String>,
    /// Related actions or topics
    pub related: Vec<String>,
    /// Category for grouping
    pub category: Option<String>,
    /// Icon identifier
    pub icon: Option<String>,
}

impl ContextualHelp {
    /// Create new contextual help
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            ..Default::default()
        }
    }

    /// Builder pattern: set shortcut
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Builder pattern: add related topic
    pub fn with_related(mut self, related: impl Into<String>) -> Self {
        self.related.push(related.into());
        self
    }

    /// Builder pattern: set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Builder pattern: set icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Format shortcut for display
    pub fn formatted_shortcut(&self) -> Option<String> {
        self.shortcut.as_ref().map(|s| format!("[{}]", s))
    }
}

/// Registry for contextual help entries
#[derive(Resource, Default)]
pub struct HelpRegistry {
    /// Map of help ID to help content
    entries: HashMap<String, ContextualHelp>,
    /// Entries organized by category
    by_category: HashMap<String, Vec<String>>,
}

impl HelpRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a help entry
    pub fn register(&mut self, help: ContextualHelp) {
        let id = help.id.clone();
        if let Some(ref category) = help.category {
            self.by_category
                .entry(category.clone())
                .or_default()
                .push(id.clone());
        }
        self.entries.insert(id, help);
    }

    /// Get help by ID
    pub fn get(&self, id: &str) -> Option<&ContextualHelp> {
        self.entries.get(id)
    }

    /// Get all entries in a category
    pub fn by_category(&self, category: &str) -> Vec<&ContextualHelp> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<&String> {
        self.by_category.keys().collect()
    }

    /// Get all entries
    pub fn all(&self) -> Vec<&ContextualHelp> {
        self.entries.values().collect()
    }

    /// Search entries by text
    pub fn search(&self, query: &str) -> Vec<&ContextualHelp> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|h| {
                h.title.to_lowercase().contains(&query_lower)
                    || h.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

/// Component to mark entities that show help on F1
#[derive(Component, Debug, Clone, Default)]
pub struct HelpOverlay {
    /// Whether the help overlay is currently visible
    pub visible: bool,
    /// Search query for filtering help
    pub search_query: String,
    /// Currently selected category filter
    pub selected_category: Option<String>,
}

impl HelpOverlay {
    /// Create a new help overlay
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Show the overlay
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the overlay
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Clear search and filters
    pub fn clear_filters(&mut self) {
        self.search_query.clear();
        self.selected_category = None;
    }
}

/// Resource for the global help overlay state
#[derive(Resource, Default)]
pub struct HelpOverlayState {
    /// Whether the help overlay is visible
    pub visible: bool,
    /// Current search query
    pub search_query: String,
    /// Selected category
    pub selected_category: Option<String>,
    /// Currently viewed help entry
    pub current_entry: Option<String>,
}

impl HelpOverlayState {
    /// Toggle overlay visibility
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.search_query.clear();
            self.current_entry = None;
        }
    }
}

/// Events for tooltip system
#[derive(Event, Clone, Debug)]
pub enum TooltipEvent {
    /// Show a tooltip
    Show { id: String },
    /// Hide the current tooltip
    Hide,
    /// Register a new tooltip
    Register(Tooltip),
    /// Unregister a tooltip
    Unregister { id: String },
    /// Toggle tooltips globally
    ToggleEnabled,
    /// Show contextual help
    ShowHelp { id: String },
    /// Toggle help overlay
    ToggleHelpOverlay,
}

/// System to handle tooltip events
pub fn handle_tooltip_events(
    mut events: EventReader<TooltipEvent>,
    mut registry: ResMut<TooltipRegistry>,
    mut config: ResMut<TooltipConfig>,
    mut help_state: ResMut<HelpOverlayState>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    for event in events.read() {
        match event {
            TooltipEvent::Show { id } => {
                if registry.has(id) {
                    registry.set_active(Some(id.clone()), current_time);
                }
            }
            TooltipEvent::Hide => {
                registry.set_active(None, current_time);
            }
            TooltipEvent::Register(tooltip) => {
                registry.register(tooltip.clone());
            }
            TooltipEvent::Unregister { id } => {
                registry.unregister(id);
            }
            TooltipEvent::ToggleEnabled => {
                config.enabled = !config.enabled;
                info!(
                    "Tooltips {}",
                    if config.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
            }
            TooltipEvent::ShowHelp { id } => {
                help_state.current_entry = Some(id.clone());
                help_state.visible = true;
            }
            TooltipEvent::ToggleHelpOverlay => {
                help_state.toggle();
            }
        }
    }
}

/// System to handle F1 key for help overlay
pub fn help_overlay_hotkey_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut help_state: ResMut<HelpOverlayState>,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        help_state.toggle();
    }

    // Escape to close help
    if keyboard.just_pressed(KeyCode::Escape) && help_state.visible {
        help_state.visible = false;
    }
}

#[cfg(feature = "visual")]
/// System to render active tooltip
pub fn tooltip_render_system(
    mut contexts: EguiContexts,
    registry: Res<TooltipRegistry>,
    config: Res<TooltipConfig>,
    time: Res<Time>,
) {
    if !config.enabled {
        return;
    }

    let current_time = time.elapsed_secs_f64();

    if !registry.should_show(current_time, &config) {
        return;
    }

    if let Some(tooltip) = registry.active() {
        if !tooltip.enabled {
            return;
        }

        let ctx = contexts.ctx_mut();
        let cursor_pos = ctx
            .input(|i| i.pointer.hover_pos())
            .unwrap_or(egui::pos2(0.0, 0.0));

        let pos = match tooltip.position {
            TooltipPosition::FollowCursor => egui::pos2(
                cursor_pos.x + config.cursor_offset[0],
                cursor_pos.y + config.cursor_offset[1],
            ),
            TooltipPosition::Fixed { x, y } => egui::pos2(x as f32, y as f32),
            TooltipPosition::RelativeToElement { offset_x, offset_y } => egui::pos2(
                cursor_pos.x + offset_x as f32,
                cursor_pos.y + offset_y as f32,
            ),
            TooltipPosition::Above => egui::pos2(cursor_pos.x, cursor_pos.y - 30.0),
            TooltipPosition::Below => egui::pos2(cursor_pos.x, cursor_pos.y + 20.0),
            TooltipPosition::Left => {
                egui::pos2(cursor_pos.x - tooltip.max_width - 10.0, cursor_pos.y)
            }
            TooltipPosition::Right => egui::pos2(cursor_pos.x + 20.0, cursor_pos.y),
        };

        egui::Area::new(egui::Id::new("tooltip_area"))
            .fixed_pos(pos)
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(tooltip.style.background_color())
                    .stroke(egui::Stroke::new(1.0, tooltip.style.border_color()))
                    .rounding(4.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.set_max_width(tooltip.max_width);

                        // Title if present
                        if let Some(ref title) = tooltip.title {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(tooltip.style.icon())
                                        .color(tooltip.style.text_color())
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(title)
                                        .color(tooltip.style.text_color())
                                        .strong(),
                                );
                            });
                            ui.separator();
                        }

                        // Formatted text content
                        for segment in tooltip.formatted_text() {
                            match segment {
                                TextSegment::Normal(text) => {
                                    ui.label(
                                        egui::RichText::new(text).color(tooltip.style.text_color()),
                                    );
                                }
                                TextSegment::Bold(text) => {
                                    ui.label(
                                        egui::RichText::new(text)
                                            .color(tooltip.style.text_color())
                                            .strong(),
                                    );
                                }
                                TextSegment::Italic(text) => {
                                    ui.label(
                                        egui::RichText::new(text)
                                            .color(tooltip.style.text_color())
                                            .italics(),
                                    );
                                }
                                TextSegment::Code(text) => {
                                    ui.label(
                                        egui::RichText::new(format!("`{}`", text))
                                            .color(tooltip.style.text_color())
                                            .monospace(),
                                    );
                                }
                            }
                        }
                    });
            });
    }
}

#[cfg(not(feature = "visual"))]
pub fn tooltip_render_system() {}

#[cfg(feature = "visual")]
/// System to render help overlay
pub fn help_overlay_render_system(
    mut contexts: EguiContexts,
    mut help_state: ResMut<HelpOverlayState>,
    help_registry: Res<HelpRegistry>,
) {
    if !help_state.visible {
        return;
    }

    egui::Window::new("Help")
        .collapsible(false)
        .resizable(true)
        .default_width(600.0)
        .default_height(500.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.heading("Sim3D Help");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("X").clicked() {
                        help_state.visible = false;
                    }
                });
            });

            ui.separator();

            // Search bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut help_state.search_query);
            });

            ui.separator();

            // Category filter
            ui.horizontal(|ui| {
                ui.label("Category:");
                if ui
                    .selectable_label(help_state.selected_category.is_none(), "All")
                    .clicked()
                {
                    help_state.selected_category = None;
                }
                for category in help_registry.categories() {
                    let selected = help_state.selected_category.as_ref() == Some(category);
                    if ui.selectable_label(selected, category).clicked() {
                        help_state.selected_category = Some(category.clone());
                    }
                }
            });

            ui.separator();

            // Content area
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let entries = if !help_state.search_query.is_empty() {
                        help_registry.search(&help_state.search_query)
                    } else if let Some(ref category) = help_state.selected_category {
                        help_registry.by_category(category)
                    } else {
                        help_registry.all()
                    };

                    let is_empty = entries.is_empty();

                    for entry in entries {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(&entry.title);
                                if let Some(ref shortcut) = entry.shortcut {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!("[{}]", shortcut))
                                                    .monospace()
                                                    .small(),
                                            );
                                        },
                                    );
                                }
                            });

                            ui.label(&entry.description);

                            if !entry.related.is_empty() {
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    ui.label("Related:");
                                    for related in &entry.related {
                                        if ui.small_button(related).clicked() {
                                            help_state.current_entry = Some(related.clone());
                                        }
                                    }
                                });
                            }
                        });
                        ui.add_space(5.0);
                    }

                    if is_empty {
                        ui.label("No help entries found.");
                    }
                });

            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Press F1 to toggle help");
                ui.label("|");
                ui.label("Press Escape to close");
            });
        });
}

#[cfg(not(feature = "visual"))]
pub fn help_overlay_render_system() {}

/// Initialize default help entries
pub fn setup_default_help(mut help_registry: ResMut<HelpRegistry>) {
    // Camera controls
    help_registry.register(
        ContextualHelp::new(
            "camera_orbit",
            "Orbit Camera",
            "Right-click and drag to orbit the camera around the focus point.",
        )
        .with_shortcut("Right Mouse + Drag")
        .with_category("Camera"),
    );
    help_registry.register(
        ContextualHelp::new(
            "camera_pan",
            "Pan Camera",
            "Middle-click and drag to pan the camera view.",
        )
        .with_shortcut("Middle Mouse + Drag")
        .with_category("Camera"),
    );
    help_registry.register(
        ContextualHelp::new(
            "camera_zoom",
            "Zoom Camera",
            "Use the mouse wheel to zoom in and out.",
        )
        .with_shortcut("Mouse Wheel")
        .with_category("Camera"),
    );

    // Simulation controls
    help_registry.register(
        ContextualHelp::new(
            "sim_pause",
            "Pause/Resume",
            "Toggle simulation pause state.",
        )
        .with_shortcut("Space")
        .with_category("Simulation"),
    );
    help_registry.register(
        ContextualHelp::new(
            "sim_reset",
            "Reset Simulation",
            "Reset the simulation to its initial state.",
        )
        .with_shortcut("R")
        .with_category("Simulation"),
    );
    help_registry.register(
        ContextualHelp::new(
            "sim_speed",
            "Time Scale",
            "Use number keys 1-5 to adjust simulation speed (0.25x to 5x).",
        )
        .with_shortcut("1-5")
        .with_category("Simulation"),
    );

    // Layout controls
    help_registry.register(
        ContextualHelp::new(
            "layout_coding",
            "Coding Layout",
            "Full development layout with all panels visible.",
        )
        .with_shortcut("F2")
        .with_category("Layouts"),
    );
    help_registry.register(
        ContextualHelp::new(
            "layout_debug",
            "Debugging Layout",
            "Layout optimized for debugging with stats and console.",
        )
        .with_shortcut("F3")
        .with_category("Layouts"),
    );
    help_registry.register(
        ContextualHelp::new(
            "layout_present",
            "Presentation Layout",
            "Clean layout for demonstrations.",
        )
        .with_shortcut("F4")
        .with_category("Layouts"),
    );

    // Visualization toggles
    help_registry.register(
        ContextualHelp::new(
            "vis_debug",
            "Debug Info",
            "Toggle debug information overlay.",
        )
        .with_shortcut("D")
        .with_category("Visualization"),
    );
    help_registry.register(
        ContextualHelp::new(
            "vis_physics",
            "Physics Debug",
            "Toggle physics debug visualization.",
        )
        .with_shortcut("P")
        .with_category("Visualization"),
    );
    help_registry.register(
        ContextualHelp::new(
            "vis_sensors",
            "Sensor Rays",
            "Toggle sensor ray visualization.",
        )
        .with_shortcut("S")
        .with_category("Visualization"),
    );
    help_registry.register(
        ContextualHelp::new(
            "vis_tf",
            "TF Frames",
            "Toggle transform frame visualization.",
        )
        .with_shortcut("T")
        .with_category("Visualization"),
    );
}

/// Plugin to register tooltip and help systems
pub struct TooltipsPlugin;

impl Plugin for TooltipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipConfig>()
            .init_resource::<TooltipRegistry>()
            .init_resource::<HelpRegistry>()
            .init_resource::<HelpOverlayState>()
            .add_event::<TooltipEvent>()
            .add_systems(Startup, setup_default_help)
            .add_systems(
                Update,
                (handle_tooltip_events, help_overlay_hotkey_system).chain(),
            );

        #[cfg(feature = "visual")]
        app.add_systems(Update, (tooltip_render_system, help_overlay_render_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tooltip_style_icons() {
        assert_eq!(TooltipStyle::Info.icon(), "i");
        assert_eq!(TooltipStyle::Warning.icon(), "!");
        assert_eq!(TooltipStyle::Error.icon(), "X");
        assert_eq!(TooltipStyle::Success.icon(), "*");
        assert_eq!(TooltipStyle::Hint.icon(), "?");
    }

    #[test]
    fn test_tooltip_creation() {
        let tooltip = Tooltip::new("test_id", "Test tooltip text");
        assert_eq!(tooltip.id, "test_id");
        assert_eq!(tooltip.text, "Test tooltip text");
        assert!(tooltip.enabled);
        assert_eq!(tooltip.style, TooltipStyle::Info);
    }

    #[test]
    fn test_tooltip_builder() {
        let tooltip = Tooltip::new("test", "Text")
            .with_title("Title")
            .with_style(TooltipStyle::Warning)
            .with_show_delay(1000)
            .with_duration(5000)
            .with_max_width(400.0)
            .with_enabled(false);

        assert_eq!(tooltip.title, Some("Title".to_string()));
        assert_eq!(tooltip.style, TooltipStyle::Warning);
        assert_eq!(tooltip.show_delay_ms, 1000);
        assert_eq!(tooltip.duration_ms, 5000);
        assert_eq!(tooltip.max_width, 400.0);
        assert!(!tooltip.enabled);
    }

    #[test]
    fn test_tooltip_position_variants() {
        let tooltip = Tooltip::new("test", "Text").with_position(TooltipPosition::Above);
        assert_eq!(tooltip.position, TooltipPosition::Above);

        let tooltip =
            Tooltip::new("test", "Text").with_position(TooltipPosition::Fixed { x: 100, y: 200 });
        match tooltip.position {
            TooltipPosition::Fixed { x, y } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Expected Fixed position"),
        }
    }

    #[test]
    fn test_tooltip_duration_conversion() {
        let tooltip = Tooltip::new("test", "Text")
            .with_show_delay(500)
            .with_duration(2000);

        assert_eq!(tooltip.show_delay(), Duration::from_millis(500));
        assert_eq!(tooltip.duration(), Duration::from_millis(2000));
    }

    #[test]
    fn test_tooltip_config_defaults() {
        let config = TooltipConfig::default();
        assert!(config.enabled);
        assert_eq!(config.show_delay_ms, 500);
        assert_eq!(config.hide_delay_ms, 200);
        assert_eq!(config.max_width, 300.0);
    }

    #[test]
    fn test_tooltip_config_builder() {
        let config = TooltipConfig::new()
            .with_enabled(false)
            .with_show_delay(1000)
            .with_hide_delay(500)
            .with_max_width(400.0)
            .with_opacity(0.8);

        assert!(!config.enabled);
        assert_eq!(config.show_delay_ms, 1000);
        assert_eq!(config.hide_delay_ms, 500);
        assert_eq!(config.max_width, 400.0);
        assert_eq!(config.opacity, 0.8);
    }

    #[test]
    fn test_tooltip_config_opacity_clamping() {
        let config = TooltipConfig::new().with_opacity(1.5);
        assert_eq!(config.opacity, 1.0);

        let config = TooltipConfig::new().with_opacity(-0.5);
        assert_eq!(config.opacity, 0.0);
    }

    #[test]
    fn test_tooltip_registry_operations() {
        let mut registry = TooltipRegistry::new();

        // Register
        registry.register(Tooltip::new("tooltip1", "First tooltip"));
        registry.register_simple("tooltip2", "Second tooltip");

        assert!(registry.has("tooltip1"));
        assert!(registry.has("tooltip2"));
        assert!(!registry.has("tooltip3"));

        // Get
        let tooltip = registry.get("tooltip1").unwrap();
        assert_eq!(tooltip.text, "First tooltip");

        // Unregister
        let removed = registry.unregister("tooltip1");
        assert!(removed.is_some());
        assert!(!registry.has("tooltip1"));
    }

    #[test]
    fn test_tooltip_registry_active() {
        let mut registry = TooltipRegistry::new();
        registry.register(Tooltip::new("test_tooltip", "Test"));

        assert!(registry.active().is_none());

        registry.set_active(Some("test_tooltip".to_string()), 0.0);
        assert!(registry.active().is_some());
        assert_eq!(registry.active().unwrap().id, "test_tooltip");

        registry.set_active(None, 1.0);
        assert!(registry.active().is_none());
    }

    #[test]
    fn test_tooltip_registry_should_show() {
        let mut registry = TooltipRegistry::new();
        let config = TooltipConfig::default();

        registry.register(Tooltip::new("test", "Test").with_show_delay(100));
        registry.set_active(Some("test".to_string()), 0.0);

        // Not enough time has passed
        assert!(!registry.should_show(0.05, &config));

        // Enough time has passed
        assert!(registry.should_show(1.0, &config));
    }

    #[test]
    fn test_contextual_help_creation() {
        let help = ContextualHelp::new("help_id", "Help Title", "This is the description.");
        assert_eq!(help.id, "help_id");
        assert_eq!(help.title, "Help Title");
        assert_eq!(help.description, "This is the description.");
    }

    #[test]
    fn test_contextual_help_builder() {
        let help = ContextualHelp::new("test", "Test", "Description")
            .with_shortcut("Ctrl+T")
            .with_category("Testing")
            .with_related("other_topic")
            .with_icon("test_icon");

        assert_eq!(help.shortcut, Some("Ctrl+T".to_string()));
        assert_eq!(help.category, Some("Testing".to_string()));
        assert!(help.related.contains(&"other_topic".to_string()));
        assert_eq!(help.icon, Some("test_icon".to_string()));
    }

    #[test]
    fn test_contextual_help_formatted_shortcut() {
        let help = ContextualHelp::new("test", "Test", "Desc").with_shortcut("Ctrl+S");
        assert_eq!(help.formatted_shortcut(), Some("[Ctrl+S]".to_string()));

        let help_no_shortcut = ContextualHelp::new("test", "Test", "Desc");
        assert_eq!(help_no_shortcut.formatted_shortcut(), None);
    }

    #[test]
    fn test_help_registry_operations() {
        let mut registry = HelpRegistry::new();

        registry.register(ContextualHelp::new("help1", "Title 1", "Desc 1").with_category("Cat A"));
        registry.register(ContextualHelp::new("help2", "Title 2", "Desc 2").with_category("Cat A"));
        registry.register(ContextualHelp::new("help3", "Title 3", "Desc 3").with_category("Cat B"));

        // Get by ID
        let help = registry.get("help1").unwrap();
        assert_eq!(help.title, "Title 1");

        // Get by category
        let cat_a = registry.by_category("Cat A");
        assert_eq!(cat_a.len(), 2);

        let cat_b = registry.by_category("Cat B");
        assert_eq!(cat_b.len(), 1);

        // Get categories
        let categories = registry.categories();
        assert_eq!(categories.len(), 2);

        // Get all
        let all = registry.all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_help_registry_search() {
        let mut registry = HelpRegistry::new();

        registry.register(ContextualHelp::new(
            "h1",
            "Camera Controls",
            "How to control the camera",
        ));
        registry.register(ContextualHelp::new(
            "h2",
            "Simulation",
            "Running the simulation",
        ));
        registry.register(ContextualHelp::new(
            "h3",
            "Camera Reset",
            "Reset the camera view",
        ));

        let results = registry.search("camera");
        assert_eq!(results.len(), 2);

        let results = registry.search("simulation");
        assert_eq!(results.len(), 1);

        let results = registry.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_help_overlay_state() {
        let mut state = HelpOverlayState::default();
        assert!(!state.visible);

        state.toggle();
        assert!(state.visible);

        state.toggle();
        assert!(!state.visible);
        assert!(state.search_query.is_empty()); // Should be cleared on close
    }

    #[test]
    fn test_help_overlay_component() {
        let mut overlay = HelpOverlay::new();
        assert!(!overlay.visible);

        overlay.show();
        assert!(overlay.visible);

        overlay.hide();
        assert!(!overlay.visible);

        overlay.toggle();
        assert!(overlay.visible);

        overlay.search_query = "test".to_string();
        overlay.selected_category = Some("cat".to_string());
        overlay.clear_filters();
        assert!(overlay.search_query.is_empty());
        assert!(overlay.selected_category.is_none());
    }

    #[test]
    fn test_parse_markdown_text() {
        // Normal text
        let segments = parse_markdown_text("normal text");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Normal(s) => assert_eq!(s, "normal text"),
            _ => panic!("Expected normal text"),
        }

        // Bold text
        let segments = parse_markdown_text("**bold**");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Bold(s) => assert_eq!(s, "bold"),
            _ => panic!("Expected bold text"),
        }

        // Italic text
        let segments = parse_markdown_text("*italic*");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Italic(s) => assert_eq!(s, "italic"),
            _ => panic!("Expected italic text"),
        }

        // Code text
        let segments = parse_markdown_text("`code`");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            TextSegment::Code(s) => assert_eq!(s, "code"),
            _ => panic!("Expected code text"),
        }

        // Mixed text
        let segments = parse_markdown_text("normal **bold** more *italic* end");
        assert!(segments.len() >= 4);
    }

    #[test]
    fn test_tooltip_formatted_text() {
        let tooltip = Tooltip::new("test", "Press **Enter** to confirm or `Escape` to cancel.");
        let segments = tooltip.formatted_text();
        assert!(segments.len() > 1);
    }

    #[test]
    fn test_tooltip_registry_cursor_update() {
        let mut registry = TooltipRegistry::new();
        registry.update_cursor(100.0, 200.0);
        assert_eq!(registry.cursor_position, [100.0, 200.0]);
    }

    #[test]
    fn test_tooltip_registry_clear() {
        let mut registry = TooltipRegistry::new();
        registry.register(Tooltip::new("t1", "Test 1"));
        registry.register(Tooltip::new("t2", "Test 2"));
        registry.set_active(Some("t1".to_string()), 0.0);

        registry.clear();

        assert!(!registry.has("t1"));
        assert!(!registry.has("t2"));
        assert!(registry.active().is_none());
    }

    #[test]
    fn test_tooltip_registry_ids() {
        let mut registry = TooltipRegistry::new();
        registry.register(Tooltip::new("id_a", "A"));
        registry.register(Tooltip::new("id_b", "B"));
        registry.register(Tooltip::new("id_c", "C"));

        let ids = registry.ids();
        assert_eq!(ids.len(), 3);
        assert!(ids.iter().any(|id| *id == "id_a"));
        assert!(ids.iter().any(|id| *id == "id_b"));
        assert!(ids.iter().any(|id| *id == "id_c"));
    }

    #[test]
    fn test_tooltip_registry_get_mut() {
        let mut registry = TooltipRegistry::new();
        registry.register(Tooltip::new("mutable", "Original text"));

        {
            let tooltip = registry.get_mut("mutable").unwrap();
            tooltip.text = "Modified text".to_string();
        }

        let tooltip = registry.get("mutable").unwrap();
        assert_eq!(tooltip.text, "Modified text");
    }

    #[test]
    fn test_tooltip_config_serialization() {
        let config = TooltipConfig::new().with_show_delay(750).with_opacity(0.9);

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TooltipConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.show_delay_ms, 750);
        assert_eq!(deserialized.opacity, 0.9);
    }

    #[test]
    fn test_tooltip_serialization() {
        let tooltip = Tooltip::new("ser_test", "Serialization test")
            .with_title("Title")
            .with_style(TooltipStyle::Warning);

        let json = serde_json::to_string(&tooltip).unwrap();
        let deserialized: Tooltip = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "ser_test");
        assert_eq!(deserialized.text, "Serialization test");
        assert_eq!(deserialized.title, Some("Title".to_string()));
        assert_eq!(deserialized.style, TooltipStyle::Warning);
    }

    #[test]
    fn test_contextual_help_serialization() {
        let help = ContextualHelp::new("ch_test", "Title", "Description")
            .with_shortcut("Ctrl+H")
            .with_category("Test");

        let json = serde_json::to_string(&help).unwrap();
        let deserialized: ContextualHelp = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "ch_test");
        assert_eq!(deserialized.shortcut, Some("Ctrl+H".to_string()));
    }
}
