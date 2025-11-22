//! Models for the Soul programming language.
//!
//! This crate provides the core data structures and types used to represent
//! Soul language programs, including the Abstract Syntax Tree (AST), error types,
//! scope management, and language keywords/symbols.

pub mod error;
pub mod scope;
pub mod soul_names;
pub mod symbool_kind;
pub mod soul_page_path;
pub mod abstract_syntax_tree;