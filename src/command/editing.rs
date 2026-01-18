//! Editing commands - text modification operations

use crate::editor::EditorState;
use crate::error::Result;
use crate::line::Line;
use super::CommandStatus;
use super::navigation::{forward_word, backward_word};

/// Delete character at cursor (forward)
pub fn delete_char_forward(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
    if n < 0 {
        return delete_char_backward(editor, f, -n);
    }

    editor.display.force_redraw();
    let start_line = editor.current_window().cursor_line();

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
            if let Some(ch) = editor.current_buffer_mut().delete_char(cursor_line, cursor_col) {
                if f {
                    editor.kill_append(&ch.to_string());
                }
            }
        } else if cursor_line + 1 < editor.current_buffer().line_count() {
            editor.current_buffer_mut().join_line(cursor_line);
            if f {
                editor.kill_append("\n");
            }
        } else {
            return Ok(CommandStatus::Failure);
        }
    }

    editor.invalidate_syntax_from(start_line);
    Ok(CommandStatus::Success)
}

/// Delete character before cursor (backward)
pub fn delete_char_backward(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
    if n < 0 {
        return delete_char_forward(editor, f, -n);
    }

    editor.display.force_redraw();
    let start_line = editor.current_window().cursor_line();

    if f {
        editor.start_kill();
    }

    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        if cursor_col > 0 {
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

    let end_line = editor.current_window().cursor_line();
    editor.invalidate_syntax_from(end_line.min(start_line));
    Ok(CommandStatus::Success)
}

/// Kill to end of line
pub fn kill_line(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
    editor.start_kill();
    editor.display.force_redraw();

    let cursor_line = editor.current_window().cursor_line();
    let cursor_col = editor.current_window().cursor_col();

    if f && n == 0 {
        if let Some(line) = editor.current_buffer().line(cursor_line) {
            let killed = line.safe_slice_to(cursor_col).to_string();
            let actual_end = killed.len();
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                if actual_end > 0 {
                    line_mut.delete_range(0, actual_end);
                }
            }
            editor.current_buffer_mut().set_modified(true);
            editor.current_window_mut().set_cursor(cursor_line, 0);
            editor.kill_append(&killed);
        }
    } else if f && n > 0 {
        for _ in 0..n {
            let cursor_line = editor.current_window().cursor_line();
            if let Some(killed) = editor.current_buffer_mut().kill_to_eol(cursor_line, 0) {
                editor.kill_append(&killed);
            }
        }
    } else {
        if let Some(killed) = editor
            .current_buffer_mut()
            .kill_to_eol(cursor_line, cursor_col)
        {
            editor.kill_append(&killed);
        }
    }

    editor.invalidate_syntax_from(cursor_line);
    Ok(CommandStatus::Success)
}

/// Yank killed text
pub fn yank(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    if n < 0 {
        return Ok(CommandStatus::Failure);
    }

    let text = match editor.yank_text() {
        Some(t) => t.to_string(),
        None => return Ok(CommandStatus::Success),
    };

    let start_line = editor.current_window().cursor_line();
    let start_col = editor.current_window().cursor_col();

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

    let end_line = editor.current_window().cursor_line();
    let end_col = editor.current_window().cursor_col();
    editor.last_yank_start = Some((start_line, start_col));
    editor.last_yank_end = Some((end_line, end_col));
    editor.last_was_yank = true;
    editor.reset_kill_ring_idx();

    editor.invalidate_syntax_from(start_line);
    editor.ensure_cursor_visible();
    Ok(CommandStatus::Success)
}

