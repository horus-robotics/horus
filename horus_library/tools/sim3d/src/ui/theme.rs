//! Theme System for Sim3D Robotics Simulator
//!
//! Provides a comprehensive theming system with dark, light, high contrast,
//! and custom theme support. Includes persistence for user preferences.

#[cfg(feature = "visual")]
use bevy::prelude::*;
#[cfg(feature = "visual")]
use bevy_egui::{egui, EguiContexts};

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Theme variant selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Theme {
    /// Dark theme with dark backgrounds and light text (default)
    #[default]
    Dark,
    /// Light theme with light backgrounds and dark text
    Light,
    /// High contrast theme for accessibility
    HighContrast,
    /// Custom user-defined theme
    Custom,
}

impl Theme {
    /// Get the display name for the theme
    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::HighContrast => "High Contrast",
            Theme::Custom => "Custom",
        }
    }

    /// Get all available theme variants
    pub fn all() -> &'static [Theme] {
        &[
            Theme::Dark,
            Theme::Light,
            Theme::HighContrast,
            Theme::Custom,
        ]
    }

    /// Cycle to the next theme
    pub fn next(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::HighContrast,
            Theme::HighContrast => Theme::Custom,
            Theme::Custom => Theme::Dark,
        }
    }
}

/// RGBA color representation for theme colors
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ThemeColor {
    /// Create a new theme color from RGBA values
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new opaque color from RGB values
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a color from hex value (e.g., 0x1E1E2E)
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// Create a color from hex value with alpha
    pub const fn from_hex_alpha(hex: u32, alpha: u8) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: alpha,
        }
    }

    /// Convert to egui Color32
    #[cfg(feature = "visual")]
    pub fn to_egui(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.r, self.g, self.b, self.a)
    }

    /// Convert to Bevy Color
    #[cfg(feature = "visual")]
    pub fn to_bevy(&self) -> Color {
        Color::srgba_u8(self.r, self.g, self.b, self.a)
    }

    /// Create a lighter version of the color
    pub fn lighten(&self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 + (255.0 - self.r as f32) * amount) as u8,
            g: (self.g as f32 + (255.0 - self.g as f32) * amount) as u8,
            b: (self.b as f32 + (255.0 - self.b as f32) * amount) as u8,
            a: self.a,
        }
    }

    /// Create a darker version of the color
    pub fn darken(&self, amount: f32) -> Self {
        let amount = amount.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 * (1.0 - amount)) as u8,
            g: (self.g as f32 * (1.0 - amount)) as u8,
            b: (self.b as f32 * (1.0 - amount)) as u8,
            a: self.a,
        }
    }

    /// Create a version with different alpha
    pub const fn with_alpha(&self, alpha: u8) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: alpha,
        }
    }

    /// Blend with another color
    pub fn blend(&self, other: &ThemeColor, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 * (1.0 - t) + other.r as f32 * t) as u8,
            g: (self.g as f32 * (1.0 - t) + other.g as f32 * t) as u8,
            b: (self.b as f32 * (1.0 - t) + other.b as f32 * t) as u8,
            a: (self.a as f32 * (1.0 - t) + other.a as f32 * t) as u8,
        }
    }
}

impl Default for ThemeColor {
    fn default() -> Self {
        Self::rgb(128, 128, 128)
    }
}

/// Background color set for different UI layers
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundColors {
    /// Primary background (main window/canvas)
    pub primary: ThemeColor,
    /// Secondary background (panels, sidebars)
    pub secondary: ThemeColor,
    /// Tertiary background (nested elements, tooltips)
    pub tertiary: ThemeColor,
}

impl Default for BackgroundColors {
    fn default() -> Self {
        Self {
            primary: ThemeColor::from_hex(0x1E1E2E),
            secondary: ThemeColor::from_hex(0x313244),
            tertiary: ThemeColor::from_hex(0x45475A),
        }
    }
}

/// Text color set for different emphasis levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextColors {
    /// Primary text (main content)
    pub primary: ThemeColor,
    /// Secondary text (labels, descriptions)
    pub secondary: ThemeColor,
    /// Muted text (hints, disabled state)
    pub muted: ThemeColor,
}

impl Default for TextColors {
    fn default() -> Self {
        Self {
            primary: ThemeColor::from_hex(0xCDD6F4),
            secondary: ThemeColor::from_hex(0xBAC2DE),
            muted: ThemeColor::from_hex(0x6C7086),
        }
    }
}

/// Accent colors for semantic UI elements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccentColors {
    /// Primary accent (active states, selections)
    pub primary: ThemeColor,
    /// Secondary accent (alternative highlights)
    pub secondary: ThemeColor,
    /// Success state (confirmations, completed)
    pub success: ThemeColor,
    /// Warning state (caution, attention needed)
    pub warning: ThemeColor,
    /// Error state (failures, critical)
    pub error: ThemeColor,
    /// Info state (informational messages)
    pub info: ThemeColor,
}

impl Default for AccentColors {
    fn default() -> Self {
        Self {
            primary: ThemeColor::from_hex(0x89B4FA),   // Blue
            secondary: ThemeColor::from_hex(0xCBA6F7), // Purple
            success: ThemeColor::from_hex(0xA6E3A1),   // Green
            warning: ThemeColor::from_hex(0xF9E2AF),   // Yellow
            error: ThemeColor::from_hex(0xF38BA8),     // Red
            info: ThemeColor::from_hex(0x94E2D5),      // Teal
        }
    }
}

/// Panel-specific colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelColors {
    /// Panel header background
    pub header: ThemeColor,
    /// Panel content background
    pub content: ThemeColor,
    /// Panel border color
    pub border: ThemeColor,
    /// Panel shadow color
    pub shadow: ThemeColor,
}

