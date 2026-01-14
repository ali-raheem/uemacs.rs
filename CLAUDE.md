# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Goal

Port uEmacs/PK 4.0 from C to Rust for modern platforms. The original C code serves as the reference implementation.

## Original C Build (Reference Implementation)

```bash
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
| Command | `src/command.rs` | In progress - command dispatch |
| Editor | `src/editor.rs` | In progress - main state/loop |
| Error | `src/error.rs` | Complete - error types |
| Main | `src/main.rs` | Complete - entry point |

### Working Features

- Cursor movement (C-f, C-b, C-n, C-p, arrows)
- Line navigation (C-a, C-e, Home, End)
- Page movement (C-v, M-v, PageUp, PageDown)
- Buffer navigation (M-<, M->)
- Self-insert characters
- File loading
- Display/mode line
- Screen refresh (C-l)
- Quit (C-x C-c)

### In-Progress: Text Editing Commands

Commands implemented but need build verification:

| Command | Key | Function | Status |
|---------|-----|----------|--------|
| Delete forward | C-d, Del | `delete_char_forward` | Code added |
| Delete backward | Backspace, C-h | `delete_char_backward` | Code added |
| Kill line | C-k | `kill_line` | Code added |
| Yank | C-y | `yank` | Code added |
| Newline | Enter (C-m) | `newline` | Code added |
| Open line | C-o | `open_line` | Code added |
| Indent newline | C-j | `indent_newline` | Code added |
| Tab | Tab (C-i) | `insert_tab` | Code added |
| Transpose | C-t | `transpose_chars` | Code added |
| Quote char | C-q | `quote_char` | Code added |

**Next step:** Run `cargo build` to verify compilation.

### Uncompleted Tasks (Priority Order)

1. **Build and test** - Verify text editing commands compile and work
2. **Search/Replace** - Forward/reverse search, incremental search (isearch)
3. **File operations** - Save (C-x C-s), write-to, file switching
4. **Multiple buffers** - Buffer switching (C-x b), buffer list
5. **Multiple windows** - Window split (C-x 2), window switching (C-x o)
6. **Word operations** - Forward/backward word, kill word
7. **Region operations** - Set mark (C-space), kill region, copy region

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

#### Key Codes Reference

From `src/input.rs`:
- Backspace: `Key(0x7f)`
- Delete: `Key::special(0x53)`
- Enter: `Key::ctrl('m')`
- Tab: `Key::ctrl('i')`
- Arrows: `Key::special(0x48/0x50/0x4b/0x4d)` (Up/Down/Left/Right)

## Build Commands

```bash
cargo build          # Build debug
cargo build --release # Build release
cargo run             # Run editor
cargo run -- file.txt # Open file
```

