//! Progress indicators for HORUS CLI
//!
//! Provides WALL-E themed spinners and progress bars for long-running operations.
//! Uses UTF-8 characters for the compacting trash animation.

use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use console::style;

/// Global quiet mode flag
static QUIET_MODE: AtomicBool = AtomicBool::new(false);

/// Set global quiet mode
pub fn set_quiet(quiet: bool) {
    QUIET_MODE.store(quiet, Ordering::SeqCst);
}

/// Check if quiet mode is enabled
pub fn is_quiet() -> bool {
    QUIET_MODE.load(Ordering::SeqCst)
}

// =============================================================================
// WALL-E Robot Faces (UTF-8)
// =============================================================================
// [□_□]  - Normal WALL-E
// [■_■]  - Happy/success WALL-E (filled eyes)
// [×_×]  - Error WALL-E
// [□■□]  - WALL-E eating (one eye filled)
// [▣_▣]  - WALL-E compacting (checkered eyes)
// [▪_▪]  - WALL-E compressed (small eyes)
// [;_;]  - Sad WALL-E
// =============================================================================

/// WALL-E compacting trash animation (UTF-8)
/// Sees trash ▮▮▮, eats it, compacts, ejects small cubes ▫▫▫
pub const ROBOT_SPINNER: &[&str] = &[
    "[□_□]  ▮▮▮",
    "[□_□] ▮▮▮ ",
    "[□_□]▮▮▮  ",
    "[□■□]▮▮   ",
    "[■_■]▮    ",
    "[▣_▣]     ",
    "[▪_▪]     ",
    "[□_□]▫    ",
    "[□_□] ▫▫  ",
    "[□_□]  ▫▫▫",
];

/// WALL-E compacting animation (same as spinner, for builds)
pub const ROBOT_BUILD: &[&str] = &[
    "[□_□]  ▮▮▮",
    "[□_□] ▮▮▮ ",
    "[□_□]▮▮▮  ",
    "[□■□]▮▮   ",
    "[■_■]▮    ",
    "[▣_▣]     ",
    "[▪_▪]     ",
    "[□_□]▫    ",
    "[□_□] ▫▫  ",
    "[□_□]  ▫▫▫",
];

/// WALL-E downloading animation
pub const ROBOT_DOWNLOAD: &[&str] = &[
    "[□_□]  ▮▮▮",
    "[□_□] ▮▮▮ ",
    "[□_□]▮▮▮  ",
    "[□■□]▮▮   ",
    "[■_■]▮    ",
    "[▣_▣]     ",
    "[▪_▪]     ",
    "[□_□]▫    ",
    "[□_□] ▫▫  ",
    "[□_□]  ▫▫▫",
];

/// WALL-E thinking animation
pub const ROBOT_THINK: &[&str] = &[
    "[□_□]?    ",
    "[□_□] ?   ",
    "[□_□]  ?  ",
    "[□_□]   ? ",
    "[□_□]  ?  ",
    "[□_□] ?   ",
];

/// WALL-E with success (filled eyes)
pub const ROBOT_SUCCESS: &str = "[■_■]";

/// WALL-E with warning
pub const ROBOT_WARNING: &str = "[□_□]!";

/// WALL-E with error
pub const ROBOT_ERROR: &str = "[×_×]";

/// Simple dots for less intrusive operations
pub const DOTS: &[&str] = &[".", "..", "...", "....", "...", "..", "."];

/// Create a robot spinner for indefinite operations (wheel rolling)
pub fn robot_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_SPINNER)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a robot rolling spinner (uses WALL-E compacting animation)
pub fn robot_roll_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_SPINNER)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a robot build spinner
pub fn robot_build_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_BUILD)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(150));
    pb
}

/// Create a robot run spinner (uses WALL-E compacting animation)
pub fn robot_run_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_SPINNER)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a robot download/install spinner
pub fn robot_download_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_DOWNLOAD)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

