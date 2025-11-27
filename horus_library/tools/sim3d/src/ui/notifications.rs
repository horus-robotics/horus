//! Toast Notification System for sim3d
//!
//! Provides a flexible notification system with support for various notification types,
//! progress indicators, actions, and customizable positioning.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bevy::prelude::*;
#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

/// Global counter for unique notification IDs
static NOTIFICATION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generates a unique notification ID
fn next_notification_id() -> u64 {
    NOTIFICATION_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Type of notification that determines its visual appearance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NotificationType {
    /// Informational notification (blue)
    #[default]
    Info,
    /// Success notification (green)
    Success,
    /// Warning notification (yellow/orange)
    Warning,
    /// Error notification (red)
    Error,
    /// Progress notification with progress bar
    Progress,
}

impl NotificationType {
    /// Returns the icon character for this notification type
    pub fn icon(&self) -> &'static str {
        match self {
            NotificationType::Info => "i",
            NotificationType::Success => "+",
            NotificationType::Warning => "!",
            NotificationType::Error => "x",
            NotificationType::Progress => "*",
        }
    }

    /// Returns the color for this notification type
    #[cfg(feature = "visual")]
    pub fn color(&self) -> egui::Color32 {
        match self {
            NotificationType::Info => egui::Color32::from_rgb(66, 135, 245),
            NotificationType::Success => egui::Color32::from_rgb(76, 175, 80),
            NotificationType::Warning => egui::Color32::from_rgb(255, 152, 0),
            NotificationType::Error => egui::Color32::from_rgb(244, 67, 54),
            NotificationType::Progress => egui::Color32::from_rgb(156, 39, 176),
        }
    }
}

/// An action that can be attached to a notification
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Unique identifier for the action
    pub id: String,
    /// Display label for the action button
    pub label: String,
    /// Whether clicking this action dismisses the notification
    pub dismiss_on_click: bool,
}

impl NotificationAction {
    /// Creates a new notification action
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            dismiss_on_click: true,
        }
    }

    /// Sets whether clicking dismisses the notification
    pub fn with_dismiss(mut self, dismiss: bool) -> Self {
        self.dismiss_on_click = dismiss;
        self
    }
}

/// Duration configuration for notifications
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationDuration {
    /// Notification stays until manually dismissed
    Persistent,
    /// Notification auto-dismisses after the specified duration
    Timed(Duration),
}

impl Default for NotificationDuration {
    fn default() -> Self {
        NotificationDuration::Timed(Duration::from_secs(5))
    }
}

/// A notification to be displayed to the user
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique identifier
    pub id: u64,
    /// Type of notification
    pub notification_type: NotificationType,
    /// Title text (bold, first line)
    pub title: String,
    /// Message text (detailed description)
    pub message: String,
    /// How long the notification should be displayed
    pub duration: NotificationDuration,
    /// Progress value (0.0 - 1.0) for progress notifications
    pub progress: Option<f32>,
    /// Clickable action buttons
    pub actions: Vec<NotificationAction>,
    /// When the notification was created
    pub timestamp: f64,
    /// Time remaining until auto-dismiss (for timed notifications)
    pub time_remaining: Option<Duration>,
    /// Whether the notification is being hovered
    pub hovered: bool,
}

impl Notification {
    /// Creates a new notification with the specified type and title
    pub fn new(notification_type: NotificationType, title: impl Into<String>) -> Self {
        Self {
            id: next_notification_id(),
            notification_type,
            title: title.into(),
            message: String::new(),
            duration: NotificationDuration::default(),
            progress: None,
            actions: Vec::new(),
            timestamp: 0.0,
            time_remaining: None,
            hovered: false,
        }
    }

    /// Sets the message text
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Sets the duration
    pub fn with_duration(mut self, duration: NotificationDuration) -> Self {
        self.duration = duration;
        if let NotificationDuration::Timed(d) = duration {
            self.time_remaining = Some(d);
        }
        self
    }