/// Cycle through kill ring after yank (M-y)
pub fn yank_pop(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    if !editor.last_was_yank {
        editor.display.set_message("Previous command was not a yank");
        return Ok(CommandStatus::Failure);
    }

    let (start_line, start_col) = match editor.last_yank_start {
        Some(pos) => pos,
        None => return Ok(CommandStatus::Failure),
    };

    let (end_line, end_col) = match editor.last_yank_end {
        Some(pos) => pos,
        None => return Ok(CommandStatus::Failure),
    };

    let new_idx = editor.cycle_kill_ring();
    let new_text = match editor.yank_text_at(new_idx) {
        Some(t) => t.to_string(),
        None => return Ok(CommandStatus::Failure),
    };

    for line_idx in (start_line..=end_line).rev() {
        if line_idx == start_line && line_idx == end_line {
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                line_mut.delete_range(start_col, end_col);
            }
        } else if line_idx == end_line {
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                line_mut.delete_range(0, end_col);
            }
            if line_idx > 0 {
                editor.current_buffer_mut().join_line(line_idx - 1);
            }
        } else if line_idx == start_line {
            if let Some(line_mut) = editor.current_buffer_mut().line_mut(line_idx) {
                let line_len = line_mut.len();
                line_mut.delete_range(start_col, line_len);
            }
        } else {
            editor.current_buffer_mut().delete_line(line_idx);
        }
    }

    editor.current_window_mut().set_cursor(start_line, start_col);

    for ch in new_text.chars() {
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

    let new_end_line = editor.current_window().cursor_line();
    let new_end_col = editor.current_window().cursor_col();
    editor.last_yank_end = Some((new_end_line, new_end_col));
    editor.last_was_yank = true;

    editor.current_buffer_mut().set_modified(true);
    editor.ensure_cursor_visible();
    Ok(CommandStatus::Success)
}

/// Insert newline and move cursor to new line
pub fn newline(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    if n < 0 {
        return Ok(CommandStatus::Failure);
    }

    let start_line = editor.current_window().cursor_line();

    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        editor
            .current_buffer_mut()
            .insert_newline(cursor_line, cursor_col);
        editor.current_window_mut().set_cursor(cursor_line + 1, 0);
    }

    editor.invalidate_syntax_from(start_line);
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

    for _ in 0..n {
        editor
            .current_buffer_mut()
            .insert_newline(cursor_line, cursor_col);
    }

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

        editor
            .current_buffer_mut()
            .insert_newline(cursor_line, cursor_col);
        editor.current_window_mut().set_cursor(cursor_line + 1, 0);

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

    let (idx1, idx2) = if cursor_col >= text.len() {
        (len - 2, len - 1)
    } else {
        let cur_idx = chars.iter().position(|(pos, ch)| {
            *pos <= cursor_col && cursor_col < *pos + ch.len_utf8()
        });
        match cur_idx {
            Some(i) if i > 0 => (i - 1, i),
            Some(0) if len > 1 => (0, 1),
            _ => return Ok(CommandStatus::Failure),
        }
    };

    let ch1 = chars[idx1].1;
    let ch2 = chars[idx2].1;

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

    if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
        *line_mut = Line::from(new_text.as_str());
    }
    editor.current_buffer_mut().set_modified(true);

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

/// Zap to character - delete from cursor up to and including specified char (M-z)
pub fn zap_to_char(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    editor.display.set_message("Zap to char: ");
    editor.display.force_redraw();

    let target = if let Ok(Some(key)) = editor.read_key_for_describe() {
        if let Some(ch) = key.base_char() {
            ch
        } else {
            editor.display.set_message("Aborted");
            return Ok(CommandStatus::Abort);
        }
    } else {
        editor.display.set_message("Aborted");
        return Ok(CommandStatus::Abort);
    };

    let count = n.abs().max(1) as usize;
    let forward = n >= 0;

    editor.start_kill();

    for _ in 0..count {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();
        let line_count = editor.current_buffer().line_count();

        if forward {
            let mut found = false;
            let mut search_line = cursor_line;
            let mut search_col = cursor_col;

            'outer: while search_line < line_count {
                if let Some(line) = editor.current_buffer().line(search_line) {
                    let text = line.text();
                    let start = if search_line == cursor_line { search_col } else { 0 };

                    for (pos, ch) in text[start..].char_indices() {
                        if ch == target {
                            let end_col = start + pos + ch.len_utf8();

                            if search_line == cursor_line {
                                let killed = text[cursor_col..end_col].to_string();
                                editor.kill_append(&killed);
                                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                    line_mut.delete_range(cursor_col, end_col);
                                }
                                editor.current_buffer_mut().set_modified(true);
                            } else {
                                let mut killed = String::new();
                                if let Some(start_line) = editor.current_buffer().line(cursor_line) {
                                    killed.push_str(&start_line.text()[cursor_col..]);
                                    killed.push('\n');
                                }
                                for mid_line in (cursor_line + 1)..search_line {
                                    if let Some(line) = editor.current_buffer().line(mid_line) {
                                        killed.push_str(line.text());
                                        killed.push('\n');
                                    }
                                }
                                killed.push_str(&text[..end_col]);
                                editor.kill_append(&killed);

                                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                    let len = line_mut.len();
                                    line_mut.delete_range(cursor_col, len);
                                }
                                for _ in (cursor_line + 1)..=search_line {
                                    editor.current_buffer_mut().join_line(cursor_line);
                                }
                                if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
                                    line_mut.delete_range(cursor_col, cursor_col + end_col);
                                }
                                editor.current_buffer_mut().set_modified(true);
                            }

                            found = true;
                            break 'outer;
                        }
                    }
                }
                search_line += 1;
                search_col = 0;
            }

            if !found {
                editor.display.set_message(&format!("'{}' not found", target));
                return Ok(CommandStatus::Failure);
            }
        } else {
            editor.display.set_message("Backward zap not yet implemented");
            return Ok(CommandStatus::Failure);
        }
    }

    editor.display.clear_message();
    Ok(CommandStatus::Success)
}

