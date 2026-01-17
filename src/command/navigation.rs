//! Navigation commands - cursor movement

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;

/// Helper: check if character is a word character
pub(crate) fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

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
            col = pos + ch.len_utf8();
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
                            break;
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

        editor.current_window_mut().set_cursor(cursor_line, cursor_col);
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
                            break;
                        }
                    }
                }
            } else if cursor_line > 0 {
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

        editor.current_window_mut().set_cursor(cursor_line, cursor_col);
    }

    editor.ensure_cursor_visible();
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

/// Scroll the other window (M-C-v)
pub fn scroll_other_window(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
    if editor.windows.len() <= 1 {
        editor.display.set_message("No other window");
        return Ok(CommandStatus::Failure);
    }

    let other_idx = if editor.current_window + 1 < editor.windows.len() {
        editor.current_window + 1
    } else {
        0
    };

    let buf_idx = editor.windows[other_idx].buffer_idx();
    let line_count = editor.buffers[buf_idx].line_count();
    let height = editor.windows[other_idx].height() as usize;

    let scroll_amount = if f {
        n.unsigned_abs() as usize
    } else {
        height.saturating_sub(2)
    };

    if n >= 0 || !f {
        editor.windows[other_idx].scroll_down(scroll_amount, line_count);
    } else {
        editor.windows[other_idx].scroll_up(scroll_amount);
    }

    editor.display.force_redraw();
    Ok(CommandStatus::Success)
}

/// Jump to matching fence character (bracket, paren, brace) (M-C-f)
pub fn goto_matching_fence(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    let cursor_line = editor.current_window().cursor_line();
    let cursor_col = editor.current_window().cursor_col();

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
        let mut line_idx = cursor_line;
        let mut col = cursor_col + ch.len_utf8();

        while line_idx < line_count && depth > 0 {
            if let Some(line) = editor.current_buffer().line(line_idx) {
                let text = line.text();
                for (pos, c) in text[col..].char_indices() {
                    if c == ch {
                        depth += 1;
                    } else if c == target {
                        depth -= 1;
                        if depth == 0 {
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
        let mut line_idx = cursor_line;
        let mut search_up_to = cursor_col;

        loop {
            if let Some(line) = editor.current_buffer().line(line_idx) {
                let text = line.text();
                let slice = &text[..search_up_to];

                for (pos, c) in slice.char_indices().rev() {
                    if c == ch {
                        depth += 1;
                    } else if c == target {
                        depth -= 1;
                        if depth == 0 {
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
            search_up_to = editor.current_buffer()
                .line(line_idx)
                .map(|l| l.len())
                .unwrap_or(0);
        }
    }

    editor.display.set_message("No matching fence found");
    Ok(CommandStatus::Failure)
}
