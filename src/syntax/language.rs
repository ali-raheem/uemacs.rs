//! Language definitions for syntax highlighting
//!
//! This module provides the LanguageDefinition struct that combines
//! pattern rules, multiline rules, and keywords for a programming language.

use super::rules::{PatternRule, MultilineRule, LineState, HighlightResult};
use super::style::Span;
use super::tokens::TokenType;

/// A complete language definition for syntax highlighting
pub struct LanguageDefinition {
    /// Language name (e.g., "Rust", "Python")
    pub name: String,
    /// File extensions (e.g., ["rs"], ["py", "pyw"])
    pub extensions: Vec<String>,
    /// Single-line pattern rules, sorted by priority (highest first)
    pub patterns: Vec<PatternRule>,
    /// Multi-line rules for comments, strings, etc.
    pub multiline_rules: Vec<MultilineRule>,
}

impl LanguageDefinition {
    /// Create a new empty language definition
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            extensions: Vec::new(),
            patterns: Vec::new(),
            multiline_rules: Vec::new(),
        }
    }

    /// Add a file extension
    pub fn add_extension(&mut self, ext: &str) {
        self.extensions.push(ext.to_string());
    }

    /// Add a pattern rule
    pub fn add_pattern(&mut self, rule: PatternRule) {
        self.patterns.push(rule);
        // Keep sorted by priority (highest first)
        self.patterns.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Add a multiline rule
    pub fn add_multiline(&mut self, rule: MultilineRule) {
        self.multiline_rules.push(rule);
    }

    /// Get multiline rule by state ID
    fn get_multiline_rule(&self, state_id: u8) -> Option<&MultilineRule> {
        self.multiline_rules.iter().find(|r| r.state_id == state_id)
    }

    /// Highlight a single line of text
    ///
    /// Takes the line text and the state from the previous line.
    /// Returns spans and the state for the next line.
    pub fn highlight_line(&self, text: &str, prev_state: LineState) -> HighlightResult {
        let mut spans = Vec::new();
        let mut pos = 0;
        let mut current_state = prev_state;

        while pos < text.len() {
            // If inside a multiline construct, look for its end
            if current_state.is_inside_multiline() {
                if let Some(rule) = self.get_multiline_rule(current_state.multiline_id) {
                    if let Some(end_pos) = rule.find_end(text, pos) {
                        // Found end on this line
                        spans.push(Span::new(pos, end_pos, rule.token_type.default_style()));
                        pos = end_pos;
                        current_state = LineState::default();
                        continue;
                    } else {
                        // No end found - rest of line is inside this construct
                        spans.push(Span::new(pos, text.len(), rule.token_type.default_style()));
                        return HighlightResult {
                            spans,
                            end_state: current_state,
                        };
                    }
                } else {
                    // Invalid state - reset
                    current_state = LineState::default();
                }
            }

            // Try multiline rule starts
            let mut best_multiline: Option<(usize, usize, &MultilineRule)> = None;
            for rule in &self.multiline_rules {
                if let Some((start, end)) = rule.find_start(text, pos) {
                    if start == pos {
                        // Starts at current position
                        if best_multiline.map_or(true, |(_, _, _)| false) {
                            best_multiline = Some((start, end, rule));
                            break; // First match at position wins
                        }
                    }
                }
            }

            if let Some((start, end, rule)) = best_multiline {
                // Check if multiline ends on same line
                if let Some(close_pos) = rule.find_end(text, end) {
                    // Complete construct on this line
                    spans.push(Span::new(start, close_pos, rule.token_type.default_style()));
                    pos = close_pos;
                    continue;
                } else {
                    // Multiline continues to next line
                    spans.push(Span::new(start, text.len(), rule.token_type.default_style()));
                    return HighlightResult {
                        spans,
                        end_state: LineState::inside(rule.state_id),
                    };
                }
            }

            // Try single-line patterns
            let mut best_match: Option<(usize, usize, &PatternRule)> = None;
            for rule in &self.patterns {
                if let Some((start, end)) = rule.find_at(text, pos) {
                    if start == pos {
                        // Pattern matches at current position
                        best_match = Some((start, end, rule));
                        break; // First match (highest priority) wins
                    } else if best_match.is_none() {
                        // Track earliest match for skipping
                        best_match = Some((start, end, rule));
                    }
                }
            }

            if let Some((start, end, rule)) = best_match {
                if start == pos {
                    // Match at current position
                    spans.push(Span::new(start, end, rule.token_type.default_style()));
                    pos = end;
                } else {
                    // Skip to match
                    pos = start;
                }
            } else {
                // No match - skip one byte
                pos += 1;
                // Try to skip to next char boundary for UTF-8
                while pos < text.len() && !text.is_char_boundary(pos) {
                    pos += 1;
                }
            }
        }

        HighlightResult {
            spans,
            end_state: current_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_language() -> LanguageDefinition {
        let mut lang = LanguageDefinition::new("Test");
        lang.add_extension("test");

        // Add line comment
        if let Some(rule) = PatternRule::new("line_comment", r"//.*$", TokenType::Comment, 100) {
            lang.add_pattern(rule);
        }

        // Add numbers
        if let Some(rule) = PatternRule::new("number", r"\b\d+\b", TokenType::Number, 50) {
            lang.add_pattern(rule);
        }

        // Add block comment
        if let Some(rule) = MultilineRule::new("block_comment", r"/\*", r"\*/", TokenType::Comment, 1) {
            lang.add_multiline(rule);
        }

        lang
    }

    #[test]
    fn test_simple_highlighting() {
        let lang = create_test_language();
        let result = lang.highlight_line("let x = 42;", LineState::default());

        assert!(result.end_state.is_normal());
        // Should have span for "42"
        assert!(result.spans.iter().any(|s| s.start == 8 && s.end == 10));
    }

    #[test]
    fn test_line_comment() {
        let lang = create_test_language();
        let result = lang.highlight_line("code // comment", LineState::default());

        assert!(result.end_state.is_normal());
        // Should have span for "// comment"
        assert!(result.spans.iter().any(|s| s.start == 5));
    }

    #[test]
    fn test_multiline_start() {
        let lang = create_test_language();
        let result = lang.highlight_line("code /* comment", LineState::default());

        // Should end in multiline state
        assert!(result.end_state.is_inside_multiline());
        assert_eq!(result.end_state.multiline_id, 1);
    }

    #[test]
    fn test_multiline_continue() {
        let lang = create_test_language();
        let state = LineState::inside(1);
        let result = lang.highlight_line("still in comment", state);

        // Should still be in multiline state
        assert!(result.end_state.is_inside_multiline());
        // Entire line should be styled as comment
        assert!(!result.spans.is_empty());
    }

    #[test]
    fn test_multiline_end() {
        let lang = create_test_language();
        let state = LineState::inside(1);
        let result = lang.highlight_line("end */ code", state);

        // Should return to normal
        assert!(result.end_state.is_normal());
    }
}
