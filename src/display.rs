//! Display rendering

use crate::buffer::Buffer;
use crate::error::Result;
use crate::syntax::Style;
use crate::terminal::Terminal;
use crate::window::Window;

/// Region bounds (normalized so start <= end)
#[derive(Debug, Clone, Copy)]
struct Region {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

impl Region {
    /// Create a region from mark and cursor positions, normalizing so start <= end
    fn from_mark_and_cursor(
        mark_line: usize,
        mark_col: usize,
        cursor_line: usize,
        cursor_col: usize,
    ) -> Self {
        if mark_line < cursor_line || (mark_line == cursor_line && mark_col <= cursor_col) {
            Self {
                start_line: mark_line,
                start_col: mark_col,
                end_line: cursor_line,
                end_col: cursor_col,
            }
        } else {
            Self {
                start_line: cursor_line,
                start_col: cursor_col,
                end_line: mark_line,
                end_col: mark_col,
            }
        }
    }

    /// Get the portion of a line that's in the region (as byte offsets)
    /// Returns None if line is not in region, Some((start, end)) otherwise
    fn line_intersection(&self, line_idx: usize, line_len: usize) -> Option<(usize, usize)> {
        if line_idx < self.start_line || line_idx > self.end_line {
            return None;
        }

        let start = if line_idx == self.start_line {
            self.start_col
        } else {
            0
        };

        let end = if line_idx == self.end_line {
            self.end_col
        } else {
            line_len
        };

        if start < end || (start == end && line_idx < self.end_line) {
            // Include the newline for intermediate lines
            Some((start, end.max(start)))
        } else if start == end && start == 0 && line_idx == self.end_line {
            // Cursor at beginning of end line, don't highlight
            None
        } else {
            Some((start, end.max(start)))
        }
    }
}

/// Display state
pub struct Display {
    /// Whether a full redraw is needed
    needs_redraw: bool,
    /// Message to show in minibuffer (bottom line)
    message: Option<String>,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
}

impl Display {
    pub fn new() -> Self {
        Self {
            needs_redraw: true,
            message: None,
            show_line_numbers: false,
        }
    }

    /// Toggle line numbers on/off
    pub fn toggle_line_numbers(&mut self) {
        self.show_line_numbers = !self.show_line_numbers;
        self.needs_redraw = true;
    }

    /// Calculate width needed for line numbers (including separator)
    fn line_number_width(&self, line_count: usize) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        // Width of largest line number + 1 for separator space
        let digits = if line_count == 0 { 1 } else { (line_count as f64).log10().floor() as usize + 1 };
        digits.max(3) + 1 // minimum 3 digits + space
    }

    /// Mark that a full redraw is needed
    pub fn force_redraw(&mut self) {
        self.needs_redraw = true;
    }

    /// Set a message to display
    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    /// Clear the message
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Render the editor display
    pub fn render(
        &mut self,
        terminal: &mut Terminal,
        windows: &[Window],
        buffers: &[Buffer],
        current_window: usize,
    ) -> Result<()> {
        let cols = terminal.cols() as usize;
        let rows = terminal.rows();

        if self.needs_redraw {
            terminal.clear_screen()?;
        }

        // Render each window
        for (i, window) in windows.iter().enumerate() {
            let is_current = i == current_window;
            self.render_window(terminal, window, buffers, cols, is_current)?;
        }

        // Render minibuffer (message line) at bottom
        self.render_minibuffer(terminal, rows - 1, cols)?;

        // Position cursor in current window
        if let Some(window) = windows.get(current_window) {
            self.position_cursor(terminal, window, buffers)?;
        }

        terminal.set_cursor_visible(true)?;
        terminal.flush()?;

        self.needs_redraw = false;
        Ok(())
    }

