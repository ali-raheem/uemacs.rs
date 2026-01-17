//! File and buffer commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;

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

/// Insert file contents at cursor (C-x i)
pub fn insert_file(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.start_prompt("Insert file", crate::editor::PromptAction::InsertFile, None);
    Ok(CommandStatus::Success)
}

/// Kill buffer
pub fn kill_buffer(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    // Default to current buffer
    let default = Some(editor.current_buffer().name().to_string());
    editor.start_prompt("Kill buffer", crate::editor::PromptAction::KillBuffer, default);
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

/// Go to line number
pub fn goto_line(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.start_prompt("Goto line", crate::editor::PromptAction::GotoLine, None);
    Ok(CommandStatus::Success)
}
