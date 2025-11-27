pub mod controls;
pub mod crash_recovery;
pub mod debug_panel;
pub mod dock;
pub mod keybindings;
pub mod layouts;
pub mod notifications;
pub mod plugin_panel;
pub mod recent_files;
pub mod stats_panel;
pub mod status_bar;
pub mod tf_panel;
pub mod theme;
pub mod tooltips;
pub mod view_modes;

// Re-export theme components for convenience
pub use theme::{
    Theme, ThemeChangedEvent, ThemeColor, ThemeColors, ThemeConfig, ThemeError, ThemePlugin,
};

// Re-export notification components for convenience
pub use notifications::{
    Notification, NotificationAction, NotificationActionEvent, NotificationConfig,
    NotificationDuration, NotificationHistory, NotificationManager, NotificationPosition,
    NotificationType, NotificationsPlugin, NotifyEvent, StackDirection,
};

// Re-export status bar components for convenience
pub use status_bar::{
    EditorToolState, MouseWorldPosition, SceneState, SimulationStatusInfo, StatusBarConfig,
    StatusBarManager, StatusBarPlugin, StatusBarSection, StatusItem, StatusItemClickEvent,
    StatusItemColor, StatusItemContent,
};

// Re-export keybindings for convenience
pub use keybindings::{
    check_keybinding, check_keybindings, KeyBinding, KeyBindingAction, KeyBindingCategory,
    KeyBindingError, KeyBindingMap, KeyBindingPreset, KeyBindingTriggeredEvent, KeyBindingsPlugin,
    KeyCombination, KeyModifiers,
};

// Re-export layouts components for convenience
pub use layouts::{
    LayoutConfig, LayoutEvent, LayoutManager, LayoutPlugin, LayoutPreset, PanelAnchor, PanelConfig,
    ViewportConfig, ViewportMargins,
};

// Re-export tooltips components for convenience
pub use tooltips::{
    ContextualHelp, HelpOverlay, HelpOverlayState, HelpRegistry, Tooltip, TooltipConfig,
    TooltipEvent, TooltipPosition, TooltipRegistry, TooltipStyle, TooltipsPlugin,
};

// Re-export recent files components for convenience
pub use recent_files::{
    AddRecentFileEvent, ClearRecentFilesEvent, RecentFile, RecentFileSelectedEvent, RecentFileType,
    RecentFilesConfig, RecentFilesError, RecentFilesManager, RecentFilesPlugin,
};

// Re-export crash recovery components for convenience
pub use crash_recovery::{
    AutoSaveCompletedEvent, AutoSaveConfig, AutoSaveState, CrashRecoveryError,
    CrashRecoveryManager, CrashRecoveryPlugin, RecoveryAction, RecoveryFile,
    RecoveryFilesDetectedEvent, RecoverySelectedEvent, RecoveryState, SceneChangedEvent,
    SceneSavedEvent, TriggerAutoSaveEvent,
};

// Re-export plugin panel components for convenience
pub use plugin_panel::{
    OpenPluginSettingsEvent, PluginPanelConfig, PluginPanelPlugin, PluginUiRegistry, SettingsTab,
    ToggleSettingsPanelEvent,
};

// Re-export dock components for convenience
pub use dock::{
    AddPluginTabEvent, ChangeDockLayoutEvent, DockContext, DockLayoutPreset, DockPlugin, DockTab,
    DockWorkspace, SimDockViewer,
};
