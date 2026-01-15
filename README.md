# uEmacs-rs

[![Crates.io](https://img.shields.io/crates/v/uemacs.svg)](https://crates.io/crates/uemacs)
[![License](https://img.shields.io/crates/l/uemacs.svg)](https://github.com/ali-raheem/uemacs.rs/blob/master/LICENSE.txt)

A Rust port of [uEmacs/PK](https://git.kernel.org/pub/scm/editors/uemacs/uemacs.git), the lightweight Emacs-style text editor.

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

- Emacs-style keybindings (C-f, C-b, C-n, C-p, C-x, M-x, etc.)
- Multiple buffers and split windows
- Incremental search and query-replace
- Kill ring (clipboard) with yank
- Undo support with operation grouping
- Keyboard macros (record, playback)
- Word/region case operations (upcase, downcase, capitalize)
- Paragraph operations (movement, fill)
- Shell command execution
- UTF-8/Unicode text handling
- Cross-platform (Windows, Linux, macOS)

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

| Key | Action | Key | Action |
|-----|--------|-----|--------|
| C-f/C-b | Forward/backward char | C-x C-f | Open file |
| C-n/C-p | Next/previous line | C-x C-s | Save file |
| C-a/C-e | Beginning/end of line | C-x C-c | Quit |
| C-v/M-v | Page down/up | C-x b | Switch buffer |
| C-s/C-r | Search forward/backward | C-x 2 | Split window |
| C-k | Kill line | C-x o | Other window |
| C-y | Yank (paste) | C-x 1 | One window |
| C-w | Kill region | C-space | Set mark |
| C-/ | Undo | C-g | Abort |
| M-% | Query replace | C-l | Refresh screen |

**Note:** `C-` = Ctrl, `M-` = Alt or ESC prefix, `C-x` = Ctrl-X prefix

## License

Based on uEmacs/PK 4.0 by Petri Kutvonen, which is based on MicroEMACS 3.9 by Daniel M. Lawrence. Free for non-commercial use. See [LICENSE.txt](LICENSE.txt) for details.