impl Default for PanelColors {
    fn default() -> Self {
        Self {
            header: ThemeColor::from_hex(0x313244),
            content: ThemeColor::from_hex(0x1E1E2E),
            border: ThemeColor::from_hex(0x45475A),
            shadow: ThemeColor::from_hex_alpha(0x000000, 64),
        }
    }
}

/// Button state colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ButtonColors {
    /// Default button state
    pub default: ThemeColor,
    /// Hovered button state
    pub hover: ThemeColor,
    /// Pressed/active button state
    pub pressed: ThemeColor,
    /// Disabled button state
    pub disabled: ThemeColor,
    /// Button text color
    pub text: ThemeColor,
    /// Button border color
    pub border: ThemeColor,
}

impl Default for ButtonColors {
    fn default() -> Self {
        Self {
            default: ThemeColor::from_hex(0x45475A),
            hover: ThemeColor::from_hex(0x585B70),
            pressed: ThemeColor::from_hex(0x313244),
            disabled: ThemeColor::from_hex(0x313244),
            text: ThemeColor::from_hex(0xCDD6F4),
            border: ThemeColor::from_hex(0x6C7086),
        }
    }
}

/// Input field colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputColors {
    /// Input background color
    pub background: ThemeColor,
    /// Input border color
    pub border: ThemeColor,
    /// Input focus border color
    pub focus: ThemeColor,
    /// Input placeholder text color
    pub placeholder: ThemeColor,
    /// Input text color
    pub text: ThemeColor,
    /// Input selection color
    pub selection: ThemeColor,
}

impl Default for InputColors {
    fn default() -> Self {
        Self {
            background: ThemeColor::from_hex(0x1E1E2E),
            border: ThemeColor::from_hex(0x45475A),
            focus: ThemeColor::from_hex(0x89B4FA),
            placeholder: ThemeColor::from_hex(0x6C7086),
            text: ThemeColor::from_hex(0xCDD6F4),
            selection: ThemeColor::from_hex_alpha(0x89B4FA, 100),
        }
    }
}

/// Scrollbar colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScrollbarColors {
    /// Scrollbar track background
    pub track: ThemeColor,
    /// Scrollbar thumb color
    pub thumb: ThemeColor,
    /// Scrollbar thumb hover color
    pub thumb_hover: ThemeColor,
}

impl Default for ScrollbarColors {
    fn default() -> Self {
        Self {
            track: ThemeColor::from_hex_alpha(0x313244, 128),
            thumb: ThemeColor::from_hex(0x45475A),
            thumb_hover: ThemeColor::from_hex(0x585B70),
        }
    }
}

/// Separator and divider colors
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeparatorColors {
    /// Primary separator color
    pub primary: ThemeColor,
    /// Secondary separator color (lighter/subtle)
    pub secondary: ThemeColor,
}

impl Default for SeparatorColors {
    fn default() -> Self {
        Self {
            primary: ThemeColor::from_hex(0x45475A),
            secondary: ThemeColor::from_hex_alpha(0x45475A, 128),
        }
    }
}

/// Complete theme colors structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThemeColors {
    /// Background colors for different layers
    pub background: BackgroundColors,
    /// Text colors for different emphasis levels
    pub text: TextColors,
    /// Accent colors for semantic elements
    pub accent: AccentColors,
    /// Panel-specific colors
    pub panel: PanelColors,
    /// Button state colors
    pub button: ButtonColors,
    /// Input field colors
    pub input: InputColors,
    /// Scrollbar colors
    pub scrollbar: ScrollbarColors,
    /// Separator colors
    pub separator: SeparatorColors,
}

impl ThemeColors {
    /// Create dark theme colors (Catppuccin Mocha inspired)
    pub fn dark() -> Self {
        Self {
            background: BackgroundColors {
                primary: ThemeColor::from_hex(0x1E1E2E),
                secondary: ThemeColor::from_hex(0x313244),
                tertiary: ThemeColor::from_hex(0x45475A),
            },
            text: TextColors {
                primary: ThemeColor::from_hex(0xCDD6F4),
                secondary: ThemeColor::from_hex(0xBAC2DE),
                muted: ThemeColor::from_hex(0x6C7086),
            },
            accent: AccentColors {
                primary: ThemeColor::from_hex(0x89B4FA),
                secondary: ThemeColor::from_hex(0xCBA6F7),
                success: ThemeColor::from_hex(0xA6E3A1),
                warning: ThemeColor::from_hex(0xF9E2AF),
                error: ThemeColor::from_hex(0xF38BA8),
                info: ThemeColor::from_hex(0x94E2D5),
            },
            panel: PanelColors {
                header: ThemeColor::from_hex(0x313244),
                content: ThemeColor::from_hex(0x1E1E2E),
                border: ThemeColor::from_hex(0x45475A),
                shadow: ThemeColor::from_hex_alpha(0x000000, 64),
            },
            button: ButtonColors {
                default: ThemeColor::from_hex(0x45475A),
                hover: ThemeColor::from_hex(0x585B70),
                pressed: ThemeColor::from_hex(0x313244),
                disabled: ThemeColor::from_hex(0x313244),
                text: ThemeColor::from_hex(0xCDD6F4),
                border: ThemeColor::from_hex(0x6C7086),
            },
            input: InputColors {
                background: ThemeColor::from_hex(0x1E1E2E),
                border: ThemeColor::from_hex(0x45475A),
                focus: ThemeColor::from_hex(0x89B4FA),
                placeholder: ThemeColor::from_hex(0x6C7086),
                text: ThemeColor::from_hex(0xCDD6F4),
                selection: ThemeColor::from_hex_alpha(0x89B4FA, 100),
            },
            scrollbar: ScrollbarColors {
                track: ThemeColor::from_hex_alpha(0x313244, 128),
                thumb: ThemeColor::from_hex(0x45475A),
                thumb_hover: ThemeColor::from_hex(0x585B70),
            },
            separator: SeparatorColors {
                primary: ThemeColor::from_hex(0x45475A),
                secondary: ThemeColor::from_hex_alpha(0x45475A, 128),
            },
        }
    }

