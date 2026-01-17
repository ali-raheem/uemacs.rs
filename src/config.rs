//! Configuration file support
//!
//! Loads settings from ~/.uemacs.conf (or %USERPROFILE%\.uemacs.conf on Windows)
//!
//! Format: simple key=value pairs, one per line
//! Lines starting with # are comments
//!
//! Example:
//! ```text
//! # uEmacs.rs configuration
//! line-numbers = true
//! auto-save = true
//! auto-save-interval = 60
//! tab-width = 4
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Configuration settings
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Whether auto-save is enabled
    pub auto_save: bool,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
    /// Tab width for display
    pub tab_width: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_line_numbers: false,
            auto_save: true,
            auto_save_interval: 30,
            tab_width: 8,
        }
    }
}

impl Config {
    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        #[cfg(windows)]
        {
            std::env::var("USERPROFILE")
                .ok()
                .map(|home| PathBuf::from(home).join(".uemacs.conf"))
        }

        #[cfg(not(windows))]
        {
            std::env::var("HOME")
                .ok()
                .map(|home| PathBuf::from(home).join(".uemacs.conf"))
        }
    }

    /// Load configuration from file
    pub fn load() -> Self {
        let mut config = Config::default();

        if let Some(path) = Self::config_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                let settings = Self::parse(&contents);
                config.apply(&settings);
            }
        }

        config
    }

    /// Parse config file contents into key-value pairs
    fn parse(contents: &str) -> HashMap<String, String> {
        let mut settings = HashMap::new();

        for line in contents.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key = value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                settings.insert(key, value);
            }
        }

        settings
    }

    /// Apply settings from parsed config
    fn apply(&mut self, settings: &HashMap<String, String>) {
        if let Some(value) = settings.get("line-numbers") {
            self.show_line_numbers = parse_bool(value);
        }

        if let Some(value) = settings.get("auto-save") {
            self.auto_save = parse_bool(value);
        }

        if let Some(value) = settings.get("auto-save-interval") {
            if let Ok(n) = value.parse::<u64>() {
                self.auto_save_interval = n.max(10); // Minimum 10 seconds
            }
        }

        if let Some(value) = settings.get("tab-width") {
            if let Ok(n) = value.parse::<usize>() {
                self.tab_width = n.clamp(1, 16); // Between 1 and 16
            }
        }
    }

    /// Save current configuration to file
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::config_path() {
            let contents = format!(
                "# uEmacs.rs configuration\n\
                 # Generated automatically\n\n\
                 line-numbers = {}\n\
                 auto-save = {}\n\
                 auto-save-interval = {}\n\
                 tab-width = {}\n",
                self.show_line_numbers,
                self.auto_save,
                self.auto_save_interval,
                self.tab_width
            );
            fs::write(path, contents)?;
        }
        Ok(())
    }
}

/// Parse a boolean value from string
fn parse_bool(s: &str) -> bool {
    let s = s.to_lowercase();
    matches!(s.as_str(), "true" | "yes" | "on" | "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let contents = r#"
# Comment
line-numbers = true
auto-save = false
auto-save-interval = 60
tab-width = 4
        "#;

        let settings = Config::parse(contents);
        assert_eq!(settings.get("line-numbers"), Some(&"true".to_string()));
        assert_eq!(settings.get("auto-save"), Some(&"false".to_string()));
        assert_eq!(settings.get("auto-save-interval"), Some(&"60".to_string()));
        assert_eq!(settings.get("tab-width"), Some(&"4".to_string()));
    }

    #[test]
    fn test_apply_settings() {
        let mut config = Config::default();
        let mut settings = HashMap::new();
        settings.insert("line-numbers".to_string(), "true".to_string());
        settings.insert("auto-save".to_string(), "false".to_string());
        settings.insert("auto-save-interval".to_string(), "120".to_string());
        settings.insert("tab-width".to_string(), "2".to_string());

        config.apply(&settings);

        assert!(config.show_line_numbers);
        assert!(!config.auto_save);
        assert_eq!(config.auto_save_interval, 120);
        assert_eq!(config.tab_width, 2);
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true"));
        assert!(parse_bool("True"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("on"));
        assert!(parse_bool("1"));

        assert!(!parse_bool("false"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool("off"));
        assert!(!parse_bool("0"));
        assert!(!parse_bool("anything"));
    }
}
