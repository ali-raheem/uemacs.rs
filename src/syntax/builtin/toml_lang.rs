//! TOML language definition

use crate::syntax::language::LanguageDefinition;
use crate::syntax::rules::{PatternRule, MultilineRule};
use crate::syntax::tokens::TokenType;

/// Create TOML language definition
pub fn toml_language() -> LanguageDefinition {
    let mut lang = LanguageDefinition::new("TOML");
    lang.add_extension("toml");

    // Multiline rules
    // Multi-line basic strings
    if let Some(rule) = MultilineRule::new("multiline_basic", r#"""""#, r#"""""#, TokenType::String, 1) {
        lang.add_multiline(rule);
    }
    // Multi-line literal strings
    if let Some(rule) = MultilineRule::new("multiline_literal", r"'''", r"'''", TokenType::String, 2) {
        lang.add_multiline(rule);
    }

    // Basic strings (with escape support)
    if let Some(rule) = MultilineRule::with_escape("basic_string", r#"""#, r#"""#, TokenType::String, 3, '\\') {
        lang.add_multiline(rule);
    }
    // Literal strings (no escapes)
    if let Some(rule) = MultilineRule::new("literal_string", r"'", r"'", TokenType::String, 4) {
        lang.add_multiline(rule);
    }

    // Single-line patterns

    // Comments
    if let Some(rule) = PatternRule::new("comment", r"#.*$", TokenType::Comment, 100) {
        lang.add_pattern(rule);
    }

    // Table headers
    if let Some(rule) = PatternRule::new("table", r"^\s*\[\[?[^\]]+\]\]?", TokenType::Keyword, 95) {
        lang.add_pattern(rule);
    }

    // Keys (before = sign)
    if let Some(rule) = PatternRule::new("key", r"^[\w\-\.]+(?=\s*=)", TokenType::Type, 90) {
        lang.add_pattern(rule);
    }

    // Booleans
    if let Some(rule) = PatternRule::new("boolean", r"\b(true|false)\b", TokenType::Constant, 80) {
        lang.add_pattern(rule);
    }

    // Dates/times
    if let Some(rule) = PatternRule::new("datetime", r"\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?", TokenType::Number, 75) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("time", r"\d{2}:\d{2}:\d{2}(?:\.\d+)?", TokenType::Number, 74) {
        lang.add_pattern(rule);
    }

    // Numbers
    // Hex
    if let Some(rule) = PatternRule::new("hex", r"\b0x[0-9a-fA-F_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Binary
    if let Some(rule) = PatternRule::new("binary", r"\b0b[01_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Octal
    if let Some(rule) = PatternRule::new("octal", r"\b0o[0-7_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Float (including special values)
    if let Some(rule) = PatternRule::new("float", r"[+-]?(?:\d[\d_]*\.\d[\d_]*(?:[eE][+-]?\d+)?|inf|nan)\b", TokenType::Number, 64) {
        lang.add_pattern(rule);
    }
    // Integer
    if let Some(rule) = PatternRule::new("integer", r"[+-]?\d[\d_]*\b", TokenType::Number, 63) {
        lang.add_pattern(rule);
    }

    lang
}
