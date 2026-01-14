//! Window representation - a viewport into a buffer

/// A window displaying a portion of a buffer
#[derive(Debug)]
pub struct Window {
    /// Index of the buffer being displayed
    buffer_idx: usize,
    /// First visible line in the window
    top_line: usize,
    /// Cursor line position (0-indexed)
    cursor_line: usize,
    /// Cursor column position (byte offset within line)
    cursor_col: usize,
    /// Mark line position (for region operations)
    mark_line: Option<usize>,
    /// Mark column position
    mark_col: Option<usize>,
    /// Row on screen where window starts
    top_row: u16,
    /// Number of text rows in window (excluding mode line)
    height: u16,
    /// Goal column for vertical movement
    goal_col: usize,
}

impl Window {
    /// Create a new window for a buffer
    pub fn new(buffer_idx: usize, top_row: u16, height: u16) -> Self {
        Self {
            buffer_idx,
            top_line: 0,
            cursor_line: 0,
            cursor_col: 0,
            mark_line: None,
            mark_col: None,
            top_row,
            height,
            goal_col: 0,
        }
    }

    /// Get the buffer index
    pub fn buffer_idx(&self) -> usize {
        self.buffer_idx
    }

    /// Set the buffer index
    pub fn set_buffer_idx(&mut self, idx: usize) {
        self.buffer_idx = idx;
        self.top_line = 0;
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.mark_line = None;
        self.mark_col = None;
    }

    /// Get the top visible line
    pub fn top_line(&self) -> usize {
        self.top_line
    }

    /// Set the top visible line
    pub fn set_top_line(&mut self, line: usize) {
        self.top_line = line;
    }

    /// Get cursor line
    pub fn cursor_line(&self) -> usize {
        self.cursor_line
    }

    /// Get cursor column (byte offset)
    pub fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Set cursor position
    pub fn set_cursor(&mut self, line: usize, col: usize) {
        self.cursor_line = line;
        self.cursor_col = col;
    }

    /// Get the goal column for vertical movement
    pub fn goal_col(&self) -> usize {
        self.goal_col
    }

    /// Set the goal column
    pub fn set_goal_col(&mut self, col: usize) {
        self.goal_col = col;
    }

    /// Get mark position
    pub fn mark(&self) -> Option<(usize, usize)> {
        match (self.mark_line, self.mark_col) {
            (Some(line), Some(col)) => Some((line, col)),
            _ => None,
        }
    }

    /// Set mark at current cursor position
    pub fn set_mark(&mut self) {
        self.mark_line = Some(self.cursor_line);
        self.mark_col = Some(self.cursor_col);
    }

    /// Clear the mark
    pub fn clear_mark(&mut self) {
        self.mark_line = None;
        self.mark_col = None;
    }

    /// Get top row on screen
    pub fn top_row(&self) -> u16 {
        self.top_row
    }

    /// Set top row on screen
    pub fn set_top_row(&mut self, row: u16) {
        self.top_row = row;
    }

    /// Get window height in rows
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Set window height
    pub fn set_height(&mut self, height: u16) {
        self.height = height;
    }

    /// Check if a line is visible in the window
    pub fn is_line_visible(&self, line: usize) -> bool {
        line >= self.top_line && line < self.top_line + self.height as usize
    }

    /// Ensure cursor is visible, adjusting top_line if needed
    pub fn ensure_cursor_visible(&mut self) {
        if self.cursor_line < self.top_line {
            self.top_line = self.cursor_line;
        } else if self.cursor_line >= self.top_line + self.height as usize {
            self.top_line = self.cursor_line - self.height as usize + 1;
        }
    }

    /// Get the screen row for a buffer line (None if not visible)
    pub fn line_to_screen_row(&self, line: usize) -> Option<u16> {
        if self.is_line_visible(line) {
            Some(self.top_row + (line - self.top_line) as u16)
        } else {
            None
        }
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize, max_line: usize) {
        let new_top = self.top_line.saturating_add(n);
        self.top_line = new_top.min(max_line.saturating_sub(1));
        // Move cursor to stay in view
        if self.cursor_line < self.top_line {
            self.cursor_line = self.top_line;
        }
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.top_line = self.top_line.saturating_sub(n);
        // Move cursor to stay in view
        let bottom = self.top_line + self.height as usize - 1;
        if self.cursor_line > bottom {
            self.cursor_line = bottom;
        }
    }
}
