//! Syntax and styling module
//!
//! This module provides text styling infrastructure used for:
//! - Region/selection highlighting
//! - Syntax highlighting
//! - Search match highlighting (future)

mod style;
mod tokens;
mod rules;
mod language;
mod manager;
mod builtin;

pub use style::{Color, Span, Style};
pub use tokens::TokenType;
pub use rules::LineState;
pub use language::LanguageDefinition;
pub use manager::{SyntaxManager, HighlightCache};
