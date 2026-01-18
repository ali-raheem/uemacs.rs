# uEmacs-rs

[![Crates.io](https://img.shields.io/crates/v/uemacs.svg)](https://crates.io/crates/uemacs)
[![License](https://img.shields.io/crates/l/uemacs.svg)](https://github.com/ali-raheem/uemacs.rs/blob/master/LICENSE.txt)

A complete loose re-write in Rust of [uEmacs/PK](https://git.kernel.org/pub/scm/editors/uemacs/uemacs.git), the lightweight Emacs-style text editor.
Mostly vibe coded with Claude Code.

## Installation

### From crates.io (Recommended)

```bash
cargo install uemacs
```

### From source

```bash
git clone https://github.com/ali-raheem/uemacs.rs.git
cd uemacs.rs
cargo build --release
```

## Features

- **Navigation** - Character, word, line, page, buffer, paragraph movement
- **Editing** - Kill/yank, transpose, fill paragraph, zap-to-char
- **Search** - Incremental search, query-replace, replace-string, hunt repeat
- **Buffers** - Multiple buffers, split windows, buffer cycling
- **Files** - Open, save, Save As, insert file, read-only toggle
- **Macros** - Record, playback, named slots (0-9), persistent storage
- **Case** - Upcase/downcase/capitalize word and region
- **Shell** - Execute commands, filter buffer through shell
- **Help** - Describe key, list all bindings
- **Undo** - Full undo with operation grouping
- **UTF-8** - Full Unicode text handling
- **Syntax Highlighting** - Built-in support for Rust, C/C++, Python, TOML, Markdown
- **Region Highlighting** - Visual selection between mark and cursor
- **Cross-platform** - Windows, Linux, macOS

## Usage

```bash
# Open a file
uemacs filename.txt

# Or run without a file (starts with *scratch* buffer)
uemacs

# Show help
uemacs --help
```

If running from source:
```bash
cargo run --release -- filename.txt
```

## Key Bindings

### Navigation
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-f/C-b | Forward/backward char | M-f/M-b | Forward/backward word |
| C-n/C-p | Next/previous line | C-v/M-v | Page down/up |
| C-a/C-e | Beginning/end of line | M-m | Back to indentation |
| M-</M-> | Beginning/end of buffer | M-{/M-} | Backward/forward paragraph |
| M-C-f | Goto matching fence | | |

### Editing
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-d | Delete char forward | C-k | Kill line |
| C-y | Yank (paste) | C-w | Kill region |
| M-w | Copy region | C-t | Transpose chars |
| M-d | Kill word | M-z | Zap to char |
| C-/ | Undo | M-q | Fill paragraph |
| C-x t | Trim trailing whitespace | | |

### Search & Replace
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-s/C-r | Search forward/backward | M-s/M-S | Hunt forward/backward |
| M-% | Query replace | M-r | Replace string (all) |

### Files & Buffers
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-x C-f | Open file | C-x C-s | Save file |
| C-x C-w | Write file (Save As) | C-x i | Insert file |
| C-x b | Switch buffer | C-x C-b | List buffers |
| C-x k | Kill buffer | C-x n/p | Next/prev buffer |
| C-x C-q | Toggle read-only | C-x C-c | Quit |

### Windows
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-x 2 | Split window | C-x 1 | One window |
| C-x 0 | Delete window | C-x o | Other window |
| C-x ^ | Enlarge window | C-x v | Shrink window |

### Macros
| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-x ( | Start recording | C-x ) | Stop recording |
| C-x e | Execute macro | C-x M-s | Store to slot 0-9 |
| C-x M-l | Load from slot | C-x M-S | Save macros to file |
| C-x M-L | Load macros from file | | |

### Help
| Key | Action |
|-----|--------|
| F1 | List all key bindings |
| M-? | Describe key |
| M-= | Word count |
| C-x = | Cursor position info |

### Display
| Key | Action |
|-----|--------|
| C-x # | Toggle line numbers |
| C-x s | Toggle syntax highlighting |

**Note:** `C-` = Ctrl, `M-` = Alt or ESC prefix, `C-x` = Ctrl-X prefix

## Configuration

uEmacs.rs reads configuration from `~/.uemacs.conf` (or `%USERPROFILE%\.uemacs.conf` on Windows):

```ini
# Display settings
line-numbers = true
syntax-highlighting = true

# Auto-save (saves to #filename# periodically)
auto-save = true
auto-save-interval = 30

# Tab width for display
tab-width = 8

# Warn before closing modified buffers
warn-unsaved = true
```

## Persistent Macros

Keyboard macros can be saved to disk and automatically loaded on startup.

**File location:** `~/.uemacs-macros` (or `%USERPROFILE%\.uemacs-macros` on Windows)

**Workflow:**
1. Record a macro with `C-x (` ... `C-x )`
2. Store it to a slot (0-9) with `C-x M-s`
3. Save all slots to disk with `C-x M-S`
4. Macros are automatically loaded when uEmacs starts

**File format:** Human-readable text, one key per line:
```ini
# uEmacs.rs keyboard macros

[macro.0]
C-a
C-k
C-n
C-y

[macro.3]
M-f
M-d
```

## License

To stay in line with spirit of the original license of uEmacs/PK 4.0 whos code was used as reference this is licensed under PolyForm Noncommercial 1.0.0.
Please see LICENSE.txt, I hope to release this under a more permissive license if possible.

Based on uEmacs/PK 4.0 by Petri Kutvonen, which is based on MicroEMACS 3.9 by Daniel M. Lawrence. Free for non-commercial use.

### Original Copyright Notice for uEmacs/PK 4.0

Copyright Notices:

MicroEMACS 3.9 (c) Copyright 1987 Daniel M. Lawrence.
Reference Manual Copyright 1987 by Brian Straight and
Daniel M. Lawrence. No copyright claimed for modifications
made by Petri H. Kutvonen.

Original statement of copying policy:

MicroEMACS 3.9 can be copied and distributed freely for any
non-commercial purposes. MicroEMACS 3.9 can only be incorporated
into commercial software with the permission of the current author
Daniel M. Lawrence].