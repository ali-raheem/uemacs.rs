//! Markdown language definition

use crate::syntax::language::LanguageDefinition;
use crate::syntax::rules::{PatternRule, MultilineRule};
use crate::syntax::tokens::TokenType;

/// Create Markdown language definition
pub fn markdown_language() -> LanguageDefinition {
    let mut lang = LanguageDefinition::new("Markdown");
    lang.add_extension("md");
    lang.add_extension("markdown");
    lang.add_extension("mkd");

    // Multiline rules
    // Fenced code blocks
    if let Some(rule) = MultilineRule::new("code_block", r"^```", r"^```", TokenType::String, 1) {
        lang.add_multiline(rule);
    }

    // Single-line patterns

    // Headers
    if let Some(rule) = PatternRule::new("header", r"^#{1,6}\s+.*$", TokenType::Keyword, 100) {
        lang.add_pattern(rule);
    }

    // Bold (** or __)
    if let Some(rule) = PatternRule::new("bold", r"\*\*[^*]+\*\*", TokenType::Type, 90) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("bold2", r"__[^_]+__", TokenType::Type, 90) {
        lang.add_pattern(rule);
    }

    // Italic (* or _)
    if let Some(rule) = PatternRule::new("italic", r"\*[^*]+\*", TokenType::Special, 85) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("italic2", r"_[^_]+_", TokenType::Special, 85) {
        lang.add_pattern(rule);
    }

    // Inline code
    if let Some(rule) = PatternRule::new("inline_code", r"`[^`]+`", TokenType::String, 88) {
        lang.add_pattern(rule);
    }

    // Links [text](url)
    if let Some(rule) = PatternRule::new("link", r"\[[^\]]+\]\([^)]+\)", TokenType::Function, 80) {
        lang.add_pattern(rule);
    }

    // Reference links [text][ref]
    if let Some(rule) = PatternRule::new("ref_link", r"\[[^\]]+\]\[[^\]]*\]", TokenType::Function, 79) {
        lang.add_pattern(rule);
    }

    // Images ![alt](url)
    if let Some(rule) = PatternRule::new("image", r"!\[[^\]]*\]\([^)]+\)", TokenType::Macro, 80) {
        lang.add_pattern(rule);
    }

    // Blockquotes
    if let Some(rule) = PatternRule::new("blockquote", r"^>\s+.*$", TokenType::Comment, 75) {
        lang.add_pattern(rule);
    }

    // Horizontal rules
    if let Some(rule) = PatternRule::new("hr", r"^(?:---+|\*\*\*+|___+)\s*$", TokenType::Operator, 70) {
        lang.add_pattern(rule);
    }

    // List items
    if let Some(rule) = PatternRule::new("list", r"^[\s]*[-*+]\s", TokenType::Operator, 65) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("numbered_list", r"^[\s]*\d+\.\s", TokenType::Operator, 65) {
        lang.add_pattern(rule);
    }

    // Strikethrough
    if let Some(rule) = PatternRule::new("strikethrough", r"~~[^~]+~~", TokenType::Comment, 60) {
        lang.add_pattern(rule);
    }

    lang
}
