//! Editor state and main loop

use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    /// Track last yank position for yank-pop
    pub last_yank_start: Option<(usize, usize)>,
    pub last_yank_end: Option<(usize, usize)>,
    pub last_was_yank: bool,
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
    /// Universal argument (C-u prefix)
    pub prefix_arg: PrefixArg,
    /// Stored region for filter operation
    pub filter_region: Option<(usize, usize, usize, usize)>,
    /// Auto-save: last time we auto-saved
    pub last_auto_save: Instant,
    /// Auto-save: interval between saves (default 30 seconds)
    pub auto_save_interval: Duration,
    /// Auto-save: whether enabled
    pub auto_save_enabled: bool,
    /// Whether to warn before closing unsaved buffers
    pub warn_unsaved: bool,
    /// Pending quit (waiting for confirmation after unsaved warning)
    pub pending_quit: bool,
}

/// Universal argument state for C-u prefix
#[derive(Debug, Clone, Default)]
pub struct PrefixArg {
    /// Whether a prefix argument is active
    pub active: bool,
    /// The numeric value (None = just C-u with no digits)
    pub value: Option<i32>,
    /// Number of times C-u was pressed (for C-u C-u = 16, etc.)
    pub multiplier: i32,
}

/// What action to perform when prompt completes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptAction {
    None,
    FindFile,
    InsertFile,           // Insert file contents at cursor
    SwitchBuffer,
    KillBuffer,
    GotoLine,
    QueryReplaceSearch,   // First prompt: enter search string
    QueryReplaceReplace,  // Second prompt: enter replacement string
    ReplaceStringSearch,  // Non-interactive replace: enter search string
    ReplaceStringReplace, // Non-interactive replace: enter replacement string
    ShellCommand,         // Execute shell command
    FilterBuffer,         // Pipe buffer through shell command
    FilterRegion,         // Pipe region through shell command (output to buffer)
    FilterRegionReplace,  // Pipe region through shell command (replace region)
    WriteFile,            // Save buffer to a new filename
    ExtendedCommand,      // M-x: execute command by name
    ConfirmQuit,          // Confirm quit with unsaved buffers
    ConfirmKillBuffer,    // Confirm kill buffer with unsaved changes
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
    /// Recorded key sequence (current macro)
    pub keys: Vec<Key>,
    /// Whether we're playing back (to prevent recursion)
    pub playing: bool,
    /// Named macro slots (0-9)
    pub slots: [Vec<Key>; 10],
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
            last_yank_start: None,
            last_yank_end: None,
            last_was_yank: false,
            quote_pending: false,
            search: SearchState::default(),
            prompt: PromptState::default(),
            query_replace: QueryReplaceState::default(),
            macro_state: MacroState::default(),
            prefix_arg: PrefixArg::default(),
            filter_region: None,
            last_auto_save: Instant::now(),
            auto_save_interval: Duration::from_secs(30),
            auto_save_enabled: true,
            warn_unsaved: true,
            pending_quit: false,
        }
    }

    /// Apply configuration settings
    pub fn apply_config(&mut self, config: &crate::config::Config) {
        // Display settings
        self.display.show_line_numbers = config.show_line_numbers;

        // Auto-save settings
        self.auto_save_enabled = config.auto_save;
        self.auto_save_interval = std::time::Duration::from_secs(config.auto_save_interval);

        // Warning settings
        self.warn_unsaved = config.warn_unsaved;

        // Load saved macros from disk
        self.load_macros_on_startup();

        // Tab width is stored in Line, but we don't have a global tab width setting yet
        // This could be added in the future
    }

    /// Load macros from the macros file at startup
    fn load_macros_on_startup(&mut self) {
        let slots = crate::macro_store::load_macros();
        let count = crate::macro_store::count_stored_macros(&slots);
        if count > 0 {
            self.macro_state.slots = slots;
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

    /// Get the key table for looking up bindings
    pub fn key_table(&self) -> &KeyTable {
        &self.keytab
    }

    /// Read a key for describe-key command, handling prefix sequences
    pub fn read_key_for_describe(&mut self) -> Result<Option<Key>> {
        use crate::input::InputState;

        // Use a fresh input state for reading the describe key
        let mut input_state = InputState::new();

        loop {
            let key_event = self.terminal.read_key()?;

            // Check for C-g (abort)
            if let crossterm::event::KeyCode::Char('g') = key_event.code {
                if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                    return Ok(None);
                }
            }

            if let Some(key) = input_state.translate_key(key_event) {
                return Ok(Some(key));
            }

            // Show visual feedback for pending prefix
            if input_state.is_pending() {
                if input_state.is_ctlx_pending() {
                    self.display.set_message("Describe key: C-x -");
                } else if input_state.is_meta_pending() {
                    self.display.set_message("Describe key: ESC -");
                }
                self.display.render(
                    &mut self.terminal,
                    &self.windows,
                    &self.buffers,
                    self.current_window,
                )?;
            }
        }
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
                // Check if it's time to auto-save
                self.check_auto_save();
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

        // Handle quote mode - insert next character literally
        if self.quote_pending {
            self.quote_pending = false;
            self.prefix_arg = PrefixArg::default(); // Clear prefix on quote
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

        // Handle C-u (universal argument)
        if key == Key::ctrl('u') {
            if self.prefix_arg.active {
                // Another C-u multiplies by 4
                if self.prefix_arg.value.is_none() {
                    self.prefix_arg.multiplier *= 4;
                }
            } else {
                // Start prefix arg
                self.prefix_arg.active = true;
                self.prefix_arg.multiplier = 4;
                self.prefix_arg.value = None;
            }
            self.show_prefix_arg();
            return Ok(());
        }

        // Handle M-digit (digit-argument) - start prefix arg with that digit
        if key.is_meta() && !key.is_ctrl() && !key.is_ctlx() {
            if let Some(ch) = key.base_char() {
                if ch.is_ascii_digit() {
                    let digit = ch.to_digit(10).unwrap() as i32;
                    if self.prefix_arg.active {
                        // Continue building the number
                        self.prefix_arg.value = Some(
                            self.prefix_arg.value.unwrap_or(0) * 10 + digit
                        );
                    } else {
                        // Start prefix arg with this digit
                        self.prefix_arg.active = true;
                        self.prefix_arg.multiplier = 1;
                        self.prefix_arg.value = Some(digit);
                    }
                    self.show_prefix_arg();
                    return Ok(());
                }
                // M-- (negative-argument)
                if ch == '-' {
                    if !self.prefix_arg.active {
                        self.prefix_arg.active = true;
                        self.prefix_arg.multiplier = -1;
                        self.prefix_arg.value = None;
                    } else if self.prefix_arg.value.is_none() {
                        self.prefix_arg.multiplier = -self.prefix_arg.multiplier.abs();
                    }
                    self.show_prefix_arg();
                    return Ok(());
                }
            }
        }

        // Handle digits during prefix arg
        if self.prefix_arg.active {
            if let Some(ch) = key.base_char() {
                if ch.is_ascii_digit() {
                    let digit = ch.to_digit(10).unwrap() as i32;
                    self.prefix_arg.value = Some(
                        self.prefix_arg.value.unwrap_or(0) * 10 + digit
                    );
                    // Clear multiplier when explicit digits are entered
                    self.prefix_arg.multiplier = 1;
                    self.show_prefix_arg();
                    return Ok(());
                }
                // Negative argument with -
                if ch == '-' && self.prefix_arg.value.is_none() {
                    self.prefix_arg.value = Some(0);
                    self.prefix_arg.multiplier = -self.prefix_arg.multiplier.abs();
                    self.show_prefix_arg();
                    return Ok(());
                }
            }
        }

        // Clear any previous message (but not if we're showing prefix)
        if !self.prefix_arg.active {
            self.display.clear_message();
        }

        // Get the prefix argument values
        let (has_arg, arg_value) = if self.prefix_arg.active {
            let value = self.prefix_arg.value.unwrap_or(1) * self.prefix_arg.multiplier;
            (true, value)
        } else {
            (false, 1)
        };

        // Clear prefix arg before executing (so command sees clean state)
        self.prefix_arg = PrefixArg::default();
        self.display.clear_message();

        // Clear kill/yank flags - commands will set them if needed
        // This ensures consecutive kills append but non-consecutive kills don't
        self.last_was_kill = false;
        self.last_was_yank = false;

        // Look up command
        if let Some(cmd) = self.keytab.lookup(key) {
            // Execute command with prefix argument
            match cmd(self, has_arg, arg_value)? {
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
            // Self-insert character (possibly multiple times with prefix)
            if let Some(ch) = key.base_char() {
                let count = if has_arg { arg_value.max(1) } else { 1 };
                for _ in 0..count {
                    self.insert_char(ch);
                }
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
        // First, delete the old paragraph lines (deleting from para_start repeatedly)
        let lines_to_delete = para_end - para_start;
        for _ in 0..lines_to_delete {
            if para_start < self.current_buffer().line_count() {
                self.current_buffer_mut().delete_line(para_start);
            }
        }

        // Insert new lines at para_start
        for (i, line_content) in new_lines.iter().enumerate() {
            let line_idx = para_start + i;
            // Always insert a new line (except if we're past the end and need to append)
            if line_idx >= self.current_buffer().line_count() {
                // Append at end
                self.current_buffer_mut().append_line();
            } else {
                // Insert line at this position
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

    /// Show the current prefix argument in the message line
    fn show_prefix_arg(&mut self) {
        let msg = if let Some(value) = self.prefix_arg.value {
            format!("C-u {}", value * self.prefix_arg.multiplier)
        } else {
            format!("C-u {}-", self.prefix_arg.multiplier)
        };
        self.display.set_message(&msg);
    }

    /// Force a full redraw
    pub fn force_redraw(&mut self) {
        self.display.force_redraw();
    }

    /// Quit the editor
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Check if any buffers have unsaved modifications
    pub fn has_modified_buffers(&self) -> bool {
        self.buffers.iter().any(|b| b.is_modified())
    }

    /// Get names of all modified buffers
    pub fn modified_buffer_names(&self) -> Vec<&str> {
        self.buffers
            .iter()
            .filter(|b| b.is_modified())
            .map(|b| b.name())
            .collect()
    }

    /// Toggle the warn-unsaved setting
    pub fn toggle_warn_unsaved(&mut self) {
        self.warn_unsaved = !self.warn_unsaved;
        let status = if self.warn_unsaved {
            "Unsaved buffer warnings enabled"
        } else {
            "Unsaved buffer warnings disabled"
        };
        self.display.set_message(status);
    }

    /// Force quit without checking for unsaved buffers
    pub fn force_quit(&mut self) {
        self.pending_quit = false;
        self.running = false;
    }

    /// Force kill a buffer without checking for modifications
    pub fn force_kill_buffer(&mut self, name: &str) {
        if let Some(idx) = self.buffers.iter().position(|b| b.name() == name) {
            if self.buffers.len() <= 1 {
                self.display.set_message("Can't kill the only buffer");
                return;
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
            self.display.set_message(&format!("Killed buffer {}", name));
        }
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

    /// Get text at specific kill ring index (0 = most recent)
    pub fn yank_text_at(&self, idx: usize) -> Option<&str> {
        if self.kill_ring.is_empty() {
            return None;
        }
        let len = self.kill_ring.len();
        let actual_idx = len.saturating_sub(1).saturating_sub(idx % len);
        self.kill_ring.get(actual_idx).map(|s| s.as_str())
    }

    /// Cycle kill ring index for yank-pop (returns new index)
    pub fn cycle_kill_ring(&mut self) -> usize {
        if self.kill_ring.is_empty() {
            return 0;
        }
        self.kill_ring_idx = (self.kill_ring_idx + 1) % self.kill_ring.len();
        self.kill_ring_idx
    }

    /// Reset kill ring index to most recent
    pub fn reset_kill_ring_idx(&mut self) {
        self.kill_ring_idx = 0;
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

    /// Hunt - repeat last search in specified direction without prompting
    pub fn hunt(&mut self, direction: SearchDirection) -> bool {
        if self.search.pattern.is_empty() {
            self.display.set_message("No previous search pattern");
            return false;
        }

        let pattern = self.search.pattern.clone();
        let start_line = self.current_window().cursor_line();
        let start_col = self.current_window().cursor_col();
        let line_count = self.current_buffer().line_count();

        match direction {
            SearchDirection::Forward => {
                // Search forward from current position (skip current match)
                let search_start_col = start_col + 1;
                for line_idx in start_line..line_count {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let col_start = if line_idx == start_line { search_start_col } else { 0 };
                        if col_start < text.len() {
                            if let Some(pos) = text[col_start..].find(&pattern) {
                                let match_col = col_start + pos;
                                self.current_window_mut().set_cursor(line_idx, match_col);
                                self.ensure_cursor_visible();
                                self.display.set_message(&format!("Found: {}", pattern));
                                return true;
                            }
                        }
                    }
                }
                // Wrap around to beginning
                for line_idx in 0..=start_line {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let col_end = if line_idx == start_line { start_col } else { text.len() };
                        if let Some(pos) = text[..col_end].find(&pattern) {
                            self.current_window_mut().set_cursor(line_idx, pos);
                            self.ensure_cursor_visible();
                            self.display.set_message(&format!("Wrapped: {}", pattern));
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
                        let col_end = if line_idx == start_line { start_col } else { text.len() };
                        if let Some(pos) = text[..col_end].rfind(&pattern) {
                            self.current_window_mut().set_cursor(line_idx, pos);
                            self.ensure_cursor_visible();
                            self.display.set_message(&format!("Found: {}", pattern));
                            return true;
                        }
                    }
                }
                // Wrap around to end
                for line_idx in (start_line..line_count).rev() {
                    if let Some(line) = self.current_buffer().line(line_idx) {
                        let text = line.text();
                        let col_start = if line_idx == start_line { start_col + 1 } else { 0 };
                        if col_start < text.len() {
                            if let Some(pos) = text[col_start..].rfind(&pattern) {
                                let match_col = col_start + pos;
                                self.current_window_mut().set_cursor(line_idx, match_col);
                                self.ensure_cursor_visible();
                                self.display.set_message(&format!("Wrapped: {}", pattern));
                                return true;
                            }
                        }
                    }
                }
            }
        }

        self.display.set_message(&format!("Not found: {}", pattern));
        false
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
                    // Check if modified and warning is enabled
                    if self.warn_unsaved && self.buffers[idx].is_modified() {
                        // Prompt for confirmation
                        self.prompt.active = true;
                        self.prompt.prompt = format!("Buffer {} modified; kill anyway? (y/n) ", input);
                        self.prompt.input.clear();
                        self.prompt.action = PromptAction::ConfirmKillBuffer;
                        self.prompt.default = Some(input);
                        self.update_prompt_display();
                        return Ok(());
                    }
                    // Kill the buffer
                    self.force_kill_buffer(&input);
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
            PromptAction::ReplaceStringSearch => {
                if input.is_empty() {
                    self.display.set_message("No search string");
                    return Ok(());
                }
                // Store search string and prompt for replacement
                self.query_replace.search = input.clone();
                self.prompt.active = true;
                self.prompt.prompt = format!("Replace {} with: ", input);
                self.prompt.input.clear();
                self.prompt.action = PromptAction::ReplaceStringReplace;
                self.prompt.default = None;
                self.update_prompt_display();
            }
            PromptAction::ReplaceStringReplace => {
                // Perform all replacements without prompting
                let count = self.replace_all_occurrences(&self.query_replace.search.clone(), &input);
                self.display.set_message(&format!("Replaced {} occurrences", count));
            }
            PromptAction::ShellCommand => {
                if input.is_empty() {
                    return Ok(());
                }
                self.execute_shell_command(&input);
            }
            PromptAction::InsertFile => {
                if input.is_empty() {
                    self.display.set_message("No file name");
                    return Ok(());
                }
                self.insert_file(&input);
            }
            PromptAction::FilterBuffer => {
                if input.is_empty() {
                    return Ok(());
                }
                self.filter_buffer(&input);
            }
            PromptAction::FilterRegion => {
                if input.is_empty() {
                    return Ok(());
                }
                self.filter_region(&input, false);
            }
            PromptAction::FilterRegionReplace => {
                if input.is_empty() {
                    return Ok(());
                }
                self.filter_region(&input, true);
            }
            PromptAction::WriteFile => {
                if input.is_empty() {
                    self.display.set_message("No file name");
                    return Ok(());
                }
                self.write_file(&input);
            }
            PromptAction::ExtendedCommand => {
                if input.is_empty() {
                    return Ok(());
                }
                self.execute_named_command(&input);
            }
            PromptAction::ConfirmQuit => {
                let input_lower = input.to_lowercase();
                if input_lower == "y" || input_lower == "yes" {
                    self.force_quit();
                } else {
                    self.display.set_message("Quit cancelled");
                }
            }
            PromptAction::ConfirmKillBuffer => {
                let input_lower = input.to_lowercase();
                if input_lower == "y" || input_lower == "yes" {
                    // Get the buffer name from the stored prompt default
                    if let Some(ref buf_name) = self.prompt.default {
                        let buf_name = buf_name.clone();
                        self.force_kill_buffer(&buf_name);
                    }
                } else {
                    self.display.set_message("Buffer not killed");
                }
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

    /// Enlarge current window by n lines (taking from adjacent window)
    pub fn enlarge_window(&mut self, n: u16) -> bool {
        if self.windows.len() < 2 {
            return false;
        }

        // Find adjacent window to take space from (prefer below, else above)
        let other_idx = if self.current_window + 1 < self.windows.len() {
            self.current_window + 1
        } else {
            self.current_window - 1
        };

        let other_height = self.windows[other_idx].height();
        if other_height <= n + 1 {
            // Other window would become too small (need at least 2 lines)
            return false;
        }

        // Adjust heights
        let current_height = self.windows[self.current_window].height();
        self.windows[self.current_window].set_height(current_height + n);
        self.windows[other_idx].set_height(other_height - n);

        self.recalculate_window_positions();
        self.display.force_redraw();
        true
    }

    /// Shrink current window by n lines (giving to adjacent window)
    pub fn shrink_window(&mut self, n: u16) -> bool {
        if self.windows.len() < 2 {
            return false;
        }

        let current_height = self.windows[self.current_window].height();
        if current_height <= n + 1 {
            // Current window would become too small (need at least 2 lines)
            return false;
        }

        // Find adjacent window to give space to (prefer below, else above)
        let other_idx = if self.current_window + 1 < self.windows.len() {
            self.current_window + 1
        } else {
            self.current_window - 1
        };

        // Adjust heights
        let other_height = self.windows[other_idx].height();
        self.windows[self.current_window].set_height(current_height - n);
        self.windows[other_idx].set_height(other_height + n);

        // Make sure cursor is still visible in current window
        self.current_window_mut().ensure_cursor_visible();

        self.recalculate_window_positions();
        self.display.force_redraw();
        true
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
            let end_col = (col + search_len).min(line.len());
            line.delete_range(col, end_col);
            // Insert the replacement - track byte offset for multi-byte chars
            let mut byte_offset = col;
            for ch in replace_str.chars() {
                line.insert_char(byte_offset, ch);
                byte_offset += ch.len_utf8();
            }
        }

        self.current_buffer_mut().set_modified(true);
        self.query_replace.count += 1;

        // Move cursor past the replacement (use byte length)
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

    /// Replace all occurrences of search with replace in current buffer (non-interactive)
    pub fn replace_all_occurrences(&mut self, search: &str, replace: &str) -> usize {
        if search.is_empty() {
            return 0;
        }

        let mut count = 0;
        let line_count = self.current_buffer().line_count();

        // Process each line
        for line_idx in 0..line_count {
            loop {
                // Find next occurrence in this line
                let found = if let Some(line) = self.current_buffer().line(line_idx) {
                    line.text().find(search)
                } else {
                    None
                };

                if let Some(col) = found {
                    // Perform replacement
                    if let Some(line) = self.current_buffer_mut().line_mut(line_idx) {
                        let end_col = col + search.len();
                        line.delete_range(col, end_col);
                        let mut byte_offset = col;
                        for ch in replace.chars() {
                            line.insert_char(byte_offset, ch);
                            byte_offset += ch.len_utf8();
                        }
                    }
                    count += 1;
                } else {
                    break;
                }
            }
        }

        if count > 0 {
            self.current_buffer_mut().set_modified(true);
        }

        count
    }

    /// Create or update the help buffer showing all key bindings
    pub fn describe_bindings(&mut self) {
        use crate::input::Key;

        // Generate bindings content
        let mut content = String::new();
        content.push_str("Key Bindings\n");
        content.push_str("============\n\n");

        // Get all bindings from keytab
        let bindings = self.keytab.all_bindings();

        // Group by command name for easier reading
        let mut prev_name = "";
        for (code, name) in &bindings {
            // Add blank line between different commands
            if !prev_name.is_empty() && *name != prev_name {
                // Only add newline for visual grouping occasionally
            }
            prev_name = name;

            let key = Key(*code);
            content.push_str(&format!("{:<20} {}\n", key.display_name(), name));
        }

        content.push_str(&format!("\n{} bindings total\n", bindings.len()));

        // Find or create the *Help* buffer
        let help_buf_name = "*Help*";
        if let Some(idx) = self.buffers.iter().position(|b| b.name() == help_buf_name) {
            // Update existing buffer
            self.buffers[idx].set_content(&content);
            // Switch to it
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        } else {
            // Create new buffer
            let buffer = Buffer::from_content(help_buf_name, &content);
            self.buffers.push(buffer);
            let idx = self.buffers.len() - 1;
            if let Some(window) = self.windows.get_mut(self.current_window) {
                window.set_buffer_idx(idx);
                window.set_cursor(0, 0);
            }
        }

        self.display.force_redraw();
        self.display.set_message("Type C-x b to return to previous buffer");
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

    /// Execute a command by name (M-x)
    pub fn execute_named_command(&mut self, name: &str) {
        // Look up the command by name
        let cmd_fn = self.keytab.lookup_by_name(name);

        if let Some(cmd_fn) = cmd_fn {
            // Execute with default arguments (no prefix, count=1)
            match cmd_fn(self, false, 1) {
                Ok(_) => {}
                Err(e) => {
                    self.display.set_message(&format!("Error: {}", e));
                }
            }
        } else {
            // Try to find partial matches for a helpful message
            let names = self.keytab.command_names();
            let matches: Vec<&str> = names.iter()
                .filter(|n| n.contains(name))
                .take(3)
                .copied()
                .collect();

            if matches.is_empty() {
                self.display.set_message(&format!("Unknown command: {}", name));
            } else {
                self.display.set_message(&format!(
                    "Unknown command: {}. Did you mean: {}?",
                    name,
                    matches.join(", ")
                ));
            }
        }
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

    /// Save current macro to a numbered slot (0-9)
    pub fn save_macro_to_slot(&mut self, slot: usize) {
        if slot > 9 {
            self.display.set_message("Invalid macro slot (use 0-9)");
            return;
        }
        if self.macro_state.keys.is_empty() {
            self.display.set_message("No macro to save");
            return;
        }
        self.macro_state.slots[slot] = self.macro_state.keys.clone();
        self.display.set_message(&format!("Macro saved to slot {}", slot));
    }

    /// Load macro from a numbered slot (0-9) into current macro
    pub fn load_macro_from_slot(&mut self, slot: usize) {
        if slot > 9 {
            self.display.set_message("Invalid macro slot (use 0-9)");
            return;
        }
        if self.macro_state.slots[slot].is_empty() {
            self.display.set_message(&format!("No macro in slot {}", slot));
            return;
        }
        self.macro_state.keys = self.macro_state.slots[slot].clone();
        let key_count = self.macro_state.keys.len();
        self.display.set_message(&format!("Loaded macro from slot {} ({} keys)", slot, key_count));
    }

    /// Execute macro from a numbered slot (0-9)
    pub fn execute_macro_slot(&mut self, slot: usize) -> Result<()> {
        if slot > 9 {
            self.display.set_message("Invalid macro slot (use 0-9)");
            return Ok(());
        }
        if self.macro_state.playing {
            return Ok(());
        }
        if self.macro_state.recording {
            self.display.set_message("Can't execute macro while defining it");
            return Ok(());
        }
        if self.macro_state.slots[slot].is_empty() {
            self.display.set_message(&format!("No macro in slot {}", slot));
            return Ok(());
        }

        let keys: Vec<Key> = self.macro_state.slots[slot].clone();

        self.macro_state.playing = true;
        for key in keys {
            self.handle_key(key)?;
            if !self.running {
                break;
            }
        }
        self.macro_state.playing = false;

        Ok(())
    }

    /// Write buffer to a new filename (Save As)
    pub fn write_file(&mut self, filename: &str) {
        use std::fs::File;
        use std::io::Write;
        use std::path::PathBuf;

        let path = PathBuf::from(filename);

        // Collect buffer content
        let mut content = String::new();
        let line_count = self.current_buffer().line_count();
        for i in 0..line_count {
            if let Some(line) = self.current_buffer().line(i) {
                content.push_str(line.text());
                if i + 1 < line_count {
                    content.push('\n');
                }
            }
        }

        match File::create(&path) {
            Ok(mut file) => {
                match file.write_all(content.as_bytes()) {
                    Ok(()) => {
                        // Delete old auto-save file if there was a previous filename
                        if let Some(old_path) = self.current_buffer().filename() {
                            self.delete_auto_save_file(old_path);
                        }
                        // Update buffer's filename and clear modified flag
                        self.current_buffer_mut().set_filename(path.clone());
                        self.current_buffer_mut().set_modified(false);
                        // Delete auto-save file for new path too
                        self.delete_auto_save_file(&path);
                        // Update buffer name to match new filename
                        let name = path.file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| filename.to_string());
                        self.current_buffer_mut().set_name(&name);
                        self.display.set_message(&format!(
                            "Wrote {} lines to {}",
                            line_count, filename
                        ));
                    }
                    Err(e) => {
                        self.display.set_message(&format!("Error writing file: {}", e));
                    }
                }
            }
            Err(e) => {
                self.display.set_message(&format!("Cannot create {}: {}", filename, e));
            }
        }
    }

    /// Insert file contents at current cursor position
    pub fn insert_file(&mut self, filename: &str) {
        use std::fs;
        use std::path::Path;

        let path = Path::new(filename);
        match fs::read_to_string(path) {
            Ok(content) => {
                let line_count_before = self.current_buffer().line_count();

                // Insert the content at cursor position
                for ch in content.chars() {
                    if ch == '\n' {
                        let cursor_line = self.current_window().cursor_line();
                        let cursor_col = self.current_window().cursor_col();
                        self.current_buffer_mut()
                            .insert_newline(cursor_line, cursor_col);
                        self.current_window_mut().set_cursor(cursor_line + 1, 0);
                    } else if ch != '\r' {
                        // Skip carriage returns (handle Windows line endings)
                        self.insert_char(ch);
                    }
                }

                let lines_inserted = self.current_buffer().line_count() - line_count_before;
                self.current_buffer_mut().set_modified(true);
                self.ensure_cursor_visible();
                self.display.set_message(&format!(
                    "Inserted {} lines from {}",
                    lines_inserted, filename
                ));
            }
            Err(e) => {
                self.display.set_message(&format!("Error reading {}: {}", filename, e));
            }
        }
    }

    /// Filter buffer contents through a shell command
    pub fn filter_buffer(&mut self, command: &str) {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Collect buffer content
        let mut content = String::new();
        let line_count = self.current_buffer().line_count();
        for i in 0..line_count {
            if let Some(line) = self.current_buffer().line(i) {
                content.push_str(line.text());
                if i + 1 < line_count {
                    content.push('\n');
                }
            }
        }

        // Run command with buffer as stdin
        #[cfg(windows)]
        let result = Command::new("cmd")
            .args(["/C", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        #[cfg(not(windows))]
        let result = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                // Write content to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }

                // Wait for output
                match child.wait_with_output() {
                    Ok(output) => {
                        let new_content = String::from_utf8_lossy(&output.stdout);

                        // Replace buffer content
                        self.current_buffer_mut().set_content(&new_content);
                        self.current_window_mut().set_cursor(0, 0);
                        self.current_window_mut().set_top_line(0);
                        self.current_buffer_mut().set_modified(true);
                        self.display.force_redraw();

                        let new_line_count = self.current_buffer().line_count();
                        self.display.set_message(&format!(
                            "Filter complete: {} lines",
                            new_line_count
                        ));
                    }
                    Err(e) => {
                        self.display.set_message(&format!("Filter error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.display.set_message(&format!("Failed to run command: {}", e));
            }
        }
    }

    /// Filter region through shell command (M-|)
    /// If replace is true, replaces the region with output
    /// If replace is false, shows output in *Shell Command Output* buffer
    pub fn filter_region(&mut self, command: &str, replace: bool) {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Get the stored region
        let region = match self.filter_region.take() {
            Some(r) => r,
            None => {
                self.display.set_message("No region stored");
                return;
            }
        };

        let (start_line, start_col, end_line, end_col) = region;

        // Collect region content
        let mut content = String::new();
        for line_idx in start_line..=end_line {
            if let Some(line) = self.current_buffer().line(line_idx) {
                let text = line.text();
                let line_start = if line_idx == start_line {
                    // Convert char index to byte index
                    text.char_indices()
                        .nth(start_col)
                        .map(|(i, _)| i)
                        .unwrap_or(text.len())
                } else {
                    0
                };
                let line_end = if line_idx == end_line {
                    text.char_indices()
                        .nth(end_col)
                        .map(|(i, _)| i)
                        .unwrap_or(text.len())
                } else {
                    text.len()
                };

                if line_start <= line_end && line_end <= text.len() {
                    content.push_str(&text[line_start..line_end]);
                }
                if line_idx < end_line {
                    content.push('\n');
                }
            }
        }

        // Run command with region as stdin
        #[cfg(windows)]
        let result = Command::new("cmd")
            .args(["/C", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        #[cfg(not(windows))]
        let result = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                // Write content to stdin
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(content.as_bytes());
                }

                // Wait for output
                match child.wait_with_output() {
                    Ok(output) => {
                        let output_text = String::from_utf8_lossy(&output.stdout);

                        if replace {
                            // Delete the region (following kill_region pattern)
                            self.current_buffer_mut().add_undo_boundary();

                            // Move cursor to start of region
                            self.current_window_mut().set_cursor(start_line, start_col);

                            // Delete the region
                            if start_line == end_line {
                                // Same line - simple case
                                if let Some(line) = self.current_buffer_mut().line_mut(start_line) {
                                    line.delete_range(start_col, end_col);
                                }
                            } else {
                                // Multi-line deletion
                                // Delete from start_col to end of start_line
                                if let Some(line) = self.current_buffer_mut().line_mut(start_line) {
                                    let line_len = line.len();
                                    if start_col < line_len {
                                        line.delete_range(start_col, line_len);
                                    }
                                }

                                // Delete intermediate lines (in reverse to avoid index shifting)
                                for line_idx in (start_line + 1..end_line).rev() {
                                    self.current_buffer_mut().delete_line(line_idx);
                                }

                                // Handle the end line - take remaining content after end_col
                                if end_line > start_line {
                                    // The end line is now at start_line + 1 (after removing intermediates)
                                    let remaining_line_idx = start_line + 1;
                                    if let Some(end_line_content) = self.current_buffer().line(remaining_line_idx) {
                                        let remaining = if end_col < end_line_content.len() {
                                            end_line_content.text()[end_col..].to_string()
                                        } else {
                                            String::new()
                                        };

                                        // Append remaining to start line and remove the end line
                                        if let Some(start_line_ref) = self.current_buffer_mut().line_mut(start_line) {
                                            start_line_ref.append_str(&remaining);
                                        }
                                        self.current_buffer_mut().delete_line(remaining_line_idx);
                                    }
                                }
                            }

                            // Insert the output at the cursor position
                            for ch in output_text.chars() {
                                if ch == '\n' {
                                    let cursor_line = self.current_window().cursor_line();
                                    let cursor_col = self.current_window().cursor_col();
                                    self.current_buffer_mut().insert_newline(cursor_line, cursor_col);
                                    self.current_window_mut().set_cursor(cursor_line + 1, 0);
                                } else {
                                    let cursor_line = self.current_window().cursor_line();
                                    let cursor_col = self.current_window().cursor_col();
                                    self.current_buffer_mut().insert_char(cursor_line, cursor_col, ch);
                                    // Advance cursor by 1 char
                                    let new_col = cursor_col + ch.len_utf8();
                                    self.current_window_mut().set_cursor(cursor_line, new_col);
                                }
                            }

                            self.current_buffer_mut().set_modified(true);
                            self.display.force_redraw();
                            self.display.set_message(&format!("Region filtered through '{}'", command));
                        } else {
                            // Show output in *Shell Command Output* buffer
                            let buf_name = "*Shell Command Output*";
                            if let Some(idx) = self.buffers.iter().position(|b| b.name() == buf_name) {
                                self.buffers[idx].set_content(&output_text);
                                if let Some(window) = self.windows.get_mut(self.current_window) {
                                    window.set_buffer_idx(idx);
                                    window.set_cursor(0, 0);
                                }
                            } else {
                                let buffer = Buffer::from_content(buf_name, &output_text);
                                self.buffers.push(buffer);
                                let idx = self.buffers.len() - 1;
                                if let Some(window) = self.windows.get_mut(self.current_window) {
                                    window.set_buffer_idx(idx);
                                    window.set_cursor(0, 0);
                                }
                            }
                            self.display.force_redraw();
                            self.display.set_message(&format!("Shell command on region: {}", command));
                        }
                    }
                    Err(e) => {
                        self.display.set_message(&format!("Filter error: {}", e));
                    }
                }
            }
            Err(e) => {
                self.display.set_message(&format!("Failed to run command: {}", e));
            }
        }
    }

    /// Generate auto-save filename for a buffer
    /// Emacs style: /path/to/file.txt -> /path/to/#file.txt#
    pub fn auto_save_path(path: &PathBuf) -> PathBuf {
        if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name() {
                let auto_name = format!("#{}", filename.to_string_lossy());
                return parent.join(format!("{}#", auto_name));
            }
        }
        // Fallback: just add # around the whole path
        PathBuf::from(format!("#{}#", path.display()))
    }

    /// Check if it's time to auto-save and do it if needed
    pub fn check_auto_save(&mut self) {
        if !self.auto_save_enabled {
            return;
        }

        let now = Instant::now();
        if now.duration_since(self.last_auto_save) < self.auto_save_interval {
            return;
        }

        self.last_auto_save = now;
        self.do_auto_save();
    }

    /// Perform auto-save on all modified buffers with filenames
    fn do_auto_save(&mut self) {
        let mut saved_count = 0;

        for buffer in &self.buffers {
            // Only auto-save modified buffers that have a filename
            // Skip special buffers (names starting with *)
            if !buffer.is_modified() {
                continue;
            }
            if buffer.name().starts_with('*') {
                continue;
            }
            if let Some(path) = buffer.filename() {
                let auto_path = Self::auto_save_path(path);
                if buffer.write_to(&auto_path).is_ok() {
                    saved_count += 1;
                }
            }
        }

        if saved_count > 0 {
            self.display.set_message(&format!("Auto-saved {} buffer(s)", saved_count));
        }
    }

    /// Delete auto-save file for a buffer (called after successful save)
    pub fn delete_auto_save_file(&self, path: &PathBuf) {
        let auto_path = Self::auto_save_path(path);
        let _ = std::fs::remove_file(auto_path);
    }

    /// Toggle auto-save on/off
    pub fn toggle_auto_save(&mut self) {
        self.auto_save_enabled = !self.auto_save_enabled;
        if self.auto_save_enabled {
            self.display.set_message("Auto-save enabled");
            self.last_auto_save = Instant::now();
        } else {
            self.display.set_message("Auto-save disabled");
        }
    }
}
