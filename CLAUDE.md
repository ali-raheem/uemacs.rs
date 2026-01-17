# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Goal

Port uEmacs/PK 4.0 from C to Rust for modern platforms. The original C code serves as the reference implementation in `c-reference/`.

## Progress Summary

**Status: ~99% Complete** - A fully functional text editor with comprehensive Emacs keybindings.

### What Works

| Category | Features |
|----------|----------|
| **Navigation** | Cursor (C-f/b/n/p, arrows), words (M-f/b), lines (C-a/e), back-to-indent (M-m), pages (C-v, M-v), buffer (M-<, M->), paragraphs (M-{ M-}), matching fence (M-C-f) |
| **Editing** | Insert, delete (C-d, Backspace), kill line (C-k), yank (C-y), kill word (M-d), transpose (C-t), quote (C-q), fill paragraph (M-q), just-one-space (M-SPC), delete-horizontal-space (M-\), delete-blank-lines (C-x C-o), tab-to-tab-stop (M-i), trim-line (C-x t), zap-to-char (M-z) |
| **Case** | Upcase word (M-u), downcase word (M-l), capitalize (M-c), upcase region (C-x C-u), downcase region (C-x C-l) |
| **Mark/Region** | Set mark (C-space), kill region (C-w), copy region (M-w), exchange point/mark (C-x C-x) |
| **Search** | Incremental search (C-s, C-r), query replace (M-%), replace-string (M-r), hunt forward/backward (M-s, M-S) |
| **Files** | Open (C-x C-f), save (C-x C-s), write-file (C-x C-w), insert (C-x i), read-only toggle (C-x C-q), quit (C-x C-c) |
| **Shell** | Shell command (M-!), filter buffer (C-x \|) |
| **Buffers** | Switch (C-x b), list (C-x C-b), kill (C-x k), next/prev (C-x n/p), goto line (M-g), shell command (M-!) |
| **Info/Help** | Cursor position (C-x =), describe-key (M-?), describe-bindings (F1), word-count (M-=) |
| **Windows** | Split (C-x 2), delete (C-x 0/1), switch (C-x o), enlarge (C-x ^), shrink (C-x v) |
| **Undo** | Undo (C-/ or C-_) with operation grouping |
| **Macros** | Record (C-x (), stop (C-x )), execute (C-x e), store to slot (C-x M-s), load from slot (C-x M-l) |
| **Prefix** | Universal argument (C-u) for repeat counts |

### Potential Future Enhancements
- Rectangle operations
- Customizable key bindings file
- M-x command execution by name
- Auto-save

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
M-{/}  paragraph         M-SPC  just one space   M-?      describe key
                         M-\    del horiz space
                         M-i    tab to tab stop
                         C-x C-o del blank lines

Search:              Windows:              Case:
C-s    search fwd        C-x 2  split            M-u    upcase word
C-r    search back       C-x 1  delete others    M-l    downcase word
M-%    query replace     C-x 0  delete window    M-c    capitalize word
M-r    replace string    C-x o  other window     C-x C-u upcase region
M-s    hunt forward      C-x ^  enlarge window   C-x C-l downcase region
M-S    hunt backward     C-x v  shrink window

Macros:              Mark/Region:          Buffers:
C-x (  start recording   C-space  set mark       C-x n  next buffer
C-x )  stop recording    C-x C-x  exchange       C-x p  prev buffer
C-x e  execute macro
C-x M-s store to slot    Files:                Shell:
C-x M-l load from slot   C-x i  insert file     C-x |  filter buffer

Help/Info:           Navigation:           Other:
F1     describe bindings M-C-f  match fence    C-u      prefix arg
M-?    describe key      M-{/}  paragraph      C-g      abort
M-=    word count                               C-l      refresh
C-x =  cursor position                          C-/ C-_  undo
C-x t  trim line
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