    /// Sets the progress value (0.0 - 1.0)
    pub fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Adds an action button
    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    /// Sets the timestamp
    pub fn with_timestamp(mut self, timestamp: f64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Makes the notification persistent (won't auto-dismiss)
    pub fn persistent(mut self) -> Self {
        self.duration = NotificationDuration::Persistent;
        self.time_remaining = None;
        self
    }

    /// Creates an info notification
    pub fn info(title: impl Into<String>) -> Self {
        Self::new(NotificationType::Info, title)
    }

    /// Creates a success notification
    pub fn success(title: impl Into<String>) -> Self {
        Self::new(NotificationType::Success, title)
    }

    /// Creates a warning notification
    pub fn warning(title: impl Into<String>) -> Self {
        Self::new(NotificationType::Warning, title)
    }

    /// Creates an error notification
    pub fn error(title: impl Into<String>) -> Self {
        Self::new(NotificationType::Error, title)
    }

    /// Creates a progress notification
    pub fn progress(title: impl Into<String>, progress: f32) -> Self {
        Self::new(NotificationType::Progress, title)
            .with_progress(progress)
            .persistent()
    }
}

/// Position where notifications are displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NotificationPosition {
    /// Top-right corner (default)
    #[default]
    TopRight,
    /// Top-left corner
    TopLeft,
    /// Bottom-right corner
    BottomRight,
    /// Bottom-left corner
    BottomLeft,
    /// Top-center
    TopCenter,
    /// Bottom-center
    BottomCenter,
}

/// Direction in which notifications stack
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StackDirection {
    /// Notifications stack downward from the anchor position
    #[default]
    Down,
    /// Notifications stack upward from the anchor position
    Up,
}

/// Configuration for the notification system
#[derive(Resource, Debug, Clone)]
pub struct NotificationConfig {
    /// Position where notifications appear
    pub position: NotificationPosition,
    /// Maximum number of visible notifications
    pub max_visible: usize,
    /// Default duration for notifications
    pub default_duration: Duration,
    /// Whether to play sounds for notifications
    pub enable_sounds: bool,
    /// Direction in which notifications stack
    pub stack_direction: StackDirection,
    /// Horizontal margin from screen edge
    pub margin_x: f32,
    /// Vertical margin from screen edge
    pub margin_y: f32,
    /// Spacing between notifications
    pub spacing: f32,
    /// Width of notification windows
    pub notification_width: f32,
    /// Whether to pause timer when hovering
    pub pause_on_hover: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            position: NotificationPosition::TopRight,
            max_visible: 5,
            default_duration: Duration::from_secs(5),
            enable_sounds: false,
            stack_direction: StackDirection::Down,
            margin_x: 20.0,
            margin_y: 20.0,
            spacing: 10.0,
            notification_width: 350.0,
            pause_on_hover: true,
        }
    }
}

impl NotificationConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the position
    pub fn with_position(mut self, position: NotificationPosition) -> Self {
        self.position = position;
        self
    }

    /// Sets the maximum visible notifications
    pub fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max.max(1);
        self
    }

    /// Sets the default duration
    pub fn with_default_duration(mut self, duration: Duration) -> Self {
        self.default_duration = duration;
        self
    }

    /// Sets whether sounds are enabled
    pub fn with_sounds(mut self, enabled: bool) -> Self {
        self.enable_sounds = enabled;
        self
    }

    /// Sets the stack direction
    pub fn with_stack_direction(mut self, direction: StackDirection) -> Self {
        self.stack_direction = direction;
        self
    }
}

/// Event fired when a notification action is clicked
#[derive(Event, Debug, Clone)]
pub struct NotificationActionEvent {
    /// ID of the notification
    pub notification_id: u64,
    /// ID of the action that was clicked
    pub action_id: String,
}

/// Event for creating notifications
#[derive(Event, Debug, Clone)]
pub struct NotifyEvent {
    /// The notification to display
    pub notification: Notification,
}

impl NotifyEvent {
    /// Creates a new notify event
    pub fn new(notification: Notification) -> Self {
        Self { notification }
    }
}

