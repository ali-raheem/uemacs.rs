//! Editor state and main loop

use std::path::PathBuf;

use crate::buffer::Buffer;
use crate::command::{CommandStatus, KeyTable};
use crate::display::Display;
use crate::error::Result;
use crate::input::{InputState, Key};
use crate::terminal::Terminal;
use crate::window::Window;

/// Main editor state - consolidates all globals from C version
pub struct EditorState {
    /// All open buffers
    pub buffers: Vec<Buffer>,
    /// All windows
    pub windows: Vec<Window>,
    /// Index of current window
    pub current_window: usize,
    /// Terminal interface
    pub terminal: Terminal,
    /// Display state
    pub display: Display,
    /// Input state
    pub input: InputState,
    /// Key bindings
    pub keytab: KeyTable,
    /// Whether editor is running
    pub running: bool,
    /// Kill ring (clipboard)
    pub kill_ring: Vec<String>,
    /// Current position in kill ring
    pub kill_ring_idx: usize,
    /// Track consecutive kills for appending
    pub last_was_kill: bool,
    /// Waiting for literal character (C-q)
    pub quote_pending: bool,
    /// Incremental search state
    pub search: SearchState,
}

/// Search direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Incremental search state
#[derive(Debug, Clone)]
pub struct SearchState {
    /// Whether we're in search mode
    pub active: bool,
    /// Current search pattern
    pub pattern: String,
    /// Search direction
    pub direction: SearchDirection,
    /// Original cursor position (to restore on abort)
    pub origin_line: usize,
    pub origin_col: usize,
    /// Last successful match position
    pub last_match_line: Option<usize>,
    pub last_match_col: Option<usize>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            active: false,
            pattern: String::new(),
            direction: SearchDirection::Forward,
            origin_line: 0,
            origin_col: 0,
            last_match_line: None,
            last_match_col: None,
        }
    }
}

impl EditorState {
    /// Create a new editor state
    pub fn new(terminal: Terminal) -> Self {
        // Create initial scratch buffer
        let buffer = Buffer::new("*scratch*");

        // Calculate window size (leave 1 row for mode line, 1 for minibuffer)
        let height = terminal.rows().saturating_sub(2);

        // Create initial window
        let window = Window::new(0, 0, height);

        Self {
            buffers: vec![buffer],
            windows: vec![window],
            current_window: 0,
            terminal,
            display: Display::new(),
            input: InputState::new(),
            keytab: KeyTable::with_defaults(),
            running: true,
            kill_ring: Vec::new(),
            kill_ring_idx: 0,
            last_was_kill: false,
            quote_pending: false,
            search: SearchState::default(),
        }
    }

    /// Open a file in a new buffer
    pub fn open_file(&mut self, path: &PathBuf) -> Result<()> {
        let buffer = Buffer::from_file(path)?;
        self.buffers.push(buffer);
        let buf_idx = self.buffers.len() - 1;

        // Set current window to show the new buffer
        if let Some(window) = self.windows.get_mut(self.current_window) {
            window.set_buffer_idx(buf_idx);
        }

        self.display.force_redraw();
        Ok(())
    }

    /// Get current window
    pub fn current_window(&self) -> &Window {
        &self.windows[self.current_window]
    }

    /// Get current window mutably
    pub fn current_window_mut(&mut self) -> &mut Window {
        &mut self.windows[self.current_window]
    }

    /// Get current buffer
    pub fn current_buffer(&self) -> &Buffer {
        let buf_idx = self.windows[self.current_window].buffer_idx();
        &self.buffers[buf_idx]
    }

    /// Get current buffer mutably
    pub fn current_buffer_mut(&mut self) -> &mut Buffer {
        let buf_idx = self.windows[self.current_window].buffer_idx();
        &mut self.buffers[buf_idx]
    }

    /// Run the main editor loop
    pub fn run(&mut self) -> Result<()> {
        self.display.force_redraw();

        while self.running {
            // Render display
            self.display.render(
                &mut self.terminal,
                &self.windows,
                &self.buffers,
                self.current_window,
            )?;

            // Read and handle input
            let key_event = self.terminal.read_key()?;

            // Translate key event
            if let Some(key) = self.input.translate_key(key_event) {
                self.handle_key(key)?;
            }
            // If None, we're waiting for continuation (C-x or ESC sequence)
        }

        Ok(())
    }

