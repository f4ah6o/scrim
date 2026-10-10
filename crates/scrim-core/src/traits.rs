//! Platform-agnostic accessibility client trait.

use crate::{ElementFilter, UIElement};

/// Error type for accessibility operations.
pub type Result<T, E = AccessibilityError> = std::result::Result<T, E>;

/// Errors that can occur during accessibility operations.
#[derive(Debug, thiserror::Error)]
pub enum AccessibilityError {
    #[error("Accessibility permissions not granted")]
    PermissionDenied,

    #[error("No focused window found")]
    NoFocusedWindow,

    #[error("Element not found: {0}")]
    ElementNotFound(String),

    #[error("Action failed: {action} on {element_id}")]
    ActionFailed { element_id: String, action: String },

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Platform-agnostic accessibility client trait.
///
/// This trait abstracts over platform-specific accessibility APIs:
/// - macOS: Accessibility API (AX)
/// - Windows: UI Automation (UIA)
/// - Linux: AT-SPI2
#[async_trait::async_trait]
pub trait AccessibilityClient: Send + Sync {
    /// Check if the process has accessibility permissions.
    fn is_trusted(&self) -> bool;

    /// Request accessibility permissions from the user (opens system prompt).
    /// Returns `false` if the user denies or the platform doesn't support prompting.
    fn request_trust(&self) -> bool;

    /// Get the currently focused window.
    fn get_focused_window(&self) -> Result<UIElement>;

    /// Traverse the UI tree starting from root, returning matching elements.
    fn traverse_tree(&self, root: &UIElement, filter: ElementFilter) -> Vec<UIElement>;

    /// Perform an action on an element (e.g., "Press", "Click").
    async fn perform_action(&self, element_id: &str, action: &str) -> Result<()>;

    /// Get all interactive elements in the focused window.
    async fn get_interactive_elements(&self, filter: ElementFilter) -> Result<Vec<UIElement>> {
        let root = self.get_focused_window()?;
        Ok(self.traverse_tree(&root, filter))
    }
}