/// A record of a past notification
#[derive(Debug, Clone)]
pub struct NotificationRecord {
    /// The original notification
    pub notification: Notification,
    /// When the notification was dismissed
    pub dismissed_at: f64,
    /// Whether it was dismissed by user action
    pub dismissed_by_user: bool,
}

/// History of past notifications
#[derive(Resource, Debug, Default)]
pub struct NotificationHistory {
    /// Past notifications (most recent first)
    records: VecDeque<NotificationRecord>,
    /// Maximum records to keep
    max_records: usize,
}

impl NotificationHistory {
    /// Creates a new history with default capacity
    pub fn new() -> Self {
        Self {
            records: VecDeque::new(),
            max_records: 100,
        }
    }

    /// Creates a new history with specified capacity
    pub fn with_capacity(max_records: usize) -> Self {
        Self {
            records: VecDeque::with_capacity(max_records),
            max_records,
        }
    }

    /// Adds a notification to history
    pub fn add(&mut self, notification: Notification, dismissed_at: f64, by_user: bool) {
        let record = NotificationRecord {
            notification,
            dismissed_at,
            dismissed_by_user: by_user,
        };

        self.records.push_front(record);

        // Trim if over capacity
        while self.records.len() > self.max_records {
            self.records.pop_back();
        }
    }

    /// Gets all records
    pub fn records(&self) -> &VecDeque<NotificationRecord> {
        &self.records
    }

    /// Gets records filtered by type
    pub fn records_by_type(&self, notification_type: NotificationType) -> Vec<&NotificationRecord> {
        self.records
            .iter()
            .filter(|r| r.notification.notification_type == notification_type)
            .collect()
    }

    /// Clears all history
    pub fn clear(&mut self) {
        self.records.clear();
    }

    /// Gets the number of records
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Checks if history is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

/// Manager for active notifications
#[derive(Resource, Debug)]
pub struct NotificationManager {
    /// Active notifications queue
    notifications: VecDeque<Notification>,
    /// IDs of notifications to dismiss
    pending_dismissals: Vec<u64>,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    /// Creates a new notification manager
    pub fn new() -> Self {
        Self {
            notifications: VecDeque::new(),
            pending_dismissals: Vec::new(),
        }
    }

    /// Adds a notification to the queue
    pub fn notify(&mut self, mut notification: Notification, current_time: f64) {
        notification.timestamp = current_time;

        // Initialize time remaining for timed notifications
        if let NotificationDuration::Timed(duration) = notification.duration {
            notification.time_remaining = Some(duration);
        }

        self.notifications.push_back(notification);
    }

    /// Creates and adds an info notification
    pub fn notify_info(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        current_time: f64,
    ) -> u64 {
        let notification = Notification::info(title).with_message(message);
        let id = notification.id;
        self.notify(notification, current_time);
        id
    }

    /// Creates and adds a success notification
    pub fn notify_success(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        current_time: f64,
    ) -> u64 {
        let notification = Notification::success(title).with_message(message);
        let id = notification.id;
        self.notify(notification, current_time);
        id
    }

    /// Creates and adds a warning notification
    pub fn notify_warning(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        current_time: f64,
    ) -> u64 {
        let notification = Notification::warning(title).with_message(message);
        let id = notification.id;
        self.notify(notification, current_time);
        id
    }

    /// Creates and adds an error notification
    pub fn notify_error(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        current_time: f64,
    ) -> u64 {
        let notification = Notification::error(title).with_message(message);
        let id = notification.id;
        self.notify(notification, current_time);
        id
    }

    /// Creates and adds a progress notification
    pub fn notify_progress(
        &mut self,
        title: impl Into<String>,
        progress: f32,
        current_time: f64,
    ) -> u64 {
        let notification = Notification::progress(title, progress);
        let id = notification.id;
        self.notify(notification, current_time);
        id
    }

    /// Updates the progress of an existing progress notification
    pub fn update_progress(&mut self, id: u64, progress: f32) -> bool {
        if let Some(notification) = self.notifications.iter_mut().find(|n| n.id == id) {
            notification.progress = Some(progress.clamp(0.0, 1.0));
            true
        } else {
            false
        }
    }

