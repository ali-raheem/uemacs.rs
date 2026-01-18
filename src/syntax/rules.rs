//! Pattern rules for syntax highlighting
//!
//! This module defines the rule types used to match and tokenize
//! source code for syntax highlighting.

use regex::Regex;
use super::tokens::TokenType;

/// A single-line pattern rule
///
/// Matches a regex pattern and assigns a token type to the match.
/// Rules are tried in priority order (highest first).
pub struct PatternRule {
    /// Name for debugging
    pub name: String,
    /// Compiled regex pattern
    pub pattern: Regex,
    /// Token type to assign to matches
    pub token_type: TokenType,
    /// Priority (higher = matched first)
    pub priority: i32,
}

impl PatternRule {
    /// Create a new pattern rule
    pub fn new(name: &str, pattern: &str, token_type: TokenType, priority: i32) -> Option<Self> {
        Regex::new(pattern).ok().map(|regex| Self {
            name: name.to_string(),
            pattern: regex,
            token_type,
            priority,
        })
    }

    /// Find the first match in text starting at position
    pub fn find_at(&self, text: &str, start: usize) -> Option<(usize, usize)> {
        if start >= text.len() {
            return None;
        }
        self.pattern.find(&text[start..]).map(|m| (start + m.start(), start + m.end()))
    }
}

/// A multi-line construct rule (block comments, strings)
///
/// These rules track state across lines for constructs that
/// can span multiple lines.
pub struct MultilineRule {
    /// Name for debugging
    pub name: String,
    /// Pattern that starts the construct
    pub start: Regex,
    /// Pattern that ends the construct
    pub end: Regex,
    /// Token type for this construct
    pub token_type: TokenType,
    /// Whether the end pattern can be escaped
    pub escapable: bool,
    /// Escape character (usually backslash)
    pub escape_char: Option<char>,
    /// Unique ID for this multiline state (1-255, 0 = normal)
    pub state_id: u8,
}

impl MultilineRule {
    /// Create a new multiline rule
    pub fn new(
        name: &str,
        start_pattern: &str,
        end_pattern: &str,
        token_type: TokenType,
        state_id: u8,
    ) -> Option<Self> {
        let start = Regex::new(start_pattern).ok()?;
        let end = Regex::new(end_pattern).ok()?;
        Some(Self {
            name: name.to_string(),
            start,
            end,
            token_type,
            escapable: false,
            escape_char: None,
            state_id,
        })
    }

    /// Create a multiline rule with escape support
    pub fn with_escape(
        name: &str,
        start_pattern: &str,
        end_pattern: &str,
        token_type: TokenType,
        state_id: u8,
        escape_char: char,
    ) -> Option<Self> {
        let mut rule = Self::new(name, start_pattern, end_pattern, token_type, state_id)?;
        rule.escapable = true;
        rule.escape_char = Some(escape_char);
        Some(rule)
    }

    /// Find start of this construct in text
    pub fn find_start(&self, text: &str, start: usize) -> Option<(usize, usize)> {
        if start >= text.len() {
            return None;
        }
        self.start.find(&text[start..]).map(|m| (start + m.start(), start + m.end()))
    }

    /// Find end of this construct in text, respecting escapes
    pub fn find_end(&self, text: &str, start: usize) -> Option<usize> {
        if start >= text.len() {
            return None;
        }

        let search_text = &text[start..];

        if self.escapable {
            // Search for end pattern while respecting escapes
            let mut pos = 0;
            while pos < search_text.len() {
                if let Some(m) = self.end.find(&search_text[pos..]) {
                    let match_start = pos + m.start();
                    // Check if preceded by escape char
                    if match_start > 0 {
                        let preceding = &search_text[..match_start];
                        let escape_count = preceding.chars().rev()
                            .take_while(|&c| Some(c) == self.escape_char)
                            .count();
                        if escape_count % 2 == 1 {
                            // Odd number of escapes = escaped
                            pos = pos + m.end();
                            continue;
                        }
                    }
                    return Some(start + pos + m.end());
                }
                break;
            }
            None
        } else {
            self.end.find(search_text).map(|m| start + m.end())
        }
    }
}

/// Line state for tracking multi-line constructs
///
/// This is stored per-line to track whether we're inside a
/// multi-line comment, string, etc.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineState {
    /// 0 = normal, non-zero = inside multiline rule with this ID
    pub multiline_id: u8,
}

impl LineState {
    /// Create state for being inside a multiline construct
    pub fn inside(state_id: u8) -> Self {
        Self { multiline_id: state_id }
    }

    /// Check if we're inside a multiline construct
    pub fn is_inside_multiline(&self) -> bool {
        self.multiline_id != 0
    }

    /// Check if we're in normal (no multiline) state
    pub fn is_normal(&self) -> bool {
        self.multiline_id == 0
    }
}

/// Result of highlighting a single line
#[derive(Debug)]
pub struct HighlightResult {
    /// Spans of styled text in this line
    pub spans: Vec<super::style::Span>,
    /// State at end of line (for next line)
    pub end_state: LineState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_rule() {
        let rule = PatternRule::new("number", r"\d+", TokenType::Number, 50).unwrap();
        assert_eq!(rule.find_at("abc 123 def", 0), Some((4, 7)));
        assert_eq!(rule.find_at("abc 123 def", 5), Some((5, 7)));
        assert_eq!(rule.find_at("no numbers", 0), None);
    }

    #[test]
    fn test_multiline_rule() {
        let rule = MultilineRule::new(
            "block_comment",
            r"/\*",
            r"\*/",
            TokenType::Comment,
            1,
        ).unwrap();

        assert_eq!(rule.find_start("/* comment */", 0), Some((0, 2)));
        assert_eq!(rule.find_end("/* comment */", 2), Some(13));
    }

    #[test]
    fn test_multiline_with_escape() {
        let rule = MultilineRule::with_escape(
            "string",
            r#"""#,
            r#"""#,
            TokenType::String,
            2,
            '\\',
        ).unwrap();

        // Regular end
        assert_eq!(rule.find_end(r#"hello""#, 0), Some(6));
        // Escaped quote
        assert_eq!(rule.find_end(r#"hello\"world""#, 0), Some(13));
    }

    #[test]
    fn test_line_state() {
        let normal = LineState::default();
        assert!(normal.is_normal());
        assert!(!normal.is_inside_multiline());

        let inside = LineState::inside(1);
        assert!(!inside.is_normal());
        assert!(inside.is_inside_multiline());
    }
}
