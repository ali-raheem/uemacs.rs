//! Input handling - key reading and translation

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Key modifier flags (matching original C version)
pub mod key_flags {
    pub const CONTROL: u32 = 0x1000_0000;
    pub const META: u32 = 0x2000_0000;
    pub const CTLX: u32 = 0x4000_0000;
    pub const SPEC: u32 = 0x8000_0000;
}

/// Represents a key input with modifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub u32);

impl Key {
    /// Create a key from a character
    pub fn char(ch: char) -> Self {
        Key(ch as u32)
    }

    /// Create a control key (C-x)
    pub fn ctrl(ch: char) -> Self {
        Key(key_flags::CONTROL | ch.to_ascii_lowercase() as u32)
    }

    /// Create a meta key (M-x or ESC x)
    pub fn meta(ch: char) -> Self {
        Key(key_flags::META | ch.to_ascii_lowercase() as u32)
    }

    /// Create a C-x prefixed key (C-x x)
    pub fn ctlx(ch: char) -> Self {
        Key(key_flags::CTLX | ch.to_ascii_lowercase() as u32)
    }

    /// Create a C-x C-x key (C-x C-x)
    pub fn ctlx_ctrl(ch: char) -> Self {
        Key(key_flags::CTLX | key_flags::CONTROL | ch.to_ascii_lowercase() as u32)
    }

    /// Create a C-x M-x key (C-x ESC x or C-x Alt-x)
    pub fn ctlx_meta(ch: char) -> Self {
        Key(key_flags::CTLX | key_flags::META | ch.to_ascii_lowercase() as u32)
    }

    /// Create a special key (function keys, etc.)
    pub fn special(code: u32) -> Self {
        Key(key_flags::SPEC | code)
    }

    /// Get the raw key code
    pub fn code(&self) -> u32 {
        self.0
    }

    /// Check if this is a control key
    pub fn is_ctrl(&self) -> bool {
        self.0 & key_flags::CONTROL != 0
    }

    /// Check if this is a meta key
    pub fn is_meta(&self) -> bool {
        self.0 & key_flags::META != 0
    }

    /// Check if this is a C-x prefixed key
    pub fn is_ctlx(&self) -> bool {
        self.0 & key_flags::CTLX != 0
    }

    /// Check if this is a special key
    pub fn is_special(&self) -> bool {
        self.0 & key_flags::SPEC != 0
    }

    /// Get the base character (without modifiers)
    pub fn base_char(&self) -> Option<char> {
        let code = self.0 & 0x00FF_FFFF;
        if code <= 0x10FFFF {
            char::from_u32(code)
        } else {
            None
        }
    }

    /// Check if this is a printable self-insert character
    pub fn is_self_insert(&self) -> bool {
        // Not a modified key (except plain character)
        if self.0 & 0xF000_0000 != 0 {
            return false;
        }
        // Check if it's a printable character
        if let Some(ch) = char::from_u32(self.0) {
            ch >= ' ' && ch != '\x7f'
        } else {
            false
        }
    }

    /// Convert key to a human-readable string (e.g., "C-f", "M-x", "C-x C-s")
    pub fn display_name(&self) -> String {
        let mut result = String::new();

        // Handle C-x prefix
        if self.is_ctlx() {
            result.push_str("C-x ");
        }

        // Handle Meta prefix
        if self.is_meta() {
            result.push_str("M-");
        }

        // Handle Control prefix (not C-x, that's handled above)
        if self.is_ctrl() && !self.is_ctlx() {
            result.push_str("C-");
        } else if self.is_ctrl() && self.is_ctlx() {
            // C-x C-something
            result.push_str("C-");
        }

        // Handle special keys
        if self.is_special() {
            let code = self.0 & 0xFF;
            let special_name = match code {
                0x47 => "Home",
                0x48 => "Up",
                0x49 => "PageUp",
                0x4b => "Left",
                0x4d => "Right",
                0x4f => "End",
                0x50 => "Down",
                0x51 => "PageDown",
                0x53 => "Delete",
                n if n >= 0x3b && n <= 0x44 => {
                    return format!("{}F{}", result, n - 0x3a);
                }
                _ => return format!("{}special-0x{:02x}", result, code),
            };
            result.push_str(special_name);
            return result;
        }

        // Handle base character
        let base = self.0 & 0x00FF_FFFF;
        if base == 0x7f {
            result.push_str("Backspace");
        } else if base == 0x20 {
            result.push_str("SPC");
        } else if let Some(ch) = char::from_u32(base) {
            result.push(ch);
        } else {
            result.push_str(&format!("0x{:x}", base));
        }

        result
    }
}

