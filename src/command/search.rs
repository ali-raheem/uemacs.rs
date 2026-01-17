//! Search commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;

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
