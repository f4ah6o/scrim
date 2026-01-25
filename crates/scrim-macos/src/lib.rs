//! macOS accessibility implementation using the Accessibility API (AX).

#[cfg(target_os = "macos")]
pub mod client;
#[cfg(target_os = "macos")]
pub mod permissions;

#[cfg(target_os = "macos")]
pub use client::MacOSAccessibilityClient;