    /// Handle a key press
    fn handle_key(&mut self, key: Key) -> Result<()> {
        // Handle search mode
        if self.search.active {
            return self.handle_search_key(key);
        }

        // Clear any previous message
        self.display.clear_message();

        // Handle quote mode - insert next character literally
        if self.quote_pending {
            self.quote_pending = false;
            if let Some(ch) = key.base_char() {
                if ch == '\r' || ch == '\n' {
                    // Insert newline
                    let cursor_line = self.current_window().cursor_line();
                    let cursor_col = self.current_window().cursor_col();
                    self.current_buffer_mut()
                        .insert_newline(cursor_line, cursor_col);
                    self.current_window_mut().set_cursor(cursor_line + 1, 0);
                } else {
                    self.insert_char(ch);
                }
            }
            return Ok(());
        }

        // Look up command
        if let Some(cmd) = self.keytab.lookup(key) {
            // Execute command with no numeric argument
            match cmd(self, false, 1)? {
                CommandStatus::Success => {}
                CommandStatus::Failure => {
                    self.terminal.beep()?;
                }
                CommandStatus::Abort => {
                    self.display.set_message("Quit");
                    self.terminal.beep()?;
                }
            }
        } else if key.is_self_insert() {
            // Self-insert character
            if let Some(ch) = key.base_char() {
                self.insert_char(ch);
            }
        } else {
            // Unknown key
            self.terminal.beep()?;
            self.display.set_message("Key not bound");
        }

        Ok(())
    }

    /// Insert a character at cursor
    pub fn insert_char(&mut self, ch: char) {
        let cursor_line = self.current_window().cursor_line();
        let cursor_col = self.current_window().cursor_col();

        self.current_buffer_mut()
            .insert_char(cursor_line, cursor_col, ch);

        // Move cursor forward
        let new_col = cursor_col + ch.len_utf8();
        self.current_window_mut().set_cursor(cursor_line, new_col);
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        let cursor_col = self.current_window().cursor_col();

        if let Some(line) = self.current_buffer().line(cursor_line) {
            let line_len = line.len();
            if cursor_col < line_len {
                // Move to next character
                let text = line.text();
                if let Some(ch) = text[cursor_col..].chars().next() {
                    let new_col = cursor_col + ch.len_utf8();
                    self.current_window_mut().set_cursor(cursor_line, new_col);
                }
            } else if cursor_line + 1 < self.current_buffer().line_count() {
                // Move to beginning of next line
                self.current_window_mut().set_cursor(cursor_line + 1, 0);
            }
        }

        self.ensure_cursor_visible();
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        let cursor_col = self.current_window().cursor_col();

        if cursor_col > 0 {
            // Move to previous character
            if let Some(line) = self.current_buffer().line(cursor_line) {
                let text = line.text();
                let before = &text[..cursor_col];
                if let Some(ch) = before.chars().last() {
                    let new_col = cursor_col - ch.len_utf8();
                    self.current_window_mut().set_cursor(cursor_line, new_col);
                }
            }
        } else if cursor_line > 0 {
            // Move to end of previous line
            let prev_line_len = self
                .current_buffer()
                .line(cursor_line - 1)
                .map(|l| l.len())
                .unwrap_or(0);
            self.current_window_mut()
                .set_cursor(cursor_line - 1, prev_line_len);
        }

        self.ensure_cursor_visible();
    }

    /// Move cursor down
    pub fn move_cursor_down(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        let goal_col = self.current_window().goal_col();

        if cursor_line + 1 < self.current_buffer().line_count() {
            // Move to next line, trying to maintain column
            let new_col = self.col_to_byte_in_line(cursor_line + 1, goal_col);
            self.current_window_mut().set_cursor(cursor_line + 1, new_col);
        }

        self.ensure_cursor_visible();
    }

    /// Move cursor up
    pub fn move_cursor_up(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        let goal_col = self.current_window().goal_col();

        if cursor_line > 0 {
            // Move to previous line, trying to maintain column
            let new_col = self.col_to_byte_in_line(cursor_line - 1, goal_col);
            self.current_window_mut().set_cursor(cursor_line - 1, new_col);
        }

        self.ensure_cursor_visible();
    }

    /// Move to beginning of line
    pub fn move_to_bol(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        self.current_window_mut().set_cursor(cursor_line, 0);
        self.current_window_mut().set_goal_col(0);
    }

    /// Move to end of line
    pub fn move_to_eol(&mut self) {
        let cursor_line = self.current_window().cursor_line();
        let line_len = self
            .current_buffer()
            .line(cursor_line)
            .map(|l| l.len())
            .unwrap_or(0);
        self.current_window_mut().set_cursor(cursor_line, line_len);

        // Update goal column
        let display_col = self
            .current_buffer()
            .line(cursor_line)
            .map(|l| l.display_width())
            .unwrap_or(0);
        self.current_window_mut().set_goal_col(display_col);
    }

