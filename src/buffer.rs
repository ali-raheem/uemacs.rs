//! Buffer representation - a collection of lines with associated metadata

use std::path::PathBuf;

use crate::line::Line;

/// Buffer flags/modes (matching original C version)
#[derive(Debug, Clone, Copy, Default)]
pub struct BufferModes {
    pub wrap: bool,      // Word wrap mode
    pub c_mode: bool,    // C indentation mode
    pub exact: bool,     // Exact case matching for search
    pub view: bool,      // Read-only view mode
    pub overwrite: bool, // Overwrite mode
}

/// A buffer containing text and metadata
#[derive(Debug)]
pub struct Buffer {
    /// Lines of text
    lines: Vec<Line>,
    /// Buffer name (e.g., "main", "*scratch*")
    name: String,
    /// Associated file path (None for unnamed buffers)
    filename: Option<PathBuf>,
    /// Whether buffer has unsaved changes
    modified: bool,
    /// Buffer modes
    modes: BufferModes,
}

impl Buffer {
    /// Create a new empty buffer with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            lines: vec![Line::new()], // Always have at least one line
            name: name.into(),
            filename: None,
            modified: false,
            modes: BufferModes::default(),
        }
    }

    /// Create a buffer from file contents
    pub fn from_file(path: &PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed".to_string());

        let lines: Vec<Line> = if content.is_empty() {
            vec![Line::new()]
        } else {
            content.lines().map(Line::from).collect()
        };

        // Ensure at least one line
        let lines = if lines.is_empty() {
            vec![Line::new()]
        } else {
            lines
        };

        Ok(Self {
            lines,
            name,
            filename: Some(path.clone()),
            modified: false,
            modes: BufferModes::default(),
        })
    }

    /// Get buffer name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get filename if set
    pub fn filename(&self) -> Option<&PathBuf> {
        self.filename.as_ref()
    }

    /// Set the filename
    pub fn set_filename(&mut self, path: PathBuf) {
        self.filename = Some(path);
    }

    /// Check if buffer is modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark buffer as modified
    pub fn set_modified(&mut self, modified: bool) {
        self.modified = modified;
    }

    /// Get buffer modes
    pub fn modes(&self) -> &BufferModes {
        &self.modes
    }

    /// Get mutable buffer modes
    pub fn modes_mut(&mut self) -> &mut BufferModes {
        &mut self.modes
    }

    /// Get number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get a line by index
    pub fn line(&self, idx: usize) -> Option<&Line> {
        self.lines.get(idx)
    }

    /// Get a mutable line by index
    pub fn line_mut(&mut self, idx: usize) -> Option<&mut Line> {
        self.lines.get_mut(idx)
    }

    /// Get all lines
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Insert a character at position
    pub fn insert_char(&mut self, line_idx: usize, byte_pos: usize, ch: char) {
        if let Some(line) = self.lines.get_mut(line_idx) {
            line.insert_char(byte_pos, ch);
            self.modified = true;
        }
    }

    /// Insert a newline, splitting the current line
    pub fn insert_newline(&mut self, line_idx: usize, byte_pos: usize) {
        if let Some(line) = self.lines.get_mut(line_idx) {
            let new_line = line.split_off(byte_pos);
            self.lines.insert(line_idx + 1, new_line);
            self.modified = true;
        }
    }

    /// Delete a character at position, returns the deleted char
    pub fn delete_char(&mut self, line_idx: usize, byte_pos: usize) -> Option<char> {
        if let Some(line) = self.lines.get_mut(line_idx) {
            let text = line.text();
            if byte_pos < text.len() {
                let ch = text[byte_pos..].chars().next()?;
                let ch_len = ch.len_utf8();
                line.delete_range(byte_pos, byte_pos + ch_len);
                self.modified = true;
                return Some(ch);
            }
        }
        None
    }

    /// Delete backward (backspace), returns deleted char
    pub fn delete_backward(&mut self, line_idx: usize, byte_pos: usize) -> Option<(char, usize)> {
        if byte_pos > 0 {
            if let Some(line) = self.lines.get_mut(line_idx) {
                // Find the char before byte_pos
                let text = line.text();
                let before = &text[..byte_pos];
                if let Some(ch) = before.chars().last() {
                    let ch_len = ch.len_utf8();
                    let new_pos = byte_pos - ch_len;
                    line.delete_range(new_pos, byte_pos);
                    self.modified = true;
                    return Some((ch, new_pos));
                }
            }
        }
        None
    }

    /// Join line with the next line (delete newline at end of line)
    pub fn join_line(&mut self, line_idx: usize) -> bool {
        if line_idx + 1 < self.lines.len() {
            let next_line = self.lines.remove(line_idx + 1);
            if let Some(line) = self.lines.get_mut(line_idx) {
                line.append(next_line);
                self.modified = true;
                return true;
            }
        }
        false
    }

    /// Join with previous line (when backspacing at start of line)
    pub fn join_with_previous(&mut self, line_idx: usize) -> Option<usize> {
        if line_idx > 0 {
            let current_line = self.lines.remove(line_idx);
            if let Some(prev_line) = self.lines.get_mut(line_idx - 1) {
                let join_pos = prev_line.len();
                prev_line.append(current_line);
                self.modified = true;
                return Some(join_pos);
            }
        }
        None
    }

    /// Delete a line by index
    pub fn delete_line(&mut self, line_idx: usize) {
        if line_idx < self.lines.len() && self.lines.len() > 1 {
            self.lines.remove(line_idx);
            self.modified = true;
        }
    }

    /// Kill from position to end of line, returns killed text
    pub fn kill_to_eol(&mut self, line_idx: usize, byte_pos: usize) -> Option<String> {
        if let Some(line) = self.lines.get_mut(line_idx) {
            let line_len = line.len();
            if byte_pos < line_len {
                let killed = line.delete_range(byte_pos, line_len);
                self.modified = true;
                return Some(killed);
            } else if line_idx + 1 < self.lines.len() {
                // At end of line, kill the newline (join with next)
                self.join_line(line_idx);
                return Some("\n".to_string());
            }
        }
        None
    }

    /// Write buffer to file
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = &self.filename {
            self.write_to(path)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No filename set",
            ))
        }
    }

    /// Write buffer to specific path
    pub fn write_to(&self, path: &PathBuf) -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        for (i, line) in self.lines.iter().enumerate() {
            write!(file, "{}", line.text())?;
            if i < self.lines.len() - 1 {
                writeln!(file)?;
            }
        }
        Ok(())
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new("*scratch*")
    }
}
