//! Core types and traits for scrim cross-platform keyboard navigation system.
//!
//! This crate provides platform-agnostic abstractions for UI element introspection
//! and interaction via accessibility APIs.

mod element;
mod hint;
mod traits;

pub use element::{ElementFilter, Rect, UIElement};
pub use hint::Hint;
pub use traits::{AccessibilityClient, AccessibilityError, Result};
