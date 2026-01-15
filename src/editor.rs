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
    /// Minibuffer prompt state
    pub prompt: PromptState,
    /// Query-replace state
    pub query_replace: QueryReplaceState,
    /// Keyboard macro state
    pub macro_state: MacroState,
}

/// What action to perform when prompt completes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAction {
    None,
    FindFile,
    SwitchBuffer,
    KillBuffer,
    GotoLine,
    QueryReplaceSearch,   // First prompt: enter search string
    QueryReplaceReplace,  // Second prompt: enter replacement string
    ShellCommand,         // Execute shell command
}

/// Minibuffer prompt state
#[derive(Debug, Clone)]
pub struct PromptState {
    /// Whether prompt is active
    pub active: bool,
    /// The prompt string (e.g., "Find file: ")
    pub prompt: String,
    /// Current input
    pub input: String,
    /// What to do when complete
    pub action: PromptAction,
    /// Default value (shown in prompt)
    pub default: Option<String>,
}

impl Default for PromptState {
    fn default() -> Self {
        Self {
            active: false,
            prompt: String::new(),
            input: String::new(),
            action: PromptAction::None,
            default: None,
        }
    }
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

/// Query-replace state
#[derive(Debug, Clone)]
pub struct QueryReplaceState {
    /// Whether query-replace is active
    pub active: bool,
    /// Search pattern
    pub search: String,
    /// Replacement string
    pub replace: String,
    /// Replace all remaining without prompting
    pub replace_all: bool,
    /// Number of replacements made
    pub count: usize,
}

impl Default for QueryReplaceState {
    fn default() -> Self {
        Self {
            active: false,
            search: String::new(),
            replace: String::new(),
            replace_all: false,
            count: 0,
        }
    }
}

/// Keyboard macro state
#[derive(Debug, Clone, Default)]
pub struct MacroState {
    /// Whether we're recording a macro
    pub recording: bool,
    /// Recorded key sequence
    pub keys: Vec<Key>,
    /// Whether we're playing back (to prevent recursion)
    pub playing: bool,
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
            prompt: PromptState::default(),
            query_replace: QueryReplaceState::default(),
            macro_state: MacroState::default(),
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

    /// Create a new buffer for a file that doesn't exist yet
    pub fn open_new_file(&mut self, path: &PathBuf) {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let mut buffer = Buffer::new(&name);
        buffer.set_filename(path.clone());
        self.buffers.push(buffer);
        let buf_idx = self.buffers.len() - 1;

        // Set current window to show the new buffer
        if let Some(window) = self.windows.get_mut(self.current_window) {
            window.set_buffer_idx(buf_idx);
        }

        self.display.force_redraw();
        self.display.set_message(&format!("(New file) {}", name));
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
            } else if self.input.is_pending() {
                // Show visual feedback that we're waiting for continuation key
                if self.input.is_ctlx_pending() {
                    self.display.set_message("C-x -");
                } else if self.input.is_meta_pending() {
                    self.display.set_message("ESC -");
                }
            }
            // Debug: if key event was ignored (not Press), we just continue the loop
        }