    /// Updates the message of an existing notification
    pub fn update_message(&mut self, id: u64, message: impl Into<String>) -> bool {
        if let Some(notification) = self.notifications.iter_mut().find(|n| n.id == id) {
            notification.message = message.into();
            true
        } else {
            false
        }
    }

    /// Marks a notification for dismissal
    pub fn dismiss(&mut self, id: u64) {
        if !self.pending_dismissals.contains(&id) {
            self.pending_dismissals.push(id);
        }
    }

    /// Marks all notifications for dismissal
    pub fn dismiss_all(&mut self) {
        for notification in &self.notifications {
            if !self.pending_dismissals.contains(&notification.id) {
                self.pending_dismissals.push(notification.id);
            }
        }
    }

    /// Gets the active notifications
    pub fn notifications(&self) -> &VecDeque<Notification> {
        &self.notifications
    }

    /// Gets a mutable reference to active notifications
    pub fn notifications_mut(&mut self) -> &mut VecDeque<Notification> {
        &mut self.notifications
    }

    /// Gets the number of active notifications
    pub fn count(&self) -> usize {
        self.notifications.len()
    }

    /// Checks if there are any active notifications
    pub fn is_empty(&self) -> bool {
        self.notifications.is_empty()
    }

    /// Gets a notification by ID
    pub fn get(&self, id: u64) -> Option<&Notification> {
        self.notifications.iter().find(|n| n.id == id)
    }

    /// Gets a mutable reference to a notification by ID
    pub fn get_mut(&mut self, id: u64) -> Option<&mut Notification> {
        self.notifications.iter_mut().find(|n| n.id == id)
    }

    /// Processes pending dismissals and returns dismissed notifications
    fn process_dismissals(&mut self) -> Vec<Notification> {
        let mut dismissed = Vec::new();

        for id in self.pending_dismissals.drain(..) {
            if let Some(pos) = self.notifications.iter().position(|n| n.id == id) {
                dismissed.push(self.notifications.remove(pos).unwrap());
            }
        }

        dismissed
    }

