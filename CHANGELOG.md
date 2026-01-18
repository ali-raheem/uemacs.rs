# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- **Region Highlighting**
  - Selected text between mark and cursor is now visually highlighted
  - Uses reverse video styling for clear visibility
  - Foundation infrastructure for future syntax highlighting
- **Universal Argument (C-u)**
  - `C-u` - Prefix argument (default 4)
  - `C-u C-u` - Multiply by 4 (16)
  - `C-u 5` - Explicit numeric argument
  - `C-u 1 2` - Multi-digit argument (12)
  - `M-0` to `M-9` - Digit argument (quick numeric prefix)
  - `M--` - Negative argument (make next command use negative count)
  - Works with all commands (e.g., `C-u 10 C-f` moves 10 characters)
  - Works with self-insert (e.g., `C-u 5 a` inserts "aaaaa")

- **Case Operations**
  - `M-u` - Upcase word at cursor
  - `M-l` - Downcase word at cursor
  - `M-c` - Capitalize word at cursor
  - `C-x C-u` - Upcase region
  - `C-x C-l` - Downcase region

- **Mark/Cursor Operations**
  - `C-x C-x` - Exchange point and mark (swap cursor and mark positions)
  - `C-x h` - Mark whole buffer (select entire buffer)

- **Utility Commands**
  - `C-x =` - Show cursor position info (line, column, character code)
  - `M-SPC` - Just one space (delete surrounding whitespace, leave exactly one space)
  - `M-\` - Delete horizontal space (delete all spaces/tabs around cursor)
  - `C-x C-o` - Delete blank lines (delete blank lines around cursor)
  - `M-i` - Tab to tab stop (insert spaces to next tab stop)
  - `M-?` - Describe key (show what command a key is bound to)
  - `F1` - Describe bindings (list all key bindings in a help buffer)
  - `M-=` - Word count (count words, lines, and characters in buffer or region)
  - `C-x t` - Trim line (remove trailing whitespace; with C-u, trim all lines)

- **Navigation Enhancements**
  - `M-C-f` - Goto matching fence (jump to matching bracket, paren, or brace)

- **Display**
  - `C-l` - Recenter (center cursor line on screen; with C-u n, put cursor on line n from top)
  - `C-x #` - Toggle line numbers (show/hide line numbers in left margin)

- **Extended Command**
  - `M-x` - Execute extended command (run any command by name)

- **Configuration**
  - Configuration file support (`~/.uemacs.conf` or `%USERPROFILE%\.uemacs.conf` on Windows)
  - Supported settings: `line-numbers`, `auto-save`, `auto-save-interval`, `tab-width`, `warn-unsaved`

- **Unsaved Buffer Warning**
  - Warning prompt when quitting with unsaved buffers (answer y/n)
  - Warning prompt when killing a modified buffer
  - `C-x w` - Toggle unsaved buffer warnings on/off
  - `C-u C-x C-c` - Force quit without warning (bypass check)
  - Configurable via `warn-unsaved = true/false` in config file

- **File Operations**
  - `C-x i` - Insert file (insert file contents at cursor position)

- **Shell Integration**
  - `M-|` - Shell command on region (pipe region through shell command; with C-u, replace region with output)
  - `C-x |` - Filter buffer (pipe entire buffer contents through shell command)

- **Macro Enhancements**
  - `C-x M-s` - Store macro to slot (save current macro to slot 0-9)
  - `C-x M-l` - Load macro from slot (load macro from slot 0-9)
  - `C-u <n> C-x e` - Execute macro from slot n (0-9)

- **Persistent Macro Storage**
  - `C-x M-S` - Save all macros to disk (`~/.uemacs-macros`)
  - `C-x M-L` - Load macros from disk
  - Macros are automatically loaded on startup
  - Human-readable file format (one key per line)

- **Additional File Operations**
  - `C-x C-w` - Write file (Save As - save buffer to a new filename)
  - `M-~` - Not modified (clear the modification flag)
  - `C-x C-q` - Toggle read-only mode

- **Auto-Save**
  - Automatic periodic saving of modified buffers (every 30 seconds)
  - Saves to `#filename#` in same directory (Emacs-style)
  - `C-x a` - Toggle auto-save on/off
  - Auto-save files automatically deleted on proper save

- **Additional Navigation**
  - `M-m` - Back to indentation (move to first non-whitespace on line)
  - `C-x l` - What line (display current line number)

- **Line Joining**
  - `M-^` - Delete indentation / join line (join current line to previous, removing leading whitespace)

- **Window Operations (Multi-window)**
  - `M-C-v` - Scroll other window (scroll the other window down)

- **Transpose Operations**
  - `M-t` - Transpose words (swap word before cursor with word after)
  - `C-x C-t` - Transpose lines (swap current line with previous line)

- **Mark Operations**
  - `M-h` - Mark paragraph (set mark at paragraph start, point at end)
  - `M-@` - Mark word (set mark at point, move forward by word)

- **Kill Ring Enhancements**
  - `M-y` - Yank-pop (cycle through kill ring after yank)
  - `M-C-k` - Kill paragraph (kill from point to end of paragraph)
  - `M-C-w` - Append next kill (make next kill append to kill ring)
  - Consecutive kills now properly append to kill ring

- **Line Operations**
  - `M-C-o` - Split line (split line at cursor, cursor stays in place)