/// Input state for handling multi-key sequences
pub struct InputState {
    /// Waiting for C-x continuation
    ctlx_pending: bool,
    /// Waiting for Meta continuation (after ESC)
    meta_pending: bool,
    /// Waiting for C-x Meta continuation (C-x ESC sequence)
    ctlx_meta_pending: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            ctlx_pending: false,
            meta_pending: false,
            ctlx_meta_pending: false,
        }
    }

    /// Reset input state
    pub fn reset(&mut self) {
        self.ctlx_pending = false;
        self.meta_pending = false;
        self.ctlx_meta_pending = false;
    }

    /// Check if waiting for continuation key
    pub fn is_pending(&self) -> bool {
        self.ctlx_pending || self.meta_pending || self.ctlx_meta_pending
    }

    /// Check if waiting for C-x continuation
    pub fn is_ctlx_pending(&self) -> bool {
        self.ctlx_pending
    }

    /// Check if waiting for Meta/ESC continuation
    pub fn is_meta_pending(&self) -> bool {
        self.meta_pending
    }

    /// Translate a crossterm KeyEvent to our Key representation
    pub fn translate_key(&mut self, event: KeyEvent) -> Option<Key> {
        let KeyEvent {
            code, modifiers, kind, ..
        } = event;

        // Only process key press events, ignore release and repeat
        // This is critical on Windows where crossterm sends all event types
        if kind != KeyEventKind::Press {
            return None;
        }

        // Handle based on current state
        if self.ctlx_meta_pending {
            self.ctlx_meta_pending = false;
            // C-x ESC <key> -> C-x M-<key>
            return self.translate_with_ctlx_meta(code, modifiers);
        }

        if self.meta_pending {
            self.meta_pending = false;
            return self.translate_with_meta(code, modifiers);
        }

        if self.ctlx_pending {
            self.ctlx_pending = false;
            // Check if ESC is pressed after C-x - start C-x M- sequence
            if code == KeyCode::Esc {
                self.ctlx_meta_pending = true;
                return None; // Wait for next key
            }
            return self.translate_with_ctlx(code, modifiers);
        }

        // Check for ESC (starts Meta sequence)
        if code == KeyCode::Esc {
            self.meta_pending = true;
            return None; // Wait for next key
        }

        // Check for C-x (starts C-x sequence)
        if code == KeyCode::Char('x') && modifiers.contains(KeyModifiers::CONTROL) {
            self.ctlx_pending = true;
            return None; // Wait for next key
        }

        // Normal key translation
        self.translate_normal(code, modifiers)
    }

    fn translate_normal(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);

        match code {
            KeyCode::Char(ch) => {
                if ctrl && alt {
                    Some(Key(key_flags::META | key_flags::CONTROL | ch.to_ascii_lowercase() as u32))
                } else if ctrl {
                    Some(Key::ctrl(ch))
                } else if alt {
                    Some(Key::meta(ch))
                } else {
                    Some(Key::char(ch))
                }
            }
            KeyCode::Enter => Some(Key::ctrl('m')),
            KeyCode::Tab => Some(Key::ctrl('i')),
            KeyCode::Backspace => Some(Key(0x7f)), // DEL
            KeyCode::Delete => Some(Key::special(0x53)),
            KeyCode::Home => Some(Key::special(0x47)),
            KeyCode::End => Some(Key::special(0x4f)),
            KeyCode::PageUp => Some(Key::special(0x49)),
            KeyCode::PageDown => Some(Key::special(0x51)),
            KeyCode::Up => Some(Key::special(0x48)),
            KeyCode::Down => Some(Key::special(0x50)),
            KeyCode::Left => Some(Key::special(0x4b)),
            KeyCode::Right => Some(Key::special(0x4d)),
            KeyCode::F(n) => Some(Key::special(0x3a + n as u32)),
            KeyCode::Esc => {
                // ESC on its own (not starting a sequence)
                Some(Key::ctrl('['))
            }
            _ => None,
        }
    }

    fn translate_with_meta(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);

        match code {
            KeyCode::Char(ch) => {
                if ctrl {
                    Some(Key(key_flags::META | key_flags::CONTROL | ch.to_ascii_lowercase() as u32))
                } else {
                    Some(Key::meta(ch))
                }
            }
            _ => self.translate_normal(code, modifiers).map(|k| Key(k.0 | key_flags::META)),
        }
    }

    fn translate_with_ctlx(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);

        match code {
            KeyCode::Char(ch) => {
                let mut key_code = key_flags::CTLX | ch.to_ascii_lowercase() as u32;
                if ctrl {
                    key_code |= key_flags::CONTROL;
                }
                if alt {
                    key_code |= key_flags::META;
                }
                Some(Key(key_code))
            }
            _ => self.translate_normal(code, modifiers).map(|k| Key(k.0 | key_flags::CTLX)),
        }
    }

    fn translate_with_ctlx_meta(&self, code: KeyCode, modifiers: KeyModifiers) -> Option<Key> {
        // C-x ESC <key> -> CTLX | META | key
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);

        match code {
            KeyCode::Char(ch) => {
                let mut key_code = key_flags::CTLX | key_flags::META | ch.to_ascii_lowercase() as u32;
                if ctrl {
                    key_code |= key_flags::CONTROL;
                }
                Some(Key(key_code))
            }
            _ => self.translate_normal(code, modifiers).map(|k| Key(k.0 | key_flags::CTLX | key_flags::META)),
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