    /// Updates notification timers and returns expired notifications
    fn update_timers(&mut self, delta: Duration) -> Vec<Notification> {
        let mut expired = Vec::new();

        for notification in &mut self.notifications {
            // Skip if hovered and pause_on_hover is enabled
            if notification.hovered {
                continue;
            }

            if let Some(remaining) = &mut notification.time_remaining {
                if *remaining <= delta {
                    expired.push(notification.clone());
                } else {
                    *remaining -= delta;
                }
            }
        }

        // Remove expired notifications
        for exp in &expired {
            if let Some(pos) = self.notifications.iter().position(|n| n.id == exp.id) {
                self.notifications.remove(pos);
            }
        }

        expired
    }
}

/// System to process notification events
pub fn process_notify_events(
    mut events: EventReader<NotifyEvent>,
    mut manager: ResMut<NotificationManager>,
    time: Res<Time>,
) {
    for event in events.read() {
        manager.notify(event.notification.clone(), time.elapsed_secs_f64());
    }
}

/// System to update notification timers and manage dismissals
pub fn update_notifications_system(
    mut manager: ResMut<NotificationManager>,
    mut history: ResMut<NotificationHistory>,
    time: Res<Time>,
    config: Res<NotificationConfig>,
) {
    let current_time = time.elapsed_secs_f64();
    let delta = Duration::from_secs_f32(time.delta_secs());

    // Process manual dismissals
    let dismissed = manager.process_dismissals();
    for notification in dismissed {
        history.add(notification, current_time, true);
    }

    // Update timers (only if not pausing on hover or not hovered)
    if !config.pause_on_hover {
        // Force update all timers
        for notification in manager.notifications_mut() {
            notification.hovered = false;
        }
    }

    let expired = manager.update_timers(delta);
    for notification in expired {
        history.add(notification, current_time, false);
    }
}

#[cfg(feature = "visual")]
/// System to render notifications
pub fn render_notifications_system(
    mut contexts: EguiContexts,
    mut manager: ResMut<NotificationManager>,
    config: Res<NotificationConfig>,
    mut action_events: EventWriter<NotificationActionEvent>,
) {
    let ctx = contexts.ctx_mut();
    let screen_rect = ctx.screen_rect();

    // Calculate starting position based on config
    let (start_x, start_y, direction_multiplier) = match config.position {
        NotificationPosition::TopRight => (
            screen_rect.max.x - config.notification_width - config.margin_x,
            config.margin_y,
            1.0,
        ),
        NotificationPosition::TopLeft => (config.margin_x, config.margin_y, 1.0),
        NotificationPosition::BottomRight => (
            screen_rect.max.x - config.notification_width - config.margin_x,
            screen_rect.max.y - config.margin_y,
            -1.0,
        ),
        NotificationPosition::BottomLeft => {
            (config.margin_x, screen_rect.max.y - config.margin_y, -1.0)
        }
        NotificationPosition::TopCenter => (
            (screen_rect.max.x - config.notification_width) / 2.0,
            config.margin_y,
            1.0,
        ),
        NotificationPosition::BottomCenter => (
            (screen_rect.max.x - config.notification_width) / 2.0,
            screen_rect.max.y - config.margin_y,
            -1.0,
        ),
    };

    let stack_multiplier = match config.stack_direction {
        StackDirection::Down => 1.0,
        StackDirection::Up => -1.0,
    };

    // Collect actions to fire after rendering
    let mut actions_to_fire = Vec::new();
    let mut to_dismiss = Vec::new();

    // Render visible notifications
    let visible_count = config.max_visible.min(manager.count());
    let mut current_y = start_y;

    for (index, notification) in manager
        .notifications_mut()
        .iter_mut()
        .take(visible_count)
        .enumerate()
    {
        let window_id = egui::Id::new(format!("notification_{}", notification.id));

        // Calculate position
        let y_offset = if direction_multiplier < 0.0 {
            // Bottom positions need to account for window height (estimate)
            -80.0 - (index as f32 * (80.0 + config.spacing) * stack_multiplier)
        } else {
            index as f32 * (80.0 + config.spacing) * stack_multiplier * direction_multiplier
        };

        let window_pos = egui::pos2(start_x, current_y + y_offset);

        let notification_type = notification.notification_type;
        let notification_id = notification.id;
        let notification_progress = notification.progress;
        let notification_actions = notification.actions.clone();

        let response = egui::Window::new("")
            .id(window_id)
            .fixed_pos(window_pos)
            .fixed_size([config.notification_width, 0.0])
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgb(40, 40, 40)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Type indicator
                    let color = notification_type.color();
                    ui.colored_label(color, format!("[{}]", notification_type.icon()));

                    ui.vertical(|ui| {
                        // Title
                        ui.strong(&notification.title);

                        // Message
                        if !notification.message.is_empty() {
                            ui.label(&notification.message);
                        }

                        // Progress bar for progress notifications
                        if let Some(progress) = notification_progress {
                            ui.add(
                                egui::ProgressBar::new(progress)
                                    .text(format!("{:.0}%", progress * 100.0)),
                            );
                        }

                        // Action buttons
                        if !notification_actions.is_empty() {
                            ui.horizontal(|ui| {
                                for action in &notification_actions {
                                    if ui.small_button(&action.label).clicked() {
                                        actions_to_fire.push((notification_id, action.id.clone()));
                                        if action.dismiss_on_click {
                                            to_dismiss.push(notification_id);
                                        }
                                    }
                                }
                            });
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                        if ui.small_button("x").clicked() {
                            to_dismiss.push(notification_id);
                        }
                    });
                });
            });

        // Update hover state
        if let Some(response) = response {
            notification.hovered = response.response.hovered();
        }

        current_y = window_pos.y;
    }

    // Fire action events
    for (notification_id, action_id) in actions_to_fire {
        action_events.send(NotificationActionEvent {
            notification_id,
            action_id,
        });
    }

    // Process dismissals
    for id in to_dismiss {
        manager.dismiss(id);
    }
}

