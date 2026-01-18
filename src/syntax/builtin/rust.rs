//! Rust language definition

use crate::syntax::language::LanguageDefinition;
use crate::syntax::rules::{PatternRule, MultilineRule};
use crate::syntax::tokens::TokenType;

/// Create Rust language definition
pub fn rust_language() -> LanguageDefinition {
    let mut lang = LanguageDefinition::new("Rust");
    lang.add_extension("rs");

    // Multiline rules (state IDs 1-10)
    // Block comments (can nest, but we'll handle simple case)
    if let Some(rule) = MultilineRule::new("block_comment", r"/\*", r"\*/", TokenType::Comment, 1) {
        lang.add_multiline(rule);
    }

    // Raw strings r#"..."# (simplified - doesn't handle all cases)
    if let Some(rule) = MultilineRule::new("raw_string", r##"r#""##, r##""#"##, TokenType::String, 2) {
        lang.add_multiline(rule);
    }

    // Regular strings (with escape support)
    if let Some(rule) = MultilineRule::with_escape("string", "\"", "\"", TokenType::String, 3, '\\') {
        lang.add_multiline(rule);
    }

    // Single-line patterns (priority 0-100, higher = first)

    // Line comments (highest priority)
    if let Some(rule) = PatternRule::new("line_comment", r"//.*$", TokenType::Comment, 100) {
        lang.add_pattern(rule);
    }

    // Doc comments (before line comments for priority, but actually same pattern)
    if let Some(rule) = PatternRule::new("doc_comment", r"///.*$", TokenType::Comment, 101) {
        lang.add_pattern(rule);
    }

    // Attributes
    if let Some(rule) = PatternRule::new("attribute", r"#!\?\[[\w:(),\s]*\]", TokenType::Attribute, 95) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("attribute_simple", r"#\[[\w]+\]", TokenType::Attribute, 94) {
        lang.add_pattern(rule);
    }

    // Character literals
    if let Some(rule) = PatternRule::new("char", r"'(?:[^'\\]|\\.)'", TokenType::Char, 90) {
        lang.add_pattern(rule);
    }

    // Lifetimes (before char to avoid conflict)
    if let Some(rule) = PatternRule::new("lifetime", r"'\w+", TokenType::Lifetime, 89) {
        lang.add_pattern(rule);
    }

    // Macros (ending with !)
    if let Some(rule) = PatternRule::new("macro", r"\b\w+!", TokenType::Macro, 85) {
        lang.add_pattern(rule);
    }

    // Keywords
    let keywords = r"\b(as|async|await|break|const|continue|crate|dyn|else|enum|extern|false|fn|for|if|impl|in|let|loop|match|mod|move|mut|pub|ref|return|self|Self|static|struct|super|trait|true|type|union|unsafe|use|where|while)\b";
    if let Some(rule) = PatternRule::new("keyword", keywords, TokenType::Keyword, 80) {
        lang.add_pattern(rule);
    }

    // Built-in types
    let types = r"\b(bool|char|str|u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize|f32|f64|String|Vec|Box|Rc|Arc|Option|Result|Some|None|Ok|Err)\b";
    if let Some(rule) = PatternRule::new("type", types, TokenType::Type, 75) {
        lang.add_pattern(rule);
    }

    // Type names (capitalized identifiers)
    if let Some(rule) = PatternRule::new("type_name", r"\b[A-Z][a-zA-Z0-9_]*\b", TokenType::Type, 60) {
        lang.add_pattern(rule);
    }

    // Function definitions
    if let Some(rule) = PatternRule::new("fn_def", r"\bfn\s+(\w+)", TokenType::Function, 70) {
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
    // Float
    if let Some(rule) = PatternRule::new("float", r"\b\d[\d_]*\.\d[\d_]*(?:[eE][+-]?\d+)?\b", TokenType::Number, 64) {
        lang.add_pattern(rule);
    }
    // Integer
    if let Some(rule) = PatternRule::new("integer", r"\b\d[\d_]*(?:u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)?\b", TokenType::Number, 63) {
        lang.add_pattern(rule);
    }

    // Operators
    if let Some(rule) = PatternRule::new("operator", r"[+\-*/%&|^!<>=@]+", TokenType::Operator, 40) {
        lang.add_pattern(rule);
    }

    lang
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::rules::LineState;

    #[test]
    fn test_rust_keywords() {
        let lang = rust_language();
        let result = lang.highlight_line("let mut x = 5;", LineState::default());

        // Should highlight "let" and "mut" as keywords
        assert!(result.spans.len() >= 2);
    }

    #[test]
    fn test_rust_string() {
        let lang = rust_language();
        let result = lang.highlight_line(r#"let s = "hello";"#, LineState::default());

        // Should have spans for the string
        assert!(!result.spans.is_empty());
    }

    #[test]
    fn test_rust_comment() {
        let lang = rust_language();
        let result = lang.highlight_line("// this is a comment", LineState::default());

        // Entire line should be comment
        assert_eq!(result.spans.len(), 1);
        assert_eq!(result.spans[0].start, 0);
    }

    #[test]
    fn test_rust_macro() {
        let lang = rust_language();
        let result = lang.highlight_line("println!(\"test\");", LineState::default());

        // Should highlight println! as macro
        assert!(result.spans.iter().any(|s| s.start == 0));
    }
}
