//! Miscellaneous commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;
use super::mark::get_region;

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

/// Toggle line numbers display (C-x #)
pub fn toggle_line_numbers(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.display.toggle_line_numbers();
    editor.force_redraw();
    let status = if editor.display.show_line_numbers {
        "Line numbers enabled"
    } else {
        "Line numbers disabled"
    };
    editor.display.set_message(status);
    Ok(CommandStatus::Success)
}

/// Toggle syntax highlighting (C-x s)
pub fn toggle_syntax_highlighting(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.syntax.toggle();
    editor.force_redraw();
    let status = if editor.syntax.enabled {
        "Syntax highlighting enabled"
    } else {
        "Syntax highlighting disabled"
    };
    editor.display.set_message(status);
    Ok(CommandStatus::Success)
}

/// Quit the editor
pub fn quit(editor: &mut EditorState, f: bool, _n: i32) -> Result<CommandStatus> {
    // With prefix argument (C-u), force quit without checking
    if f {
        editor.force_quit();
        return Ok(CommandStatus::Success);
    }

    // Check for unsaved buffers if warning is enabled
    if editor.warn_unsaved && editor.has_modified_buffers() {
        let modified = editor.modified_buffer_names();
        let msg = if modified.len() == 1 {
            format!("Buffer {} modified; really quit? (y/n)", modified[0])
        } else {
            format!("{} buffers modified; really quit? (y/n)", modified.len())
        };
        editor.start_prompt(&msg, crate::editor::PromptAction::ConfirmQuit, None);
        return Ok(CommandStatus::Success);
    }

    editor.quit();
    Ok(CommandStatus::Success)
}

/// Toggle unsaved buffer warning (C-x w)
pub fn toggle_warn_unsaved(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.toggle_warn_unsaved();
    Ok(CommandStatus::Success)
}

/// Abort current operation
pub fn abort(_editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    Ok(CommandStatus::Abort)
}

/// Execute shell command
pub fn shell_command(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.start_prompt("Shell command", crate::editor::PromptAction::ShellCommand, None);
    Ok(CommandStatus::Success)
}

/// Execute a command by name (M-x)
pub fn execute_extended_command(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.start_prompt("M-x", crate::editor::PromptAction::ExtendedCommand, None);
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
        let line_count = editor.current_buffer().line_count();
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

