//! Terminal abstraction using crossterm

use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyEvent},
    execute, queue,
    style::Print,
    terminal::{self, ClearType},
};

use crate::error::Result;

/// Terminal wrapper for cross-platform terminal I/O
pub struct Terminal {
    /// Terminal width in columns
    cols: u16,
    /// Terminal height in rows
    rows: u16,
}

impl Terminal {
    /// Create a new terminal instance and enter raw mode
    pub fn new() -> Result<Self> {
        terminal::enable_raw_mode()?;
        let (cols, rows) = terminal::size()?;

        let mut term = Self { cols, rows };
        term.enter_alternate_screen()?;
        term.hide_cursor()?;

        Ok(term)
    }

    /// Enter alternate screen buffer
    fn enter_alternate_screen(&mut self) -> Result<()> {
        execute!(io::stdout(), terminal::EnterAlternateScreen)?;
        Ok(())
    }

    /// Leave alternate screen buffer
    fn leave_alternate_screen(&mut self) -> Result<()> {
        execute!(io::stdout(), terminal::LeaveAlternateScreen)?;
        Ok(())
    }

    /// Hide the cursor
    fn hide_cursor(&mut self) -> Result<()> {
        execute!(io::stdout(), cursor::Hide)?;
        Ok(())
    }

    /// Show the cursor
    fn show_cursor(&mut self) -> Result<()> {
        execute!(io::stdout(), cursor::Show)?;
        Ok(())
    }

    /// Get terminal width
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Get terminal height
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Update terminal size (call after resize event)
    pub fn update_size(&mut self) -> Result<()> {
        let (cols, rows) = terminal::size()?;
        self.cols = cols;
        self.rows = rows;
        Ok(())
    }

    /// Clear the entire screen
    pub fn clear_screen(&mut self) -> Result<()> {
        queue!(io::stdout(), terminal::Clear(ClearType::All))?;
        Ok(())
    }

    /// Clear from cursor to end of line
    pub fn clear_to_eol(&mut self) -> Result<()> {
        queue!(io::stdout(), terminal::Clear(ClearType::UntilNewLine))?;
        Ok(())
    }

    /// Move cursor to position (0-indexed)
    pub fn move_cursor(&mut self, row: u16, col: u16) -> Result<()> {
        queue!(io::stdout(), cursor::MoveTo(col, row))?;
        Ok(())
    }

    /// Write a string at current cursor position
    pub fn write_str(&mut self, s: &str) -> Result<()> {
        queue!(io::stdout(), Print(s))?;
        Ok(())
    }

    /// Write a single character
    pub fn write_char(&mut self, ch: char) -> Result<()> {
        queue!(io::stdout(), Print(ch))?;
        Ok(())
    }

    /// Flush output buffer to terminal
    pub fn flush(&mut self) -> Result<()> {
        io::stdout().flush()?;
        Ok(())
    }

    /// Set cursor visibility
    pub fn set_cursor_visible(&mut self, visible: bool) -> Result<()> {
        if visible {
            queue!(io::stdout(), cursor::Show)?;
        } else {
            queue!(io::stdout(), cursor::Hide)?;
        }
        Ok(())
    }

    /// Read a key event (blocking)
    pub fn read_key(&mut self) -> Result<KeyEvent> {
        loop {
            match event::read()? {
                Event::Key(key_event) => return Ok(key_event),
                Event::Resize(cols, rows) => {
                    self.cols = cols;
                    self.rows = rows;
                    // Continue waiting for key event
                }
                _ => {
                    // Ignore other events (mouse, focus, etc.)
                }
            }
        }
    }

    /// Check if a key is available (non-blocking)
    pub fn poll_key(&mut self, timeout: std::time::Duration) -> Result<bool> {
        Ok(event::poll(timeout)?)
    }

