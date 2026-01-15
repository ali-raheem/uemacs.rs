# uEmacs-rs

A Rust port of [uEmacs/PK](https://git.kernel.org/pub/scm/editors/uemacs/uemacs.git), the lightweight Emacs-style text editor.

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

## Building

```bash
cargo build --release
```

## Usage

```bash
# Open a file
cargo run --release -- filename.txt

# Or run without a file (starts with *scratch* buffer)
cargo run --release
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

Based on uEmacs/PK 4.0. See original source for license details.