- **Indentation**
  - `C-x TAB` - Indent rigidly (indent/outdent region; use C-u for amount)

- **Additional Editing**
  - `M-z` - Zap to char (delete from cursor up to and including specified character)
  - `C-x d` - Duplicate line (duplicate current line below)
  - `C-x C-k` - Copy line (copy current line to kill ring)

- **Additional File/Buffer Operations**
  - `C-x C-r` - Revert buffer (reload file from disk)

- **Display Name**
  - App now displays as "uEmacs.rs" to differentiate from C version

- **Search Enhancements**
  - `M-r` - Replace string (non-interactive search and replace all)
  - `M-s` - Hunt forward (repeat last search forward)
  - `M-S` - Hunt backward (repeat last search backward)

- **Buffer Operations**
  - `C-x n` - Next buffer (cycle to next buffer)
  - `C-x p` - Previous buffer (cycle to previous buffer)

- **Window Operations**
  - `C-x ^` - Enlarge window (increase height by 1 line)
  - `C-x v` - Shrink window (decrease height by 1 line)

- **UTF-8 Safety**
  - `safe_slice()`, `safe_slice_to()`, `safe_slice_from()` methods in Line struct
  - `char_to_byte()`, `byte_to_char()` conversion helpers
  - Proper UTF-8 boundary detection for all string operations

- **Testing**
  - Unit tests for Line struct UTF-8 handling (10 tests)
  - Tests for emoji, Chinese characters, and boundary conditions

### Fixed
- M-d (kill word) hanging on blank lines due to stale line index after join operations
- M-Backspace (backward kill word) same fix for multi-line deletion
- UTF-8 string slicing in `kill_word`, `backward_kill_word`, `copy_region`
- Query replace multi-byte character insertion (was using char index as byte offset)
- Transpose chars UTF-8 boundary detection
- Fill paragraph line insertion (first line wasn't being inserted correctly)
- Macro recording cleanup (removed redundant `keys.pop()` in `start_macro`)

### Changed
- Updated CLAUDE.md with comprehensive feature documentation
- Updated README.md with expanded feature list
- Project status updated to ~98% complete

## [0.1.0] - 2024

### Added
- **Core Editor**
  - Full EditorState with buffers, windows, terminal, display
  - Event loop with key handling
  - Minibuffer prompts for user input

- **Navigation**
  - Character movement: `C-f`, `C-b`, arrow keys
  - Line movement: `C-n`, `C-p`, arrow keys
  - Word movement: `M-f`, `M-b`
  - Line boundaries: `C-a`, `C-e`, Home, End
  - Page movement: `C-v`, `M-v`, PageUp, PageDown
  - Buffer boundaries: `M-<`, `M->`
  - Paragraph movement: `M-{`, `M-}`

- **Editing**
  - Character insertion (self-insert)
  - Delete forward: `C-d`, Delete key
  - Delete backward: Backspace, `C-h`
  - Kill line: `C-k`
  - Kill word: `M-d`
  - Backward kill word: `M-Backspace`
  - Yank: `C-y`
  - Transpose characters: `C-t`
  - Quote character: `C-q`
  - Newline: Enter, `C-m`
  - Open line: `C-o`
  - Indent newline: `C-j`
  - Tab insertion: `C-i`
  - Fill paragraph: `M-q`

- **Mark and Region**
  - Set mark: `C-Space`
  - Kill region: `C-w`
  - Copy region: `M-w`

- **Search**
  - Incremental search forward: `C-s`
  - Incremental search backward: `C-r`
  - Query replace: `M-%`

- **File Operations**
  - Find file: `C-x C-f`
  - Save buffer: `C-x C-s`
  - Quit: `C-x C-c`

- **Buffer Operations**
  - Switch buffer: `C-x b`
  - List buffers: `C-x C-b`
  - Kill buffer: `C-x k`
  - Goto line: `M-g`

- **Window Operations**
  - Split window: `C-x 2`
  - Delete window: `C-x 0`
  - Delete other windows: `C-x 1`
  - Other window: `C-x o`

- **Undo System**
  - Undo: `C-/`, `C-_`
  - Operation grouping with boundaries
  - Per-buffer undo stack

- **Keyboard Macros**
  - Start recording: `C-x (`
  - Stop recording: `C-x )`
  - Execute macro: `C-x e`

- **Shell Integration**
  - Shell command: `M-!`
  - Output displayed in `*Shell Command Output*` buffer

- **Display**
  - Mode line with buffer name, modified flag, line/column
  - Visual feedback for C-x and ESC prefix keys
  - Screen refresh: `C-l`
  - Abort: `C-g`

- **Platform Support**
  - Windows key event filtering (Press only, not Release/Repeat)
  - Cross-platform terminal handling via crossterm

### Technical Details
- Written in Rust with crossterm for terminal I/O
- UTF-8 native text handling with unicode-width support
- Modular architecture: Line, Buffer, Window, Terminal, Display, Input, Command, Editor
- Command function signature: `fn(&mut EditorState, bool, i32) -> Result<CommandStatus>`

## Reference

This is a Rust port of [uEmacs/PK 4.0](https://git.kernel.org/pub/scm/editors/uemacs/uemacs.git) by Petri Kutvonen, which itself is based on MicroEMACS by Dave G. Conroy, modified by Daniel M. Lawrence.
