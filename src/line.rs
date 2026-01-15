//! Line representation and text operations

use unicode_width::UnicodeWidthStr;

/// A single line of text in a buffer
#[derive(Debug, Clone)]
pub struct Line {
    /// The text content (without trailing newline)
    text: String,
}

impl Line {
    /// Create a new empty line
    pub fn new() -> Self {
        Self {
            text: String::new(),
        }
    }

    /// Create a line from a string
    pub fn from_string(s: String) -> Self {
        Self { text: s }
    }

    /// Get the text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get mutable access to the text
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if the line is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the display width of the line
    pub fn display_width(&self) -> usize {
        self.text.width()
    }

    /// Insert a character at byte position
    pub fn insert_char(&mut self, byte_pos: usize, ch: char) {
        self.text.insert(byte_pos, ch);
    }

    /// Insert a string at byte position
    pub fn insert_str(&mut self, byte_pos: usize, s: &str) {
        self.text.insert_str(byte_pos, s);
    }

    /// Delete a range of bytes and return the deleted text
    pub fn delete_range(&mut self, start: usize, end: usize) -> String {
        let deleted: String = self.text[start..end].to_string();
        self.text.replace_range(start..end, "");
        deleted
    }

    /// Split the line at byte position, returning the remainder
    pub fn split_off(&mut self, byte_pos: usize) -> Line {
        let remainder = self.text.split_off(byte_pos);
        Line::from_string(remainder)
    }

    /// Append another line's content to this line
    pub fn append(&mut self, other: Line) {
        self.text.push_str(other.text());
    }

    /// Append a string to this line
    pub fn append_str(&mut self, s: &str) {
        self.text.push_str(s);
    }

    /// Clear the line content
    pub fn clear(&mut self) {
        self.text.clear();
    }

    /// Get byte position for a given column (display position)
    /// Returns None if column is beyond line end
    pub fn col_to_byte(&self, col: usize) -> Option<usize> {
        let mut current_col = 0;
        for (byte_idx, ch) in self.text.char_indices() {
            if current_col >= col {
                return Some(byte_idx);
            }
            current_col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        }
        if current_col >= col {
            Some(self.text.len())
        } else {
            None
        }
    }

    /// Get column (display position) for a given byte position
    pub fn byte_to_col(&self, byte_pos: usize) -> usize {
        self.text[..byte_pos.min(self.text.len())].width()
    }

    /// Find the nearest valid UTF-8 char boundary at or before `pos`
    fn floor_char_boundary(&self, pos: usize) -> usize {
        if pos >= self.text.len() {
            return self.text.len();
        }
        // Walk backwards to find valid UTF-8 start byte
        let mut p = pos;
        while p > 0 && !self.text.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    /// Find the nearest valid UTF-8 char boundary at or after `pos`
    fn ceil_char_boundary(&self, pos: usize) -> usize {
        if pos >= self.text.len() {
            return self.text.len();
        }
        // Walk forwards to find valid UTF-8 start byte
        let mut p = pos;
        while p < self.text.len() && !self.text.is_char_boundary(p) {
            p += 1;
        }
        p
    }

    /// Safely slice the line text, adjusting to valid UTF-8 boundaries
    /// Returns empty string if range is invalid
    pub fn safe_slice(&self, start: usize, end: usize) -> &str {
        if start >= self.text.len() {
            return "";
        }
        let start = self.floor_char_boundary(start);
        let end = self.ceil_char_boundary(end.min(self.text.len()));
        if start >= end {
            return "";
        }
        &self.text[start..end]
    }

    /// Safely slice from start to a position
    pub fn safe_slice_to(&self, end: usize) -> &str {
        self.safe_slice(0, end)
    }

    /// Safely slice from a position to end
    pub fn safe_slice_from(&self, start: usize) -> &str {
        self.safe_slice(start, self.text.len())
    }

    /// Get the byte position of the character at a given character index
    /// Returns None if index is out of bounds
    pub fn char_to_byte(&self, char_idx: usize) -> Option<usize> {
        self.text.char_indices().nth(char_idx).map(|(pos, _)| pos)
    }

    /// Get the character index for a given byte position
    pub fn byte_to_char(&self, byte_pos: usize) -> usize {
        self.text[..byte_pos.min(self.text.len())].chars().count()
    }
}

impl Default for Line {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for Line {
    fn from(s: &str) -> Self {
        Self::from_string(s.to_string())
    }
}

impl From<String> for Line {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_line_operations() {
        let mut line = Line::from("Hello");
        assert_eq!(line.text(), "Hello");
        assert_eq!(line.len(), 5);
        assert!(!line.is_empty());

        line.insert_char(5, '!');
        assert_eq!(line.text(), "Hello!");
    }

    #[test]
    fn test_utf8_emoji() {
        let line = Line::from("Hello 😀 World");
        // "Hello " (6) + emoji (4) + " World" (6) = 16 bytes
        assert_eq!(line.len(), 16);

        // safe_slice should handle emoji correctly
        assert_eq!(line.safe_slice(0, 6), "Hello ");
        assert_eq!(line.safe_slice(6, 10), "😀");
        assert_eq!(line.safe_slice(10, 16), " World");
    }

    #[test]
    fn test_utf8_chinese() {
        let line = Line::from("你好世界"); // 4 Chinese chars, 3 bytes each = 12 bytes
        assert_eq!(line.len(), 12);

        // Each character is 3 bytes
        assert_eq!(line.safe_slice(0, 3), "你");
        assert_eq!(line.safe_slice(3, 6), "好");
        assert_eq!(line.safe_slice(0, 6), "你好");
    }

    #[test]
    fn test_safe_slice_boundary_handling() {
        let line = Line::from("café"); // 'é' is 2 bytes
        assert_eq!(line.len(), 5); // c(1) + a(1) + f(1) + é(2) = 5

        // Slicing in middle of 'é' should adjust to valid boundaries
        // Position 4 is middle of 'é', should round up to include it
        let slice = line.safe_slice(3, 4);
        // Should get 'é' since it adjusts boundaries
        assert!(!slice.is_empty() || slice == "é");
    }

    #[test]
    fn test_safe_slice_from() {
        let line = Line::from("Hello World");
        assert_eq!(line.safe_slice_from(6), "World");
        assert_eq!(line.safe_slice_from(0), "Hello World");
        assert_eq!(line.safe_slice_from(100), ""); // Beyond end
    }

    #[test]
    fn test_safe_slice_to() {
        let line = Line::from("Hello World");
        assert_eq!(line.safe_slice_to(5), "Hello");
        assert_eq!(line.safe_slice_to(11), "Hello World");
        assert_eq!(line.safe_slice_to(0), "");
    }

    #[test]
    fn test_empty_line() {
        let line = Line::new();
        assert!(line.is_empty());
        assert_eq!(line.len(), 0);
        assert_eq!(line.safe_slice(0, 10), "");
    }

    #[test]
    fn test_delete_range() {
        let mut line = Line::from("Hello World");
        let deleted = line.delete_range(0, 6);
        assert_eq!(deleted, "Hello ");
        assert_eq!(line.text(), "World");
    }

    #[test]
    fn test_split_off() {
        let mut line = Line::from("Hello World");
        let remainder = line.split_off(6);
        assert_eq!(line.text(), "Hello ");
        assert_eq!(remainder.text(), "World");
    }

    #[test]
    fn test_append() {
        let mut line = Line::from("Hello ");
        let other = Line::from("World");
        line.append(other);
        assert_eq!(line.text(), "Hello World");
    }
}
