//! Python language definition

use crate::syntax::language::LanguageDefinition;
use crate::syntax::rules::{PatternRule, MultilineRule};
use crate::syntax::tokens::TokenType;

/// Create Python language definition
pub fn python_language() -> LanguageDefinition {
    let mut lang = LanguageDefinition::new("Python");
    lang.add_extension("py");
    lang.add_extension("pyw");
    lang.add_extension("pyi");

    // Multiline rules
    // Triple-quoted strings (docstrings)
    if let Some(rule) = MultilineRule::new("triple_double", r#"""""#, r#"""""#, TokenType::String, 1) {
        lang.add_multiline(rule);
    }
    if let Some(rule) = MultilineRule::new("triple_single", r"'''", r"'''", TokenType::String, 2) {
        lang.add_multiline(rule);
    }

    // Regular strings (with escape support)
    if let Some(rule) = MultilineRule::with_escape("double_string", r#"""#, r#"""#, TokenType::String, 3, '\\') {
        lang.add_multiline(rule);
    }
    if let Some(rule) = MultilineRule::with_escape("single_string", r"'", r"'", TokenType::String, 4, '\\') {
        lang.add_multiline(rule);
    }

    // Single-line patterns

    // Comments
    if let Some(rule) = PatternRule::new("comment", r"#.*$", TokenType::Comment, 100) {
        lang.add_pattern(rule);
    }

    // Decorators
    if let Some(rule) = PatternRule::new("decorator", r"@\w+", TokenType::Attribute, 95) {
        lang.add_pattern(rule);
    }

    // f-strings prefix
    if let Some(rule) = PatternRule::new("fstring_prefix", r#"[fFrRbBuU]+(?=["'])"#, TokenType::Special, 92) {
        lang.add_pattern(rule);
    }

    // Keywords
    let keywords = r"\b(False|None|True|and|as|assert|async|await|break|class|continue|def|del|elif|else|except|finally|for|from|global|if|import|in|is|lambda|nonlocal|not|or|pass|raise|return|try|while|with|yield)\b";
    if let Some(rule) = PatternRule::new("keyword", keywords, TokenType::Keyword, 80) {
        lang.add_pattern(rule);
    }

    // Built-in functions
    let builtins = r"\b(abs|all|any|ascii|bin|bool|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip)\b";
    if let Some(rule) = PatternRule::new("builtin", builtins, TokenType::Function, 75) {
        lang.add_pattern(rule);
    }

    // self/cls
    if let Some(rule) = PatternRule::new("self", r"\b(self|cls)\b", TokenType::Special, 77) {
        lang.add_pattern(rule);
    }

    // Numbers
    // Hex
    if let Some(rule) = PatternRule::new("hex", r"\b0[xX][0-9a-fA-F_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Binary
    if let Some(rule) = PatternRule::new("binary", r"\b0[bB][01_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Octal
    if let Some(rule) = PatternRule::new("octal", r"\b0[oO][0-7_]+\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Float
    if let Some(rule) = PatternRule::new("float", r"\b\d[\d_]*\.\d[\d_]*(?:[eE][+-]?\d+)?j?\b", TokenType::Number, 64) {
        lang.add_pattern(rule);
    }
    // Integer
    if let Some(rule) = PatternRule::new("integer", r"\b\d[\d_]*j?\b", TokenType::Number, 63) {
        lang.add_pattern(rule);
    }

    // Operators
    if let Some(rule) = PatternRule::new("operator", r"[+\-*/%&|^!<>=@~]+", TokenType::Operator, 40) {
        lang.add_pattern(rule);
    }

    lang
}