/// Kill word forward
pub fn kill_word(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    if n < 0 {
        return backward_kill_word(editor, false, -n);
    }

    editor.start_kill();
    editor.display.force_redraw();

    for _ in 0..n.max(1) {
        let start_line = editor.current_window().cursor_line();
        let start_col = editor.current_window().cursor_col();

        forward_word(editor, false, 1)?;

        let end_line = editor.current_window().cursor_line();
        let end_col = editor.current_window().cursor_col();

        // No word found (at end of buffer or no words after cursor)
        if start_line == end_line && start_col == end_col {
            return Ok(CommandStatus::Success);
        }

        editor.current_window_mut().set_cursor(start_line, start_col);

        if start_line == end_line {
            if let Some(line) = editor.current_buffer().line(start_line) {
                let killed = line.safe_slice(start_col, end_col).to_string();
                let actual_start = line.text().len() - line.safe_slice_from(start_col).len();
                let actual_end = actual_start + killed.len();
                editor.kill_append(&killed);
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(start_line) {
                    if actual_end > actual_start {
                        line_mut.delete_range(actual_start, actual_end);
                    }
                }
            }
            editor.current_buffer_mut().set_modified(true);
        } else {
            // Multi-line case: track end position which shifts as we join lines
            let mut target_line = end_line;
            let mut target_col = end_col;

            while editor.current_window().cursor_line() < target_line
                || (editor.current_window().cursor_line() == target_line
                    && editor.current_window().cursor_col() < target_col)
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
                    // About to join with next line - update target position
                    if target_line == cur_line + 1 {
                        // Target is on the next line, after join it will be on current line
                        target_line = cur_line;
                        target_col = line_len + target_col;
                    } else if target_line > cur_line {
                        // Target is further down, just decrement line number
                        target_line -= 1;
                    }
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
    editor.display.force_redraw();

    for _ in 0..n.max(1) {
        let end_line = editor.current_window().cursor_line();
        let end_col = editor.current_window().cursor_col();

        backward_word(editor, false, 1)?;

        let start_line = editor.current_window().cursor_line();
        let start_col = editor.current_window().cursor_col();

        // No word found (at beginning of buffer or no words before cursor)
        if start_line == end_line && start_col == end_col {
            return Ok(CommandStatus::Success);
        }

        if start_line == end_line {
            if let Some(line) = editor.current_buffer().line(start_line) {
                let killed = line.safe_slice(start_col, end_col).to_string();
                let actual_start = line.text().len() - line.safe_slice_from(start_col).len();
                let actual_end = actual_start + killed.len();
                editor.kill_prepend(&killed);
                if let Some(line_mut) = editor.current_buffer_mut().line_mut(start_line) {
                    if actual_end > actual_start {
                        line_mut.delete_range(actual_start, actual_end);
                    }
                }
            }
            editor.current_buffer_mut().set_modified(true);
        } else {
            // Multi-line case: track end position which shifts as we join lines
            let mut target_line = end_line;
            let mut target_col = end_col;
            let mut deleted = String::new();

            while editor.current_window().cursor_line() < target_line
                || (editor.current_window().cursor_line() == target_line
                    && editor.current_window().cursor_col() < target_col)
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
                    // About to join with next line - update target position
                    if target_line == cur_line + 1 {
                        target_line = cur_line;
                        target_col = line_len + target_col;
                    } else if target_line > cur_line {
                        target_line -= 1;
                    }
                    editor.current_buffer_mut().join_line(cur_line);
                    deleted.push('\n');
                }
            }
            editor.kill_prepend(&deleted);
        }
    }

    Ok(CommandStatus::Success)
}

