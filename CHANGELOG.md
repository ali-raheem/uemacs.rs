# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- **Universal Argument (C-u)**
  - `C-u` - Prefix argument (default 4)
  - `C-u C-u` - Multiply by 4 (16)
  - `C-u 5` - Explicit numeric argument
  - `C-u 1 2` - Multi-digit argument (12)
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

- **Utility Commands**
  - `C-x =` - Show cursor position info (line, column, character code)
  - `M-SPC` - Just one space (delete surrounding whitespace, leave exactly one space)

- **UTF-8 Safety**
  - `safe_slice()`, `safe_slice_to()`, `safe_slice_from()` methods in Line struct
  - `char_to_byte()`, `byte_to_char()` conversion helpers
  - Proper UTF-8 boundary detection for all string operations

- **Testing**
  - Unit tests for Line struct UTF-8 handling (10 tests)
  - Tests for emoji, Chinese characters, and boundary conditions

### Fixed
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