    /// Render a single window
    fn render_window(
        &self,
        terminal: &mut Terminal,
        window: &Window,
        buffers: &[Buffer],
        cols: usize,
        is_current: bool,
    ) -> Result<()> {
        let buffer = match buffers.get(window.buffer_idx()) {
            Some(b) => b,
            None => return Ok(()),
        };

        let top_row = window.top_row();
        let height = window.height() as usize;
        let top_line = window.top_line();

        // Calculate line number width
        let lnum_width = self.line_number_width(buffer.line_count());
        let text_cols = cols.saturating_sub(lnum_width);

        // Get region if mark is set (only for current window)
        let region = if is_current {
            window.mark().map(|(mark_line, mark_col)| {
                Region::from_mark_and_cursor(
                    mark_line,
                    mark_col,
                    window.cursor_line(),
                    window.cursor_col(),
                )
            })
        } else {
            None
        };

        // Render each line in the window
        for row_offset in 0..height {
            let screen_row = top_row + row_offset as u16;
            let line_idx = top_line + row_offset;

            terminal.move_cursor(screen_row, 0)?;

            if let Some(line) = buffer.line(line_idx) {
                // Render line number if enabled
                if self.show_line_numbers {
                    let lnum_str = format!("{:>width$} ", line_idx + 1, width = lnum_width - 1);
                    terminal.set_dim(true)?;
                    terminal.write_str(&lnum_str)?;
                    terminal.set_dim(false)?;
                }

                // Render line content with possible region highlighting
                let text = line.text();
                self.render_line_with_region(terminal, text, line_idx, text_cols, &region)?;
            } else {
                // Empty line indicator (like vim's ~)
                if self.show_line_numbers {
                    terminal.write_str(&" ".repeat(lnum_width))?;
                }
                terminal.set_dim(true)?;
                terminal.write_char('~')?;
                terminal.set_dim(false)?;
            }

            terminal.clear_to_eol()?;
        }

        // Render mode line
        let mode_line_row = top_row + height as u16;
        self.render_mode_line(terminal, buffer, window, mode_line_row, cols, is_current)?;

        Ok(())
    }

