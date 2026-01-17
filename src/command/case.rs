//! Case transformation commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;
use super::navigation::forward_word;
use super::mark::get_region;

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