    /// Create light theme colors (Catppuccin Latte inspired)
    pub fn light() -> Self {
        Self {
            background: BackgroundColors {
                primary: ThemeColor::from_hex(0xEFF1F5),
                secondary: ThemeColor::from_hex(0xE6E9EF),
                tertiary: ThemeColor::from_hex(0xDCE0E8),
            },
            text: TextColors {
                primary: ThemeColor::from_hex(0x4C4F69),
                secondary: ThemeColor::from_hex(0x5C5F77),
                muted: ThemeColor::from_hex(0x9CA0B0),
            },
            accent: AccentColors {
                primary: ThemeColor::from_hex(0x1E66F5),
                secondary: ThemeColor::from_hex(0x8839EF),
                success: ThemeColor::from_hex(0x40A02B),
                warning: ThemeColor::from_hex(0xDF8E1D),
                error: ThemeColor::from_hex(0xD20F39),
                info: ThemeColor::from_hex(0x179299),
            },
            panel: PanelColors {
                header: ThemeColor::from_hex(0xE6E9EF),
                content: ThemeColor::from_hex(0xEFF1F5),
                border: ThemeColor::from_hex(0xCCD0DA),
                shadow: ThemeColor::from_hex_alpha(0x4C4F69, 32),
            },
            button: ButtonColors {
                default: ThemeColor::from_hex(0xDCE0E8),
                hover: ThemeColor::from_hex(0xCCD0DA),
                pressed: ThemeColor::from_hex(0xBCC0CC),
                disabled: ThemeColor::from_hex(0xE6E9EF),
                text: ThemeColor::from_hex(0x4C4F69),
                border: ThemeColor::from_hex(0x9CA0B0),
            },
            input: InputColors {
                background: ThemeColor::from_hex(0xFFFFFF),
                border: ThemeColor::from_hex(0xCCD0DA),
                focus: ThemeColor::from_hex(0x1E66F5),
                placeholder: ThemeColor::from_hex(0x9CA0B0),
                text: ThemeColor::from_hex(0x4C4F69),
                selection: ThemeColor::from_hex_alpha(0x1E66F5, 100),
            },
            scrollbar: ScrollbarColors {
                track: ThemeColor::from_hex_alpha(0xE6E9EF, 180),
                thumb: ThemeColor::from_hex(0xCCD0DA),
                thumb_hover: ThemeColor::from_hex(0xBCC0CC),
            },
            separator: SeparatorColors {
                primary: ThemeColor::from_hex(0xCCD0DA),
                secondary: ThemeColor::from_hex_alpha(0xCCD0DA, 128),
            },
        }
    }

    /// Create high contrast theme for accessibility
    pub fn high_contrast() -> Self {
        Self {
            background: BackgroundColors {
                primary: ThemeColor::from_hex(0x000000),
                secondary: ThemeColor::from_hex(0x1A1A1A),
                tertiary: ThemeColor::from_hex(0x2D2D2D),
            },
            text: TextColors {
                primary: ThemeColor::from_hex(0xFFFFFF),
                secondary: ThemeColor::from_hex(0xE0E0E0),
                muted: ThemeColor::from_hex(0xB0B0B0),
            },
            accent: AccentColors {
                primary: ThemeColor::from_hex(0x00FFFF),   // Cyan
                secondary: ThemeColor::from_hex(0xFF00FF), // Magenta
                success: ThemeColor::from_hex(0x00FF00),   // Bright green
                warning: ThemeColor::from_hex(0xFFFF00),   // Yellow
                error: ThemeColor::from_hex(0xFF0000),     // Red
                info: ThemeColor::from_hex(0x00BFFF),      // Deep sky blue
            },
            panel: PanelColors {
                header: ThemeColor::from_hex(0x1A1A1A),
                content: ThemeColor::from_hex(0x000000),
                border: ThemeColor::from_hex(0xFFFFFF),
                shadow: ThemeColor::from_hex_alpha(0x000000, 0),
            },
            button: ButtonColors {
                default: ThemeColor::from_hex(0x2D2D2D),
                hover: ThemeColor::from_hex(0x404040),
                pressed: ThemeColor::from_hex(0x1A1A1A),
                disabled: ThemeColor::from_hex(0x1A1A1A),
                text: ThemeColor::from_hex(0xFFFFFF),
                border: ThemeColor::from_hex(0xFFFFFF),
            },
            input: InputColors {
                background: ThemeColor::from_hex(0x000000),
                border: ThemeColor::from_hex(0xFFFFFF),
                focus: ThemeColor::from_hex(0x00FFFF),
                placeholder: ThemeColor::from_hex(0x808080),
                text: ThemeColor::from_hex(0xFFFFFF),
                selection: ThemeColor::from_hex_alpha(0x00FFFF, 150),
            },
            scrollbar: ScrollbarColors {
                track: ThemeColor::from_hex(0x1A1A1A),
                thumb: ThemeColor::from_hex(0xFFFFFF),
                thumb_hover: ThemeColor::from_hex(0x00FFFF),
            },
            separator: SeparatorColors {
                primary: ThemeColor::from_hex(0xFFFFFF),
                secondary: ThemeColor::from_hex_alpha(0xFFFFFF, 180),
            },
        }
    }

