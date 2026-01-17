//! Window commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;

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
