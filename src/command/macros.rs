//! Keyboard macro and undo commands

use crate::editor::EditorState;
use crate::error::Result;
use super::CommandStatus;

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
