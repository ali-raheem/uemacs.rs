//! Mark and region commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;
use super::navigation::forward_word;

/// Set mark at current cursor position
pub fn set_mark(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.current_window_mut().set_mark();
    editor.display.set_message("Mark set");
    Ok(CommandStatus::Success)
}

/// Helper: get region bounds (start_line, start_col, end_line, end_col)
/// Returns None if mark is not set
pub(crate) fn get_region(editor: &EditorState) -> Option<(usize, usize, usize, usize)> {
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
pub(crate) fn collect_region_text(editor: &EditorState, start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> String {
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

/// Make next kill command append to kill ring (M-C-w)
pub fn append_next_kill(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    // Set flag so next kill appends instead of creating new entry
    editor.last_was_kill = true;
    editor.display.set_message("Next kill appends");
    Ok(CommandStatus::Success)
}
