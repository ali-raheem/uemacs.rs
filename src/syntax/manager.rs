//! Syntax highlighting manager
//!
//! This module provides the SyntaxManager that coordinates language
//! detection, highlighting, and per-line caching.

use std::collections::HashMap;
use std::path::Path;

use crate::buffer::Buffer;

use super::builtin;
use super::language::LanguageDefinition;
use super::rules::LineState;
use super::style::Span;

/// Per-buffer highlighting cache
pub struct HighlightCache {
    /// Language for this buffer (None if no highlighting)
    pub language: Option<String>,
    /// Per-line state (multiline construct tracking)
    pub line_states: Vec<LineState>,
    /// Cached spans per line (None = not computed)
    pub line_spans: Vec<Option<Vec<Span>>>,
    /// First line that needs recomputation
    pub invalid_from: usize,
}

impl HighlightCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self {
            language: None,
            line_states: Vec::new(),
            line_spans: Vec::new(),
            invalid_from: 0,
        }
    }

    /// Set the language for this buffer
    pub fn set_language(&mut self, language: Option<String>) {
        self.language = language;
        self.invalidate_all();
    }

    /// Invalidate cache from a specific line onwards
    pub fn invalidate_from(&mut self, line: usize) {
        self.invalid_from = self.invalid_from.min(line);
        // Clear cached spans from this line onwards
        for i in line..self.line_spans.len() {
            self.line_spans[i] = None;
        }
    }

    /// Invalidate entire cache
    pub fn invalidate_all(&mut self) {
        self.invalid_from = 0;
        self.line_states.clear();
        self.line_spans.clear();
    }

    /// Ensure cache vectors are large enough
    pub fn ensure_size(&mut self, line_count: usize) {
        if self.line_states.len() > line_count {
            self.line_states.truncate(line_count);
        }
        if self.line_spans.len() > line_count {
            self.line_spans.truncate(line_count);
        }
        if self.line_states.len() < line_count {
            self.line_states.resize(line_count, LineState::default());
        }
        if self.line_spans.len() < line_count {
            self.line_spans.resize(line_count, None);
        }
        self.invalid_from = self.invalid_from.min(line_count);
    }
}

impl Default for HighlightCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Main syntax highlighting manager
pub struct SyntaxManager {
    /// Loaded language definitions
    languages: HashMap<String, LanguageDefinition>,
    /// Extension to language name mapping
    extension_map: HashMap<String, String>,
    /// Per-buffer caches (buffer index -> cache)
    caches: HashMap<usize, HighlightCache>,
    /// Whether syntax highlighting is enabled
    pub enabled: bool,
}

impl SyntaxManager {
    /// Create a new syntax manager with built-in languages
    pub fn new() -> Self {
        let mut manager = Self {
            languages: HashMap::new(),
            extension_map: HashMap::new(),
            caches: HashMap::new(),
            enabled: true,
        };

        // Load built-in languages
        for lang in builtin::all_languages() {
            manager.add_language(lang);
        }

        manager
    }

    /// Add a language definition
    pub fn add_language(&mut self, lang: LanguageDefinition) {
        let name = lang.name.clone();
        for ext in &lang.extensions {
            self.extension_map.insert(ext.to_lowercase(), name.clone());
        }
        self.languages.insert(name, lang);
    }

    /// Detect language from filename
    pub fn detect_language(&self, filename: &Path) -> Option<&str> {
        let ext = filename.extension()?.to_str()?.to_lowercase();
        self.extension_map.get(&ext).map(|s| s.as_str())
    }

    /// Get a language definition by name
    pub fn get_language(&self, name: &str) -> Option<&LanguageDefinition> {
        self.languages.get(name)
    }

    /// Get or create a cache for a buffer
    pub fn get_cache(&mut self, buffer_idx: usize) -> &mut HighlightCache {
        self.caches.entry(buffer_idx).or_default()
    }

    /// Remove cache for a buffer (when buffer is closed)
    pub fn remove_cache(&mut self, buffer_idx: usize) {
        self.caches.remove(&buffer_idx);
    }

    /// Remap cache indices after removing a buffer.
    ///
    /// All caches above `removed_idx` shift down by one.
    pub fn remap_after_buffer_removal(&mut self, removed_idx: usize) {
        let mut remapped = HashMap::with_capacity(self.caches.len());
        for (idx, cache) in std::mem::take(&mut self.caches) {
            if idx == removed_idx {
                continue;
            }
            let new_idx = if idx > removed_idx { idx - 1 } else { idx };
            remapped.insert(new_idx, cache);
        }
        self.caches = remapped;
    }

    /// Set language for a buffer based on filename
    pub fn set_buffer_language(&mut self, buffer_idx: usize, filename: Option<&Path>) {
        let lang_name = filename
            .and_then(|f| self.detect_language(f))
            .map(|s| s.to_string());
        let cache = self.get_cache(buffer_idx);
        cache.set_language(lang_name);
    }

    /// Invalidate cache from a line onwards
    pub fn invalidate_from(&mut self, buffer_idx: usize, line: usize) {
        if let Some(cache) = self.caches.get_mut(&buffer_idx) {
            cache.invalidate_from(line);
        }
    }

