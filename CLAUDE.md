# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Goal

Port uEmacs/PK 4.0 from C to Rust for modern platforms. The original C code serves as the reference implementation in `c-reference/`.

## Progress Summary

**Status: ~98% Complete** - A fully functional text editor with comprehensive Emacs keybindings.

### What Works

| Category | Features |
|----------|----------|
| **Navigation** | Cursor (C-f/b/n/p, arrows), words (M-f/b), lines (C-a/e), pages (C-v, M-v), buffer (M-<, M->), paragraphs (M-{ M-}) |
| **Editing** | Insert, delete (C-d, Backspace), kill line (C-k), yank (C-y), kill word (M-d), transpose (C-t), quote (C-q), fill paragraph (M-q), just-one-space (M-SPC) |
| **Case** | Upcase word (M-u), downcase word (M-l), capitalize (M-c), upcase region (C-x C-u), downcase region (C-x C-l) |
| **Mark/Region** | Set mark (C-space), kill region (C-w), copy region (M-w), exchange point/mark (C-x C-x) |
| **Search** | Incremental search (C-s, C-r), query replace (M-%) |
| **Files** | Open (C-x C-f), save (C-x C-s), quit (C-x C-c) |
| **Buffers** | Switch (C-x b), list (C-x C-b), kill (C-x k), goto line (M-g), shell command (M-!), cursor position (C-x =) |
| **Windows** | Split (C-x 2), delete (C-x 0/1), switch (C-x o) |
| **Undo** | Undo (C-/ or C-_) with operation grouping |
| **Macros** | Record (C-x (), stop (C-x )), execute (C-x e) |

### Potential Future Enhancements

- Numeric argument prefix (C-u)
- Named macros and macro persistence
- Rectangle operations
- Customizable key bindings file
- Tab/detab operations

### Key Bindings Quick Reference

```
Navigation:          Editing:              Files/Buffers:
C-f/b  char left/right   C-d    delete char      C-x C-f  find file
C-n/p  line down/up      C-k    kill line        C-x C-s  save
C-a/e  line start/end    C-y    yank             C-x C-b  list buffers
M-f/b  word fwd/back     C-w    kill region      C-x b    switch buffer
C-v    page down         M-w    copy region      C-x k    kill buffer
M-v    page up           C-t    transpose        C-x C-c  quit
M-</>  buffer start/end  C-q    quote char       C-x =    cursor position
M-{/}  paragraph         M-SPC  just one space

Search:              Windows:              Case:
C-s    search fwd        C-x 2  split            M-u    upcase word
C-r    search back       C-x 1  delete others    M-l    downcase word
M-%    query replace     C-x 0  delete window    M-c    capitalize word
                         C-x o  other window     C-x C-u upcase region
                                                 C-x C-l downcase region

Macros:              Mark/Region:          Other:
C-x (  start recording   C-space  set mark       C-g      abort
C-x )  stop recording    C-x C-x  exchange       C-l      refresh
C-x e  execute macro                             C-/ C-_  undo
```

## Build Commands

```bash
cargo build              # Build debug
cargo build --release    # Build release
cargo run                # Run editor
cargo run -- file.txt    # Open file
```

## Architecture Overview

### Rust Modules

| Module | File | Purpose |
|--------|------|---------|
| Line | `src/line.rs` | UTF-8 aware line representation |
| Buffer | `src/buffer.rs` | Text storage, editing ops, undo stack |
| Window | `src/window.rs` | Viewport into buffer with cursor |
| Terminal | `src/terminal.rs` | Crossterm-based terminal I/O |
| Display | `src/display.rs` | Screen rendering, mode line |
| Input | `src/input.rs` | Key translation, prefix handling |
| Command | `src/command.rs` | Command functions, key bindings |
| Editor | `src/editor.rs` | Main state, event loop |
| Error | `src/error.rs` | Error types |

### Key Design Patterns

**Command Signature:**
```rust
pub type CommandFn = fn(&mut EditorState, bool, i32) -> Result<CommandStatus>;
// &mut EditorState: editor context
// bool (f): true if numeric argument provided
// i32 (n): repeat count (default 1)
```

**Kill Ring:** `Vec<String>` with consecutive kills appending to same entry.

**Undo Stack:** Per-buffer stack of `UndoEntry` variants (Insert, Delete, InsertNewline, DeleteNewline, Boundary).

**Search State:** Embedded in EditorState with pattern, direction, origin position for abort.

**Query Replace:** Two-phase prompt (search string, then replacement), interactive y/n/!/q responses.

### Platform Notes

**Windows:** Input handler filters `KeyEventKind::Press` only (crossterm sends Press/Release/Repeat on Windows). Visual feedback ("C-x -", "ESC -") shown for pending prefix keys.

## C Reference

Original C code in `c-reference/`. Key files:
- `main.c` - Entry point, command loop
- `display.c` - Screen update
- `buffer.c`, `line.c` - Text storage
- `basic.c`, `random.c` - Commands
- `search.c`, `isearch.c` - Search
- `bind.c` - Key bindings

Build with `cd c-reference && make`.