/// Create a robot thinking spinner
pub fn robot_think_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(ROBOT_THINK)
            .template("{spinner} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(200));
    pb
}

/// Create a subtle dots spinner (for less prominent operations)
pub fn dots_spinner(message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(DOTS)
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(150));
    pb
}

/// Create a progress bar with WALL-E theme (UTF-8)
pub fn robot_progress_bar(total: u64, message: &str) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[□_□] {msg}\n      [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("▮▫-")
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a progress bar for byte downloads (WALL-E theme)
pub fn download_progress_bar(total: u64) -> ProgressBar {
    if is_quiet() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[□_□]~ {msg}\n       [{elapsed_precise}] [{bar:40.green/black}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("▮▫-")
    );
    pb
}

/// Finish a spinner with success
pub fn finish_success(pb: &ProgressBar, message: &str) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{msg}")
            .unwrap()
    );
    pb.finish_with_message(format!("{} {}", ROBOT_SUCCESS, style(message).green()));
}

/// Finish a spinner with warning
pub fn finish_warning(pb: &ProgressBar, message: &str) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{msg}")
            .unwrap()
    );
    pb.finish_with_message(format!("{} {}", ROBOT_WARNING, style(message).yellow()));
}

/// Finish a spinner with error
pub fn finish_error(pb: &ProgressBar, message: &str) {
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{msg}")
            .unwrap()
    );
    pb.finish_with_message(format!("{} {}", ROBOT_ERROR, style(message).red()));
}

/// Finish a spinner and clear it (no message)
pub fn finish_clear(pb: &ProgressBar) {
    pb.finish_and_clear();
}

/// A multi-progress manager for parallel operations
pub struct RobotMultiProgress {
    mp: MultiProgress,
    quiet: bool,
}

impl RobotMultiProgress {
    pub fn new(quiet: bool) -> Self {
        Self {
            mp: MultiProgress::new(),
            quiet,
        }
    }

    /// Add a robot spinner
    pub fn add_spinner(&self, message: &str) -> ProgressBar {
        if self.quiet {
            return ProgressBar::hidden();
        }
        let pb = robot_spinner(message);
        self.mp.add(pb)
    }

    /// Add a build spinner
    pub fn add_build_spinner(&self, message: &str) -> ProgressBar {
        if self.quiet {
            return ProgressBar::hidden();
        }
        let pb = robot_build_spinner(message);
        self.mp.add(pb)
    }

    /// Add a progress bar
    pub fn add_progress_bar(&self, total: u64, message: &str) -> ProgressBar {
        if self.quiet {
            return ProgressBar::hidden();
        }
        let pb = robot_progress_bar(total, message);
        self.mp.add(pb)
    }
}

/// Helper to create a spinner that respects quiet mode
pub fn maybe_spinner(quiet: bool, message: &str) -> ProgressBar {
    if quiet {
        ProgressBar::hidden()
    } else {
        robot_spinner(message)
    }
}

/// Helper to create a build spinner that respects quiet mode
pub fn maybe_build_spinner(quiet: bool, message: &str) -> ProgressBar {
    if quiet {
        ProgressBar::hidden()
    } else {
        robot_build_spinner(message)
    }
}

/// Helper to create a run spinner that respects quiet mode
pub fn maybe_run_spinner(quiet: bool, message: &str) -> ProgressBar {
    if quiet {
        ProgressBar::hidden()
    } else {
        robot_run_spinner(message)
    }
}

/// Helper to create a download spinner that respects quiet mode
pub fn maybe_download_spinner(quiet: bool, message: &str) -> ProgressBar {
    if quiet {
        ProgressBar::hidden()
    } else {
        robot_download_spinner(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_robot_spinner_creation() {
        let pb = robot_spinner("Testing...");
        thread::sleep(Duration::from_millis(500));
        finish_success(&pb, "Test complete!");
    }

    #[test]
    fn test_quiet_mode() {
        let pb = maybe_spinner(true, "Should be hidden");
        assert!(pb.is_hidden());
    }
}
