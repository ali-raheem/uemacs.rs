# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Goal

Port uEmacs/PK 4.0 from C to Rust for modern platforms. The original C code serves as the reference implementation.

## Original C Build (Reference Implementation)

The original C code is in the `c-reference/` subdirectory.

```bash
cd c-reference
make          # Build (output: em)
make clean    # Clean artifacts
```

## C Architecture Reference

Understanding the original C architecture is essential for the Rust port.

### Core Data Structures

- **`struct line`** (line.h): Circular doubly-linked list of text lines. Lines don't store newlines; they're implied. Access macros: `lforw()`, `lback()`, `lgetc()`, `lputc()`, `llength()`.

- **`struct buffer`** (estruct.h): Open file/buffer containing linked list of lines (`b_linep`), cursor position (`b_dotp`/`b_doto`), mark, mode flags, filename. Buffers linked via `b_bufp`.

- **`struct window`** (estruct.h): Display windows (split views) with own dot/mark into a buffer. Windows linked via `w_wndp`.

- **`struct terminal`** (estruct.h): Terminal abstraction with function pointers for I/O. Global `term` accessed via macros: `TTopen`, `TTputc`, `TTmove`, `TTeeol`, etc.

### C Source Organization

| File | Purpose |
|------|---------|
| main.c | Entry point, command loop, initialization |
| display.c | Screen update, virtual terminal |
| buffer.c | Buffer management |
| window.c | Window operations |
| line.c | Line manipulation, kill buffer, yank |
| file.c, fileio.c | File I/O |
| search.c | Search/replace, regex (MAGIC mode) |
| isearch.c | Incremental search |
| input.c | Keyboard input, minibuffer |
| bind.c | Key binding, startup files |
| exec.c | Macro execution |
| eval.c | Variable evaluation |
| word.c | Word/paragraph operations |
| region.c | Region operations |
| random.c | Misc commands (tabs, indent, fences) |
| basic.c | Cursor movement |
| spawn.c | Shell commands |
| tcap.c/posix.c | Terminal backend (Unix) |

### Command Function Signature

All C commands use:
```c
int command_name(int f, int n);
// f: was numeric argument provided?
// n: repeat count (default 1)
// Returns: TRUE, FALSE, or ABORT
```

### Key Representation

Keys are integers with modifier flags:
- `CONTROL` (0x10000000)
- `META` (0x20000000) - Alt/Escape prefix
- `CTLX` (0x40000000) - C-x prefix
- `SPEC` (0x80000000) - Function keys

### Feature Flags (estruct.h)

- `MAGIC`: Regex support
- `CRYPT`: File encryption
- `ISRCH`: Incremental search
- `WORDPRO`: Paragraph fill/justify
- `FILOCK`: File locking

## Rust Port Considerations

### Suggested Crate Dependencies

- **crossterm** or **termion**: Cross-platform terminal handling
- **unicode-segmentation**: Proper Unicode/grapheme handling
- **regex**: Search with MAGIC mode

### Data Structure Mapping

| C Structure | Rust Approach |
|-------------|---------------|
| Circular linked list of lines | `Vec<String>` or rope data structure |
| Global mutable state | Consider `RefCell`, or pass context structs |
| Function pointer tables | Trait objects or enum dispatch |
| Preprocessor conditionals | Cargo features |

### Key Differences to Address

1. **Memory safety**: C uses manual allocation; Rust ownership eliminates this
2. **Global state**: C has many globals (edef.h); Rust should use explicit state passing or controlled interior mutability
3. **Terminal I/O**: Replace termcap/curses with crossterm for true cross-platform support
4. **Unicode**: Original is byte-oriented with partial UTF-8; Rust should be Unicode-native
5. **Error handling**: C returns TRUE/FALSE/ABORT; Rust should use `Result<T, E>`

## Current Rust Port Status

### Completed Modules

| Module | File | Status |
|--------|------|--------|
| Line | `src/line.rs` | Complete - UTF-8 aware line representation |
| Buffer | `src/buffer.rs` | Complete - includes editing methods |
| Window | `src/window.rs` | Complete - viewport with scrolling |
| Terminal | `src/terminal.rs` | Complete - crossterm-based |
| Display | `src/display.rs` | Complete - screen rendering |
| Input | `src/input.rs` | Complete - key translation |
| Command | `src/command.rs` | Complete - 30+ commands implemented |
| Editor | `src/editor.rs` | Complete - main state/loop with search |
| Error | `src/error.rs` | Complete - error types |
| Main | `src/main.rs` | Complete - entry point |

### Working Features

**Navigation:**
- Cursor movement (C-f, C-b, C-n, C-p, arrows)
- Line navigation (C-a, C-e, Home, End)
- Page movement (C-v, M-v, PageUp, PageDown)
- Buffer navigation (M-<, M->)
- Word movement (M-f, M-b)

