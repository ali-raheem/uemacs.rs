//! uEmacs.rs - MicroEMACS text editor ported to Rust
//!
//! Based on uEmacs/PK 4.0 by Petri Kutvonen

mod buffer;
mod command;
mod display;
mod editor;
mod error;
mod input;
mod line;
mod terminal;
mod window;

use std::env;
use std::path::PathBuf;
use std::process;

use editor::EditorState;
use error::Result;
use terminal::Terminal;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Handle --help and --version
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            "--version" | "-V" => {
                print_version();
                return Ok(());
            }
            _ => {}
        }
    }

    // Initialize terminal
    let terminal = Terminal::new()?;

    // Create editor state
    let mut editor = EditorState::new(terminal);

    // Open file if provided
    if args.len() > 1 && !args[1].starts_with('-') {
        let path = PathBuf::from(&args[1]);
        if let Err(_) = editor.open_file(&path) {
            // File doesn't exist - create new buffer with that filename
            editor.open_new_file(&path);
        }
    }

    // Run the editor
    editor.run()?;

    Ok(())
}

fn print_usage() {
    println!("uEmacs.rs {} - MicroEMACS text editor in Rust", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Usage: uemacs [OPTIONS] [FILE]");
    println!();
    println!("Options:");
    println!("  -h, --help     Show this help message");
    println!("  -V, --version  Show version information");
    println!();
    println!("Key bindings:");
    println!("  C-f, Right     Move forward one character");
    println!("  C-b, Left      Move backward one character");
    println!("  C-n, Down      Move to next line");
    println!("  C-p, Up        Move to previous line");
    println!("  C-a, Home      Move to beginning of line");
    println!("  C-e, End       Move to end of line");
    println!("  C-v, PageDown  Scroll down one page");
    println!("  M-v, PageUp    Scroll up one page");
    println!("  M-<            Move to beginning of buffer");
    println!("  M->            Move to end of buffer");
    println!("  C-u            Universal argument (repeat count)");
    println!("  C-l            Redraw screen");
    println!("  C-g            Abort current operation");
    println!("  C-x C-c        Quit");
    println!();
    println!("Press F1 in editor for complete key binding list");
}

fn print_version() {
    println!("uEmacs.rs {}", env!("CARGO_PKG_VERSION"));
    println!("A Rust port of uEmacs/PK 4.0");
    println!();
    println!("Original MicroEMACS by Dave G. Conroy");
    println!("Modified by Daniel M. Lawrence");
    println!("Enhanced by Petri H. Kutvonen");
    println!("Rust port by Ali Raheem & Claude");
}
