//! Command dispatch system
//!
//! This module provides the key binding table and command implementations.
//! Commands are organized into submodules by category.

mod navigation;
mod editing;
mod mark;
mod search;
mod files;
mod windows;
mod macros;
mod case;
mod misc;

use std::collections::HashMap;

use crate::editor::EditorState;
use crate::error::Result;
use crate::input::{Key, key_flags};

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
        bindings.sort_by(|a, b| a.1.cmp(b.1));
        bindings
    }

    /// Look up a command by name (returns first matching binding)
    pub fn lookup_by_name(&self, name: &str) -> Option<CommandFn> {
        self.bindings
            .values()
            .find(|entry| entry.name == name)
            .map(|entry| entry.function)
    }

    /// Get all unique command names (sorted)
    pub fn command_names(&self) -> Vec<&'static str> {
        let mut names: Vec<_> = self.bindings
            .values()
            .map(|entry| entry.name)
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Set up default key bindings
    fn setup_defaults(&mut self) {
        use crate::input::key_flags;
        use navigation::*;
        use editing::*;
        use mark::*;
        use search::*;
        use files::*;
        use windows::*;
        use macros::*;
        use case::*;
        use misc::*;

        // Basic cursor movement
        self.bind_named(Key::ctrl('f'), forward_char, "forward-char");
        self.bind_named(Key::ctrl('b'), backward_char, "backward-char");
        self.bind_named(Key::ctrl('n'), next_line, "next-line");
        self.bind_named(Key::ctrl('p'), previous_line, "previous-line");
        self.bind_named(Key::ctrl('a'), beginning_of_line, "beginning-of-line");
        self.bind_named(Key::ctrl('e'), end_of_line, "end-of-line");
        self.bind_named(Key::meta('m'), back_to_indentation, "back-to-indentation");

        // Page movement
        self.bind_named(Key::ctrl('v'), scroll_down, "scroll-down");
        self.bind_named(Key::meta('v'), scroll_up, "scroll-up");
        self.bind_named(Key::meta('<'), beginning_of_buffer, "beginning-of-buffer");
        self.bind_named(Key::meta('>'), end_of_buffer, "end-of-buffer");

        // Arrow keys (special keys)
        self.bind_named(Key::special(0x4d), forward_char, "forward-char");
        self.bind_named(Key::special(0x4b), backward_char, "backward-char");
        self.bind_named(Key::special(0x50), next_line, "next-line");
        self.bind_named(Key::special(0x48), previous_line, "previous-line");
        self.bind_named(Key::special(0x49), scroll_up, "scroll-up");
        self.bind_named(Key::special(0x51), scroll_down, "scroll-down");
        self.bind_named(Key::special(0x47), beginning_of_line, "beginning-of-line");
        self.bind_named(Key::special(0x4f), end_of_line, "end-of-line");

        // Screen refresh
        self.bind_named(Key::ctrl('l'), redraw_display, "redraw-display");
        self.bind_named(Key::ctlx('#'), toggle_line_numbers, "toggle-line-numbers");
        self.bind_named(Key::ctlx('s'), toggle_syntax_highlighting, "toggle-syntax-highlighting");

        // Quit
        self.bind_named(Key::ctlx_ctrl('c'), quit, "save-buffers-kill-emacs");

        // Abort
        self.bind_named(Key::ctrl('g'), abort, "keyboard-quit");

        // Editing commands
        self.bind_named(Key::ctrl('d'), delete_char_forward, "delete-char");
        self.bind_named(Key::special(0x53), delete_char_forward, "delete-char");
        self.bind_named(Key(0x7f), delete_char_backward, "delete-backward-char");
        self.bind_named(Key::ctrl('h'), delete_char_backward, "delete-backward-char");
        self.bind_named(Key::meta('z'), zap_to_char, "zap-to-char");

        self.bind_named(Key::ctrl('k'), kill_line, "kill-line");
        self.bind_named(Key::ctrl('y'), yank, "yank");
        self.bind_named(Key::meta('y'), yank_pop, "yank-pop");

        self.bind_named(Key::ctrl('m'), newline, "newline");
        self.bind_named(Key::ctrl('o'), open_line, "open-line");
        self.bind_named(Key::ctrl('j'), indent_newline, "newline-and-indent");
        self.bind_named(Key::ctrl('i'), insert_tab, "tab-to-tab-stop");

        self.bind_named(Key::ctrl('t'), transpose_chars, "transpose-chars");
        self.bind_named(Key::ctrl('q'), quote_char, "quoted-insert");

        // File operations
        self.bind_named(Key::ctlx_ctrl('s'), save_buffer, "save-buffer");
        self.bind_named(Key::ctlx_ctrl('w'), write_file, "write-file");
        self.bind_named(Key::ctlx('i'), insert_file, "insert-file");
        self.bind_named(Key::meta('~'), not_modified, "not-modified");
        self.bind_named(Key::ctlx_ctrl('q'), toggle_read_only, "toggle-read-only");
        self.bind_named(Key::ctlx_ctrl('r'), revert_buffer, "revert-buffer");
        self.bind_named(Key::ctlx('a'), toggle_auto_save, "auto-save-mode");
        self.bind_named(Key::ctlx('w'), toggle_warn_unsaved, "toggle-warn-unsaved");

        // Line operations
        self.bind_named(Key::ctlx_ctrl('k'), copy_line, "copy-line");
        self.bind_named(Key::ctlx('d'), duplicate_line, "duplicate-line");

        // Word operations
        self.bind_named(Key::meta('f'), forward_word, "forward-word");
        self.bind_named(Key::meta('b'), backward_word, "backward-word");
        self.bind_named(Key::meta('d'), kill_word, "kill-word");
        self.bind_named(Key(0x2000_007f), backward_kill_word, "backward-kill-word");

        // Paragraph operations
        self.bind_named(Key::meta('{'), backward_paragraph, "backward-paragraph");
        self.bind_named(Key::meta('}'), forward_paragraph, "forward-paragraph");
        self.bind_named(Key::meta('q'), fill_paragraph, "fill-paragraph");

        // Mark/Region operations
        self.bind_named(Key::ctrl(' '), set_mark, "set-mark-command");
        self.bind_named(Key::ctrl('w'), kill_region, "kill-region");
        self.bind_named(Key::meta('w'), copy_region, "kill-ring-save");

        // Search
        self.bind_named(Key::ctrl('s'), search_forward, "isearch-forward");
        self.bind_named(Key::ctrl('r'), search_backward, "isearch-backward");
        self.bind_named(Key::meta('s'), hunt_forward, "hunt-forward");
        self.bind_named(Key::meta('S'), hunt_backward, "hunt-backward");
        self.bind_named(Key::meta('%'), query_replace, "query-replace");
        self.bind_named(Key::meta('r'), replace_string, "replace-string");

        // Buffer operations
        self.bind_named(Key::ctlx_ctrl('f'), find_file, "find-file");
        self.bind_named(Key::ctlx('b'), switch_buffer, "switch-to-buffer");
        self.bind_named(Key::ctlx_ctrl('b'), list_buffers, "list-buffers");
        self.bind_named(Key::ctlx('k'), kill_buffer, "kill-buffer");
        self.bind_named(Key::ctlx('n'), next_buffer, "next-buffer");
        self.bind_named(Key::ctlx('p'), previous_buffer, "previous-buffer");

        // Go to line
        self.bind_named(Key::meta('g'), goto_line, "goto-line");

        // Window operations
        self.bind_named(Key::ctlx('2'), split_window, "split-window-below");
        self.bind_named(Key::ctlx('1'), delete_other_windows, "delete-other-windows");
        self.bind_named(Key::ctlx('0'), delete_window, "delete-window");
        self.bind_named(Key::ctlx('o'), other_window, "other-window");
        self.bind_named(Key::ctlx('^'), enlarge_window, "enlarge-window");
        self.bind_named(Key::ctlx('v'), shrink_window, "shrink-window");

        // Undo
        self.bind_named(Key::ctrl('/'), undo, "undo");
        self.bind_named(Key::ctrl('_'), undo, "undo");

        // Shell
        self.bind_named(Key::meta('!'), shell_command, "shell-command");
        self.bind_named(Key::meta('|'), shell_command_on_region, "shell-command-on-region");
        self.bind_named(Key::ctlx('|'), filter_buffer, "filter-buffer");

        // Extended command
        self.bind_named(Key::meta('x'), execute_extended_command, "execute-extended-command");

        // Keyboard macros
        self.bind_named(Key::ctlx('('), start_macro, "kmacro-start-macro");
        self.bind_named(Key::ctlx(')'), end_macro, "kmacro-end-macro");
        self.bind_named(Key::ctlx('e'), execute_macro, "kmacro-end-and-call-macro");
        self.bind_named(Key::ctlx_meta('s'), store_macro, "store-kbd-macro");
        self.bind_named(Key::ctlx_meta('l'), load_macro, "load-kbd-macro");
        // Use M-S-s and M-S-l (uppercase) for file operations
        self.bind_named(Key(key_flags::CTLX | key_flags::META | 'S' as u32), save_macros_to_file, "save-macros-to-file");
        self.bind_named(Key(key_flags::CTLX | key_flags::META | 'L' as u32), load_macros_from_file, "load-macros-from-file");

        // Case operations
        self.bind_named(Key::meta('u'), upcase_word, "upcase-word");
        self.bind_named(Key::meta('l'), downcase_word, "downcase-word");
        self.bind_named(Key::meta('c'), capitalize_word, "capitalize-word");
        self.bind_named(Key::ctlx_ctrl('u'), upcase_region, "upcase-region");
        self.bind_named(Key::ctlx_ctrl('l'), downcase_region, "downcase-region");

        // Swap mark and point
        self.bind_named(Key::ctlx_ctrl('x'), exchange_point_and_mark, "exchange-point-and-mark");

        // Buffer position info
        self.bind_named(Key::ctlx('='), what_cursor_position, "what-cursor-position");

        // Whitespace operations
        self.bind_named(Key::meta(' '), just_one_space, "just-one-space");
        self.bind_named(Key::meta('\\'), delete_horizontal_space, "delete-horizontal-space");
        self.bind_named(Key::ctlx_ctrl('o'), delete_blank_lines, "delete-blank-lines");

        // Indentation
        self.bind_named(Key::meta('i'), tab_to_tab_stop, "tab-to-tab-stop");

        // Help
        self.bind_named(Key::meta('?'), describe_key, "describe-key");
        self.bind_named(Key::special(0x3b), describe_bindings, "describe-bindings");

        // Statistics
        self.bind_named(Key::meta('='), word_count, "count-words");

        // Navigation
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'f' as u32), goto_matching_fence, "goto-matching-fence");

        // Whitespace cleanup
        self.bind_named(Key::ctlx('t'), trim_line, "trim-line");

        // Join/delete-indentation
        self.bind_named(Key::meta('^'), join_line, "delete-indentation");

        // Scroll other window
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'v' as u32), scroll_other_window, "scroll-other-window");

        // Information
        self.bind_named(Key::ctlx('l'), what_line, "what-line");

        // Transpose operations
        self.bind_named(Key::meta('t'), transpose_words, "transpose-words");
        self.bind_named(Key::ctlx_ctrl('t'), transpose_lines, "transpose-lines");

        // Mark operations
        self.bind_named(Key::meta('h'), mark_paragraph, "mark-paragraph");
        self.bind_named(Key::ctlx('h'), mark_whole_buffer, "mark-whole-buffer");
        self.bind_named(Key::meta('@'), mark_word, "mark-word");

        // Kill operations
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'k' as u32), kill_paragraph, "kill-paragraph");
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'w' as u32), append_next_kill, "append-next-kill");

        // Line splitting
        self.bind_named(Key(key_flags::META | key_flags::CONTROL | 'o' as u32), split_line, "split-line");

        // Indentation
        self.bind_named(Key::ctlx_ctrl('i'), indent_rigidly, "indent-rigidly");
    }
}

impl Default for KeyTable {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// Re-export commands module for backwards compatibility
pub mod commands {
    pub use super::navigation::*;
    pub use super::editing::*;
    pub use super::mark::*;
    pub use super::search::*;
    pub use super::files::*;
    pub use super::windows::*;
    pub use super::macros::*;
    pub use super::case::*;
    pub use super::misc::*;
}