    /// Page down
    pub fn page_down(&mut self) {
        let height = self.current_window().height() as usize;
        let max_line = self.current_buffer().line_count();
        let window = self.current_window_mut();

        window.scroll_down(height.saturating_sub(2), max_line);

        // Move cursor to top of new view
        let new_cursor = window.top_line();
        let goal_col = window.goal_col();
        let new_col = self.col_to_byte_in_line(new_cursor, goal_col);
        self.current_window_mut().set_cursor(new_cursor, new_col);
    }

    /// Page up
    pub fn page_up(&mut self) {
        let height = self.current_window().height() as usize;
        let window = self.current_window_mut();

        window.scroll_up(height.saturating_sub(2));

        // Move cursor to top of new view
        let new_cursor = window.top_line();
        let goal_col = window.goal_col();
        let new_col = self.col_to_byte_in_line(new_cursor, goal_col);
        self.current_window_mut().set_cursor(new_cursor, new_col);
    }

    /// Move to beginning of buffer
    pub fn move_to_buffer_start(&mut self) {
        self.current_window_mut().set_cursor(0, 0);
        self.current_window_mut().set_top_line(0);
        self.current_window_mut().set_goal_col(0);
    }

    /// Move to end of buffer
    pub fn move_to_buffer_end(&mut self) {
        let last_line = self.current_buffer().line_count().saturating_sub(1);
        let last_col = self
            .current_buffer()
            .line(last_line)
            .map(|l| l.len())
            .unwrap_or(0);
        self.current_window_mut().set_cursor(last_line, last_col);
        self.ensure_cursor_visible();
    }

    /// Force a full redraw
    pub fn force_redraw(&mut self) {
        self.display.force_redraw();
    }

    /// Quit the editor
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Start a new kill sequence or continue appending
    pub fn start_kill(&mut self) {
        if !self.last_was_kill {
            // Start new kill entry
            self.kill_ring.push(String::new());
            self.kill_ring_idx = self.kill_ring.len().saturating_sub(1);
        }
        self.last_was_kill = true;
    }

    /// Append text to current kill entry
    pub fn kill_append(&mut self, text: &str) {
        if let Some(entry) = self.kill_ring.last_mut() {
            entry.push_str(text);
        }
    }

    /// Prepend text to current kill entry (for backward kills)
    pub fn kill_prepend(&mut self, text: &str) {
        if let Some(entry) = self.kill_ring.last_mut() {
            entry.insert_str(0, text);
        }
    }

    /// Get text for yanking
    pub fn yank_text(&self) -> Option<&str> {
        self.kill_ring.last().map(|s| s.as_str())
    }

    /// Clear the kill flag (called after non-kill commands)
    pub fn clear_kill_flag(&mut self) {
        self.last_was_kill = false;
    }

    /// Start incremental search
    pub fn start_search(&mut self, direction: SearchDirection) {
        self.search.active = true;
        self.search.pattern.clear();
        self.search.direction = direction;
        self.search.origin_line = self.current_window().cursor_line();
        self.search.origin_col = self.current_window().cursor_col();
        self.search.last_match_line = None;
        self.search.last_match_col = None;
        self.update_search_prompt();
    }

    /// Update search prompt in minibuffer
    fn update_search_prompt(&mut self) {
        let dir_str = match self.search.direction {
            SearchDirection::Forward => "I-search: ",
            SearchDirection::Backward => "I-search backward: ",
        };
        self.display.set_message(&format!("{}{}", dir_str, self.search.pattern));
    }