    /// Get colors for a specific theme variant
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
            Theme::HighContrast => Self::high_contrast(),
            Theme::Custom => Self::dark(), // Custom starts from dark as base
        }
    }
}

/// Theme configuration resource for Bevy
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Currently active theme variant
    pub current_theme: Theme,
    /// Current theme colors
    pub colors: ThemeColors,
    /// Custom theme colors (used when theme is Custom)
    pub custom_colors: ThemeColors,
    /// Enable automatic theme switching based on system preference
    pub auto_switch: bool,
    /// Animation duration for theme transitions (in seconds)
    pub transition_duration: f32,
    /// Path to config file for persistence
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            current_theme: Theme::Dark,
            colors: ThemeColors::dark(),
            custom_colors: ThemeColors::dark(),
            auto_switch: false,
            transition_duration: 0.2,
            config_path: None,
        }
    }
}

impl ThemeConfig {
    /// Create a new theme config with the specified theme
    pub fn new(theme: Theme) -> Self {
        Self {
            current_theme: theme,
            colors: ThemeColors::for_theme(theme),
            custom_colors: ThemeColors::dark(),
            auto_switch: false,
            transition_duration: 0.2,
            config_path: None,
        }
    }

    /// Create a theme config with persistence path
    pub fn with_persistence(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    /// Set the current theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.current_theme = theme;
        self.colors = match theme {
            Theme::Custom => self.custom_colors.clone(),
            _ => ThemeColors::for_theme(theme),
        };
    }

    /// Toggle between dark and light themes
    pub fn toggle_theme(&mut self) {
        let new_theme = match self.current_theme {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
            Theme::HighContrast => Theme::Dark,
            Theme::Custom => Theme::Dark,
        };
        self.set_theme(new_theme);
    }

    /// Cycle to the next theme
    pub fn cycle_theme(&mut self) {
        self.set_theme(self.current_theme.next());
    }

    /// Set custom theme colors
    pub fn set_custom_colors(&mut self, colors: ThemeColors) {
        self.custom_colors = colors.clone();
        if self.current_theme == Theme::Custom {
            self.colors = colors;
        }
    }

    /// Check if current theme is dark
    pub fn is_dark(&self) -> bool {
        matches!(self.current_theme, Theme::Dark | Theme::HighContrast)
    }

    /// Get the default config file path
    pub fn default_config_path() -> Option<PathBuf> {
        // Cross-platform config directory detection
        // Try XDG_CONFIG_HOME first (Linux), then platform-specific config dir
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(dirs::config_dir)
            .map(|p| p.join("sim3d").join("theme.json"))
    }

    /// Save theme preference to config file
    pub fn save(&self) -> Result<(), ThemeError> {
        let path = self
            .config_path
            .clone()
            .or_else(Self::default_config_path)
            .ok_or(ThemeError::NoConfigPath)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ThemeError::IoError(e.to_string()))?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ThemeError::SerializationError(e.to_string()))?;

        fs::write(&path, json).map_err(|e| ThemeError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Load theme preference from config file
    pub fn load(path: Option<PathBuf>) -> Result<Self, ThemeError> {
        let path = path
            .or_else(Self::default_config_path)
            .ok_or(ThemeError::NoConfigPath)?;

        if !path.exists() {
            return Err(ThemeError::ConfigNotFound);
        }

        let json = fs::read_to_string(&path).map_err(|e| ThemeError::IoError(e.to_string()))?;

        let mut config: Self = serde_json::from_str(&json)
            .map_err(|e| ThemeError::DeserializationError(e.to_string()))?;

        config.config_path = Some(path);

        // Ensure colors match the loaded theme
        config.colors = match config.current_theme {
            Theme::Custom => config.custom_colors.clone(),
            _ => ThemeColors::for_theme(config.current_theme),
        };

        Ok(config)
    }

    /// Load theme preference or create default
    pub fn load_or_default(path: Option<PathBuf>) -> Self {
        Self::load(path.clone()).unwrap_or_else(|_| {
            let config_path = path.or_else(Self::default_config_path);
            Self {
                config_path,
                ..Default::default()
            }
        })
    }
}

/// Theme-related errors
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeError {
    /// No config path available
    NoConfigPath,
    /// Config file not found
    ConfigNotFound,
    /// IO error during save/load
    IoError(String),
    /// Serialization error
    SerializationError(String),
    /// Deserialization error
    DeserializationError(String),
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeError::NoConfigPath => write!(f, "No configuration path available"),
            ThemeError::ConfigNotFound => write!(f, "Theme configuration file not found"),
            ThemeError::IoError(e) => write!(f, "IO error: {}", e),
            ThemeError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            ThemeError::DeserializationError(e) => write!(f, "Deserialization error: {}", e),
        }
    }
}

impl std::error::Error for ThemeError {}

/// Event sent when theme changes
#[derive(Event, Clone, Debug)]
pub struct ThemeChangedEvent {
    pub old_theme: Theme,
    pub new_theme: Theme,
}

/// System to apply theme to egui context
#[cfg(feature = "visual")]
pub fn apply_theme_system(mut contexts: EguiContexts, theme_config: Res<ThemeConfig>) {
    if !theme_config.is_changed() {
        return;
    }

    let ctx = contexts.ctx_mut();
    apply_theme_to_egui(ctx, &theme_config.colors);
}

