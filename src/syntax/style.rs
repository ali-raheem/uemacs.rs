//! Style types for text rendering
//!
//! This module provides the foundation for styled text rendering,
//! used for both region highlighting and syntax highlighting.

/// Terminal colors (ANSI 16-color palette for compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

/// Text style attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    /// Foreground color
    pub fg: Color,
    /// Background color
    pub bg: Color,
    /// Bold text
    pub bold: bool,
    /// Italic text
    pub italic: bool,
    /// Underlined text
    pub underline: bool,
    /// Reverse video (swap fg/bg)
    pub reverse: bool,
}

impl Style {
    /// Create a style with just foreground color
    pub fn fg(color: Color) -> Self {
        Self {
            fg: color,
            ..Default::default()
        }
    }

    /// Create a style with just background color
    pub fn bg(color: Color) -> Self {
        Self {
            bg: color,
            ..Default::default()
        }
    }

    /// Create a reverse video style (for selections)
    pub fn reverse() -> Self {
        Self {
            reverse: true,
            ..Default::default()
        }
    }

    /// Builder: set foreground color
    pub fn with_fg(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Builder: set background color
    pub fn with_bg(mut self, color: Color) -> Self {
        self.bg = color;
        self
    }

    /// Builder: set bold
    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Builder: set reverse
    pub fn with_reverse(mut self) -> Self {
        self.reverse = true;
        self
    }

    /// Check if this is the default (no styling)
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// A styled span of text within a line
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// Byte offset where this span starts (inclusive)
    pub start: usize,
    /// Byte offset where this span ends (exclusive)
    pub end: usize,
    /// Style to apply to this span
    pub style: Style,
}

impl Span {
    /// Create a new span
    pub fn new(start: usize, end: usize, style: Style) -> Self {
        Self { start, end, style }
    }

    /// Check if this span contains a byte position
    pub fn contains(&self, pos: usize) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Get the length of this span in bytes
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Check if span is empty
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_default() {
        let style = Style::default();
        assert!(style.is_default());
        assert_eq!(style.fg, Color::Default);
        assert_eq!(style.bg, Color::Default);
        assert!(!style.bold);
    }

    #[test]
    fn test_style_builders() {
        let style = Style::fg(Color::Red).with_bold().with_bg(Color::Blue);
        assert_eq!(style.fg, Color::Red);
        assert_eq!(style.bg, Color::Blue);
        assert!(style.bold);
        assert!(!style.is_default());
    }

    #[test]
    fn test_span_contains() {
        let span = Span::new(5, 10, Style::default());
        assert!(!span.contains(4));
        assert!(span.contains(5));
        assert!(span.contains(9));
        assert!(!span.contains(10));
    }
}
