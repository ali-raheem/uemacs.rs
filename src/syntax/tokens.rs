//! Token types for syntax highlighting
//!
//! This module defines the semantic token types that can be
//! recognized in source code and their default visual styles.

use super::style::{Color, Style};

/// Semantic token types for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Source code comments (// or /* */)
    Comment,
    /// String literals ("..." or '...')
    String,
    /// Character literals
    Char,
    /// Numeric literals (integers, floats)
    Number,
    /// Language keywords (if, else, fn, let, etc.)
    Keyword,
    /// Type names (String, i32, etc.)
    Type,
    /// Function names
    Function,
    /// Operators (+, -, *, /, etc.)
    Operator,
    /// Punctuation (, ; : etc.)
    Punctuation,
    /// Preprocessor directives (#include, #define)
    Preprocessor,
    /// Macros (println!, vec!)
    Macro,
    /// Constants and enum variants
    Constant,
    /// Special tokens (escape sequences, etc.)
    Special,
    /// Attributes (#[derive], @decorator)
    Attribute,
    /// Lifetime annotations ('a)
    Lifetime,
    /// Module/namespace names
    Module,
    /// Labels and gotos
    Label,
    /// Default/plain text (no special highlighting)
    Default,
}

impl TokenType {
    /// Get the default style for this token type
    pub fn default_style(&self) -> Style {
        match self {
            TokenType::Comment => Style::fg(Color::BrightBlack).with_italic(),
            TokenType::String => Style::fg(Color::Green),
            TokenType::Char => Style::fg(Color::Green),
            TokenType::Number => Style::fg(Color::Cyan),
            TokenType::Keyword => Style::fg(Color::Magenta).with_bold(),
            TokenType::Type => Style::fg(Color::Yellow),
            TokenType::Function => Style::fg(Color::Blue),
            TokenType::Operator => Style::fg(Color::BrightWhite),
            TokenType::Punctuation => Style::default(),
            TokenType::Preprocessor => Style::fg(Color::BrightMagenta),
            TokenType::Macro => Style::fg(Color::BrightCyan),
            TokenType::Constant => Style::fg(Color::BrightRed),
            TokenType::Special => Style::fg(Color::BrightYellow),
            TokenType::Attribute => Style::fg(Color::BrightBlue),
            TokenType::Lifetime => Style::fg(Color::BrightMagenta),
            TokenType::Module => Style::fg(Color::BrightBlue),
            TokenType::Label => Style::fg(Color::Yellow).with_underline(),
            TokenType::Default => Style::default(),
        }
    }

    /// Get a human-readable name for this token type
    pub fn name(&self) -> &'static str {
        match self {
            TokenType::Comment => "Comment",
            TokenType::String => "String",
            TokenType::Char => "Char",
            TokenType::Number => "Number",
            TokenType::Keyword => "Keyword",
            TokenType::Type => "Type",
            TokenType::Function => "Function",
            TokenType::Operator => "Operator",
            TokenType::Punctuation => "Punctuation",
            TokenType::Preprocessor => "Preprocessor",
            TokenType::Macro => "Macro",
            TokenType::Constant => "Constant",
            TokenType::Special => "Special",
            TokenType::Attribute => "Attribute",
            TokenType::Lifetime => "Lifetime",
            TokenType::Module => "Module",
            TokenType::Label => "Label",
            TokenType::Default => "Default",
        }
    }

    /// Parse a token type from a string name (for TOML loading)
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Comment" => Some(TokenType::Comment),
            "String" => Some(TokenType::String),
            "Char" => Some(TokenType::Char),
            "Number" => Some(TokenType::Number),
            "Keyword" => Some(TokenType::Keyword),
            "Type" => Some(TokenType::Type),
            "Function" => Some(TokenType::Function),
            "Operator" => Some(TokenType::Operator),
            "Punctuation" => Some(TokenType::Punctuation),
            "Preprocessor" => Some(TokenType::Preprocessor),
            "Macro" => Some(TokenType::Macro),
            "Constant" => Some(TokenType::Constant),
            "Special" => Some(TokenType::Special),
            "Attribute" => Some(TokenType::Attribute),
            "Lifetime" => Some(TokenType::Lifetime),
            "Module" => Some(TokenType::Module),
            "Label" => Some(TokenType::Label),
            "Default" => Some(TokenType::Default),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_styles_not_empty() {
        // Most token types should have non-default styling
        assert!(!TokenType::Comment.default_style().is_default());
        assert!(!TokenType::String.default_style().is_default());
        assert!(!TokenType::Keyword.default_style().is_default());
        // Punctuation and Default should be plain
        assert!(TokenType::Punctuation.default_style().is_default());
        assert!(TokenType::Default.default_style().is_default());
    }

    #[test]
    fn test_from_name_roundtrip() {
        let types = [
            TokenType::Comment,
            TokenType::String,
            TokenType::Keyword,
            TokenType::Default,
        ];
        for token_type in types {
            let name = token_type.name();
            let parsed = TokenType::from_name(name);
            assert_eq!(parsed, Some(token_type));
        }
    }

    #[test]
    fn test_from_name_invalid() {
        assert_eq!(TokenType::from_name("InvalidType"), None);
        assert_eq!(TokenType::from_name(""), None);
    }
}