    /// Highlight a single line, recomputing cache state from the first invalid line.
    ///
    /// Returns spans for the line. Empty vec if no highlighting.
    pub fn highlight_line(
        &mut self,
        buffer_idx: usize,
        line_idx: usize,
        buffer: &Buffer,
    ) -> Vec<Span> {
        if !self.enabled {
            return Vec::new();
        }

        let line_count = buffer.line_count();
        if line_idx >= line_count {
            return Vec::new();
        }

        let (languages, caches) = (&self.languages, &mut self.caches);
        let cache = caches.entry(buffer_idx).or_default();
        cache.ensure_size(line_count);

        let lang_name = match &cache.language {
            Some(name) => name.clone(),
            None => return Vec::new(),
        };

        let lang = match languages.get(&lang_name) {
            Some(lang) => lang,
            None => return Vec::new(),
        };

        if cache.invalid_from > line_idx {
            if let Some(spans) = cache.line_spans.get(line_idx).and_then(|s| s.as_ref()) {
                return spans.clone();
            }
        }

        let start_line = cache.invalid_from.min(line_idx);
        let mut prev_state = if start_line == 0 {
            LineState::default()
        } else {
            cache.line_states[start_line - 1]
        };

        for current_line in start_line..=line_idx {
            let text = buffer
                .line(current_line)
                .map(|line| line.text())
                .unwrap_or("");
            let result = lang.highlight_line(text, prev_state);
            cache.line_states[current_line] = result.end_state;
            cache.line_spans[current_line] = Some(result.spans);
            prev_state = result.end_state;
        }

        cache.invalid_from = (line_idx + 1).min(line_count);
        cache.line_spans[line_idx].clone().unwrap_or_default()
    }

    /// List available languages
    pub fn list_languages(&self) -> Vec<&str> {
        let mut names: Vec<_> = self.languages.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Toggle syntax highlighting on/off
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

impl Default for SyntaxManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        let manager = SyntaxManager::new();

        assert_eq!(manager.detect_language(Path::new("main.rs")), Some("Rust"));
        assert_eq!(
            manager.detect_language(Path::new("test.py")),
            Some("Python")
        );
        assert_eq!(
            manager.detect_language(Path::new("Cargo.toml")),
            Some("TOML")
        );
        assert_eq!(
            manager.detect_language(Path::new("README.md")),
            Some("Markdown")
        );
        assert_eq!(manager.detect_language(Path::new("main.c")), Some("C"));
        assert_eq!(manager.detect_language(Path::new("no_extension")), None);
    }

    #[test]
    fn test_highlight_line() {
        let mut manager = SyntaxManager::new();

        // Set up a buffer with Rust language
        manager.set_buffer_language(0, Some(Path::new("test.rs")));

        let buffer = Buffer::from_content("test.rs", "let x = 42;");

        // Highlight a simple line
        let spans = manager.highlight_line(0, 0, &buffer);

        // Should have some spans
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut manager = SyntaxManager::new();
        manager.set_buffer_language(0, Some(Path::new("test.rs")));
        let buffer = Buffer::from_content("test.rs", "let x = 1;\nlet y = 2;");

        // Highlight first line
        let spans1 = manager.highlight_line(0, 0, &buffer);
        assert!(!spans1.is_empty());

        // Invalidate and re-highlight
        manager.invalidate_from(0, 0);
        let spans2 = manager.highlight_line(0, 0, &buffer);
        assert!(!spans2.is_empty());
    }

    #[test]
    fn test_multiline_state_propagates() {
        let mut manager = SyntaxManager::new();
        manager.set_buffer_language(0, Some(Path::new("test.rs")));
        let buffer =
            Buffer::from_content("test.rs", "/* block comment\nstill comment */ let x = 1;");

        let line0 = manager.highlight_line(0, 0, &buffer);
        assert!(!line0.is_empty());

        let line1 = manager.highlight_line(0, 1, &buffer);
        assert!(!line1.is_empty());
        assert_eq!(line1[0].start, 0);
    }

    #[test]
    fn test_unterminated_quoted_string_does_not_poison_following_lines() {
        let mut manager = SyntaxManager::new();
        manager.set_buffer_language(0, Some(Path::new("test.rs")));
        let buffer = Buffer::from_content(
            "test.rs",
            "let s = \"unterminated\nlet x = 42;\nlet y = 24;",
        );

        let _ = manager.highlight_line(0, 0, &buffer);
        let line1 = manager.highlight_line(0, 1, &buffer);
        let line2 = manager.highlight_line(0, 2, &buffer);

        assert!(
            !line1.is_empty(),
            "line after unterminated quote should still tokenize normally"
        );
        assert!(
            !line2.is_empty(),
            "subsequent lines should not remain in string state"
        );
    }

    #[test]
    fn test_no_language() {
        let mut manager = SyntaxManager::new();
        let buffer = Buffer::from_content("plain.txt", "some text");

        // No language set - should return empty spans
        let spans = manager.highlight_line(0, 0, &buffer);
        assert!(spans.is_empty());
    }

    #[test]
    fn test_cache_remap_after_buffer_removal() {
        let mut manager = SyntaxManager::new();
        manager.set_buffer_language(0, Some(Path::new("a.rs")));
        manager.set_buffer_language(1, Some(Path::new("b.py")));
        manager.set_buffer_language(2, Some(Path::new("c.md")));

        manager.remap_after_buffer_removal(1);

        assert!(manager.caches.contains_key(&0));
        assert!(manager.caches.contains_key(&1));
        assert!(!manager.caches.contains_key(&2));
        assert_eq!(
            manager.caches.get(&0).and_then(|c| c.language.as_deref()),
            Some("Rust")
        );
        assert_eq!(
            manager.caches.get(&1).and_then(|c| c.language.as_deref()),
            Some("Markdown")
        );
    }
}
