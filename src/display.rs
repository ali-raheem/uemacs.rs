//! Display rendering

use crate::buffer::Buffer;
use crate::error::Result;
use crate::terminal::Terminal;
use crate::window::Window;

/// Display state
pub struct Display {
    /// Whether a full redraw is needed
    needs_redraw: bool,
    /// Message to show in minibuffer (bottom line)
    message: Option<String>,
}

impl Display {
    pub fn new() -> Self {
        Self {
            needs_redraw: true,
            message: None,
        }
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

        // Render each line in the window
        for row_offset in 0..height {
            let screen_row = top_row + row_offset as u16;
            let line_idx = top_line + row_offset;

            terminal.move_cursor(screen_row, 0)?;

            if let Some(line) = buffer.line(line_idx) {
                // Render line content, truncated to screen width
                let text = line.text();
                let display_text = truncate_to_width(text, cols);
                terminal.write_str(&display_text)?;
            } else {
                // Empty line indicator (like vim's ~)
                terminal.write_char('~')?;
            }

            terminal.clear_to_eol()?;
        }

        // Render mode line
        let mode_line_row = top_row + height as u16;
        self.render_mode_line(terminal, buffer, window, mode_line_row, cols, is_current)?;

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

        // Calculate screen position
        let screen_row = if cursor_line >= window.top_line() {
            window.top_row() + (cursor_line - window.top_line()) as u16
        } else {
            window.top_row()
        };

        let screen_col = display_col.min(terminal.cols() as usize - 1) as u16;

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