    /// Render a line with optional region highlighting
    fn render_line_with_region(
        &self,
        terminal: &mut Terminal,
        text: &str,
        line_idx: usize,
        max_cols: usize,
        region: &Option<Region>,
    ) -> Result<()> {
        // Check if this line intersects with the region
        let intersection = region.as_ref().and_then(|r| r.line_intersection(line_idx, text.len()));

        match intersection {
            None => {
                // No region on this line, render normally
                let display_text = truncate_to_width(text, max_cols);
                terminal.write_str(&display_text)?;
            }
            Some((start_byte, end_byte)) => {
                // Line has region, render in three parts: before, selected, after
                let selection_style = Style::reverse();

                // Calculate display widths and positions
                let mut col = 0;
                let mut char_iter = text.char_indices().peekable();
                let mut before_end_col = 0;
                let mut selection_end_col = 0;

                // Find display column positions for byte offsets
                while let Some(&(byte_pos, ch)) = char_iter.peek() {
                    let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);

                    if byte_pos < start_byte {
                        before_end_col = col + ch_width;
                    }
                    if byte_pos < end_byte {
                        selection_end_col = col + ch_width;
                    }

                    col += ch_width;
                    char_iter.next();
                }

                // Handle end-of-string case
                if end_byte >= text.len() {
                    selection_end_col = col;
                }

                // Part 1: Before selection
                if start_byte > 0 {
                    let before = safe_slice_to(text, start_byte);
                    let display_before = truncate_to_width(before, max_cols);
                    terminal.write_str(&display_before)?;
                }

                // Part 2: Selected region (reverse video)
                if start_byte < text.len() && start_byte < end_byte {
                    let selected = safe_slice(text, start_byte, end_byte);
                    let remaining_cols = max_cols.saturating_sub(before_end_col);
                    if remaining_cols > 0 {
                        let display_selected = truncate_to_width(selected, remaining_cols);
                        terminal.apply_style(&selection_style)?;
                        terminal.write_str(&display_selected)?;
                        terminal.reset_attributes()?;
                    }
                }

                // Part 3: After selection
                if end_byte < text.len() {
                    let after = safe_slice_from(text, end_byte);
                    let remaining_cols = max_cols.saturating_sub(selection_end_col);
                    if remaining_cols > 0 {
                        let display_after = truncate_to_width(after, remaining_cols);
                        terminal.write_str(&display_after)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Render the mode line for a window
    fn render_mode_line(
        &self,
        terminal: &mut Terminal,
        buffer: &Buffer,
        window: &Window,
        row: u16,
        cols: usize,
        is_current: bool,
    ) -> Result<()> {
        terminal.move_cursor(row, 0)?;
        terminal.set_reverse(true)?;

        // Build mode line content
        let modified = if buffer.is_modified() { "**" } else { "--" };
        let name = buffer.name();
        let filename = buffer
            .filename()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "".to_string());

        // Calculate position percentage
        let line_count = buffer.line_count();
        let cursor_line = window.cursor_line() + 1;
        let percent = if line_count <= 1 {
            "All".to_string()
        } else if cursor_line == 1 {
            "Top".to_string()
        } else if cursor_line >= line_count {
            "Bot".to_string()
        } else {
            format!("{}%", cursor_line * 100 / line_count)
        };

        // Format: -- uEmacs.rs: buffername (filename) --line-- percent --
        let indicator = if is_current { "=" } else { "-" };
        let mode_line = format!(
            "{}{} uEmacs.rs: {} ({}) L{} {} {}",
            modified,
            indicator,
            name,
            if filename.is_empty() { "no file" } else { &filename },
            cursor_line,
            percent,
            indicator.repeat(10)
        );

        // Pad or truncate to fill width
        let padded = if mode_line.len() < cols {
            format!("{}{}", mode_line, "-".repeat(cols - mode_line.len()))
        } else {
            truncate_to_width(&mode_line, cols)
        };

        terminal.write_str(&padded)?;
        terminal.set_reverse(false)?;

        Ok(())
    }

    /// Render the minibuffer (message area)
    fn render_minibuffer(
        &self,
        terminal: &mut Terminal,
        row: u16,
        cols: usize,
    ) -> Result<()> {
        terminal.move_cursor(row, 0)?;

        if let Some(ref msg) = self.message {
            let truncated = truncate_to_width(msg, cols);
            terminal.write_str(&truncated)?;
        }

        terminal.clear_to_eol()?;
        Ok(())
    }

    /// Position the hardware cursor at the correct location
    fn position_cursor(
        &self,
        terminal: &mut Terminal,
        window: &Window,
        buffers: &[Buffer],
    ) -> Result<()> {
        let buffer = match buffers.get(window.buffer_idx()) {
            Some(b) => b,
            None => return Ok(()),
        };

        let cursor_line = window.cursor_line();
        let cursor_col = window.cursor_col();

        // Convert byte offset to display column
        let display_col = if let Some(line) = buffer.line(cursor_line) {
            line.byte_to_col(cursor_col)
        } else {
            0
        };

        // Account for line number width
        let lnum_width = self.line_number_width(buffer.line_count());

        // Calculate screen position
        let screen_row = if cursor_line >= window.top_line() {
            window.top_row() + (cursor_line - window.top_line()) as u16
        } else {
            window.top_row()
        };

        let screen_col = (lnum_width + display_col).min(terminal.cols() as usize - 1) as u16;

        terminal.move_cursor(screen_row, screen_col)?;
        Ok(())
    }
}

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
}

/// Truncate a string to fit within a display width
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;

    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if width + ch_width > max_width {
            break;
        }
        result.push(ch);
        width += ch_width;
    }

    result
}

/// UTF-8 safe slice from start to end byte offset
fn safe_slice(s: &str, start: usize, end: usize) -> &str {
    let start = find_safe_boundary(s, start, true);
    let end = find_safe_boundary(s, end, false);
    &s[start..end]
}

/// UTF-8 safe slice from start to end of string
fn safe_slice_from(s: &str, start: usize) -> &str {
    let start = find_safe_boundary(s, start, true);
    &s[start..]
}

/// UTF-8 safe slice from beginning to end byte offset
fn safe_slice_to(s: &str, end: usize) -> &str {
    let end = find_safe_boundary(s, end, false);
    &s[..end]
}

/// Find a safe UTF-8 boundary near the given byte offset
/// If forward is true, search forward; otherwise search backward
fn find_safe_boundary(s: &str, offset: usize, forward: bool) -> usize {
    if offset >= s.len() {
        return s.len();
    }
    if offset == 0 {
        return 0;
    }

    if s.is_char_boundary(offset) {
        return offset;
    }

    if forward {
        // Search forward for next boundary
        for i in offset..=s.len() {
            if s.is_char_boundary(i) {
                return i;
            }
        }
        s.len()
    } else {
        // Search backward for previous boundary
        for i in (0..offset).rev() {
            if s.is_char_boundary(i) {
                return i;
            }
        }
        0
    }
}
