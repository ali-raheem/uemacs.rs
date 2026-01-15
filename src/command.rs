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

/// Key binding table
pub struct KeyTable {
    bindings: HashMap<u32, CommandFn>,
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

    /// Add a key binding
    pub fn bind(&mut self, key: Key, cmd: CommandFn) {
        self.bindings.insert(key.code(), cmd);
    }

    /// Remove a key binding
    pub fn unbind(&mut self, key: Key) {
        self.bindings.remove(&key.code());
    }

    /// Look up a command for a key
    pub fn lookup(&self, key: Key) -> Option<CommandFn> {
        self.bindings.get(&key.code()).copied()
    }

    /// Set up default key bindings
    fn setup_defaults(&mut self) {
        use commands::*;

        // Basic cursor movement
        self.bind(Key::ctrl('f'), forward_char);
        self.bind(Key::ctrl('b'), backward_char);
        self.bind(Key::ctrl('n'), next_line);
        self.bind(Key::ctrl('p'), previous_line);
        self.bind(Key::ctrl('a'), beginning_of_line);
        self.bind(Key::ctrl('e'), end_of_line);

        // Page movement
        self.bind(Key::ctrl('v'), scroll_down);
        self.bind(Key::meta('v'), scroll_up);
        self.bind(Key::meta('<'), beginning_of_buffer);
        self.bind(Key::meta('>'), end_of_buffer);

        // Arrow keys (special keys)
        self.bind(Key::special(0x4d), forward_char);  // Right
        self.bind(Key::special(0x4b), backward_char); // Left
        self.bind(Key::special(0x50), next_line);     // Down
        self.bind(Key::special(0x48), previous_line); // Up
        self.bind(Key::special(0x49), scroll_up);     // PageUp
        self.bind(Key::special(0x51), scroll_down);   // PageDown
        self.bind(Key::special(0x47), beginning_of_line); // Home
        self.bind(Key::special(0x4f), end_of_line);       // End

        // Screen refresh
        self.bind(Key::ctrl('l'), redraw_display);

        // Quit
        self.bind(Key::ctlx_ctrl('c'), quit);

        // Abort
        self.bind(Key::ctrl('g'), abort);

        // Editing commands
        self.bind(Key::ctrl('d'), delete_char_forward);
        self.bind(Key::special(0x53), delete_char_forward); // Delete key
        self.bind(Key(0x7f), delete_char_backward);         // Backspace
        self.bind(Key::ctrl('h'), delete_char_backward);    // C-h also backspace

        self.bind(Key::ctrl('k'), kill_line);
        self.bind(Key::ctrl('y'), yank);

        self.bind(Key::ctrl('m'), newline);    // Enter
        self.bind(Key::ctrl('o'), open_line);
        self.bind(Key::ctrl('j'), indent_newline);
        self.bind(Key::ctrl('i'), insert_tab); // Tab

        self.bind(Key::ctrl('t'), transpose_chars);
        self.bind(Key::ctrl('q'), quote_char);

        // File operations
        self.bind(Key::ctlx_ctrl('s'), save_buffer);

        // Word operations
        self.bind(Key::meta('f'), forward_word);
        self.bind(Key::meta('b'), backward_word);
        self.bind(Key::meta('d'), kill_word);
        // M-Backspace for backward kill word
        self.bind(Key(0x2000_007f), backward_kill_word); // META | 0x7f

        // Paragraph operations
        self.bind(Key::meta('{'), backward_paragraph);
        self.bind(Key::meta('}'), forward_paragraph);
        self.bind(Key::meta('q'), fill_paragraph);

        // Mark/Region operations
        self.bind(Key::ctrl(' '), set_mark);  // C-space
        self.bind(Key::ctrl('w'), kill_region);
        self.bind(Key::meta('w'), copy_region);

        // Search
        self.bind(Key::ctrl('s'), search_forward);
        self.bind(Key::ctrl('r'), search_backward);
        self.bind(Key::meta('%'), query_replace);  // M-%

        // Buffer operations
        self.bind(Key::ctlx_ctrl('f'), find_file);
        self.bind(Key::ctlx('b'), switch_buffer);
        self.bind(Key::ctlx_ctrl('b'), list_buffers);  // C-x C-b
        self.bind(Key::ctlx('k'), kill_buffer);

        // Go to line
        self.bind(Key::meta('g'), goto_line);

        // Window operations
        self.bind(Key::ctlx('2'), split_window);
        self.bind(Key::ctlx('1'), delete_other_windows);
        self.bind(Key::ctlx('0'), delete_window);
        self.bind(Key::ctlx('o'), other_window);

        // Undo
        self.bind(Key::ctrl('/'), undo);  // C-/
        self.bind(Key::ctrl('_'), undo);  // C-_ (same as C-/ in many terminals)

        // Shell
        self.bind(Key::meta('!'), shell_command);  // M-!

        // Keyboard macros
        self.bind(Key::ctlx('('), start_macro);    // C-x (
        self.bind(Key::ctlx(')'), end_macro);      // C-x )
        self.bind(Key::ctlx('e'), execute_macro);  // C-x e
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
    pub fn redraw_display(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
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

        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        if f && n == 0 {
            // Kill from BOL to cursor
            if let Some(line) = editor.current_buffer().line(cursor_line) {
                let killed = line.text()[..cursor_col].to_string();
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                    line_mut.delete_range(0, cursor_col);
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
            // Find current char index
            let cur_idx = chars.iter().position(|(pos, _)| *pos == cursor_col);
            match cur_idx {
                Some(i) if i > 0 => (i - 1, i),
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
                    let killed = line.text()[start_col..end_col].to_string();
                    editor.kill_append(&killed);
                }
                if let Some(line) = editor.current_buffer_mut().line_mut(start_line) {
                    line.delete_range(start_col, end_col);
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
                    let killed = line.text()[start_col..end_col].to_string();
                    editor.kill_prepend(&killed);
                }
                if let Some(line) = editor.current_buffer_mut().line_mut(start_line) {
                    line.delete_range(start_col, end_col);
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
                let line_text = line.text();
                let start = if line_idx == start_line { start_col } else { 0 };
                let end = if line_idx == end_line { end_col.min(line_text.len()) } else { line_text.len() };

                if start < line_text.len() {
                    text.push_str(&line_text[start..end]);
                }

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

    /// Query replace (search and replace with confirmation)
    pub fn query_replace(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Query replace", crate::editor::PromptAction::QueryReplaceSearch, None);
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

    /// Execute shell command
    pub fn shell_command(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
        editor.start_prompt("Shell command", crate::editor::PromptAction::ShellCommand, None);
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
        // Remove this key from macro recording (it shouldn't be part of the macro)
        editor.macro_state.keys.pop();
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
    pub fn execute_macro(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
        // Execute n times (or 1 if no argument)
        let count = if n > 0 { n } else { 1 };
        for _ in 0..count {
            editor.execute_macro()?;
        }
        Ok(CommandStatus::Success)
    }
}
