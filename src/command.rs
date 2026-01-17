//! Command dispatch system

use std::collections::HashMap;

use crate::editor::EditorState;
use crate::error::Result;
use crate::input::Key;

/// Command result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Success,
    Failure,
    Abort,
}

/// Command function signature
/// - editor: mutable reference to editor state
/// - f: true if numeric argument was provided
/// - n: numeric argument (repeat count, default 1)
pub type CommandFn = fn(&mut EditorState, bool, i32) -> Result<CommandStatus>;

/// A named command
pub struct Command {
    pub name: &'static str,
    pub function: CommandFn,
}

/// Key binding entry with command function and name
struct BindingEntry {
    function: CommandFn,
    name: &'static str,
}

/// Key binding table
pub struct KeyTable {
    bindings: HashMap<u32, BindingEntry>,
}

impl KeyTable {
    /// Create an empty key table
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Create key table with default bindings
    pub fn with_defaults() -> Self {
        let mut table = Self::new();
        table.setup_defaults();
        table
    }

    /// Add a key binding with command name
    pub fn bind_named(&mut self, key: Key, cmd: CommandFn, name: &'static str) {
        self.bindings.insert(key.code(), BindingEntry { function: cmd, name });
    }

    /// Add a key binding (uses function pointer as identifier)
    pub fn bind(&mut self, key: Key, cmd: CommandFn) {
        // For backwards compatibility, derive name from function if not specified
        self.bindings.insert(key.code(), BindingEntry { function: cmd, name: "unknown" });
    }

    /// Remove a key binding
    pub fn unbind(&mut self, key: Key) {
        self.bindings.remove(&key.code());
    }

    /// Look up a command for a key
    pub fn lookup(&self, key: Key) -> Option<CommandFn> {
        self.bindings.get(&key.code()).map(|e| e.function)
    }

    /// Look up a command name for a key
    pub fn lookup_name(&self, key: Key) -> Option<&'static str> {
        self.bindings.get(&key.code()).map(|e| e.name)
    }

    /// Get all bindings as (key_code, command_name) pairs
    pub fn all_bindings(&self) -> Vec<(u32, &'static str)> {
        let mut bindings: Vec<_> = self.bindings
            .iter()
            .map(|(&code, entry)| (code, entry.name))
            .collect();
        // Sort by command name for easier reading
        bindings.sort_by(|a, b| a.1.cmp(b.1));
        bindings
    }

    /// Set up default key bindings
    fn setup_defaults(&mut self) {
        use commands::*;
        use crate::input::key_flags;

        // Basic cursor movement
        self.bind_named(Key::ctrl('f'), forward_char, "forward-char");
        self.bind_named(Key::ctrl('b'), backward_char, "backward-char");
        self.bind_named(Key::ctrl('n'), next_line, "next-line");
        self.bind_named(Key::ctrl('p'), previous_line, "previous-line");
        self.bind_named(Key::ctrl('a'), beginning_of_line, "beginning-of-line");
        self.bind_named(Key::ctrl('e'), end_of_line, "end-of-line");
        self.bind_named(Key::meta('m'), back_to_indentation, "back-to-indentation"); // M-m

        // Page movement
        self.bind_named(Key::ctrl('v'), scroll_down, "scroll-down");
        self.bind_named(Key::meta('v'), scroll_up, "scroll-up");
        self.bind_named(Key::meta('<'), beginning_of_buffer, "beginning-of-buffer");
        self.bind_named(Key::meta('>'), end_of_buffer, "end-of-buffer");

        // Arrow keys (special keys)
        self.bind_named(Key::special(0x4d), forward_char, "forward-char");  // Right
        self.bind_named(Key::special(0x4b), backward_char, "backward-char"); // Left
        self.bind_named(Key::special(0x50), next_line, "next-line");     // Down
        self.bind_named(Key::special(0x48), previous_line, "previous-line"); // Up
        self.bind_named(Key::special(0x49), scroll_up, "scroll-up");     // PageUp
        self.bind_named(Key::special(0x51), scroll_down, "scroll-down");   // PageDown
        self.bind_named(Key::special(0x47), beginning_of_line, "beginning-of-line"); // Home
        self.bind_named(Key::special(0x4f), end_of_line, "end-of-line");       // End

        // Screen refresh
        self.bind_named(Key::ctrl('l'), redraw_display, "redraw-display");

        // Quit
        self.bind_named(Key::ctlx_ctrl('c'), quit, "save-buffers-kill-emacs");

        // Abort
        self.bind_named(Key::ctrl('g'), abort, "keyboard-quit");

        // Editing commands
        self.bind_named(Key::ctrl('d'), delete_char_forward, "delete-char");
        self.bind_named(Key::special(0x53), delete_char_forward, "delete-char"); // Delete key
        self.bind_named(Key(0x7f), delete_char_backward, "delete-backward-char");         // Backspace
        self.bind_named(Key::ctrl('h'), delete_char_backward, "delete-backward-char");    // C-h also backspace
        self.bind_named(Key::meta('z'), zap_to_char, "zap-to-char"); // M-z

        self.bind_named(Key::ctrl('k'), kill_line, "kill-line");
        self.bind_named(Key::ctrl('y'), yank, "yank");
        self.bind_named(Key::meta('y'), yank_pop, "yank-pop");

        self.bind_named(Key::ctrl('m'), newline, "newline");    // Enter
        self.bind_named(Key::ctrl('o'), open_line, "open-line");
        self.bind_named(Key::ctrl('j'), indent_newline, "newline-and-indent");
        self.bind_named(Key::ctrl('i'), insert_tab, "tab-to-tab-stop"); // Tab

        self.bind_named(Key::ctrl('t'), transpose_chars, "transpose-chars");
        self.bind_named(Key::ctrl('q'), quote_char, "quoted-insert");

        // File operations
        self.bind_named(Key::ctlx_ctrl('s'), save_buffer, "save-buffer");
        self.bind_named(Key::ctlx_ctrl('w'), write_file, "write-file"); // C-x C-w
        self.bind_named(Key::ctlx('i'), insert_file, "insert-file"); // C-x i
        self.bind_named(Key::meta('~'), not_modified, "not-modified"); // M-~
        self.bind_named(Key::ctlx_ctrl('q'), toggle_read_only, "toggle-read-only"); // C-x C-q
        self.bind_named(Key::ctlx_ctrl('r'), revert_buffer, "revert-buffer"); // C-x C-r
        self.bind_named(Key::ctlx('a'), toggle_auto_save, "auto-save-mode"); // C-x a

        // Line operations
        self.bind_named(Key::ctlx_ctrl('k'), copy_line, "copy-line"); // C-x C-k
        self.bind_named(Key::ctlx('d'), duplicate_line, "duplicate-line"); // C-x d

        // Word operations
        self.bind_named(Key::meta('f'), forward_word, "forward-word");
        self.bind_named(Key::meta('b'), backward_word, "backward-word");
        self.bind_named(Key::meta('d'), kill_word, "kill-word");
        // M-Backspace for backward kill word
        self.bind_named(Key(0x2000_007f), backward_kill_word, "backward-kill-word"); // META | 0x7f

        // Paragraph operations
        self.bind_named(Key::meta('{'), backward_paragraph, "backward-paragraph");
        self.bind_named(Key::meta('}'), forward_paragraph, "forward-paragraph");
        self.bind_named(Key::meta('q'), fill_paragraph, "fill-paragraph");

        // Mark/Region operations
        self.bind_named(Key::ctrl(' '), set_mark, "set-mark-command");  // C-space
        self.bind_named(Key::ctrl('w'), kill_region, "kill-region");
        self.bind_named(Key::meta('w'), copy_region, "kill-ring-save");

        // Search
        self.bind_named(Key::ctrl('s'), search_forward, "isearch-forward");
        self.bind_named(Key::ctrl('r'), search_backward, "isearch-backward");
        self.bind_named(Key::meta('s'), hunt_forward, "hunt-forward");  // M-s
        self.bind_named(Key::meta('S'), hunt_backward, "hunt-backward");  // M-S (Meta-Shift-s)
        self.bind_named(Key::meta('%'), query_replace, "query-replace");  // M-%
        self.bind_named(Key::meta('r'), replace_string, "replace-string");  // M-r

        // Buffer operations
        self.bind_named(Key::ctlx_ctrl('f'), find_file, "find-file");
        self.bind_named(Key::ctlx('b'), switch_buffer, "switch-to-buffer");
        self.bind_named(Key::ctlx_ctrl('b'), list_buffers, "list-buffers");  // C-x C-b
        self.bind_named(Key::ctlx('k'), kill_buffer, "kill-buffer");
        self.bind_named(Key::ctlx('n'), next_buffer, "next-buffer");  // C-x n
        self.bind_named(Key::ctlx('p'), previous_buffer, "previous-buffer");  // C-x p

        // Go to line
        self.bind_named(Key::meta('g'), goto_line, "goto-line");

        // Window operations
        self.bind_named(Key::ctlx('2'), split_window, "split-window-below");
        self.bind_named(Key::ctlx('1'), delete_other_windows, "delete-other-windows");
        self.bind_named(Key::ctlx('0'), delete_window, "delete-window");
        self.bind_named(Key::ctlx('o'), other_window, "other-window");
        self.bind_named(Key::ctlx('^'), enlarge_window, "enlarge-window");  // C-x ^
        self.bind_named(Key::ctlx('v'), shrink_window, "shrink-window");  // C-x v

        // Undo
        self.bind_named(Key::ctrl('/'), undo, "undo");  // C-/
        self.bind_named(Key::ctrl('_'), undo, "undo");  // C-_ (same as C-/ in many terminals)

        // Shell
        self.bind_named(Key::meta('!'), shell_command, "shell-command");  // M-!
        self.bind_named(Key::meta('|'), shell_command_on_region, "shell-command-on-region"); // M-|
        self.bind_named(Key::ctlx('|'), filter_buffer, "filter-buffer"); // C-x |

        // Keyboard macros
        self.bind_named(Key::ctlx('('), start_macro, "kmacro-start-macro");    // C-x (
        self.bind_named(Key::ctlx(')'), end_macro, "kmacro-end-macro");      // C-x )
        self.bind_named(Key::ctlx('e'), execute_macro, "kmacro-end-and-call-macro");  // C-x e
        self.bind_named(Key::ctlx_meta('s'), store_macro, "store-kbd-macro"); // C-x M-s
        self.bind_named(Key::ctlx_meta('l'), load_macro, "load-kbd-macro"); // C-x M-l

        // Case operations
        self.bind_named(Key::meta('u'), upcase_word, "upcase-word");       // M-u
        self.bind_named(Key::meta('l'), downcase_word, "downcase-word");     // M-l
        self.bind_named(Key::meta('c'), capitalize_word, "capitalize-word");   // M-c
        self.bind_named(Key::ctlx_ctrl('u'), upcase_region, "upcase-region");   // C-x C-u
        self.bind_named(Key::ctlx_ctrl('l'), downcase_region, "downcase-region"); // C-x C-l

        // Swap mark and point
        self.bind_named(Key::ctlx_ctrl('x'), exchange_point_and_mark, "exchange-point-and-mark"); // C-x C-x

        // Buffer position info
        self.bind_named(Key::ctlx('='), what_cursor_position, "what-cursor-position"); // C-x =

        // Whitespace operations
        self.bind_named(Key::meta(' '), just_one_space, "just-one-space"); // M-SPC
        self.bind_named(Key::meta('\\'), delete_horizontal_space, "delete-horizontal-space"); // M-\
        self.bind_named(Key::ctlx_ctrl('o'), delete_blank_lines, "delete-blank-lines"); // C-x C-o

        // Indentation
        self.bind_named(Key::meta('i'), tab_to_tab_stop, "tab-to-tab-stop"); // M-i

        // Help (M-? since C-h is backspace in uEmacs)
        self.bind_named(Key::meta('?'), describe_key, "describe-key"); // M-?
        self.bind_named(Key::special(0x3b), describe_bindings, "describe-bindings"); // F1

        // Statistics
        self.bind_named(Key::meta('='), word_count, "count-words"); // M-=

        // Navigation
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'f' as u32), goto_matching_fence, "goto-matching-fence"); // M-C-f

        // Whitespace cleanup
        self.bind_named(Key::ctlx('t'), trim_line, "trim-line"); // C-x t

        // Join/delete-indentation
        self.bind_named(Key::meta('^'), join_line, "delete-indentation"); // M-^

        // Scroll other window
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'v' as u32), scroll_other_window, "scroll-other-window"); // M-C-v

        // Information
        self.bind_named(Key::ctlx('l'), what_line, "what-line"); // C-x l

        // Transpose operations
        self.bind_named(Key::meta('t'), transpose_words, "transpose-words"); // M-t
        self.bind_named(Key::ctlx_ctrl('t'), transpose_lines, "transpose-lines"); // C-x C-t

        // Mark operations
        self.bind_named(Key::meta('h'), mark_paragraph, "mark-paragraph"); // M-h
        self.bind_named(Key::ctlx('h'), mark_whole_buffer, "mark-whole-buffer"); // C-x h
        self.bind_named(Key::meta('@'), mark_word, "mark-word"); // M-@

        // Kill operations
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'k' as u32), kill_paragraph, "kill-paragraph"); // M-C-k
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'w' as u32), append_next_kill, "append-next-kill"); // M-C-w

        // Line splitting
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'o' as u32), split_line, "split-line"); // M-C-o

        // Indentation
        self.bind_named(Key::ctlx_ctrl('i'), indent_rigidly, "indent-rigidly"); // C-x TAB (C-x C-i)
    }
}