/// Apply theme colors to egui context
#[cfg(feature = "visual")]
pub fn apply_theme_to_egui(ctx: &egui::Context, colors: &ThemeColors) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;

    // Set dark/light mode
    visuals.dark_mode = colors.background.primary.r < 128;

    // Window colors
    visuals.window_fill = colors.background.secondary.to_egui();
    visuals.window_stroke = egui::Stroke::new(1.0, colors.panel.border.to_egui());
    visuals.window_shadow = egui::Shadow {
        offset: egui::vec2(0.0, 2.0),
        blur: 8.0,
        spread: 0.0,
        color: colors.panel.shadow.to_egui(),
    };

    // Panel colors
    visuals.panel_fill = colors.background.secondary.to_egui();

    // Widget colors
    visuals.widgets.noninteractive.bg_fill = colors.background.tertiary.to_egui();
    visuals.widgets.noninteractive.fg_stroke =
        egui::Stroke::new(1.0, colors.text.secondary.to_egui());
    visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, colors.separator.secondary.to_egui());

    visuals.widgets.inactive.bg_fill = colors.button.default.to_egui();
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors.button.text.to_egui());
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors.button.border.to_egui());

    visuals.widgets.hovered.bg_fill = colors.button.hover.to_egui();
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, colors.button.text.to_egui());
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors.accent.primary.to_egui());

    visuals.widgets.active.bg_fill = colors.button.pressed.to_egui();
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, colors.button.text.to_egui());
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, colors.accent.primary.to_egui());

    // Selection colors
    visuals.selection.bg_fill = colors.input.selection.to_egui();
    visuals.selection.stroke = egui::Stroke::new(1.0, colors.accent.primary.to_egui());

    // Hyperlink color
    visuals.hyperlink_color = colors.accent.primary.to_egui();

    // Error color
    visuals.error_fg_color = colors.accent.error.to_egui();
    visuals.warn_fg_color = colors.accent.warning.to_egui();

    // Extreme background
    visuals.extreme_bg_color = colors.background.primary.to_egui();

    // Code background
    visuals.code_bg_color = colors.background.tertiary.to_egui();

    // Faint background
    visuals.faint_bg_color = colors.background.tertiary.with_alpha(128).to_egui();

    // Override text color
    visuals.override_text_color = Some(colors.text.primary.to_egui());

    ctx.set_style(style);
}

/// System to handle theme change events
#[cfg(feature = "visual")]
pub fn handle_theme_change_system(
    mut theme_config: ResMut<ThemeConfig>,
    mut events: EventReader<ThemeChangedEvent>,
) {
    for event in events.read() {
        // Update colors based on new theme
        theme_config.colors = match event.new_theme {
            Theme::Custom => theme_config.custom_colors.clone(),
            _ => ThemeColors::for_theme(event.new_theme),
        };

        // Save preference
        if let Err(e) = theme_config.save() {
            tracing::warn!("Failed to save theme preference: {}", e);
        }
    }
}

/// System to handle keyboard shortcuts for theme switching
#[cfg(feature = "visual")]
pub fn theme_keyboard_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut theme_config: ResMut<ThemeConfig>,
    mut events: EventWriter<ThemeChangedEvent>,
) {
    // Ctrl+Shift+T: Toggle theme
    if (keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight))
        && (keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight))
        && keyboard.just_pressed(KeyCode::KeyT)
    {
        let old_theme = theme_config.current_theme;
        theme_config.toggle_theme();
        events.send(ThemeChangedEvent {
            old_theme,
            new_theme: theme_config.current_theme,
        });
    }
}

/// Theme settings panel for UI
#[cfg(feature = "visual")]
pub fn theme_settings_panel_system(
    mut contexts: EguiContexts,
    mut theme_config: ResMut<ThemeConfig>,
    mut events: EventWriter<ThemeChangedEvent>,
    mut show_panel: Local<bool>,
) {
    // Only show if explicitly enabled (toggle handled elsewhere)
    if !*show_panel {
        return;
    }

    egui::Window::new("Theme Settings")
        .default_width(300.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Theme");
            ui.separator();

            // Theme selection
            ui.horizontal(|ui| {
                ui.label("Current Theme:");
                egui::ComboBox::from_id_salt("theme_selector")
                    .selected_text(theme_config.current_theme.display_name())
                    .show_ui(ui, |ui| {
                        for theme in Theme::all() {
                            let old_theme = theme_config.current_theme;
                            if ui
                                .selectable_label(
                                    theme_config.current_theme == *theme,
                                    theme.display_name(),
                                )
                                .clicked()
                            {
                                theme_config.set_theme(*theme);
                                events.send(ThemeChangedEvent {
                                    old_theme,
                                    new_theme: *theme,
                                });
                            }
                        }
                    });
            });

            ui.add_space(10.0);

            // Quick toggle button
            if ui.button("Toggle Dark/Light").clicked() {
                let old_theme = theme_config.current_theme;
                theme_config.toggle_theme();
                events.send(ThemeChangedEvent {
                    old_theme,
                    new_theme: theme_config.current_theme,
                });
            }

            ui.add_space(10.0);
            ui.separator();
            ui.heading("Preview");

            // Color preview
            ui.horizontal(|ui| {
                ui.label("Background:");
                let color = theme_config.colors.background.primary.to_egui();
                // Draw a colored rectangle as preview
                let (rect, _response) =
                    ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
            });

            ui.horizontal(|ui| {
                ui.label("Text:");
                let color = theme_config.colors.text.primary.to_egui();
                ui.colored_label(color, "Sample Text");
            });

            ui.horizontal(|ui| {
                ui.label("Accent:");
                let color = theme_config.colors.accent.primary.to_egui();
                ui.colored_label(color, "Accent Color");
            });

            ui.add_space(10.0);

            // Status indicators
            ui.horizontal(|ui| {
                ui.colored_label(theme_config.colors.accent.success.to_egui(), "Success");
                ui.colored_label(theme_config.colors.accent.warning.to_egui(), "Warning");
                ui.colored_label(theme_config.colors.accent.error.to_egui(), "Error");
            });

            ui.add_space(10.0);
            ui.separator();

            // Persistence controls
            ui.horizontal(|ui| {
                if ui.button("Save Preferences").clicked() {
                    if let Err(e) = theme_config.save() {
                        tracing::error!("Failed to save theme: {}", e);
                    }
                }
                if ui.button("Reset to Default").clicked() {
                    let old_theme = theme_config.current_theme;
                    theme_config.set_theme(Theme::Dark);
                    events.send(ThemeChangedEvent {
                        old_theme,
                        new_theme: Theme::Dark,
                    });
                }
            });

            // Close button
            ui.add_space(10.0);
            if ui.button("Close").clicked() {
                *show_panel = false;
            }
        });
}

