//! Built-in language definitions
//!
//! This module provides syntax highlighting definitions for
//! common programming languages.

mod rust;
mod c;
mod python;
mod toml_lang;
mod markdown;

use super::language::LanguageDefinition;

/// Get all built-in language definitions
pub fn all_languages() -> Vec<LanguageDefinition> {
    vec![
        rust::rust_language(),
        c::c_language(),
        python::python_language(),
        toml_lang::toml_language(),
        markdown::markdown_language(),
    ]
}
