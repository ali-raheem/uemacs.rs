//! C/C++ language definition

use crate::syntax::language::LanguageDefinition;
use crate::syntax::rules::{PatternRule, MultilineRule};
use crate::syntax::tokens::TokenType;

/// Create C language definition (also works for C++)
pub fn c_language() -> LanguageDefinition {
    let mut lang = LanguageDefinition::new("C");
    lang.add_extension("c");
    lang.add_extension("h");
    lang.add_extension("cpp");
    lang.add_extension("hpp");
    lang.add_extension("cc");
    lang.add_extension("cxx");

    // Multiline rules
    // Block comments
    if let Some(rule) = MultilineRule::new("block_comment", r"/\*", r"\*/", TokenType::Comment, 1) {
        lang.add_multiline(rule);
    }

    // Strings (with escape support)
    if let Some(rule) = MultilineRule::with_escape("string", r#"""#, r#"""#, TokenType::String, 2, '\\') {
        lang.add_multiline(rule);
    }

    // Single-line patterns

    // Line comments
    if let Some(rule) = PatternRule::new("line_comment", r"//.*$", TokenType::Comment, 100) {
        lang.add_pattern(rule);
    }

    // Preprocessor directives
    if let Some(rule) = PatternRule::new("preprocessor", r"^\s*#\s*\w+", TokenType::Preprocessor, 95) {
        lang.add_pattern(rule);
    }

    // Character literals
    if let Some(rule) = PatternRule::new("char", r"'(?:[^'\\]|\\.)'", TokenType::Char, 90) {
        lang.add_pattern(rule);
    }

    // Keywords
    let keywords = r"\b(auto|break|case|char|const|continue|default|do|double|else|enum|extern|float|for|goto|if|inline|int|long|register|restrict|return|short|signed|sizeof|static|struct|switch|typedef|union|unsigned|void|volatile|while|_Alignas|_Alignof|_Atomic|_Bool|_Complex|_Generic|_Imaginary|_Noreturn|_Static_assert|_Thread_local)\b";
    if let Some(rule) = PatternRule::new("keyword", keywords, TokenType::Keyword, 80) {
        lang.add_pattern(rule);
    }

    // C++ keywords
    let cpp_keywords = r"\b(alignas|alignof|and|and_eq|asm|bitand|bitor|bool|catch|class|compl|concept|consteval|constexpr|constinit|const_cast|co_await|co_return|co_yield|decltype|delete|dynamic_cast|explicit|export|false|friend|mutable|namespace|new|noexcept|not|not_eq|nullptr|operator|or|or_eq|private|protected|public|reinterpret_cast|requires|static_assert|static_cast|template|this|thread_local|throw|true|try|typeid|typename|using|virtual|xor|xor_eq)\b";
    if let Some(rule) = PatternRule::new("cpp_keyword", cpp_keywords, TokenType::Keyword, 79) {
        lang.add_pattern(rule);
    }

    // Type names (standard types)
    let types = r"\b(size_t|ptrdiff_t|intptr_t|uintptr_t|int8_t|int16_t|int32_t|int64_t|uint8_t|uint16_t|uint32_t|uint64_t|FILE|NULL)\b";
    if let Some(rule) = PatternRule::new("type", types, TokenType::Type, 75) {
        lang.add_pattern(rule);
    }

    // Numbers
    // Hex
    if let Some(rule) = PatternRule::new("hex", r"\b0[xX][0-9a-fA-F]+[uUlL]*\b", TokenType::Number, 65) {
        lang.add_pattern(rule);
    }
    // Float
    if let Some(rule) = PatternRule::new("float", r"\b\d+\.\d*(?:[eE][+-]?\d+)?[fFlL]?\b", TokenType::Number, 64) {
        lang.add_pattern(rule);
    }
    if let Some(rule) = PatternRule::new("float2", r"\b\d*\.\d+(?:[eE][+-]?\d+)?[fFlL]?\b", TokenType::Number, 64) {
        lang.add_pattern(rule);
    }
    // Integer
    if let Some(rule) = PatternRule::new("integer", r"\b\d+[uUlL]*\b", TokenType::Number, 63) {
        lang.add_pattern(rule);
    }

    // Operators
    if let Some(rule) = PatternRule::new("operator", r"[+\-*/%&|^!<>=~?:]+", TokenType::Operator, 40) {
        lang.add_pattern(rule);
    }

    lang
}
