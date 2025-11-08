//! # HORUS Macros
//!
//! Procedural macros for the HORUS robotics framework.
//!
//! This crate provides derive macros and function-like macros to reduce
//! boilerplate and improve the developer experience when building HORUS applications.
//!
//! ## Available Macros
//!
//! - `#[node]` - Generate Node trait implementation with automatic topic registration
//!
//! ## Safety
//!
//! These macros generate safe code and use proper error handling with `HorusError`.
//! All generated code follows Rust safety guidelines and avoids undefined behavior.

use proc_macro::TokenStream;

mod message;
mod node;

/// Generate a HORUS node implementation with automatic topic registration.
///
/// # Example
///
/// ```rust,ignore
/// use horus_macros::node;
/// use horus::prelude::*;
///
/// node! {
///     CameraNode {
///         pub {
///             image: Image -> "camera/image",
///             status: Status -> "camera/status",
///         }
///
///         sub {
///             command: Command -> "camera/command",
///         }
///
///         data {
///             frame_count: u32 = 0,
///             buffer: Vec<u8> = Vec::new(),
///         }
///
///         tick(ctx) {
///             // Process one message per tick (bounded execution time)
///             if let Some(cmd) = self.command.recv(ctx) {
///                 // Process command
///             }
///             self.frame_count += 1;
///             let img = self.capture_frame();
///             self.image.send(img, ctx).ok();
///         }
///     }
/// }
/// ```
///
/// This generates:
/// - Complete struct definition with Hub fields
/// - `new()` constructor that creates all Hubs
/// - `Node` trait implementation
/// - `Default` trait implementation
/// - Automatic snake_case node naming
///
/// # Sections
///
/// - `pub {}` - Publishers (optional, can be empty)
/// - `sub {}` - Subscribers (optional, can be empty)
/// - `data {}` - Internal state fields (optional)
/// - `tick {}` - Main update logic (required)
/// - `init(ctx) {}` - Initialization (optional)
/// - `shutdown(ctx) {}` - Cleanup (optional)
/// - `impl {}` - Additional methods (optional)
#[proc_macro]
pub fn node(input: TokenStream) -> TokenStream {
    node::impl_node_macro(input)
}

/// Define a HORUS message type with automatic trait implementations.
///
/// This macro generates a message type with all necessary traits:
/// - `Debug`, `Clone`, `Serialize`, `Deserialize`
/// - `LogSummary` (for efficient logging without cloning)
/// - `Pod`, `Zeroable` (for zero-copy serialization, if fields support it)
///
/// # Syntax
///
/// ## Tuple-style (recommended for simple types):
///
/// ```rust,ignore
/// use horus_macros::message;
///
/// message!(Position = (f32, f32));
/// message!(Color = (u8, u8, u8));
/// message!(Command = (u32, bool));
/// ```
///
/// ## Struct-style (for complex types):
///
/// ```rust,ignore
/// message! {
///     RobotStatus {
///         position_x: f32,
///         position_y: f32,
///         battery: u8,
///         is_moving: bool,
///     }
/// }
/// ```
///
/// # Generated Code
///
/// For `message!(Position = (f32, f32))`, generates:
///
/// ```rust,ignore
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// #[repr(C)]
/// pub struct Position(pub f32, pub f32);
///
/// impl LogSummary for Position {
///     fn log_summary(&self) -> String {
///         format!("{:?}", self)
///     }
/// }
///
/// unsafe impl bytemuck::Pod for Position { }
/// unsafe impl bytemuck::Zeroable for Position { }
/// ```
///
/// # Usage with Hub
///
/// ```rust,ignore
/// message!(Position = (f32, f32));
///
/// let hub = Hub::<Position>::new("robot/position")?;
/// hub.send(Position(1.0, 2.0), ctx)?;  // Works automatically!
/// ```
///
/// # Benefits
///
/// - **Zero boilerplate**: One line defines everything
/// - **No breaking changes**: LogSummary is auto-implemented
/// - **Performance**: Large messages can override LogSummary manually
/// - **Type safety**: All fields are strongly typed
#[proc_macro]
pub fn message(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as message::MessageInput);
    let output = message::generate_message(input);
    TokenStream::from(output)
}