        Ok(())
    }

    /// Handle a key press
    fn handle_key(&mut self, key: Key) -> Result<()> {
        // Handle prompt mode
        if self.prompt.active {
            return self.handle_prompt_key(key);
        }

        // Handle search mode
        if self.search.active {
            return self.handle_search_key(key);
        }

        // Handle query-replace mode
        if self.query_replace.active {
            return self.handle_query_replace_key(key);
        }

        // Record key for macro (if recording and not playing back)
        // We record the key before processing so we can record what was pressed
        // Note: macro control keys (C-x (, C-x ), C-x e) will exclude themselves
        let should_record = self.macro_state.recording && !self.macro_state.playing;

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
                CommandStatus::Success => {
                    // Record successful command keys for macro
                    // (macro control commands will clear this themselves)
                    if should_record {
                        self.macro_state.keys.push(key);
                    }
                }
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
            // Record self-insert keys for macro
            if should_record {
                self.macro_state.keys.push(key);
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

    /// Check if a line is blank (empty or only whitespace)
    fn is_blank_line(&self, line_idx: usize) -> bool {
        self.current_buffer()
            .line(line_idx)
            .map(|l| l.text().trim().is_empty())
            .unwrap_or(true)
    }

    /// Move backward to start of paragraph
    pub fn backward_paragraph(&mut self) {
        let mut line = self.current_window().cursor_line();
        let line_count = self.current_buffer().line_count();

        if line == 0 {
            self.current_window_mut().set_cursor(0, 0);
            return;
        }

        // If on a non-blank line, first skip to a blank line
        while line > 0 && !self.is_blank_line(line) {
            line -= 1;
        }

        // Then skip over blank lines
        while line > 0 && self.is_blank_line(line) {
            line -= 1;
        }

        // Then find the start of this paragraph (first blank line or start of buffer)
        while line > 0 && !self.is_blank_line(line - 1) {
            line -= 1;
        }

        self.current_window_mut().set_cursor(line, 0);
        self.current_window_mut().set_goal_col(0);
        self.ensure_cursor_visible();
    }

    /// Move forward to end of paragraph
    pub fn forward_paragraph(&mut self) {
        let mut line = self.current_window().cursor_line();
        let line_count = self.current_buffer().line_count();

        if line >= line_count {
            return;
        }

        // Skip over blank lines first
        while line < line_count && self.is_blank_line(line) {
            line += 1;
        }

        // Then skip over non-blank lines (the paragraph content)
        while line < line_count && !self.is_blank_line(line) {
            line += 1;
        }

        // Position at the blank line after paragraph (or end of buffer)
        let final_line = line.min(line_count.saturating_sub(1));
        self.current_window_mut().set_cursor(final_line, 0);
        self.current_window_mut().set_goal_col(0);
        self.ensure_cursor_visible();
    }

    /// Fill (reflow) the current paragraph to fill_column width
    pub fn fill_paragraph(&mut self, fill_column: usize) {
        let start_line = self.current_window().cursor_line();
        let line_count = self.current_buffer().line_count();

        // Find paragraph boundaries
        let mut para_start = start_line;
        let mut para_end = start_line;

        // Find start of paragraph
        while para_start > 0 && !self.is_blank_line(para_start - 1) {
            para_start -= 1;
        }
        // Skip if we're on a blank line
        if self.is_blank_line(para_start) {
            self.display.set_message("Not in a paragraph");
            return;
        }

        // Find end of paragraph
        while para_end < line_count && !self.is_blank_line(para_end) {
            para_end += 1;
        }

        // Collect all words from the paragraph
        let mut words: Vec<String> = Vec::new();
        for line_idx in para_start..para_end {
            if let Some(line) = self.current_buffer().line(line_idx) {
                for word in line.text().split_whitespace() {
                    words.push(word.to_string());
                }
            }
        }

        if words.is_empty() {
            return;
        }

        // Reflow words into lines
        let mut new_lines: Vec<String> = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word;
            } else if current_line.len() + 1 + word.len() <= fill_column {
                current_line.push(' ');
                current_line.push_str(&word);
            } else {
                new_lines.push(current_line);
                current_line = word;
            }
        }
        if !current_line.is_empty() {
            new_lines.push(current_line);
        }

        // Replace the paragraph lines
        // First, delete the old paragraph lines
        for _ in para_start..para_end {
            self.current_buffer_mut().delete_line(para_start);
        }

        // Insert new lines (in reverse order since we insert at para_start)
        for (i, line_content) in new_lines.iter().enumerate() {
            let line_idx = para_start + i;
            // Insert a new line at the position
            if line_idx >= self.current_buffer().line_count() {
                // Append at end
                self.current_buffer_mut().append_line();
            } else if i > 0 {
                // Insert line before
                self.current_buffer_mut().insert_line_at(line_idx);
            }
            // Set line content
            if let Some(line) = self.current_buffer_mut().line_mut(line_idx) {
                line.clear();
                for ch in line_content.chars() {
                    line.insert_char(line.len(), ch);
                }
            }
        }

        self.current_buffer_mut().set_modified(true);
        self.current_window_mut().set_cursor(para_start, 0);
        self.ensure_cursor_visible();
        self.display.set_message(&format!("Filled paragraph ({} lines)", new_lines.len()));
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

    /// Start a minibuffer prompt
    pub fn start_prompt(&mut self, prompt: &str, action: PromptAction, default: Option<String>) {
        self.prompt.active = true;
        self.prompt.prompt = prompt.to_string();
        self.prompt.input.clear();
        self.prompt.action = action;
        self.prompt.default = default;
        self.update_prompt_display();
    }

    /// Update the prompt display
    fn update_prompt_display(&mut self) {
        let display = if let Some(ref def) = self.prompt.default {
            if self.prompt.input.is_empty() {
                format!("{} (default {}): ", self.prompt.prompt, def)
            } else {
                format!("{}: {}", self.prompt.prompt, self.prompt.input)
            }
        } else {
            format!("{}: {}", self.prompt.prompt, self.prompt.input)
        };
        self.display.set_message(&display);
    }

    /// Handle key press during prompt mode
    fn handle_prompt_key(&mut self, key: Key) -> Result<()> {
        // C-g aborts
        if key == Key::ctrl('g') {
            self.prompt.active = false;
            self.prompt.action = PromptAction::None;
            self.display.set_message("Quit");
            return Ok(());
        }

        // Enter completes
        if key == Key::ctrl('m') {
            let input = if self.prompt.input.is_empty() {
                self.prompt.default.clone().unwrap_or_default()
            } else {
                self.prompt.input.clone()
            };
            let action = self.prompt.action.clone();
            self.prompt.active = false;
            self.display.clear_message();
            return self.complete_prompt(action, input);
        }

        // Backspace
        if key == Key(0x7f) || key == Key::ctrl('h') {
            self.prompt.input.pop();
            self.update_prompt_display();
            return Ok(());
        }

        // Printable character
        if key.is_self_insert() {
            if let Some(ch) = key.base_char() {
                self.prompt.input.push(ch);
                self.update_prompt_display();
            }
            return Ok(());
        }

        // Unknown key - beep
        let _ = self.terminal.beep();
        Ok(())
    }

    /// Complete a prompt action
    fn complete_prompt(&mut self, action: PromptAction, input: String) -> Result<()> {
        match action {
            PromptAction::FindFile => {
                if input.is_empty() {
                    self.display.set_message("No file name");
                    return Ok(());
                }
                let path = PathBuf::from(&input);
                match self.open_file(&path) {
                    Ok(()) => {
                        self.display.set_message(&format!("Opened {}", input));
                    }
                    Err(e) => {
                        // File doesn't exist - create new buffer with that name
                        let mut buffer = Buffer::new(&input);
                        buffer.set_filename(path);
                        self.buffers.push(buffer);
                        let buf_idx = self.buffers.len() - 1;
                        if let Some(window) = self.windows.get_mut(self.current_window) {
                            window.set_buffer_idx(buf_idx);
                        }
                        self.display.set_message(&format!("(New file) {}", input));
                    }
                }
            }
            PromptAction::SwitchBuffer => {
                if input.is_empty() {
                    return Ok(());
                }
                // Find buffer by name
                if let Some(idx) = self.buffers.iter().position(|b| b.name() == input) {
                    if let Some(window) = self.windows.get_mut(self.current_window) {
                        window.set_buffer_idx(idx);
                        window.set_cursor(0, 0);
                    }
                    self.display.force_redraw();
                } else {
                    self.display.set_message(&format!("No buffer named {}", input));
                }
            }
            PromptAction::KillBuffer => {
                if input.is_empty() {
                    return Ok(());
                }
                // Find buffer by name
                if let Some(idx) = self.buffers.iter().position(|b| b.name() == input) {
                    if self.buffers.len() <= 1 {
                        self.display.set_message("Can't kill the only buffer");
                        return Ok(());
                    }
                    // Check if modified
                    if self.buffers[idx].is_modified() {
                        self.display.set_message(&format!("Buffer {} modified; kill anyway? (not implemented)", input));
                        return Ok(());
                    }
                    self.buffers.remove(idx);
                    // Update window buffer indices
                    for window in &mut self.windows {
                        let win_buf = window.buffer_idx();
                        if win_buf == idx {
                            window.set_buffer_idx(0);
                            window.set_cursor(0, 0);
                        } else if win_buf > idx {
                            window.set_buffer_idx(win_buf - 1);
                        }
                    }
                    self.display.force_redraw();
                    self.display.set_message(&format!("Killed buffer {}", input));
                } else {
                    self.display.set_message(&format!("No buffer named {}", input));
                }
            }
            PromptAction::GotoLine => {
                if let Ok(line_num) = input.parse::<usize>() {
                    let target = line_num.saturating_sub(1); // Convert to 0-indexed
                    let max_line = self.current_buffer().line_count().saturating_sub(1);
                    let target = target.min(max_line);
                    self.current_window_mut().set_cursor(target, 0);
                    self.ensure_cursor_visible();
                } else {
                    self.display.set_message("Invalid line number");
                }
            }
            PromptAction::QueryReplaceSearch => {
                if input.is_empty() {
                    self.display.set_message("No search string");
                    return Ok(());
                }
                // Store search string and prompt for replacement
                self.query_replace.search = input.clone();
                self.prompt.active = true;
                self.prompt.prompt = format!("Query replace {} with: ", input);
                self.prompt.input.clear();
                self.prompt.action = PromptAction::QueryReplaceReplace;
                self.prompt.default = None;
                self.update_prompt_display();
            }
            PromptAction::QueryReplaceReplace => {
                // Store replacement and start query-replace mode
                self.query_replace.replace = input;
                self.query_replace.active = true;
                self.query_replace.replace_all = false;
                self.query_replace.count = 0;
                // Find first match
                self.query_replace_next();
            }
            PromptAction::ShellCommand => {
                if input.is_empty() {
                    return Ok(());
                }
                self.execute_shell_command(&input);
            }
            PromptAction::None => {}
        }
        Ok(())
    }

    /// Get list of buffer names for completion
    pub fn buffer_names(&self) -> Vec<&str> {
        self.buffers.iter().map(|b| b.name()).collect()
    }

    /// Split current window horizontally
    pub fn split_window(&mut self) -> bool {
        let current_height = self.current_window().height();

        // Need at least 3 rows to split (1 for each window + mode line)
        if current_height < 4 {
            return false;
        }

        let top_row = self.current_window().top_row();
        let buffer_idx = self.current_window().buffer_idx();
        let cursor_line = self.current_window().cursor_line();
        let cursor_col = self.current_window().cursor_col();

        // Calculate new heights
        let top_height = current_height / 2;
        let bottom_height = current_height - top_height - 1; // -1 for mode line

        // Update current window (becomes top)
        self.current_window_mut().set_height(top_height);

        // Create new window (bottom), showing same buffer
        let mut new_window = Window::new(buffer_idx, top_row + top_height + 1, bottom_height);
        new_window.set_cursor(cursor_line, cursor_col);
        new_window.ensure_cursor_visible();

        // Insert new window after current
        let insert_pos = self.current_window + 1;
        self.windows.insert(insert_pos, new_window);

        // Update positions of windows below the split
        self.recalculate_window_positions();

        self.display.force_redraw();
        true
    }

    /// Delete current window
    pub fn delete_window(&mut self) -> bool {
        if self.windows.len() <= 1 {
            return false;
        }

        let deleted_height = self.current_window().height() + 1; // +1 for mode line
        let deleted_idx = self.current_window;

        // Give space to adjacent window
        if deleted_idx > 0 {
            // Give to window above
            let above_height = self.windows[deleted_idx - 1].height();
            self.windows[deleted_idx - 1].set_height(above_height + deleted_height);
        } else if deleted_idx + 1 < self.windows.len() {
            // Give to window below
            let below_height = self.windows[deleted_idx + 1].height();
            self.windows[deleted_idx + 1].set_height(below_height + deleted_height);
        }

        self.windows.remove(deleted_idx);

        // Update current window index
        if self.current_window >= self.windows.len() {
            self.current_window = self.windows.len() - 1;
        }

        self.recalculate_window_positions();
        self.display.force_redraw();
        true
    }

    /// Delete all windows except current
    pub fn delete_other_windows(&mut self) -> bool {
        if self.windows.len() <= 1 {
            return false;
        }

        let buffer_idx = self.current_window().buffer_idx();
        let cursor_line = self.current_window().cursor_line();
        let cursor_col = self.current_window().cursor_col();
        let top_line = self.current_window().top_line();

        // Calculate total height available
        let total_height = self.terminal.rows().saturating_sub(2); // -2 for mode line and minibuffer

        // Create single window
        let mut window = Window::new(buffer_idx, 0, total_height);
        window.set_cursor(cursor_line, cursor_col);
        window.set_top_line(top_line);

        self.windows = vec![window];
        self.current_window = 0;

        self.display.force_redraw();
        true
    }

    /// Switch to other window
    pub fn other_window(&mut self) {
        if self.windows.len() > 1 {
            self.current_window = (self.current_window + 1) % self.windows.len();
        }
    }

    /// Recalculate window positions after split/delete
    fn recalculate_window_positions(&mut self) {
        let mut current_row: u16 = 0;
        for window in &mut self.windows {
            window.set_top_row(current_row);
            current_row += window.height() + 1; // +1 for mode line
        }
    }

    /// Get number of windows
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Find and move to next match for query-replace
    pub fn query_replace_next(&mut self) -> bool {
        if self.query_replace.search.is_empty() {
            return false;
        }

        let start_line = self.current_window().cursor_line();
        let start_col = self.current_window().cursor_col();
        let line_count = self.current_buffer().line_count();
        let pattern = self.query_replace.search.clone();

        // Search forward from current position
        for line_idx in start_line..line_count {
            if let Some(line) = self.current_buffer().line(line_idx) {
                let text = line.text();
                let search_start = if line_idx == start_line {
                    start_col
                } else {
                    0
                };

                if search_start < text.len() {
                    if let Some(pos) = text[search_start..].find(&pattern) {
                        let match_col = search_start + pos;
                        self.current_window_mut().set_cursor(line_idx, match_col);
                        self.ensure_cursor_visible();

                        // Show prompt for this match
                        if self.query_replace.replace_all {
                            // Auto-replace without prompting
                            self.query_replace_do_replace();
                            return self.query_replace_next();
                        } else {
                            let search = &self.query_replace.search;
                            let replace = &self.query_replace.replace;
                            self.display.set_message(&format!(
                                "Query replacing {} with {}: (y/n/!/q/?)",
                                search, replace
                            ));
                        }
                        return true;
                    }
                }
            }
        }

        // No more matches - wrap to beginning? For now, just end
        let count = self.query_replace.count;
        self.query_replace.active = false;
        self.display.set_message(&format!("Replaced {} occurrences", count));
        false
    }

    /// Perform replacement at current cursor position
    pub fn query_replace_do_replace(&mut self) {
        let line_idx = self.current_window().cursor_line();
        let col = self.current_window().cursor_col();
        let search_len = self.query_replace.search.len();
        let replace_str = self.query_replace.replace.clone();

        if let Some(line) = self.current_buffer_mut().line_mut(line_idx) {
            // Delete the search string
            line.delete_range(col, col + search_len);
            // Insert the replacement
            for (i, ch) in replace_str.chars().enumerate() {
                line.insert_char(col + i, ch);
            }
        }

        self.current_buffer_mut().set_modified(true);
        self.query_replace.count += 1;

        // Move cursor past the replacement
        let new_col = col + replace_str.len();
        self.current_window_mut().set_cursor(line_idx, new_col);
    }

    /// Handle key input during query-replace mode
    pub fn handle_query_replace_key(&mut self, key: Key) -> Result<()> {
        match key.base_char() {
            Some('y') | Some(' ') => {
                // Replace and continue
                self.query_replace_do_replace();
                self.query_replace_next();
            }
            Some('n') => {
                // Skip and continue
                let line = self.current_window().cursor_line();
                let col = self.current_window().cursor_col();
                let search_len = self.query_replace.search.len();
                self.current_window_mut().set_cursor(line, col + search_len);
                self.query_replace_next();
            }
            Some('!') => {
                // Replace all remaining
                self.query_replace.replace_all = true;
                self.query_replace_do_replace();
                self.query_replace_next();
            }
            Some('q') | Some('\r') => {
                // Quit
                let count = self.query_replace.count;
                self.query_replace.active = false;
                self.display.set_message(&format!("Replaced {} occurrences", count));
            }
            Some('.') => {
                // Replace this one and quit
                self.query_replace_do_replace();
                let count = self.query_replace.count;
                self.query_replace.active = false;
                self.display.set_message(&format!("Replaced {} occurrences", count));
            }
            Some('?') => {
                // Show help
                self.display.set_message("y:replace n:skip !:all q:quit .:replace+quit");
            }
            _ => {
                // Check for C-g abort
                if key == Key::ctrl('g') {
                    self.query_replace.active = false;
                    self.display.set_message("Quit");
                } else {
                    self.terminal.beep()?;
                }
            }
        }
        Ok(())
    }

    /// Create or update the buffer list and switch to it
    pub fn list_buffers(&mut self) {
        // Generate buffer list content
        let mut content = String::new();
        content.push_str(" MR Buffer           Size  File\n");
        content.push_str(" -- ------           ----  ----\n");

        for (idx, buffer) in self.buffers.iter().enumerate() {
            // Check if this buffer is displayed in current window
            let current_marker = if self.current_window().buffer_idx() == idx {
                '.'
            } else {
                ' '
            };

            // Modified marker
            let modified_marker = if buffer.is_modified() { '*' } else { ' ' };

            // Buffer name (truncate if too long)
            let name = buffer.name();
            let name_display = if name.len() > 16 {
                &name[..16]
            } else {
                name
            };

            // Size (line count)
            let size = buffer.line_count();

            // File path
            let file = buffer
                .filename()
                .map(|p| p.display().to_string())
                .unwrap_or_default();

            content.push_str(&format!(
                " {}{} {:<16} {:>5}  {}\n",
                current_marker, modified_marker, name_display, size, file
            ));
        }

        // Find or create the *Buffer List* buffer
        let list_buf_name = "*Buffer List*";
        if let Some(idx) = self.buffers.iter().position(|b| b.name() == list_buf_name) {
            // Update existing buffer
            self.buffers[idx].set_content(&content);
            // Switch to it
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        } else {
            // Create new buffer
            let buffer = Buffer::from_content(list_buf_name, &content);
            self.buffers.push(buffer);
            let idx = self.buffers.len() - 1;
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        }

        self.display.force_redraw();
        self.display.set_message("");
    }

    /// Execute a shell command and display output in a buffer
    pub fn execute_shell_command(&mut self, command: &str) {
        use std::process::Command;

        // Determine shell based on platform
        #[cfg(windows)]
        let output = Command::new("cmd")
            .args(["/C", command])
            .output();

        #[cfg(not(windows))]
        let output = Command::new("sh")
            .args(["-c", command])
            .output();

        let content = match output {
            Ok(output) => {
                let mut result = String::new();
                if !output.stdout.is_empty() {
                    result.push_str(&String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    if !result.is_empty() {
                        result.push('\n');
                    }
                    result.push_str(&String::from_utf8_lossy(&output.stderr));
                }
                if result.is_empty() {
                    "(No output)".to_string()
                } else {
                    result
                }
            }
            Err(e) => format!("Error executing command: {}", e),
        };

        // Find or create the *Shell Command Output* buffer
        let buf_name = "*Shell Command Output*";
        if let Some(idx) = self.buffers.iter().position(|b| b.name() == buf_name) {
            self.buffers[idx].set_content(&content);
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        } else {
            let buffer = Buffer::from_content(buf_name, &content);
            self.buffers.push(buffer);
            let idx = self.buffers.len() - 1;
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        }

        self.display.force_redraw();
        self.display.set_message(&format!("Shell command: {}", command));
    }

    /// Start recording a keyboard macro
    pub fn start_macro(&mut self) {
        if self.macro_state.playing {
            self.display.set_message("Can't define macro while executing macro");
            return;
        }
        self.macro_state.recording = true;
        self.macro_state.keys.clear();
        self.display.set_message("Defining keyboard macro...");
    }

    /// End recording a keyboard macro
    pub fn end_macro(&mut self) {
        if !self.macro_state.recording {
            self.display.set_message("Not defining keyboard macro");
            return;
        }
        self.macro_state.recording = false;
        let count = self.macro_state.keys.len();
        self.display.set_message(&format!("Keyboard macro defined ({} keys)", count));
    }

    /// Execute the keyboard macro
    pub fn execute_macro(&mut self) -> Result<()> {
        if self.macro_state.playing {
            // Already playing - ignore to prevent infinite recursion
            return Ok(());
        }
        if self.macro_state.recording {
            self.display.set_message("Can't execute macro while defining it");
            return Ok(());
        }
        if self.macro_state.keys.is_empty() {
            self.display.set_message("No keyboard macro defined");
            return Ok(());
        }

        // Copy keys to avoid borrow issues
        let keys: Vec<Key> = self.macro_state.keys.clone();

        self.macro_state.playing = true;
        for key in keys {
            self.handle_key(key)?;
            // Check if we should stop (e.g., user aborted)
            if !self.running {
                break;
            }
        }
        self.macro_state.playing = false;

        Ok(())
    }
}
