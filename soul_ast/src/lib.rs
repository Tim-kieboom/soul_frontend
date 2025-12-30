//! Models for the Soul programming language.
//!
//! This crate provides the core data structures and types used to represent
//! Soul language programs, including the Abstract Syntax Tree (AST), error types,
//! scope management, and language keywords/symbols.

pub mod define_enums;
pub mod abstract_syntax_tree;
pub mod error;
pub mod sementic_models;
pub mod soul_names;
pub mod soul_page_path;
pub mod symbool_kind;

use crate::{abstract_syntax_tree::AbstractSyntaxTree, sementic_models::{ASTSemanticInfo, sementic_fault::SementicLevel}};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseResonse {
    pub syntax_tree: AbstractSyntaxTree,
    pub sementic_info: ASTSemanticInfo,
}
impl ParseResonse {
    pub fn get_fatal_count(&self, fatal_level: SementicLevel) -> usize {
        self.sementic_info
            .faults
            .iter()
            .filter(|fault| fault.is_fatal(fatal_level))
            .count()
    }
}