#[cfg(not(feature = "visual"))]
pub fn render_notifications_system() {}

/// Plugin for the notification system
pub struct NotificationsPlugin;

impl Plugin for NotificationsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NotificationConfig>()
            .init_resource::<NotificationManager>()
            .init_resource::<NotificationHistory>()
            .add_event::<NotifyEvent>()
            .add_event::<NotificationActionEvent>()
            .add_systems(
                Update,
                (
                    process_notify_events,
                    update_notifications_system,
                    render_notifications_system,
                )
                    .chain(),
            );
    }
}

/// Convenience trait for sending notifications via commands
pub trait NotificationCommands {
    /// Sends an info notification
    fn notify_info(&mut self, title: impl Into<String>, message: impl Into<String>);
    /// Sends a success notification
    fn notify_success(&mut self, title: impl Into<String>, message: impl Into<String>);
    /// Sends a warning notification
    fn notify_warning(&mut self, title: impl Into<String>, message: impl Into<String>);
    /// Sends an error notification
    fn notify_error(&mut self, title: impl Into<String>, message: impl Into<String>);
}

impl<'w, 's> NotificationCommands for Commands<'w, 's> {
    fn notify_info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let notification = Notification::info(title).with_message(message);
        self.send_event(NotifyEvent::new(notification));
    }

    fn notify_success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let notification = Notification::success(title).with_message(message);
        self.send_event(NotifyEvent::new(notification));
    }

    fn notify_warning(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let notification = Notification::warning(title).with_message(message);
        self.send_event(NotifyEvent::new(notification));
    }

    fn notify_error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        let notification = Notification::error(title).with_message(message);
        self.send_event(NotifyEvent::new(notification));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification::info("Test Title")
            .with_message("Test message")
            .with_duration(NotificationDuration::Timed(Duration::from_secs(10)));

        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test message");
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(
            notification.duration,
            NotificationDuration::Timed(Duration::from_secs(10))
        );
    }

    #[test]
    fn test_notification_types() {
        let info = Notification::info("Info");
        assert_eq!(info.notification_type, NotificationType::Info);

        let success = Notification::success("Success");
        assert_eq!(success.notification_type, NotificationType::Success);

        let warning = Notification::warning("Warning");
        assert_eq!(warning.notification_type, NotificationType::Warning);

        let error = Notification::error("Error");
        assert_eq!(error.notification_type, NotificationType::Error);

        let progress = Notification::progress("Progress", 0.5);
        assert_eq!(progress.notification_type, NotificationType::Progress);
        assert_eq!(progress.progress, Some(0.5));
    }

    #[test]
    fn test_notification_id_uniqueness() {
        let n1 = Notification::info("Test 1");
        let n2 = Notification::info("Test 2");
        let n3 = Notification::info("Test 3");

        assert_ne!(n1.id, n2.id);
        assert_ne!(n2.id, n3.id);
        assert_ne!(n1.id, n3.id);
    }

    #[test]
    fn test_notification_manager() {
        let mut manager = NotificationManager::new();

        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);

        let id = manager.notify_info("Test", "Message", 0.0);

        assert!(!manager.is_empty());
        assert_eq!(manager.count(), 1);
        assert!(manager.get(id).is_some());
    }

    #[test]
    fn test_notification_manager_dismiss() {
        let mut manager = NotificationManager::new();

        let id1 = manager.notify_info("Test 1", "Message 1", 0.0);
        let id2 = manager.notify_success("Test 2", "Message 2", 0.0);

        assert_eq!(manager.count(), 2);

        manager.dismiss(id1);
        let dismissed = manager.process_dismissals();

        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].id, id1);
        assert_eq!(manager.count(), 1);
        assert!(manager.get(id1).is_none());
        assert!(manager.get(id2).is_some());
    }

    #[test]
    fn test_notification_manager_dismiss_all() {
        let mut manager = NotificationManager::new();

        manager.notify_info("Test 1", "Message 1", 0.0);
        manager.notify_success("Test 2", "Message 2", 0.0);
        manager.notify_warning("Test 3", "Message 3", 0.0);

        assert_eq!(manager.count(), 3);

        manager.dismiss_all();
        let dismissed = manager.process_dismissals();

        assert_eq!(dismissed.len(), 3);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_notification_manager_update_progress() {
        let mut manager = NotificationManager::new();

        let id = manager.notify_progress("Loading", 0.0, 0.0);

        assert!(manager.update_progress(id, 0.5));
        assert_eq!(manager.get(id).unwrap().progress, Some(0.5));

        assert!(manager.update_progress(id, 1.0));
        assert_eq!(manager.get(id).unwrap().progress, Some(1.0));

        // Test clamping
        assert!(manager.update_progress(id, 1.5));
        assert_eq!(manager.get(id).unwrap().progress, Some(1.0));

        // Test non-existent ID
        assert!(!manager.update_progress(99999, 0.5));
    }

    #[test]
    fn test_notification_manager_update_message() {
        let mut manager = NotificationManager::new();

        let id = manager.notify_info("Test", "Original", 0.0);

        assert!(manager.update_message(id, "Updated"));
        assert_eq!(manager.get(id).unwrap().message, "Updated");

        // Test non-existent ID
        assert!(!manager.update_message(99999, "Nothing"));
    }

    #[test]
    fn test_notification_action() {
        let action = NotificationAction::new("action1", "Click Me").with_dismiss(false);

        assert_eq!(action.id, "action1");
        assert_eq!(action.label, "Click Me");
        assert!(!action.dismiss_on_click);
    }

    #[test]
    fn test_notification_with_actions() {
        let notification = Notification::info("Test")
            .with_action(NotificationAction::new("ok", "OK"))
            .with_action(NotificationAction::new("cancel", "Cancel").with_dismiss(true));

        assert_eq!(notification.actions.len(), 2);
        assert_eq!(notification.actions[0].id, "ok");
        assert_eq!(notification.actions[1].id, "cancel");
    }

    #[test]
    fn test_notification_duration() {
        let persistent = Notification::info("Test").persistent();
        assert_eq!(persistent.duration, NotificationDuration::Persistent);
        assert!(persistent.time_remaining.is_none());

        let timed = Notification::info("Test")
            .with_duration(NotificationDuration::Timed(Duration::from_secs(10)));
        assert_eq!(
            timed.duration,
            NotificationDuration::Timed(Duration::from_secs(10))
        );
    }

    #[test]
    fn test_notification_history() {
        let mut history = NotificationHistory::with_capacity(3);

        assert!(history.is_empty());
        assert_eq!(history.len(), 0);

        let n1 = Notification::info("Test 1");
        let n2 = Notification::success("Test 2");

        history.add(n1.clone(), 1.0, false);
        history.add(n2.clone(), 2.0, true);

        assert_eq!(history.len(), 2);

        // Most recent first
        let records = history.records();
        assert_eq!(records[0].notification.title, "Test 2");
        assert_eq!(records[1].notification.title, "Test 1");

        // Test filtering by type
        let info_records = history.records_by_type(NotificationType::Info);
        assert_eq!(info_records.len(), 1);

        let success_records = history.records_by_type(NotificationType::Success);
        assert_eq!(success_records.len(), 1);
    }

    #[test]
    fn test_notification_history_capacity() {
        let mut history = NotificationHistory::with_capacity(2);

        history.add(Notification::info("Test 1"), 1.0, false);
        history.add(Notification::info("Test 2"), 2.0, false);
        history.add(Notification::info("Test 3"), 3.0, false);

        // Should only keep the 2 most recent
        assert_eq!(history.len(), 2);

        let records = history.records();
        assert_eq!(records[0].notification.title, "Test 3");
        assert_eq!(records[1].notification.title, "Test 2");
    }

    #[test]
    fn test_notification_history_clear() {
        let mut history = NotificationHistory::new();

        history.add(Notification::info("Test 1"), 1.0, false);
        history.add(Notification::info("Test 2"), 2.0, false);

        assert_eq!(history.len(), 2);

        history.clear();

        assert!(history.is_empty());
    }

    #[test]
    fn test_notification_config() {
        let config = NotificationConfig::new()
            .with_position(NotificationPosition::BottomLeft)
            .with_max_visible(10)
            .with_default_duration(Duration::from_secs(10))
            .with_sounds(true)
            .with_stack_direction(StackDirection::Up);

        assert_eq!(config.position, NotificationPosition::BottomLeft);
        assert_eq!(config.max_visible, 10);
        assert_eq!(config.default_duration, Duration::from_secs(10));
        assert!(config.enable_sounds);
        assert_eq!(config.stack_direction, StackDirection::Up);
    }

    #[test]
    fn test_notification_config_max_visible_minimum() {
        let config = NotificationConfig::new().with_max_visible(0);

        // Should be clamped to at least 1
        assert_eq!(config.max_visible, 1);
    }

    #[test]
    fn test_notification_type_icons() {
        assert_eq!(NotificationType::Info.icon(), "i");
        assert_eq!(NotificationType::Success.icon(), "+");
        assert_eq!(NotificationType::Warning.icon(), "!");
        assert_eq!(NotificationType::Error.icon(), "x");
        assert_eq!(NotificationType::Progress.icon(), "*");
    }

    #[test]
    fn test_notification_progress_clamping() {
        let n1 = Notification::progress("Test", 1.5);
        assert_eq!(n1.progress, Some(1.0));

        let n2 = Notification::progress("Test", -0.5);
        assert_eq!(n2.progress, Some(0.0));

        let n3 = Notification::progress("Test", 0.5);
        assert_eq!(n3.progress, Some(0.5));
    }

    #[test]
    fn test_timer_updates() {
        let mut manager = NotificationManager::new();

        // Create a notification with 1 second duration
        let mut notification = Notification::info("Test")
            .with_duration(NotificationDuration::Timed(Duration::from_secs(1)));
        notification.timestamp = 0.0;
        notification.time_remaining = Some(Duration::from_secs(1));

        manager.notifications.push_back(notification);

        // Update with 500ms
        let expired = manager.update_timers(Duration::from_millis(500));
        assert!(expired.is_empty());
        assert_eq!(manager.count(), 1);

        // Check remaining time
        let remaining = manager.notifications()[0].time_remaining.unwrap();
        assert_eq!(remaining, Duration::from_millis(500));

        // Update with another 600ms (should expire)
        let expired = manager.update_timers(Duration::from_millis(600));
        assert_eq!(expired.len(), 1);
        assert!(manager.is_empty());
    }

    #[test]
    fn test_persistent_notification_no_expiry() {
        let mut manager = NotificationManager::new();

        let mut notification = Notification::info("Test").persistent();
        notification.timestamp = 0.0;

        manager.notifications.push_back(notification);

        // Update with a very long time
        let expired = manager.update_timers(Duration::from_secs(3600));
        assert!(expired.is_empty());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_notification_position_variants() {
        // Just ensure all variants exist and can be used
        let positions = vec![
            NotificationPosition::TopRight,
            NotificationPosition::TopLeft,
            NotificationPosition::BottomRight,
            NotificationPosition::BottomLeft,
            NotificationPosition::TopCenter,
            NotificationPosition::BottomCenter,
        ];

        for pos in positions {
            let config = NotificationConfig::new().with_position(pos);
            assert_eq!(config.position, pos);
        }
    }

    #[test]
    fn test_notify_event() {
        let notification = Notification::info("Test");
        let id = notification.id;
        let event = NotifyEvent::new(notification);

        assert_eq!(event.notification.id, id);
    }

    #[test]
    fn test_notification_action_event() {
        let event = NotificationActionEvent {
            notification_id: 123,
            action_id: "test_action".to_string(),
        };

        assert_eq!(event.notification_id, 123);
        assert_eq!(event.action_id, "test_action");
    }
}