    /// Set reverse video mode
    pub fn set_reverse(&mut self, enabled: bool) -> Result<()> {
        use crossterm::style::{Attribute, SetAttribute};
        if enabled {
            queue!(io::stdout(), SetAttribute(Attribute::Reverse))?;
        } else {
            queue!(io::stdout(), SetAttribute(Attribute::NoReverse))?;
        }
        Ok(())
    }

    /// Set dim/faint mode (for line numbers, etc.)
    pub fn set_dim(&mut self, enabled: bool) -> Result<()> {
        use crossterm::style::{Attribute, SetAttribute};
        if enabled {
            queue!(io::stdout(), SetAttribute(Attribute::Dim))?;
        } else {
            queue!(io::stdout(), SetAttribute(Attribute::NormalIntensity))?;
        }
        Ok(())
    }

    /// Reset all attributes
    pub fn reset_attributes(&mut self) -> Result<()> {
        use crossterm::style::{Attribute, SetAttribute};
        queue!(io::stdout(), SetAttribute(Attribute::Reset))?;
        Ok(())
    }

    /// Set foreground color
    pub fn set_foreground(&mut self, color: crate::syntax::Color) -> Result<()> {
        use crossterm::style::{Color as XColor, SetForegroundColor};
        let xcolor = Self::convert_color(color);
        queue!(io::stdout(), SetForegroundColor(xcolor))?;
        Ok(())
    }

    /// Set background color
    pub fn set_background(&mut self, color: crate::syntax::Color) -> Result<()> {
        use crossterm::style::{Color as XColor, SetBackgroundColor};
        let xcolor = Self::convert_color(color);
        queue!(io::stdout(), SetBackgroundColor(xcolor))?;
        Ok(())
    }

    /// Set bold attribute
    pub fn set_bold(&mut self, enabled: bool) -> Result<()> {
        use crossterm::style::{Attribute, SetAttribute};
        if enabled {
            queue!(io::stdout(), SetAttribute(Attribute::Bold))?;
        } else {
            queue!(io::stdout(), SetAttribute(Attribute::NormalIntensity))?;
        }
        Ok(())
    }

    /// Apply a complete style
    pub fn apply_style(&mut self, style: &crate::syntax::Style) -> Result<()> {
        // Reset first to clear any previous styling
        if style.reverse {
            self.set_reverse(true)?;
        }
        if style.fg != crate::syntax::Color::Default {
            self.set_foreground(style.fg)?;
        }
        if style.bg != crate::syntax::Color::Default {
            self.set_background(style.bg)?;
        }
        if style.bold {
            self.set_bold(true)?;
        }
        Ok(())
    }

    /// Convert our Color enum to crossterm's Color
    fn convert_color(color: crate::syntax::Color) -> crossterm::style::Color {
        use crate::syntax::Color;
        use crossterm::style::Color as XColor;
        match color {
            Color::Default => XColor::Reset,
            Color::Black => XColor::Black,
            Color::Red => XColor::DarkRed,
            Color::Green => XColor::DarkGreen,
            Color::Yellow => XColor::DarkYellow,
            Color::Blue => XColor::DarkBlue,
            Color::Magenta => XColor::DarkMagenta,
            Color::Cyan => XColor::DarkCyan,
            Color::White => XColor::Grey,
            Color::BrightBlack => XColor::DarkGrey,
            Color::BrightRed => XColor::Red,
            Color::BrightGreen => XColor::Green,
            Color::BrightYellow => XColor::Yellow,
            Color::BrightBlue => XColor::Blue,
            Color::BrightMagenta => XColor::Magenta,
            Color::BrightCyan => XColor::Cyan,
            Color::BrightWhite => XColor::White,
        }
    }

    /// Sound the bell
    pub fn beep(&mut self) -> Result<()> {
        print!("\x07");
        self.flush()?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        // Restore terminal state
        let _ = self.show_cursor();
        let _ = self.leave_alternate_screen();
        let _ = terminal::disable_raw_mode();
    }
}
