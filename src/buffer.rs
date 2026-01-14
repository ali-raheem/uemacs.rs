//! Buffer representation - a collection of lines with associated metadata

use std::path::PathBuf;

use crate::line::Line;

/// An entry in the undo stack
#[derive(Debug, Clone)]
pub enum UndoEntry {
    /// Text was inserted at (line, col) - to undo, delete it
    Insert {
        line: usize,
        col: usize,
        text: String,
    },
    /// Text was deleted from (line, col) - to undo, insert it back
    Delete {
        line: usize,
        col: usize,
        text: String,
    },
    /// A newline was inserted, splitting line at col - to undo, join lines
    InsertNewline {
        line: usize,
        col: usize,
    },
    /// Lines were joined (newline deleted) - to undo, split them
    DeleteNewline {
        line: usize,
        col: usize,  // Where the join happened
    },
    /// Boundary marker for grouping multiple operations
    Boundary,
}

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
    /// Undo stack
    undo_stack: Vec<UndoEntry>,
    /// Whether to record undo entries (disabled during undo itself)
    recording_undo: bool,
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
            undo_stack: Vec::new(),
            recording_undo: true,
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
            undo_stack: Vec::new(),
            recording_undo: true,
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
            self.push_undo(UndoEntry::Insert {
                line: line_idx,
                col: byte_pos,
                text: ch.to_string(),
            });
        }
    }

    /// Insert a newline, splitting the current line
    pub fn insert_newline(&mut self, line_idx: usize, byte_pos: usize) {
        if let Some(line) = self.lines.get_mut(line_idx) {
            let new_line = line.split_off(byte_pos);
            self.lines.insert(line_idx + 1, new_line);
            self.modified = true;
            self.push_undo(UndoEntry::InsertNewline {
                line: line_idx,
                col: byte_pos,
            });
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
                self.push_undo(UndoEntry::Delete {
                    line: line_idx,
                    col: byte_pos,
                    text: ch.to_string(),
                });
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
                    self.push_undo(UndoEntry::Delete {
                        line: line_idx,
                        col: new_pos,
                        text: ch.to_string(),
                    });
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
                let join_col = line.len();
                line.append(next_line);
                self.modified = true;
                self.push_undo(UndoEntry::DeleteNewline {
                    line: line_idx,
                    col: join_col,
                });
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
                self.push_undo(UndoEntry::DeleteNewline {
                    line: line_idx - 1,
                    col: join_pos,
                });
                return Some(join_pos);
            }
        }
        None
    }

    /// Delete a line by index
    pub fn delete_line(&mut self, line_idx: usize) {
        if line_idx < self.lines.len() && self.lines.len() > 1 {
            let removed = self.lines.remove(line_idx);
            self.modified = true;
            // Record deletion of line content
            if !removed.is_empty() {
                self.push_undo(UndoEntry::Delete {
                    line: line_idx,
                    col: 0,
                    text: removed.text().to_string(),
                });
            }
            // Record deletion of the newline that joined this with next/prev
            if line_idx > 0 {
                // Line was joined with previous
                self.push_undo(UndoEntry::DeleteNewline {
                    line: line_idx - 1,
                    col: self.lines.get(line_idx - 1).map(|l| l.len()).unwrap_or(0),
                });
            }
        }
    }

    /// Kill from position to end of line, returns killed text
    pub fn kill_to_eol(&mut self, line_idx: usize, byte_pos: usize) -> Option<String> {
        if let Some(line) = self.lines.get_mut(line_idx) {
            let line_len = line.len();
            if byte_pos < line_len {
                let killed = line.delete_range(byte_pos, line_len);
                self.modified = true;
                self.push_undo(UndoEntry::Delete {
                    line: line_idx,
                    col: byte_pos,
                    text: killed.clone(),
                });
                return Some(killed);
            } else if line_idx + 1 < self.lines.len() {
                // At end of line, kill the newline (join with next)
                // join_line will record its own undo entry
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

    /// Push an undo entry (if recording is enabled)
    fn push_undo(&mut self, entry: UndoEntry) {
        if self.recording_undo {
            self.undo_stack.push(entry);
        }
    }

    /// Add a boundary marker to group operations
    pub fn add_undo_boundary(&mut self) {
        if self.recording_undo && !self.undo_stack.is_empty() {
            // Only add if last entry isn't already a boundary
            if !matches!(self.undo_stack.last(), Some(UndoEntry::Boundary)) {
                self.undo_stack.push(UndoEntry::Boundary);
            }
        }
    }

    /// Perform undo, returns (line, col) to move cursor to, or None if nothing to undo
    pub fn undo(&mut self) -> Option<(usize, usize)> {
        // Disable recording while undoing
        self.recording_undo = false;

        // Skip any boundary at the top
        while matches!(self.undo_stack.last(), Some(UndoEntry::Boundary)) {
            self.undo_stack.pop();
        }

        let mut cursor_pos = None;

        // Undo entries until we hit a boundary or empty
        while let Some(entry) = self.undo_stack.pop() {
            match entry {
                UndoEntry::Boundary => break,
                UndoEntry::Insert { line, col, text } => {
                    // Text was inserted, so delete it
                    if let Some(line_ref) = self.lines.get_mut(line) {
                        let end = col + text.len();
                        if end <= line_ref.len() {
                            line_ref.delete_range(col, end);
                        }
                    }
                    cursor_pos = Some((line, col));
                }
                UndoEntry::Delete { line, col, text } => {
                    // Text was deleted, so insert it back
                    if let Some(line_ref) = self.lines.get_mut(line) {
                        let text_to_insert = text.clone();
                        for (i, ch) in text_to_insert.chars().enumerate() {
                            line_ref.insert_char(col + i, ch);
                        }
                    }
                    cursor_pos = Some((line, col + text.len()));
                }
                UndoEntry::InsertNewline { line, col } => {
                    // Newline was inserted, so join the lines
                    if line + 1 < self.lines.len() {
                        let next_line = self.lines.remove(line + 1);
                        if let Some(current) = self.lines.get_mut(line) {
                            current.append(next_line);
                        }
                    }
                    cursor_pos = Some((line, col));
                }
                UndoEntry::DeleteNewline { line, col } => {
                    // Newline was deleted (lines joined), so split them
                    if let Some(line_ref) = self.lines.get_mut(line) {
                        let new_line = line_ref.split_off(col);
                        self.lines.insert(line + 1, new_line);
                    }
                    cursor_pos = Some((line, col));
                }
            }
        }

        self.recording_undo = true;

        if cursor_pos.is_some() {
            self.modified = true;
        }

        cursor_pos
    }

    /// Check if there's anything to undo
    pub fn can_undo(&self) -> bool {
        self.undo_stack.iter().any(|e| !matches!(e, UndoEntry::Boundary))
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new("*scratch*")
    }
}