impl Default for KeyTable {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Commands module - actual command implementations
pub mod commands {
    use super::*;

    /// Move cursor forward one character
    pub fn forward_char(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs() {
            if n > 0 {
                editor.move_cursor_right();
            } else {
                editor.move_cursor_left();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move cursor backward one character
    pub fn backward_char(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs() {
            if n > 0 {
                editor.move_cursor_left();
            } else {
                editor.move_cursor_right();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move cursor to next line
    pub fn next_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs() {
            if n > 0 {
                editor.move_cursor_down();
            } else {
                editor.move_cursor_up();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move cursor to previous line
    pub fn previous_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs() {
            if n > 0 {
                editor.move_cursor_up();
            } else {
                editor.move_cursor_down();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move cursor to beginning of line
    pub fn beginning_of_line(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.move_to_bol();
        Ok(CommandStatus::Success)
    }

    /// Move cursor to end of line
    pub fn end_of_line(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.move_to_eol();
        Ok(CommandStatus::Success)
    }

    /// Move cursor to first non-whitespace character on line (M-m)
    pub fn back_to_indentation(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();

        if let Some(line) = editor.current_buffer().line(cursor_line) {
            let text = line.text();
            let mut col = 0;

            // Find first non-whitespace character
            for (pos, ch) in text.char_indices() {
                if !ch.is_whitespace() {
                    col = pos;
                    break;
                }
                col = pos + ch.len_utf8(); // Will be at end if all whitespace
            }

            editor.current_window_mut().set_cursor(cursor_line, col);
        }

        Ok(CommandStatus::Success)
    }

    /// Scroll down (forward) one page
    pub fn scroll_down(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let pages = n.abs() as usize;
        for _ in 0..pages {
            if n > 0 {
                editor.page_down();
            } else {
                editor.page_up();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Scroll up (backward) one page
    pub fn scroll_up(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let pages = n.abs() as usize;
        for _ in 0..pages {
            if n > 0 {
                editor.page_up();
            } else {
                editor.page_down();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move to beginning of buffer
    pub fn beginning_of_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.move_to_buffer_start();
        Ok(CommandStatus::Success)
    }

    /// Move to end of buffer
    pub fn end_of_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.move_to_buffer_end();
        Ok(CommandStatus::Success)
    }

    /// Redraw the display
    /// Recenter display with cursor line in middle of window (C-l)
    pub fn redraw_display(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let height = editor.current_window().height() as usize;

        // Calculate new top line to center cursor (or use n as line from top if given)
        let new_top = if n != 1 {
            // With argument, put cursor on line n from top (0-indexed internally)
            cursor_line.saturating_sub((n - 1).max(0) as usize)
        } else {
            // Default: center cursor vertically
            cursor_line.saturating_sub(height / 2)
        };

        editor.current_window_mut().set_top_line(new_top);
        editor.force_redraw();
        Ok(CommandStatus::Success)
    }

    /// Quit the editor
    pub fn quit(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.quit();
        Ok(CommandStatus::Success)
    }

    /// Abort current operation
    pub fn abort(_editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        Ok(CommandStatus::Abort)
    }

    /// Delete character at cursor (forward)
    pub fn delete_char_forward(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return delete_char_backward(editor, f, -n);
        }

        editor.display.force_redraw();

        // Start kill if argument provided
        if f {
            editor.start_kill();
        }

        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();
            let line_len = editor
                .current_buffer()
                .line(cursor_line)
                .map(|l| l.len())
                .unwrap_or(0);

            if cursor_col < line_len {
                // Delete character on current line
                if let Some(ch) = editor.current_buffer_mut().delete_char(cursor_line, cursor_col) {
                    if f {
                        editor.kill_append(&ch.to_string());
                    }
                }
            } else if cursor_line + 1 < editor.current_buffer().line_count() {
                // At end of line - join with next line (delete newline)
                editor.current_buffer_mut().join_line(cursor_line);
                if f {
                    editor.kill_append("\n");
                }
            } else {
                return Ok(CommandStatus::Failure);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Delete character before cursor (backward)
    pub fn delete_char_backward(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return delete_char_forward(editor, f, -n);
        }

        editor.display.force_redraw();

        if f {
            editor.start_kill();
        }

        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            if cursor_col > 0 {
                // Delete character backward on current line
                if let Some((ch, new_col)) = editor
                    .current_buffer_mut()
                    .delete_backward(cursor_line, cursor_col)
                {
                    editor.current_window_mut().set_cursor(cursor_line, new_col);
                    if f {
                        editor.kill_prepend(&ch.to_string());
                    }
                }
            } else if cursor_line > 0 {
                // At start of line - join with previous line
                if let Some(join_pos) = editor.current_buffer_mut().join_with_previous(cursor_line) {
                    editor
                        .current_window_mut()
                        .set_cursor(cursor_line - 1, join_pos);
                    if f {
                        editor.kill_prepend("\n");
                    }
                }
            } else {
                return Ok(CommandStatus::Failure);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Kill to end of line
    pub fn kill_line(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        editor.start_kill();
        editor.display.force_redraw();

        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        if f && n == 0 {
            // Kill from BOL to cursor
            if let Some(line) = editor.current_buffer().line(cursor_line) {
                let killed = line.safe_slice_to(cursor_col).to_string();
                let actual_end = killed.len(); // Use actual byte length after safe slicing
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                    if actual_end > 0 {
                        line_mut.delete_range(0, actual_end);
                    }
                }
                editor.current_buffer_mut().set_modified(true);
                editor.current_window_mut().set_cursor(cursor_line, 0);
                editor.kill_append(&killed);
            }
        } else if f && n > 0 {
            // Kill n lines forward
            for _ in 0..n {
                let cursor_line = editor.current_window().cursor_line();
                if let Some(killed) = editor.current_buffer_mut().kill_to_eol(cursor_line, 0) {
                    editor.kill_append(&killed);
                }
            }
        } else {
            // No argument: kill from cursor to EOL (or newline if at EOL)
            if let Some(killed) = editor
                .current_buffer_mut()
                .kill_to_eol(cursor_line, cursor_col)
            {
                editor.kill_append(&killed);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Yank killed text
    pub fn yank(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return Ok(CommandStatus::Failure);
        }

        let text = match editor.yank_text() {
            Some(t) => t.to_string(),
            None => return Ok(CommandStatus::Success), // Nothing to yank
        };

        // Track start position for yank-pop
        let start_line = editor.current_window().cursor_line();
        let start_col = editor.current_window().cursor_col();

        for _ in 0..n.max(1) {
            for ch in text.chars() {
                if ch == '\n' {
                    let cursor_line = editor.current_window().cursor_line();
                    let cursor_col = editor.current_window().cursor_col();
                    editor
                        .current_buffer_mut()
                        .insert_newline(cursor_line, cursor_col);
                    editor.current_window_mut().set_cursor(cursor_line + 1, 0);
                } else {
                    editor.insert_char(ch);
                }
            }
        }

        // Track end position for yank-pop
        let end_line = editor.current_window().cursor_line();
        let end_col = editor.current_window().cursor_col();
        editor.last_yank_start = Some((start_line, start_col));
        editor.last_yank_end = Some((end_line, end_col));
        editor.last_was_yank = true;
        editor.reset_kill_ring_idx();

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Cycle through kill ring after yank (M-y)
    pub fn yank_pop(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Only works immediately after yank or yank-pop
        if !editor.last_was_yank {
            editor.display.set_message("Previous command was not a yank");
            return Ok(CommandStatus::Failure);
        }

        let (start_line, start_col) = match editor.last_yank_start {
            Some(pos) => pos,
            None => return Ok(CommandStatus::Failure),
        };

        let (end_line, end_col) = match editor.last_yank_end {
            Some(pos) => pos,
            None => return Ok(CommandStatus::Failure),
        };

        // Cycle to next kill ring entry
        let new_idx = editor.cycle_kill_ring();
        let new_text = match editor.yank_text_at(new_idx) {
            Some(t) => t.to_string(),
            None => return Ok(CommandStatus::Failure),
        };

        // Delete the previous yank (from start to end)
        // Work backwards to avoid index shifting issues
        for line_idx in (start_line..=end_line).rev() {
            if line_idx == start_line && line_idx == end_line {
                // Same line
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    line_mut.delete_range(start_col, end_col);
                }
            } else if line_idx == end_line {
                // End line - delete from beginning to end_col, then join
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    line_mut.delete_range(0, end_col);
                }
                if line_idx > 0 {
                    editor.current_buffer_mut().join_line(line_idx - 1);
                }
            } else if line_idx == start_line {
                // Start line - delete from start_col to end
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    let line_len = line_mut.len();
                    line_mut.delete_range(start_col, line_len);
                }
            } else {
                // Middle line - delete entire line
                editor.current_buffer_mut().delete_line(line_idx);
            }
        }

        // Move cursor to start position
        editor.current_window_mut().set_cursor(start_line, start_col);

        // Insert new text
        for ch in new_text.chars() {
            if ch == '\n' {
                let cursor_line = editor.current_window().cursor_line();
                let cursor_col = editor.current_window().cursor_col();
                editor
                    .current_buffer_mut()
                    .insert_newline(cursor_line, cursor_col);
                editor.current_window_mut().set_cursor(cursor_line + 1, 0);
            } else {
                editor.insert_char(ch);
            }
        }

        // Update end position
        let new_end_line = editor.current_window().cursor_line();
        let new_end_col = editor.current_window().cursor_col();
        editor.last_yank_end = Some((new_end_line, new_end_col));
        editor.last_was_yank = true;

        editor.current_buffer_mut().set_modified(true);
        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Insert newline and move cursor to new line
    pub fn newline(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return Ok(CommandStatus::Failure);
        }

        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            editor
                .current_buffer_mut()
                .insert_newline(cursor_line, cursor_col);
            editor.current_window_mut().set_cursor(cursor_line + 1, 0);
        }

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Open line below (insert newline, cursor stays)
    pub fn open_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n <= 0 {
            return Ok(if n == 0 {
                CommandStatus::Success
            } else {
                CommandStatus::Failure
            });
        }

        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        // Insert n newlines
        for _ in 0..n {
            editor
                .current_buffer_mut()
                .insert_newline(cursor_line, cursor_col);
        }

        // Cursor stays at original position
        editor
            .current_window_mut()
            .set_cursor(cursor_line, cursor_col);

        Ok(CommandStatus::Success)
    }

    /// Insert newline with indentation from current line
    pub fn indent_newline(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return Ok(CommandStatus::Failure);
        }

        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            // Get indentation from current line
            let indent = editor
                .current_buffer()
                .line(cursor_line)
                .map(|line| {
                    let text = line.text();
                    let indent_str: String = text
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect();
                    indent_str
                })
                .unwrap_or_default();

            // Insert newline
            editor
                .current_buffer_mut()
                .insert_newline(cursor_line, cursor_col);
            editor.current_window_mut().set_cursor(cursor_line + 1, 0);

            // Insert indentation
            for ch in indent.chars() {
                editor.insert_char(ch);
            }
        }

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Insert tab character
    pub fn insert_tab(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return Ok(CommandStatus::Failure);
        }

        for _ in 0..n.max(1) {
            editor.insert_char('\t');
        }

        Ok(CommandStatus::Success)
    }

    /// Transpose (swap) two characters
    pub fn transpose_chars(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        let line = match editor.current_buffer().line(cursor_line) {
            Some(l) => l,
            None => return Ok(CommandStatus::Failure),
        };

        let text = line.text();
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let len = chars.len();

        if len < 2 {
            return Ok(CommandStatus::Failure);
        }

        // Find the two character positions to swap
        // If at end of line, swap the two before cursor
        // Otherwise, swap char at cursor with char before
        let (idx1, idx2) = if cursor_col >= text.len() {
            // At EOL: swap last two chars
            (len - 2, len - 1)
        } else {
            // Find current char index - use <= to handle cursor in middle of multi-byte char
            let cur_idx = chars.iter().position(|(pos, ch)| {
                *pos <= cursor_col && cursor_col < *pos + ch.len_utf8()
            });
            match cur_idx {
                Some(i) if i > 0 => (i - 1, i),
                Some(0) if len > 1 => (0, 1), // At first char, swap with next
                _ => return Ok(CommandStatus::Failure),
            }
        };

        let ch1 = chars[idx1].1;
        let ch2 = chars[idx2].1;

        // Build new text with swapped characters
        let mut new_text = String::with_capacity(text.len());
        for (i, (_, ch)) in chars.iter().enumerate() {
            if i == idx1 {
                new_text.push(ch2);
            } else if i == idx2 {
                new_text.push(ch1);
            } else {
                new_text.push(*ch);
            }
        }

        // Replace line content
        if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
            *line_mut = crate::line::Line::from(new_text.as_str());
        }
        editor.current_buffer_mut().set_modified(true);

        // Move cursor forward past the transposed chars
        if cursor_col < new_text.len() {
            editor.move_cursor_right();
        }

        Ok(CommandStatus::Success)
    }

    /// Quote next character (insert literally)
    pub fn quote_char(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.quote_pending = true;
        editor.display.set_message("C-q");
        Ok(CommandStatus::Success)
    }

    /// Zap to character - delete from cursor up to and including specified char (M-z)
    pub fn zap_to_char(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        editor.display.set_message("Zap to char: ");
        editor.display.force_redraw();

        // Read the target character
        let target = if let Ok(Some(key)) = editor.read_key_for_describe() {
            if let Some(ch) = key.base_char() {
                ch
            } else {
                editor.display.set_message("Aborted");
                return Ok(CommandStatus::Abort);
            }
        } else {
            editor.display.set_message("Aborted");
            return Ok(CommandStatus::Abort);
        };

        let count = n.abs().max(1) as usize;
        let forward = n >= 0;

        editor.start_kill();

        for _ in 0..count {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();
            let line_count = editor.current_buffer().line_count();

            if forward {
                // Search forward for target character
                let mut found = false;
                let mut search_line = cursor_line;
                let mut search_col = cursor_col;

                'outer: while search_line < line_count {
                    if let Some(line) = editor.current_buffer().line(search_line) {
                        let text = line.text();
                        let start = if search_line == cursor_line { search_col } else { 0 };

                        for (pos, ch) in text[start..].char_indices() {
                            if ch == target {
                                // Found - delete from cursor to here (inclusive)
                                let end_col = start + pos + ch.len_utf8();

                                // Delete the region
                                if search_line == cursor_line {
                                    // Same line
                                    let killed = text[cursor_col..end_col].to_string();
                                    editor.kill_append(&killed);
                                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                        line_mut.delete_range(cursor_col, end_col);
                                    }
                                    editor.current_buffer_mut().set_modified(true);
                                } else {
                                    // Multi-line - collect and delete
                                    let mut killed = String::new();

                                    // Rest of start line
                                    if let Some(start_line) = editor.current_buffer().line(cursor_line) {
                                        killed.push_str(&start_line.text()[cursor_col..]);
                                        killed.push('\n');
                                    }

                                    // Middle lines
                                    for mid_line in (cursor_line + 1)..search_line {
                                        if let Some(line) = editor.current_buffer().line(mid_line) {
                                            killed.push_str(line.text());
                                            killed.push('\n');
                                        }
                                    }

                                    // Part of end line
                                    killed.push_str(&text[..end_col]);
                                    editor.kill_append(&killed);

                                    // Delete from end backwards
                                    // Delete from cursor_col to end of cursor_line
                                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                        let len = line_mut.len();
                                        line_mut.delete_range(cursor_col, len);
                                    }

                                    // Delete intermediate lines
                                    for _ in (cursor_line + 1)..=search_line {
                                        // Join with next line and delete up to end_col
                                        editor.current_buffer_mut().join_line(cursor_line);
                                    }

                                    // Now delete up to end_col from the joined line
                                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                        line_mut.delete_range(cursor_col, cursor_col + end_col);
                                    }
                                    editor.current_buffer_mut().set_modified(true);
                                }

                                found = true;
                                break 'outer;
                            }
                        }
                    }
                    search_line += 1;
                    search_col = 0;
                }

                if !found {
                    editor.display.set_message(&format!("'{}' not found", target));
                    return Ok(CommandStatus::Failure);
                }
            } else {
                // Search backward for target character
                editor.display.set_message("Backward zap not yet implemented");
                return Ok(CommandStatus::Failure);
            }
        }

        editor.display.clear_message();
        Ok(CommandStatus::Success)
    }

    /// Helper: check if character is a word character
    fn is_word_char(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    /// Move forward to end of word
    pub fn forward_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return backward_word(editor, false, -n);
        }

        for _ in 0..n.max(1) {
            let mut cursor_line = editor.current_window().cursor_line();
            let mut cursor_col = editor.current_window().cursor_col();
            let line_count = editor.current_buffer().line_count();

            // Skip word characters first (if in a word)
            loop {
                if let Some(line) = editor.current_buffer().line(cursor_line) {
                    let text = line.text();
                    if cursor_col < text.len() {
                        if let Some(ch) = text[cursor_col..].chars().next() {
                            if is_word_char(ch) {
                                cursor_col += ch.len_utf8();
                                continue;
                            }
                        }
                    }
                }
                break;
            }

            // Skip non-word characters to find next word
            loop {
                if let Some(line) = editor.current_buffer().line(cursor_line) {
                    let text = line.text();
                    if cursor_col < text.len() {
                        if let Some(ch) = text[cursor_col..].chars().next() {
                            if !is_word_char(ch) {
                                cursor_col += ch.len_utf8();
                                continue;
                            } else {
                                break; // Found start of next word
                            }
                        }
                    }
                    // End of line - move to next line
                    if cursor_line + 1 < line_count {
                        cursor_line += 1;
                        cursor_col = 0;
                        continue;
                    }
                }
                break;
            }

            editor
                .current_window_mut()
                .set_cursor(cursor_line, cursor_col);
        }

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Move backward to start of word
    pub fn backward_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return forward_word(editor, false, -n);
        }

        for _ in 0..n.max(1) {
            let mut cursor_line = editor.current_window().cursor_line();
            let mut cursor_col = editor.current_window().cursor_col();

            // Skip non-word characters backward
            loop {
                if cursor_col > 0 {
                    if let Some(line) = editor.current_buffer().line(cursor_line) {
                        let text = line.text();
                        let before = &text[..cursor_col];
                        if let Some(ch) = before.chars().last() {
                            if !is_word_char(ch) {
                                cursor_col -= ch.len_utf8();
                                continue;
                            } else {
                                break; // Found end of a word
                            }
                        }
                    }
                } else if cursor_line > 0 {
                    // Go to end of previous line
                    cursor_line -= 1;
                    if let Some(line) = editor.current_buffer().line(cursor_line) {
                        cursor_col = line.len();
                    }
                    continue;
                }
                break;
            }

            // Skip word characters backward to find start of word
            loop {
                if cursor_col > 0 {
                    if let Some(line) = editor.current_buffer().line(cursor_line) {
                        let text = line.text();
                        let before = &text[..cursor_col];
                        if let Some(ch) = before.chars().last() {
                            if is_word_char(ch) {
                                cursor_col -= ch.len_utf8();
                                continue;
                            }
                        }
                    }
                }
                break;
            }

            editor
                .current_window_mut()
                .set_cursor(cursor_line, cursor_col);
        }

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Kill word forward
    pub fn kill_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return backward_kill_word(editor, false, -n);
        }

        editor.start_kill();
        editor.display.force_redraw();

        for _ in 0..n.max(1) {
            let start_line = editor.current_window().cursor_line();
            let start_col = editor.current_window().cursor_col();

            // Move forward one word
            forward_word(editor, false, 1)?;

            let end_line = editor.current_window().cursor_line();
            let end_col = editor.current_window().cursor_col();

            // Delete from start to end position
            // Move cursor back to start
            editor.current_window_mut().set_cursor(start_line, start_col);

            // Collect and delete the text
            if start_line == end_line {
                // Same line - simple case
                if let Some(line) = editor.current_buffer().line(start_line) {
                    let killed = line.safe_slice(start_col, end_col).to_string();
                    let actual_start = line.text().len() - line.safe_slice_from(start_col).len();
                    let actual_end = actual_start + killed.len();
                    editor.kill_append(&killed);
                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(start_line) {
                        if actual_end > actual_start {
                            line_mut.delete_range(actual_start, actual_end);
                        }
                    }
                }
                editor.current_buffer_mut().set_modified(true);
            } else {
                // Spans lines - delete character by character
                while editor.current_window().cursor_line() < end_line
                    || (editor.current_window().cursor_line() == end_line
                        && editor.current_window().cursor_col() < end_col)
                {
                    let cur_line = editor.current_window().cursor_line();
                    let cur_col = editor.current_window().cursor_col();
                    let line_len = editor
                        .current_buffer()
                        .line(cur_line)
                        .map(|l| l.len())
                        .unwrap_or(0);

                    if cur_col < line_len {
                        if let Some(ch) =
                            editor.current_buffer_mut().delete_char(cur_line, cur_col)
                        {
                            editor.kill_append(&ch.to_string());
                        }
                    } else {
                        // Join lines
                        editor.current_buffer_mut().join_line(cur_line);
                        editor.kill_append("\n");
                    }
                }
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Kill word backward
    pub fn backward_kill_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        if n < 0 {
            return kill_word(editor, false, -n);
        }

        editor.start_kill();
        editor.display.force_redraw();

        for _ in 0..n.max(1) {
            let end_line = editor.current_window().cursor_line();
            let end_col = editor.current_window().cursor_col();

            // Move backward one word
            backward_word(editor, false, 1)?;

            let start_line = editor.current_window().cursor_line();
            let start_col = editor.current_window().cursor_col();

            // Collect and delete the text (cursor is at start)
            if start_line == end_line {
                // Same line - simple case
                if let Some(line) = editor.current_buffer().line(start_line) {
                    let killed = line.safe_slice(start_col, end_col).to_string();
                    let actual_start = line.text().len() - line.safe_slice_from(start_col).len();
                    let actual_end = actual_start + killed.len();
                    editor.kill_prepend(&killed);
                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(start_line) {
                        if actual_end > actual_start {
                            line_mut.delete_range(actual_start, actual_end);
                        }
                    }
                }
                editor.current_buffer_mut().set_modified(true);
            } else {
                // Spans lines - need to handle carefully
                // For simplicity, delete forward until we reach end position
                let mut deleted = String::new();
                while editor.current_window().cursor_line() < end_line
                    || (editor.current_window().cursor_line() == end_line
                        && editor.current_window().cursor_col() < end_col)
                {
                    let cur_line = editor.current_window().cursor_line();
                    let cur_col = editor.current_window().cursor_col();
                    let line_len = editor
                        .current_buffer()
                        .line(cur_line)
                        .map(|l| l.len())
                        .unwrap_or(0);

                    if cur_col < line_len {
                        if let Some(ch) =
                            editor.current_buffer_mut().delete_char(cur_line, cur_col)
                        {
                            deleted.push(ch);
                        }
                    } else {
                        editor.current_buffer_mut().join_line(cur_line);
                        deleted.push('\n');
                    }
                }
                editor.kill_prepend(&deleted);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Move backward to start of paragraph
    pub fn backward_paragraph(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs().max(1) {
            if n >= 0 {
                editor.backward_paragraph();
            } else {
                editor.forward_paragraph();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Move forward to end of paragraph
    pub fn forward_paragraph(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.abs().max(1) {
            if n >= 0 {
                editor.forward_paragraph();
            } else {
                editor.backward_paragraph();
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Fill (reflow) the current paragraph
    pub fn fill_paragraph(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.fill_paragraph(72); // Default fill column
        Ok(CommandStatus::Success)
    }

    /// Set mark at current cursor position
    pub fn set_mark(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.current_window_mut().set_mark();
        editor.display.set_message("Mark set");
        Ok(CommandStatus::Success)
    }

    /// Helper: get region bounds (start_line, start_col, end_line, end_col)
    /// Returns None if mark is not set
    fn get_region(editor: &EditorState) -> Option<(usize, usize, usize, usize)> {
        let mark = editor.current_window().mark()?;
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        // Determine which is start and which is end
        if mark.0 < cursor_line || (mark.0 == cursor_line && mark.1 <= cursor_col) {
            Some((mark.0, mark.1, cursor_line, cursor_col))
        } else {
            Some((cursor_line, cursor_col, mark.0, mark.1))
        }
    }

    /// Helper: collect text in region
    fn collect_region_text(editor: &EditorState, start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> String {
        let mut text = String::new();

        for line_idx in start_line..=end_line {
            if let Some(line) = editor.current_buffer().line(line_idx) {
                let start = if line_idx == start_line { start_col } else { 0 };
                let end = if line_idx == end_line { end_col } else { line.len() };

                // Use safe_slice to handle UTF-8 boundaries correctly
                text.push_str(line.safe_slice(start, end));

                // Add newline between lines (but not after the last line segment)
                if line_idx < end_line {
                    text.push('\n');
                }
            }
        }

        text
    }

    /// Kill region (text between mark and point)
    pub fn kill_region(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let region = match get_region(editor) {
            Some(r) => r,
            None => {
                editor.display.set_message("No mark set");
                return Ok(CommandStatus::Failure);
            }
        };

        editor.display.force_redraw();
        let (start_line, start_col, end_line, end_col) = region;

        // Collect the text first
        let killed_text = collect_region_text(editor, start_line, start_col, end_line, end_col);

        // Start a new kill entry
        editor.start_kill();
        editor.kill_append(&killed_text);

        // Move cursor to start of region
        editor.current_window_mut().set_cursor(start_line, start_col);

        // Delete the region
        if start_line == end_line {
            // Same line - simple case
            if let Some(line) = editor.current_buffer_mut().line_mut(start_line) {
                line.delete_range(start_col, end_col);
            }
            editor.current_buffer_mut().set_modified(true);
        } else {
            // Multi-line deletion
            // Delete from start_col to end of start_line
            if let Some(line) = editor.current_buffer_mut().line_mut(start_line) {
                let line_len = line.len();
                if start_col < line_len {
                    line.delete_range(start_col, line_len);
                }
            }

            // Delete intermediate lines (in reverse to avoid index shifting)
            for line_idx in (start_line + 1..end_line).rev() {
                editor.current_buffer_mut().delete_line(line_idx);
            }

            // Handle the end line - take remaining content after end_col
            if end_line > start_line {
                // The end line is now at start_line + 1 (after removing intermediates)
                let remaining_line_idx = start_line + 1;
                if let Some(end_line_content) = editor.current_buffer().line(remaining_line_idx) {
                    let remaining = if end_col < end_line_content.len() {
                        end_line_content.text()[end_col..].to_string()
                    } else {
                        String::new()
                    };

                    // Append remaining to start line and remove the end line
                    if let Some(start_line_ref) = editor.current_buffer_mut().line_mut(start_line) {
                        start_line_ref.append_str(&remaining);
                    }
                    editor.current_buffer_mut().delete_line(remaining_line_idx);
                }
            }

            editor.current_buffer_mut().set_modified(true);
        }

        // Clear mark after kill
        editor.current_window_mut().clear_mark();

        Ok(CommandStatus::Success)
    }

    /// Start incremental search forward
    pub fn search_forward(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_search(crate::editor::SearchDirection::Forward);
        Ok(CommandStatus::Success)
    }

    /// Start incremental search backward
    pub fn search_backward(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_search(crate::editor::SearchDirection::Backward);
        Ok(CommandStatus::Success)
    }

    /// Hunt forward - repeat last search forward
    pub fn hunt_forward(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let count = n.max(1) as usize;
        for _ in 0..count {
            if !editor.hunt(crate::editor::SearchDirection::Forward) {
                return Ok(CommandStatus::Failure);
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Hunt backward - repeat last search backward
    pub fn hunt_backward(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let count = n.max(1) as usize;
        for _ in 0..count {
            if !editor.hunt(crate::editor::SearchDirection::Backward) {
                return Ok(CommandStatus::Failure);
            }
        }
        Ok(CommandStatus::Success)
    }

    /// Query replace (search and replace with confirmation)
    pub fn query_replace(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Query replace", crate::editor::PromptAction::QueryReplaceSearch, None);
        Ok(CommandStatus::Success)
    }

    /// Replace string (search and replace all without confirmation)
    pub fn replace_string(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Replace string", crate::editor::PromptAction::ReplaceStringSearch, None);
        Ok(CommandStatus::Success)
    }

    /// Find file (open or create)
    pub fn find_file(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Find file", crate::editor::PromptAction::FindFile, None);
        Ok(CommandStatus::Success)
    }

    /// Switch to buffer
    pub fn switch_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Default to the "other" buffer (most recently used that isn't current)
        let current_buf = editor.current_window().buffer_idx();
        let default = editor.buffers.iter()
            .enumerate()
            .find(|(i, _)| *i != current_buf)
            .map(|(_, b)| b.name().to_string());
        editor.start_prompt("Switch to buffer", crate::editor::PromptAction::SwitchBuffer, default);
        Ok(CommandStatus::Success)
    }

    /// List all buffers
    pub fn list_buffers(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.list_buffers();
        Ok(CommandStatus::Success)
    }

    /// Switch to next buffer in buffer list
    pub fn next_buffer(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let count = n.max(1) as usize;
        let num_buffers = editor.buffers.len();
        if num_buffers <= 1 {
            editor.display.set_message("Only one buffer");
            return Ok(CommandStatus::Success);
        }

        let current_buf = editor.current_window().buffer_idx();
        let new_buf = (current_buf + count) % num_buffers;
        editor.current_window_mut().set_buffer_idx(new_buf);
        editor.current_window_mut().set_cursor(0, 0);
        editor.display.force_redraw();

        let name = editor.buffers[new_buf].name().to_string();
        editor.display.set_message(&format!("Buffer: {}", name));
        Ok(CommandStatus::Success)
    }

    /// Switch to previous buffer in buffer list
    pub fn previous_buffer(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let count = n.max(1) as usize;
        let num_buffers = editor.buffers.len();
        if num_buffers <= 1 {
            editor.display.set_message("Only one buffer");
            return Ok(CommandStatus::Success);
        }

        let current_buf = editor.current_window().buffer_idx();
        let new_buf = (current_buf + num_buffers - (count % num_buffers)) % num_buffers;
        editor.current_window_mut().set_buffer_idx(new_buf);
        editor.current_window_mut().set_cursor(0, 0);
        editor.display.force_redraw();

        let name = editor.buffers[new_buf].name().to_string();
        editor.display.set_message(&format!("Buffer: {}", name));
        Ok(CommandStatus::Success)
    }

    /// Execute shell command
    pub fn shell_command(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Shell command", crate::editor::PromptAction::ShellCommand, None);
        Ok(CommandStatus::Success)
    }

    /// Insert file contents at cursor (C-x i)
    pub fn insert_file(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Insert file", crate::editor::PromptAction::InsertFile, None);
        Ok(CommandStatus::Success)
    }

    /// Filter buffer through shell command (C-x |)
    pub fn filter_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Filter through", crate::editor::PromptAction::FilterBuffer, None);
        Ok(CommandStatus::Success)
    }

    /// Pipe region through shell command (M-|)
    /// Without C-u: output goes to *Shell Command Output* buffer
    /// With C-u: output replaces the region
    pub fn shell_command_on_region(editor: &mut EditorState, f: bool, _n: i32) -> Result<CommandStatus> {
        // Get and store the region
        let region = match get_region(editor) {
            Some(r) => r,
            None => {
                editor.display.set_message("No mark set");
                return Ok(CommandStatus::Failure);
            }
        };

        // Store region for later use after prompt
        editor.filter_region = Some(region);

        // Choose action based on C-u prefix
        let action = if f {
            crate::editor::PromptAction::FilterRegionReplace
        } else {
            crate::editor::PromptAction::FilterRegion
        };

        editor.start_prompt("Shell command on region", action, None);
        Ok(CommandStatus::Success)
    }

    /// Kill buffer
    pub fn kill_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Default to current buffer
        let default = Some(editor.current_buffer().name().to_string());
        editor.start_prompt("Kill buffer", crate::editor::PromptAction::KillBuffer, default);
        Ok(CommandStatus::Success)
    }

    /// Go to line number
    pub fn goto_line(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Goto line", crate::editor::PromptAction::GotoLine, None);
        Ok(CommandStatus::Success)
    }

    /// Split current window horizontally
    pub fn split_window(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        if editor.split_window() {
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("Window too small to split");
            Ok(CommandStatus::Failure)
        }
    }

    /// Delete all windows except current
    pub fn delete_other_windows(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.delete_other_windows();
        Ok(CommandStatus::Success)
    }

    /// Delete current window
    pub fn delete_window(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        if editor.delete_window() {
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("Can't delete the only window");
            Ok(CommandStatus::Failure)
        }
    }

    /// Switch to other window
    pub fn other_window(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.other_window();
        Ok(CommandStatus::Success)
    }

    /// Enlarge current window
    pub fn enlarge_window(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        let lines = if f { n.max(1) as u16 } else { 1 };
        if editor.enlarge_window(lines) {
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("Can't enlarge window");
            Ok(CommandStatus::Failure)
        }
    }

    /// Shrink current window
    pub fn shrink_window(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        let lines = if f { n.max(1) as u16 } else { 1 };
        if editor.shrink_window(lines) {
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("Can't shrink window");
            Ok(CommandStatus::Failure)
        }
    }

    /// Copy region (without deleting)
    pub fn copy_region(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let region = match get_region(editor) {
            Some(r) => r,
            None => {
                editor.display.set_message("No mark set");
                return Ok(CommandStatus::Failure);
            }
        };

        let (start_line, start_col, end_line, end_col) = region;

        // Collect the text
        let copied_text = collect_region_text(editor, start_line, start_col, end_line, end_col);

        // Add to kill ring (so C-y can paste it)
        editor.start_kill();
        editor.kill_append(&copied_text);

        // Clear mark after copy
        editor.current_window_mut().clear_mark();
        editor.display.set_message("Region copied");

        Ok(CommandStatus::Success)
    }

    /// Clear modified flag (M-~)
    pub fn not_modified(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.current_buffer_mut().set_modified(false);
        editor.display.set_message("Modification flag cleared");
        Ok(CommandStatus::Success)
    }

    /// Toggle read-only mode (C-x C-q)
    pub fn toggle_read_only(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let is_read_only = editor.current_buffer().modes().view;
        editor.current_buffer_mut().modes_mut().view = !is_read_only;

        if !is_read_only {
            editor.display.set_message("Buffer is now read-only");
        } else {
            editor.display.set_message("Buffer is now writable");
        }
        Ok(CommandStatus::Success)
    }

    /// Toggle auto-save mode (C-x a)
    pub fn toggle_auto_save(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.toggle_auto_save();
        Ok(CommandStatus::Success)
    }

    /// Revert buffer to saved file contents (M-x revert-buffer)
    pub fn revert_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        use std::fs;

        let filename = match editor.current_buffer().filename() {
            Some(path) => path.clone(),
            None => {
                editor.display.set_message("Buffer has no file");
                return Ok(CommandStatus::Failure);
            }
        };

        match fs::read_to_string(&filename) {
            Ok(content) => {
                editor.current_buffer_mut().set_content(&content);
                editor.current_buffer_mut().set_modified(false);
                editor.current_window_mut().set_cursor(0, 0);
                editor.current_window_mut().set_top_line(0);
                editor.display.force_redraw();
                editor.display.set_message(&format!("Reverted {}", filename.display()));
                Ok(CommandStatus::Success)
            }
            Err(e) => {
                editor.display.set_message(&format!("Error reading file: {}", e));
                Ok(CommandStatus::Failure)
            }
        }
    }

    /// Copy current line to kill ring (C-x C-k)
    pub fn copy_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let count = n.max(1) as usize;
        let line_count = editor.current_buffer().line_count();

        let mut copied = String::new();
        for i in 0..count {
            let line_idx = cursor_line + i;
            if line_idx >= line_count {
                break;
            }
            if let Some(line) = editor.current_buffer().line(line_idx) {
                copied.push_str(line.text());
                copied.push('\n');
            }
        }

        if !copied.is_empty() {
            editor.start_kill();
            editor.kill_append(&copied);
            let actual_count = count.min(line_count - cursor_line);
            editor.display.set_message(&format!("Copied {} line(s)", actual_count));
        }

        Ok(CommandStatus::Success)
    }

    /// Duplicate current line (C-x d)
    pub fn duplicate_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();
        let count = n.max(1) as usize;

        // Get the current line text
        let line_text = match editor.current_buffer().line(cursor_line) {
            Some(line) => line.text().to_string(),
            None => return Ok(CommandStatus::Failure),
        };

        // Insert duplicates after current line
        for i in 0..count {
            let insert_at = cursor_line + 1 + i;
            editor.current_buffer_mut().insert_line_at(insert_at);
            if let Some(line) = editor.current_buffer_mut().line_mut(insert_at) {
                line.append_str(&line_text);
            }
        }

        editor.current_buffer_mut().set_modified(true);
        // Move cursor to first duplicate
        editor.current_window_mut().set_cursor(cursor_line + 1, cursor_col);
        editor.display.set_message(&format!("Duplicated {} time(s)", count));

        Ok(CommandStatus::Success)
    }

    /// Sort lines in region
    pub fn sort_lines(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let region = get_region(editor);

        let (start_line, end_line) = match region {
            Some((sl, _, el, _)) => (sl, el),
            None => {
                editor.display.set_message("No region set");
                return Ok(CommandStatus::Failure);
            }
        };

        // Collect lines
        let mut lines: Vec<String> = Vec::new();
        for i in start_line..=end_line {
            if let Some(line) = editor.current_buffer().line(i) {
                lines.push(line.text().to_string());
            }
        }

        // Sort
        lines.sort();

        // Replace lines
        for (i, new_text) in lines.iter().enumerate() {
            let line_idx = start_line + i;
            if let Some(line) = editor.current_buffer_mut().line_mut(line_idx) {
                line.clear();
                line.append_str(new_text);
            }
        }

        editor.current_buffer_mut().set_modified(true);
        editor.display.set_message(&format!("Sorted {} lines", end_line - start_line + 1));

        Ok(CommandStatus::Success)
    }

    /// Write buffer to a new file (Save As) (C-x C-w)
    pub fn write_file(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Default to current filename if any
        let default = editor.current_buffer()
            .filename()
            .map(|p| p.display().to_string());
        editor.start_prompt("Write file", crate::editor::PromptAction::WriteFile, default);
        Ok(CommandStatus::Success)
    }

    /// Save current buffer to file
    pub fn save_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let buffer = editor.current_buffer();

        // Check if buffer has a filename
        if buffer.filename().is_none() {
            editor.display.set_message("No file name");
            return Ok(CommandStatus::Failure);
        }

        // Check if buffer is modified
        if !buffer.is_modified() {
            editor.display.set_message("(No changes need to be saved)");
            return Ok(CommandStatus::Success);
        }

        // Get filename for message before mutable borrow
        let filename = buffer
            .filename()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let line_count = buffer.line_count();

        // Save the buffer
        match editor.current_buffer().save() {
            Ok(()) => {
                editor.current_buffer_mut().set_modified(false);
                // Delete auto-save file after successful save
                if let Some(path) = editor.current_buffer().filename() {
                    editor.delete_auto_save_file(path);
                }
                editor
                    .display
                    .set_message(&format!("Wrote {} lines to {}", line_count, filename));
                Ok(CommandStatus::Success)
            }
            Err(e) => {
                editor
                    .display
                    .set_message(&format!("Error writing file: {}", e));
                Ok(CommandStatus::Failure)
            }
        }
    }

    /// Undo the last edit operation
    pub fn undo(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Add boundary before undoing so consecutive undos work correctly
        editor.current_buffer_mut().add_undo_boundary();

        if let Some((line, col)) = editor.current_buffer_mut().undo() {
            // Move cursor to the undo position
            editor.current_window_mut().set_cursor(line, col);
            editor.ensure_cursor_visible();
            editor.display.set_message("Undo!");
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("Nothing to undo");
            Ok(CommandStatus::Failure)
        }
    }

    /// Start recording a keyboard macro (C-x ()
    pub fn start_macro(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Note: No need to pop - recording wasn't active yet when this key was processed
        editor.start_macro();
        Ok(CommandStatus::Success)
    }

    /// End recording a keyboard macro (C-x ))
    pub fn end_macro(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Remove this key from macro recording (it shouldn't be part of the macro)
        editor.macro_state.keys.pop();
        editor.end_macro();
        Ok(CommandStatus::Success)
    }

    /// Execute the keyboard macro (C-x e)
    pub fn execute_macro(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        // With prefix arg 0-9, execute from that slot
        if f && n >= 0 && n <= 9 {
            editor.execute_macro_slot(n as usize)?;
            return Ok(CommandStatus::Success);
        }

        // Otherwise execute n times (or 1 if no argument)
        let count = if n > 0 { n } else { 1 };
        for _ in 0..count {
            editor.execute_macro()?;
        }
        Ok(CommandStatus::Success)
    }

    /// Save current macro to a numbered slot (C-x C-k s, then digit)
    pub fn store_macro(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        // With prefix arg, use that slot
        if f && n >= 0 && n <= 9 {
            editor.save_macro_to_slot(n as usize);
            return Ok(CommandStatus::Success);
        }

        // Without prefix arg, prompt (using slot 0 as default)
        editor.display.set_message("Store macro to slot (0-9): ");
        editor.display.force_redraw();

        // Read a digit
        if let Ok(key) = editor.read_key_for_describe() {
            if let Some(k) = key {
                if let Some(ch) = k.base_char() {
                    if let Some(digit) = ch.to_digit(10) {
                        editor.save_macro_to_slot(digit as usize);
                        return Ok(CommandStatus::Success);
                    }
                }
            }
        }

        editor.display.set_message("Aborted");
        Ok(CommandStatus::Abort)
    }

    /// Load macro from a numbered slot (C-x C-k l, then digit)
    pub fn load_macro(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        // With prefix arg, use that slot
        if f && n >= 0 && n <= 9 {
            editor.load_macro_from_slot(n as usize);
            return Ok(CommandStatus::Success);
        }

        // Without prefix arg, prompt
        editor.display.set_message("Load macro from slot (0-9): ");
        editor.display.force_redraw();

        // Read a digit
        if let Ok(key) = editor.read_key_for_describe() {
            if let Some(k) = key {
                if let Some(ch) = k.base_char() {
                    if let Some(digit) = ch.to_digit(10) {
                        editor.load_macro_from_slot(digit as usize);
                        return Ok(CommandStatus::Success);
                    }
                }
            }
        }

        editor.display.set_message("Aborted");
        Ok(CommandStatus::Abort)
    }

    /// Uppercase word at cursor (M-u)
    pub fn upcase_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            transform_word(editor, |s| s.to_uppercase());
        }
        Ok(CommandStatus::Success)
    }

    /// Lowercase word at cursor (M-l)
    pub fn downcase_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            transform_word(editor, |s| s.to_lowercase());
        }
        Ok(CommandStatus::Success)
    }

    /// Capitalize word at cursor (M-c)
    pub fn capitalize_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            transform_word(editor, |s| {
                let mut chars = s.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars.flat_map(|c| c.to_lowercase())).collect(),
                }
            });
        }
        Ok(CommandStatus::Success)
    }

    /// Helper: transform word at cursor using a transformation function
    fn transform_word<F>(editor: &mut EditorState, transform: F)
    where
        F: Fn(&str) -> String,
    {
        let start_line = editor.current_window().cursor_line();
        let start_col = editor.current_window().cursor_col();

        // Move forward to find end of word
        let _ = forward_word(editor, false, 1);

        let end_line = editor.current_window().cursor_line();
        let end_col = editor.current_window().cursor_col();

        // If on same line, transform the word
        if start_line == end_line {
            if let Some(line) = editor.current_buffer().line(start_line) {
                let text = line.safe_slice(start_col, end_col);
                let transformed = transform(text);

                // Calculate actual byte boundaries
                let actual_start = line.text().len() - line.safe_slice_from(start_col).len();
                let actual_end = actual_start + line.safe_slice(start_col, end_col).len();

                if let Some(line_mut) = editor.current_buffer_mut().line_mut(start_line) {
                    line_mut.delete_range(actual_start, actual_end);
                    // Insert transformed text
                    let mut pos = actual_start;
                    for ch in transformed.chars() {
                        line_mut.insert_char(pos, ch);
                        pos += ch.len_utf8();
                    }
                }
                editor.current_buffer_mut().set_modified(true);

                // Update cursor to end of transformed word
                editor.current_window_mut().set_cursor(start_line, actual_start + transformed.len());
            }
        }
        // Multi-line words are rare; just move to word end
    }

    /// Uppercase region (C-x C-u)
    pub fn upcase_region(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        transform_region(editor, |s| s.to_uppercase())
    }

    /// Lowercase region (C-x C-l)
    pub fn downcase_region(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        transform_region(editor, |s| s.to_lowercase())
    }

    /// Helper: transform text in region
    fn transform_region<F>(editor: &mut EditorState, transform: F) -> Result<CommandStatus>
    where
        F: Fn(&str) -> String,
    {
        let region = match get_region(editor) {
            Some(r) => r,
            None => {
                editor.display.set_message("No mark set");
                return Ok(CommandStatus::Failure);
            }
        };

        let (start_line, start_col, end_line, end_col) = region;

        // Collect and transform text line by line
        for line_idx in start_line..=end_line {
            if let Some(line) = editor.current_buffer().line(line_idx) {
                let line_start = if line_idx == start_line { start_col } else { 0 };
                let line_end = if line_idx == end_line { end_col } else { line.len() };

                let text = line.safe_slice(line_start, line_end);
                let transformed = transform(text);

                // Calculate actual byte boundaries
                let actual_start = line.text().len() - line.safe_slice_from(line_start).len();
                let actual_end = actual_start + line.safe_slice(line_start, line_end).len();

                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    line_mut.delete_range(actual_start, actual_end);
                    // Insert transformed text
                    let mut pos = actual_start;
                    for ch in transformed.chars() {
                        line_mut.insert_char(pos, ch);
                        pos += ch.len_utf8();
                    }
                }
            }
        }

        editor.current_buffer_mut().set_modified(true);
        editor.display.set_message("Region case changed");
        Ok(CommandStatus::Success)
    }

    /// Exchange point and mark (C-x C-x)
    pub fn exchange_point_and_mark(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let window = editor.current_window();
        let cursor_line = window.cursor_line();
        let cursor_col = window.cursor_col();

        if let Some(mark) = window.mark() {
            let (mark_line, mark_col) = mark;

            // Set cursor to mark position
            editor.current_window_mut().set_cursor(mark_line, mark_col);
            // Set mark to old cursor position
            editor.current_window_mut().set_mark_at(cursor_line, cursor_col);

            editor.ensure_cursor_visible();
            Ok(CommandStatus::Success)
        } else {
            editor.display.set_message("No mark set");
            Ok(CommandStatus::Failure)
        }
    }

    /// Show cursor position information (C-x =)
    pub fn what_cursor_position(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();
        let line_count = editor.current_buffer().line_count();

        let (char_info, char_code) = if let Some(line) = editor.current_buffer().line(cursor_line) {
            let text = line.text();
            if cursor_col < text.len() {
                if let Some(ch) = text[cursor_col..].chars().next() {
                    (format!("'{}'", ch), format!("0x{:04X}", ch as u32))
                } else {
                    ("EOL".to_string(), "".to_string())
                }
            } else {
                ("EOL".to_string(), "".to_string())
            }
        } else {
            ("EOB".to_string(), "".to_string())
        };

        let msg = if char_code.is_empty() {
            format!("Line {} of {} Col {} {}", cursor_line + 1, line_count, cursor_col, char_info)
        } else {
            format!("Line {} of {} Col {} {} ({})", cursor_line + 1, line_count, cursor_col, char_info, char_code)
        };

        editor.display.set_message(&msg);
        Ok(CommandStatus::Success)
    }

    /// Delete all spaces and tabs around cursor, leave exactly one space (M-SPC)
    pub fn just_one_space(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        if let Some(line) = editor.current_buffer().line(cursor_line) {
            let text = line.text();

            // Find start of whitespace (going backward)
            let mut start = cursor_col;
            while start > 0 {
                let before = &text[..start];
                if let Some(ch) = before.chars().last() {
                    if ch == ' ' || ch == '\t' {
                        start -= ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Find end of whitespace (going forward)
            let mut end = cursor_col;
            while end < text.len() {
                if let Some(ch) = text[end..].chars().next() {
                    if ch == ' ' || ch == '\t' {
                        end += ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Delete the whitespace range
            if start < end {
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                    line_mut.delete_range(start, end);
                    // Insert exactly one space
                    line_mut.insert_char(start, ' ');
                }
                editor.current_buffer_mut().set_modified(true);
                // Move cursor to after the space
                editor.current_window_mut().set_cursor(cursor_line, start + 1);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Delete all spaces and tabs around cursor (M-\)
    pub fn delete_horizontal_space(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        if let Some(line) = editor.current_buffer().line(cursor_line) {
            let text = line.text();

            // Find start of whitespace (going backward)
            let mut start = cursor_col;
            while start > 0 {
                let before = &text[..start];
                if let Some(ch) = before.chars().last() {
                    if ch == ' ' || ch == '\t' {
                        start -= ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Find end of whitespace (going forward)
            let mut end = cursor_col;
            while end < text.len() {
                if let Some(ch) = text[end..].chars().next() {
                    if ch == ' ' || ch == '\t' {
                        end += ch.len_utf8();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Delete the whitespace range (don't insert any space)
            if start < end {
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                    line_mut.delete_range(start, end);
                }
                editor.current_buffer_mut().set_modified(true);
                editor.current_window_mut().set_cursor(cursor_line, start);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Delete blank lines around cursor (C-x C-o)
    pub fn delete_blank_lines(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();

        // Check if current line is blank
        let current_is_blank = editor
            .current_buffer()
            .line(cursor_line)
            .map(|l| l.text().trim().is_empty())
            .unwrap_or(false);

        if current_is_blank {
            // Delete all contiguous blank lines, leave one
            let mut first_blank = cursor_line;
            let mut last_blank = cursor_line;

            // Find first blank line in this group
            while first_blank > 0 {
                let prev_blank = editor
                    .current_buffer()
                    .line(first_blank - 1)
                    .map(|l| l.text().trim().is_empty())
                    .unwrap_or(false);
                if prev_blank {
                    first_blank -= 1;
                } else {
                    break;
                }
            }

            // Find last blank line in this group
            let line_count = editor.current_buffer().line_count();
            while last_blank + 1 < line_count {
                let next_blank = editor
                    .current_buffer()
                    .line(last_blank + 1)
                    .map(|l| l.text().trim().is_empty())
                    .unwrap_or(false);
                if next_blank {
                    last_blank += 1;
                } else {
                    break;
                }
            }

            // Delete all but one blank line
            let lines_to_delete = last_blank - first_blank;
            for _ in 0..lines_to_delete {
                if first_blank + 1 < editor.current_buffer().line_count() {
                    editor.current_buffer_mut().delete_line(first_blank + 1);
                }
            }

            if lines_to_delete > 0 {
                editor.current_buffer_mut().set_modified(true);
            }
            editor.current_window_mut().set_cursor(first_blank, 0);
        } else {
            // On non-blank line: delete any blank lines immediately following
            let line_count = editor.current_buffer().line_count();
            let mut deleted = 0;

            while cursor_line + 1 < editor.current_buffer().line_count() {
                let next_blank = editor
                    .current_buffer()
                    .line(cursor_line + 1)
                    .map(|l| l.text().trim().is_empty())
                    .unwrap_or(false);
                if next_blank {
                    editor.current_buffer_mut().delete_line(cursor_line + 1);
                    deleted += 1;
                } else {
                    break;
                }
            }

            if deleted > 0 {
                editor.current_buffer_mut().set_modified(true);
            }
        }

        editor.ensure_cursor_visible();
        Ok(CommandStatus::Success)
    }

    /// Insert spaces to next tab stop (M-i)
    pub fn tab_to_tab_stop(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        let tab_width = 8; // Standard tab width
        let count = if f { n.max(1) } else { 1 };

        for _ in 0..count {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            // Calculate display column
            let display_col = editor
                .current_buffer()
                .line(cursor_line)
                .map(|l| l.byte_to_col(cursor_col))
                .unwrap_or(0);

            // Calculate spaces needed to reach next tab stop
            let spaces_needed = tab_width - (display_col % tab_width);

            // Insert spaces
            for _ in 0..spaces_needed {
                editor.insert_char(' ');
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Describe what command a key is bound to (M-?)
    pub fn describe_key(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Show prompt and refresh
        editor.display.set_message("Describe key: ");
        editor.display.force_redraw();

        // Read the key (may need multiple events for prefix keys)
        let key = editor.read_key_for_describe()?;

        if let Some(k) = key {
            // Look up the binding
            if let Some(name) = editor.key_table().lookup_name(k) {
                editor.display.set_message(&format!("{} runs the command {}", k.display_name(), name));
            } else if k.is_self_insert() {
                editor.display.set_message(&format!("{} runs the command self-insert-command", k.display_name()));
            } else {
                editor.display.set_message(&format!("{} is not bound", k.display_name()));
            }
        } else {
            editor.display.set_message("Aborted");
            return Ok(CommandStatus::Abort);
        }

        Ok(CommandStatus::Success)
    }

    /// List all key bindings in a help buffer (F1)
    pub fn describe_bindings(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.describe_bindings();
        Ok(CommandStatus::Success)
    }

    /// Jump to matching fence character (bracket, paren, brace) (M-C-f)
    pub fn goto_matching_fence(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        // Get character at cursor
        let ch = if let Some(line) = editor.current_buffer().line(cursor_line) {
            let text = line.text();
            if cursor_col < text.len() {
                text[cursor_col..].chars().next()
            } else {
                None
            }
        } else {
            None
        };

        let ch = match ch {
            Some(c) => c,
            None => {
                editor.display.set_message("Not on a fence character");
                return Ok(CommandStatus::Failure);
            }
        };

        // Determine the matching fence and search direction
        let (target, forward) = match ch {
            '(' => (')', true),
            ')' => ('(', false),
            '[' => (']', true),
            ']' => ('[', false),
            '{' => ('}', true),
            '}' => ('{', false),
            '<' => ('>', true),
            '>' => ('<', false),
            _ => {
                editor.display.set_message("Not on a fence character");
                return Ok(CommandStatus::Failure);
            }
        };

        let line_count = editor.current_buffer().line_count();
        let mut depth = 1;

        if forward {
            // Search forward
            let mut line_idx = cursor_line;
            let mut col = cursor_col + ch.len_utf8(); // Start after current char

            while line_idx < line_count && depth > 0 {
                if let Some(line) = editor.current_buffer().line(line_idx) {
                    let text = line.text();
                    for (pos, c) in text[col..].char_indices() {
                        if c == ch {
                            depth += 1;
                        } else if c == target {
                            depth -= 1;
                            if depth == 0 {
                                // Found match
                                let match_col = col + pos;
                                editor.current_window_mut().set_cursor(line_idx, match_col);
                                editor.ensure_cursor_visible();
                                return Ok(CommandStatus::Success);
                            }
                        }
                    }
                }
                line_idx += 1;
                col = 0;
            }
        } else {
            // Search backward
            let mut line_idx = cursor_line;
            let mut search_up_to = cursor_col;

            loop {
                if let Some(line) = editor.current_buffer().line(line_idx) {
                    let text = line.text();
                    let slice = &text[..search_up_to];

                    // Search backward through characters
                    for (pos, c) in slice.char_indices().rev() {
                        if c == ch {
                            depth += 1;
                        } else if c == target {
                            depth -= 1;
                            if depth == 0 {
                                // Found match
                                editor.current_window_mut().set_cursor(line_idx, pos);
                                editor.ensure_cursor_visible();
                                return Ok(CommandStatus::Success);
                            }
                        }
                    }
                }

                if line_idx == 0 {
                    break;
                }
                line_idx -= 1;
                // For previous lines, search from end
                search_up_to = editor.current_buffer()
                    .line(line_idx)
                    .map(|l| l.len())
                    .unwrap_or(0);
            }
        }

        editor.display.set_message("No matching fence found");
        Ok(CommandStatus::Failure)
    }

    /// Remove trailing whitespace from current line, or all lines with prefix arg
    pub fn trim_line(editor: &mut EditorState, f: bool, _n: i32) -> Result<CommandStatus> {
        let mut trimmed_count = 0;

        if f {
            // With prefix arg, trim all lines in buffer
            let line_count = editor.current_buffer().line_count();
            for line_idx in 0..line_count {
                if trim_line_trailing_whitespace(editor, line_idx) {
                    trimmed_count += 1;
                }
            }
            if trimmed_count > 0 {
                editor.current_buffer_mut().set_modified(true);
                editor.display.set_message(&format!("Trimmed {} lines", trimmed_count));
            } else {
                editor.display.set_message("No trailing whitespace found");
            }
        } else {
            // Without prefix arg, trim current line only
            let cursor_line = editor.current_window().cursor_line();
            if trim_line_trailing_whitespace(editor, cursor_line) {
                editor.current_buffer_mut().set_modified(true);
                editor.display.set_message("Trailing whitespace removed");
            } else {
                editor.display.set_message("No trailing whitespace");
            }
            // Make sure cursor doesn't go past end of line
            let line_len = editor.current_buffer()
                .line(cursor_line)
                .map(|l| l.len())
                .unwrap_or(0);
            let cursor_col = editor.current_window().cursor_col();
            if cursor_col > line_len {
                editor.current_window_mut().set_cursor(cursor_line, line_len);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Helper: trim trailing whitespace from a single line, returns true if modified
    fn trim_line_trailing_whitespace(editor: &mut EditorState, line_idx: usize) -> bool {
        // Get the lengths we need before borrowing mutably
        let (new_len, old_len) = if let Some(line) = editor.current_buffer().line(line_idx) {
            let text = line.text();
            let trimmed_len = text.trim_end().len();
            (trimmed_len, text.len())
        } else {
            return false;
        };

        if new_len < old_len {
            // There was trailing whitespace
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                line_mut.delete_range(new_len, old_len);
            }
            return true;
        }
        false
    }

    /// Count words in buffer or region (M-=)
    pub fn word_count(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Check if region is active (mark is set)
        let region = get_region(editor);

        let (lines, words, chars, description) = if let Some((start_line, start_col, end_line, end_col)) = region {
            // Count in region
            let mut line_count = 0;
            let mut word_count = 0;
            let mut char_count = 0;
            let mut in_word = false;

            for line_idx in start_line..=end_line {
                if let Some(line) = editor.current_buffer().line(line_idx) {
                    let text = line.text();
                    let start = if line_idx == start_line { start_col } else { 0 };
                    let end = if line_idx == end_line { end_col } else { text.len() };

                    // Get the slice of text in the region
                    let slice = if end > start && end <= text.len() {
                        &text[start..end]
                    } else {
                        ""
                    };

                    for ch in slice.chars() {
                        char_count += 1;
                        if ch.is_whitespace() {
                            in_word = false;
                        } else if !in_word {
                            word_count += 1;
                            in_word = true;
                        }
                    }

                    // Count newline as character (except for last line of region)
                    if line_idx < end_line {
                        char_count += 1; // newline
                        in_word = false;
                        line_count += 1;
                    }
                }
            }
            line_count += 1; // Count the region span as at least 1 line

            (line_count, word_count, char_count, "Region")
        } else {
            // Count in entire buffer
            let mut line_count = editor.current_buffer().line_count();
            let mut word_count = 0;
            let mut char_count = 0;
            let mut in_word = false;

            for line_idx in 0..line_count {
                if let Some(line) = editor.current_buffer().line(line_idx) {
                    let text = line.text();
                    for ch in text.chars() {
                        char_count += 1;
                        if ch.is_whitespace() {
                            in_word = false;
                        } else if !in_word {
                            word_count += 1;
                            in_word = true;
                        }
                    }
                    // Count newline as character (except for last line)
                    if line_idx + 1 < line_count {
                        char_count += 1; // newline
                        in_word = false;
                    }
                }
            }

            (line_count, word_count, char_count, "Buffer")
        };

        editor.display.set_message(&format!(
            "{}: {} lines, {} words, {} characters",
            description, lines, words, chars
        ));

        Ok(CommandStatus::Success)
    }

    /// Join current line to previous line, removing indentation (M-^ / delete-indentation)
    pub fn join_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();

            if cursor_line == 0 {
                // Can't join first line with previous
                return Ok(CommandStatus::Failure);
            }

            // Get the length of the previous line (where we'll join)
            let prev_line_len = editor
                .current_buffer()
                .line(cursor_line - 1)
                .map(|l| l.len())
                .unwrap_or(0);

            // Check if previous line is empty (for space insertion decision)
            let prev_line_empty = prev_line_len == 0;

            // Get leading whitespace length of current line
            let leading_ws_len = if let Some(line) = editor.current_buffer().line(cursor_line) {
                let text = line.text();
                let mut ws_len = 0;
                for ch in text.chars() {
                    if ch == ' ' || ch == '\t' {
                        ws_len += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                ws_len
            } else {
                0
            };

            // Delete leading whitespace from current line first
            if leading_ws_len > 0 {
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                    line_mut.delete_range(0, leading_ws_len);
                }
                editor.current_buffer_mut().set_modified(true);
            }

            // Join with previous line
            if let Some(join_pos) = editor.current_buffer_mut().join_with_previous(cursor_line) {
                // Insert a single space at the join point if previous line wasn't empty
                // and there's actual content after the join
                let should_add_space = !prev_line_empty
                    && editor
                        .current_buffer()
                        .line(cursor_line - 1)
                        .map(|l| l.len() > join_pos)
                        .unwrap_or(false);

                if should_add_space {
                    editor
                        .current_buffer_mut()
                        .insert_char(cursor_line - 1, join_pos, ' ');
                    editor
                        .current_window_mut()
                        .set_cursor(cursor_line - 1, join_pos + 1);
                } else {
                    editor
                        .current_window_mut()
                        .set_cursor(cursor_line - 1, join_pos);
                }
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Scroll the other window (M-C-v)
    pub fn scroll_other_window(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        if editor.windows.len() <= 1 {
            editor.display.set_message("No other window");
            return Ok(CommandStatus::Failure);
        }

        // Find the other window index
        let other_idx = if editor.current_window + 1 < editor.windows.len() {
            editor.current_window + 1
        } else {
            0
        };

        // Get the other window's buffer's line count
        let buf_idx = editor.windows[other_idx].buffer_idx();
        let line_count = editor.buffers[buf_idx].line_count();
        let height = editor.windows[other_idx].height() as usize;

        // Calculate scroll amount
        let scroll_amount = if f {
            n.unsigned_abs() as usize
        } else {
            height.saturating_sub(2) // default page scroll
        };

        // Scroll the other window
        if n >= 0 || !f {
            // Scroll down
            editor.windows[other_idx].scroll_down(scroll_amount, line_count);
        } else {
            // Scroll up (negative argument)
            editor.windows[other_idx].scroll_up(scroll_amount);
        }

        editor.display.force_redraw();
        Ok(CommandStatus::Success)
    }

    /// Display current line number (C-x l)
    pub fn what_line(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        let cursor_line = editor.current_window().cursor_line();
        let total_lines = editor.current_buffer().line_count();

        editor.display.set_message(&format!(
            "Line {} of {}",
            cursor_line + 1, // Display as 1-indexed
            total_lines
        ));

        Ok(CommandStatus::Success)
    }

    /// Transpose words (M-t)
    pub fn transpose_words(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            let text = match editor.current_buffer().line(cursor_line) {
                Some(l) => l.text().to_string(),
                None => return Ok(CommandStatus::Failure),
            };

            // Find word boundaries
            let chars: Vec<(usize, char)> = text.char_indices().collect();
            if chars.is_empty() {
                return Ok(CommandStatus::Failure);
            }

            // Find current position in char indices
            let cur_char_idx = chars
                .iter()
                .position(|(pos, _)| *pos >= cursor_col)
                .unwrap_or(chars.len());

            // Find first word end (the word before or at cursor)
            let mut word1_end = cur_char_idx;
            // Skip whitespace backward to find end of word1
            while word1_end > 0 && chars.get(word1_end.saturating_sub(1)).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
                word1_end = word1_end.saturating_sub(1);
            }
            if word1_end == 0 && chars.get(0).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
                return Ok(CommandStatus::Failure);
            }

            // Find word1 start
            let mut word1_start = word1_end;
            while word1_start > 0 && !chars.get(word1_start.saturating_sub(1)).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
                word1_start = word1_start.saturating_sub(1);
            }

            // Find word2 start (skip whitespace after word1_end)
            let mut word2_start = word1_end;
            while word2_start < chars.len() && chars.get(word2_start).map(|(_, c)| c.is_whitespace()).unwrap_or(false) {
                word2_start += 1;
            }
            if word2_start >= chars.len() {
                return Ok(CommandStatus::Failure);
            }

            // Find word2 end
            let mut word2_end = word2_start;
            while word2_end < chars.len() && !chars.get(word2_end).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
                word2_end += 1;
            }

            // Get byte positions
            let byte_word1_start = chars[word1_start].0;
            let byte_word1_end = if word1_end < chars.len() {
                chars[word1_end].0
            } else {
                text.len()
            };
            let byte_word2_start = chars[word2_start].0;
            let byte_word2_end = if word2_end < chars.len() {
                chars[word2_end].0
            } else {
                text.len()
            };

            // Extract words and whitespace
            let word1 = &text[byte_word1_start..byte_word1_end];
            let between = &text[byte_word1_end..byte_word2_start];
            let word2 = &text[byte_word2_start..byte_word2_end];

            // Build new text with swapped words
            let mut new_text = String::new();
            new_text.push_str(&text[..byte_word1_start]);
            new_text.push_str(word2);
            new_text.push_str(between);
            new_text.push_str(word1);
            new_text.push_str(&text[byte_word2_end..]);

            // Update the line
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                *line_mut = crate::line::Line::from(new_text.clone());
            }
            editor.current_buffer_mut().set_modified(true);

            // Move cursor to end of second word (now at word1's original position + word2's length)
            let new_cursor = byte_word1_start + word2.len() + between.len() + word1.len();
            editor.current_window_mut().set_cursor(cursor_line, new_cursor);
        }

        Ok(CommandStatus::Success)
    }

    /// Transpose lines (C-x C-t)
    pub fn transpose_lines(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();

            if cursor_line == 0 {
                // Can't transpose first line with nothing above
                return Ok(CommandStatus::Failure);
            }

            let line_count = editor.current_buffer().line_count();
            if cursor_line >= line_count {
                return Ok(CommandStatus::Failure);
            }

            // Get both lines' text
            let line1_text = editor
                .current_buffer()
                .line(cursor_line - 1)
                .map(|l| l.text().to_string())
                .unwrap_or_default();
            let line2_text = editor
                .current_buffer()
                .line(cursor_line)
                .map(|l| l.text().to_string())
                .unwrap_or_default();

            // Swap line contents
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line - 1) {
                *line_mut = crate::line::Line::from(line2_text);
            }
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                *line_mut = crate::line::Line::from(line1_text);
            }
            editor.current_buffer_mut().set_modified(true);

            // Move cursor to next line (or stay if at end)
            if cursor_line + 1 < line_count {
                editor.current_window_mut().set_cursor(cursor_line + 1, 0);
            }
        }

        Ok(CommandStatus::Success)
    }

    /// Mark paragraph (M-h)
    pub fn mark_paragraph(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Move to beginning of paragraph
        editor.backward_paragraph();

        // Set mark at paragraph start
        editor.current_window_mut().set_mark();

        // Move to end of paragraph
        editor.forward_paragraph();

        editor.display.set_message("Mark set");
        Ok(CommandStatus::Success)
    }

    /// Mark word (M-@)
    pub fn mark_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        // Set mark at current position
        editor.current_window_mut().set_mark();

        // Move forward by n words using the forward_word command
        forward_word(editor, false, n)?;

        editor.display.set_message("Mark set");
        Ok(CommandStatus::Success)
    }

    /// Mark whole buffer (C-x h) - select entire buffer
    pub fn mark_whole_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Move to end of buffer
        let line_count = editor.current_buffer().line_count();
        if line_count > 0 {
            let last_line = line_count - 1;
            let last_col = editor.current_buffer().line(last_line)
                .map(|l| l.len())
                .unwrap_or(0);
            editor.current_window_mut().set_cursor(last_line, last_col);
        }

        // Set mark at end
        editor.current_window_mut().set_mark();

        // Move to beginning of buffer
        editor.current_window_mut().set_cursor(0, 0);
        editor.current_window_mut().set_top_line(0);

        editor.display.set_message("Mark set (whole buffer)");
        Ok(CommandStatus::Success)
    }

    /// Kill paragraph (M-C-k) - kill from point to end of paragraph
    pub fn kill_paragraph(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        editor.start_kill();

        for _ in 0..n.max(1) {
            let start_line = editor.current_window().cursor_line();
            let start_col = editor.current_window().cursor_col();

            // Move to end of paragraph
            editor.forward_paragraph();

            let end_line = editor.current_window().cursor_line();
            let end_col = editor.current_window().cursor_col();

            // If we didn't move, nothing to kill
            if start_line == end_line && start_col == end_col {
                return Ok(CommandStatus::Failure);
            }

            // Collect and delete text from start to end
            let mut deleted = String::new();

            // Build the text that will be deleted
            for line_idx in start_line..=end_line {
                if let Some(line) = editor.current_buffer().line(line_idx) {
                    let text = line.text();
                    let start = if line_idx == start_line { start_col } else { 0 };
                    let end = if line_idx == end_line { end_col } else { text.len() };

                    if end > start && end <= text.len() {
                        deleted.push_str(&text[start..end]);
                    }
                    if line_idx < end_line {
                        deleted.push('\n');
                    }
                }
            }

            // Delete the region by working backwards
            // First, delete content on lines between start and end
            for line_idx in (start_line..=end_line).rev() {
                if line_idx == start_line && line_idx == end_line {
                    // Same line - just delete the range
                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                        let text_len = line_mut.len();
                        if end_col <= text_len {
                            line_mut.delete_range(start_col, end_col);
                        }
                    }
                } else if line_idx == end_line {
                    // End line - delete from beginning to end_col
                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                        line_mut.delete_range(0, end_col);
                    }
                    // Join with previous line
                    if line_idx > 0 {
                        editor.current_buffer_mut().join_line(line_idx - 1);
                    }
                } else if line_idx == start_line {
                    // Start line - delete from start_col to end
                    if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                        let line_len = line_mut.len();
                        if start_col < line_len {
                            line_mut.delete_range(start_col, line_len);
                        }
                    }
                } else {
                    // Middle line - delete entire line
                    editor.current_buffer_mut().delete_line(line_idx);
                }
            }

            editor.current_buffer_mut().set_modified(true);
            editor.current_window_mut().set_cursor(start_line, start_col);
            editor.kill_append(&deleted);
        }

        Ok(CommandStatus::Success)
    }

    /// Split line at point (M-C-o) - like newline but cursor stays in place
    pub fn split_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        editor.display.force_redraw();

        for _ in 0..n.max(1) {
            let cursor_line = editor.current_window().cursor_line();
            let cursor_col = editor.current_window().cursor_col();

            // Insert newline at cursor position
            editor
                .current_buffer_mut()
                .insert_newline(cursor_line, cursor_col);

            // Cursor stays at the same position (now at end of current line)
            // The text after cursor moved to the new line below
        }

        Ok(CommandStatus::Success)
    }

    /// Make next kill command append to kill ring (M-C-w)
    pub fn append_next_kill(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        // Set flag so next kill appends instead of creating new entry
        editor.last_was_kill = true;
        editor.display.set_message("Next kill appends");
        Ok(CommandStatus::Success)
    }

    /// Indent region rigidly (C-x TAB)
    pub fn indent_rigidly(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
        let region = match get_region(editor) {
            Some(r) => r,
            None => {
                editor.display.set_message("No mark set");
                return Ok(CommandStatus::Failure);
            }
        };

        editor.display.force_redraw();

        let (start_line, _, end_line, _) = region;
        let indent_amount = if f { n } else { 1 }; // Default to 1 space

        for line_idx in start_line..=end_line {
            if let Some(line) = editor.current_buffer().line(line_idx) {
                // Skip empty lines
                if line.text().trim().is_empty() {
                    continue;
                }
            }

            if indent_amount > 0 {
                // Add spaces at beginning
                let spaces = " ".repeat(indent_amount as usize);
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    let new_text = format!("{}{}", spaces, line_mut.text());
                    *line_mut = crate::line::Line::from(new_text);
                }
            } else if indent_amount < 0 {
                // Remove spaces from beginning
                let remove_count = (-indent_amount) as usize;
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                    let text = line_mut.text();
                    let mut chars_to_remove = 0;
                    for ch in text.chars().take(remove_count) {
                        if ch == ' ' || ch == '\t' {
                            chars_to_remove += ch.len_utf8();
                        } else {
                            break;
                        }
                    }
                    if chars_to_remove > 0 {
                        line_mut.delete_range(0, chars_to_remove);
                    }
                }
            }
        }

        editor.current_buffer_mut().set_modified(true);
        let direction = if indent_amount >= 0 { "right" } else { "left" };
        editor.display.set_message(&format!(
            "Indented {} lines {}",
            end_line - start_line + 1,
            direction
        ));

        Ok(CommandStatus::Success)
    }
}
