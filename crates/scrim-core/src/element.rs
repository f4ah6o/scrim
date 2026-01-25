//! Core UI element types.

use serde::{Deserialize, Serialize};

/// A rectangle in screen coordinates.
///
/// All platforms use top-left origin with logical pixels.
/// macOS coordinate system (bottom-left origin) is converted by platform implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get the center point of this rectangle.
    pub fn center(&self) -> (i32, i32) {
        (
            self.x + self.width as i32 / 2,
            self.y + self.height as i32 / 2,
        )
    }

    /// Check if a point is inside this rectangle.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }
}

/// An interactive UI element discovered via accessibility APIs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    /// Unique identifier for this element (platform-specific reference)
    pub id: String,

    /// Accessibility role (e.g., "button", "link", "checkbox")
    pub role: String,

    /// Accessible label or title
    pub label: Option<String>,

    /// Screen bounds for this element
    pub rect: Rect,

    /// Available actions on this element (e.g., "Press", "Click")
    pub actions: Vec<String>,

    /// Child elements
    pub children: Vec<UIElement>,
}

impl UIElement {
    pub fn new(
        id: String,
        role: String,
        label: Option<String>,
        rect: Rect,
        actions: Vec<String>,
    ) -> Self {
        Self {
            id,
            role,
            label,
            rect,
            actions,
            children: Vec::new(),
        }
    }

    /// Add a child element.
    pub fn add_child(&mut self, child: UIElement) {
        self.children.push(child);
    }

    /// Get all descendant elements recursively (flattened).
    pub fn descendants(&self) -> Vec<&UIElement> {
        let mut result = Vec::new();
        self.collect_descendants(&mut result);
        result
    }

    fn collect_descendants<'a>(&'a self, result: &mut Vec<&'a UIElement>) {
        for child in &self.children {
            result.push(child);
            child.collect_descendants(result);
        }
    }
}

/// Filter for selecting interactive elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementFilter {
    /// Include all elements
    All,
    /// Only elements with specific roles
    Roles(&'static [&'static str]),
    /// Only elements with specific actions available
    HasActions,
}

impl Default for ElementFilter {
    fn default() -> Self {
        ElementFilter::Roles(&["button", "link", "checkbox", "radio", "menuitem"])
    }
}
