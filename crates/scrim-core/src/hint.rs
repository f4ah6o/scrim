//! Hint label generation for UI elements.
//!
//! Generates short labels (e.g., "AA", "AB", "AC") that users can type
//! to quickly select and activate UI elements.

use std::collections::HashMap;

/// A hint label for a UI element.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hint {
    /// The hint string (e.g., "AA", "AB", "AC")
    pub label: String,
    /// Index of the target element
    pub target_index: usize,
}

impl Hint {
    pub fn new(label: String, target_index: usize) -> Self {
        Self {
            label,
            target_index,
        }
    }
}

/// Hint generator using variable-length labels.
///
/// Labels are generated using a base-N character set, starting with single
/// characters, then expanding to two characters when needed.
///
/// Example with home row keys:
/// - First elements: "A", "S", "D", "F"...
/// - Then: "AA", "AS", "AD", "AF"...
/// - Then: "SA", "SS", "SD", "SF"...
pub struct HintGenerator {
    /// Characters to use for hint labels
    chars: Vec<char>,
    /// Map from element index to hint label
    hints: HashMap<usize, String>,
}

impl HintGenerator {
    /// Create a new hint generator with the default home row key set.
    ///
    /// Default characters: ASDFGHJKL (home row left hand, home row right hand)
    pub fn new() -> Self {
        Self::with_chars("ASDFGHJKL")
    }

    /// Create a hint generator with a custom character set.
    pub fn with_chars(chars: &str) -> Self {
        Self {
            chars: chars.chars().collect(),
            hints: HashMap::new(),
        }
    }

    /// Generate hint labels for the given number of elements.
    pub fn generate(&mut self, count: usize) -> Vec<Hint> {
        self.hints.clear();
        let mut hints = Vec::with_capacity(count);

        for i in 0..count {
            let label = self.index_to_label(i);
            self.hints.insert(i, label.clone());
            hints.push(Hint::new(label, i));
        }

        hints
    }

    /// Convert an index to a hint label.
    fn index_to_label(&self, mut index: usize) -> String {
        let base = self.chars.len();
        let mut label = String::new();

        loop {
            label.push(self.chars[index % base]);
            index /= base;
            if index == 0 {
                break;
            }
            index -= 1; // Adjust for 1-based indexing
        }

        label.chars().rev().collect()
    }

    /// Get the hint label for a specific element index.
    pub fn get_hint(&self, index: usize) -> Option<&str> {
        self.hints.get(&index).map(|s| s.as_str())
    }

    /// Find the element index for a given hint label.
    pub fn find_index(&self, label: &str) -> Option<usize> {
        self.hints
            .iter()
            .find(|(_, l)| *l == label)
            .map(|(i, _)| *i)
    }
}

impl Default for HintGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_char_hints() {
        let mut gen = HintGenerator::with_chars("ABC");
        let hints = gen.generate(3);

        assert_eq!(hints[0].label, "A");
        assert_eq!(hints[1].label, "B");
        assert_eq!(hints[2].label, "C");
    }

    #[test]
    fn test_multi_char_hints() {
        let mut gen = HintGenerator::with_chars("AB");
        let hints = gen.generate(5);

        assert_eq!(hints[0].label, "A");
        assert_eq!(hints[1].label, "B");
        assert_eq!(hints[2].label, "AA");
        assert_eq!(hints[3].label, "AB");
        assert_eq!(hints[4].label, "BA");
    }

    #[test]
    fn test_find_index() {
        let mut gen = HintGenerator::with_chars("ABC");
        gen.generate(10);

        assert_eq!(gen.find_index("A"), Some(0));
        assert_eq!(gen.find_index("B"), Some(1));
        assert_eq!(gen.find_index("C"), Some(2));
        assert_eq!(gen.find_index("AA"), Some(3));
    }
}
