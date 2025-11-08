//! Extension trait for Option<&mut NodeInfo> to simplify logging
//!
//! This allows clean logging syntax:
//! ```ignore
//! ctx.log_debug("message");  // Instead of if let Some(ref mut c) = ctx { c.log_debug(...) }
//! ```

use super::NodeInfo;

/// Extension trait for Option<&mut NodeInfo> to enable direct logging calls
pub trait NodeInfoExt {
    /// Log a debug message (only if ctx is Some and logging is enabled)
    fn log_debug(&mut self, message: &str);

    /// Log an info message (only if ctx is Some and logging is enabled)
    fn log_info(&mut self, message: &str);

    /// Log a warning message (only if ctx is Some and logging is enabled)
    fn log_warning(&mut self, message: &str);

    /// Log an error message (only if ctx is Some and logging is enabled)
    fn log_error(&mut self, message: &str);
}

impl NodeInfoExt for Option<&mut NodeInfo> {
    #[inline]
    fn log_debug(&mut self, message: &str) {
        if let Some(ref mut ctx) = self {
            ctx.log_debug(message);
        }
    }

    #[inline]
    fn log_info(&mut self, message: &str) {
        if let Some(ref mut ctx) = self {
            ctx.log_info(message);
        }
    }

    #[inline]
    fn log_warning(&mut self, message: &str) {
        if let Some(ref mut ctx) = self {
            ctx.log_warning(message);
        }
    }

    #[inline]
    fn log_error(&mut self, message: &str) {
        if let Some(ref mut ctx) = self {
            ctx.log_error(message);
        }
    }
}
