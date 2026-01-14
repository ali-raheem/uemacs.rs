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
