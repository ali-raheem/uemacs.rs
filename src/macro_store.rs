//! Persistent macro storage
//!
//! Saves and loads keyboard macros to/from a file.
//! File format is simple text, one macro per section:
//!
//! ```text
//! [macro.0]
//! C-a
//! C-k
//! C-n
//! C-y
//!
//! [macro.3]
//! M-f
//! M-d
//! ```

use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use crate::input::Key;

/// Get the path to the macros file
pub fn macros_file_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .ok()
            .map(|p| PathBuf::from(p).join(".uemacs-macros"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".uemacs-macros"))
    }
}

/// Load macros from the macros file
/// Returns an array of 10 macro slots (indices 0-9)
pub fn load_macros() -> [Vec<Key>; 10] {
    let mut slots: [Vec<Key>; 10] = Default::default();

    let path = match macros_file_path() {
        Some(p) => p,
        None => return slots,
    };

    if !path.exists() {
        return slots;
    }

    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return slots,
    };

    let reader = io::BufReader::new(file);
    let mut current_slot: Option<usize> = None;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        // Check for section header [macro.N]
        if trimmed.starts_with("[macro.") && trimmed.ends_with(']') {
            let slot_str = &trimmed[7..trimmed.len() - 1];
            current_slot = slot_str.parse::<usize>().ok().filter(|&n| n <= 9);
            continue;
        }

        // Parse key and add to current slot
        if let Some(slot) = current_slot {
            if let Some(key) = Key::from_display_name(trimmed) {
                slots[slot].push(key);
            }
        }
    }

    slots
}

/// Save macros to the macros file
/// Only saves non-empty slots
pub fn save_macros(slots: &[Vec<Key>; 10]) -> io::Result<()> {
    let path = match macros_file_path() {
        Some(p) => p,
        None => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine macros file path",
            ))
        }
    };

    let mut file = fs::File::create(&path)?;

    writeln!(file, "# uEmacs.rs keyboard macros")?;
    writeln!(file, "# Each [macro.N] section defines macro slot N (0-9)")?;
    writeln!(file, "# Each line is a key in display format (C-f, M-x, etc.)")?;
    writeln!(file)?;

    let mut wrote_any = false;

    for (i, keys) in slots.iter().enumerate() {
        if keys.is_empty() {
            continue;
        }

        if wrote_any {
            writeln!(file)?;
        }

        writeln!(file, "[macro.{}]", i)?;
        for key in keys {
            writeln!(file, "{}", key.display_name())?;
        }

        wrote_any = true;
    }

    Ok(())
}

/// Count how many macros are stored (non-empty slots)
pub fn count_stored_macros(slots: &[Vec<Key>; 10]) -> usize {
    slots.iter().filter(|s| !s.is_empty()).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        // Create test macros
        let mut slots: [Vec<Key>; 10] = Default::default();
        slots[0] = vec![Key::ctrl('a'), Key::ctrl('k'), Key::ctrl('y')];
        slots[3] = vec![Key::meta('f'), Key::meta('d')];
        slots[9] = vec![Key::ctlx_ctrl('s')];

        // Serialize to string
        let mut output = Vec::new();
        {
            let mut cursor = io::Cursor::new(&mut output);

            for (i, keys) in slots.iter().enumerate() {
                if keys.is_empty() {
                    continue;
                }
                writeln!(cursor, "[macro.{}]", i).unwrap();
                for key in keys {
                    writeln!(cursor, "{}", key.display_name()).unwrap();
                }
                writeln!(cursor).unwrap();
            }
        }

        // Parse back
        let content = String::from_utf8(output).unwrap();
        let mut parsed_slots: [Vec<Key>; 10] = Default::default();
        let mut current_slot: Option<usize> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("[macro.") && trimmed.ends_with(']') {
                let slot_str = &trimmed[7..trimmed.len() - 1];
                current_slot = slot_str.parse::<usize>().ok().filter(|&n| n <= 9);
                continue;
            }
            if let Some(slot) = current_slot {
                if let Some(key) = Key::from_display_name(trimmed) {
                    parsed_slots[slot].push(key);
                }
            }
        }

        // Verify
        assert_eq!(slots[0], parsed_slots[0]);
        assert_eq!(slots[3], parsed_slots[3]);
        assert_eq!(slots[9], parsed_slots[9]);
    }
}