/// Fill (reflow) the current paragraph
pub fn fill_paragraph(editor: &mut EditorState, _f: bool, _n: i32) -> Result<CommandStatus> {
    editor.fill_paragraph(72); // Default fill column
    Ok(CommandStatus::Success)
}

/// Transpose words (M-t)
pub fn transpose_words(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        let text = match editor.current_buffer().line(cursor_line) {
            Some(l) => l.text().to_string(),
            None => return Ok(CommandStatus::Failure),
        };

        let chars: Vec<(usize, char)> = text.char_indices().collect();
        if chars.is_empty() {
            return Ok(CommandStatus::Failure);
        }

        let cur_char_idx = chars
            .iter()
            .position(|(pos, _)| *pos >= cursor_col)
            .unwrap_or(chars.len());

        let mut word1_end = cur_char_idx;
        while word1_end > 0 && chars.get(word1_end.saturating_sub(1)).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
            word1_end = word1_end.saturating_sub(1);
        }
        if word1_end == 0 && chars.get(0).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
            return Ok(CommandStatus::Failure);
        }

        let mut word1_start = word1_end;
        while word1_start > 0 && !chars.get(word1_start.saturating_sub(1)).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
            word1_start = word1_start.saturating_sub(1);
        }

        let mut word2_start = word1_end;
        while word2_start < chars.len() && chars.get(word2_start).map(|(_, c)| c.is_whitespace()).unwrap_or(false) {
            word2_start += 1;
        }
        if word2_start >= chars.len() {
            return Ok(CommandStatus::Failure);
        }

        let mut word2_end = word2_start;
        while word2_end < chars.len() && !chars.get(word2_end).map(|(_, c)| c.is_whitespace()).unwrap_or(true) {
            word2_end += 1;
        }

        let byte_word1_start = chars[word1_start].0;
        let byte_word1_end = if word1_end < chars.len() { chars[word1_end].0 } else { text.len() };
        let byte_word2_start = chars[word2_start].0;
        let byte_word2_end = if word2_end < chars.len() { chars[word2_end].0 } else { text.len() };

        let word1 = &text[byte_word1_start..byte_word1_end];
        let between = &text[byte_word1_end..byte_word2_start];
        let word2 = &text[byte_word2_start..byte_word2_end];

        let mut new_text = String::new();
        new_text.push_str(&text[..byte_word1_start]);
        new_text.push_str(word2);
        new_text.push_str(between);
        new_text.push_str(word1);
        new_text.push_str(&text[byte_word2_end..]);

        if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
            *line_mut = Line::from(new_text.clone());
        }
        editor.current_buffer_mut().set_modified(true);

        let new_cursor = byte_word1_start + word2.len() + between.len() + word1.len();
        editor.current_window_mut().set_cursor(cursor_line, new_cursor);
    }

    Ok(CommandStatus::Success)
}

/// Transpose lines (C-x C-t)
pub fn transpose_lines(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();

        if cursor_line == 0 {
            return Ok(CommandStatus::Failure);
        }

        let line_count = editor.current_buffer().line_count();
        if cursor_line >= line_count {
            return Ok(CommandStatus::Failure);
        }

        let line1_text = editor
            .current_buffer()
            .line(cursor_line - 1)
            .map(|l| l.text().to_string())
            .unwrap_or_default();
        let line2_text = editor
            .current_buffer()
            .line(cursor_line)
            .map(|l| l.text().to_string())
            .unwrap_or_default();

        if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line - 1) {
            *line_mut = Line::from(line2_text);
        }
        if let Some(line_mut) = editor.current_buffer_mut().line_mut(cursor_line) {
            *line_mut = Line::from(line1_text);
        }
        editor.current_buffer_mut().set_modified(true);

        if cursor_line + 1 < line_count {
            editor.current_window_mut().set_cursor(cursor_line + 1, 0);
        }
    }

    Ok(CommandStatus::Success)
}

