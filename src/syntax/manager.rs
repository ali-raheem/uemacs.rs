//! Syntax highlighting manager
//!
//! This module provides the SyntaxManager that coordinates language
//! detection, highlighting, and per-line caching.

use std::collections::HashMap;
use std::path::Path;

use super::language::LanguageDefinition;
use super::rules::LineState;
use super::style::Span;
use super::builtin;

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
        if self.line_states.len() < line_count {
            self.line_states.resize(line_count, LineState::default());
        }
        if self.line_spans.len() < line_count {
            self.line_spans.resize(line_count, None);
        }
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

    /// Set language for a buffer based on filename
    pub fn set_buffer_language(&mut self, buffer_idx: usize, filename: Option<&Path>) {
        let lang_name = filename.and_then(|f| self.detect_language(f)).map(|s| s.to_string());
        let cache = self.get_cache(buffer_idx);
        cache.set_language(lang_name);
    }

    /// Invalidate cache from a line onwards
    pub fn invalidate_from(&mut self, buffer_idx: usize, line: usize) {
        if let Some(cache) = self.caches.get_mut(&buffer_idx) {
            cache.invalidate_from(line);
        }
    }

    /// Highlight a single line, using cache if available
    ///
    /// Returns spans for the line. Empty vec if no highlighting.
    pub fn highlight_line(
        &mut self,
        buffer_idx: usize,
        line_idx: usize,
        text: &str,
        line_count: usize,
    ) -> Vec<Span> {
        if !self.enabled {
            return Vec::new();
        }

        let cache = self.caches.entry(buffer_idx).or_default();
        cache.ensure_size(line_count);

        // Check if we have this language
        let lang_name = match &cache.language {
            Some(name) => name.clone(),
            None => return Vec::new(),
        };

        let lang = match self.languages.get(&lang_name) {
            Some(lang) => lang,
            None => return Vec::new(),
        };

        // Get previous line's state
        let prev_state = if line_idx == 0 {
            LineState::default()
        } else {
            // May need to compute previous lines first
            self.ensure_states_up_to(buffer_idx, line_idx, line_count)
        };

        // Check cache
        let cache = self.caches.get_mut(&buffer_idx).unwrap();
        if let Some(spans) = cache.line_spans.get(line_idx).and_then(|s| s.as_ref()) {
            return spans.to_vec();
        }

        // Compute highlighting
        let lang = self.languages.get(&lang_name).unwrap();
        let result = lang.highlight_line(text, prev_state);

        // Store in cache
        let cache = self.caches.get_mut(&buffer_idx).unwrap();
        if line_idx < cache.line_states.len() {
            cache.line_states[line_idx] = result.end_state;
        }
        if line_idx < cache.line_spans.len() {
            cache.line_spans[line_idx] = Some(result.spans.clone());
        }

        result.spans
    }

    /// Ensure line states are computed up to (but not including) a line
    fn ensure_states_up_to(&mut self, buffer_idx: usize, up_to: usize, line_count: usize) -> LineState {
        // This is a simplified version - in practice we'd need the actual line text
        // For now, just return the stored state or default
        let cache = self.caches.get(&buffer_idx);
        if let Some(cache) = cache {
            if up_to > 0 && up_to <= cache.line_states.len() {
                return cache.line_states[up_to - 1];
            }
        }
        LineState::default()
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
    use std::path::PathBuf;

    #[test]
    fn test_detect_language() {
        let manager = SyntaxManager::new();

        assert_eq!(manager.detect_language(Path::new("main.rs")), Some("Rust"));
        assert_eq!(manager.detect_language(Path::new("test.py")), Some("Python"));
        assert_eq!(manager.detect_language(Path::new("Cargo.toml")), Some("TOML"));
        assert_eq!(manager.detect_language(Path::new("README.md")), Some("Markdown"));
        assert_eq!(manager.detect_language(Path::new("main.c")), Some("C"));
        assert_eq!(manager.detect_language(Path::new("no_extension")), None);
    }

    #[test]
    fn test_highlight_line() {
        let mut manager = SyntaxManager::new();

        // Set up a buffer with Rust language
        manager.set_buffer_language(0, Some(Path::new("test.rs")));

        // Highlight a simple line
        let spans = manager.highlight_line(0, 0, "let x = 42;", 1);

        // Should have some spans
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut manager = SyntaxManager::new();
        manager.set_buffer_language(0, Some(Path::new("test.rs")));

        // Highlight first line
        let spans1 = manager.highlight_line(0, 0, "let x = 1;", 2);
        assert!(!spans1.is_empty());

        // Invalidate and re-highlight
        manager.invalidate_from(0, 0);
        let spans2 = manager.highlight_line(0, 0, "let y = 2;", 2);
        assert!(!spans2.is_empty());
    }

    #[test]
    fn test_no_language() {
        let mut manager = SyntaxManager::new();

        // No language set - should return empty spans
        let spans = manager.highlight_line(0, 0, "some text", 1);
        assert!(spans.is_empty());
    }
}