**Editing:**
- Self-insert characters
- Delete forward/backward (C-d, Del, Backspace, C-h)
- Kill line (C-k) with kill ring
- Yank (C-y)
- Kill/copy word (M-d, M-Backspace)
- Newline (Enter), open line (C-o), indent newline (C-j)
- Tab insertion (Tab)
- Transpose characters (C-t)
- Quote literal character (C-q)

**Mark/Region:**
- Set mark (C-space)
- Kill region (C-w)
- Copy region (M-w)

**Search:**
- Incremental search forward (C-s)
- Incremental search backward (C-r)
- Query replace (M-%) - search and replace with confirmation
- Search wraps around buffer
- Backspace removes from pattern
- C-g aborts, Enter exits at match

**File Operations:**
- File loading (command line argument)
- File save (C-x C-s)
- Find file (C-x C-f) - open existing or create new
- Display/mode line
- Screen refresh (C-l)
- Quit (C-x C-c)

**Buffer Operations:**
- Switch buffer (C-x b)
- Kill buffer (C-x k)
- Go to line (M-g)

**Multiple Windows:**
- Split window (C-x 2)
- Delete other windows (C-x 1)
- Delete window (C-x 0)
- Other window (C-x o)

**Undo:**
- Undo (C-/ or C-_)
- Supports undoing insert, delete, newline, line joins
- Boundary markers group operations

### Uncompleted Tasks (Priority Order)

1. **List buffers** - C-x C-b (buffer list in special buffer)
2. **Paragraph operations** - M-{ M-} (paragraph movement), M-q (fill)
3. **Shell command** - M-! (shell-command)
4. **Keyboard macros** - C-x ( start, C-x ) end, C-x e execute

### Architectural Decisions

#### Kill Ring Design

The kill ring uses a simple `Vec<String>` approach:

```rust
// In EditorState
pub kill_ring: Vec<String>,      // Killed text entries
pub kill_ring_idx: usize,        // Current position
pub last_was_kill: bool,         // Track consecutive kills for appending
```

- Consecutive kills (C-k C-k) append to the same entry
- `start_kill()` creates new entry or continues appending
- `kill_append()` adds to end, `kill_prepend()` adds to start (for backward kills)
- `yank_text()` returns the most recent kill entry

#### Quote Mode Design

For C-q (insert literal character):

```rust
pub quote_pending: bool,  // In EditorState
```

- `quote_char` command sets `quote_pending = true`
- `handle_key` checks this flag first and inserts next key literally

#### Command Signature

Rust commands follow the C convention:

```rust
pub type CommandFn = fn(&mut EditorState, bool, i32) -> Result<CommandStatus>;
// &mut EditorState: editor context
// bool (f): true if numeric argument provided
// i32 (n): repeat count (default 1)
// Returns: Success, Failure, or Abort
```

#### Undo Stack Design

Undo uses a simple stack-based approach in Buffer:

```rust
pub enum UndoEntry {
    Insert { line: usize, col: usize, text: String },     // Text inserted
    Delete { line: usize, col: usize, text: String },     // Text deleted
    InsertNewline { line: usize, col: usize },            // Line split
    DeleteNewline { line: usize, col: usize },            // Lines joined
    Boundary,                                              // Group marker
}
```

- Each edit operation pushes an undo entry
- `undo()` reverses operations until hitting a boundary or empty stack
- `add_undo_boundary()` groups consecutive operations
- Recording is disabled during undo itself (prevents redo-as-undo issues)

#### Incremental Search Design

Search state is embedded in EditorState:

```rust
pub struct SearchState {
    pub active: bool,           // In search mode?
    pub pattern: String,        // Current search pattern
    pub direction: SearchDirection,  // Forward or Backward
    pub origin_line: usize,     // Starting position (for abort)
    pub origin_col: usize,
}
```

- `start_search(direction)` enters search mode
- `handle_search_key()` processes keys during search
- C-s/C-r repeats search in forward/backward direction
- Backspace removes from pattern and re-searches from origin
- C-g aborts (restores origin), Enter exits at current position

#### Windows Key Input Fix

On Windows, crossterm sends multiple event types (Press, Release, Repeat) for each keystroke. The input handler filters to only process `KeyEventKind::Press` events to prevent double-execution of commands.

Visual feedback is shown when prefix keys are pending:
- "C-x -" appears when waiting for C-x continuation
- "ESC -" appears when waiting for Meta/ESC continuation

#### Key Codes Reference

From `src/input.rs`:
- Backspace: `Key(0x7f)`
- Delete: `Key::special(0x53)`
- Enter: `Key::ctrl('m')`
- Tab: `Key::ctrl('i')`
- Arrows: `Key::special(0x48/0x50/0x4b/0x4d)` (Up/Down/Left/Right)
- Meta + key: `Key::meta('x')` or `Key(0x2000_0000 | char)`

## Build Commands

```bash
cargo build          # Build debug
cargo build --release # Build release
cargo run             # Run editor
cargo run -- file.txt # Open file
```