/// Copy current line to kill ring (C-x C-k)
pub fn copy_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    let cursor_line = editor.current_window().cursor_line();
    let count = n.max(1) as usize;
    let line_count = editor.current_buffer().line_count();

    let mut copied = String::new();
    for i in 0..count {
        let line_idx = cursor_line + i;
        if line_idx >= line_count {
            break;
        }
        if let Some(line) = editor.current_buffer().line(line_idx) {
            copied.push_str(line.text());
            copied.push('\n');
        }
    }

    if !copied.is_empty() {
        editor.start_kill();
        editor.kill_append(&copied);
        let actual_count = count.min(line_count - cursor_line);
        editor.display.set_message(&format!("Copied {} line(s)", actual_count));
    }

    Ok(CommandStatus::Success)
}

/// Duplicate current line (C-x d)
pub fn duplicate_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    let cursor_line = editor.current_window().cursor_line();
    let cursor_col = editor.current_window().cursor_col();
    let count = n.max(1) as usize;

    let line_text = match editor.current_buffer().line(cursor_line) {
        Some(line) => line.text().to_string(),
        None => return Ok(CommandStatus::Failure),
    };

    for i in 0..count {
        let insert_at = cursor_line + 1 + i;
        editor.current_buffer_mut().insert_line_at(insert_at);
        if let Some(line) = editor.current_buffer_mut().line_mut(insert_at) {
            line.append_str(&line_text);
        }
    }

    editor.current_buffer_mut().set_modified(true);
    editor.current_window_mut().set_cursor(cursor_line + 1, cursor_col);
    editor.display.set_message(&format!("Duplicated {} time(s)", count));

    Ok(CommandStatus::Success)
}

/// Split line at cursor, cursor stays in place (M-C-o)
pub fn split_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();
        let cursor_col = editor.current_window().cursor_col();

        editor.current_buffer_mut().insert_newline(cursor_line, cursor_col);
        editor.current_window_mut().set_cursor(cursor_line, cursor_col);
    }

    Ok(CommandStatus::Success)
}

/// Join current line to previous, removing leading whitespace (M-^)
pub fn join_line(editor: &mut EditorState, _f: bool, n: i32) -> Result<CommandStatus> {
    for _ in 0..n.max(1) {
        let cursor_line = editor.current_window().cursor_line();

        if cursor_line == 0 {
            return Ok(CommandStatus::Failure);
        }

        let prev_line_len = editor
            .current_buffer()
            .line(cursor_line - 1)
            .map(|l| l.len())
            .unwrap_or(0);

        let current_text = editor
            .current_buffer()
            .line(cursor_line)
            .map(|l| {
                let text = l.text();
                text.trim_start().to_string()
            })
            .unwrap_or_default();

        if let Some(prev_line) = editor.current_buffer_mut().line_mut(cursor_line - 1) {
            if !current_text.is_empty() {
                if prev_line.len() > 0 {
                    prev_line.append_str(" ");
                }
                prev_line.append_str(&current_text);
            }
        }

        editor.current_buffer_mut().delete_line(cursor_line);
        editor.current_buffer_mut().set_modified(true);
        editor.current_window_mut().set_cursor(cursor_line - 1, prev_line_len);
    }

    Ok(CommandStatus::Success)
}

/// Indent region rigidly (C-x TAB)
pub fn indent_rigidly(editor: &mut EditorState, f: bool, n: i32) -> Result<CommandStatus> {
    let region = super::mark::get_region(editor);

    let (start_line, end_line) = match region {
        Some((sl, _, el, _)) => (sl, el),
        None => {
            let cursor_line = editor.current_window().cursor_line();
            (cursor_line, cursor_line)
        }
    };

    let indent_amount = if f { n } else { 4 };

    for line_idx in start_line..=end_line {
        if indent_amount > 0 {
            let spaces = " ".repeat(indent_amount as usize);
            if let Some(line) = editor.current_buffer_mut().line_mut(line_idx) {
                let old_text = line.text().to_string();
                line.clear();
                line.append_str(&spaces);
                line.append_str(&old_text);
            }
        } else if indent_amount < 0 {
            let remove_count = (-indent_amount) as usize;
            if let Some(line) = editor.current_buffer_mut().line_mut(line_idx) {
                let text = line.text();
                let leading_spaces = text.chars().take_while(|c| *c == ' ').count();
                let to_remove = leading_spaces.min(remove_count);
                if to_remove > 0 {
                    line.delete_range(0, to_remove);
                }
            }
        }
    }

    editor.current_buffer_mut().set_modified(true);
    editor.display.set_message(&format!(
        "Indented {} lines by {}",
        end_line - start_line + 1,
        indent_amount
    ));

    Ok(CommandStatus::Success)
}