    /// Perform the search from current position
    pub fn do_search(&mut self) -> bool {
        if self.search.pattern.is_empty() {
            return false;
        }

        let start_line = self.current_window().cursor_line();
        let start_col = self.current_window().cursor_col();
        let line_count = self.current_buffer().line_count();

        match self.search.direction {
            SearchDirection::Forward => {
                // Search forward from current position
                for line_idx in start_line..line_count {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let search_start = if line_idx == start_line {
                            // Skip past current position to find next match
                            start_col + 1
                        } else {
                            0
                        };

                        if search_start < text.len() {
                            if let Some(pos) = text[search_start..].find(&self.search.pattern) {
                                let match_col = search_start + pos;
                                self.current_window_mut().set_cursor(line_idx, match_col);
                                self.search.last_match_line = Some(line_idx);
                                self.search.last_match_col = Some(match_col);
                                self.ensure_cursor_visible();
                                return true;
                            }
                        }
                    }
                }
                // Wrap around from beginning
                for line_idx in 0..=start_line {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let search_end = if line_idx == start_line { start_col } else { text.len() };

                        if let Some(pos) = text[..search_end].find(&self.search.pattern) {
                            self.current_window_mut().set_cursor(line_idx, pos);
                            self.search.last_match_line = Some(line_idx);
                            self.search.last_match_col = Some(pos);
                            self.ensure_cursor_visible();
                            self.display.set_message(&format!("Wrapped: {}{}",
                                if self.search.direction == SearchDirection::Forward { "I-search: " } else { "I-search backward: " },
                                self.search.pattern));
                            return true;
                        }
                    }
                }
            }
            SearchDirection::Backward => {
                // Search backward from current position
                for line_idx in (0..=start_line).rev() {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let search_end = if line_idx == start_line {
                            start_col
                        } else {
                            text.len()
                        };

                        if search_end > 0 {
                            // Find last occurrence before search_end
                            if let Some(pos) = text[..search_end].rfind(&self.search.pattern) {
                                self.current_window_mut().set_cursor(line_idx, pos);
                                self.search.last_match_line = Some(line_idx);
                                self.search.last_match_col = Some(pos);
                                self.ensure_cursor_visible();
                                return true;
                            }
                        }
                    }
                }
                // Wrap around from end
                for line_idx in (start_line..line_count).rev() {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let search_start = if line_idx == start_line { start_col + 1 } else { 0 };

                        if search_start < text.len() {
                            if let Some(pos) = text[search_start..].rfind(&self.search.pattern) {
                                let match_col = search_start + pos;
                                self.current_window_mut().set_cursor(line_idx, match_col);
                                self.search.last_match_line = Some(line_idx);
                                self.search.last_match_col = Some(match_col);
                                self.ensure_cursor_visible();
                                self.display.set_message(&format!("Wrapped: {}{}",
                                    if self.search.direction == SearchDirection::Forward { "I-search: " } else { "I-search backward: " },
                                    self.search.pattern));
                                return true;
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// End search mode
    pub fn end_search(&mut self, abort: bool) {
        if abort {
            // Restore original position
            let origin_line = self.search.origin_line;
            let origin_col = self.search.origin_col;
            self.current_window_mut().set_cursor(origin_line, origin_col);
        }
        self.search.active = false;
        self.display.clear_message();
    }

    /// Handle key press during search mode
    fn handle_search_key(&mut self, key: Key) -> Result<()> {
        // C-g aborts search
        if key == Key::ctrl('g') {
            self.end_search(true);
            self.display.set_message("Quit");
            return Ok(());
        }

        // Enter/Escape exits search at current position
        if key == Key::ctrl('m') || key == Key::ctrl('[') {
            self.end_search(false);
            return Ok(());
        }

        // C-s repeats search forward
        if key == Key::ctrl('s') {
            self.search.direction = SearchDirection::Forward;
            if !self.search.pattern.is_empty() {
                if !self.do_search() {
                    self.display.set_message(&format!("Failing I-search: {}", self.search.pattern));
                    let _ = self.terminal.beep();
                } else {
                    self.update_search_prompt();
                }
            } else {
                self.update_search_prompt();
            }
            return Ok(());
        }

        // C-r repeats search backward
        if key == Key::ctrl('r') {
            self.search.direction = SearchDirection::Backward;
            if !self.search.pattern.is_empty() {
                if !self.do_search() {
                    self.display.set_message(&format!("Failing I-search backward: {}", self.search.pattern));
                    let _ = self.terminal.beep();
                } else {
                    self.update_search_prompt();
                }
            } else {
                self.update_search_prompt();
            }
            return Ok(());
        }

        // Backspace removes last character from pattern
        if key == Key(0x7f) || key == Key::ctrl('h') {
            if !self.search.pattern.is_empty() {
                self.search.pattern.pop();
                // Re-search from origin
                let origin_line = self.search.origin_line;
                let origin_col = self.search.origin_col;
                self.current_window_mut().set_cursor(origin_line, origin_col);
                if !self.search.pattern.is_empty() {
                    self.do_search();
                }
                self.update_search_prompt();
            }
            return Ok(());
        }

        // Printable character - add to pattern and search
        if key.is_self_insert() {
            if let Some(ch) = key.base_char() {
                self.search.pattern.push(ch);
                if !self.do_search() {
                    self.display.set_message(&format!("Failing {}: {}",
                        if self.search.direction == SearchDirection::Forward { "I-search" } else { "I-search backward" },
                        self.search.pattern));
                    let _ = self.terminal.beep();
                } else {
                    self.update_search_prompt();
                }
            }
            return Ok(());
        }

        // Unknown key - beep
        let _ = self.terminal.beep();
        Ok(())
    }

    /// Ensure cursor is visible, scrolling if needed
    pub fn ensure_cursor_visible(&mut self) {
        self.current_window_mut().ensure_cursor_visible();
    }

    /// Convert display column to byte offset in a line
    fn col_to_byte_in_line(&self, line_idx: usize, display_col: usize) -> usize {
        if let Some(line) = self.current_buffer().line(line_idx) {
            line.col_to_byte(display_col).unwrap_or(line.len())
        } else {
            0
        }
    }
}