/// Startup system to load and apply saved theme
#[cfg(feature = "visual")]
pub fn load_saved_theme_system(mut theme_config: ResMut<ThemeConfig>) {
    // Try to load saved preferences
    if let Ok(loaded) = ThemeConfig::load(theme_config.config_path.clone()) {
        theme_config.current_theme = loaded.current_theme;
        theme_config.colors = loaded.colors;
        theme_config.custom_colors = loaded.custom_colors;
        theme_config.auto_switch = loaded.auto_switch;
        theme_config.transition_duration = loaded.transition_duration;
        tracing::info!("Loaded theme preference: {:?}", theme_config.current_theme);
    }
}

/// Theme plugin for Bevy integration
pub struct ThemePlugin;

#[cfg(feature = "visual")]
impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ThemeConfig>()
            .add_event::<ThemeChangedEvent>()
            .add_systems(Startup, load_saved_theme_system)
            .add_systems(
                Update,
                (
                    apply_theme_system,
                    handle_theme_change_system,
                    theme_keyboard_system,
                )
                    .chain(),
            );
    }
}

#[cfg(not(feature = "visual"))]
impl Plugin for ThemePlugin {
    fn build(&self, _app: &mut App) {
        // No-op in headless mode
    }
}

#[cfg(not(feature = "visual"))]
use bevy::app::{App, Plugin};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_enum_variants() {
        assert_eq!(Theme::Dark.display_name(), "Dark");
        assert_eq!(Theme::Light.display_name(), "Light");
        assert_eq!(Theme::HighContrast.display_name(), "High Contrast");
        assert_eq!(Theme::Custom.display_name(), "Custom");
    }

    #[test]
    fn test_theme_cycling() {
        assert_eq!(Theme::Dark.next(), Theme::Light);
        assert_eq!(Theme::Light.next(), Theme::HighContrast);
        assert_eq!(Theme::HighContrast.next(), Theme::Custom);
        assert_eq!(Theme::Custom.next(), Theme::Dark);
    }

    #[test]
    fn test_theme_all() {
        let all = Theme::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&Theme::Dark));
        assert!(all.contains(&Theme::Light));
        assert!(all.contains(&Theme::HighContrast));
        assert!(all.contains(&Theme::Custom));
    }

    #[test]
    fn test_theme_color_creation() {
        let color = ThemeColor::new(255, 128, 64, 200);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 200);

        let rgb = ThemeColor::rgb(100, 150, 200);
        assert_eq!(rgb.r, 100);
        assert_eq!(rgb.g, 150);
        assert_eq!(rgb.b, 200);
        assert_eq!(rgb.a, 255);
    }

    #[test]
    fn test_theme_color_from_hex() {
        let color = ThemeColor::from_hex(0x1E1E2E);
        assert_eq!(color.r, 0x1E);
        assert_eq!(color.g, 0x1E);
        assert_eq!(color.b, 0x2E);
        assert_eq!(color.a, 255);

        let color_alpha = ThemeColor::from_hex_alpha(0xFF0000, 128);
        assert_eq!(color_alpha.r, 255);
        assert_eq!(color_alpha.g, 0);
        assert_eq!(color_alpha.b, 0);
        assert_eq!(color_alpha.a, 128);
    }

    #[test]
    fn test_theme_color_lighten() {
        let color = ThemeColor::rgb(100, 100, 100);
        let lighter = color.lighten(0.5);
        assert!(lighter.r > color.r);
        assert!(lighter.g > color.g);
        assert!(lighter.b > color.b);

        // Test boundary
        let white = ThemeColor::rgb(255, 255, 255);
        let still_white = white.lighten(0.5);
        assert_eq!(still_white.r, 255);
    }

    #[test]
    fn test_theme_color_darken() {
        let color = ThemeColor::rgb(200, 200, 200);
        let darker = color.darken(0.5);
        assert!(darker.r < color.r);
        assert!(darker.g < color.g);
        assert!(darker.b < color.b);

        // Test boundary
        let black = ThemeColor::rgb(0, 0, 0);
        let still_black = black.darken(0.5);
        assert_eq!(still_black.r, 0);
    }

    #[test]
    fn test_theme_color_with_alpha() {
        let color = ThemeColor::rgb(100, 150, 200);
        let transparent = color.with_alpha(128);
        assert_eq!(transparent.r, 100);
        assert_eq!(transparent.g, 150);
        assert_eq!(transparent.b, 200);
        assert_eq!(transparent.a, 128);
    }

    #[test]
    fn test_theme_color_blend() {
        let color1 = ThemeColor::rgb(0, 0, 0);
        let color2 = ThemeColor::rgb(255, 255, 255);

        let mid = color1.blend(&color2, 0.5);
        assert!((mid.r as i32 - 127).abs() <= 1);
        assert!((mid.g as i32 - 127).abs() <= 1);
        assert!((mid.b as i32 - 127).abs() <= 1);

        // Test boundaries
        let at_start = color1.blend(&color2, 0.0);
        assert_eq!(at_start.r, 0);

        let at_end = color1.blend(&color2, 1.0);
        assert_eq!(at_end.r, 255);
    }

    #[test]
    fn test_dark_theme_colors() {
        let colors = ThemeColors::dark();

        // Verify dark background
        assert!(colors.background.primary.r < 100);
        assert!(colors.background.primary.g < 100);
        assert!(colors.background.primary.b < 100);

        // Verify light text
        assert!(colors.text.primary.r > 150);
        assert!(colors.text.primary.g > 150);
        assert!(colors.text.primary.b > 150);

        // Verify accent colors are distinct
        assert_ne!(colors.accent.success, colors.accent.error);
        assert_ne!(colors.accent.warning, colors.accent.info);
    }

    #[test]
    fn test_light_theme_colors() {
        let colors = ThemeColors::light();

        // Verify light background
        assert!(colors.background.primary.r > 200);
        assert!(colors.background.primary.g > 200);
        assert!(colors.background.primary.b > 200);

        // Verify dark text (Catppuccin Latte has slight blue tint, 0x4C4F69 = RGB 76, 79, 105)
        assert!(colors.text.primary.r < 100);
        assert!(colors.text.primary.g < 100);
        assert!(colors.text.primary.b < 120); // Allow for Catppuccin's blue-tinted text
    }

    #[test]
    fn test_high_contrast_theme_colors() {
        let colors = ThemeColors::high_contrast();

        // Verify pure black background
        assert_eq!(colors.background.primary.r, 0);
        assert_eq!(colors.background.primary.g, 0);
        assert_eq!(colors.background.primary.b, 0);

        // Verify pure white text
        assert_eq!(colors.text.primary.r, 255);
        assert_eq!(colors.text.primary.g, 255);
        assert_eq!(colors.text.primary.b, 255);

        // Verify bright accent colors for visibility
        let success = colors.accent.success;
        assert!(success.r > 200 || success.g > 200 || success.b > 200);
    }

    #[test]
    fn test_theme_colors_for_theme() {
        let dark = ThemeColors::for_theme(Theme::Dark);
        let light = ThemeColors::for_theme(Theme::Light);
        let high_contrast = ThemeColors::for_theme(Theme::HighContrast);
        let custom = ThemeColors::for_theme(Theme::Custom);

        // Dark and custom should be the same (custom defaults to dark)
        assert_eq!(dark, custom);

        // Dark and light should be different
        assert_ne!(dark.background.primary, light.background.primary);

        // High contrast should have pure black/white
        assert_eq!(high_contrast.background.primary.r, 0);
        assert_eq!(high_contrast.text.primary.r, 255);
    }

    #[test]
    fn test_theme_config_creation() {
        let config = ThemeConfig::new(Theme::Light);
        assert_eq!(config.current_theme, Theme::Light);
        assert_eq!(config.colors, ThemeColors::light());
    }

    #[test]
    fn test_theme_config_set_theme() {
        let mut config = ThemeConfig::new(Theme::Dark);
        assert_eq!(config.current_theme, Theme::Dark);

        config.set_theme(Theme::Light);
        assert_eq!(config.current_theme, Theme::Light);
        assert_eq!(config.colors, ThemeColors::light());

        config.set_theme(Theme::HighContrast);
        assert_eq!(config.current_theme, Theme::HighContrast);
        assert_eq!(config.colors, ThemeColors::high_contrast());
    }

    #[test]
    fn test_theme_config_toggle() {
        let mut config = ThemeConfig::new(Theme::Dark);
        assert_eq!(config.current_theme, Theme::Dark);

        config.toggle_theme();
        assert_eq!(config.current_theme, Theme::Light);

        config.toggle_theme();
        assert_eq!(config.current_theme, Theme::Dark);
    }

    #[test]
    fn test_theme_config_cycle() {
        let mut config = ThemeConfig::new(Theme::Dark);

        config.cycle_theme();
        assert_eq!(config.current_theme, Theme::Light);

        config.cycle_theme();
        assert_eq!(config.current_theme, Theme::HighContrast);

        config.cycle_theme();
        assert_eq!(config.current_theme, Theme::Custom);

        config.cycle_theme();
        assert_eq!(config.current_theme, Theme::Dark);
    }

    #[test]
    fn test_theme_config_custom_colors() {
        let mut config = ThemeConfig::new(Theme::Dark);

        let custom = ThemeColors::light();
        config.set_custom_colors(custom.clone());

        // Should not affect current theme yet
        assert_eq!(config.colors, ThemeColors::dark());

        // Switch to custom theme
        config.set_theme(Theme::Custom);
        assert_eq!(config.colors, custom);
    }

    #[test]
    fn test_theme_config_is_dark() {
        let mut config = ThemeConfig::new(Theme::Dark);
        assert!(config.is_dark());

        config.set_theme(Theme::Light);
        assert!(!config.is_dark());

        config.set_theme(Theme::HighContrast);
        assert!(config.is_dark());

        config.set_theme(Theme::Custom);
        assert!(!config.is_dark()); // Custom is not dark by enum match
    }

    #[test]
    fn test_theme_config_persistence() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let config_path = dir.path().join("theme.json");

        // Create and save config
        let mut config = ThemeConfig::new(Theme::Light);
        config.config_path = Some(config_path.clone());
        config.save().unwrap();

        // Verify file exists
        assert!(config_path.exists());

        // Load and verify
        let loaded = ThemeConfig::load(Some(config_path.clone())).unwrap();
        assert_eq!(loaded.current_theme, Theme::Light);
    }

    #[test]
    fn test_theme_config_load_or_default() {
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let nonexistent_path = dir.path().join("nonexistent.json");

        // Should return default when file doesn't exist
        let config = ThemeConfig::load_or_default(Some(nonexistent_path));
        assert_eq!(config.current_theme, Theme::Dark);
    }

    #[test]
    fn test_theme_error_display() {
        let err = ThemeError::NoConfigPath;
        assert_eq!(format!("{}", err), "No configuration path available");

        let err = ThemeError::ConfigNotFound;
        assert_eq!(format!("{}", err), "Theme configuration file not found");

        let err = ThemeError::IoError("test error".to_string());
        assert_eq!(format!("{}", err), "IO error: test error");
    }

    #[test]
    fn test_background_colors_default() {
        let bg = BackgroundColors::default();
        assert_eq!(bg.primary, ThemeColor::from_hex(0x1E1E2E));
        assert_eq!(bg.secondary, ThemeColor::from_hex(0x313244));
        assert_eq!(bg.tertiary, ThemeColor::from_hex(0x45475A));
    }

    #[test]
    fn test_text_colors_default() {
        let text = TextColors::default();
        assert_eq!(text.primary, ThemeColor::from_hex(0xCDD6F4));
        assert_eq!(text.secondary, ThemeColor::from_hex(0xBAC2DE));
        assert_eq!(text.muted, ThemeColor::from_hex(0x6C7086));
    }

    #[test]
    fn test_accent_colors_default() {
        let accent = AccentColors::default();
        assert_eq!(accent.primary, ThemeColor::from_hex(0x89B4FA));
        assert_eq!(accent.success, ThemeColor::from_hex(0xA6E3A1));
        assert_eq!(accent.warning, ThemeColor::from_hex(0xF9E2AF));
        assert_eq!(accent.error, ThemeColor::from_hex(0xF38BA8));
    }

    #[test]
    fn test_button_colors_default() {
        let button = ButtonColors::default();
        assert!(button.hover != button.default);
        assert!(button.pressed != button.default);
        assert_eq!(button.disabled, ThemeColor::from_hex(0x313244));
    }

    #[test]
    fn test_input_colors_default() {
        let input = InputColors::default();
        assert!(input.focus != input.border);
        assert_eq!(input.text, ThemeColor::from_hex(0xCDD6F4));
    }

    #[test]
    fn test_theme_config_serialization() {
        let config = ThemeConfig::new(Theme::Light);
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ThemeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.current_theme, Theme::Light);
        assert_eq!(deserialized.colors, ThemeColors::light());
    }

    #[test]
    fn test_theme_colors_equality() {
        let dark1 = ThemeColors::dark();
        let dark2 = ThemeColors::dark();
        let light = ThemeColors::light();

        assert_eq!(dark1, dark2);
        assert_ne!(dark1, light);
    }

    #[test]
    fn test_theme_changed_event() {
        let event = ThemeChangedEvent {
            old_theme: Theme::Dark,
            new_theme: Theme::Light,
        };

        assert_eq!(event.old_theme, Theme::Dark);
        assert_eq!(event.new_theme, Theme::Light);
    }

    #[test]
    fn test_scrollbar_colors_default() {
        let scrollbar = ScrollbarColors::default();
        assert!(scrollbar.thumb != scrollbar.track);
        assert!(scrollbar.thumb_hover != scrollbar.thumb);
    }

    #[test]
    fn test_separator_colors_default() {
        let separator = SeparatorColors::default();
        assert_eq!(separator.primary, ThemeColor::from_hex(0x45475A));
        assert_eq!(separator.secondary.a, 128);
    }

    #[test]
    fn test_panel_colors_default() {
        let panel = PanelColors::default();
        assert_eq!(panel.header, ThemeColor::from_hex(0x313244));
        assert_eq!(panel.content, ThemeColor::from_hex(0x1E1E2E));
        assert!(panel.shadow.a < 255); // Shadow should be semi-transparent
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme, Theme::Dark);
    }

    #[test]
    fn test_theme_config_default() {
        let config = ThemeConfig::default();
        assert_eq!(config.current_theme, Theme::Dark);
        assert_eq!(config.colors, ThemeColors::dark());
        assert!(!config.auto_switch);
        assert_eq!(config.transition_duration, 0.2);
    }

    #[test]
    fn test_theme_color_default() {
        let color = ThemeColor::default();
        assert_eq!(color.r, 128);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 128);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_lighten_darken_clamp() {
        let color = ThemeColor::rgb(128, 128, 128);

        // Test clamping of amount
        let lighter = color.lighten(2.0); // Should clamp to 1.0
        let expected = color.lighten(1.0);
        assert_eq!(lighter, expected);

        let darker = color.darken(-1.0); // Should clamp to 0.0
        assert_eq!(darker, color);
    }

    #[test]
    fn test_blend_clamp() {
        let color1 = ThemeColor::rgb(0, 0, 0);
        let color2 = ThemeColor::rgb(255, 255, 255);

        // Test clamping of t
        let blend_over = color1.blend(&color2, 2.0); // Should clamp to 1.0
        assert_eq!(blend_over, color2);

        let blend_under = color1.blend(&color2, -1.0); // Should clamp to 0.0
        assert_eq!(blend_under, color1);
    }
}
